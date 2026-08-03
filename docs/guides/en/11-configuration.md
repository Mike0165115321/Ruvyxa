# Configuration Reference

Every Ruvyxa project has a `ruvyxa.config.ts` file at its root. This file is your control panel --
it tells the framework where your app lives, how to build it, how to render pages, and how to
deploy.

This chapter documents the configuration fields and validation rules currently exposed by the
repository. When a field is not listed in the source-backed section, treat it as unsupported until
the relevant type and Rust parser confirm it.

---

## What You Will Learn

- The full config object structure
- The supported fields with their TypeScript type, Rust mapping, defaults where resolved by source,
  validation, and behavior
- Configuration validation rules (RUV1600-RUV1603 in the current config path)
- Rust `ProjectConfig` struct field mapping for every field
- How to customize server, build, render, cache, image, security, middleware, and plugins
- Plugin and adapter configuration
- Validation error codes and when they trigger
- Minimal and production config examples

---

## The Config Function

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'

export default config({
  // ... your settings
})
```

The `config()` function provides autocomplete and validation. Unknown fields are rejected by the
config renderer with `RUV1602`; they are not a runtime `RUV1200` warning.

### Config Loading Flow

```
1. CLI reads --root (default: ".")
2. load_project_config(root) called
3. find_runtime_script(root, "config-renderer.mjs")
4. If not found: return ProjectConfig::default() with defaults
5. If found: spawn Node/Bun process running config-renderer.mjs
6. Config renderer evaluates ruvyxa.config.ts, outputs JSON to stdout
7. Config renderer output parsed -> ProjectConfig struct
8. validate_paths() called on the parsed config
9. Runtime override (--runtime flag) applied if present
10. dependency_hash computed for cache invalidation
```

The config renderer is run twice if the selected runtime differs from the bootstrap runtime.

---

## Full Config Type (TypeScript)

```ts
type RuvyxaConfig = {
  appDir?: string
  outDir?: string
  runtime?: 'node' | 'bun' | 'edge' | 'static'
  server?: {
    host?: string
    port?: number
  }
  site?: {
    url?: string
    sitemap?:
      | boolean
      | {
          exclude?: string[]
          additionalPaths?: string[]
          defaults?: {
            changeFrequency?:
              'always' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly' | 'never'
            priority?: number
          }
          entries?: Array<{
            url: string
            changeFrequency?: string
            priority?: number
            lastModified?: string | Date
            images?: string[]
          }>
        }
    robots?: boolean | { rules?: unknown; sitemap?: string | string[]; host?: string }
  }
  render?: {
    strategy?: 'ssr' | 'ssg' | 'isr' | 'csr' | 'ppr'
    revalidate?: number
  }
  build?: {
    minify?: boolean
    map?: boolean
    treeShake?: boolean
    split?: 'route' | 'single' | 'manual'
    workers?: number
    jsx?: 'automatic' | 'classic'
    target?: 'es2018' | 'es2019' | 'es2020' | 'es2022' | 'esnext'
    manifest?: boolean
    warm?: boolean
    prerenderCache?: boolean
  }
  cache?: {
    routes?: boolean
    css?: boolean
    dir?: string
  }
  debug?: {
    overlay?: boolean
    traces?: boolean
  }
  css?: {
    entries?: string[]
  }
  image?: {
    optimize?: boolean
    quality?: number
    lossless?: boolean
    keepOriginal?: boolean
    variantWidths?: number[]
    workers?: number
    onDemand?: boolean | { enabled?: boolean; maxWidth?: number }
  }
  i18n?: {
    locales: string[]
    defaultLocale: string
    localeParam?: string
    detectLocale?: boolean
    cookie?: string
  }
  security?: {
    actionLimit?: number
    apiLimit?: number
    pluginLimit?: number
    actionRateLimit?: { max?: number; window?: number }
    sameOrigin?: boolean
    fetchMeta?: boolean
    trustedProxyIps?: string[]
    headers?: boolean
  }
  middleware?: {
    workers?: number
    timeoutMs?: number
  }
  plugins?: RuvyxaPlugin[]
  adapter?: Adapter
  adapterOptions?: Record<string, unknown>
}
```

---

## Rust ProjectConfig Struct Mapping

The full Rust config structs live in `crates/ruvyxa_cli/src/config.rs`:

```rust
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectConfig {
    app_dir: Option<String>,
    out_dir: Option<String>,
    runtime: Option<BuildTarget>,
    #[serde(default, rename = "render")]
    rendering: RenderingConfigOptions,
    #[serde(default)]
    server: ServerConfigOptions,
    #[serde(default)]
    css: CssConfigOptions,
    #[serde(default)]
    build: BuildConfigOptions,
    #[serde(default)]
    debug: DebugConfigOptions,
    #[serde(default, rename = "image")]
    images: ImageOptimizationOptions,
    i18n: Option<I18nConfigOptions>,
    #[serde(default)]
    security: SecurityConfigOptions,
    #[serde(default)]
    cache: CacheConfigOptions,
    #[serde(default)]
    site: SiteConfigOptions,
    #[serde(default)]
    middleware: MiddlewareConfig,
    #[serde(default)]
    plugins: Vec<BuildPluginConfig>,
    #[serde(rename = "adapter")]
    adapter: Option<serde_json::Value>,
    #[serde(rename = "adapterOptions")]
    adapter_options: Option<serde_json::Value>,
    // skip fields for internal use
}
```

Key: `deny_unknown_fields` means any field not in the struct causes a deserialization error at
config load time.

---

## Common Fields

### appDir

```ts
appDir: 'app' // default
appDir: 'src/app' // custom
```

| Property             | Value                                                              |
| -------------------- | ------------------------------------------------------------------ |
| TS type              | `string`                                                           |
| Rust field           | `app_dir: Option<String>`                                          |
| Rust default         | `None` -> resolved to `"app"` by `app_dir()`                       |
| Validation           | Must be relative path (no leading `/`, no `..`), must not be empty |
| Error on empty       | `RUV1601: config field 'appDir' must not be empty`                 |
| Error on absolute    | `RUV1601: config field 'appDir' must be a project-relative path`   |
| Error on nonexistent | `RUV1001: App directory was not found` (route discovery)           |

### outDir

```ts
outDir: '.ruvyxa' // default
outDir: 'dist' // custom
```

| Property     | Value                                                        |
| ------------ | ------------------------------------------------------------ |
| TS type      | `string`                                                     |
| Rust field   | `out_dir: Option<String>`                                    |
| Rust default | `None` -> resolved to `".ruvyxa"` by `out_dir()`             |
| Validation   | Relative path project-scoped. Added to .gitignore by default |
| Error        | Same validation as appDir                                    |

### runtime

```ts
runtime: 'node' // default: auto-detect
runtime: 'bun' // use Bun runtime
runtime: 'edge' // edge-compatible build
runtime: 'static' // fully static output
```

| Property   | Value                                            |
| ---------- | ------------------------------------------------ |
| TS type    | `"node" \| "bun" \| "edge" \| "static"`          |
| Rust field | `runtime: Option<BuildTarget>`                   |
| Rust enum  | `BuildTarget { Node, Bun, Edge, Static }`        |
| Default    | auto-detected (checks system for `bun` command)  |
| Override   | CLI `--runtime` flag or `RUVYXA_RUNTIME` env var |

---

## Server Configuration

### server.host

```ts
server: {
  host: '0.0.0.0'
} // default -- all interfaces
```

| Property     | Value                               |
| ------------ | ----------------------------------- |
| TS type      | `string`                            |
| Rust field   | `host: Option<String>`              |
| Rust default | `None` -> renders as `0.0.0.0`      |
| Validation   | None (passed through to Axum/Tokio) |

### server.port

```ts
server: {
  port: 3000
} // default
server: {
  port: 8080
} // custom
```

| Property     | Value                                                                                                          |
| ------------ | -------------------------------------------------------------------------------------------------------------- |
| TS type      | `number`                                                                                                       |
| Rust field   | `port: Option<u16>`                                                                                            |
| Rust default | `None` -> renders as `3000`                                                                                    |
| Validation   | Parsed as `u16`; the framework does not impose a 1024 minimum. Binding failures are reported by the OS/server. |

---

## Site & SEO

### site.sitemap

```ts
site: {
  sitemap: {
    defaults: { changeFrequency: "weekly", priority: 0.5 },
    entries: [
      { url: "/", changeFrequency: "daily", priority: 1.0 },
    ],
  },
}
```

| Property                    | Type             | Default                | Description                   |
| --------------------------- | ---------------- | ---------------------- | ----------------------------- |
| `defaults.changeFrequency`  | enum             | `"weekly"`             | Default change frequency      |
| `defaults.priority`         | number (0.0-1.0) | `0.5`                  | Default priority              |
| `entries[].url`             | string           | --                     | Root-relative or absolute URL |
| `entries[].changeFrequency` | string           | inherits from defaults | Per-route override            |
| `entries[].priority`        | number           | inherits from defaults | Per-route override            |
| `entries[].lastModified`    | string or Date   | --                     | Last modification date        |
| `entries[].images`          | string[]         | --                     | Image URLs for sitemap:image  |

Rust: `SiteConfigOptions` in `site_discovery.rs`. Sitemap generation is built in; no plugin is
required.

Changefreq values and meanings:

| Value       | Meaning                          |
| ----------- | -------------------------------- |
| `"always"`  | Changes every request            |
| `"hourly"`  | Changes roughly every hour       |
| `"daily"`   | Changes daily                    |
| `"weekly"`  | Changes weekly                   |
| `"monthly"` | Changes monthly                  |
| `"yearly"`  | Changes yearly                   |
| `"never"`   | Archived, not expected to change |

---

## Render Configuration

### render.strategy

```ts
render: {
  strategy: 'ssr'
} // default
render: {
  strategy: 'ssg'
}
render: {
  strategy: 'isr'
}
render: {
  strategy: 'csr'
}
render: {
  strategy: 'ppr'
}
```

| Property           | Value                                                               |
| ------------------ | ------------------------------------------------------------------- |
| TS type            | `"ssr" \| "ssg" \| "isr" \| "csr" \| "ppr"`                         |
| Rust field         | `default_strategy: Option<RenderStrategy>`                          |
| Rust default       | `None` (no global default -- per-route auto-detection)              |
| Strategy used when | `RenderingConfigOptions.default_strategy` used in `DiscoverOptions` |
| Validation         | Enum parsing -- invalid values cause deserialization error          |

Strategy summary:

| Strategy | Build Behavior                    | Runtime     | Best for                          |
| -------- | --------------------------------- | ----------- | --------------------------------- |
| SSR      | Server renders on each request    | Dynamic     | Personalized content              |
| SSG      | Pre-renders to HTML at build time | Static      | Blog posts, marketing pages       |
| ISR      | SSG + revalidates on demand       | Hybrid      | Content that changes occasionally |
| CSR      | Empty shell, renders in browser   | Client-only | App-like interfaces               |
| PPR      | Partial prerender, hydrate rest   | Hybrid      | Mixed static/dynamic pages        |

### render.revalidate

```ts
render: { strategy: "isr", revalidate: 60 }
```

| Property     | Value                                  |
| ------------ | -------------------------------------- |
| TS type      | `number`                               |
| Rust field   | `default_revalidate: Option<u64>`      |
| Rust default | `None` (no revalidation)               |
| Validation   | Must be >= 0. Only meaningful for ISR. |
| Units        | Seconds                                |

---

## Build Configuration

### build.minify

```ts
build: {
  minify: true
} // default -- minify JS/CSS
build: {
  minify: false
} // skip minification
```

| Property     | Value                        |
| ------------ | ---------------------------- |
| TS type      | `boolean`                    |
| Rust field   | `minify: Option<bool>`       |
| Rust default | `None` -> treated as `true`  |
| Effect       | Controls Oxc minifier passes |

### build.map

```ts
build: {
  map: false
} // default -- no source maps
build: {
  map: true
} // include source maps
```

| Property     | Value                                          |
| ------------ | ---------------------------------------------- |
| TS type      | `boolean`                                      |
| Rust field   | `sourcemap: Option<bool>` (serde: `map`)       |
| Rust default | `None` -> treated as `false`                   |
| Note         | Source maps increase bundle size significantly |

### build.treeShake

```ts
build: {
  treeShake: true
} // default
build: {
  treeShake: false
}
```

| Property     | Value                                             |
| ------------ | ------------------------------------------------- |
| TS type      | `boolean`                                         |
| Rust field   | `tree_shaking: Option<bool>` (serde: `treeShake`) |
| Rust default | `None` -> treated as `true`                       |
| Effect       | Removes unused exports via Oxc tree shaking       |

### build.split

```ts
build: {
  split: 'route'
} // default -- one chunk per route
build: {
  split: 'single'
} // single monolithic bundle
build: {
  split: 'manual'
} // manual chunk control (same as single)
```

| Property         | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| TS type          | `"route" \| "single" \| "manual"`                                     |
| Rust field       | `split_strategy: Option<String>` (serde: `split`)                     |
| Rust default     | `None` -> resolved to `"route"` by `parse_split_strategy()`           |
| Error on invalid | `RUV1601: build.splitStrategy must be 'single', 'route', or 'manual'` |
| Rust parsing     | `parse_split_strategy()` -> `SplitStrategy::{Route, Single}`          |

Note: In TypeScript config the type is `boolean` in the original schema but the actual Rust code
accepts `"route" | "single" | "manual"`. The TypeScript config() function should accept the string
values.

### build.workers

```ts
build: {
  workers: 4
} // custom
build: {
  workers: 0
} // auto-detect (CPU count)
```

| Property     | Value                                                                                     |
| ------------ | ----------------------------------------------------------------------------------------- |
| TS type      | `number`                                                                                  |
| Rust field   | `parallelism: Option<usize>` (serde: `workers`)                                           |
| Rust default | `None` -> treated as `0` (auto)                                                           |
| Validation   | Must be 0 or positive integer                                                             |
| Error        | `RUV1601: config field 'build.workers' must be greater than zero` (if 0 allowed for auto) |

### build.jsx

```ts
build: {
  jsx: 'automatic'
} // default -- React 17+ JSX runtime
build: {
  jsx: 'classic'
} // classic React.createElement
```

| Property         | Value                                                        |
| ---------------- | ------------------------------------------------------------ |
| TS type          | `"automatic" \| "classic"`                                   |
| Rust field       | `jsx_runtime: Option<String>` (serde: `jsx`)                 |
| Rust default     | `None` -> resolved to `"automatic"`                          |
| Error on invalid | `RUV1601: build.jsxRuntime must be 'classic' or 'automatic'` |
| Rust parsing     | `parse_jsx_runtime()` -> `JsxRuntime::{Automatic, Classic}`  |

### build.target

```ts
build: {
  target: 'es2022'
} // default
build: {
  target: 'es2018'
} // broader browser support
build: {
  target: 'esnext'
} // latest features
```

| Property         | Value                                                                       |
| ---------------- | --------------------------------------------------------------------------- |
| TS type          | `"es2018" \| "es2019" \| "es2020" \| "es2022" \| "esnext"`                  |
| Rust field       | `es_target: Option<String>` (serde: `target`)                               |
| Rust default     | `None` -> resolved to `"es2022"`                                            |
| Error on invalid | `RUV1601: build.esTarget must be es2018, es2019, es2020, es2022, or esnext` |
| Rust parsing     | `parse_es_target()` -> `EsTarget::{Es2018, Es2019, Es2020, Es2022, EsNext}` |

| Target   | Supported browsers     | Features                              |
| -------- | ---------------------- | ------------------------------------- |
| `es2018` | Older browsers (2018+) | Async generators, rest/spread         |
| `es2019` | Modern-ish (2019+)     | Array.flat, Object.fromEntries        |
| `es2020` | Modern (2020+)         | Optional chaining, nullish coalescing |
| `es2022` | Bleeding-edge (2022+)  | Class fields, top-level await         |
| `esnext` | Latest spec            | Unstable proposals may be included    |

### build.manifest

```ts
build: {
  manifest: true
} // generate manifest.json in outDir
```

| Property     | Value                                                   |
| ------------ | ------------------------------------------------------- |
| TS type      | `boolean`                                               |
| Rust field   | `emit_chunk_manifest: Option<bool>` (serde: `manifest`) |
| Rust default | `None` -> treated as `true`                             |
| Effect       | Writes `.ruvyxa/manifest.json` with chunk metadata      |

### build.warm

```ts
build: {
  warm: false
} // default
build: {
  warm: true
} // pre-warm render cache on server start
```

| Property     | Value                                                  |
| ------------ | ------------------------------------------------------ |
| TS type      | `boolean`                                              |
| Rust field   | `prebundle_dependencies: Option<bool>` (serde: `warm`) |
| Rust default | `None` -> treated as `false`                           |
| Effect       | Pre-warms render cache to avoid cold starts            |

### build.prerenderCache

```ts
build: {
  prerenderCache: true
} // default
```

| Property     | Value                                                     |
| ------------ | --------------------------------------------------------- |
| TS type      | `boolean`                                                 |
| Rust field   | `prerender_cache: Option<bool>` (serde: `prerenderCache`) |
| Rust default | `None` -> treated as `true`                               |
| Effect       | Caches prerendered pages to disk for instant serving      |

---

## Cache Configuration

### cache.routes

```ts
cache: {
  routes: true
} // cache route manifest
```

| Property     | Value                                            |
| ------------ | ------------------------------------------------ |
| TS type      | `boolean`                                        |
| Rust field   | `route_manifest: Option<bool>` (serde: `routes`) |
| Rust default | `None` -> treated as `true`                      |
| Effect       | Caches route discovery results                   |

### cache.css

```ts
cache: {
  css: true
} // cache CSS compilation
```

| Property     | Value                       |
| ------------ | --------------------------- |
| TS type      | `boolean`                   |
| Rust field   | `css: Option<bool>`         |
| Rust default | `None` -> treated as `true` |
| Effect       | Caches compiled CSS output  |

### cache.dir

```ts
cache: {
  dir: '.cache'
} // default
```

| Property     | Value                                      |
| ------------ | ------------------------------------------ |
| TS type      | `string`                                   |
| Rust field   | `build_dir: Option<String>` (serde: `dir`) |
| Rust default | `None` -> resolved to `".cache"`           |
| Validation   | Must be relative path                      |

---

## Debug Configuration

### debug.overlay

```ts
debug: {
  overlay: true
} // default -- show error overlay in browser
```

| Property     | Value                                            |
| ------------ | ------------------------------------------------ |
| TS type      | `boolean`                                        |
| Rust field   | `overlay: Option<bool>`                          |
| Rust default | `None` -> treated as `true`                      |
| Effect       | Shows build errors in-browser overlay during dev |

### debug.traces

```ts
debug: {
  traces: false
} // default
debug: {
  traces: true
} // output performance traces
```

| Property     | Value                                       |
| ------------ | ------------------------------------------- |
| TS type      | `boolean`                                   |
| Rust field   | `traces: Option<bool>`                      |
| Rust default | `None` -> treated as `false`                |
| Effect       | Enables detailed performance tracing output |

---

## CSS Configuration

### css.entries

```ts
css: {
  entries: ["./styles/global.css", "./styles/fonts.css"],
}
```

| Property          | Value                                                                 |
| ----------------- | --------------------------------------------------------------------- |
| TS type           | `string[]`                                                            |
| Rust field        | `entries: Vec<String>`                                                |
| Rust default      | `[]` (empty vec)                                                      |
| Validation        | Each path must be relative, project-scoped, must exist on disk        |
| Error             | `RUV1601: config field 'css.entries' must be a project-relative path` |
| Error (not found) | `RUV1403: Configured CSS entry was not found`                         |

Use for stylesheets loaded on every page but not explicitly imported by any component.

---

## Image Configuration

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageOptimizationOptions {
    pub optimize: bool,
    pub quality: u8,
    pub lossless: bool,
    pub keep_original: bool,
    pub variant_widths: Vec<u32>,
    pub parallelism: usize,
}
```

### image.optimize

```ts
image: {
  optimize: true
} // default
```

| Property   | Value                                               |
| ---------- | --------------------------------------------------- |
| TS type    | `boolean`                                           |
| Rust field | `optimize: bool`                                    |
| Default    | `true`                                              |
| Effect     | When true, PNG/JPEG converted to WebP at build time |

### image.quality

```ts
image: {
  quality: 82
} // default
image: {
  quality: 95
} // high quality
```

| Property   | Value                            |
| ---------- | -------------------------------- |
| TS type    | `number`                         |
| Rust field | `quality: u8`                    |
| Default    | `82`                             |
| Validation | Range 1-100 (clamped by encoder) |

### image.lossless

```ts
image: {
  lossless: false
} // default -- lossy WebP
image: {
  lossless: true
} // lossless WebP (larger files)
```

| Property   | Value            |
| ---------- | ---------------- |
| TS type    | `boolean`        |
| Rust field | `lossless: bool` |
| Default    | `false`          |

### image.keepOriginal

```ts
image: {
  keepOriginal: true
} // default
```

| Property   | Value                                                                                   |
| ---------- | --------------------------------------------------------------------------------------- |
| TS type    | `boolean`                                                                               |
| Rust field | `keep_original: bool`                                                                   |
| Default    | `true`                                                                                  |
| Effect     | When true, original PNG/JPEG copied beside WebP output. Critical for CDN compatibility. |

### image.variantWidths

```ts
image: {
  variantWidths: [640, 750, 828, 1080, 1200, 1920, 2048, 3840]
}
```

| Property   | Value                                                           |
| ---------- | --------------------------------------------------------------- |
| TS type    | `number[]`                                                      |
| Rust field | `variant_widths: Vec<u32>`                                      |
| Default    | `[640, 750, 828, 1080, 1200, 1920, 2048, 3840]`                 |
| Must match | `packages/@ruvyxa/react/src/image.tsx` (test asserts agreement) |

### image.workers

```ts
image: {
  workers: 0
} // default -- Rayon global pool
image: {
  workers: 4
} // dedicated thread pool
```

| Property   | Value                                                      |
| ---------- | ---------------------------------------------------------- |
| TS type    | `number`                                                   |
| Rust field | `parallelism: usize` (serde: `workers`)                    |
| Default    | `0` (Rayon global worker pool)                             |
| Behavior   | 0 = global pool; >0 = dedicated `rayon::ThreadPoolBuilder` |

---

## Security Configuration

```rust
struct SecurityConfigOptions {
    action_body_limit_bytes: Option<usize>,       // actionLimit
    api_body_limit_bytes: Option<usize>,           // apiLimit
    plugin_response_body_limit_bytes: Option<usize>, // pluginLimit
    action_rate_limit: Option<ActionRateLimitOptions>,
    same_origin_actions: Option<bool>,             // sameOrigin
    fetch_metadata_actions: Option<bool>,           // fetchMeta
    trusted_proxy_ips: Vec<String>,
    security_headers: Option<bool>,                 // headers
}

struct ActionRateLimitOptions {
    max: Option<usize>,
    window: Option<u64>,  // seconds
}
```

### security.actionLimit

```ts
security: {
  actionLimit: 1048576
} // default -- 1 MB
```

| Property          | Value                                                                    |
| ----------------- | ------------------------------------------------------------------------ |
| TS type           | `number`                                                                 |
| Rust field        | `action_body_limit_bytes: Option<usize>`                                 |
| Default           | not specified in code (validated by `validate_bounded_limit`)            |
| Validation        | Must be > 0, max `MAX_ACTION_BODY_LIMIT_BYTES` (16,777,216 = 16 MiB)     |
| Error on 0        | `RUV1601: config field 'security.actionLimit' must be greater than zero` |
| Error on too high | `RUV1602: config field 'security.actionLimit' must not exceed <max>`     |

### security.apiLimit

```ts
security: {
  apiLimit: 10485760
} // default -- 10 MB
```

| Property   | Value                                       |
| ---------- | ------------------------------------------- |
| TS type    | `number`                                    |
| Rust field | `api_body_limit_bytes: Option<usize>`       |
| Validation | Must be > 0, max `MAX_API_BODY_LIMIT_BYTES` |
| Error      | RUV1601 (zero) / RUV1602 (exceeds max)      |

### security.pluginLimit

```ts
security: {
  pluginLimit: 33554432
} // default -- 32 MB
```

| Property          | Value                                                                          |
| ----------------- | ------------------------------------------------------------------------------ |
| TS type           | `number`                                                                       |
| Rust field        | `plugin_response_body_limit_bytes: Option<usize>`                              |
| Validation        | Must be > 0, max `MAX_PLUGIN_RESPONSE_BODY_LIMIT_BYTES` (268,435,456 = 256 MB) |
| Error on 0        | `RUV1601: config field 'security.pluginLimit' must be greater than zero`       |
| Error on too high | `RUV1602: config field 'security.pluginLimit' must not exceed 268435456 bytes` |

### security.actionRateLimit

```ts
security: {
  actionRateLimit: {
    max: 100,      // requests
    window: 60,    // seconds
  },
}
```

| Property | Type               | Default | Validation  |
| -------- | ------------------ | ------- | ----------- |
| `max`    | `number`           | varies  | Must be > 0 |
| `window` | `number` (seconds) | varies  | Must be > 0 |
| Error    | RUV1601 (zero)     |         |             |

### security.sameOrigin

```ts
security: {
  sameOrigin: true
} // default -- reject cross-origin actions
```

| Property   | Value                                       |
| ---------- | ------------------------------------------- |
| TS type    | `boolean`                                   |
| Rust field | `same_origin_actions: Option<bool>`         |
| Default    | `None` -> treated as `true`                 |
| Effect     | Server actions reject cross-origin requests |

### security.fetchMeta

```ts
security: {
  fetchMeta: true
} // default
```

| Property   | Value                                               |
| ---------- | --------------------------------------------------- |
| TS type    | `boolean`                                           |
| Rust field | `fetch_metadata_actions: Option<bool>`              |
| Default    | `None` -> treated as `true`                         |
| Effect     | Include rendering environment metadata in responses |

### security.trustedProxyIps

```ts
security: {
  // Exact addresses, CIDR ranges, or a mix of both.
  trustedProxyIps: ["10.0.0.1", "172.16.0.0/12", "2001:db8::/32"],
}
```

| Property   | Value                                                                                |
| ---------- | ------------------------------------------------------------------------------------ |
| TS type    | `string[]`                                                                           |
| Rust field | `trusted_proxy_ips: Vec<String>`                                                     |
| Default    | `[]` (empty)                                                                         |
| Validation | Each entry is an IPv4/IPv6 address or a CIDR range                                   |
| Error      | `RUV1602: config field 'security.trustedProxyIps' contains invalid IP or CIDR range` |

An entry without a `/` is treated as a host route (`/32` for IPv4, `/128` for IPv6). Host bits below
the prefix are masked, so `10.1.2.3/8` and `10.0.0.0/8` describe the same range. An IPv4 range also
matches an IPv4-mapped peer (`::ffff:10.0.0.9`), which is how a dual-stack listener reports IPv4
clients.

Loopback is always trusted and does not need to be listed.

### security.headers

```ts
security: {
  headers: true
} // default -- enable security headers
```

| Property   | Value                                                                                                  |
| ---------- | ------------------------------------------------------------------------------------------------------ |
| TS type    | `boolean`                                                                                              |
| Rust field | `security_headers: Option<bool>`                                                                       |
| Default    | `None` -> treated as `true`                                                                            |
| Effect     | When true, applies default security headers (X-Frame-Options, X-Content-Type-Options, Referrer-Policy) |

---

## Middleware Configuration

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MiddlewareConfig {
    pub workers: Option<usize>,
    pub timeout_ms: Option<u64>,
}
```

### middleware.workers

```ts
middleware: {
  workers: 2
} // default
```

| Property   | Value                                                                    |
| ---------- | ------------------------------------------------------------------------ |
| TS type    | `number`                                                                 |
| Rust field | `workers: Option<usize>`                                                 |
| Default    | `2`                                                                      |
| Validation | Must be between 1 and `MAX_MIDDLEWARE_WORKERS`                           |
| Error      | `RUV1602: config field 'middleware.workers' must be between 1 and <max>` |

### middleware.timeoutMs

```ts
middleware: {
  timeoutMs: 5000
} // default -- 5 seconds
```

| Property   | Value                                                                      |
| ---------- | -------------------------------------------------------------------------- |
| TS type    | `number`                                                                   |
| Rust field | `timeout_ms: Option<u64>`                                                  |
| Validation | Must be between 1 and `MAX_MIDDLEWARE_TIMEOUT_MS`                          |
| Error      | `RUV1602: config field 'middleware.timeoutMs' must be between 1 and <max>` |

---

## Plugin Configuration

### plugins

```ts
import { definePlugin } from 'ruvyxa/plugin'

plugins: [definePlugin({ name: 'require-env', build: { onComplete() {} } })]
```

| Property     | Value                                 |
| ------------ | ------------------------------------- |
| TS type      | `RuvyxaPlugin[]`                      |
| Rust field   | `plugins: Vec<BuildPluginConfig>`     |
| Runtime rule | Every plugin provides `register(api)` |
| Default      | `[]`                                  |

Plugin `head` entries contribute elements to every rendered document's `<head>`:

```ts
head: [
  { tag: 'meta', attrs: { name: 'theme-color', content: '#000' } },
  { tag: 'script', content: "console.log('hello')" },
]
```

---

## Adapter Configuration

### adapter

`config.adapter` is an `Adapter` object. It is not a string field in `ruvyxa.config.ts`; the object
must expose `name`, `target`, and `build(context)`. The current renderer reports `RUV1603` when this
contract or the returned adapter output is invalid.

```ts
import { config } from 'ruvyxa/config'
import { vercelAdapter } from '@ruvyxa/adapter-vercel'

export default config({
  adapter: vercelAdapter({ regions: ['iad1'] }),
})
```

For a named built-in or installed third-party adapter, select it at the command line instead of
putting a string in the config object:

```bash
npm run doctor -- --adapter vercel
npm run build -- --adapter vercel
npm run build -- --adapter @scope/ruvyxa-adapter-node
```

When neither the config object nor `--adapter` selects an adapter, the CLI may use `RUVYXA_ADAPTER`
or recognized hosting-platform environment variables. This is selection behavior, not a TypeScript
type for `config.adapter`.

| Property        | Value                                                                                              |
| --------------- | -------------------------------------------------------------------------------------------------- |
| TS type         | `Adapter`                                                                                          |
| Rust field      | serialized adapter output in `adapter: Option<serde_json::Value>`                                  |
| Config contract | object with `build(context)`; output includes string `name` and `target`                           |
| Named selection | `--adapter <name>` or an installed adapter package                                                 |
| Known names     | `node`, `bun`, `static`, `vercel`, `netlify`, `cloudflare`, `railway`, `render`, `firebase`, `aws` |

Third-party adapter names supplied through the CLI must be valid npm package names and resolvable
from the project or Ruvyxa runtime.

### adapterOptions

```ts
import { config } from 'ruvyxa/config'

export default config({
  adapterOptions: { regions: ['iad1', 'hnd1'], imageOptimization: true },
})
```

| Property   | Value                                        |
| ---------- | -------------------------------------------- |
| TS type    | `Record<string, unknown>`                    |
| Rust field | `adapter_options: Option<serde_json::Value>` |
| Default    | `None`                                       |

Per-adapter options passed through to the adapter runner.

---

## Configuration Validation

## Validation Rules — Complete Reference (Rust)

### Error Codes Used by the Current Config Path

The current config path emits `RUV1600` for config-loading failure, `RUV1601` for invalid values,
`RUV1602` for invalid shape, unknown fields, and limit violations, and `RUV1603` for adapter
definition/output failures. Do not infer that every number in the `RUV1600`-`RUV1699` range is
implemented.

| Code    | Condition                  | Field                                       | Solution                  |
| ------- | -------------------------- | ------------------------------------------- | ------------------------- |
| RUV1601 | Invalid value              | Multiple fields                             | Check allowed values      |
| RUV1602 | Value out of range         | Multiple fields                             | Adjust value within range |
| RUV1602 | Unknown field              | A config object contains an unsupported key | Check camelCase spelling  |
| RUV1603 | Invalid config structure   | Adapter definition or output is invalid     | Fix the adapter contract  |
| RUV1603 | Invalid adapter definition | Adapter missing valid `build(context)`      | Fix adapter contract      |

### Validation Matrix

| Config Field                         | RUV1601     | RUV1602       | Notes                              |
| ------------------------------------ | ----------- | ------------- | ---------------------------------- |
| `appDir` empty/absolute              | ✅          | -             | relative path required             |
| `outDir` empty/absolute              | ✅          | -             | relative path required             |
| `server.port` outside `u16`          | ✅          | -             | Parsed as an unsigned 16-bit port  |
| `server.host` invalid                | -           | ✅            | valid hostname/IP                  |
| `site.url` invalid                   | -           | ✅            | origin only                        |
| `site.sitemap.defaults.priority`     | -           | ✅ (0-1)      | float                              |
| `build.workers` 0                    | ✅          | -             | must be greater than zero when set |
| `build.split` invalid                | ✅          | -             | single/route/manual                |
| `build.jsx` invalid                  | ✅          | -             | automatic/classic                  |
| `build.target` invalid               | ✅          | -             | es2018-esnext                      |
| `security.actionLimit` 0             | ✅          | ✅ (>16 MiB)  | 1B-16 MiB                          |
| `security.apiLimit` 0                | ✅          | ✅ (>256 MiB) | 1B-256 MiB                         |
| `security.pluginLimit` 0             | ✅          | ✅ (>256 MiB) | 1B-256 MiB                         |
| `security.trustedProxyIps[]` invalid | -           | ✅            | valid IP/CIDR                      |
| `security.actionRateLimit.max` 0     | ✅          | -             | ≥ 1                                |
| `security.actionRateLimit.window` 0  | ✅          | -             | ≥ 1                                |
| `middleware.workers` 0               | ✅          | ✅ (>8)       | 1-8                                |
| `middleware.timeoutMs` 0             | ✅          | ✅ (>300s)    | 1ms-300s                           |
| `image.quality` out of range         | ✅ (0/100+) | -             | WebP quality, 1-100                |
| `image.optimize` invalid             | ✅          | -             | boolean                            |
| `image.lossless` invalid             | ✅          | -             | boolean                            |
| `image.keepOriginal` invalid         | ✅          | -             | boolean                            |
| `image.variantWidths` invalid        | ✅          | -             | number array                       |
| `image.workers` invalid              | ✅          | -             | number                             |
| `css.entries[]` absolute             | ✅          | -             | relative path                      |
| `cache.buildDir` absolute            | ✅          | -             | relative path                      |
| `adapter` is not an Adapter object   | ✅          | -             | Provide `build(context)`           |
| `plugins[].name` empty/duplicate     | ✅          | -             | unique, non-empty                  |

---

Ruvyxa validates your config at startup via `validate_paths()`.

### Validation Functions

```rust
fn validate_paths(&self) -> anyhow::Result<()> {
    validate_project_relative_path("appDir", self.app_dir())?;
    validate_project_relative_path("outDir", self.out_dir())?;
    for entry in &self.css.entries {
        validate_project_relative_path("css.entries", entry)?;
    }
    validate_bounded_limit("security.actionLimit", ...)?;
    validate_bounded_limit("security.apiLimit", ...)?;
    validate_plugin_response_limit(self.security.plugin_response_body_limit_bytes)?;
    if let Some(rate_limit) = &self.security.action_rate_limit {
        validate_bounded_limit("security.actionRateLimit.max", rate_limit.max, ...)?;
        validate_bounded_limit("security.actionRateLimit.window", rate_limit.window, ...)?;
    }
    validate_trusted_proxy_ips(&self.security.trusted_proxy_ips)?;
    parse_jsx_runtime(self.build.jsx_runtime.as_deref())?;
    Ok(())
}
```

### Validation Error Codes

| Error Code | Condition                                       | Example                                                                       |
| ---------- | ----------------------------------------------- | ----------------------------------------------------------------------------- |
| RUV1600    | Config load failure (generic)                   | `config load failed: RUV1600 syntax error`                                    |
| RUV1601    | Field value invalid (zero, empty, unknown enum) | `RUV1601 config field 'appDir' must not be empty`                             |
| RUV1602    | Field value exceeds maximum                     | `RUV1602 config field 'security.pluginLimit' must not exceed 268435456 bytes` |

### Field Validation Table

| Field                             | Rules                                  | Error Code         | Example Error                                                      |
| --------------------------------- | -------------------------------------- | ------------------ | ------------------------------------------------------------------ |
| `appDir`                          | Must exist, relative, non-empty        | RUV1601            | `RUV1601: appDir must be a project-relative path`                  |
| `outDir`                          | Must be relative, non-empty            | RUV1601            | `RUV1601: outDir must be a project-relative path`                  |
| `css.entries`                     | Each path relative, must exist         | RUV1601 / RUV1403  | `RUV1403: Configured CSS entry was not found`                      |
| `server.port`                     | Unsigned 16-bit value                  | Rust/OS bind       | Binding errors depend on the selected host and OS                  |
| `image.quality`                   | 1-100                                  | Clamped by encoder | (no error, clamped)                                                |
| `build.workers`                   | 0 or positive integer                  | RUV1601            | `build.workers must be greater than zero`                          |
| `build.jsx`                       | `"automatic"` or `"classic"`           | RUV1601            | `build.jsxRuntime must be 'classic' or 'automatic'`                |
| `build.target`                    | es2018, es2019, es2020, es2022, esnext | RUV1601            | `build.esTarget must be es2018, es2019, es2020, es2022, or esnext` |
| `build.split`                     | `"single"`, `"route"`, or `"manual"`   | RUV1601            | `build.splitStrategy must be 'single', 'route', or 'manual'`       |
| `security.actionLimit`            | > 0, max 16777216                      | RUV1601/1602       | `security.actionLimit must not exceed 16777216`                    |
| `security.apiLimit`               | > 0, max 268435456                     | RUV1601/1602       | `security.apiLimit must not exceed 268435456`                      |
| `security.pluginLimit`            | > 0, max 268435456                     | RUV1601/1602       | `security.pluginLimit must not exceed 268435456`                   |
| `security.actionRateLimit.max`    | > 0                                    | RUV1601            | `security.actionRateLimit.max must be greater than zero`           |
| `security.actionRateLimit.window` | > 0                                    | RUV1601            | `security.actionRateLimit.window must be greater than zero`        |
| `security.trustedProxyIps`        | IPv4/IPv6 address or CIDR range        | RUV1602            | `RUV1602: trustedProxyIps contains invalid IP or CIDR range`       |
| `middleware.workers`              | 1-MAX                                  | RUV1602            | `middleware.workers must be between 1 and <max>`                   |
| `middleware.timeoutMs`            | 1-MAX                                  | RUV1602            | `middleware.timeoutMs must be between 1 and <max>`                 |
| Unknown field                     | Not in struct                          | RUV1602            | `unknown config field "unknownField"`                              |
| `appDir` not found                | Directory missing                      | RUV1001            | `App directory was not found`                                      |

---

## Minimal Complete Config

```ts
import { config } from 'ruvyxa/config'

export default config({
  appDir: 'app',
  outDir: '.ruvyxa',
  server: { host: '0.0.0.0', port: 3000 },
  build: { minify: true, map: false, treeShake: true, split: 'route' },
  render: { strategy: 'ssr' },
  image: { optimize: true, quality: 82 },
  security: { actionLimit: 1048576, apiLimit: 10485760, sameOrigin: true },
  plugins: [],
})
```

---

## Full Production Config

```ts
import { config } from 'ruvyxa/config'

export default config({
  appDir: 'app',
  outDir: '.ruvyxa',
  server: { host: '0.0.0.0', port: 3000 },
  site: {
    sitemap: {
      defaults: { changeFrequency: 'weekly', priority: 0.5 },
      entries: [{ url: '/', changeFrequency: 'daily', priority: 1.0 }],
    },
  },
  build: {
    minify: true,
    map: false,
    treeShake: true,
    split: 'route',
    workers: 4,
    jsx: 'automatic',
    target: 'es2022',
    manifest: true,
    warm: true,
    prerenderCache: true,
  },
  render: { strategy: 'ssr', revalidate: 60 },
  cache: { routes: true, css: true, dir: '.cache' },
  debug: { overlay: false, traces: false },
  css: { entries: ['./styles/global.css'] },
  image: { optimize: true, quality: 82, lossless: false, keepOriginal: true, workers: 4 },
  security: {
    actionLimit: 1048576,
    apiLimit: 10485760,
    pluginLimit: 33554432,
    actionRateLimit: { max: 100, window: 60 },
    sameOrigin: true,
    fetchMeta: true,
    trustedProxyIps: [],
    headers: true,
  },
  middleware: { workers: 2 },
  plugins: [
    { name: 'sitemap' },
    { name: 'robots' },
    { name: 'securityHeaders' },
    { name: 'requireEnv', options: { variables: ['DATABASE_URL'] } },
  ],
  // Select a named adapter with: npm run build -- --adapter vercel
  adapterOptions: { regions: ['iad1'] },
})
```

---

## Doctor Command

Run `npm run doctor` to validate your config against all rules:

```
> npm run doctor

  Config valid
  Port 3000 available
  OutDir .ruvyxa/ writable
  TypeScript config found
  .env file present
```

---

## Try It Yourself

1. **Basic Config**
   - Open `ruvyxa.config.ts`
   - Change `server.port` to 4000 → `npm run dev`

2. **Security**
   - Set `security.sameOrigin: true`
   - Set `security.actionLimit: 2_097_152`
   - Try sending a request larger than the limit

3. **Image**
   - Set `image.quality`, `image.lossless`, or `image.variantWidths`
   - Run `npm run build` and inspect the generated WebP assets

4. **Middleware**
   - Enable CORS with origins `['https://example.com']`
   - Test it from a different origin

5. **Plugin**
   - Add the `redirects` plugin → redirect `/old` → `/new`
   - Add the `requireEnv` plugin → require `DATABASE_URL`

6. **Inspection**
   - `npm run doctor` — View validation results
   - `npm run doctor -- --json` — View JSON output

---

## Summary

- `ruvyxa.config.ts` is the central configuration file
- `config()` provides type safety and auto-completion
- Current config errors use RUV1600-RUV1603 as described in the validation section
- Adapters can auto-detect from platform environment variables
- 16 built-in plugin builders are exported from `ruvyxa/plugins` in this release
- Defaults and validation are documented only where the current type, renderer, or Rust parser
  defines them
- Backed by robust Rust validation
- Use `npm run doctor` to inspect everything

---

## Configuration as a Typed Contract

The public contract is `RuvyxaConfig` from `@ruvyxa/core`, normally authored through `config()` from
`ruvyxa/config`. Its top-level fields are `appDir`, `outDir`, `runtime`, `react`, `typescript`,
`css`, `server`, `build`, `render`, `debug`, `image`, `security`, `cache`, `site`, `middleware`,
`adapter`, `adapterOptions`, and `plugins`. Keep configuration narrow: omit fields whose defaults
match the application instead of copying a speculative "full production" object.

```ts
import { config, type RuvyxaConfig } from 'ruvyxa/config'

const settings: RuvyxaConfig = {
  server: { host: 'localhost', port: 3000 },
  build: { minify: true, split: 'route', workers: 4 },
  render: { strategy: 'ssr' },
  css: { entries: ['styles/print.css'] },
}

export default config(settings)
```

`appDir`, `outDir`, and every `css.entries` value are project-relative paths. The CLI rejects an
empty, absolute, or escaping path rather than resolving it outside the project. This is a safety
boundary: use a relative directory inside the application rather than an operating-system-specific
absolute path.

### Precedence Is Per Input, Not a Global Override

For commands that accept `--runtime`, the command-line value wins over `RUVYXA_RUNTIME` and
`config.runtime`. For `dev`, `start`, and `preview`, command-line `--host` and `--port` override the
corresponding server configuration. Build target and adapter have their own CLI overrides. Do not
generalize that precedence to unrelated config fields.

```bash
npm run dev -- --port 4000 --runtime bun
npm run build -- --target static --adapter static
npm run doctor -- --adapter cloudflare --json
```

### Validate Before Deploying

Use `doctor` to inspect configuration/runtime/adapter compatibility and `analyze` for route/import
boundaries. Their concerns differ, so a successful one does not replace the other:

```bash
npm run doctor
npm run analyze -- --format human
npm run check
```

Configuration validation enforces positive bounded limits such as action/API payload limits and
valid trusted-proxy IP/CIDR values. If a config value is rejected, correct the field rather than
adding an undocumented environment override.

---

## Next Steps

- [12-cli-commands.md](./12-cli-commands.md) -- CLI commands that consume this config
- [13-deployment.md](./13-deployment.md) -- Deploy with adapter config
- [14-plugins.md](./14-plugins.md) -- Configure plugins
- [16-error-handling.md](./16-error-handling.md) -- Config error codes (RUV1600-RUV1603)

## Diagnostics: Configuration Validation

The `ruvyxa.config.ts` file is rendered to JSON and then parsed/validated by the CLI. The Rust
config structs use `deny_unknown_fields`; an unsupported field such as `experimentalDocker` is
rejected with the current config-renderer diagnostic (`RUV1602`). This prevents a typo from being
silently treated as a supported setting.

# Advanced routing and image config

```ts
export default config({
  i18n: {
    locales: ['en', 'th'],
    defaultLocale: 'en',
    localeParam: 'lang',
    detectLocale: true,
    cookie: 'RUVYXA_LOCALE',
  },
  image: {
    onDemand: { enabled: true, maxWidth: 3840 },
  },
})
```

Locale identifiers are validated, case-insensitive duplicates are rejected, and `defaultLocale` must
be present in `locales`. `localeParam` must be a JavaScript identifier. On-demand image width must
be between 16 and 8192 pixels; the endpoint remains disabled by default.
