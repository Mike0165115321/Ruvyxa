# CLI Architecture

**Crate**: `ruvyxa_cli`  
**Source**: `crates/ruvyxa_cli/src/`

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
| `config`                                           | `ruvyxa.config.*` loading and validation                                         |
| `runtime_config`                                   | args + config → `ServerConfig`, adapter and runtime selection                    |
| `cli_args`                                         | argument spelling normalization, plugin scaffolding                              |
| `commands`                                         | `routes`, `analyze`, `check`, `doctor`, `clean`, `trace`, `bench`, `test:parity` |
| `environment`                                      | toolchain and dependency probing for `doctor`                                    |
| `ui`                                               | progress bars, tables, colouring, byte/duration formatting                       |
| `image_optimizer`, `image_usage`, `site_discovery` | asset and discovery-file generation                                              |

The split is by responsibility, not by size. A command that needs more than dispatch belongs beside
the other logic of its kind rather than in `main.rs`.

## Entry Point

```
struct Cli {
    command: Command
}
```

No global flags (`root`, `verbose`, etc.) — each subcommand carries its own args. Clap v4 with
styled ANSI output.

## Command Enum (13 variants)

| Variant      | Args Struct   | Purpose                                                                                                |
| ------------ | ------------- | ------------------------------------------------------------------------------------------------------ |
| `Dev`        | `ServerArgs`  | Axum dev server with HMR, file watching, live reload                                                   |
| `Build`      | `BuildArgs`   | Production build: route discovery, validation, client bundling, SSG/ISR/PPR prerender, adapter, commit |
| `Check`      | `ProjectArgs` | `tsc --noEmit` + parity test; production readiness gate                                                |
| `Start`      | `ServerArgs`  | Axum production server from `.ruvyxa/` output                                                          |
| `Preview`    | `ServerArgs`  | Same as `Start` — alias for local preview of production build                                          |
| `Routes`     | `ProjectArgs` | Discover and print route table (kind, path, file, strategy)                                            |
| `Analyze`    | `AnalyzeArgs` | Validate routes, imports, server/client boundary; output as Human, JSON, or SARIF                      |
| `Doctor`     | `DoctorArgs`  | Full project diagnostics: versions, tools, adapter compatibility, dependency check                     |
| `Clean`      | `ProjectArgs` | Remove `.ruvyxa/` output directory                                                                     |
| `Trace`      | `TraceArgs`   | Inspect one route manifest entry by route path, print as JSON                                          |
| `Bench`      | `BenchArgs`   | Benchmark route discovery + analysis + production build over N samples                                 |
| `TestParity` | `ProjectArgs` | Build then compare dev vs production route manifests + smoke render (alias: `parity`)                  |
| `Plugin`     | `PluginArgs`  | Subcommand `PluginCommand::Create(PluginCreateArgs)` — scaffold plugin package                         |

## Args Structs

```
struct ProjectArgs          { root: PathBuf, runtime: Option<CliRuntime> }
struct ServerArgs           { root: PathBuf, host: Option<String>, port: Option<u16>, runtime: Option<CliRuntime> }
struct BuildArgs            { root: PathBuf, target: Option<BuildTarget>, adapter: Option<String>, runtime: Option<CliRuntime> }
struct AnalyzeArgs          { root: PathBuf, runtime: Option<CliRuntime>, format: AnalyzeFormat, output: Option<PathBuf> }
struct DoctorArgs           { root: PathBuf, target: Option<BuildTarget>, adapter: Option<String>, runtime: Option<CliRuntime>, json: bool }
struct TraceArgs            { route: String, root: PathBuf }
struct BenchArgs            { root: PathBuf, samples: usize, json: bool }
struct PluginArgs           { command: PluginCommand }
  enum PluginCommand        { Create(PluginCreateArgs) }
    struct PluginCreateArgs { name: String, root: PathBuf, dir: Option<PathBuf> }
```

## Key Enums

```
BuildTarget  → Node | Bun | Edge | Static
CliRuntime   → Node | Bun
AnalyzeFormat → Auto | Human | Json | Sarif
```

`BuildTarget` is also `serde::Deserialize` and stored as `config.runtime`. The CLI `--runtime` flag
uses `CliRuntime` (Node | Bun only) and maps to `JavaScriptRuntime` (from `ruvyxa_dev_server`).

## Config Loading

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

## Config Override Priority

`RUVYXA_RUNTIME` env → `--runtime` CLI flag → `config.runtime` → default detection. The `--adapter`
CLI flag parses through `parse_adapter_name()` which accepts 10 known names (node, bun, static,
vercel, netlify, cloudflare, railway, render, firebase, aws) or any npm package name. Platform
auto-detection reads 6 env vars (VERCEL, NETLIFY, CF_PAGES, RAILWAY_PROJECT_ID, RENDER, AWS_APP_ID).

## Build Pipeline

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
14. **Adapter runner** (if adapter configured or detected) — `run_adapter_runner()` spawns
    `adapter-runner.mjs` which produces artifact reports

## Module Resolution & Bundler

`emit_client_bundles_with_session()` uses `ruvyxa_bundler::BundleContext`:

- Creates `CompileCache` and `ResolveGraphCache` at `cache/bundler/`
- When plugins present → attaches `BuildHookPipeline` with `TypeScriptPluginBridge` hooks
- Plugin hooks: `resolve_id`, `load`, `transform` — each communicates with persistent Node worker
  via NDJSON over stdin/stdout
- Supports `SplitStrategy::Route` (default) and `SplitStrategy::Single`
- Client bundling respects minify, sourcemap, tree-shaking, JSX runtime (classic/automatic), ES
  target (es2018–esnext)
- Progress bar on TTY; silent in pipes/CI

## Plugin Host

`TypeScriptPluginBuildSession` manages a persistent Node process running `plugin-runtime.mjs`:

- `run_start(out_dir)` → calls `build.start` hook
- `run_complete(out_dir, manifest)` → calls `build.complete` hook
- Hooks are used during bundling via `TypeScriptPluginBridge` which implements `BuildHooks` trait
- Worker protocol: JSON line → stdin, JSON line ← stdout, errors → stderr
- Round-robin across workers for concurrent hook calls

## Plugin Create Scaffolding

`plugin create <name>` copies 6 template files from `templates/plugin/`:

- `src/index.ts`, `test/plugin.test.mjs`, `package.json`, `tsconfig.json`, `README.md`, `.gitignore`
- Replaces `__PLUGIN_NAME__`, `__PLUGIN_IDENTIFIER__`, `__RUVYXA_VERSION__`
- Validates plugin name: lowercase + digits + single hyphens only
- Default dir is `<name>` under root; `--dir` overrides (must be relative, no `..`)
- Package is named `ruvyxa-plugin-<name>`

## CLI Normalization

Before clap parsing, `normalized_cli_args()` normalizes option and command casing (case-insensitive
matching to canonical forms). This makes `ruvyxa BUILD --Target node` equivalent to
`ruvyxa build --target node`.

## Error Handling

- `anyhow::Result` with `.context()` for all failures
- Error codes: `RUV1205` (prerender path escape), `RUV1600`–`RUV1602` (config validation),
  `RUV1700`–`RUV1701` (plugin errors), `RUV2200`–`RUV2203` (adapter errors)
- Diagnostics bubble through `fail_on_diagnostics()` which prints each diagnostic and bails with
  count
- Invalid config fields rejected at deserialization via `deny_unknown_fields`
- Security limits validated against hard ceilings from `ruvyxa_dev_server` constants
- Build staging directory has drop-guard cleanup; commit failures trigger rollback
