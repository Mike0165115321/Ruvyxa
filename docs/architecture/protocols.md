# Protocols · โพรโทคอล

**Scope**: Cross-crate (dev server HMR, server actions, client module protocol)

## สรุป

สาม wire protocols: (1) HMR WebSocket สำหรับ hot reload, (2) Server Action HTTP สำหรับ form/action
submissions, (3) Client Module HTTP สำหรับ lazy-loading client-side bundles

---

## 1. HMR WebSocket Protocol

### Endpoint

```
ws://<host>:<port>/_ruvyxa/hmr
```

### Server → Client Messages

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

### Client → Server Messages

```json
{
  "type": "manifest-hash",
  "hash": "a1b2c3d4e5f6..."
}
```

Sent on `connected` acknowledgment. Server compares hash with current manifest; if different, sends
`state-sync` with the delta.

### Connection Lifecycle

```
Client opens WebSocket
  → Server sends { type: "connected" }
  → Client sends { type: "manifest-hash", hash }
  → Server sends delta or full manifest if hash mismatch
  → Loop: Server pushes update events
  → On disconnect: Client retries every 1s (exponential backoff, max 30s)
```

---

## 2. Server Action Protocol

### Request

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

### Response (Success)

```
Status: 200
Content-Type: application/json
```

```json
{
  "data": {/* action return value */}
}
```

### Response (Error)

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

### Action Discovery

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

### Security

- Actions are POST-only. GET returns 405.
- CSRF protection via `Ruvyxa-Action` header (must match action name).
- Origin check: `Origin` header must match allowed origins.
- Rate limiting: applies to action endpoints when `RateLimitConfig` is enabled.

---

## 3. Client Module Protocol

### Request

```
GET /_ruvyxa/client/{module_path}
```

`module_path` is URL-encoded relative path from project root. Example:
`GET /_ruvyxa/client/src%2Fapp%2Fpage.tsx`

### Response

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

### Module Registry (Browser-side)

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

### Lazy Loading

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

## 4. Render Proxy Protocol

### Internal Use Only

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

## Protocol Comparison

| Protocol      | Transport | Encoding          | Direction                  | Caching        |
| ------------- | --------- | ----------------- | -------------------------- | -------------- |
| HMR           | WebSocket | JSON              | Bidirectional              | None           |
| Server Action | HTTP POST | JSON              | Client → Server (result ←) | None           |
| Client Module | HTTP GET  | JavaScript (IIFE) | Server → Client            | Immutable (1y) |
| Render Proxy  | HTTP POST | HTML              | Internal                   | LRU            |

---

## Why This Design

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
