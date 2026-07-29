# Security · ความปลอดภัย

**Scope**: Cross-crate (middleware, CLI, dev server, graph validation)

## สรุป

Ruvyxa employs defense in depth: configurable CORS, rate limiting, CSRF protection for actions,
server/client boundary enforcement, private env var isolation, and origin validation.

---

## 1. CORS (Cross-Origin Resource Sharing)

### Configuration

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

### Runtime

`CorsLayer` wraps `tower_http::cors::CorsLayer`. Preflight `OPTIONS` is handled automatically.
Origin matching is strict (no wildcard `*` when credentials enabled).

### Default (dev)

```json
{
  "cors": {
    "allowOrigins": ["http://localhost:3000"],
    "allowCredentials": true
  }
}
```

### Default (build)

No CORS middleware unless explicitly configured (production API is expected to set `allowOrigins` to
the actual domain).

---

## 2. Rate Limiting

### Configuration

```rust
pub struct RateLimitConfig {
    pub requests: u64,              // max requests per window
    pub window_secs: u64,            // time window in seconds
    pub key_by: RateLimitKey,        // Ip | Header(name) | Session
}

pub enum RateLimitKey {
    Ip,                              // Client IP address
    Header(String),                  // Custom header (e.g. X-Api-Key)
    Session,                         // Session cookie (requires session middleware)
}
```

### Default Store

In-memory: `HashMap<String, (Instant, u64)>`. Cleanup runs every `window_secs` via lazy sweep. Max
100,000 tracked keys.

### Response on Limit

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

## 3. CSRF Protection for Server Actions

All POST requests to `/_ruvyxa/action/*` require:

1. **Origin validation**: `Origin` header must match an allowed origin from
   `CorsConfig.allow_origins`. No `Origin` header → blocked (browsers always send `Origin` on
   cross-origin POST).
2. **`Ruvyxa-Action` header**: Must equal the action name being called. E.g.,
   `Ruvyxa-Action: submitForm`. Absent or mismatched → 403.
3. **Method check**: Only POST allowed. GET → 405.

---

## 4. Server/Client Boundary

Enforced at bundle time by `ruxyva_bundler::boundary`:

```rust
pub fn check_boundary(module: &CompiledModule, graph: &[CompiledModule], base: &Path) -> Result<Vec<Diagnostic>>
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

## 5. Private Environment Variables

### Convention

```typescript
// Public (safe for browser):
const apiUrl = process.env.RUVYXA_PUBLIC_API_URL

// Private (server-only):
const dbPassword = process.env.DATABASE_PASSWORD
```

### Enforcement

During bundling, the compiler replaces `process.env.RUVYXA_PUBLIC_*` with the actual value in client
bundles.

Any `process.env.<NAME>` where `<NAME>` does not start with `RUVYXA_PUBLIC_` triggers RUV1008 if
found in a client-accessible module.

### Implementation

```rust
fn rewrite_env_vars(code: &str, env: &HashMap<String, String>, target: BundleTarget) -> String {
    // For client target:
    //   Match process.env.RUVYXA_PUBLIC_<NAME> → replace with env value
    //   Match process.env.<NAME> (non-public) → replace with "undefined"
    // For server target:
    //   Replace process.env.<NAME> → env.get(NAME) or "undefined"
}
```

---

## 6. Plugin Security

Plugins from `ruvyxa.plugin.ts` run inside the server process. Safety measures:

1. **PluginHost isolates hooks** — `before_request` / `after_response` receive sanitized
   `PluginHttpRequest` objects (body capped at 1MB, headers audited).
2. **Plugin code is compiled** — Modified plugin files trigger HMR full-reload, not silent
   injection.
3. **No filesystem access** — Plugin hooks do not receive `fs` or `process` references. They mutate
   HTTP request/response data only.

---

## 7. Default Headers

Every response includes:

```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Content-Security-Policy: default-src 'self'
Referrer-Policy: strict-origin-when-cross-origin
```

Configurable via `ruvyxa.config.ts` `security.headers` field.

---

## 8. Validation at Route Discovery

### Safe paths

```rust
fn validate_route_path(path: &str) -> Result<()> {
    // Reject: .., ~, \0, null bytes
    // Reject: absolute paths (/etc/passwd)
    // Reject: Windows drive letters
    // Reject: path traversal via encoded slashes
}
```

Route paths are validated during `discover_routes()` to prevent directory traversal.

---

## Why This Design

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
