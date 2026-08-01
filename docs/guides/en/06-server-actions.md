# Server Actions

Server actions let you mutate data without writing a custom API endpoint. Think of them as
**functions that run on the server but are called from the client**. Actions are POST-only,
automatically validated, rate-limited, and CSRF-protected.

```
Client (browser)              Server
     │                          │
     │  POST /__ruvyxa/action   │
     │  ─────────────────────→  │
     │                          │  ┌──────────────┐
     │                          │  │ Validate      │
     │                          │  │  - body size  │
     │                          │  │  - content    │
     │                          │  │    type       │
     │                          │  │  - same-origin│
     │                          │  │  - fetch-meta │
     │                          │  │  - rate limit │
     │                          │  ├──────────────┤
     │                          │  │ Parse input   │
     │                          │  │ Run handler   │
     │                          │  │ Invalidate    │
     │                          │  │ cache         │
     │                          │  └──────────────┘
     │                          │
     │  { ok: true }           │
     │  ←───────────────────── │
     │                          │
```

---

## Type Definitions

```ts
// file: packages/@ruvyxa/core/src/server.ts

export interface Schema<TInput> {
  parse(value: unknown): TInput
}

export interface ActionContext<TInput> {
  input: TInput
  request: Request
  user?: unknown
  invalidate(key: string): void
}

export interface ActionBuilder<TInput = unknown> {
  input<TNextInput>(schema: Schema<TNextInput>): ActionBuilder<TNextInput>
  realtime(channels?: string | readonly string[]): ActionBuilder<TInput>
  handler<TResult>(
    handler: (ctx: ActionContext<TInput>) => TResult | Promise<TResult>,
  ): ServerAction<TInput, TResult>
}

export interface ServerAction<TInput, TResult> {
  (input: TInput, ctx?: Partial<ActionContext<TInput>>): Promise<TResult>
  ruvyxa: {
    kind: 'action'
    realtime?: ActionRealtimeOptions
  }
}

export interface ActionRealtimeOptions {
  channels: readonly string[]
}

export const action: ActionBuilder = createActionBuilder()
```

### Config Types

```ts
// file: packages/@ruvyxa/core/src/types.ts

export interface RuvyxaConfig {
  security?: {
    actionLimit?: number // @default 1048576 (1 MiB)
    apiLimit?: number // @default 10485760 (10 MiB)
    pluginLimit?: number // @default 33554432 (32 MiB) @maximum 268435456 (256 MiB)
    actionRateLimit?: {
      max?: number // @default 600
      window?: number // @default 60 (seconds)
    }
    sameOrigin?: boolean // @default true
    fetchMeta?: boolean // @default true
    trustedProxyIps?: string[] // @default []
    headers?: boolean // @default true
  }
}
```

### Server-Side Constants

```rust
// file: crates/ruvyxa_dev_server/src/lib.rs

const MAX_ACTION_BODY_BYTES: usize = 1024 * 1024;           // 1 MiB — default limit
pub const MAX_ACTION_BODY_LIMIT_BYTES: usize = 16 * 1024 * 1024;  // 16 MiB — hard max
const ACTION_RATE_LIMIT_MAX: usize = 600;
const ACTION_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
pub const MAX_ACTION_RATE_LIMIT_REQUESTS: usize = 10_000;
pub const MAX_ACTION_RATE_LIMIT_WINDOW_SECS: u64 = 86_400;  // 24 hours
```

---

## Creating an Action

Actions live in `action.ts` files. Each file can export multiple actions.

```ts
// app/todos/action.ts
import { action } from 'ruvyxa/server'

interface CreateTodoInput {
  text: string
}

export const createTodo = action
  .input({
    parse(value: unknown): CreateTodoInput {
      if (typeof value !== 'object' || value === null) throw new Error('Expected object')
      const obj = value as Record<string, unknown>
      if (typeof obj.text !== 'string' || obj.text.length === 0) throw new Error('text is required')
      return { text: obj.text }
    },
  })
  .handler(async ({ input, invalidate }) => {
    // input is fully typed as CreateTodoInput
    const id = await db.query('INSERT INTO todos (text) VALUES (?)', [input.text])

    // Invalidate related cache
    invalidate('todos:all')

    return { ok: true, id }
  })
```

### Input Validation — `parse` Contract

The `parse` function must:

1. Accept `unknown` — the first parameter type
2. Return `TInput` — the typed output
3. Throw on invalid input — error message becomes the 400 response body

```ts
.input({
  parse(value: unknown): MyType {
    if (typeof value !== "object" || value === null) {
      throw new Error("Body must be a JSON object");
    }
    // Safe cast after check
    const data = value as Record<string, unknown>;

    if (typeof data.email !== "string") {
      throw new Error("email must be a string");
    }

    return {
      email: data.email,
      score: typeof data.score === "number" ? data.score : 0,
    };
  },
})
```

If `parse` throws, the action returns a **400 Bad Request** response with the error message as the
body.

### Handler Context — Full Type

```ts
.handler(async ({ input, invalidate, request, user }) => {
  // input: TInput — validated by parse()
  // invalidate(key: string): void — invalidate cache keys
  // request: Request — the original HTTP request (for reading headers, etc.)
  // user: unknown — authenticated user if auth middleware is set up

  return { ok: true };
});
```

---

## Calling Actions

### From HTML Forms (No JavaScript)

The simplest way: set `method="post"` and
`action="/__ruvyxa/action?path=<route>&name=<actionName>"`.

```tsx
// app/todos/page.tsx
import { createTodo } from './action'

export default function TodosPage() {
  return (
    <form method="post" action="/__ruvyxa/action?path=/todos&name=createTodo">
      <input name="text" required placeholder="What needs doing?" />
      <button type="submit">Add</button>
    </form>
  )
}
```

The form fields are serialized as URL-encoded form data. The action handler receives them as a JSON
object with matching keys.

### With JavaScript (fetch)

```tsx
'use client'

import { useState } from 'react'

export function AddTodoForm() {
  const [text, setText] = useState('')

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()

    const res = await fetch('/__ruvyxa/action?path=/todos&name=createTodo', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ text }),
    })

    const result = await res.json()
    if (res.ok) {
      setText('')
      // Optionally refresh the page or update state
      window.location.reload()
    } else {
      alert(result.error || 'Something went wrong')
    }
  }

  return (
    <form onSubmit={handleSubmit}>
      <input
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder="What needs doing?"
      />
      <button type="submit">Add</button>
    </form>
  )
}
```

### URL Construction

The action URL follows this pattern:

```
/__ruvyxa/action?path={routePath}&name={exportName}
```

| Query param | Value                                      | Example      |
| ----------- | ------------------------------------------ | ------------ |
| `path`      | The route path where the action file lives | `/todos`     |
| `name`      | The exported action name                   | `createTodo` |

Full URL: `/__ruvyxa/action?path=/todos&name=createTodo`

---

## Content-Type Handling

Actions accept two content types. Exact algorithm:

```
fn action_content_type(headers) → Option<ContentType>:
    content_type = headers.CONTENT_TYPE
        .split(';')[0]          // strip parameters (charset, boundary)
        .trim()
    if content_type == "application/json":
        → Some("application/json")
    if content_type == "application/x-www-form-urlencoded":
        → Some("application/x-www-form-urlencoded")
    → None (rejected as UNSUPPORTED_MEDIA_TYPE)
```

| Content-Type                        | How data is parsed                             | Example body       |
| ----------------------------------- | ---------------------------------------------- | ------------------ |
| `application/json`                  | `JSON.parse(body)` — validated as valid JSON   | `{"text":"hello"}` |
| `application/x-www-form-urlencoded` | Parsed as form fields, combined into an object | `text=hello`       |
| Any other                           | **415 UNSUPPORTED MEDIA_TYPE**                 | —                  |

### Edge Cases

| Input                       | Behavior                                                         |
| --------------------------- | ---------------------------------------------------------------- |
| Empty JSON body `""`        | Treated as `"{}"` for `application/json`                         |
| Empty URL-encoded body `""` | `parse({})` receives an empty object                             |
| Non-UTF-8 body              | **400 BAD_REQUEST**: "Action payload must be valid UTF-8"        |
| Malformed JSON              | **400 BAD_REQUEST**: "Action JSON payload is malformed: {error}" |
| Missing `Content-Type`      | **415 UNSUPPORTED_MEDIA_TYPE**                                   |

---

## Security Defaults

Ruvyxa applies security measures automatically on every action request:

| Protection               | Default             | Notes                                                               |
| ------------------------ | ------------------- | ------------------------------------------------------------------- |
| Body size limit          | 1 MiB               | Actions reject payloads larger than 1 MiB (hard max: 16 MiB)        |
| Content-Type enforcement | JSON or URL-encoded | 415 for unsupported types                                           |
| Same-origin              | Enforced            | CSRF protection via **Origin** header vs **Host** header comparison |
| Fetch metadata           | Enforced            | `Sec-Fetch-Site` must equal `same-origin`                           |
| Rate limiting            | 600 req / 60s       | Sliding-window counter, per-IP and per-action                       |

### Same-Origin Check — Exact Algorithm

```rust
fn action_origin_is_cross_site(headers, config, peer) → bool:
    if headers.ORIGIN is absent:
        // Fail-closed: only allow if Sec-Fetch-Site: same-origin present
        return !(headers.Sec-Fetch-Site == "same-origin")

    (origin_scheme, origin_host) = split_once(ORIGIN, "://")
    host = headers.HOST

    // The host comparison is the check that stops CSRF: a browser sets Origin
    // itself, so a cross-site page cannot forge a matching host.
    if origin_host != host:
        return true

    // The scheme is only compared when something trustworthy stated it.
    // Ruvyxa never terminates TLS, so the sole evidence of the browser's
    // scheme is X-Forwarded-Proto from a trusted peer (loopback, or an entry
    // in security.trustedProxyIps). With no such evidence the scheme is
    // unknown and is not asserted.
    if peer is trusted AND headers.X-Forwarded-Proto in ("http", "https"):
        return origin_scheme != headers.X-Forwarded-Proto

    return false
```

> Earlier releases assumed `http` whenever no trusted proxy reported a scheme. That rejected every
> deployment whose TLS-terminating proxy is neither loopback nor listed in
> `security.trustedProxyIps` — the usual Docker Compose, Kubernetes, and managed-platform-edge
> shapes — with `403 Cross-origin action request blocked` on every action. Configuring
> `trustedProxyIps` is still recommended: it is what enables forwarded client-IP detection for the
> rate limiter and restores the strict scheme comparison.

### Fetch Metadata Check

```rust
fn action_fetch_site_is_cross_site(headers) → bool:
    return headers.Sec-Fetch-Site == "cross-site"
```

### Rate Limiter — Sliding Window Counter Detail

Each key hashes into one of a fixed number of counter slots, so the limiter's memory does not depend
on how many distinct clients it has seen.

```rust
const ACTION_RATE_LIMIT_SLOTS: usize = 8192;

struct RateSlot {
    window_start: Instant,
    current: u32,   // requests in the window that started at window_start
    previous: u32,  // requests in the window before it
}

struct ActionRateLimiter {
    slots: Vec<Option<RateSlot>>,  // ACTION_RATE_LIMIT_SLOTS entries
    hasher: RandomState,           // seeded per process
    max_hits: usize,               // default: 600
    window: Duration,              // default: 60s
}

fn allow(&mut self, key) → bool:
    slot = slots[hash(key) % ACTION_RATE_LIMIT_SLOTS]
    roll slot forward so window_start is within one window of now
    // Two adjacent windows approximate a sliding one: weight the previous
    // window by how much of it still falls inside the trailing window.
    overlap   = 1 - (now - slot.window_start) / window
    estimated = slot.previous * overlap + slot.current
    if estimated >= max_hits:
        → false  // rate limited
    slot.current += 1
    → true       // allowed
```

Two properties matter here:

- **A client is never denied on another client's behalf.** Admission is never refused for lack of
  room. Earlier releases tracked a key map capped at 10,000 entries and rejected any key they could
  not admit, so an attacker rotating source addresses — trivial with an IPv6 `/64` — could fill the
  map and lock out every first-time client for the rest of the window.
- **A slot collision can only limit a client early, never grant extra budget.** Two clients sharing
  a slot share one budget. The slot array is seeded per process, so keys cannot be crafted to
  collide with a chosen victim.

**Rate limit key format**: `{client_ip}:{action_path}:{action_name}` where `client_ip` is either the
direct peer IP or the rightmost untrusted IP from `X-Forwarded-For` (if a trusted proxy is
configured).

### Retry-After Header

When rate-limited, response includes `Retry-After` header in seconds:

```
HTTP/1.1 429 Too Many Requests
Retry-After: 45
```

### Security Configuration

Override defaults in `ruvyxa.config.ts`:

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'

export default config({
  security: {
    // Increase body limit for file upload actions
    actionLimit: 5 * 1024 * 1024, // 5 MiB (max: 16 MiB)
    // Trust proxies for correct IP detection
    trustedProxyIps: ['10.0.0.0/8', '172.16.0.0/12'],
    // Disable same-origin check (not recommended)
    sameOrigin: false,
    // Disable fetch-metadata check (not recommended)
    fetchMeta: false,
    // Custom rate limits
    actionRateLimit: {
      max: 100,
      window: 30,
    },
    // Disable security headers
    headers: false,
  },
})
```

| Config                   | Type               | Default             | Description                                | Max Value             |
| ------------------------ | ------------------ | ------------------- | ------------------------------------------ | --------------------- |
| `actionLimit`            | `number` (bytes)   | `1048576` (1 MiB)   | Max action body size                       | 16,777,216 (16 MiB)   |
| `apiLimit`               | `number` (bytes)   | `10485760` (10 MiB) | Max API route body size                    | 268,435,456 (256 MiB) |
| `pluginLimit`            | `number` (bytes)   | `33554432` (32 MiB) | Max middleware plugin response             | 268,435,456 (256 MiB) |
| `sameOrigin`             | `boolean`          | `true`              | Enforce Origin == Host                     | —                     |
| `fetchMeta`              | `boolean`          | `true`              | Reject cross-site Sec-Fetch-Site           | —                     |
| `actionRateLimit.max`    | `number`           | `600`               | Max requests in window                     | 10,000                |
| `actionRateLimit.window` | `number` (seconds) | `60`                | Rolling window                             | 86,400 (24h)          |
| `trustedProxyIps`        | `string[]`         | `[]`                | CIDR ranges trusted for forwarding headers | —                     |
| `headers`                | `boolean`          | `true`              | Apply default security response headers    | —                     |

### Forwarded Client IP Detection

```rust
fn forwarded_client_ip(config, headers) → Option<IpAddr>:
    // Scan X-Forwarded-For from RIGHT (most proxy-added)
    // Skip addresses that match config.trustedProxyIps
    // Return the first non-proxy address
    // Also checks X-Real-IP as fallback
```

**Trusted proxy logic**:

- Loopback (`127.0.0.1`, `::1`) is always trusted
- Additional addresses or CIDR ranges via `trustedProxyIps` config
- Without trusted proxies: direct peer IP is used for rate limiting

---

## Realtime Actions

Actions can publish realtime events after successful execution:

```ts
export const createTodo = action
  .input({
    parse(value: unknown) {
      /* ... */
    },
  })
  .realtime() // uses route channel
  .handler(async ({ input, invalidate }) => {
    await db.insert('todos', input)
    invalidate('todos:all')
    return { ok: true }
  })

// Or specify custom channels
export const notify = action
  .input({
    parse(value: unknown) {
      /* ... */
    },
  })
  .realtime(['chat:general', 'admin:alerts'])
  .handler(async ({ input }) => {
    return { ok: true }
  })
```

### `.realtime()` API

| Aspect         | Detail                                    |
| -------------- | ----------------------------------------- |
| Signature      | `.realtime(channels?: string              | string[])` |
| Default        | Omit → route channel (`route:/todos`)     |
| Max channels   | 16                                        |
| Channel format | `^[A-Za-z0-9:._/-]{1,128}$`               |
| Duplicates     | Deduplicated automatically                |
| Trimming       | Whitespace trimmed from each channel name |
| Frozen         | Channel array is `Object.freeze()`d       |

### Realtime Message Format

When action completes successfully, a message is broadcast:

```json
{
  "type": "action",
  "action": "createTodo",
  "invalidated": ["todos:all"]
}
```

### Error Handling

If `.realtime()` is called with invalid channels:

```
TypeError: action.realtime() accepts at most 16 channels
TypeError: action.realtime() channels[0] must use 1-128 letters, digits, colon, dot, underscore, slash, or dash
```

---

## Action Response Format

### Success

```json
{
  "data": { ... },
  "invalidated": ["todos:all"]
}
```

The `data` field contains whatever the handler returns. `invalidated` lists cache keys that were
invalidated during the action.

### Error (Validation)

```json
{
  "error": "text is required",
  "invalidated": []
}
```

Returned with appropriate HTTP status (400 for validation, 429 for rate limit, etc.).

### Error (Server Crash)

If the handler throws an uncaught error, the response is a 500 Internal Server Error. In
development, the error overlay shows the stack trace. In production, a generic error message is
returned.

---

## Cache Invalidation from Actions

The `invalidate` function in the handler argument lets you clear specific cache keys after mutation.

```ts
.handler(async ({ input, invalidate }) => {
  await db.query("UPDATE posts SET title = ? WHERE id = ?", [input.title, input.id]);

  // Invalidate caches that include this post
  invalidate("blog:post:" + input.slug);   // Single post cache
  invalidate("blog:recent");               // Recent posts list

  return { ok: true };
});
```

### Invalidation Rules

| Call                     | Behavior                                                       |
| ------------------------ | -------------------------------------------------------------- |
| `invalidate("key")`      | Removes exact key `key` AND any keys starting with `key + ":"` |
| `invalidate()` (no args) | **Not available** on ActionContext — must provide a key        |

Any loader using those keys will refetch on the next request.

---

## Full Action Example

```ts
// app/todos/action.ts
import { action } from 'ruvyxa/server'

// --- Create ---

interface CreateInput {
  text: string
}

export const createTodo = action
  .input({
    parse(value: unknown): CreateInput {
      if (typeof value !== 'object' || value === null) throw new Error('Invalid body')
      const data = value as Record<string, unknown>
      if (typeof data.text !== 'string' || data.text.trim().length === 0) {
        throw new Error('text is required')
      }
      return { text: data.text.trim() }
    },
  })
  .handler(async ({ input, invalidate }) => {
    const id = await insertTodo(input.text)
    invalidate('todos:all')
    return { ok: true, id }
  })

// --- Toggle done ---

interface ToggleInput {
  id: number
}

export const toggleTodo = action
  .input({
    parse(value: unknown): ToggleInput {
      if (typeof value !== 'object' || value === null) throw new Error('Invalid body')
      const data = value as Record<string, unknown>
      if (typeof data.id !== 'number') throw new Error('id must be a number')
      return { id: data.id }
    },
  })
  .handler(async ({ input, invalidate }) => {
    await toggleTodoDone(input.id)
    invalidate('todos:all')
    return { ok: true }
  })

// --- Delete ---

export const deleteTodo = action
  .input({
    parse(value: unknown): ToggleInput {
      if (typeof value !== 'object' || value === null) throw new Error('Invalid body')
      const data = value as Record<string, unknown>
      if (typeof data.id !== 'number') throw new Error('id must be a number')
      return { id: data.id }
    },
  })
  .handler(async ({ input, invalidate }) => {
    await removeTodo(input.id)
    invalidate('todos:all')
    return { ok: true }
  })
```

### Plugin Response Limits

TypeScript plugin middleware can buffer action responses. The limit applies to the entire serialized
response:

| Default | Maximum |
| ------- | ------- |
| 32 MiB  | 256 MiB |

Configure in `ruvyxa.config.ts`:

```ts
export default config({
  security: {
    pluginLimit: 64 * 1024 * 1024, // 64 MiB
  },
})
```

---

## Under the Hood: ServerConfig for Actions

Key fields in `ServerConfig` that control action behavior:

| Field                      | Type           | Default           | Maximum Limit |
| -------------------------- | -------------- | ----------------- | ------------- |
| `action_body_limit_bytes`  | usize          | 1,048,576 (1 MiB) | 100 MiB       |
| `action_rate_limit_max`    | usize          | 600               | 100,000       |
| `action_rate_limit_window` | Duration       | 60s               | 3,600s        |
| `same_origin_actions`      | bool           | true              | —             |
| `fetch_metadata_actions`   | bool           | true              | —             |
| `trusted_proxies`          | TrustedProxies | empty             | —             |

---

## Error Codes

| Code | Condition                    | HTTP Status                |
| ---- | ---------------------------- | -------------------------- |
| N/A  | Body exceeds `actionLimit`   | 413 Payload Too Large      |
| N/A  | Unsupported Content-Type     | 415 Unsupported Media Type |
| N/A  | Cross-origin request blocked | 403 Forbidden              |
| N/A  | Cross-site request blocked   | 403 Forbidden              |
| N/A  | Rate limit exceeded          | 429 Too Many Requests      |
| N/A  | `parse()` throws             | 400 Bad Request            |
| N/A  | Malformed JSON body          | 400 Bad Request            |
| N/A  | Non-UTF-8 body               | 400 Bad Request            |
| N/A  | Server error in handler      | 500 Internal Server Error  |
| N/A  | Mutex poisoned               | 503 Service Unavailable    |

---

## Edge Cases

| Scenario                                  | Behavior                                                                   |
| ----------------------------------------- | -------------------------------------------------------------------------- |
| **Action file has no matching exports**   | Ruvyxa returns 400 — the name query param references a non-existent export |
| **Two actions with same name**            | Build error — duplicate exports                                            |
| **JSON body is `null`**                   | `parse(null)` receives `null` — check `value !== null`                     |
| **Form data with duplicate keys**         | Last value wins (standard URL-encoded parsing)                             |
| **Empty form**                            | `parse({})` receives an object with no keys                                |
| **Invalid UTF-8 in URL-encoded body**     | 400 Bad Request                                                            |
| **Rate limit for unrelated actions**      | Each `(path, name)` pair has its own rate limit counter                    |
| **Action called via GET**                 | 405 Method Not Allowed (actions are POST-only)                             |
| **Missing `path` or `name` query params** | 400 Bad Request                                                            |

---

## Performance Characteristics

| Operation                  | Overhead                                         |
| -------------------------- | ------------------------------------------------ |
| Security checks            | Depends on request/configuration                 |
| Body parsing (JSON)        | Payload-size-dependent                           |
| Body parsing (URL-encoded) | Payload-size-dependent                           |
| Rate limiter check         | O(1) average                                     |
| Worker dispatch            | Depends on the selected runtime and worker state |
| Handler execution          | User-defined                                     |

---

## Troubleshooting

| Symptom                                   | Cause                                              | Fix                                              |
| ----------------------------------------- | -------------------------------------------------- | ------------------------------------------------ |
| Action returns 413                        | Body exceeds `actionLimit`                         | Increase `actionLimit` or reduce payload         |
| Action returns 403                        | Cross-origin request                               | Ensure action URL matches page origin            |
| Action returns 429                        | Rate limit hit                                     | Wait for Retry-After or increase limit           |
| `parse()` error returned as 400           | Validation failed                                  | Check input format, field names, types           |
| Action never reaches handler              | Content-Type wrong                                 | Set `Content-Type: application/json` or use form |
| 415 Unsupported Media Type                | Missing or wrong Content-Type                      | Set supported Content-Type header                |
| 503 Service Unavailable                   | Rate limiter mutex poisoned (rare)                 | Restart server                                   |
| Action works with HTML form but not fetch | Missing JS handler or wrong URL                    | Check `path` and `name` query params             |
| Realtime messages not delivered           | Channel format invalid, or WebSocket not connected | Verify channel names and WebSocket connection    |

---

## Best Practices

1. **One action file per route.** Co-locate actions with the pages that use them.

2. **Validate exhaustively in `parse`.** Never trust client input. Check types, bounds, existence.

3. **Invalidate all affected caches.** If an action updates multiple resources, invalidate all
   relevant keys.

4. **Keep handlers focused.** One action = one mutation. Don't create a monolithic "do everything"
   action.

5. **Use HTML forms as default.** They work without JS. Layer JS enhancements on top for better UX.

6. **Return structured responses.** `{ ok: true }` or `{ error: "message" }` makes client handling
   predictable.

7. **Use `trustedProxyIps` in production.** If behind a reverse proxy (nginx, Cloudflare), configure
   trusted IPs for correct rate limiting.

8. **Test rate limits during load testing.** Default 600 req/min is generous but not unlimited.

9. **Use `.realtime()` for multi-client sync.** When multiple users need real-time updates,
   broadcast via action channels.

10. **Keep `parse()` pure.** Side effects in `parse()` are unsafe — it runs before any handler
    checks.

---

## File Uploads via Actions

For file uploads, use `multipart/form-data` with `request.formData()`:

```tsx
'use client'

export function UploadForm() {
  async function handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault()
    const form = new FormData(e.currentTarget)
    const res = await fetch('/__ruvyxa/action?path=/upload&name=uploadFile', {
      method: 'POST',
      body: form,
      // Note: no Content-Type header — browser sets multipart boundary
    })
  }

  return (
    <form onSubmit={handleSubmit}>
      <input type="file" name="file" />
      <button type="submit">Upload</button>
    </form>
  )
}
```

```ts
// app/upload/action.ts
import { action } from 'ruvyxa/server'

export const uploadFile = action
  .input({
    parse(value: unknown) {
      if (typeof value !== 'object' || value === null) throw new Error('Invalid')
      const data = value as Record<string, unknown>
      if (!(data.file instanceof File)) throw new Error('file must be a File')
      return { file: data.file as File }
    },
  })
  .handler(async ({ input, invalidate }) => {
    const buffer = await input.file.arrayBuffer()
    await fs.writeFile(`./uploads/${input.file.name}`, Buffer.from(buffer))
    invalidate('uploads:list')
    return { ok: true, name: input.file.name }
  })
```

**Note**: Increase `actionLimit` for file uploads:

```ts
export default config({
  security: {
    actionLimit: 10 * 1024 * 1024, // 10 MiB
  },
})
```

## Action Composition

Call one action from another:

```ts
export const createUser = action
  .input({
    parse(value: unknown) {
      if (typeof value !== 'object' || value === null) throw new Error('Invalid')
      const data = value as Record<string, unknown>
      return { email: data.email as string, name: data.name as string }
    },
  })
  .handler(async ({ input, invalidate }) => {
    const userId = await db.query('INSERT INTO users (email, name) VALUES (?, ?)', [
      input.email,
      input.name,
    ])

    // Call another action's handler directly
    await sendWelcomeEmail.handler({
      input: { userId, email: input.email },
      invalidate,
      request: new Request('http://localhost/'),
    })

    invalidate('users:list')
    return { ok: true, userId }
  })

export const sendWelcomeEmail = action
  .input({
    parse(value: unknown) {
      if (typeof value !== 'object' || value === null) throw new Error('Invalid')
      const data = value as Record<string, unknown>
      return { userId: data.userId as number, email: data.email as string }
    },
  })
  .handler(async ({ input }) => {
    await emailService.sendWelcome(input.email, input.userId)
    return { ok: true }
  })
```

## Auth-Guarded Actions

Use the `user` context field from middleware:

```ts
export const deleteAccount = action
  .input({
    parse(value: unknown) {
      if (typeof value !== 'object' || value === null) throw new Error('Invalid')
      return value as { confirm: boolean }
    },
  })
  .handler(async ({ input, user, invalidate }) => {
    if (!user) {
      throw new Error('Authentication required')
    }
    if (!input.confirm) {
      throw new Error('Must confirm deletion')
    }

    await db.query('DELETE FROM users WHERE id = ?', [(user as { id: number }).id])
    invalidate('user:*')
    invalidate('dashboard:*')
    return { ok: true }
  })
```

## Optimistic Updates Pattern

Combine actions with client-side state for optimistic UI:

```tsx
'use client'

import { useState, useTransition } from 'react'

export function LikeButton({ postId, initialLikes }: { postId: number; initialLikes: number }) {
  const [likes, setLikes] = useState(initialLikes)
  const [isPending, startTransition] = useTransition()

  function handleLike() {
    // Optimistic update
    setLikes((l) => l + 1)

    startTransition(async () => {
      const res = await fetch('/__ruvyxa/action?path=/posts&name=likePost', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ postId }),
      })

      if (!res.ok) {
        // Revert on error
        setLikes((l) => l - 1)
      }
    })
  }

  return (
    <button onClick={handleLike} disabled={isPending}>
      {likes} {isPending ? '...' : '♥'}
    </button>
  )
}
```

## Testing Actions

Test actions in isolation:

```ts
// tests/actions.test.ts
import { createTodo } from '../app/todos/action'

describe('createTodo', () => {
  it('creates a todo with valid input', async () => {
    const result = await createTodo.handler({
      input: { text: 'Test todo' },
      invalidate: (key: string) => {
        /* mock */
      },
      request: new Request('http://localhost/', {
        method: 'POST',
        body: JSON.stringify({ text: 'Test todo' }),
      }),
    })

    expect(result).toEqual({ ok: true, id: expect.any(Number) })
  })

  it('rejects empty text', async () => {
    await expect(
      createTodo.handler({
        input: { text: '' },
        invalidate: () => {},
        request: new Request('http://localhost/'),
      }),
    ).rejects.toThrow('text is required')
  })
})
```

## Action Error Handling Patterns

### Structured Error Responses

```ts
export const updateProfile = action
  .input({
    parse(value: unknown) {
      if (typeof value !== 'object' || value === null) throw new Error('Invalid')
      const data = value as Record<string, unknown>
      const errors: string[] = []
      if (typeof data.name !== 'string') errors.push('name is required')
      if (data.age !== undefined && (typeof data.age !== 'number' || data.age < 0)) {
        errors.push('age must be a positive number')
      }
      if (errors.length > 0) throw new Error(errors.join('; '))
      return { name: data.name, age: data.age as number | undefined }
    },
  })
  .handler(async ({ input, invalidate }) => {
    try {
      await db.query('UPDATE users SET name = ? WHERE id = ?', [input.name, userId])
      invalidate('user:profile:' + userId)
      return { ok: true }
    } catch (dbError) {
      console.error('Database error:', dbError)
      return { ok: false, error: 'Failed to update profile' }
    }
  })
```

### Retry Logic

```ts
export const processPayment = action
  .input({
    parse(value: unknown) {
      if (typeof value !== 'object' || value === null) throw new Error('Invalid')
      return value as { orderId: number; amount: number }
    },
  })
  .handler(async ({ input, invalidate }) => {
    let lastError: Error | null = null
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        const result = await paymentGateway.charge(input.amount)
        invalidate('order:' + input.orderId)
        invalidate('dashboard:stats')
        return { ok: true, transactionId: result.id }
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error))
        if (attempt < 2) await new Promise((r) => setTimeout(r, 1000 * Math.pow(2, attempt)))
      }
    }
    return { ok: false, error: lastError.message }
  })
```

## Actions vs API Routes Comparison

| Aspect             | Actions                                    | API Routes                                   |
| ------------------ | ------------------------------------------ | -------------------------------------------- |
| Primary use        | Mutations                                  | Any HTTP endpoint                            |
| HTTP methods       | POST only                                  | GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS |
| CSRF protection    | Automatic (same-origin + fetch-meta)       | Manual                                       |
| Rate limiting      | Automatic (600/60s)                        | Manual (via middleware)                      |
| Body limit         | 1 MiB default (max 16 MiB)                 | 10 MiB default (max 256 MiB)                 |
| Input validation   | Built-in `parse()`                         | Manual                                       |
| Cache invalidation | Built-in `invalidate()`                    | Manual                                       |
| Realtime events    | Built-in `.realtime()`                     | Manual WebSocket                             |
| Response format    | JSON only                                  | Any                                          |
| Streaming          | Not supported                              | Full ReadableStream support                  |
| Form integration   | Native HTML `action="/__ruvyxa/action?...` | Manual fetch                                 |

---

## Try It Yourself

Build a simple counter app with a server action.

**Step 1:** Create `app/counter/action.ts`:

```ts
import { action } from 'ruvyxa/server'

// In-memory counter (replace with DB in real app)
let count = 0

export const increment = action.input({ parse: () => ({}) }).handler(async ({ invalidate }) => {
  count++
  invalidate('counter:value')
  return { count }
})

export const reset = action.input({ parse: () => ({}) }).handler(async ({ invalidate }) => {
  count = 0
  invalidate('counter:value')
  return { count }
})
```

**Step 2:** Create `app/counter/page.tsx`:

```tsx
// This page uses HTML forms — no JS needed for basic functionality
export default function CounterPage() {
  return (
    <main>
      <h1>Counter</h1>
      <form
        method="post"
        action="/__ruvyxa/action?path=/counter&name=increment"
        style={{ display: 'inline' }}
      >
        <button type="submit">+1</button>
      </form>
      <form
        method="post"
        action="/__ruvyxa/action?path=/counter&name=reset"
        style={{ display: 'inline' }}
      >
        <button type="submit">Reset</button>
      </form>
    </main>
  )
}
```

**Step 3:** Visit `/counter`, click the buttons, watch the counter mutate.

**Step 4 (bonus):** Add a client component that fetches the current count and displays it:

```tsx
'use client'

import { useRuvyxaLoader } from '@ruvyxa/react'

export function DisplayCount() {
  const { data, loading } = useRuvyxaLoader(() => fetch('/api/counter').then((r) => r.json()))

  if (loading) return <p>Loading...</p>
  return <p>Count: {data?.count ?? 0}</p>
}
```

---

## The Action Builder Contract

The supported server-action API is a builder exported by `@ruvyxa/core/server`. It makes three
separate decisions explicit: optional input validation, optional realtime publication, and the
server handler. A schema only needs a `parse(value)` method, which keeps the framework independent
of a particular validation library.

```ts
import { action } from '@ruvyxa/core/server'

const createTodoInput = {
  parse(value: unknown) {
    if (
      !value ||
      typeof value !== 'object' ||
      typeof (value as { title?: unknown }).title !== 'string'
    ) {
      throw new TypeError('title is required')
    }
    return { title: (value as { title: string }).title.trim() }
  },
}

export const createTodo = action
  .input(createTodoInput)
  .realtime('todos')
  .handler(async ({ input, request, user, invalidate }) => {
    const todo = { id: crypto.randomUUID(), title: input.title, actor: user }
    invalidate('todos')
    return { todo, requestId: request.headers.get('x-request-id') }
  })
```

The handler context contains validated `input`, the incoming `request`, an optional `user` supplied
by the runtime integration, and `invalidate(key)`. It does not create authentication or persistent
storage by itself; the application remains responsible for authenticating the request and writing to
its own data store.

### Realtime Channels Have Deliberate Limits

`.realtime()` accepts one channel or a list of channels. Channel names are trimmed, de-duplicated,
limited to 16 values, and must contain 1–128 letters, digits, `:`, `.`, `_`, `/`, or `-`. Omitting
the argument selects the route channel. Treat a realtime event as a notification to refresh or
invalidate state, not as proof that every subscriber has received a durable database change.

### Request Safety Happens Before the Handler

The action endpoint applies its request validation and configured rate limiter before it asks the
worker to run the action. `security.actionLimit`, `security.actionRateLimit`, same-origin checks,
Fetch Metadata checks, and trusted-proxy settings therefore belong in the deployment/security
review; they are not replaced by a schema's `parse` method.

Use `ruvyxa analyze --format human` to check that action dependencies stay out of the client graph,
then use `npm run check` for the project-level type/parity gate. Do not document or call a
`'use server'` directive as the action registration mechanism: route-local `action.ts`/`action.js`
and exported action values are the current route convention.

---

## Next Steps

- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — Caching server-side data
- **[07-api-routes.md](./07-api-routes.md)** — Building REST APIs
- **[03-server-client-components.md](./03-server-client-components.md)** — Server vs client
  components
