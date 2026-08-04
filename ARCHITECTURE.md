# Ruvyxa Framework: Comprehensive System Architecture

> This document is the definitive, deep-dive architectural manual for the Ruvyxa Framework. It
> synthesizes all subsystem documentation into a single, cohesive master reference.

## Table of Contents

- [Ruvyxa System Overview](#ruvyxa-system-overview)
- [CLI Architecture](#cli-architecture)
- [Route Discovery & Validation · การค้นหาและตรวจสอบเส้นทาง](#route-discovery-validation-)
- [Bundler · การรวมโค้ด](#bundler-)
- [Dev Server](#dev-server)
- [Middleware](#middleware)
- [Worker Pool · กลุ่มผู้ทำงาน](#worker-pool-)
- [Concurrency Model · โมเดลการทำงานพร้อมกัน](#concurrency-model-)
- [Protocols · โพรโทคอล](#protocols-)
- [Site Discovery & Image Optimization](#site-discovery-image-optimization)
- [Diagnostics · การวินิจฉัย](#diagnostics-)
- [Security · ความปลอดภัย](#security-)
- [Deployment Adapters · อาดาปเตอร์สำหรับการปรับใช้](#deployment-adapters-)

---

## Ruvyxa System Overview

**Philosophy**: Rust before render (route discovery, bundling, minification, serving). JS runtime
(Node/Bun) during render (SSR, SSG, API, config). This gives Rust speed + JS ecosystem.

```
┌──────────────────────────────────────────────────────────────────┐
│                        ruvyxa_cli                               │
│   (clap CLI dispatch · config loading · build orchestration)     │
├──────────┬───────────┬──────────────┬──────────┬────────────────┤
│ruvyxa_   │ruvyxa_    │ruvyxa_dev_   │ruvyxa_   │ruvyxa_         │
│graph     │bundler    │server        │middleware│diagnostics      │
│(route    │(TS/JSX    │(Axum + HMR + │(Tower    │(RUV#### codes)  │
│disc+val) │comp+link) │router+cache) │+host)    │                 │
└────┬─────┴─────┬─────┴──────┬───────┴────┬─────┴────────┬───────┘
     │           │            │            │              │
     └───────────┴────────────┴────────────┴──────────────┘
                               │
                    ┌──────────▼──────────┐
                    │  Node / Bun Workers  │
                    │  (SSR, SSG, API,     │
                    │   Action, Config)    │
                    └─────────────────────┘
```

---

### Crate Dependency Graph

```
ruvyxa_diagnostics          (serde + thiserror — nothing else)
    ↑
    ├── ruvyxa_graph        (route discovery, validation, manifest)
    ├── ruvyxa_bundler      (Oxc compiler, resolver, linker, minifier, CSS modules)
    ├── ruvyxa_middleware   (Tower middleware, plugin bridge)
    └── ruvyxa_dev_server   (Axum serving, HMR, cache, worker pool, router)
         │
         └── ruvyxa_cli     (depends ALL crates — binary entry via clap)
```

---

### Key Design Decisions

| Decision                               | Why                                                                                                                   |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| **Rust core, Node/Bun render**         | Rust owns discovery/build orchestration; persistent workers handle JS rendering without per-request process creation. |
| **Oxc for TS/JSX**                     | Oxc provides the repository's parser/compiler/minifier pipeline. Performance must be measured for the target project. |
| **Persistent worker pool**             | Server workers are bounded to available parallelism (2–8 by default). NDJSON over stdin/stdout.                       |
| **Radix trie router**                  | O(path_depth) vs O(n) linear scan. Recompiled on manifest change.                                                     |
| **Blake3 content hashing**             | Immutable caching (max-age=31536000).                                                                                 |
| **Staging + atomic commit**            | Build writes to staging and restores the previous output if the commit fails.                                         |
| **fnv1a_64 deterministic CSS scoping** | Reproducible builds: `fnv1a_64(project_relative_path + class_name)`.                                                  |
| **`deny_unknown_fields` config**       | Typos fail fast, not silently ignored.                                                                                |

---

### Rendering Strategy Decision Tree

```
Your page is...
│
├── Browser-only (game, editor, canvas)?
│   └── → CSR  (add 'use client')
│
├── Mostly static, a few slow parts?
│   └── → PPR  (export const ppr = true + <Suspense>)
│
├── Changes every few minutes?
│   └── → ISR  (export const revalidate = 60)
│
├── Dynamic paths known at build time?
│   └── → SSG  (export getStaticParams)
│
├── Same for everyone, rarely changes?
│   └── → SSG  (auto-detected — do nothing)
│
└── Fresh data per request?
    └── → SSR  (default — do nothing)
```

**Detection priority** (first match wins):

1. `'use client'` → CSR
2. `export const ppr = true` → PPR
3. `export const revalidate = <n>` → ISR
4. `getStaticParams` / `staticParams` → SSG
5. Static candidates → SSG
6. Default → SSR

---

### NPM Package Architecture

```
ruvyxa (CLI + re-exports)
├── @ruvyxa/core          — config types, server APIs, adapter contracts
├── @ruvyxa/react         — Image, SEO, hydration, loaders, error boundary
├── @ruvyxa/auth          — sessions, OAuth, magic-link, WebAuthn
├── @ruvyxa/database      — typed CRUD with adapter pattern
├── @ruvyxa/realtime      — WebSocket action transport
├── @ruvyxa/adapter-*     — 10 platform adapters
├── @ruvyxa/cli-*         — 5 platform binaries
└── create-ruvyxa         — project scaffold
```

`ruvyxa` re-exports `@ruvyxa/core` subpaths:

- `ruvyxa/config` → `@ruvyxa/core/config`
- `ruvyxa/server` → `@ruvyxa/core/server`
- `ruvyxa/plugin` → `@ruvyxa/core/plugin`
- `ruvyxa/plugins` → built-in plugins (redirects, headers, sitemap, PWA, etc.)

---

### Source File → URL Mapping

| Pattern                         | URL                    | Type                    |
| ------------------------------- | ---------------------- | ----------------------- |
| `app/page.tsx`                  | `/`                    | Page                    |
| `app/about/page.tsx`            | `/about`               | Page                    |
| `app/blog/[slug]/page.tsx`      | `/blog/:slug`          | Dynamic                 |
| `app/docs/[...rest]/page.tsx`   | `/docs/*`              | Catch-all               |
| `app/shop/[[...cats]]/page.tsx` | `/shop` or `/shop/a/b` | Optional catch-all      |
| `app/api/route.ts`              | `/api`                 | API                     |
| `app/layout.tsx`                | —                      | Layout (wraps children) |
| `app/(group)/page.tsx`          | `/`                    | Route group             |
| `app/@modal/page.tsx`           | —                      | Parallel slot (ignored) |
| `app/_private/page.tsx`         | —                      | Private dir (ignored)   |
| `app/action.ts`                 | —                      | Server action           |
| `app/server.ts`                 | —                      | Server module           |
| `app/client.tsx`                | —                      | Client module           |
| `app/page.md` / `.mdx`          | `/`                    | Content page            |

---

### Project Structure (created by `create-ruvyxa`)

```
my-app/
├── app/
│   ├── globals.css       # Global styles
│   ├── layout.tsx        # Root layout (HTML shell)
│   └── page.tsx          # Home page
├── public/               # Static assets served from /
├── ruvyxa.config.ts      # Framework config
├── tsconfig.json
└── package.json
```

---

### Key CLI Commands

| Command              | Description                                                   |
| -------------------- | ------------------------------------------------------------- |
| `ruvyxa dev`         | Development server with HMR                                   |
| `ruvyxa build`       | Production build → `.ruvyxa/`                                 |
| `ruvyxa check`       | App-level production-readiness checks                         |
| `ruvyxa start`       | Serve production build                                        |
| `ruvyxa preview`     | Preview an existing production build locally                  |
| `ruvyxa routes`      | Print route table (`--json` emits its manifest)               |
| `ruvyxa analyze`     | Validate routes/imports/boundaries; can emit interactive HTML |
| `ruvyxa adds`        | Scaffold a form, data table, or authentication flow           |
| `ruvyxa doctor`      | Check environment and project setup                           |
| `ruvyxa clean`       | Remove generated Ruvyxa build output                          |
| `ruvyxa trace`       | Inspect one route manifest entry by path                      |
| `ruvyxa bench`       | Benchmark route discovery, analysis, and production build     |
| `ruvyxa test:parity` | Dev/prod route comparison + smoke renders                     |
| `ruvyxa plugin`      | Create a publishable plugin package                           |

---

### Next: Architecture Deep Dives

### Implementation Entry Points and Reading Order

This documentation is easiest to verify by following the runtime path rather than reading crates
alphabetically. The table below maps each user-visible concern to its primary source boundary.

| Concern                           | Primary implementation                                                           | What it owns                                                           | Read next                                           |
| --------------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | --------------------------------------------------- |
| Command parsing and orchestration | `crates/ruvyxa_cli/src/main.rs`                                                  | CLI surface, argument precedence, dispatch                             | [CLI & Build Pipeline](#cli)                        |
| Configuration translation         | `crates/ruvyxa_cli/src/config.rs`, `packages/ruvyxa/runtime/config-renderer.mjs` | Config files, validation, runtime config hand-off                      | [CLI & Build Pipeline](#cli)                        |
| Route discovery and validation    | `crates/ruvyxa_graph/src/lib.rs`                                                 | File conventions, manifests, rendering detection, boundary diagnostics | [Route Discovery](#graph)                           |
| Client compilation and linking    | `crates/ruvyxa_bundler/src`                                                      | AST scanning, resolution, boundary checks, output                      | [Bundler](#bundler)                                 |
| HTTP serving and rendering        | `crates/ruvyxa_dev_server/src/lib.rs`                                            | Axum routes, request dispatch, HMR, render cache, security application | [Dev Server](#dev-server)                           |
| Middleware and plugin bridge      | `crates/ruvyxa_middleware/src` and `packages/ruvyxa/runtime/plugin-runtime.mjs`  | Middleware stacking and JavaScript-plugin communication                | [Middleware](#middleware)                           |
| Public TypeScript contract        | `packages/@ruvyxa/core/src`, `packages/@ruvyxa/react/src`                        | Config, server helpers, React components/hooks                         | [API Reference](docs/en/17-public-api-reference.md) |

#### Boundary Walkthrough: One Request

For a page request, the dev server obtains or refreshes the route manifest from `ruvyxa_graph`,
matches the request, and sends rendering work through its worker/runtime path. The bundler and graph
share source-scanning facts for imports and environment reads so a `check` result and a build are
less likely to disagree about a client/server boundary. Plugins and middleware wrap the HTTP path;
they are not a substitute for route discovery or rendering strategy selection.

When investigating a framework issue, start with the user-visible symptom and follow this order:

```text
CLI command -> config -> route manifest -> module/boundary validation -> dev server or build output
```

This order prevents a common debugging mistake: changing the server or adapter when the route was
never discovered, or changing a page when the failure is a module boundary violation.

- [Route Discovery & Validation](#graph) — how `app/` becomes a route manifest
- [Compilation Pipeline](#bundler) — resolver → compiler → linker → minifier
- [Dev Server](#dev-server) — Axum serving, HMR protocol, render cache
- [CLI & Build Pipeline](#cli) — command structure, config loading, staging
- [Middleware](#middleware) — Tower stack, plugin bridge
- [Worker Pool](#worker-pool) — Node/Bun workers, protocol, recovery
- [Diagnostics](#diagnostics) — RUV#### error catalog
- [Protocols](#protocols) — NDJSON, WebSocket HMR, Fetch
- [Security](#security) — env isolation, rate limiting, boundaries
- [Deployment Adapters](#deployment-adapters) — adapter system overview
- [Concurrency](#concurrency) — parallelism model, locks
- [Site Discovery](#site-discovery) — sitemap/robots generation

---

## CLI Architecture

**Crate**: `ruvyxa_cli` **Source**: `crates/ruvyxa_cli/src/`

`main.rs` holds only the clap command surface and `main`'s dispatch. Everything a command does lives
in a sibling module, and the modules reach each other through crate-root re-exports:

| Module                                             | Owns                                                                             |
| -------------------------------------------------- | -------------------------------------------------------------------------------- |
| `build`                                            | the `build` command's pipeline, stage by stage                                   |
| `build_output`                                     | staging directory, atomic commit, Windows rename retry                           |
| `client_bundle`                                    | per-route browser bundles and the shared chunk                                   |
| `prerender`                                        | static HTML generation, job planning, path safety                                |
| `artifact_cache`                                   | content-addressed caching of every build artifact                                |
| `plugins`                                          | the TypeScript build-plugin worker bridge                                        |
| `add`                                              | the `adds` command's form, data-table, and authentication scaffolding            |
| `config`                                           | `ruvyxa.config.*` loading and validation                                         |
| `runtime_config`                                   | args + config → `ServerConfig`, adapter and runtime selection                    |
| `cli_args`                                         | argument spelling normalization, plugin scaffolding                              |
| `commands`                                         | `routes`, `analyze`, `check`, `doctor`, `clean`, `trace`, `bench`, `test:parity` |
| `analyzer_html`                                    | the self-contained `analyze --html` report                                       |
| `environment`                                      | toolchain and dependency probing for `doctor`                                    |
| `ui`                                               | progress bars, tables, colouring, byte/duration formatting                       |
| `image_optimizer`, `image_usage`, `site_discovery` | asset and discovery-file generation                                              |

The split is by responsibility, not by size. A command that needs more than dispatch belongs beside
the other logic of its kind rather than in `main.rs`.

### Entry Point

```
struct Cli {
    command: Command
}
```

No global flags (`root`, `verbose`, etc.) — each subcommand carries its own args. Clap v4 with
styled ANSI output.

### Command Enum (14 variants)

| Variant      | Args Struct   | Purpose                                                                                                |
| ------------ | ------------- | ------------------------------------------------------------------------------------------------------ |
| `Dev`        | `ServerArgs`  | Axum dev server with HMR, file watching, live reload                                                   |
| `Build`      | `BuildArgs`   | Production build: route discovery, validation, client bundling, SSG/ISR/PPR prerender, adapter, commit |
| `Check`      | `ProjectArgs` | `tsc --noEmit` + parity test; production readiness gate                                                |
| `Start`      | `ServerArgs`  | Axum production server from `.ruvyxa/` output                                                          |
| `Preview`    | `ServerArgs`  | Same as `Start` — alias for local preview of production build                                          |
| `Routes`     | `RoutesArgs`  | Discover and print route table (kind, path, file, strategy; `--json` emits the manifest)               |
| `Analyze`    | `AnalyzeArgs` | Validate routes, imports, server/client boundary; output as Human, JSON, SARIF, or interactive HTML    |
| `Add`        | `AddArgs`     | Additive scaffolds for a validated form, data table, or authentication flow                            |
| `Doctor`     | `DoctorArgs`  | Full project diagnostics: versions, tools, adapter compatibility, dependency check                     |
| `Clean`      | `ProjectArgs` | Remove `.ruvyxa/` output directory                                                                     |
| `Trace`      | `TraceArgs`   | Inspect one route manifest entry by route path, print as JSON                                          |
| `Bench`      | `BenchArgs`   | Benchmark route discovery + analysis + production build over N samples                                 |
| `TestParity` | `ProjectArgs` | Build then compare dev vs production route manifests + smoke render (alias: `parity`)                  |
| `Plugin`     | `PluginArgs`  | Subcommand `PluginCommand::Create(PluginCreateArgs)` — scaffold plugin package                         |

### Args Structs

```
struct ProjectArgs          { root: PathBuf, runtime: Option<CliRuntime> }
struct RoutesArgs           { root: PathBuf, runtime: Option<CliRuntime>, json: bool }
struct ServerArgs           { root: PathBuf, host: Option<String>, port: Option<u16>, runtime: Option<CliRuntime> }
struct BuildArgs            { root: PathBuf, target: Option<BuildTarget>, adapter: Option<String>, runtime: Option<CliRuntime> }
struct AnalyzeArgs          { root: PathBuf, runtime: Option<CliRuntime>, format: AnalyzeFormat, output: Option<PathBuf>, html: bool }
struct AddArgs              { templates: Vec<AddTemplate>, root: PathBuf, runtime: Option<CliRuntime>, force: bool }
struct DoctorArgs           { root: PathBuf, target: Option<BuildTarget>, adapter: Option<String>, runtime: Option<CliRuntime>, json: bool }
struct TraceArgs            { route: String, root: PathBuf }
struct BenchArgs            { root: PathBuf, samples: usize, json: bool }
struct PluginArgs           { command: PluginCommand }
  enum PluginCommand        { Create(PluginCreateArgs) }
    struct PluginCreateArgs { name: String, root: PathBuf, dir: Option<PathBuf> }
```

### Key Enums

```
BuildTarget  → Node | Bun | Edge | Static
CliRuntime   → Node | Bun
AnalyzeFormat → Auto | Human | Json | Sarif | Html
```

`BuildTarget` is also `serde::Deserialize` and stored as `config.runtime`. The CLI `--runtime` flag
uses `CliRuntime` (Node | Bun only) and maps to `JavaScriptRuntime` (from `ruvyxa_dev_server`).

### Config Loading

`load_project_config(root)` flow:

1. Detect `RUVYXA_RUNTIME` env var or `--runtime` CLI override
2. Find `config-renderer.mjs` in npm runtime scripts
3. If absent → return default `ProjectConfig` with `dependency_hash = "no-config"`
4. Spawn Node/Bun subprocess to evaluate `ruvyxa.config.ts`
5. If runtime mismatch → re-render with correct runtime
6. Parse JSON output → `ConfigRendererOutput { ok, config, code, message, stack, dependency_hash }`
7. Validate paths (appDir, outDir, CSS entries, security limits, proxy IPs)
8. Return `ProjectConfig`

Config types (all `#[serde(deny_unknown_fields)]`):

```
ProjectConfig        { app_dir, out_dir, runtime, rendering, server, css, build, debug, images, security, cache, site, middleware, plugins, adapter, adapter_options }
ServerConfigOptions  { host, port }
CssConfigOptions     { entries: Vec<String> }
BuildConfigOptions   { minify, sourcemap, tree_shaking, split_strategy, parallelism, jsx_runtime, es_target, emit_chunk_manifest, prebundle_dependencies, prerender_cache }
RenderingConfigOptions { default_strategy, default_revalidate }
DebugConfigOptions   { overlay, traces }
SecurityConfigOptions  { action_body_limit, api_body_limit, plugin_response_body_limit, action_rate_limit, same_origin, fetch_metadata, trusted_proxy_ips, security_headers }
CacheConfigOptions   { route_manifest, css, build_dir }
BuildPluginConfig    { name, head: Vec<PluginHeadEntry> }
```

### Config Override Priority

`RUVYXA_RUNTIME` env → `--runtime` CLI flag → `config.runtime` → default detection. The `--adapter`
CLI flag parses through `parse_adapter_name()` which accepts 10 known names (node, bun, static,
vercel, netlify, cloudflare, railway, render, firebase, aws) or any npm package name. Platform
auto-detection reads 6 env vars (VERCEL, NETLIFY, CF_PAGES, RAILWAY_PROJECT_ID, RENDER, AWS_APP_ID).

### Build Pipeline

`build_with_output(args, show_summary)` runs **in order**:

1. **Config load** — `load_project_config()`
2. **Route discovery** — `discover_project_routes()` → `RouteManifest`
3. **Validation** — `validate_app()` → fails on any diagnostic
4. **Plugin start** — `TypeScriptPluginBuildSession::run_start(out_dir)` — spawns persistent Node
   worker running `plugin-runtime.mjs`
5. **Staging dir** — atomic temp directory under `out_dir`; cleanup guard on drop
6. **Asset preparation** (parallel thread scope):
   - Style collection → `collect_styles()`
   - Copy `app/`, `components/`, `server/` → staging `server/`
   - Image optimization → `optimize_public_images()` → staging `assets/`
   - Copy style source files
7. **Client bundling** (parallel with asset prep) — `emit_client_bundles_with_session()`:
   - Per-route bundle preparation (module resolution, content-hash caching)
   - Shared route module extraction (modules used by ≥2 routes)
   - Route-split bundling via `ruvyxa_bundler`
   - Artifact cache with content-addressed fingerprinting (blake3-256)
   - Writes per-route JS + source maps to staging `client/`
   - Emits `route-manifest.json` (lean, browser-safe)
   - Emits `chunk-manifest.json` (if `build.manifest` enabled)
8. **Pre-rendering** — `prerender_static_routes()` — SSG/ISR/PPR/CSR:
   - Parallel worker pool (Node subprocesses, bounded by `parallelism`)
   - Artifact cache by dependency hash + render context hash + file fingerprints
   - CSR routes → minimal shell HTML
   - SSG/ISR → full render via worker pool
   - PPR → static shell (Suspense fallbacks)
   - Writes `prerender/manifest.json`
9. **Discovery files** — sitemap.xml, robots.txt via `write_discovery_files()`
10. **Platform adapter auto-detect** — matches hosting env vars when no adapter configured
11. **Build info JSON** — writes `staging/build.json`
12. **Commit staging** — `commit_staged_build_outputs()`:
    - Backup existing output → rename staging → remove backup (with Windows retry)
13. **Plugin complete** — `TypeScriptPluginBuildSession::run_complete(out_dir, manifest)`
14. **Adapter runner** (if an adapter is selected) — `run_adapter_runner()` invokes
    `adapter-runner.mjs`, which resolves the adapter and materializes its artifact reports

### Module Resolution & Bundler

`emit_client_bundles_with_session()` uses `ruvyxa_bundler::BundleContext`:

- Creates `CompileCache` and `ResolveGraphCache` at `cache/bundler/`
- When plugins present → attaches `BuildHookPipeline` with `TypeScriptPluginBridge` hooks
- Plugin hooks: `resolve_id`, `load`, `transform` — each communicates with persistent Node worker
  via NDJSON over stdin/stdout
- Supports `SplitStrategy::Route` (default) and `SplitStrategy::Single`
- Client bundling respects minify, sourcemap, tree-shaking, JSX runtime (classic/automatic), ES
  target (es2018–esnext)
- Progress bar on TTY; silent in pipes/CI

### Plugin Host

`TypeScriptPluginBuildSession` manages a persistent Node process running `plugin-runtime.mjs`:

- `run_start(out_dir)` → calls `build.start` hook
- `run_complete(out_dir, manifest)` → calls `build.complete` hook
- Hooks are used during bundling via `TypeScriptPluginBridge` which implements `BuildHooks` trait
- Worker protocol: JSON line → stdin, JSON line ← stdout, errors → stderr
- Round-robin across workers for concurrent hook calls

### Plugin Create Scaffolding

`plugin create <name>` copies 6 template files from `templates/plugin/`:

- `src/index.ts`, `test/plugin.test.mjs`, `package.json`, `tsconfig.json`, `README.md`, `.gitignore`
- Replaces `__PLUGIN_NAME__`, `__PLUGIN_IDENTIFIER__`, `__RUVYXA_VERSION__`
- Validates plugin name: lowercase + digits + single hyphens only
- Default dir is `<name>` under root; `--dir` overrides (must be relative, no `..`)
- Package is named `ruvyxa-plugin-<name>`

### CLI Normalization

Before clap parsing, `normalized_cli_args()` normalizes option and command casing (case-insensitive
matching to canonical forms). This makes `ruvyxa BUILD --Target node` equivalent to
`ruvyxa build --target node`.

### Error Handling

- `anyhow::Result` with `.context()` for all failures
- Error codes: `RUV1205` (prerender path escape), `RUV1600`–`RUV1603` (config validation),
  `RUV1700`–`RUV1701` (plugin errors), `RUV2200`–`RUV2203` (adapter errors)
- Diagnostics bubble through `fail_on_diagnostics()` which prints each diagnostic and bails with
  count
- Invalid config fields rejected at deserialization via `deny_unknown_fields`
- Security limits validated against hard ceilings from `ruvyxa_dev_server` constants
- Build staging directory has drop-guard cleanup; commit failures trigger rollback

---

## Route Discovery & Validation · การค้นหาและตรวจสอบเส้นทาง

**Crate**: `ruvyxa_graph` **Module**: `crates/ruvyxa_graph/src/lib.rs`

### สรุป (Thai Summary)

`ruvyxa_graph` สแกนไดเรกทอรี `app/` เพื่อค้นหาไฟล์ page, layout, route, action, server, client
modules สร้าง RouteManifest ที่มีโครงสร้าง JSON พร้อมตรวจสอบความถูกต้อง (duplicate routes, boundary
violations, missing exports)

---

### Core Data Structures

#### RouteManifest

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteManifest {
    pub app_dir: PathBuf,
    pub routes: Vec<RouteEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i18n: Option<I18nRouting>,
}
```

`i18n` is present only when the project config enables locale routing. It carries the validated
policy used by discovery, native serving, and deployment runtimes; consumers do not re-parse raw
config.

#### I18nRouting

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct I18nRouting {
    pub locales: Vec<String>,
    pub default_locale: String,
    pub locale_param: String,
    pub detect_locale: bool,
    pub cookie: String,
}
```

#### RouteEntry

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

#### RouteKind

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteKind {
    Page,
    Api,
}
```

Only `page.tsx`, `page.jsx`, `page.md`, `page.mdx` → `RouteKind::Page`. Only `route.ts`, `route.js`
→ `RouteKind::Api`.

#### RenderStrategy & RenderMeta

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

#### HydrationMode

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

#### RuntimeTarget

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

### File Conventions

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

### Dynamic Segments

| Pattern       | Example URL            | params                           |
| ------------- | ---------------------- | -------------------------------- |
| `[slug]`      | `/blog/hello`          | `{ slug: "hello" }`              |
| `[...rest]`   | `/docs/a/b`            | `{ rest: ["a","b"] }`            |
| `[[...rest]]` | `/shop` or `/shop/a/b` | omitted or `{ rest: ["a","b"] }` |

Validation rule RUV1002: catch-all must be last segment. Parameter names cannot contain brackets or
start with `.`.

### Route Path Resolution

`route_path_from_dir()` strips route groups `(name)` and parallel slots `@name` from the directory
path, then translates dynamic segment syntax.

```rust
// /app/(marketing)/blog/[slug]/page.tsx → /blog/[slug]
// /app/@modal/page.tsx → ignored (@-prefixed dirs filtered)
// /app/_private/page.tsx → ignored (_-prefixed dirs filtered)
```

The directory walk uses `WalkDir::filter_entry` to skip `_` and `@` prefixed directories entirely —
they never appear in the manifest.

### Layout Nesting

`layout_chain()` walks from `app_dir` to the route directory, collecting every `layout.tsx` along
the path. The root layout at `app/layout.tsx` is always first.

```rust
fn layout_chain(app_dir: &Path, route_dir: &Path) -> Vec<String> {
    // Start at app_dir, check app/layout.tsx
    // Walk each directory segment, check <segment>/layout.tsx
}
```

Layout IDs use the same `route_id()` format: `app/layout` or `app/blog/layout`.

### Rendering Strategy Detection

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

### Validation

#### `validate_app()` → `ValidationReport`

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

#### Conflict Detection

`detect_conflicts()` normalizes route paths to a "match shape" — dynamic segments become `:`,
catch-alls become `*`, optional catch-alls become `*?`. Routes sharing the same shape at the same
depth produce RUV1003.

### DiscoverOptions

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverOptions {
    pub app_dir: PathBuf,
    pub default_render_strategy: Option<RenderStrategy>,
    pub default_revalidate: Option<u64>,
    pub i18n: Option<I18nRouting>,
}
```

`with_rendering_defaults()` applies a project-wide default when the auto-detected strategy is SSR.
This lets `ruvyxa.config.ts` set `render.strategy: "ssg"` for all routes. `with_i18n()` attaches the
config-validated locale routing policy to the resulting manifest.

### Module Graph Collection

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

#### Module memoization

`collect_relative_graph()` is called once per route and once per layout in each route's chain, so a
layout or shared component reachable from many routes would otherwise be read and scanned once per
route. `ModuleCache` memoizes, per canonical path, everything derived from one read of a file: its
source, its `ModuleAst`, its masked code, and its resolved import edges. Route discovery and route
validation each hold one for the whole run.

It caches **modules, not reachable sets**. The BFS still runs per entry, so every caller receives
exactly the set it would have computed alone; only the file read and scan are shared. Caching whole
reachable sets would be wrong here — a second walk arriving at an already-visited module would
short-circuit and return a partial graph.

Reading through one place is also what keeps Markdown masking honest. `.md` and `.mdx` sources have
their fenced examples blanked before anything scans them, so a documented `import './config'` cannot
become a real graph edge. That decision lives in `ModuleCache`, not at each call site, because when
it lived at the call sites the edge walk was the one that skipped it.

### Source File → URL Mapping

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

### Write Manifest

`write_manifest()` serializes `RouteManifest` to pretty-printed JSON at the output path. The CLI
reads this manifest downstream during bundling, middleware setup, and sitemap generation.

### Why This Design

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

---

## Bundler · การรวมโค้ด

**Crate**: `ruvyxa_bundler` **Modules**:
`crates/ruvyxa_bundler/src/{lib,types,resolver,compiler,boundary,linker}.rs`

### สรุป

`ruvyxa_bundler` รับ RouteManifest + source files → สร้างชุด bundles ที่พร้อม deploy (IIFE สำหรับ
client, ESM/CJS สำหรับ server) ใช้ Oxc เป็น parser/minifier, path resolution แบบกำหนดเอง, circular
dep detection

---

### Core Types

#### BundleTarget

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BundleTarget {
    Client,
    Server,
}
```

#### BundleOptions

```rust
#[derive(Debug, Clone)]
pub struct BundleOptions {
    pub minify: bool,
    pub source_map: bool,
    pub tree_shaking: bool,
    pub jsx_runtime: JsxRuntime,
    pub es_target: EsTarget,
    pub split_strategy: SplitStrategy,
    pub emit_chunk_manifest: bool,
    pub collect_module_manifest: bool,
}
```

`collect_module_manifest` gathers the module graph for multi-route coordination without writing a
user-facing chunk manifest; `emit_chunk_manifest` is the one that produces the file. Defaults:
`minify` and `tree_shaking` on, `source_map` off, `EsTarget::Es2022`, `JsxRuntime::Automatic`,
`SplitStrategy::Single`.

#### EsTarget

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EsTarget {
    Es2015,
    Es2016,
    Es2017,
    Es2018,
    Es2019,
    Es2020,
    Es2021,
    Es2022,
    #[default]
    EsNext,
}
```

#### JsxRuntime

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum JsxRuntime {
    #[default]
    Automatic,
    Classic,
}
```

#### SplitStrategy

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SplitStrategy {
    #[default]
    Single,
    Vendor,
    Route,
    Granular,
}
```

#### BundleOutput

```rust
#[derive(Debug, Clone)]
pub struct BundleOutput {
    pub code: String,
    pub source_map: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub stats: BundleStats,
    pub chunk_manifest: Option<ChunkManifest>,
    pub chunks: Vec<OutputChunk>,
    // …
}
```

#### BundleInput

```rust
#[derive(Debug, Clone)]
pub struct BundleInput {
    pub entry: PathBuf,
    pub project_root: PathBuf,
    pub app_dir: PathBuf,
    pub layouts: Vec<PathBuf>,
    pub request_path: String,
    pub target: BundleTarget,
    pub options: BundleOptions,
    pub specials: RouteSpecials,
}
```

---

### Pipeline

```
bundle(BundleInput)                  — or bundle_with_context / bundle_with_shared_modules
  ↓
prepare_bundle()                     — resolve, compile, and check, producing a PreparedBundle
    resolver::resolve_graph()        — entry + layouts → module graph, tsconfig paths, externals
    compiler::CompiledModule::new()  — Oxc transform (JSX, TS), then one AST scan per module
    boundary::check()                — server/client violations; hard ones abort, rest collected
  ↓
bundle_prepared()                    — reusable across routes sharing a PreparedBundle
    chunking                         — split by SplitStrategy, emit shared chunks
    linker::link / link_parallel     — IIFE for client, CommonJS for server
    minifier                         — Oxc minifier when `minify`
    sourcemap::SourceMapBuilder      — when `source_map`
  ↓
BundleOutput { code, source_map, diagnostics, stats, chunk_manifest, chunks }
```

`bundle()` returns the output; writing it to `out_dir` belongs to the CLI's `client_bundle` and
`build_output` modules, not the bundler.

---

### Resolution

#### Extensions

```rust
// `resolve_file_candidate()` probes explicit files first, then:
// ts, tsx, js, jsx, mts, cts, mjs, cjs, md, and mdx.
// It also probes `index` with the same extensions.
```

#### Resolver

The workspace resolver (`resolver.rs`) resolves:

- project-relative and absolute files with the probe order above;
- `tsconfig.json` or `jsconfig.json` `paths`/`baseUrl` aliases before package lookup;
- bare package specifiers and subpaths through `package.json` `exports`;
- CSS-like imports as non-JavaScript assets, so the style pipeline handles them.

```rust
pub fn resolve_specifier(base_dir: &Path, specifier: &str) -> Option<PathBuf>
```

Resolution order:

1. Relative path (`./` or `../`) → extension/index probing
2. Absolute path (used by framework-generated virtual imports)
3. `tsconfig.json`/`jsconfig.json` aliases
4. Bare package specifier or package subpath

#### Source scanning (`ast`)

`ast::parse_module()` is the only JavaScript byte scanner in the workspace, and every stage that
needs facts about a source file goes through it — the resolver's graph walk, the compiler's
transform plan, chunking, the server/client boundary check, and `ruvyxa_graph`'s route validation
and rendering-strategy detection.

Sharing primitives was not enough. `boundary.rs` and `ruvyxa_graph` each used to own a walk that
called into `ast` for the regex decision but re-derived everything else: where a string ends, where
a template literal's interpolations are, how a block comment closes. Those walks are gone. Every
fact a consumer needs is now recorded during the one pass and read off `ModuleAst`:

| Fact                          | Field                | Consumer                                     |
| ----------------------------- | -------------------- | -------------------------------------------- |
| Import edges                  | `imports`            | resolver, chunking, boundary, `ruvyxa_graph` |
| Runtime default export        | `has_default_export` | route validation (RUV1004)                   |
| `process.env` reads           | `env_reads`          | boundary (RUV1008), route validation         |
| JSX / TS / decorators / enums | `has_*`              | compiler transform plan                      |

Every field has a production reader, and that is enforced rather than assumed: a fact nothing
consumes is still allocated for every module in the graph and retained for the run. The named-export
list used to sit in this table with the linker listed as its consumer; the linker did not read it,
and it was removed rather than kept for a hypothetical caller.

`ast::masked_code()` covers the one consumer that needs code _text_ rather than facts: rendering
strategy detection matches on markers like `export const revalidate` and `fetch(`. It blanks
non-code regions using the same walk, preserving byte offsets and line breaks, so that consumer has
no reason to grow a lexer either.

Policy stays with the consumer. `ast` records that `process.env.X` was read; deciding which names
may reach a browser bundle is `boundary`'s.

That consolidation is deliberate. A scanner that does not classify `/` correctly reads `/["']/` as a
division followed by an unterminated string, and the string skip then swallows the rest of the file:
imports after that point vanish from the dependency graph, `server-only` stops tripping RUV1007, and
a page's default export becomes invisible to validation. That exact bug has been fixed more than
once, in more than one copy of the scanner. One walk cannot drift from itself.

It is also why each compiled module is parsed exactly once, at `CompiledModule::new`, and the result
carried on the module as `Arc<ModuleAst>`. Chunk planning previously re-parsed every module once per
dynamic root and once per emitted chunk, so scan cost grew with the number of `import()` sites in
the app rather than with its size.

The scanner tracks the last token-ending byte to tell a regex literal from a division, skips
comments and strings, and walks template literals so `${…}` interpolations are scanned as code.
Every helper it delegates to is bounded to the range being scanned, so an interpolation's scan
cannot read into the surrounding literal text.

#### Incremental graph cache

`IncrementalGraphCache` persists resolved dependency edges to `graph-manifest.json` so an unchanged
module skips import extraction and resolution on the next build. Reuse is gated on a blake3 content
hash of the current source, so a timestamp-preserving edit cannot return stale edges.

An entry stores the resolved paths **and** the specifier-to-path alias map they were resolved
through. Both or neither: the linker consults a module's alias map before matching by path suffix,
and an alias like `~/components/Button` shares no suffix with its target, so reusing the paths alone
made a warm build emit an unresolved `import … from "~/components/Button"` and omit the target
module. Entries that cannot supply both are resolved fresh.

`MANIFEST_VERSION` is a constant identity and is not bumped when the entry format grows. A
hand-maintained counter fails silently when someone forgets it, so compatibility lives in the entry
format instead: fields added later are `Option`, which keeps "absent" distinguishable from "empty".
A reader declines to reuse an entry that predates a field it needs, and the fresh resolve rewrites
that entry complete — so an older cache self-heals one module at a time instead of being discarded
wholesale.

---

### Compilation

#### Oxc Pipeline

```rust
pub fn transform(source: &str, has_jsx: bool) -> Result<String, String>;

pub fn transform_with_options(
    source: &str,
    has_jsx: bool,
    jsx_runtime: JsxRuntime,
) -> Result<String, String> {
    // 1. strip_decorators: accepted, removed, no runtime helper injected
    // 2. Parser: source → AST (TypeScript always on, JSX per `has_jsx`)
    // 3. SemanticBuilder (with_enum_eval) → TS/JSX transform
    // 4. Codegen: AST → output code
}
```

There is no compiler struct. Transformation is a free function over source text, and the compiled
result is owned by `CompiledModule`, which parses the AST once at construction.

#### CompiledModule

```rust
#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub path: PathBuf,
    pub js: Arc<str>,
    pub ast: Arc<ast::ModuleAst>,
    pub deps: Arc<[PathBuf]>,
    pub dependency_aliases: Arc<BTreeMap<String, PathBuf>>,
    pub is_external: bool,
    pub cache_hit: bool,
}
```

Dependencies are resolved to absolute paths during compilation and tracked for circular detection.
The `Arc` fields are the point: one module appears in the full graph, the entry's static closure,
and one closure per emitted chunk at the same time, and each of those used to carry its own copy of
the generated JavaScript. `ast` is parsed in `CompiledModule::new`, the single place a compiled
module comes into existence, so no later stage re-walks `js`.

---

### Boundary Checking

#### `check_boundary()`

```rust
pub fn check(
    modules: &[CompiledModule],
    input: &BundleInput,
    out: &mut Vec<Diagnostic>,
) -> Result<()>
```

Non-fatal diagnostics are appended to `out`; a hard violation — one that would produce broken output
— returns `BundleError::Diagnostic` and aborts the bundle. SSR and Edge targets enforce only the
client-only rule, since that output runs on the server.

#### Rules enforced

| Rule                  | Check                                                         | Diagnostic |
| --------------------- | ------------------------------------------------------------- | ---------- |
| Server-only imports   | Module graph from client entry contains `server-only`         | RUV1007    |
| Private `process.env` | Module graph accesses `process.env.*` not in `RUVYXA_PUBLIC_` | RUV1008    |
| Client-only in server | Module graph from server entry contains `client-only`         | RUV1009    |
| Server dir in client  | Import chain reaches `server/` directory from client entry    | RUV1010    |

The check traverses the compiled module graph (BFS) and flags any prohibited import.
`NodeModulesExternal` option can whitelist known packages.

#### `has_default_export()`

```rust
pub fn has_default_export(source: &str) -> bool
```

Reports whether a module exports a default binding, without a full parse. It reuses the same string
and comment skipping as the rest of `ast.rs`, so `"export default"` inside a string literal or a
comment is not counted. It recognises every valid form — `export default <expr>`,
`export default function`/`class`, `export { X as default }`, `export { default } from`, and
`export * as default from` — and rejects `export type { X as default }`, which erases at compile
time.

`ruvyxa_graph::validate_app` uses it for RUV1004 so a page whose default export is re-exported is
not reported as missing one.

---

### Linking Strategy

```rust
pub fn link(modules: &[CompiledModule], input: &BundleInput) -> Result<String>;
pub fn link_parallel(modules: &[CompiledModule], input: &BundleInput) -> Result<String>;
```

Both run `detect_cycles` first, then delegate to a shared inner implementation that never re-checks.
`ordered_project_modules` fixes emission order; output capacity is pre-calculated from each module's
source plus wrapper overhead so the concatenation does not reallocate.

#### Client: IIFE

Each module becomes `__ruvyxa.define("<id>", function(module, exports) { … })`, and the bundle
closes over a small registry that resolves an id to its evaluated `module.exports` on first require.
`module_id` derives the id from the project-relative path.

#### Server: CommonJS

Server modules use `require`/`module.exports` for Node.js compatibility. The target — not a separate
option — selects this: `BundleTarget::Ssr` and `BundleTarget::Edge` link as CommonJS,
`BundleTarget::Client` as the IIFE registry above.

---

### Circular Dependency Detection

```rust
pub fn detect_cycles(modules: &[CompiledModule]) -> Result&lt;()&gt;
```

Tarjan's algorithm on the compiled module list. Each detected cycle produces a `Diagnostic` but does
not halt the build — the linker handles circular references by emitting `undefined` for
not-yet-initialized module refs.

---

### Minification

Oxc minifier pipeline:

```rust
fn minify(source: &str, target: EsTarget) -> String {
    // OxcMinifier runs:
    //   1. Dead code elimination
    //   2. Constant folding
    //   3. Identifier shortening
    //   4. Whitespace removal
}
```

Source maps are generated alongside minified output if `BundleOptions.source_map` is `true`.

---

### Emit

The bundler returns a `BundleOutput`; writing it is the CLI's job (`client_bundle` and
`build_output`). Client bundles are content-addressed, which is what makes the immutable
`Cache-Control` on `/_ruvyxa/` safe — a changed file is a different filename, never a stale hit.

```
.ruvyxa/
  client/<blake3-hash>.js     # per-route bundles and shared chunks
  server/app/…                # compiled route modules
  server/styles/…
  assets/                     # copied and optimized public assets
  prerender/                  # SSG/ISR HTML
  cache/                      # content-addressed build artifact cache
  manifest.json               # route manifest
  build.json                  # build metadata, security defaults, render summary
```

---

### Why This Design

1. **Oxc over SWC** — Rust-native parsing/minification without a NAPI bridge; benchmark this
   repository's workload before making a speed claim. overhead.
2. **IIFE for client** — No ESM module system dependency in the browser. Works in all environments
   including workers, edge, and sandboxes.
3. **Custom resolver, not webpack** — No need for webpack's plugin system complexity. `ruvyxa` has 3
   entry points per route (page, client, server module); resolution is simple enough for a dedicated
   resolver.
4. **Boundary check at bundle time** — Prevents server-only code from leaking into browser bundles
   before they reach the user. Catches the error during `build`, not at runtime.
5. **Tarjan for cycles** — Cycle detection is a developer DX feature, not a correctness requirement.
   The linker handles cycles gracefully; the detection simply warns.

---

## Dev Server

**Crate**: `ruvyxa_dev_server` —
`crates/ruvyxa_dev_server/src/{lib,router,render_cache,hmr_tracker,worker_pool,style,action_security,port_binding,render_pipeline,plugin_bridge,plugin_head,html_document,env_file,static_assets,cli_output}.rs`

Axum HTTP server with HMR (WebSocket), radix-trie route matching, LRU render cache, persistent
Node/Bun worker pool, style collection pipeline, action security middleware, TypeScript plugin host,
and realtime event broadcasting.

---

### ServerConfig

30 fields. Constructed via `ServerConfig::dev(root, host, port)` or
`ServerConfig::production(root, host, port)`.

| Field                              | Type                     | Dev default                      | Production default                |
| ---------------------------------- | ------------------------ | -------------------------------- | --------------------------------- |
| `root`                             | `PathBuf`                | `root`                           | `root`                            |
| `app_dir`                          | `PathBuf`                | `root.join("app")`               | `root.join(".ruvyxa/server/app")` |
| `public_dir`                       | `PathBuf`                | `root.join("public")`            | `root.join(".ruvyxa/assets")`     |
| `client_dir`                       | `PathBuf`                | `root.join(".ruvyxa/client")`    | `root.join(".ruvyxa/client")`     |
| `prerender_dir`                    | `PathBuf`                | `root.join(".ruvyxa/prerender")` | `root.join(".ruvyxa/prerender")`  |
| `host`                             | `String`                 | `host`                           | `host`                            |
| `port`                             | `u16`                    | `port`                           | `port`                            |
| `watch`                            | `bool`                   | `true`                           | `false`                           |
| `cache_route_manifest`             | `bool`                   | `true`                           | `true`                            |
| `cache_css`                        | `bool`                   | `true`                           | `true`                            |
| `style_entries`                    | `Vec<PathBuf>`           | `Vec::new()`                     | `Vec::new()`                      |
| `prebundle_dependencies`           | `bool`                   | `true`                           | `false`                           |
| `runtime`                          | `JavaScriptRuntime`      | `JavaScriptRuntime::detect()`    | `JavaScriptRuntime::detect()`     |
| `jsx_runtime`                      | `JsxRuntime`             | `Automatic`                      | `Automatic`                       |
| `error_overlay`                    | `bool`                   | `true`                           | `false`                           |
| `debug_traces`                     | `bool`                   | `false`                          | `false`                           |
| `action_body_limit_bytes`          | `usize`                  | `1MB`                            | `1MB`                             |
| `api_body_limit_bytes`             | `usize`                  | `10MB`                           | `10MB`                            |
| `plugin_response_body_limit_bytes` | `usize`                  | `32MB`                           | `32MB`                            |
| `action_rate_limit_max`            | `usize`                  | `600`                            | `600`                             |
| `action_rate_limit_window`         | `Duration`               | `60s`                            | `60s`                             |
| `same_origin_actions`              | `bool`                   | `true`                           | `true`                            |
| `fetch_metadata_actions`           | `bool`                   | `true`                           | `true`                            |
| `trusted_proxies`                  | `TrustedProxies`         | `TrustedProxies::default()`      | `TrustedProxies::default()`       |
| `security_headers`                 | `bool`                   | `true`                           | `true`                            |
| `middleware`                       | `MiddlewareConfig`       | `default()`                      | `default()`                       |
| `plugins_enabled`                  | `bool`                   | `false`                          | `false`                           |
| `plugin_head`                      | `Vec<PluginHeadEntry>`   | `Vec::new()`                     | `Vec::new()`                      |
| `default_render_strategy`          | `Option<RenderStrategy>` | `None`                           | `None`                            |
| `default_revalidate`               | `Option<u64>`            | `None`                           | `None`                            |

Validation rejects zero/over-limit values (absolute bounds: `MAX_ACTION_BODY_LIMIT_BYTES=16MB`,
`MAX_API_BODY_LIMIT_BYTES=256MB`, `MAX_ACTION_RATE_LIMIT_REQUESTS=10000`,
`MAX_ACTION_RATE_LIMIT_WINDOW_SECS=86400`, `MAX_PLUGIN_RESPONSE_BODY_LIMIT_BYTES=256MB`).

---

### JavaScriptRuntime

```rust
pub enum JavaScriptRuntime { Node, Bun }
```

| Method                         | Returns   | Description                                      |
| ------------------------------ | --------- | ------------------------------------------------ |
| `command()`                    | `&str`    | `"node"` or `"bun"`                              |
| `executable()`                 | `PathBuf` | Resolves `bun.exe` behind `.cmd` shim on Windows |
| `is_available()`               | `bool`    | Checks `--version` exit code                     |
| `detect()`                     | `Self`    | Node preferred, Bun fallback                     |
| `from_availability(node, bun)` | `Self`    | Explicit selection                               |

---

### Framework Endpoints

| Route                                  | Method | Handler                  | Purpose                                                          |
| -------------------------------------- | ------ | ------------------------ | ---------------------------------------------------------------- |
| `/__ruvyxa/hmr`                        | GET    | `hmr_ws`                 | HMR WebSocket — broadcasts file-change JSON to browsers          |
| `/__ruvyxa/client`                     | GET    | `client_bundle`          | On-demand compiled client JS bundles per route                   |
| `/__ruvyxa/hydration-loader.js`        | GET    | `hydration_loader`       | Client hydration loader script                                   |
| `/__ruvyxa/client/route-manifest.json` | GET    | `client_manifest`        | Live route table for browser router                              |
| `/__ruvyxa/image`                      | GET    | `dynamic_image_endpoint` | Bounded same-origin WebP resize when `image.onDemand` is enabled |
| `/__ruvyxa/action`                     | POST   | `action_endpoint`        | Server action dispatch                                           |
| `/__ruvyxa/trace`                      | GET    | `trace_endpoint`         | Runtime route trace (debug only)                                 |
| `/__ruvyxa/devtools`                   | GET    | `devtools_dashboard`     | Development dashboard (only while watching)                      |
| `/__ruvyxa/devtools/data`              | GET    | `devtools_data`          | Development dashboard data (only while watching)                 |

Reserved paths (collision rejection): `/__ruvyxa/hmr`, `/__ruvyxa/client`, `/__ruvyxa/action`,
`/__ruvyxa/trace`, `/__ruvyxa/devtools`, `/__ruvyxa/devtools/data`, and `/__ruvyxa/image`.

---

### Key Modules

#### RadixRouter (`router.rs`)

```rust
pub struct RadixRouter { root: TrieNode, patterns: Vec<Vec<PatternSegment>> }
impl RadixRouter {
    pub fn compile(manifest: &RouteManifest) -> Self;
    pub fn find<'a>(&self, manifest: &'a RouteManifest, request_path: &str) -> Option<RouteMatch<'a>>;
}
```

`compile()` builds a trie from manifest routes. `find()` walks the trie by path segment: static
children first, then `[param]`, then `[...rest]`/`[[...rest]]`. Returns matched `RouteEntry` with
extracted `RouteParams`. Parameter names come from the matched route's pattern, not the trie node
(sibling routes with different param names share one node).

#### RenderCache (`render_cache.rs`)

```rust
pub struct RenderCache { entries, order, capacity, ttl, hits, misses }
impl RenderCache {
    pub fn new(capacity: usize, ttl_secs: u64) -> Self;
    pub fn default_dev() -> Self;        // 1024 entries, 300s TTL
    pub fn default_production() -> Self; // 512 entries, 1800s TTL
    pub async fn get_arc(&self, key: &str) -> Option<Arc<str>>;
    pub async fn get_stale_with_age(&self, key: &str) -> Option<(Arc<str>, Duration)>;
    pub async fn put(&self, key: String, value: String) -> Arc<str>;
    pub async fn invalidate_all(&self) -> usize;
    pub async fn invalidate_prefix(&self, prefix: &str) -> usize;
    pub async fn invalidate_route(&self, route_path: &str) -> usize;
    pub fn invalidate_all_blocking(&self) -> usize;
    pub fn invalidate_prefix_blocking(&self, prefix: &str) -> usize;
    pub fn invalidate_route_blocking(&self, route_path: &str) -> usize;
}
```

Thread-safe LRU with O(1) get/put/eviction via hash-indexed doubly-linked recency list. Entries
TTL-expired on read. ISR uses `get_stale_with_age` to serve stale while revalidating. `blocking_*`
methods for file-watcher sync context.

Entries are stored as `Arc<str>` and every read hands back the stored handle, so serving a cache hit
does not copy the document. `put` returns the same handle it stored — including when the cache is
disabled (`capacity == 0`), so the caller always gets its value back and never needs a second copy
for the response.

#### HmrTracker (`hmr_tracker.rs`)

```rust
pub struct HmrTracker { file_to_routes: BTreeMap<PathBuf, BTreeSet<String>>, route_to_files }
impl HmrTracker {
    pub fn new() -> Self;
    pub fn populate_from_manifest(&self, routes: &[RouteEntry]);
    pub fn register_route(&self, route_path: &str, source_files: &[PathBuf]);
    pub fn compute_update(&self, changed_paths: &[PathBuf]) -> HmrUpdate;
    pub fn clear(&self);
}
pub struct HmrUpdate {
    pub affected_routes: Vec<String>,
    pub full_reload: bool,
    pub changed_files: Vec<PathBuf>,
    pub event_type: HmrEventType,
}
pub enum HmrEventType { CssUpdate, ComponentUpdate, FullReload }
```

Reverse map: changed file → affected routes. Css-only → `CssUpdate`. Layout change → `FullReload`.
Unknown untracked file → `FullReload`.

#### NodeWorkerPool (`worker_pool.rs`)

```rust
pub struct NodeWorkerPool { workers, worker_script, env, runtime, next_worker, response_timeout, isolated_renders_per_worker }
impl NodeWorkerPool {
    pub async fn start(root, env) -> Result<Self>;
    pub async fn start_with_runtime(root, env, runtime) -> Result<Self>;
    pub async fn shutdown(&self);
    pub async fn warmup(&self, project_root, routes) -> usize;
    pub async fn invalidate(&self, paths: Vec<String>);
    pub fn invalidate_from_watcher(&self, paths) -> Result<usize, String>;
    pub async fn render_ssr(&self, ...) -> Result<WorkerResponse>;
    pub async fn render_client(&self, ...) -> Result<WorkerResponse>;
    pub async fn render_ssg(&self, ...) -> Result<WorkerResponse>;
    pub async fn resolve_static_params(&self, ...) -> Result<WorkerResponse>;
}
```

Persistent Node/Bun processes communicating via NDJSON over stdin/stdout. Pool size: 2-8 (default
CPU count clamped). Least-loaded worker selection with rotating start offset. Failed workers
replaced automatically; idempotent requests retried once.

**Worker recycling during builds.** Production prerendering asks for an isolated module import per
path (`render_ssg_isolated`) so page-module state cannot leak between paths. That isolation works by
importing the bundle under a fresh module URL, and Node's ESM registry never releases a URL — so
each isolated import permanently retains one more module graph, and no cache eviction inside the
worker can reclaim it. Replacing the process is the only operation that frees them.

The build pool therefore retires a worker once it has served `RUVYXA_PRERENDER_RECYCLE_AFTER`
isolated renders (default 32; `0` disables recycling). Retirement only happens when the worker is
idle, because `shutdown` clears pending requests and would otherwise fail sibling renders that were
progressing normally. The dev server passes `None` — it never requests isolated imports, so it
retains nothing to reclaim and pays nothing for the bound.

**Per-worker concurrency.** Inside each worker, `worker-pool.mjs` admits at most
`RUVYXA_WORKER_MAX_CONCURRENCY` requests at a time (default: core count clamped to 2–8). Renders are
CPU-bound and each one holds a React tree, a compiled bundle, and its response buffer, so admitting
a whole burst at once exhausts the heap or thrashes the CPU into timeouts that look like hangs.
Excess requests queue and run as slots free up; `invalidate` and `ping` bypass the queue, since
delaying a cache invalidation would leave the worker serving stale bundles exactly when it is
busiest. `ping` reports `activeRequests`, `queuedRequests`, and `maxConcurrentRequests`.

#### Worker environment variables

| Variable                         | Default              | Effect                                                     |
| -------------------------------- | -------------------- | ---------------------------------------------------------- |
| `RUVYXA_WORKER_POOL_SIZE`        | CPU count (2–8)      | Worker processes in the dev/prod pool                      |
| `RUVYXA_WORKER_MAX_CONCURRENCY`  | CPU count (2–8)      | Requests one worker executes at once                       |
| `RUVYXA_WORKER_TIMEOUT_MS`       | 30000 / 300000 build | Per-request deadline, shared by Rust and the Node watchdog |
| `RUVYXA_PRERENDER_RECYCLE_AFTER` | 32 (`0` disables)    | Isolated prerenders before a build worker is retired       |
| `RUVYXA_CACHE_MAX_ENTRIES`       | 256                  | Bundle and module cache entries per worker                 |
| `RUVYXA_MEMORY_LIMIT_MB`         | 512                  | Heap threshold that triggers in-worker cache eviction      |
| `RUVYXA_RENDER_CACHE_SIZE`       | 1024 dev / 512 prod  | Render cache entries (capped at 16384)                     |

#### StyleCollection (`style.rs`)

```rust
pub struct StyleCollection { pub css: String, pub files: Vec<PathBuf> }
pub fn collect_styles(root, app_dir, entries) -> Result<StyleCollection>;
pub fn minify_css(css: &str) -> String;
```

Walks `app/` script imports, resolves CSS/SCSS/Sass dependencies, compiles Sass, scopes CSS Modules,
compiles Tailwind via `@tailwindcss/cli`. Minifies in production mode. Escapes `</style` in output.

#### ActionSecurity (`action_security.rs`)

```rust
pub(crate) fn validate_action_request(headers, body_len, config, peer) -> Option<Response>;
pub(crate) fn validate_action_payload(headers, body) -> Result<(&str, String), Box<Response>>;
pub(crate) struct ActionRateLimiter { /* fixed slot array of sliding-window counters */ }
pub struct IpPrefix { /* network address + prefix length */ }
pub struct TrustedProxies { /* matchable prefixes from security.trustedProxyIps */ }
```

Validates: body size ≤ configured limit, Content-Type (JSON or form), same-origin (Origin == Host),
Fetch Metadata, rate limit. Rate-limit key includes client IP (forwarded from trusted proxies),
action path, and action name.

The limiter hashes each key into one of `ACTION_RATE_LIMIT_SLOTS` (8192) counter slots, so its
memory is fixed and admission is never refused for lack of room. A slot holds the current and
previous window counts; the previous count is weighted by the fraction of it still inside the
trailing window. A slot collision shares one budget between two keys, which can only limit a client
early — never grant it extra. The hasher is seeded per process, so keys cannot be crafted to collide
with a chosen victim.

`TrustedProxies` matches a peer against exact addresses and CIDR ranges, unmapping IPv4-mapped IPv6
peers first so an IPv4 range matches a dual-stack listener's `::ffff:a.b.c.d` form. Loopback is
trusted independently of the configured list.

#### PortBinding (`port_binding.rs`)

```rust
pub(crate) async fn bind_listener(config, address) -> Result<(TcpListener, SocketAddr)>;
```

Tries configured port, then scans +100 upward. On conflict, prints owner detection (netstat/lsof)
and binds first available.

---

### serve() Flow

`serve(config: ServerConfig) -> Result<()>`:

1. `validate_limits()` — reject over-limit body/rate config
2. Discover routes, compile `RadixRouter`
3. Start `NodeWorkerPool` via `start_with_runtime`
4. Warmup: spawn background pre-bundling of page dependencies (when `watch` &&
   `prebundle_dependencies`)
5. Create `RenderCache` (dev or production), `HmrTracker`, `MiddlewareStack`
6. Start TypeScript plugin host if `plugins_enabled`
7. Validate realtime config from plugin descriptor (path starts with `/`, no `?`/`#`/`*`, heartbeat
   5-120s, capacity 16-4096, no collision with reserved framework routes)
8. Build `AppState` with all components
9. Start file watcher (if `watch`): uses `notify` crate, ignores
   `.git`/`.ruvyxa`/`target`/`dist`/`.npm-pack`/`.npm-smoke`/`node_modules`
10. Register Axum routes, apply middleware stack, security headers middleware
11. Bind listener with port fallback
12. Serve with graceful shutdown (Ctrl-C / SIGTERM, 5s timeout)

---

### File Watcher & HMR

`start_watcher()` registers `notify` recursive watches on `watch_paths` (project root if exists). On
file event:

1. Filter ignored paths
2. `hmr_tracker.compute_update(paths)` → affected routes and event type
3. If `full_reload` or no affected routes: full invalidation (manifest + render cache)
4. Else: selective invalidation (styles only if CSS dep changed, render cache per route)
5. `worker_pool.invalidate_from_watcher(paths)` — queued via `try_send` (non-blocking, sync-safe)
6. Notify plugin runtime via `plugin_runtime.notify_file_change()`
7. Broadcast JSON payload via `reload_tx` to all connected HMR WebSocket clients

HMR WebSocket handler validates Origin (cross-site connection blocked), then streams broadcast
messages. Payload shape: `{ type, paths, affectedRoutes, fullReload }`.

---

### Realtime Runtime

`RealtimeRuntime { path, heartbeat, tx }` — created from TypeScript plugin host descriptor.
Validates: path is absolute, no URL special chars, heartbeat 5-120s, capacity 16-4096, no collision
with reserved framework routes.

Realtime WebSocket handler:

- Validates Origin (same as HMR)
- Parses `?channels=comma,separated` query (1-16 channels, 128 bytes each, alphanumeric + `:. _/-`)
- Filters broadcast events by channel subscription
- Sends heartbeat pings at configured interval
- Sends `{"version":1,"type":"resync","reason":"lagged"}` on channel lag

---

### Under the Hood

- **Router**: Radix trie, O(path depth) lookup. Static segments prioritized over params, params over
  catch-alls. No regex.
- **Worker pool**: Persistent Node/Bun processes. Each communicates via NDJSON over stdin/stdout.
  Pool size clamped 2-8. Least-loaded selection, auto-replacement on failure.
- **Render cache**: LRU with hash-indexed doubly-linked list. Keys prefixed by render type (`ssr:`,
  `client:`) and optionally strategy namespace (`ssg:`, `isr:`, `ppr:`). TTL-based expiry.
- **HMR**: Reverse dependency map from `HmrTracker`. Only evicts affected routes. CSS-only edits
  never invalidate JS bundles. Layout changes trigger full reload.
- **Style pipeline**: Import-graph walk from `app/` scripts, resolves TS path aliases, compiles
  Sass, scopes CSS Modules, compiles Tailwind. Minified in production.
- **Action security**: Multi-layer: body limit, content-type check, same-origin (Origin vs Host),
  Fetch Metadata, per-key sliding-window rate limiter with forwarded-proxy support.
- **Port binding**: Sequential fallback +100 ports. Detects and prints the owning process via
  `netstat`/`lsof`.
- **Plugin host**: TypeScript middleware via `PluginHost` pool. Request/response round-trip
  serialized over stdio. Realtime configured via plugin descriptor.
- **Security headers**: Applied to all responses unless `security_headers: false`. Defaults:
  `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`,
  `X-Frame-Options: DENY`, `Cross-Origin-Opener-Policy: same-origin`,
  `Cross-Origin-Resource-Policy: same-origin`,
  `Permissions-Policy: camera=(), microphone=(), geolocation=()`.

---

## Middleware

**Crate**: `ruvyxa_middleware` **Sources**:
`crates/ruvyxa_middleware/src/{config,stack,builtin,plugin_host}.rs`

### Purpose

Ruvyxa middleware has two layers: a compact set of built-in Tower layers toggled from
`ruvyxa.config.ts`, and a TypeScript plugin bridge that runs user middleware as child processes over
JSON-lines stdin/stdout. There is no plugin ordering DSL, no abstract compression algorithm enum, no
`RateLimitStore` trait — the built-in set is intentionally minimal.

### Configuration (`config.rs`)

#### `MiddlewareConfig`

```rust
pub struct MiddlewareConfig {
    pub builtin: BuiltinMiddlewareConfig,
    pub workers: Option<usize>,      // TS plugin pool size, validated 1..=8
    pub timeout_ms: Option<u64>,     // TS plugin hook timeout, 1..=300_000ms
}
```

`workers` and `timeout_ms` control the TypeScript plugin host only. Built-in middleware is always
in-process and unconstrained.

Serde: `rename_all = "camelCase"`, `deny_unknown_fields`.

#### `BuiltinMiddlewareConfig`

```rust
pub struct BuiltinMiddlewareConfig {
    pub cors: Option<CorsConfig>,                   // optional CORS
    pub timing: bool,                               // X-Response-Time header, default true
    pub logging: bool,                              // request logging, default true (serde: "log")
    pub rate_limit: Option<RateLimitConfig>,         // optional rate limiting (serde: "rate")
    pub headers: BTreeMap<String, String>,           // custom response headers
}
```

All fields optional via `serde(default)`. `timing` and `logging` default to `true` in Rust and
`false` if omitted in JSON — the serde default functions (`default_true`) match the Rust `Default`
impl.

#### `CorsConfig`

```rust
pub struct CorsConfig {
    pub origins: Vec<String>,     // allowed origins, ["*"] for permissive
    pub methods: Vec<String>,     // default: GET, POST, PUT, DELETE, OPTIONS
    pub headers: Vec<String>,     // allowed request headers
    pub credentials: bool,        // allow credentials
    pub max_age: u64,             // preflight cache seconds, default 86400
}
```

No separate `expose_headers` field — the real CORS layer is a hand-written Tower service that sets
`Access-Control-Allow-Origin`, `Allow-Methods`, `Allow-Headers`, `Allow-Credentials`, and `Max-Age`
on matching origins. Preflight `OPTIONS` requests are caught and returned `204 No Content`.
Non-matching origins get `Vary: Origin` appended to prevent cache poisoning but no CORS headers.

#### `RateLimitConfig`

```rust
pub struct RateLimitConfig {
    pub max_requests: usize,    // max requests per window (serde: "max")
    pub window_secs: u64,       // window duration in seconds (serde: "window")
    pub key_by: String,         // "ip" (default) or "header:<name>"
}
```

The rate limiter is a per-process in-memory token bucket (`BTreeMap<String, RateBucket>`). Buckets
refill when the window elapses. A hard cap of 10,000 tracked keys triggers lazy GC. Key extraction
defaults to `req.extensions().get::<SocketAddr>` (the transport peer); forwarding client identity
requires an explicit `key: "header:x-forwarded-for"`.

### Validation Rules

`MiddlewareStack::validate()` rejects invalid configurations before any layer is installed:

| Rule                                           | Error                                             |
| ---------------------------------------------- | ------------------------------------------------- |
| `workers` outside `1..=8`                      | `RUV1602 config field 'middleware.workers' ...`   |
| `timeoutMs` outside `1..=300_000`              | `RUV1602 config field 'middleware.timeoutMs' ...` |
| Custom header name/value unparseable           | `Invalid custom response header '{name}'`         |
| CORS `credentials: true` with `origins: ["*"]` | Must use explicit origin allowlist                |
| CORS method unparseable                        | `Invalid CORS method '{method}'`                  |
| CORS header unparseable                        | `Invalid CORS header '{header}'`                  |
| Rate limit `max` is `0`                        | `Rate limit 'max' must be greater than 0`         |
| Rate limit `window` is `0`                     | `Rate limit 'window' must be greater than 0`      |
| Rate limit `key` not `ip` or `header:<name>`   | Must be `ip` or `header:<valid-header-name>`      |

### MiddlewareStack (`stack.rs`)

Layers are applied bottom-to-top on the Axum `Router`. The outermost layer runs first:

```
Request
  ↓
  CompressionLayer (always on — gzip + brotli, sized bodies only)
  ↓
  CorsLayer          (if configured)
  ↓
  RateLimitLayer     (if configured)
  ↓
  TimingLayer        (if enabled)
  ↓
  RequestLoggingLayer (if enabled)
  ↓
  CustomHeadersLayer  (if non-empty)
  ↓
  Route handler
  ↓
Response
```

**Compression** uses a custom `CompleteBodyCompressionPredicate` that only compresses responses
whose body has an exact size hint. Streaming/unknown-size bodies bypass compression to avoid
incomplete chunked encoding.

**CORS** is a hand-written service, not `tower_http::cors::CorsLayer`. It inspects the `Origin`
header, matches against the allowlist, and short-circuits preflight `OPTIONS` requests to `204`.
Rejected origins get `Vary: Origin` but no CORS headers.

**Rate limiting** uses `RateLimitLayerWithKey` which decorates `RateLimitLayer` with a key
extraction strategy. The bucket is shared across clones via `Arc<Mutex<BTreeMap>>`.

**Timing** emits `X-Response-Time` in milliseconds.

**Logging** assigns a `x-request-id` (from incoming header or auto-generated `ruvyxa-{hex}`) and
logs method, path, status, and duration at `info` level.

### PluginHost (`plugin_host.rs`)

TypeScript plugin middleware runs as one or more persistent child processes (`node` or `bun`). The
runtime script loads the config registry and communicates over newline-delimited JSON on
stdin/stdout.

#### Lifecycle

1. `PluginHost::start_pool_with_timeout(root, script, executable, pool_size, timeout)` spawns
   workers
2. Worker startup sends `{"hook":"describe"}` — the worker responds with `PluginRegistryDescriptor`
3. Registry diagnostics are logged. Workers beyond the first are only spawned if the registry
   declares HTTP hooks

#### Descriptor

```rust
pub struct PluginRegistryDescriptor {
    pub plugins: Vec<String>,
    pub http: PluginHttpDescriptor,
    pub build: PluginBuildDescriptor,
    pub dev: PluginDevDescriptor,
    pub diagnostics: Vec<PluginDiagnosticDescriptor>,
    pub capabilities: Vec<NativeCapabilityDescriptor>,
}
```

`PluginHttpDescriptor` declares how many request/response hooks and route patterns are registered.
`wants_request(pathname)` and `wants_response(pathname)` check route pattern matching (`*` wildcard,
`prefix*` glob, exact match) before invoking the plugin.

#### Hook Protocol

Hooks are dispatched to workers round-robin. If the selected worker is busy, the pool scans for an
idle worker before queueing (avoids head-of-line blocking on long hooks).

- `execute_request(&PluginHttpRequest) -> PluginHttpRequestResult` — the hook either returns a
  modified `Request` or short-circuits with a `Response`
- `execute_response(&PluginHttpRequest, &PluginHttpResponse) -> PluginHttpResponse`
- `notify_file_change(&[String])` — used during development to signals file changes

#### Failure Recovery

| Scenario                         | Behavior                                                |
| -------------------------------- | ------------------------------------------------------- |
| Hook returns error               | Error propagated to caller; worker left alive           |
| Request never reached the worker | Worker restarted once; hook retried                     |
| Worker exits after the request   | Worker restarted; hook retried only if it is idempotent |
| JSON protocol corrupted          | Worker declared poisoned; replaced without retry        |
| Hook times out (> `timeout_ms`)  | Worker poisoned; replaced without retry                 |

A retry is safe only when the worker cannot have acted on the first attempt. Write and flush
failures are reported as _not delivered_ and are always retried. A failure while reading the
response means the request did reach the worker, so it is retried only for hooks with no observable
effect — currently `describe`. Retrying a delivered `request`/`response` hook would run its side
effects twice.

#### Realtime Capability

If a plugin declares `{"id": "realtime@1"}`, the descriptor exposes a `RealtimeDescriptor` with the
capability/protocol identifier `realtime@1`; this is not a separately versioned plugin package.
WebSocket path, heartbeat interval, and capacity. The dev server uses this to wire up realtime
connections.

### Under the Hood

- The built-in CORS layer is **not** `tower_http::cors::CorsLayer` — it is a hand-written service to
  keep dependencies minimal and give precise control over preflight handling and the `Vary` header.
- The rate limiter is **in-process only**. There is no Redis backend, no `RateLimitStore` trait.
  High-cardinality token buckets (10k+) trigger a full sweep of expired entries.
- Compression is applied **to all routes unconditionally** but only activates on responses with a
  known content-length. Streaming and chunked responses are not run through the async compression
  adapter. The current server source does not provide an SSE endpoint; do not infer SSE support from
  this compression rule.
- Plugin workers are **not restarted automatically** after a failed hook unless the process itself
  died or the protocol stream was poisoned. Application-level errors are returned to the caller
  without process replacement.
- The pool size fan-out to >1 workers only happens when the registry declares at least one HTTP
  hook. A build-only plugin sees a single worker regardless of the configured pool size.

---

## Worker Pool · กลุ่มผู้ทำงาน

**Modules**: `crates/ruvyxa_dev_server/src/worker_pool.rs`,
`packages/ruvyxa/runtime/worker-pool.mjs` **Crate**: `ruvyxa_dev_server`

### สรุป

Worker pool คือกลุ่ม process ของ Node/Bun ที่อยู่ยาว ทำหน้าที่รัน JavaScript ทั้งหมดของแอป — SSR,
SSG, API routes, server actions, client bundle — โดยคุยกับฝั่ง Rust ผ่าน NDJSON บน stdin/stdout ฝั่ง
Rust เป็นผู้เลือก worker และจัดการ lifecycle ส่วนฝั่ง Node เป็นผู้จำกัด concurrency ภายในตัวเอง

---

### Why Separate Processes

Rendering has to happen in a JavaScript runtime: it calls React, the app's own modules, and whatever
those import. That rules out doing it inside the Rust process. The remaining choice is how the
JavaScript runs.

- **A process per request** pays full Node startup plus a cold module graph on every render.
- **One long-lived process** keeps the bundle and module caches warm but serializes every render
  behind one event loop, and one bad render takes the whole server with it.
- **A pool of long-lived processes** (chosen) amortizes startup, keeps caches warm per worker,
  renders in parallel across processes, and contains a crash to one worker.

Each worker is an OS process, so a segfault, an OOM kill, or a runaway render is isolated and its
slot can be replaced without touching the others.

---

### Protocol

One JSON object per line, in both directions. Every request carries an `id`; the response echoes it,
which is what lets several requests be in flight on one worker's stdin at once.

Request types (`WorkerRequest`, serialized with `#[serde(tag = "type")]`):

| Type           | Purpose                                    | Idempotent |
| -------------- | ------------------------------------------ | ---------- |
| `ssr`          | Render a page for a request path           | ✅         |
| `ssg`          | Render a page for prerendering             | ✅         |
| `staticParams` | Resolve `generateStaticParams` for a route | ✅         |
| `client`       | Build the browser bundle for a route       | ✅         |
| `api`          | Execute an API route handler               | ❌         |
| `action`       | Execute a server action                    | ❌         |
| `warmup`       | Pre-populate the bundle cache              | ✅         |
| `invalidate`   | Drop cache entries for changed files       | ✅         |
| `ping`         | Health check and worker statistics         | ✅         |

`is_idempotent()` is the single source of truth for what may be retried. `api` and `action` are
excluded because re-running them would duplicate side effects.

An `api` response is **framed** rather than a single message: `api-start`, then any number of
`api-chunk` frames (body bytes base64-encoded, 64 KiB each), then a terminal frame. This is what
makes streaming responses possible across the boundary. `WorkerResponse::is_terminal()` decides when
a response is complete, and `header_pairs` is preferred over the legacy `headers` map so repeated
`Set-Cookie` values survive.

---

### Pool Size

```rust
const DEFAULT_POOL_SIZE: usize = 4;
const MIN_POOL_SIZE: usize = 2;
const MAX_POOL_SIZE: usize = 8;
```

The dev and production servers use `available_parallelism()` clamped to 2–8, overridable with
`RUVYXA_WORKER_POOL_SIZE`. A build passes an explicit count instead, clamped to 1–8: a build with
one prerender job should not start an idle second process just because a long-lived server wants a
higher minimum.

Workers are spawned concurrently — each spawn does blocking process setup — and the pool pings
worker 0 before returning, so a pool that cannot execute JavaScript fails at startup rather than on
the first request.

---

### Worker Selection

```rust
async fn select_worker(&self) -> Result<(usize, Arc<Worker>)>
```

Least-loaded by in-flight count, with a rotating start offset to break ties, short-circuiting on the
first idle worker.

Blind round-robin would ignore load: a burst can stack several requests behind one worker still
blocked in a CPU-bound `renderToString` while a sibling sits idle. The rotating offset preserves
round-robin behaviour when every worker is equally idle.

The `Arc<Worker>` handles are cloned out of the lock before probing, so worker replacement cannot
race with a selection snapshot.

---

### Per-Worker Concurrency

The Rust side bounds its stdin channel but has no view of how much work is in flight _inside_ a
worker, so the limit lives in `worker-pool.mjs`:

```js
const MAX_CONCURRENT_REQUESTS = positiveIntegerEnv(
  'RUVYXA_WORKER_MAX_CONCURRENCY',
  Math.max(2, Math.min(8, availableParallelism())),
)
```

A request awaits `acquireRequestSlot()` before dispatch and calls `releaseRequestSlot()` when it
finishes, which starts the longest-waiting queued request. Below the limit the acquire resolves
synchronously, so the common case does not wait in the request-slot queue. This is a control-flow
property, not a measured guarantee about wall-clock latency.

Renders are CPU-bound and each one holds a React tree, a compiled bundle, and its response buffer.
Admitting a whole burst at once exhausts the heap or thrashes the CPU into timeouts that present as
hangs. `invalidate` and `ping` bypass the queue — delaying a cache invalidation would leave the
worker serving stale bundles exactly when it is busiest, and a health check that queues behind
renders cannot report on them.

Concurrent `ssr` and `ssg` requests for the same page are also **coalesced**: the second await joins
the first render's promise rather than starting its own (`renderCoalesceMap`).

---

### Failure Recovery

```rust
pub async fn send(&self, request: WorkerRequest) -> Result<WorkerResponse>
async fn replace_failed_worker(&self, index: usize, failed: &Arc<Worker>) -> Option<Arc<Worker>>
```

| Scenario                                   | Behavior                                                 |
| ------------------------------------------ | -------------------------------------------------------- |
| Worker returns an error response           | Propagated to the caller; worker left alive              |
| Send/receive fails, request idempotent     | Worker replaced, request retried once on the replacement |
| Send/receive fails, request not idempotent | Worker replaced, error returned                          |
| Response times out                         | Error returned; the Node watchdog fails the request too  |
| Replacement cannot spawn                   | Logged; the pool continues at reduced capacity           |

The worker slot is replaced before the retry decision, so the failed worker cannot be selected again
by a concurrent request. A streaming API response that was already in flight when its worker exited
is delivered a body error (`RUV1704`) rather than a clean EOF, so a truncated response is never
mistaken for a complete one.

---

### Module Graph Retention and Recycling

Production prerendering uses `render_ssg_isolated`, which imports the bundle under a **fresh module
URL** so page-module state cannot leak between paths. Node's ESM registry never releases a loaded
URL, so each isolated import permanently retains one more module graph. No cache eviction inside the
worker can reclaim it — the worker tracks the cost in `registeredModuleUrls` and reports it through
`ping`, but replacing the process is the only operation that frees it.

```rust
const DEFAULT_ISOLATED_RENDERS_PER_WORKER: usize = 32;
const ISOLATED_RENDER_RECYCLE_ENV: &str = "RUVYXA_PRERENDER_RECYCLE_AFTER";

fn retains_an_isolated_module_graph(&self) -> bool  // Ssg { fresh: true }
async fn retire_worker_if_saturated(&self, index: usize, worker: &Arc<Worker>)
```

A build worker is retired once it has served its budget of isolated renders. Two conditions guard
it:

- **Only isolated renders count.** A normal `ssg` render reuses a cached module URL and retains
  nothing.
- **Only an idle worker is retired.** `shutdown` clears pending requests, so retiring a busy worker
  would fail sibling renders that were progressing fine. Deferring costs nothing — the counter stays
  over budget until the worker is next idle.

If the replacement cannot spawn, the saturated worker is kept and its counter reset: losing pool
capacity mid-build is strictly worse than carrying the retained graphs.

The dev server passes `None`, disabling recycling. It never requests isolated imports, so it retains
nothing to reclaim and pays nothing for the bound. `RUVYXA_PRERENDER_RECYCLE_AFTER=0` disables it
for builds too.

---

### In-Worker Caches

| Cache           | Bound                            | Evicted by                         |
| --------------- | -------------------------------- | ---------------------------------- |
| `bundleCache`   | `RUVYXA_CACHE_MAX_ENTRIES` (256) | LRU, `invalidate`, memory pressure |
| `moduleCache`   | `RUVYXA_CACHE_MAX_ENTRIES` (256) | LRU, memory pressure               |
| Compiler cache  | Managed by `compiler.mjs`        | `invalidate`, memory pressure      |
| ESM module URLs | **Unbounded** (see above)        | Nothing — process replacement only |

A 30-second interval checks `heapUsed` against `RUVYXA_MEMORY_LIMIT_MB` (512) and, above it, evicts
half of `bundleCache`, clears `moduleCache`, and clears the compiler cache. The timer is `unref`'d
so it never keeps the process alive.

---

### Shutdown

`shutdown(reason)` writes the reason to stderr, refuses new admissions, clears the admission queue,
and exits once active requests drain — or after 5 seconds regardless. `SIGTERM` and `SIGINT` both
route through it. On the Rust side, `NodeWorkerPool::shutdown()` closes each worker's stdin, clears
its pending responses, and waits up to 2 seconds for the child to exit before terminating it, so a
wedged worker cannot hold up server shutdown.

---

### Environment Variables

| Variable                         | Default              | Effect                                                     |
| -------------------------------- | -------------------- | ---------------------------------------------------------- |
| `RUVYXA_WORKER_POOL_SIZE`        | CPU count (2–8)      | Worker processes in the dev/prod pool                      |
| `RUVYXA_WORKER_MAX_CONCURRENCY`  | CPU count (2–8)      | Requests one worker executes at once                       |
| `RUVYXA_WORKER_TIMEOUT_MS`       | 30000 / 300000 build | Per-request deadline, shared by Rust and the Node watchdog |
| `RUVYXA_PRERENDER_RECYCLE_AFTER` | 32 (`0` disables)    | Isolated prerenders before a build worker is retired       |
| `RUVYXA_CACHE_MAX_ENTRIES`       | 256                  | Bundle and module cache entries per worker                 |
| `RUVYXA_MEMORY_LIMIT_MB`         | 512                  | Heap threshold that triggers in-worker cache eviction      |

---

### Observability

`ping` returns the state a stuck pool needs to be diagnosed from the outside:

| Field                   | Meaning                                                                            |
| ----------------------- | ---------------------------------------------------------------------------------- |
| `activeRequests`        | Requests currently executing                                                       |
| `queuedRequests`        | Requests parked on a slot — persistently non-zero means the pool is the bottleneck |
| `maxConcurrentRequests` | The admission limit in effect                                                      |
| `cacheSize`             | Bundle cache entries                                                               |
| `moduleCacheSize`       | Module cache entries                                                               |
| `retainedModuleUrls`    | Module graphs the ESM registry cannot free                                         |
| `coalesceMapSize`       | In-flight renders being shared by more than one request                            |

---

### Why This Design

1. **Processes, not threads** — JavaScript needs a runtime; a process is the only unit that isolates
   one crashing render from the rest of the pool.
2. **NDJSON over stdin/stdout** — No port to bind, no socket to secure, no serialization format to
   version beyond JSON. Works identically for Node and Bun.
3. **Request IDs, not request/response lockstep** — Several requests occupy one worker concurrently,
   which is what makes a data-fetching render's I/O wait useful instead of idle.
4. **Least-loaded selection** — A single slow render cannot serialize unrelated requests behind it.
5. **Retry gated on idempotency** — Recovery never turns one action into two.
6. **Recycling gated on isolation and idleness** — The only unbounded resource is bounded, without
   dropping work in progress or charging the dev server for a cost it does not incur.

---

## Concurrency Model · โมเดลการทำงานพร้อมกัน

**Scope**: Cross-crate (dev server, bundler, diagnostics)

### สรุป

Ruvyxa uses three distinct concurrency domains: (1) async Tokio for I/O, (2) dedicated OS threads
for SSR rendering, (3) parallel compilation via rayon for bundling. Each domain is designed for its
workload — no one-size-fits-all runtime.

---

### Domain 1: Async I/O (Tokio)

#### Where

- Dev server HTTP accept loop
- HMR WebSocket connections
- Static file serving
- Server action handlers

#### Mechanism

```rust
use tokio::net::TcpListener;
use axum::{Router, serve};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handler));

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    serve(listener, app).await.unwrap();
}
```

#### Runtime

Multiple Tokio runtimes exist:

| Runtime            | Scope                | Thread count                                  |
| ------------------ | -------------------- | --------------------------------------------- |
| Main runtime       | Dev server, CLI      | Multi-thread (default: available_parallelism) |
| Per-worker runtime | SSR rendering thread | Current-thread (1 worker = 1 runtime)         |
| Build runtime      | CLI build pipeline   | Current-thread (sequential)                   |

#### Why Tokio (not smol, monoio)

- Axum is built on Tokio. Using a different async runtime would require a bridging layer.
- Tokio's work-stealing scheduler distributes I/O across available cores. One accept loop does not
  starve others.
- `tokio::task::spawn_blocking` is available for CPU-bound operations that cannot be async (e.g.,
  heavy JSON serialization).

---

### Domain 2: Dedicated Thread Pool (SSR Rendering)

#### Where

- React `renderToString` / `renderToPipeableStream`
- Build-time static page generation (`ruvyxa build` for SSG routes)

#### Mechanism

```rust
pub struct NodeWorkerPool {
    workers: StdRwLock<Vec<Arc<Worker>>>,
    worker_script: PathBuf,
    env: BTreeMap<String, String>,
    runtime: JavaScriptRuntime,
    next_worker: AtomicU64,
    response_timeout: Duration,
    isolated_renders_per_worker: Option<usize>,
}
```

#### Why Not Tokio `spawn_blocking`

Rendering does not happen in this process at all. Each worker is a long-lived Node or Bun
**subprocess** speaking JSON lines over stdin/stdout, so the work is neither a blocking Rust call
nor something `spawn_blocking` could host. Keeping the processes alive is the point: spawning a
runtime per render dominated the render itself.

#### Dispatch

- `next_worker.fetch_add(1, Relaxed) % workers.len()` — round-robin, no lock on the hot path.
- `REQUEST_COUNTER` stamps each request with a monotonic id; `pending` maps that id to the caller
  awaiting it, so one worker can have several renders in flight over a single pipe.
- Every wait is a `tokio::time::timeout(response_timeout, …)`. A worker that stops answering fails
  its own request instead of stalling the server; the dev server and the build use different
  timeouts.
- `isolated_renders_per_worker` retires a worker after N isolated prerenders so per-render module
  graphs cannot accumulate. It is `None` for the dev server, which never requests isolated imports.

---

### Domain 3: Parallel Compilation (rayon)

#### Where

- `compiler.rs` — one job per module in a resolved graph
- `resolver.rs` — resolving a module's specifiers
- `chunking.rs` — planning and emitting chunks
- `linker.rs` — `link_parallel`, wrapping modules before concatenation
- `ruvyxa_cli/image_optimizer.rs` — one job per generated image variant

#### Mechanism

```rust
use rayon::prelude::*;

let results: Vec<Result<CompiledModuleOutput>> = graph
    .par_iter()
    .map(|module| compile_module(module, input, cache, build_hooks))
    .collect();
```

Errors are collected rather than short-circuited, so one failing module does not race the others
into an arbitrary winner: the whole batch completes, then the first error is returned.

#### Why rayon (not manual threads)

- Work-stealing: if one file takes longer to compile, other threads steal remaining work.
- Global thread pool: rayon maintains a thread pool matching `available_parallelism`. No thread
  creation overhead per `par_iter` call.
- No `Send + Sync` gymnastics: rayon handles data distribution.

#### Parallelism Boundaries

| Phase                 | Parallelism                     | Method                    |
| --------------------- | ------------------------------- | ------------------------- |
| Route discovery       | Sequential (single walk)        | Not parallelizable        |
| Module compilation    | File-level parallel             | rayon `par_iter`          |
| Module linking        | Sequential (IIFE concatenation) | Single-threaded           |
| Boundary checking     | Module-graph BFS                | Sequential (single graph) |
| Static page rendering | Path-level parallel             | Worker pool dispatch      |
| Minification          | File-level parallel             | rayon `par_iter`          |

---

### Domain 4: Shared State Concurrency

Every lock below is a `tokio::sync` lock held across `.await` points, not a `std::sync` one.

#### Render cache

```rust
pub struct RenderCache {
    entries: RwLock<HashMap<Arc<str>, CacheEntry>>,
    order: RwLock<RecencyList>,
    capacity: usize,
    ttl: Duration,
    hits: AtomicU64,
    misses: AtomicU64,
}
```

- Two locks, not one: a cache hit takes `entries` for reading and `order` for writing (to promote
  the key). A single lock would serialize concurrent hits on different keys.
- `hits`/`misses` are `AtomicU64` with `Relaxed` ordering — they are reported, never branched on.
- The two maps must agree on which keys exist. Any operation that removes a key holds **both** locks
  for the whole removal. Releasing `entries` first leaves a window where a concurrent `put` of the
  same key re-inserts it and pushes it onto `order`, after which the removal drops it from `order`
  alone — an entry eviction can never reach again.

#### HMR dependency tracker

```rust
Arc<RwLock<BTreeMap<PathBuf, BTreeSet<String>>>>   // file  → route ids
Arc<RwLock<BTreeMap<String, BTreeSet<PathBuf>>>>   // route → files
```

Two directions of the same relation. Readers (a file-change event resolving affected routes)
outnumber writers (a render recording what it touched), so `RwLock` rather than `Mutex`.

#### Worker pool

```rust
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

pending: Mutex<BTreeMap<String, PendingResponse>>,
child:   Mutex<Option<Child>>,
```

- Request IDs are monotonic and generated without a lock; they are never reused.
- `pending` correlates a response line from the worker's stdout with the caller awaiting it.
  `Mutex`, not `RwLock`: every access mutates.
- `child` is the subprocess handle, held so a restart cannot race two live workers.

#### Lock ordering

```
Acquire order (never the reverse):
  1. RenderCache::entries
  2. RenderCache::order
```

That is the only place in the dev server where two locks are held at once, so it is the only
ordering that has to be maintained. Every other lock above is a leaf: it is taken, used, and
released without acquiring another.

---

### Concurrency by Crate

| Crate                | Sync primitives                             | Threading model                   |
| -------------------- | ------------------------------------------- | --------------------------------- |
| `ruvyxa_graph`       | None (single-threaded, `&mut` caches)       | Sequential                        |
| `ruvyxa_bundler`     | `rayon`                                     | Parallel (file-level)             |
| `ruvyxa_dev_server`  | `tokio` `RwLock`/`Mutex`, atomics, channels | Mixed (async I/O + worker pool)   |
| `ruvyxa_middleware`  | `Arc<PluginHost>`                           | Async (Tokio tower layers)        |
| `ruvyxa_diagnostics` | None (owned values)                         | Sequential                        |
| `ruvyxa_cli`         | Tokio runtime                               | Async (main) + sequential (build) |

---

### Why This Design

1. **Three separate concurrency strategies** — Async I/O, thread pool rendering, and parallel
   compilation each have different optimal strategies. A single `#[tokio::main]` for everything
   would bottleneck CPU-bound SSR rendering against I/O-bound HTTP serving.
2. **Bounded channels for backpressure** — When the system is overloaded, bounded channels cause the
   accept loop to block, which causes the OS to queue TCP connections. This is the correct behavior
   — better to queue at the network layer than to accept requests that will time out.
3. **`rayon` over async compilation** — Compilation is pure CPU work with no I/O waiting. `rayon` is
   more efficient than `tokio::task::spawn_blocking` for CPU parallelism because it uses
   work-stealing and avoids Tokio's task scheduling overhead.
4. **`RwLock` where reads dominate** — HMR dependency lookups and render-cache hits are read-mostly
   and concurrent. A `Mutex` would serialize them; `RwLock` lets them proceed together and reserves
   exclusion for the rare write.

---

## Protocols · โพรโทคอล

**Scope**: Cross-crate (dev server HMR, server actions, client module protocol)

### สรุป

สาม wire protocols: (1) HMR WebSocket สำหรับ hot reload, (2) Server Action HTTP สำหรับ form/action
submissions, (3) Client Module HTTP สำหรับ lazy-loading client-side bundles

---

### 1. HMR WebSocket Protocol

#### Endpoint

```
ws://<host>:<port>/_ruvyxa/hmr
```

#### Server → Client Messages

```json
{
  "type": "hot",
  "data": {
    "module": "/src/app/blog/[slug]/page.tsx",
    "route": "/blog/:slug",
    "timestamp": 1719536400000
  }
}
```

| `type`         | Meaning                | Client behavior                                                 |
| -------------- | ---------------------- | --------------------------------------------------------------- |
| `hot`          | Module changed         | Re-import the module via dynamic `import()`, re-render in place |
| `full-reload`  | Config/root change     | `window.location.reload()`                                      |
| `style-update` | CSS changed            | Swap `<link>` href, inject new `<style>`                        |
| `error`        | Compile/boundary error | Show error overlay (red banner with diagnostics)                |
| `connected`    | Initial handshake      | Client sends its current manifest hash for diff                 |
| `state-sync`   | Full manifest delta    | Replace module registry entries                                 |

#### Client → Server Messages

```json
{
  "type": "manifest-hash",
  "hash": "a1b2c3d4e5f6..."
}
```

Sent on `connected` acknowledgment. Server compares hash with current manifest; if different, sends
`state-sync` with the delta.

#### Connection Lifecycle

```
Client opens WebSocket
  → Server sends { type: "connected" }
  → Client sends { type: "manifest-hash", hash }
  → Server sends delta or full manifest if hash mismatch
  → Loop: Server pushes update events
  → On disconnect: Client retries every 1s (exponential backoff, max 30s)
```

---

### 2. Server Action Protocol

#### Request

```
POST /_ruvyxa/action/{action_name}
Content-Type: application/json
```

```json
{
  "args": [arg1, arg2, ...],
  "headers": {
    "content-type": "application/json"
  }
}
```

#### Response (Success)

```
Status: 200
Content-Type: application/json
```

```json
{
  "data": {/* action return value */}
}
```

#### Response (Error)

```
Status: 500
Content-Type: application/json
```

```json
{
  "error": "ActionError",
  "message": "Something went wrong",
  "code": "RUV1500"
}
```

#### Action Discovery

Server actions are defined in `app/**/action.ts`:

```typescript
// app/contact/action.ts
export async function submitForm(prev: unknown, formData: FormData) {
  'use server'
  const name = formData.get('name')
  // ...validation, database...
  return { success: true }
}
```

The CLI discovers action modules during route discovery and registers them at server start. Each
action is bound to its URL namespace: `app/contact/action.ts → submitForm` is available at
`/action/contact/submitForm`.

#### Security

- Actions are POST-only. GET returns 405.
- CSRF protection via `Ruvyxa-Action` header (must match action name).
- Origin check: `Origin` header must match allowed origins.
- Rate limiting: applies to action endpoints when `RateLimitConfig` is enabled.

---

### 3. Client Module Protocol

#### Request

```
GET /_ruvyxa/client/{module_path}
```

`module_path` is URL-encoded relative path from project root. Example:
`GET /_ruvyxa/client/src%2Fapp%2Fpage.tsx`

#### Response

```
Status: 200
Content-Type: application/javascript
Cache-Control: public, max-age=31536000, immutable
```

```
(function(__ruvyxa) {
  // compiled IIFE module code
  __ruvyxa.define("src/app/page.tsx", function(module, exports) {
    // ...
  });
})(__ruvyxa);
```

#### Module Registry (Browser-side)

```javascript
// Runtime: injected into every HTML page
window.__ruvyxa = {
  registry: new Map(),
  define(name, factory) {
    this.registry.set(name, factory)
  },
  require(name) {
    if (!this.registry.has(name)) {
      throw new Error(`Module not loaded: ${name}`)
    }
    const module = { exports: {} }
    this.registry.get(name)(module, module.exports)
    return module.exports
  },
  load(path) {
    if (this.registry.has(path)) return Promise.resolve()
    return new Promise((resolve, reject) => {
      const script = document.createElement('script')
      script.src = `/_ruvyxa/client/${encodeURIComponent(path)}`
      script.onload = resolve
      script.onerror = reject
      document.head.appendChild(script)
    })
  },
}
```

#### Lazy Loading

HMR `hot` events trigger:

```javascript
// Client-side HMR handler
async function applyHotUpdate(modulePath, routeId) {
  // 1. Fetch updated module (no-cache to bypass immutable cache)
  const url = `/_ruvyxa/client/${encodeURIComponent(modulePath)}?_=${Date.now()}`
  await window.__ruvyxa.load(url)

  // 2. Re-render route component
  const Component = window.__ruvyxa.require(modulePath).default
  // re-render logic...
}
```

---

### 4. Render Proxy Protocol

#### Internal Use Only

Dev server clients proxy SSR renders through `/_ruvyxa/render`:

```
POST /_ruvyxa/render
Content-Type: application/json
```

```json
{
  "route": "/blog/hello-world",
  "method": "GET",
  "headers": {
    "accept": "text/html"
  }
}
```

Response: rendered HTML (streamed via chunked transfer encoding). This proxy is used internally by
the dev server worker pool. Not exposed to external clients.

---

### Protocol Comparison

| Protocol      | Transport | Encoding          | Direction                  | Caching        |
| ------------- | --------- | ----------------- | -------------------------- | -------------- |
| HMR           | WebSocket | JSON              | Bidirectional              | None           |
| Server Action | HTTP POST | JSON              | Client → Server (result ←) | None           |
| Client Module | HTTP GET  | JavaScript (IIFE) | Server → Client            | Immutable (1y) |
| Render Proxy  | HTTP POST | HTML              | Internal                   | LRU            |

---

### Why This Design

1. **WebSocket for HMR, not SSE** — HMR needs bidirectional messages. SSE is unidirectional
   server→client. WebSocket allows the client to send `manifest-hash` on reconnect for state
   negotiation.
2. **Immutable client module caching** — Client bundles are content-addressed by module path.
   `Cache-Control: immutable` ensures the browser never re-fetches a module that hasn't changed. HMR
   bypasses this with a cache-busting `_` query param.
3. **Action as POST-only** — Server actions mutate state. HTTP POST is the correct semantics. GET
   requests to action endpoints are rejected to prevent CSRF via `<img>` tags.
4. **Module registry over ESM** — ES modules (`<script type="module">`) have cross-origin and CORS
   complications with HMR. A synchronous `__ruvyxa` registry is simpler and works with the IIFE
   output format.

---

## Site Discovery & Image Optimization

**Source**: `crates/ruvyxa_cli/src/{site_discovery,image_optimizer,image_usage}.rs`

Two build-time subsystems: crawler-discovery file generation (robots.txt, sitemap.xml) and public
image optimization (PNG/JPEG → WebP + responsive variants).

---

### SiteConfigOptions

The `site` block in `ruvyxa.config.ts`. Deserialized from camelCase JSON with `deny_unknown_fields`.

```rust
#[derive(Debug, Default, Clone, Deserialize)]
pub struct SiteConfigOptions {
    pub url: Option<String>,       // absolute origin, e.g. "https://ruvyxa.dev"
    pub sitemap: SitemapSetting,   // bool or SitemapGenerationOptions, default true
    pub robots: RobotsSetting,     // bool or RobotsGenerationOptions, default true
}
```

#### SitemapSetting

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

#### SitemapGenerationOptions

```rust
#[derive(Debug, Default, Clone, Deserialize)]
pub struct SitemapGenerationOptions {
    pub exclude: Vec<String>,            // exact paths or trailing-`*` prefixes
    pub additional_paths: Vec<String>,   // paths not inferable from route manifest
    pub defaults: SitemapEntryMetadata,  // metadata applied to every entry
    pub entries: Vec<SitemapEntryOptions>, // per-URL overrides
}
```

#### SitemapEntryMetadata & SitemapEntryOptions

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

#### SitemapChangeFrequency

```rust
enum SitemapChangeFrequency {
    Always, Hourly, Daily, Weekly, Monthly, Yearly, Never,
}
```

#### RobotsSetting & RobotsGenerationOptions

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

### URL Resolution

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

### Sitemap Generation

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

### Robots.txt Generation

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

### ImageOptimizationOptions

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ImageOptimizationOptions {
    pub optimize: bool,                    // enable optimization, default true
    pub quality: u8,                       // 1–100, default 82
    pub lossless: bool,                    // lossless WebP encoding, default false
    pub keep_original: bool,               // keep original beside WebP, default true
    pub variant_widths: Vec<u32>,          // responsive breakpoints
    pub parallelism: usize,                // 0 = Rayon global pool
    pub on_demand: OnDemandImageOptions,   // optional runtime resize policy
}
```

`on_demand` accepts `false`, `true`, or `{ enabled, maxWidth }`. It is disabled by default; when
enabled, the dev/server runtime accepts only bounded same-origin public-image requests and emits
WebP. `maxWidth` defaults to 3840 and config validation restricts it to 16–8192.

#### Default variant widths

```rust
pub const DEFAULT_VARIANT_WIDTHS: [u32; 8] = [640, 750, 828, 1080, 1200, 1920, 2048, 3840];
```

Must stay identical to `DEFAULT_DEVICE_WIDTHS` in `packages/@ruvyxa/react/src/image.tsx`. Test
`packages/@ruvyxa/react/test/image-variants.test.mjs` asserts agreement.

---

### optimize_public_images()

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

#### ImageOptimizationReport

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

### scan_raw_image_usage()

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

### Output Structure

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

### Under the Hood

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

---

## Diagnostics · การวินิจฉัย

**Crate**: `ruvyxa_diagnostics` **Module**: `crates/ruvyxa_diagnostics/src/lib.rs`

### สรุป

Central diagnostic types for the Ruvyxa framework. `Diagnostic` carries a structured error with
source span, import chain, suggested fix, and affected routes. `RuvyxaError` is the unified error
enum — wraps `Diagnostic`, `std::io::Error`, or a plain `String`. SARIF 2.1.0 serialization for CI
integration (GitHub Code Scanning, GitLab SAST).

---

### Core Data Structures

#### SourceSpan

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
}
```

Points to a source file, optionally with line/column. Both positional fields are `Option` — a bare
file reference (e.g. missing file) is valid.

#### Diagnostic

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub title: String,
    pub explanation: String,
    pub span: Option<SourceSpan>,
    pub import_chain: Vec<PathBuf>,
    pub suggested_fix: Option<String>,
    pub affected_routes: Vec<String>,
}
```

| Field             | Type                 | Purpose                                    |
| ----------------- | -------------------- | ------------------------------------------ |
| `code`            | `&'static str`       | Error code, e.g. `"RUV1001"`               |
| `title`           | `String`             | Short human-readable summary               |
| `explanation`     | `String`             | Long-form why-this-happened                |
| `span`            | `Option<SourceSpan>` | Source location (file + optional line/col) |
| `import_chain`    | `Vec<PathBuf>`       | Import trace for boundary violations       |
| `suggested_fix`   | `Option<String>`     | How to resolve the issue                   |
| `affected_routes` | `Vec<String>`        | Routes impacted by this error              |

---

### Builder Pattern

```rust
Diagnostic::new(code, title)
    .explain("why")                        // set explanation
    .at_file("path/to/file.rs")            // set span, no line/col
    .at_file_with_span("path.rs", 42, 5)   // set span with line + col
    .suggest("move the import")            // set suggested_fix
```

Each builder method consumes and returns `Self` (not `&mut self`), enabling chaining. `at_file` and
`at_file_with_span` overwrite the span. All methods are additive — no validation or side effects.

---

### Display Format

```
CODE: title
File: /path/to/file.rs:42:5

Why:
  explanation text

Fix:
  suggested fix text

Affected routes:
  /blog/[slug]
  /about
```

Span line omission adjusts format: no line → `File: path`, line without column → `File: path:line`.
Sections omitted when empty.

---

### RuvyxaError

```rust
#[derive(Debug, Error)]
pub enum RuvyxaError {
    #[error("{0}")]
    Diagnostic(Box<Diagnostic>),

    #[error("{message}")]
    Io {
        message: String,
        #[source]
        source: std::io::Error,
    },

    #[error("{0}")]
    Message(String),
}
```

Three variants:

- **Diagnostic** — wraps a `Box<Diagnostic>`, delegates `Display` to Diagnostic's formatter.
- **Io** — structured I/O error preserving the source `std::io::Error`.
- **Message** — plain string fallback.

#### Trait Impls

```rust
impl From<Diagnostic> for RuvyxaError   // wraps into Diagnostic variant
impl From<std::io::Error> for RuvyxaError // wraps into Io variant
pub type Result<T> = std::result::Result<T, RuvyxaError>;
```

---

### SARIF Integration

```rust
pub fn diagnostics_to_sarif(
    diagnostics: &[Diagnostic],
    tool_name: &str,
    tool_version: &str,
    project_root: &Path,
) -> serde_json::Value
```

Produces SARIF 2.1.0:

```json
{
  "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "ruvyxa",
          "version": "0.1.0",
          "informationUri": "https://github.com/ruvyxa/ruvyxa",
          "rules": [
            {
              "id": "RUV1001",
              "name": "RUV1001",
              "shortDescription": { "text": "Private import" },
              "fullDescription": { "text": "A client module imports server-only code." },
              "defaultConfiguration": { "level": "error" },
              "help": { "text": "Move the import behind a server boundary." }
            }
          ]
        }
      },
      "results": [
        {
          "ruleId": "RUV1001",
          "level": "error",
          "message": { "text": "Private import: A client module imports server-only code." },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": { "uri": "app/page.tsx" },
                "region": { "startLine": 4, "startColumn": 7 }
              }
            }
          ],
          "properties": {
            "suggestedFix": "Move the import behind a server boundary.",
            "affectedRoutes": [],
            "importChain": []
          }
        }
      ]
    }
  ]
}
```

Key behavior:

- **Rules deduplicated** by `code` via `BTreeMap` — preserves insertion order, keeps first
  occurrence.
- **URIs project-relative** — each file path is stripped of `project_root` prefix after
  normalization.
- **`help` omitted** when `suggested_fix` is `None`.
- **`region` omitted** when span lacks line/column.
- **`properties`** carries `suggestedFix`, `affectedRoutes`, `importChain` as supplemental data.

---

### Error Code Catalog

| Code    | Title                                                           | Crate             |
| ------- | --------------------------------------------------------------- | ----------------- |
| RUV1001 | App directory was not found                                     | graph             |
| RUV1002 | Invalid dynamic route segment / Catch-all must be final segment | graph             |
| RUV1003 | Conflicting route paths                                         | graph             |
| RUV1004 | Page is missing a default export                                | graph, dev_server |
| RUV1007 | Server-only module imported into client graph                   | graph             |
| RUV1008 | Private environment variable used in client graph               | graph             |
| RUV1009 | Client-only module imported into server/SSR graph               | graph, bundler    |
| RUV1010 | Server directory module reached by client graph                 | graph             |
| RUV1100 | React SSR failed                                                | dev_server        |
| RUV1102 | SSR renderer was not found                                      | dev_server        |
| RUV1200 | API route execution failed                                      | dev_server        |
| RUV1201 | No available server port was found                              | dev_server        |
| RUV1202 | API renderer was not found                                      | dev_server        |
| RUV1300 | Client hydration bundling failed / Compile error                | dev_server        |
| RUV1303 | Client route was not found                                      | dev_server        |
| RUV1304 | Client bundle requested for a non-page route                    | dev_server        |
| RUV1400 | Tailwind CSS compilation failed                                 | dev_server        |
| RUV1401 | Tailwind CSS CLI was not found                                  | dev_server        |
| RUV1402 | Sass compilation failed                                         | dev_server        |
| RUV1403 | CSS import / stylesheet could not be resolved                   | dev_server        |
| RUV1404 | CSS entry must stay inside the project root                     | dev_server        |
| RUV1500 | SSG render failed                                               | dev_server        |
| RUV1501 | Route action file was not found                                 | dev_server        |
| RUV1550 | PPR render failed                                               | dev_server        |
| RUV1702 | Worker pool script was not found                                | dev_server        |
| RUV1101 | SSR renderer received missing required arguments                | runtime/SSR       |

Codes are string constants (`&'static str`), not enum variants — any crate can emit any code without
touching the diagnostics crate.

---

### Under the Hood

#### `normalized_canonical_path`

```rust
pub fn normalized_canonical_path(path: &Path) -> PathBuf
```

Wraps `std::fs::canonicalize` then strips the Windows `\\?\` verbatim prefix on `cfg(windows)`.
Falls back to the original path when the file does not exist. Used inside SARIF serialization to
produce paths that JavaScript runtimes (Bun, Node) can pass to `pathToFileURL`.

#### SARIF rule deduplication

Uses `BTreeMap<&str, &Diagnostic>` keyed by `code`. Because `BTreeMap` iterates in key order, rules
are sorted alphabetically by code in the output. The first diagnostic for each code is used as the
rule template — subsequent diagnostics with the same code are still emitted as separate results but
reference the same rule.

#### Error scope

This crate owns only the type definitions and SARIF serializer. The actual error emission happens in
domain crates (`ruvyxa_graph`, `ruvyxa_bundler`, `ruvyxa_dev_server`) which construct `Diagnostic`
values directly via the builder pattern. There is no centralized error registry — codes are
conventional strings.

---

## Security · ความปลอดภัย

**Scope**: Cross-crate (middleware, CLI, dev server, graph validation)

### สรุป

Ruvyxa employs defense in depth: configurable CORS, rate limiting, CSRF protection for actions,
server/client boundary enforcement, private env var isolation, and origin validation.

---

### 1. CORS (Cross-Origin Resource Sharing)

#### Configuration

```rust
pub struct CorsConfig {
    pub allow_origins: Vec<String>,
    pub allow_methods: Vec<String>,   // default: GET, POST, PUT, DELETE, OPTIONS
    pub allow_headers: Vec<String>,   // default: Content-Type, Authorization
    pub expose_headers: Vec<String>,  // optional: Server-Timing, X-Ruvyxa-Debug
    pub max_age: Option<u64>,         // default: 600 seconds
    pub allow_credentials: bool,      // default: false
}
```

#### Runtime

`CorsLayer` wraps `tower_http::cors::CorsLayer`. Preflight `OPTIONS` is handled automatically.
Origin matching is strict (no wildcard `*` when credentials enabled).

#### Default (dev)

```json
{
  "cors": {
    "allowOrigins": ["http://localhost:3000"],
    "allowCredentials": true
  }
}
```

#### Default (build)

No CORS middleware unless explicitly configured (production API is expected to set `allowOrigins` to
the actual domain).

---

### 2. Rate Limiting

#### Configuration

```rust
pub struct RateLimitConfig {
    pub requests: u64,     // max requests per window
    pub window_secs: u64,  // time window in seconds
    pub key_by: String,    // "ip" (default) or "header:<name>"
}
```

`key_by` is a string, not an enum, because it crosses the TypeScript config boundary as one. Any
value that is not `header:<name>` falls back to the transport peer address.

That fallback is deliberate: forwarded headers are client-supplied. `X-Forwarded-For` is only
trusted when a deployment opts in explicitly with `key: "header:x-forwarded-for"`, behind a proxy
that overwrites it.

#### Default Store

In-memory: `Arc<Mutex<BTreeMap<String, …>>>` holding a window start and a counter per key, swept
lazily as requests arrive.

#### Response on Limit

```
Status: 429 Too Many Requests
Content-Type: application/json
Retry-After: <seconds>
```

```json
{
  "error": "Rate limit exceeded",
  "retryAfter": 30
}
```

`Retry-After` header tells the client when to retry.

---

### 3. CSRF Protection for Server Actions

All POST requests to `/_ruvyxa/action/*` require:

1. **Origin validation**: `Origin` header must match an allowed origin from
   `CorsConfig.allow_origins`. No `Origin` header → blocked (browsers always send `Origin` on
   cross-origin POST).
2. **`Ruvyxa-Action` header**: Must equal the action name being called. E.g.,
   `Ruvyxa-Action: submitForm`. Absent or mismatched → 403.
3. **Method check**: Only POST allowed. GET → 405.

---

### 4. Server/Client Boundary

Enforced at bundle time by `ruvyxa_bundler::boundary`:

```rust
pub fn check(modules: &[CompiledModule], input: &BundleInput, out: &mut Vec<Diagnostic>) -> Result<()>
```

| Check                                | Condition                                                                                              | Severity |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------ | -------- |
| Server-only module in client graph   | Import chain from client entry reaches a module containing `'server-only'`                             | Error    |
| Private `process.env` in client      | Client-accessible module accesses `process.env.<NAME>` where name does not start with `RUVYXA_PUBLIC_` | Error    |
| Client-only module in server graph   | Server-compiled module contains `'client-only'`                                                        | Error    |
| Server directory imports from client | Client import chain reaches `server/` directory                                                        | Error    |

These boundary checks prevent accidentally leaking server secrets, database code, or environment
variables to the browser.

---

### 5. Private Environment Variables

#### Convention

```typescript
// Public (safe for browser):
const apiUrl = process.env.RUVYXA_PUBLIC_API_URL

// Private (server-only):
const dbPassword = process.env.DATABASE_PASSWORD
```

#### Enforcement

During bundling, the compiler replaces `process.env.RUVYXA_PUBLIC_*` with the actual value in client
bundles.

Any `process.env.<NAME>` where `<NAME>` does not start with `RUVYXA_PUBLIC_` triggers RUV1008 if
found in a client-accessible module.

#### Implementation

There is no rewrite pass. The reads are **detected**, not replaced, and a private one in a client
graph fails the build instead of being silently blanked:

```rust
// crates/ruvyxa_bundler/src/boundary.rs
module.ast.env_reads
    .iter()
    .filter(|name| name.as_str() != "NODE_ENV" && !name.starts_with("RUVYXA_PUBLIC_"))
```

`NODE_ENV` is exempt because it is substituted at build time; `RUVYXA_PUBLIC_*` is public by
contract. The Node-side compiler (`packages/ruvyxa/runtime/compiler.mjs`) applies the identical
filter so the two halves of the build cannot disagree about what counts as private.

Failing rather than rewriting is the safer default: a blanked-out read turns a missing secret into a
confusing runtime `undefined`, while a build error names the file and the variable.

---

### 6. Plugin Security

Plugins from `ruvyxa.plugin.ts` run inside the server process. Safety measures:

1. **PluginHost isolates hooks** — `before_request` / `after_response` receive sanitized
   `PluginHttpRequest` objects (body capped at 1MB, headers audited).
2. **Plugin code is compiled** — Modified plugin files trigger HMR full-reload, not silent
   injection.
3. **No filesystem access** — Plugin hooks do not receive `fs` or `process` references. They mutate
   HTTP request/response data only.

---

### 7. Default Headers

Every response includes:

```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Content-Security-Policy: default-src 'self'
Referrer-Policy: strict-origin-when-cross-origin
```

Configurable via `ruvyxa.config.ts` `security.headers` field.

---

### 8. Validation at Route Discovery

#### Safe paths

```rust
fn route_segment(segment: &str, is_last: bool) -> Result<String>;
fn validate_dynamic_name(name: &str) -> Result<()>;
```

Route URLs are not user input — they are derived from directory names under `app/`, one segment at a
time by `route_path_from_dir`. Validation is therefore about well-formed routes, not traversal:

- `[name]`, `[...name]`, and `[[...name]]` are the only bracket forms; any other use of `[` or `]`
  is RUV1002.
- A parameter name must be non-empty, bracket-free, and must not begin with `.`, which is what keeps
  `[..]` and `[.]` from becoming traversal-shaped segments.
- A catch-all must be the final segment; a child segment after it is RUV1002.
- `(group)` and `@slot` segments are stripped from the URL rather than validated as parameters.

Traversal is prevented separately, at the point where a request path is turned back into a file:
prerender output paths go through `prerenderRelativePath`, which rejects anything escaping the
prerender root.

---

### Why This Design

1. **CORS + rate limiting in middleware, not the app** — Every response goes through the middleware
   stack. It's impossible to accidentally skip CORS or rate limiting by forgetting to add middleware
   in the route handler.
2. **Boundary checks at build time, not runtime** — A server-only import in a client module fails
   `ruvyxa build` with a clear error. There is no way to deploy a broken boundary. Runtime checks
   would miss half the imports (tree-shaken or lazy-loaded modules).
3. **Environment variable prefix convention** — `RUVYXA_PUBLIC_` is a visual marker. Developers can
   immediately tell which env vars are accessible to the browser just by reading the source code. No
   build-time env whitelist needed.
4. **Origin validation, not just CORS** — CORS headers tell the browser what to allow. Origin
   validation is the server enforcing the same policy. Defense in depth — if the browser ignores
   CORS (fetch with `mode: no-cors`), the server still blocks the request.

---

## Deployment Adapters · อาดาปเตอร์สำหรับการปรับใช้

**Scope**: Platform adapter packages (`@ruvyxa/adapter-*`)

### สรุป

Adapters transform Ruvyxa build output into platform-specific formats. Each adapter implements a
common interface: receive compiled bundles + route manifest → produce deployable artifact.

---

### Adapter Interface

```typescript
// packages/ruvyxa/src/adapters/types.ts

export interface Adapter {
  name: string
  target: string
  supports?: readonly string[]
  build(ctx: BuildContext): AdapterOutput | Promise<AdapterOutput>
}
```

The bundled runtime runner resolves the explicit adapter name (or a valid package name):

```javascript
ruvyxa build --adapter node
// runtime/adapter-runner.mjs resolves the selected adapter and invokes build()
```

---

### Built-in Adapters

| Package      | Platform                     | Output |
| ------------ | ---------------------------- | ------ |
| Name         | Package                      |
| ---          | ---                          |
| `node`       | `@ruvyxa/adapter-node`       |
| `bun`        | `@ruvyxa/adapter-bun`        |
| `static`     | `@ruvyxa/adapter-static`     |
| `vercel`     | `@ruvyxa/adapter-vercel`     |
| `netlify`    | `@ruvyxa/adapter-netlify`    |
| `cloudflare` | `@ruvyxa/adapter-cloudflare` |
| `railway`    | `@ruvyxa/adapter-railway`    |
| `render`     | `@ruvyxa/adapter-render`     |
| `firebase`   | `@ruvyxa/adapter-firebase`   |
| `aws`        | `@ruvyxa/adapter-aws`        |

---

### Adapter: Node

#### Output Structure

```
.ruvyxa/deploy/node/
  ├── server/index.mjs       # Standalone node:http server
  ├── public/                 # Optional prerendered site
  ├── start.mjs
  └── README.md
```

#### Runtime

The generated `server/index.mjs` is a standalone server:

```javascript
node.ruvyxa / deploy / node / server / index.mjs
```

---

### Adapter: Static

#### Output Structure

```
.ruvyxa/static/
  ├── static/                   # Default publish directory
  ├── assets/                   # Hashed client assets
  └── build.json
```

#### Build Process

1. Run `ruvyxa build` (full SSG detection)
2. For each SSG route: use the framework's `getStaticParams`/`staticParams` metadata
3. Write HTML to the adapter's static-site artifact
4. Copy client bundles and generated headers into the publish directory

---

### Adapter: Vercel

#### Output Structure

```
.ruvyxa/vercel/
  └── .vercel/output/
      ├── config.json          # Vercel output config
      └── static/
          ├── index.html
          └── ...
      └── functions/
          ├── __ruvyxa.func/
          │   ├── .vc-config.json
          │   └── server.js
          └── blog/[slug].func/
              ├── .vc-config.json
              └── server.js
```

#### `.vc-config.json`

```json
{
  "runtime": "nodejs18.x",
  "handler": "server.js",
  "launcherType": "Nodejs",
  "shouldAddHelpers": true
}
```

---

### Adapter: Cloudflare

#### Output Structure

```
.ruvyxa/cloudflare/
  ├── wrangler.toml
  ├── worker.js              # Compiled worker (ESM)
  └── assets/                 # Static assets
```

#### `wrangler.toml`

```toml
name = "ruvyxa-app"
main = "worker.js"
compatibility_date = "2024-01-01"

[site]
bucket = "assets"
```

`worker.js` is an ES module that exports `fetch`:

```javascript
export default {
  async fetch(request, env, ctx) {
    // Match route, render, return Response
  },
}
```

---

### Using the Node output in Docker

#### Output Structure

```
.ruvyxa/deploy/node/
  ├── server/index.mjs
  └── public/
```

#### Dockerfile

```dockerfile
FROM node:22-alpine
WORKDIR /app
COPY .ruvyxa/deploy/node/ .
EXPOSE 3000
CMD ["node", "server/index.mjs"]
```

#### nginx.conf (optional)

```nginx
server {
    listen 80;
    location / {
        proxy_pass http://127.0.0.1:3000;
    }
    location /_next/static {
        # Immutable static assets
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}
```

---

### Adapter Selection

```rust
ruvyxa doctor --adapter node
ruvyxa build --adapter node
```

The runner resolves the selected name from project configuration or the CLI. `start` and `preview`
serve an existing build and do not invoke an adapter.

---

### Why This Design

1. **Common contract with platform-specific artifacts** — Every adapter returns `AdapterOutput`; the
   artifact layout stays isolated per platform.
2. **Explicit selection** — `--adapter` or project configuration is the reliable build decision;
   platform detection is only a fallback.
3. **No deployment control plane** — Adapters build artifacts. They do not implement blue-green
   swaps, health-gated promotion, or production rollback.
4. **Docker is a hosting form** — Use the Node or Bun artifact in a container rather than treating
   Docker as a separate built-in adapter.
