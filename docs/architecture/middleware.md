# Middleware

**Crate**: `ruvyxa_middleware` **Sources**:
`crates/ruvyxa_middleware/src/{config,stack,builtin,plugin_host}.rs`

## Purpose

Ruvyxa middleware has two layers: a compact set of built-in Tower layers toggled from
`ruvyxa.config.ts`, and a TypeScript plugin bridge that runs user middleware as child processes over
JSON-lines stdin/stdout. There is no plugin ordering DSL, no abstract compression algorithm enum, no
`RateLimitStore` trait — the built-in set is intentionally minimal.

## Configuration (`config.rs`)

### `MiddlewareConfig`

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

### `BuiltinMiddlewareConfig`

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

### `CorsConfig`

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

### `RateLimitConfig`

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

## Validation Rules

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

## MiddlewareStack (`stack.rs`)

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

## PluginHost (`plugin_host.rs`)

TypeScript plugin middleware runs as one or more persistent child processes (`node` or `bun`). The
runtime script loads the config registry and communicates over newline-delimited JSON on
stdin/stdout.

### Lifecycle

1. `PluginHost::start_pool_with_timeout(root, script, executable, pool_size, timeout)` spawns
   workers
2. Worker startup sends `{"hook":"describe"}` — the worker responds with `PluginRegistryDescriptor`
3. Registry diagnostics are logged. Workers beyond the first are only spawned if the registry
   declares HTTP hooks

### Descriptor

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

### Hook Protocol

Hooks are dispatched to workers round-robin. If the selected worker is busy, the pool scans for an
idle worker before queueing (avoids head-of-line blocking on long hooks).

- `execute_request(&PluginHttpRequest) -> PluginHttpRequestResult` — the hook either returns a
  modified `Request` or short-circuits with a `Response`
- `execute_response(&PluginHttpRequest, &PluginHttpResponse) -> PluginHttpResponse`
- `notify_file_change(&[String])` — used during development to signals file changes

### Failure Recovery

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

### Realtime Capability

If a plugin declares `{"id": "realtime@1"}`, the descriptor exposes a `RealtimeDescriptor` with the
capability/protocol identifier `realtime@1`; this is not a separately versioned plugin package.
WebSocket path, heartbeat interval, and capacity. The dev server uses this to wire up realtime
connections.

## Under the Hood

- The built-in CORS layer is **not** `tower_http::cors::CorsLayer` — it is a hand-written service to
  keep dependencies minimal and give precise control over preflight handling and the `Vary` header.
- The rate limiter is **in-process only**. There is no Redis backend, no `RateLimitStore` trait.
  High-cardinality token buckets (10k+) trigger a full sweep of expired entries.
- Compression is applied **to all routes unconditionally** but only activates on responses with a
  known content-length. Streaming SSE and chunked responses are never run through the async
  compression adapter.
- Plugin workers are **not restarted automatically** after a failed hook unless the process itself
  died or the protocol stream was poisoned. Application-level errors are returned to the caller
  without process replacement.
- The pool size fan-out to >1 workers only happens when the registry declares at least one HTTP
  hook. A build-only plugin sees a single worker regardless of the configured pool size.
