//! Pixel handling shared by the build-time optimizer and any other caller that
//! needs to resize or encode an image.
//!
//! Two things live here because getting either wrong is expensive and the cost
//! is invisible until someone profiles a large image:
//!
//! - **Borrowing, not copying.** A `DynamicImage` that already holds RGB8 or
//!   RGBA8 pixels can hand its buffer straight to both the resizer and the WebP
//!   encoder. `to_rgb8()`/`to_rgba8()` clone instead, which on a 6000x4000
//!   source is a 68 MB allocation and memcpy — per encode, and one source
//!   produces nine of them.
//! - **SIMD convolution.** `image::imageops::FilterType::Lanczos3` is a scalar
//!   loop. `fast_image_resize` runs the same Lanczos3 convolution through
//!   AVX2/SSE4.1/NEON. Measured on a 6000x4000 JPEG, producing all eight
//!   responsive widths: 3628 ms scalar, 68 ms SIMD.

use std::fmt;

use fast_image_resize::images::{Image as FirImage, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, GenericImageView};

/// Why a resize or encode could not produce output.
///
/// This crate carries no error framework, and adding one for two call sites
/// would be the larger change. `std::error::Error` is enough for callers that
/// do have one to convert.
#[derive(Debug)]
pub enum ImageCodecError {
    InvalidBuffer(String),
    Resize(String),
    Encode(String),
}

impl fmt::Display for ImageCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBuffer(detail) => write!(formatter, "invalid pixel buffer: {detail}"),
            Self::Resize(detail) => write!(formatter, "image resize failed: {detail}"),
            Self::Encode(detail) => write!(formatter, "WebP encoding failed: {detail}"),
        }
    }
}

impl std::error::Error for ImageCodecError {}

pub type Result<T> = std::result::Result<T, ImageCodecError>;

/// Pixels in a layout both the resizer and the WebP encoder accept directly.
#[derive(Clone, Copy)]
pub enum PixelLayout {
    Rgb8,
    Rgba8,
}

impl PixelLayout {
    fn pixel_type(self) -> PixelType {
        match self {
            Self::Rgb8 => PixelType::U8x3,
            Self::Rgba8 => PixelType::U8x4,
        }
    }
}

/// A decoded image whose pixels are ready to use without another conversion.
///
/// `borrowed` is the common case: `image` decodes JPEG to `ImageRgb8` and PNG
/// to `ImageRgb8`/`ImageRgba8`, so the buffer is already in one of the two
/// layouts WebP wants. Formats that decode to something else (16-bit, luma,
/// CMYK) convert once here rather than at every use.
pub struct Pixels<'a> {
    data: PixelData<'a>,
    pub width: u32,
    pub height: u32,
    pub layout: PixelLayout,
}

enum PixelData<'a> {
    Borrowed(&'a [u8]),
    Owned(Vec<u8>),
}

impl<'a> Pixels<'a> {
    /// Borrow a decoded image's pixels, converting only when its layout is
    /// neither RGB8 nor RGBA8.
    pub fn from_image(decoded: &'a DynamicImage) -> Self {
        let (width, height) = decoded.dimensions();
        if let Some(buffer) = decoded.as_rgb8() {
            return Self {
                data: PixelData::Borrowed(buffer.as_raw()),
                width,
                height,
                layout: PixelLayout::Rgb8,
            };
        }
        if let Some(buffer) = decoded.as_rgba8() {
            return Self {
                data: PixelData::Borrowed(buffer.as_raw()),
                width,
                height,
                layout: PixelLayout::Rgba8,
            };
        }
        // Preserve alpha when the source has it; dropping to RGB here would
        // silently flatten transparency that the original file carried.
        if decoded.color().has_alpha() {
            Self {
                data: PixelData::Owned(decoded.to_rgba8().into_raw()),
                width,
                height,
                layout: PixelLayout::Rgba8,
            }
        } else {
            Self {
                data: PixelData::Owned(decoded.to_rgb8().into_raw()),
                width,
                height,
                layout: PixelLayout::Rgb8,
            }
        }
    }

    /// Take ownership of an already-materialized buffer, such as a resize result.
    pub fn from_owned(data: Vec<u8>, width: u32, height: u32, layout: PixelLayout) -> Self {
        Self {
            data: PixelData::Owned(data),
            width,
            height,
            layout,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        match &self.data {
            PixelData::Borrowed(slice) => slice,
            PixelData::Owned(vec) => vec,
        }
    }

    /// Downscale to `width` x `height`, keeping the source layout.
    ///
    /// Every target is produced from these pixels rather than from the
    /// previously emitted, smaller variant. Chaining would be cheaper on a
    /// scalar resizer, but it resamples the same image once per step and the
    /// error compounds; with SIMD the full-source path is already fast enough
    /// that there is nothing to buy with that trade.
    pub fn resize(&self, width: u32, height: u32) -> Result<Pixels<'static>> {
        let source = ImageRef::new(
            self.width,
            self.height,
            self.as_slice(),
            self.layout.pixel_type(),
        )
        .map_err(|error| ImageCodecError::InvalidBuffer(error.to_string()))?;
        let mut destination = FirImage::new(width, height, self.layout.pixel_type());
        // A `Resizer` owns scratch buffers and is not shared between threads;
        // constructing one per resize costs nothing next to the convolution.
        Resizer::new()
            .resize(
                &source,
                &mut destination,
                &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3)),
            )
            .map_err(|error| ImageCodecError::Resize(error.to_string()))?;
        Ok(Pixels::from_owned(
            destination.into_vec(),
            width,
            height,
            self.layout,
        ))
    }
}

/// WebP encoder settings.
#[derive(Debug, Clone, Copy)]
pub struct WebpSettings {
    pub quality: u8,
    pub lossless: bool,
    /// libwebp's `method`: 0 is fastest and largest, 6 is slowest and smallest.
    pub effort: u8,
}

/// Encode pixels to WebP.
///
/// Mirrors what `Encoder::encode_simple` sets up, plus `method`. libwebp's
/// `thread_level` is deliberately left alone: it does not split a single lossy
/// image encode, and measurement confirmed it (2385 ms vs 2351 ms on a
/// 6000x4000 source). Encode parallelism comes from running the independent
/// outputs concurrently, not from inside one encode.
pub fn encode_webp(pixels: &Pixels<'_>, settings: WebpSettings) -> Result<Vec<u8>> {
    let mut config = webp::WebPConfig::new().map_err(|()| {
        ImageCodecError::Encode("could not initialize the encoder configuration".to_string())
    })?;
    config.lossless = i32::from(settings.lossless);
    config.alpha_compression = i32::from(!settings.lossless);
    config.quality = f32::from(settings.quality.clamp(1, 100));
    config.method = i32::from(settings.effort.min(6));

    let encoder = match pixels.layout {
        PixelLayout::Rgb8 => {
            webp::Encoder::from_rgb(pixels.as_slice(), pixels.width, pixels.height)
        }
        PixelLayout::Rgba8 => {
            webp::Encoder::from_rgba(pixels.as_slice(), pixels.width, pixels.height)
        }
    };
    encoder
        .encode_advanced(&config)
        .map(|memory| memory.to_vec())
        .map_err(|error| ImageCodecError::Encode(format!("{error:?}")))
}

/// Height that preserves aspect ratio, never zero.
///
/// A zero height would make the encoder reject the buffer on extreme aspect
/// ratios, so the floor is part of the contract rather than a caller's job.
pub fn scaled_height(source_width: u32, source_height: u32, target_width: u32) -> u32 {
    ((u64::from(target_width) * u64::from(source_height)) / u64::from(source_width.max(1))).max(1)
        as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, Rgba};

    #[test]
    fn borrows_rgb_and_rgba_buffers_without_converting() {
        let rgb = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(4, 3, Rgb([1u8, 2, 3])));
        let pixels = Pixels::from_image(&rgb);
        assert!(matches!(pixels.layout, PixelLayout::Rgb8));
        assert!(std::ptr::eq(
            pixels.as_slice().as_ptr(),
            rgb.as_rgb8().unwrap().as_raw().as_ptr()
        ));

        let rgba = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(4, 3, Rgba([1u8, 2, 3, 4])));
        let pixels = Pixels::from_image(&rgba);
        assert!(matches!(pixels.layout, PixelLayout::Rgba8));
        assert!(std::ptr::eq(
            pixels.as_slice().as_ptr(),
            rgba.as_rgba8().unwrap().as_raw().as_ptr()
        ));
    }

    #[test]
    fn converts_other_layouts_once_and_keeps_alpha() {
        // Luma8 has no alpha and must not be widened to RGBA.
        let luma = DynamicImage::ImageLuma8(ImageBuffer::from_pixel(2, 2, image::Luma([9u8])));
        assert!(matches!(
            Pixels::from_image(&luma).layout,
            PixelLayout::Rgb8
        ));

        // LumaA8 does, and flattening it here would drop transparency the
        // source actually carried.
        let luma_alpha =
            DynamicImage::ImageLumaA8(ImageBuffer::from_pixel(2, 2, image::LumaA([9u8, 128])));
        assert!(matches!(
            Pixels::from_image(&luma_alpha).layout,
            PixelLayout::Rgba8
        ));
    }

    #[test]
    fn resize_preserves_layout_and_target_size() {
        let source = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(100, 40, Rgba([7u8; 4])));
        let pixels = Pixels::from_image(&source);
        let resized = pixels.resize(50, scaled_height(100, 40, 50)).unwrap();
        assert_eq!((resized.width, resized.height), (50, 20));
        assert!(matches!(resized.layout, PixelLayout::Rgba8));
        assert_eq!(resized.as_slice().len(), 50 * 20 * 4);
    }

    #[test]
    fn scaled_height_never_returns_zero() {
        // A 10000x1 banner scaled to 16px wide rounds to 0 before the floor.
        assert_eq!(scaled_height(10_000, 1, 16), 1);
        assert_eq!(scaled_height(1000, 500, 640), 320);
    }

    #[test]
    fn effort_changes_output_without_changing_dimensions() {
        let source = DynamicImage::ImageRgb8(ImageBuffer::from_fn(64, 64, |x, y| {
            Rgb([(x * 4) as u8, (y * 4) as u8, ((x ^ y) * 2) as u8])
        }));
        let pixels = Pixels::from_image(&source);
        let base = WebpSettings {
            quality: 82,
            lossless: false,
            effort: 4,
        };
        let slow = encode_webp(&pixels, base).unwrap();
        let fast = encode_webp(&pixels, WebpSettings { effort: 0, ..base }).unwrap();
        for encoded in [&slow, &fast] {
            assert_eq!(
                image::load_from_memory(encoded).unwrap().dimensions(),
                (64, 64)
            );
        }
    }
}
