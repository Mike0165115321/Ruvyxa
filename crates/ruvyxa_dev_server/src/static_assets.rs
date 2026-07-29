//! Public-directory and client-bundle static file serving: path safety,
//! image format fallback, ETag/conditional responses, and content types.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime};

use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use ruvyxa_diagnostics::{Result, RuvyxaError};

use crate::apply_security_headers;

/// Maximum number of asset fingerprints kept in memory.
const ASSET_ETAG_CACHE_LIMIT: usize = 1024;

/// How long a file's modification time must be in the past before its ETag is
/// eligible for caching.
///
/// ETags are content hashes, but the cache is keyed by `(len, mtime)`. Several
/// filesystems record mtime with one-second granularity, so two writes inside
/// the same second can leave identical `(len, mtime)` for different bytes.
/// Only fingerprinting files whose mtime is already older than this window
/// removes that ambiguity: any later write necessarily lands in a newer second
/// and therefore misses the cache.
const ASSET_ETAG_SETTLE: Duration = Duration::from_secs(2);

/// Identity of the file a cached ETag was computed from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AssetIdentity {
    len: u64,
    modified: SystemTime,
}

/// Bounded `path -> (identity, etag)` index over settled public assets.
///
/// Public assets ship with `must-revalidate`, so browsers re-ask for files they
/// already hold and the conditional request is the steady-state path, not an
/// edge case. Without this index every one of those revalidations reads the
/// whole file off disk and blake3-hashes it only to answer `304 Not Modified`
/// with an empty body — the exact work a 304 exists to avoid.
///
/// Eviction is insertion-ordered, matching `ContentModuleCache` in the bundler.
#[derive(Default)]
struct AssetEtagCache {
    entries: HashMap<PathBuf, (AssetIdentity, String)>,
    insertion_order: VecDeque<PathBuf>,
}

static ASSET_ETAG_CACHE: LazyLock<Mutex<AssetEtagCache>> =
    LazyLock::new(|| Mutex::new(AssetEtagCache::default()));

/// Current identity of a file, or `None` when its mtime is unreadable.
fn asset_identity(metadata: &std::fs::Metadata) -> Option<AssetIdentity> {
    Some(AssetIdentity {
        len: metadata.len(),
        modified: metadata.modified().ok()?,
    })
}

/// ETag previously computed for exactly these bytes, if still valid.
///
/// A mismatch on either length or mtime is treated as a miss, so a rewritten
/// file never reuses its predecessor's ETag.
fn cached_asset_etag(file: &Path, identity: &AssetIdentity) -> Option<String> {
    let cache = ASSET_ETAG_CACHE.lock().ok()?;
    let (cached_identity, etag) = cache.entries.get(file)?;
    (cached_identity == identity).then(|| etag.clone())
}

/// Record the ETag of a file that has stopped changing.
///
/// Files modified within [`ASSET_ETAG_SETTLE`] are deliberately not recorded:
/// see that constant for why a fresh mtime cannot identify content.
fn store_asset_etag(file: &Path, identity: &AssetIdentity, etag: &str) {
    if !is_settled(identity, SystemTime::now()) {
        return;
    }

    let Ok(mut cache) = ASSET_ETAG_CACHE.lock() else {
        return;
    };
    if cache
        .entries
        .insert(file.to_path_buf(), (identity.clone(), etag.to_string()))
        .is_none()
    {
        cache.insertion_order.push_back(file.to_path_buf());
    }
    while cache.insertion_order.len() > ASSET_ETAG_CACHE_LIMIT {
        let Some(oldest) = cache.insertion_order.pop_front() else {
            break;
        };
        cache.entries.remove(&oldest);
    }
}

/// Whether a file has stopped changing long enough for `(len, mtime)` to
/// identify its bytes.
///
/// An mtime in the future is never settled: clock skew and timestamp-preserving
/// copies both produce one, and neither lets us bound how recently the content
/// was written.
fn is_settled(identity: &AssetIdentity, now: SystemTime) -> bool {
    now.duration_since(identity.modified)
        .is_ok_and(|age| age >= ASSET_ETAG_SETTLE)
}

/// True when the request already holds the version identified by `etag`.
fn request_matches_etag(request_headers: Option<&HeaderMap>, etag: &str) -> bool {
    request_headers
        .and_then(|headers| headers.get(header::IF_NONE_MATCH))
        .is_some_and(|value| etag_matches(value, etag))
}

fn not_modified_response() -> Response {
    let mut response = StatusCode::NOT_MODIFIED.into_response();
    apply_security_headers(&mut response);
    response
}

pub(crate) async fn serve_public_file(
    public_dir: &Path,
    request_path: &str,
    request_headers: Option<&HeaderMap>,
) -> Result<Option<Response>> {
    let trimmed = request_path.trim_start_matches('/');
    if !is_safe_relative_path(trimmed) {
        return Ok(None);
    }

    let Some(file) = resolve_public_asset(public_dir, trimmed) else {
        return Ok(None);
    };
    let metadata = match tokio::fs::metadata(&file).await {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => return Ok(None),
    };
    let identity = asset_identity(&metadata);

    // Answer a revalidation from the fingerprint index, before touching the
    // file. This is the whole point of the index: a 304 carries no body, so
    // reading and hashing the file to produce one is pure waste.
    if let Some(identity) = &identity
        && let Some(etag) = cached_asset_etag(&file, identity)
        && request_matches_etag(request_headers, &etag)
    {
        return Ok(Some(not_modified_response()));
    }

    let bytes = tokio::fs::read(&file)
        .await
        .map_err(|source| RuvyxaError::Io {
            message: format!("Failed to read public file {}", file.display()),
            source,
        })?;

    // Always hash the bytes actually being served rather than trusting the
    // index here. The file could have been rewritten between the metadata read
    // and this one, and an ETag that does not describe the response body would
    // stay wrong in every downstream cache until the file changed again.
    let etag = compute_etag(&bytes);
    if let Some(identity) = &identity {
        store_asset_etag(&file, identity, &etag);
    }

    // Check If-None-Match for conditional response
    if request_matches_etag(request_headers, &etag) {
        return Ok(Some(not_modified_response()));
    }

    let content_type = content_type_for(&file);
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(&etag).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600, must-revalidate"),
    );
    apply_security_headers(&mut response);
    Ok(Some(response))
}

/// Sync fallback for static file serving (used by render_request test/bench path).
pub(crate) fn serve_public_file_sync(
    public_dir: &Path,
    request_path: &str,
) -> Result<Option<Response>> {
    let trimmed = request_path.trim_start_matches('/');
    if !is_safe_relative_path(trimmed) {
        return Ok(None);
    }
    let Some(file) = resolve_public_asset(public_dir, trimmed) else {
        return Ok(None);
    };
    let bytes = fs::read(&file)?;
    let content_type = content_type_for(&file);
    let mut response = bytes.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    apply_security_headers(&mut response);
    Ok(Some(response))
}

/// Sync fallback for client file serving (used by render_request test/bench path).
pub(crate) fn serve_client_file_sync(
    client_dir: &Path,
    request_path: &str,
) -> Result<Option<Response>> {
    let Some(file_name) = request_path.strip_prefix("/__ruvyxa/client/") else {
        return Ok(None);
    };
    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains("..")
    {
        return Ok(None);
    }
    let Some(file) = contained_public_asset(client_dir, &client_dir.join(file_name)) else {
        return Ok(None);
    };
    if !file.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&file)?;
    let mut response = bytes.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/javascript; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    apply_security_headers(&mut response);
    Ok(Some(response))
}

pub(crate) async fn serve_client_file(
    client_dir: &Path,
    request_path: &str,
    request_headers: Option<&HeaderMap>,
) -> Result<Option<Response>> {
    let Some(file_name) = request_path.strip_prefix("/__ruvyxa/client/") else {
        return Ok(None);
    };

    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains("..")
    {
        return Ok(None);
    }

    let Some(file) = contained_public_asset(client_dir, &client_dir.join(file_name)) else {
        return Ok(None);
    };
    match tokio::fs::metadata(&file).await {
        Ok(meta) if meta.is_file() => {}
        _ => return Ok(None),
    }

    let bytes = tokio::fs::read(&file)
        .await
        .map_err(|source| RuvyxaError::Io {
            message: format!("Failed to read client file {}", file.display()),
            source,
        })?;

    // Client bundles are content-hashed, so use immutable caching with ETag
    let etag = compute_etag(&bytes);

    if let Some(headers) = request_headers
        && let Some(if_none_match) = headers.get(header::IF_NONE_MATCH)
        && etag_matches(if_none_match, &etag)
    {
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        apply_security_headers(&mut response);
        return Ok(Some(response));
    }

    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/javascript; charset=utf-8"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(&etag).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    apply_security_headers(&mut response);
    Ok(Some(response))
}

/// Map a public URL path to the file that should answer it.
///
/// Resolution is driven entirely by the requested URL extension, never by the
/// `Accept` header, so responses are not content-negotiated and need no `Vary`.
pub(crate) fn resolve_public_asset(public_dir: &Path, request_path: &str) -> Option<PathBuf> {
    let requested = public_dir.join(request_path);
    if requested.is_file() {
        return contained_public_asset(public_dir, &requested);
    }

    // Development keeps source images untouched while the React component
    // points at the production `.webp` URL. Resolve that URL to exactly one
    // source format; ambiguity matches the build-time collision guard.
    if requested.extension().and_then(|value| value.to_str()) == Some("webp") {
        let mut candidates = ["png", "jpg", "jpeg", "PNG", "JPG", "JPEG"]
            .map(|extension| requested.with_extension(extension))
            .into_iter()
            .filter_map(|path| {
                path.is_file()
                    .then(|| contained_public_asset(public_dir, &path))
                    .flatten()
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.dedup();
        if candidates.len() == 1 {
            return candidates.into_iter().next();
        }
    }

    // Keep server deployments compatible with plain `<img src="hero.png">`
    // while the build output stores only `hero.webp`.
    if is_convertible_image_url(&requested) {
        let webp = requested.with_extension("webp");
        if webp.is_file() {
            return contained_public_asset(public_dir, &webp);
        }
    }
    None
}

/// Canonicalize asset paths before serving them so public-directory symlinks
/// cannot expose files outside the configured root.
pub(crate) fn contained_public_asset(public_dir: &Path, candidate: &Path) -> Option<PathBuf> {
    if !public_dir.exists() || !candidate.exists() {
        return None;
    }
    let public_root = ruvyxa_diagnostics::normalized_canonical_path(public_dir);
    let candidate = ruvyxa_diagnostics::normalized_canonical_path(candidate);
    candidate.starts_with(&public_root).then_some(candidate)
}

/// Extensions that only ever name a build or public asset.
///
/// Restricted to images, fonts, media, and emitted web assets: none of these
/// is a plausible value for a dynamic route parameter, so refusing them cannot
/// swallow a real page. Mirrors `STATIC_ASSET_EXTENSIONS` in
/// `packages/ruvyxa/runtime/serverless-handler.mjs`.
const STATIC_ASSET_EXTENSIONS: [&str; 25] = [
    "apng", "avif", "bmp", "css", "eot", "gif", "ico", "jpeg", "jpg", "js", "map", "mjs", "mov",
    "mp3", "mp4", "ogg", "otf", "png", "svg", "ttf", "wav", "webm", "webp", "woff", "woff2",
];

/// True when the last path segment names a static asset file.
///
/// A request that reaches routing with this shape has already missed both the
/// client bundle directory and the public directory, so the file genuinely
/// does not exist and a dynamic route must not render a page for it.
pub(crate) fn is_static_asset_request(request_path: &str) -> bool {
    if is_crawler_discovery_path(request_path) {
        return true;
    }
    let segment = request_path.rsplit('/').next().unwrap_or_default();
    let Some((name, extension)) = segment.rsplit_once('.') else {
        return false;
    };
    if name.is_empty() || extension.is_empty() {
        return false;
    }
    let extension = extension.to_ascii_lowercase();
    STATIC_ASSET_EXTENSIONS.contains(&extension.as_str())
}

/// Well-known crawler files that are never a page.
///
/// `.txt` and `.xml` are deliberately absent from `STATIC_ASSET_EXTENSIONS` —
/// a route may legitimately end in either — but these exact paths are fixed by
/// convention. Letting `/[lang]` answer `/robots.txt` returns 200 with an HTML
/// body, which is exactly what Lighthouse's `robots-txt` audit fails on. The
/// build emits both files by default, so this only decides what a project that
/// turned generation off serves. Mirrors `isCrawlerDiscoveryPath()` in
/// `packages/ruvyxa/runtime/serverless-handler.mjs`.
fn is_crawler_discovery_path(request_path: &str) -> bool {
    matches!(
        request_path.trim_end_matches('/'),
        "/robots.txt" | "/sitemap.xml" | "/sitemap_index.xml"
    )
}

pub(crate) fn is_convertible_image_url(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("png" | "jpg" | "jpeg")
    )
}

pub(crate) fn is_safe_relative_path(path: &str) -> bool {
    if path.is_empty() || path.contains('\\') {
        return false;
    }

    Path::new(path).components().all(|component| {
        matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    })
}

/// Compute a strong ETag using blake3 hash of file content.
pub(crate) fn compute_etag(bytes: &[u8]) -> String {
    let hash = blake3::hash(bytes);
    format!("\"{}\"", &hash.to_hex()[..16])
}

pub(crate) fn etag_matches(value: &HeaderValue, etag: &str) -> bool {
    let Ok(value) = value.to_str() else {
        return false;
    };
    let target = etag.trim_matches('"');
    value.split(',').any(|candidate| {
        let candidate = candidate.trim();
        if candidate == "*" {
            return true;
        }
        candidate
            .strip_prefix("W/")
            .unwrap_or(candidate)
            .trim_matches('"')
            == target
    })
}

pub(crate) fn content_type_for(path: &Path) -> &'static str {
    // File-system extensions are case-preserving, and `resolve_public_asset`
    // deliberately resolves upper-case image sources such as `hero.PNG`.
    // Matching case-sensitively here would serve those as a binary download.
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("webmanifest") => "application/manifest+json; charset=utf-8",
        // RFC 9309 requires robots.txt to use text/plain. Sitemap XML is
        // likewise served as XML instead of the binary fallback, while the
        // explicit UTF-8 charset matches the generated declarations.
        Some("txt") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        _ => "application/octet-stream",
    }
}

pub(crate) fn public_asset_links(public_dir: &Path) -> String {
    let mut links = Vec::new();

    if public_dir.join("ruvyxa.png").exists() {
        links.push(r#"<link rel="icon" type="image/png" href="/ruvyxa.png">"#.to_string());
    }

    links.join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serve `file` and report the status plus the body length actually sent.
    async fn serve(
        public_dir: &Path,
        request_path: &str,
        if_none_match: Option<&str>,
    ) -> (StatusCode, usize) {
        let mut headers = HeaderMap::new();
        if let Some(value) = if_none_match {
            headers.insert(header::IF_NONE_MATCH, HeaderValue::from_str(value).unwrap());
        }
        let response = serve_public_file(public_dir, request_path, Some(&headers))
            .await
            .expect("serving must not fail")
            .expect("asset must resolve");
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body must be readable");
        (status, body.len())
    }

    async fn etag_of(public_dir: &Path, request_path: &str) -> String {
        let response = serve_public_file(public_dir, request_path, None)
            .await
            .expect("serving must not fail")
            .expect("asset must resolve");
        response
            .headers()
            .get(header::ETAG)
            .expect("a served asset carries an ETag")
            .to_str()
            .expect("ETags are ASCII")
            .to_string()
    }

    #[tokio::test]
    async fn conditional_request_for_unchanged_asset_returns_an_empty_304() {
        let temp = tempfile::tempdir().unwrap();
        let public_dir = temp.path();
        fs::write(public_dir.join("logo.png"), vec![7u8; 4096]).unwrap();

        let etag = etag_of(public_dir, "/logo.png").await;
        let (status, body_len) = serve(public_dir, "/logo.png", Some(&etag)).await;

        assert_eq!(status, StatusCode::NOT_MODIFIED);
        assert_eq!(body_len, 0, "a 304 must not carry a body");
    }

    #[tokio::test]
    async fn rewritten_asset_never_reuses_the_previous_etag() {
        // The correctness guarantee the fingerprint index must not break: an
        // ETag that outlived its content would stay wrong in every downstream
        // cache until the file changed again.
        let temp = tempfile::tempdir().unwrap();
        let public_dir = temp.path();
        let file = public_dir.join("app.css");

        fs::write(&file, "a{color:red}").unwrap();
        let first = etag_of(public_dir, "/app.css").await;

        fs::write(&file, "a{color:blue}").unwrap();
        let second = etag_of(public_dir, "/app.css").await;

        assert_ne!(first, second, "changed bytes must produce a new ETag");

        let (status, body_len) = serve(public_dir, "/app.css", Some(&first)).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "a stale ETag must not be answered with 304"
        );
        assert_eq!(body_len, "a{color:blue}".len());
    }

    #[tokio::test]
    async fn served_etag_always_describes_the_bytes_in_the_response() {
        let temp = tempfile::tempdir().unwrap();
        let public_dir = temp.path();
        fs::write(public_dir.join("data.txt"), "ruvyxa").unwrap();

        let response = serve_public_file(public_dir, "/data.txt", None)
            .await
            .unwrap()
            .unwrap();
        let etag = response
            .headers()
            .get(header::ETAG)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        assert_eq!(etag, compute_etag(&body));
    }

    #[test]
    fn only_files_that_stopped_changing_are_fingerprinted() {
        let now = SystemTime::now();
        let settled = AssetIdentity {
            len: 10,
            modified: now - ASSET_ETAG_SETTLE,
        };
        let fresh = AssetIdentity {
            len: 10,
            modified: now,
        };
        // A one-second-granularity filesystem can report this mtime for two
        // different writes, so it must not key a content hash.
        let borderline = AssetIdentity {
            len: 10,
            modified: now - Duration::from_millis(1500),
        };
        let future = AssetIdentity {
            len: 10,
            modified: now + Duration::from_secs(60),
        };

        assert!(is_settled(&settled, now));
        assert!(!is_settled(&fresh, now));
        assert!(!is_settled(&borderline, now));
        assert!(!is_settled(&future, now));
    }

    #[test]
    fn fingerprint_index_stays_bounded() {
        let identity = AssetIdentity {
            len: 1,
            modified: SystemTime::now() - ASSET_ETAG_SETTLE * 2,
        };
        for index in 0..(ASSET_ETAG_CACHE_LIMIT + 64) {
            store_asset_etag(
                &PathBuf::from(format!("/bounded-test/{index}.bin")),
                &identity,
                "\"deadbeefdeadbeef\"",
            );
        }

        let cache = ASSET_ETAG_CACHE.lock().unwrap();
        assert!(cache.entries.len() <= ASSET_ETAG_CACHE_LIMIT);
        assert_eq!(cache.entries.len(), cache.insertion_order.len());
    }

    #[test]
    fn a_changed_length_or_mtime_invalidates_the_fingerprint() {
        let file = PathBuf::from("/fingerprint-test/asset.bin");
        let modified = SystemTime::now() - ASSET_ETAG_SETTLE * 2;
        let identity = AssetIdentity { len: 32, modified };
        store_asset_etag(&file, &identity, "\"0123456789abcdef\"");

        assert_eq!(
            cached_asset_etag(&file, &identity).as_deref(),
            Some("\"0123456789abcdef\"")
        );
        assert_eq!(
            cached_asset_etag(&file, &AssetIdentity { len: 33, modified }),
            None,
            "a different length must miss"
        );
        assert_eq!(
            cached_asset_etag(
                &file,
                &AssetIdentity {
                    len: 32,
                    modified: modified + Duration::from_secs(1),
                }
            ),
            None,
            "a different mtime must miss"
        );
    }
}
