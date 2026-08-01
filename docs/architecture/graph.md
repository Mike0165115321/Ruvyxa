# Route Discovery & Validation · การค้นหาและตรวจสอบเส้นทาง

**Crate**: `ruvyxa_graph`  
**Module**: `crates/ruvyxa_graph/src/lib.rs`

## สรุป (Thai Summary)

`ruvyxa_graph` สแกนไดเรกทอรี `app/` เพื่อค้นหาไฟล์ page, layout, route, action, server, client
modules สร้าง RouteManifest ที่มีโครงสร้าง JSON พร้อมตรวจสอบความถูกต้อง (duplicate routes, boundary
violations, missing exports)

---

## Core Data Structures

### RouteManifest

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteManifest {
    pub app_dir: PathBuf,
    pub routes: Vec<RouteEntry>,
}
```

### RouteEntry

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteEntry {
    pub id: String,
    pub path: String,
    pub kind: RouteKind,
    pub file: PathBuf,
    pub layout_chain: Vec<String>,
    pub server_modules: Vec<String>,
    pub client_modules: Vec<String>,
    pub runtime: RuntimeTarget,
    pub render: RenderMeta,
}
```

### RouteKind

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteKind {
    Page,
    Api,
}
```

Only `page.tsx`, `page.jsx`, `page.md`, `page.mdx` → `RouteKind::Page`.  
Only `route.ts`, `route.js` → `RouteKind::Api`.

### RenderStrategy & RenderMeta

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RenderStrategy {
    #[default]
    Ssr,
    Ssg,
    Isr,
    Csr,
    Ppr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderMeta {
    pub strategy: RenderStrategy,
    pub revalidate: Option<u64>,
    pub has_static_params: bool,
    pub static_paths: Vec<String>,
    pub has_dynamic_slots: bool,
    pub hydrate: bool,
    pub hydration: HydrationMode,
}
```

### HydrationMode

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HydrationMode {
    #[default]
    Load,
    Idle,
    Visible,
    None,
}
```

### RuntimeTarget

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeTarget {
    Node,
    Edge,
    Static,
}
```

---

## File Conventions

| File pattern                                     | Effect                               |
| ------------------------------------------------ | ------------------------------------ |
| `page.tsx` / `page.jsx` / `page.md` / `page.mdx` | Page route (HTML output)             |
| `route.ts` / `route.js`                          | API route (JSON/any response)        |
| `layout.tsx`                                     | Layout wrapper (nestable)            |
| `action.ts` / `action.js`                        | Server action module                 |
| `server.ts` / `server.js`                        | Server-only module                   |
| `client.tsx`                                     | Client-only module                   |
| `(name)/`                                        | Route group (ignored in URL path)    |
| `@name/`                                         | Parallel slot (ignored in URL path)  |
| `_name/`                                         | Private directory (ignored entirely) |

## Dynamic Segments

| Pattern       | Example URL            | params                           |
| ------------- | ---------------------- | -------------------------------- |
| `[slug]`      | `/blog/hello`          | `{ slug: "hello" }`              |
| `[...rest]`   | `/docs/a/b`            | `{ rest: ["a","b"] }`            |
| `[[...rest]]` | `/shop` or `/shop/a/b` | omitted or `{ rest: ["a","b"] }` |

Validation rule RUV1002: catch-all must be last segment. Parameter names cannot contain brackets or
start with `.`.

## Route Path Resolution

`route_path_from_dir()` strips route groups `(name)` and parallel slots `@name` from the directory
path, then translates dynamic segment syntax.

```rust
// /app/(marketing)/blog/[slug]/page.tsx → /blog/[slug]
// /app/@modal/page.tsx → ignored (@-prefixed dirs filtered)
// /app/_private/page.tsx → ignored (_-prefixed dirs filtered)
```

The directory walk uses `WalkDir::filter_entry` to skip `_` and `@` prefixed directories entirely —
they never appear in the manifest.

## Layout Nesting

`layout_chain()` walks from `app_dir` to the route directory, collecting every `layout.tsx` along
the path. The root layout at `app/layout.tsx` is always first.

```rust
fn layout_chain(app_dir: &Path, route_dir: &Path) -> Vec<String> {
    // Start at app_dir, check app/layout.tsx
    // Walk each directory segment, check <segment>/layout.tsx
}
```

Layout IDs use the same `route_id()` format: `app/layout` or `app/blog/layout`.

## Rendering Strategy Detection

`detect_render_strategy()` scans page source with these rules (first match wins):

1. `"use client"` directive → CSR
2. `export const ppr = true` → PPR
3. `export const revalidate = <n>` → ISR
4. `export function getStaticParams` or `export function staticParams` → SSG
5. No dynamic segments + no data-fetching markers → SSG
6. Default → SSR

Data-fetching markers that block automatic SSG detection:

```rust
const MARKERS: &[&str] = &[
    "fetch(",
    "headers(",
    "cookies(",
    "searchParams",
    "Date.now(",
    "Math.random(",
    "process.env.",
];
```

Markers are matched against `ruvyxa_bundler::ast::masked_code()` output, which blanks strings,
template text, comments, and regex literals while preserving byte offsets and line breaks. This
crate owns no masking pass of its own — the one it used to own carried a duplicate regex-literal
rule, and a bug in that copy blanked every later `import` and env read in a module, silently
disabling RUV1007/RUV1008/RUV1010 for it. Detection also expands the reachable dependency graph
(including layouts) to determine whether any imported module introduces dynamic behavior.

## Validation

### `validate_app()` → `ValidationReport`

```rust
pub fn validate_app(root: &Path, manifest: &RouteManifest) -> Result<ValidationReport>

pub struct ValidationReport {
    pub routes: usize,
    pub page_routes: usize,
    pub api_routes: usize,
    pub client_modules: usize,
    pub server_modules: usize,
    pub diagnostics: Vec<Diagnostic>,
}
```

Diagnostics emitted:

| Code    | Title                                         | Why                                  |
| ------- | --------------------------------------------- | ------------------------------------ |
| RUV1001 | App directory not found                       | Missing `app/`                       |
| RUV1002 | Invalid dynamic route segment                 | Bad `[name]` syntax                  |
| RUV1003 | Conflicting route paths                       | Two routes with same URL match shape |
| RUV1004 | Page is missing default export                | No default export in TSX/JSX page    |
| RUV1007 | Server-only module imported into client graph | `server-only` reachable from client  |
| RUV1008 | Private env var in client graph               | `process.env.SECRET` in client code  |
| RUV1009 | Client-only module in server graph            | `client-only` in SSR bundle          |
| RUV1010 | Server directory reached by client graph      | `server/` dir in client import chain |

### Conflict Detection

`detect_conflicts()` normalizes route paths to a "match shape" — dynamic segments become `:`,
catch-alls become `*`, optional catch-alls become `*?`. Routes sharing the same shape at the same
depth produce RUV1003.

## DiscoverOptions

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverOptions {
    pub app_dir: PathBuf,
    pub default_render_strategy: Option<RenderStrategy>,
    pub default_revalidate: Option<u64>,
}
```

`with_rendering_defaults()` applies a project-wide default when the auto-detected strategy is SSR.
This lets `ruvyxa.config.ts` set `render.strategy: "ssg"` for all routes.

## Module Graph Collection

`collect_relative_graph()` performs a BFS over relative imports starting from a file, resolving
extension probing and `index.*` conventions. This feeds both the rendering strategy detector and the
boundary validator.

Import edges come from `ruvyxa_bundler::ast::parse_module()` — the same scanner the bundler resolves
its dependency graph with. This crate deliberately owns no scanner of its own: when it had one,
`check` walked a slightly different module set than `build` bundled, so a page could validate clean
and still miss a dependency in the output. The shared scanner handles:

- Static `import` statements, including declarations split across lines
- `import()` dynamic expressions and `require()` calls
- `export … from` re-exports and bare side-effect imports
- Type-only `import type` forms, which are excluded because they leave no runtime edge
- Strings, comments, regex literals, and template literals — with `${…}` interpolations scanned as
  code, so `${require("server-only")}` is still an edge

The same call answers the other two source questions this crate asks. `has_default_export` decides
whether a file is a real page (RUV1004), and `env_reads` feeds the private-env check (RUV1008); both
were previously separate walks over the same bytes.

### Edge memoization

`collect_relative_graph()` is called once per route and once per layout in each route's chain, so a
layout or shared component reachable from many routes would otherwise be read and scanned once per
route. `ModuleEdges` memoizes the resolved edges of each file across those walks.

It caches **edges, not reachable sets**. The BFS still runs per entry, so every caller receives
exactly the set it would have computed alone; only the file read and scan are shared. Caching whole
reachable sets would be wrong here — a second walk arriving at an already-visited module would
short-circuit and return a partial graph.

## Source File → URL Mapping

```
app/page.tsx                  →  /
app/about/page.tsx            →  /about
app/blog/[slug]/page.tsx      →  /blog/:slug
app/docs/[...rest]/page.tsx   →  /docs/*
app/shop/[[...cats]]/page.tsx →  /shop (or /shop/a/b)
app/api/route.ts              →  /api
app/(group)/page.tsx          →  /            (group stripped)
app/@modal/page.tsx           →  —            (slot, not a URL)
app/_private/page.tsx         →  —            (private, ignored)
```

## Write Manifest

`write_manifest()` serializes `RouteManifest` to pretty-printed JSON at the output path. The CLI
reads this manifest downstream during bundling, middleware setup, and sitemap generation.

## Why This Design

1. **Filesystem is the source of truth** — no manual route config files. Adding a file = adding a
   route.
2. **Single pass** — WalkDir + file convention matching happens in one `O(n)` scan, not multiple
   passes.
3. **Deterministic rendering detection** — source-scanning rules are ordered and explicit; there is
   no ML, no heuristics, no guesswork beyond the static analysis bounds.
4. **Early conflict detection** — route shape collisions fail at manifest build time, not at request
   time.
5. **Layering is structural** — `layout_chain` follows directory nesting automatically; no manual
   `children` wiring beyond the React component itself.
