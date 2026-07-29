# Site Discovery & Image Optimization

**Source**: `crates/ruvyxa_cli/src/{site_discovery,image_optimizer,image_usage}.rs`

Two build-time subsystems: crawler-discovery file generation (robots.txt, sitemap.xml) and public
image optimization (PNG/JPEG → WebP + responsive variants).

---

## SiteConfigOptions

The `site` block in `ruvyxa.config.ts`. Deserialized from camelCase JSON with `deny_unknown_fields`.

```rust
#[derive(Debug, Default, Clone, Deserialize)]
pub struct SiteConfigOptions {
    pub url: Option<String>,       // absolute origin, e.g. "https://ruvyxa.dev"
    pub sitemap: SitemapSetting,   // bool or SitemapGenerationOptions, default true
    pub robots: RobotsSetting,     // bool or RobotsGenerationOptions, default true
}
```

### SitemapSetting

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SitemapSetting {
    Enabled(bool),
    Options(SitemapGenerationOptions),
}
```

Defaults to `Enabled(true)`. `false` disables sitemap generation entirely;
`SitemapGenerationOptions` enables with fine-grained control.

### SitemapGenerationOptions

```rust
#[derive(Debug, Default, Clone, Deserialize)]
pub struct SitemapGenerationOptions {
    pub exclude: Vec<String>,            // exact paths or trailing-`*` prefixes
    pub additional_paths: Vec<String>,   // paths not inferable from route manifest
    pub defaults: SitemapEntryMetadata,  // metadata applied to every entry
    pub entries: Vec<SitemapEntryOptions>, // per-URL overrides
}
```

### SitemapEntryMetadata & SitemapEntryOptions

```rust
struct SitemapEntryMetadata {
    last_modified: Option<String>,           // ISO 8601 date or RFC 3339
    change_frequency: Option<SitemapChangeFrequency>,
    priority: Option<f64>,                   // 0.0–1.0
}

struct SitemapEntryOptions {
    url: String,
    last_modified: Option<String>,
    change_frequency: Option<SitemapChangeFrequency>,
    priority: Option<f64>,
    alternates: SitemapAlternates,           // BTreeMap<language, href>
    images: Vec<String>,                     // absolute image URLs
    videos: Vec<SitemapVideo>,               // Google video extension
}
```

### SitemapChangeFrequency

```rust
enum SitemapChangeFrequency {
    Always, Hourly, Daily, Weekly, Monthly, Yearly, Never,
}
```

### RobotsSetting & RobotsGenerationOptions

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RobotsSetting {
    Enabled(bool),
    Options(RobotsGenerationOptions),
}

pub struct RobotsGenerationOptions {
    rules: Option<OneOrManyRules>,   // user-agent blocks
    sitemap: Option<OneOrManyStrings>, // explicit sitemap URLs
    host: Option<String>,            // Host directive (RFC 9309)
}
```

Default rule when no options given: `User-agent: *`, `Disallow: /__ruvyxa/`, `Disallow: /api/` (if
API routes exist), `Allow: /`. A project-owned `public/robots.txt` or route at `/robots.txt`
suppresses generation.

---

## URL Resolution

`resolve_site_url()` builds the canonical site origin used in sitemap `<loc>` values. Priority:

1. **`config.site.url`** — explicit origin in config (must be `http://` or `https://`, no
   path/query/fragment)
2. **`RUVYXA_SITE_URL`** env var
3. **`VERCEL_PROJECT_PRODUCTION_URL`** env var (Vercel production)
4. **`VERCEL_URL`** env var — only when `VERCEL_ENV=production`
5. **`URL`** env var — only when `NETLIVY=true` (Netlify)

Preview/deploy URLs (Vercel preview `VERCEL_URL` without production env, Netlify deploy preview URL)
are never used as canonical sitemap origins. The function normalizes the result: lowercases scheme
and host, strips trailing slash, rejects credentials, validates DNS/IPv6, and prepends `https://` if
no scheme.

---

## Sitemap Generation

`write_discovery_files()` produces `sitemap.xml` from the route manifest.

**Route selection**: Only `RouteKind::Page` routes without dynamic segments (`[` params) are
included. Prerendered paths from the build output supplement the list.

**Constraints** (from constants):

| Constant                     | Value               |
| ---------------------------- | ------------------- |
| `SITEMAP_MAX_URLS`           | 50,000              |
| `SITEMAP_MAX_BYTES`          | 52,428,800 (50 MiB) |
| `SITEMAP_MAX_LOCATION_CHARS` | 2,048               |

**Sharding**: When entries exceed either limit, the generator splits into `sitemap-0.xml`,
`sitemap-1.xml`, etc. and writes a `sitemap.xml` sitemap index referencing each shard.

**Path encoding**: Non-ASCII and reserved characters are percent-encoded. XML special characters
(`&`, `<`, `>`, `"`, `'`) are entity-escaped.

**Rich extensions**: When entries include alternates, images, or videos, the `<urlset>` declaration
includes the corresponding XML namespace and the generator emits `<xhtml:link>`, `<image:image>`,
and `<video:video>` elements per the Google-extended sitemap protocol.

**Overwrite rule**: A project-owned `public/sitemap.xml` or a route at `/sitemap.xml` suppresses
generation. Shards never overwrite existing files — the build errors if a generated shard path
collides.

---

## Robots.txt Generation

`write_discovery_files()` writes `robots.txt` as RFC 9309 text.

**Built-in defaults** (with no explicit API routes):

```
User-agent: *
Disallow: /__ruvyxa/
Allow: /

Sitemap: https://<origin>/sitemap.xml
```

When the manifest contains API routes, the generator prepends `Disallow: /api/` before `Allow: /`.

**Custom rules** via `RobotsGenerationOptions.rules`:

- `userAgent`: one or more product tokens (`*` for all)
- `allow` / `disallow`: one or more root-relative paths (must start with `/`)
- `crawlDelay`: seconds between requests
- Multiple user-agent groups are separated by blank lines

**Sitemap directive**: Uses the auto-generated sitemap URL by default. Explicit `robots.sitemap`
entries override it. `robots.host` emits the `Host:` directive. All URLs are validated as absolute
HTTP(S).

**Overwrite rule**: Same as sitemap — existing `public/robots.txt` or `/robots.txt` route wins.

---

## ImageOptimizationOptions

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ImageOptimizationOptions {
    pub optimize: bool,                    // enable optimization, default true
    pub quality: u8,                       // 1–100, default 82
    pub lossless: bool,                    // lossless WebP encoding, default false
    pub keep_original: bool,               // keep original beside WebP, default true
    pub variant_widths: Vec<u32>,          // responsive breakpoints
    pub parallelism: usize,                // 0 = Rayon global pool
}
```

### Default variant widths

```rust
pub const DEFAULT_VARIANT_WIDTHS: [u32; 8] = [640, 750, 828, 1080, 1200, 1920, 2048, 3840];
```

Must stay identical to `DEFAULT_DEVICE_WIDTHS` in `packages/@ruvyxa/react/src/image.tsx`. Test
`tests/packages/react/image-variants.test.mjs` asserts agreement.

---

## optimize_public_images()

```rust
pub fn optimize_public_images(
    public_dir: &Path,
    assets_dir: &Path,
    cache_dir: &Path,
    options: &ImageOptimizationOptions,
) -> anyhow::Result<ImageOptimizationReport>
```

**Flow**:

1. **Discover** — Walk `public_dir` recursively, collect all files
2. **Collision check** — Detect case-insensitive output collisions (e.g. `Hero.png` + `hero.PNG`
   both → `hero.webp`). Bail with error.
3. **Optimize** — For each PNG/JPEG:
   - Decode with `image` crate
   - Encode as WebP via `webp::Encoder` (lossy or lossless)
   - Write content-addressed cache entry (blake3 hash of source + quality + lossless flag)
   - Materialize to `assets_dir` via hard link (fallback to copy)
   - If `keep_original`: copy source unchanged
   - If decode fails: copy source unchanged (never drop unoptimizable assets)
4. **Responsive variants** — For each configured width strictly smaller than intrinsic width:
   - Resize with Lanczos3, preserve aspect ratio
   - Write as `<stem>-<width>w.webp`
   - Content-addressed per source + options + target width
5. **Non-image files** — Copied unchanged (dotfiles, SVGs, fonts, etc.)
6. **Manifest** — Write `.ruvyxa-images.json` with per-entry dimensions, sizes, variants

**Parallelism**: Uses `rayon::par_iter`. Custom thread pool when `parallelism > 0`.

### ImageOptimizationReport

```rust
pub struct ImageOptimizationReport {
    pub optimized_images: usize,
    pub cache_hits: usize,
    pub source_bytes: u64,
    pub output_bytes: u64,
    pub entries: Vec<ImageManifestEntry>,
}

pub struct ImageManifestEntry {
    pub source: String,
    pub output: String,
    pub width: u32,
    pub height: u32,
    pub source_bytes: u64,
    pub output_bytes: u64,
    pub cache_hit: bool,
    pub variants: Vec<ImageVariant>,
}
```

---

## scan_raw_image_usage()

```rust
pub fn scan_raw_image_usage(
    app_dir: &Path,
    entries: &[ImageManifestEntry],
) -> Vec<RawImageUsage>
```

Scans `app_dir` source files (`.tsx`, `.jsx`, `.ts`, `.js`, `.mdx`, `.md`) for plain
`<img src="/...">` tags targeting images the optimizer already processed.

```rust
pub struct RawImageUsage {
    pub file: PathBuf,
    pub line: u32,
    pub url: String,
    pub source_bytes: u64,
    pub webp_bytes: u64,
}
```

**Filter**: Only reports when `source_bytes - webp_bytes >= 8192` (meaningful saving). Sorted
descending by saved bytes — loudest offender first.

**Scanner**: Literal `<img` tag matching (lowercase only — `<Image>` starts with capital I,
naturally excluded). Only root-relative literal `src` strings (no expressions). Only same-line `src`
(multi-line attributes skipped).

Results are warnings, never build failures — raw `<img>` is legal and sometimes deliberate.

---

## Output Structure

```
assets/
  robots.txt                 ← generated or project-owned
  sitemap.xml                ← index (single doc or shard index)
  sitemap-0.xml              ← shard when >50K URLs or >50 MiB
  logo.png                   ← original (keep_original=true)
  logo.webp                  ← full-size optimized (always)
  logo-640w.webp             ← responsive variant
  logo-750w.webp             ← responsive variant
  logo-828w.webp             ← responsive variant
  ...
  .ruvyxa-images.json        ← optimization manifest
```

---

## Under the Hood

- **Sitemap sharding**: Ratio of protocol limits — `sitemap_documents_with_limits()` shards on
  whichever limit is hit first (URL count or byte size). Each shard is a complete, valid XML
  document so CDNs serve them independently.
- **Cache invalidation**: blake3 hash covers source bytes + quality + lossless flag. Variant keys
  additionally include target width. Re-running with unchanged options hits cache entries, avoiding
  re-encoding. Cache entries are written atomically (write to `.tmp`, rename).
- **Deterministic ordering**: Sitemap entries sorted by URL (BTreeMap). Image manifest sorted by
  source path. Variant widths sorted ascending. All XML output is deterministic across builds —
  useful for CDN cache keys and diff-based deployment.
- **Case-insensitive filesystem safety**: Both `ensure_unique_outputs()` and
  `ensure_unique_originals()` fold paths to lowercase on the output side, catching collisions that
  would silently drop an image on NTFS/APFS.
- **URL normalization**: `normalize_site_origin()` and `normalize_absolute_http_url()` enforce
  strict validation: scheme must be HTTP(S), no credentials (`@`), no fragments, proper DNS labels,
  IPv6 in brackets, port in valid range. This prevents accidentally leaking staging URLs into
  production sitemaps.
