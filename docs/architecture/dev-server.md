# Dev Server

**Crate**: `ruvyxa_dev_server` —
`crates/ruvyxa_dev_server/src/{lib,router,render_cache,hmr_tracker,worker_pool,style,action_security,port_binding,render_pipeline,plugin_bridge,plugin_head,html_document,env_file,static_assets,cli_output}.rs`

Axum HTTP server with HMR (WebSocket), radix-trie route matching, LRU render cache, persistent
Node/Bun worker pool, style collection pipeline, action security middleware, TypeScript plugin host,
and realtime event broadcasting.

---

## ServerConfig

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

## JavaScriptRuntime

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

## Framework Endpoints

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

## Key Modules

### RadixRouter (`router.rs`)

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

### RenderCache (`render_cache.rs`)

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

### HmrTracker (`hmr_tracker.rs`)

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

### NodeWorkerPool (`worker_pool.rs`)

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

### Worker environment variables

| Variable                         | Default              | Effect                                                     |
| -------------------------------- | -------------------- | ---------------------------------------------------------- |
| `RUVYXA_WORKER_POOL_SIZE`        | CPU count (2–8)      | Worker processes in the dev/prod pool                      |
| `RUVYXA_WORKER_MAX_CONCURRENCY`  | CPU count (2–8)      | Requests one worker executes at once                       |
| `RUVYXA_WORKER_TIMEOUT_MS`       | 30000 / 300000 build | Per-request deadline, shared by Rust and the Node watchdog |
| `RUVYXA_PRERENDER_RECYCLE_AFTER` | 32 (`0` disables)    | Isolated prerenders before a build worker is retired       |
| `RUVYXA_CACHE_MAX_ENTRIES`       | 256                  | Bundle and module cache entries per worker                 |
| `RUVYXA_MEMORY_LIMIT_MB`         | 512                  | Heap threshold that triggers in-worker cache eviction      |
| `RUVYXA_RENDER_CACHE_SIZE`       | 1024 dev / 512 prod  | Render cache entries (capped at 16384)                     |

### StyleCollection (`style.rs`)

```rust
pub struct StyleCollection { pub css: String, pub files: Vec<PathBuf> }
pub fn collect_styles(root, app_dir, entries) -> Result<StyleCollection>;
pub fn minify_css(css: &str) -> String;
```

Walks `app/` script imports, resolves CSS/SCSS/Sass dependencies, compiles Sass, scopes CSS Modules,
compiles Tailwind via `@tailwindcss/cli`. Minifies in production mode. Escapes `</style` in output.

### ActionSecurity (`action_security.rs`)

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

### PortBinding (`port_binding.rs`)

```rust
pub(crate) async fn bind_listener(config, address) -> Result<(TcpListener, SocketAddr)>;
```

Tries configured port, then scans +100 upward. On conflict, prints owner detection (netstat/lsof)
and binds first available.

---

## serve() Flow

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

## File Watcher & HMR

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

## Realtime Runtime

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

## Under the Hood

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
