use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex};

use image::GenericImageView;

use crate::static_assets::{contained_public_asset, is_safe_relative_path};

const MAX_SOURCE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_SOURCE_PIXELS: u64 = 50_000_000;
const MAX_CACHE_ENTRIES: usize = 128;
const MAX_CACHE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct DynamicImageConfig {
    pub enabled: bool,
    pub max_width: u32,
    pub default_quality: u8,
}

impl Default for DynamicImageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_width: 3840,
            default_quality: 82,
        }
    }
}

#[derive(Default)]
pub(crate) struct DynamicImageCache {
    inner: Mutex<CacheInner>,
}

#[derive(Default)]
struct CacheInner {
    entries: HashMap<String, Arc<[u8]>>,
    order: VecDeque<String>,
    bytes: usize,
}

impl DynamicImageCache {
    fn get(&self, key: &str) -> Option<Arc<[u8]>> {
        let mut inner = self.inner.lock().ok()?;
        let value = inner.entries.get(key)?.clone();
        if let Some(index) = inner.order.iter().position(|entry| entry == key) {
            inner.order.remove(index);
        }
        inner.order.push_back(key.to_string());
        Some(value)
    }

    fn insert(&self, key: String, value: Arc<[u8]>) -> Arc<[u8]> {
        let Ok(mut inner) = self.inner.lock() else {
            return value;
        };
        if value.len() > MAX_CACHE_BYTES {
            return value;
        }
        if let Some(previous) = inner.entries.remove(&key) {
            inner.bytes = inner.bytes.saturating_sub(previous.len());
            if let Some(index) = inner.order.iter().position(|entry| entry == &key) {
                inner.order.remove(index);
            }
        }
        inner.bytes = inner.bytes.saturating_add(value.len());
        inner.order.push_back(key.clone());
        inner.entries.insert(key, value.clone());
        while inner.entries.len() > MAX_CACHE_ENTRIES || inner.bytes > MAX_CACHE_BYTES {
            let Some(oldest) = inner.order.pop_front() else {
                break;
            };
            if let Some(removed) = inner.entries.remove(&oldest) {
                inner.bytes = inner.bytes.saturating_sub(removed.len());
            }
        }
        value
    }
}

#[derive(Debug)]
pub(crate) enum DynamicImageError {
    InvalidRequest(&'static str),
    NotFound,
    TooLarge,
    Decode,
    Io(std::io::Error),
    Worker,
}

pub(crate) async fn optimize(
    public_dir: &Path,
    config: &DynamicImageConfig,
    cache: &DynamicImageCache,
    src: &str,
    width: u32,
    quality: Option<u8>,
) -> Result<Arc<[u8]>, DynamicImageError> {
    if !config.enabled {
        return Err(DynamicImageError::NotFound);
    }
    if width < 16 || width > config.max_width.min(8192) {
        return Err(DynamicImageError::InvalidRequest("invalid image width"));
    }
    let relative = src
        .strip_prefix('/')
        .filter(|path| !src.starts_with("//") && is_safe_relative_path(path))
        .ok_or(DynamicImageError::InvalidRequest(
            "image src must be a root-relative public path",
        ))?;
    if src.contains(['?', '#']) {
        return Err(DynamicImageError::InvalidRequest(
            "image src must not contain a query or fragment",
        ));
    }
    let candidate = public_dir.join(relative);
    let file = contained_public_asset(public_dir, &candidate).ok_or(DynamicImageError::NotFound)?;
    let extension = file
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(extension.as_deref(), Some("png" | "jpg" | "jpeg" | "webp")) {
        return Err(DynamicImageError::InvalidRequest(
            "runtime optimization supports PNG, JPEG, and WebP",
        ));
    }
    let metadata = tokio::fs::metadata(&file)
        .await
        .map_err(DynamicImageError::Io)?;
    if metadata.len() > MAX_SOURCE_BYTES {
        return Err(DynamicImageError::TooLarge);
    }
    let source = tokio::fs::read(&file)
        .await
        .map_err(DynamicImageError::Io)?;
    let quality = quality.unwrap_or(config.default_quality).clamp(1, 100);
    let mut hasher = blake3::Hasher::new();
    hasher.update(&source);
    hasher.update(&width.to_le_bytes());
    hasher.update(&[quality]);
    let key = hasher.finalize().to_hex().to_string();
    if let Some(bytes) = cache.get(&key) {
        return Ok(bytes);
    }

    let encoded = tokio::task::spawn_blocking(move || {
        let decoded = image::load_from_memory(&source).map_err(|_| DynamicImageError::Decode)?;
        let (source_width, source_height) = decoded.dimensions();
        if u64::from(source_width) * u64::from(source_height) > MAX_SOURCE_PIXELS {
            return Err(DynamicImageError::TooLarge);
        }
        let target_width = width.min(source_width).max(1);
        let resized = if target_width == source_width {
            decoded
        } else {
            decoded.resize(
                target_width,
                source_height,
                image::imageops::FilterType::Lanczos3,
            )
        };
        let (width, height) = resized.dimensions();
        let encoded = if resized.color().has_alpha() {
            let pixels = resized.to_rgba8();
            webp::Encoder::from_rgba(pixels.as_raw(), width, height)
                .encode_simple(false, f32::from(quality))
                .map_err(|_| DynamicImageError::Decode)?
                .to_vec()
        } else {
            let pixels = resized.to_rgb8();
            webp::Encoder::from_rgb(pixels.as_raw(), width, height)
                .encode_simple(false, f32::from(quality))
                .map_err(|_| DynamicImageError::Decode)?
                .to_vec()
        };
        Ok::<Vec<u8>, DynamicImageError>(encoded)
    })
    .await
    .map_err(|_| DynamicImageError::Worker)??;
    Ok(cache.insert(key, Arc::from(encoded)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_external_and_traversing_sources_before_io() {
        let temp = tempfile::tempdir().unwrap();
        let config = DynamicImageConfig {
            enabled: true,
            ..Default::default()
        };
        let cache = DynamicImageCache::default();
        for source in ["https://example.com/a.png", "/../a.png", "//host/a.png"] {
            assert!(matches!(
                optimize(temp.path(), &config, &cache, source, 640, None).await,
                Err(DynamicImageError::InvalidRequest(_))
            ));
        }
    }

    #[tokio::test]
    async fn resizes_public_images_and_reuses_the_bounded_cache() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("avatar.png");
        image::RgbaImage::from_pixel(100, 50, image::Rgba([10, 40, 90, 255]))
            .save(&source)
            .unwrap();
        let config = DynamicImageConfig {
            enabled: true,
            ..Default::default()
        };
        let cache = DynamicImageCache::default();
        let first = optimize(temp.path(), &config, &cache, "/avatar.png", 40, Some(80))
            .await
            .unwrap();
        let second = optimize(temp.path(), &config, &cache, "/avatar.png", 40, Some(80))
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        let decoded = image::load_from_memory(&first).unwrap();
        assert_eq!(decoded.dimensions(), (40, 20));
    }
}
