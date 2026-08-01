# API Routes

API routes let you build HTTP endpoints right inside your `app/` folder. Use them for REST APIs,
webhooks, proxy endpoints, or anything that returns a non-HTML response. Each route file exports
named functions for HTTP methods — Ruvyxa handles routing, method dispatch, body limits, security
headers, and middleware automatically.

```
app/api/
├── hello/
│   └── route.ts      →  GET /api/hello
├── todos/
│   └── route.ts      →  GET/POST /api/todos
└── webhooks/
    └── stripe/
        └── route.ts  →  POST /api/webhooks/stripe
```

---

## Type Definitions

### Handler Signature

```ts
// Standard Web API Request + Ruvyxa params
type ApiHandler = (ctx: {
  request: Request // Standard Web API Request object
  params: Record<string, string | string[]>
}) => Response | Promise<Response>

// Export named functions for each HTTP method
export function GET(ctx: { request: Request; params: Params }): Response | Promise<Response>
export function POST(ctx: { request: Request; params: Params }): Response | Promise<Response>
export function PUT(ctx: { request: Request; params: Params }): Response | Promise<Response>
export function DELETE(ctx: { request: Request; params: Params }): Response | Promise<Response>
export function PATCH(ctx: { request: Request; params: Params }): Response | Promise<Response>
export function HEAD(ctx: { request: Request; params: Params }): Response | Promise<Response>
export function OPTIONS(ctx: { request: Request; params: Params }): Response | Promise<Response>
```

### Config Types

```ts
export interface RuvyxaConfig {
  security?: {
    apiLimit?: number // @default 10485760 (10 MiB), @maximum 268435456 (256 MiB)
  }
}
```

### Server-Side Constants

```rust
// file: crates/ruvyxa_dev_server/src/lib.rs
const MAX_API_BODY_BYTES: usize = 10 * 1024 * 1024;                // 10 MiB — default
pub const MAX_API_BODY_LIMIT_BYTES: usize = 256 * 1024 * 1024;    // 256 MiB — hard max
```

---

## Creating an API Route

Create a `route.ts` file and export HTTP method handlers. Each handler receives a standard Web API
`Request` and `params`.

```ts
// app/api/hello/route.ts
export function GET({
  request,
  params,
}: {
  request: Request
  params: Record<string, string | string[]>
}) {
  return Response.json({
    message: 'Hello, world!',
    timestamp: new Date().toISOString(),
  })
}
```

Visit `GET /api/hello`:

```json
{
  "message": "Hello, world!",
  "timestamp": "2026-07-29T21:00:00.000Z"
}
```

### `request` — Standard Web API Request

The full [Web API `Request`](https://developer.mozilla.org/en-US/docs/Web/API/Request) interface is
available:

```ts
request.url // Full URL including query string
request.method // HTTP method (GET, POST, etc.)
request.headers // Headers object
request.body // ReadableStream | null
request.bodyUsed // Whether body has been read

// Body reading methods (each consumes the body once):
await request.json() // Parse as JSON
await request.text() // Read as string
await request.formData() // Parse as FormData
await request.blob() // Read as Blob
await request.arrayBuffer() // Read as ArrayBuffer
```

**Note**: Body methods are single-use. Calling `.json()` then `.text()` will throw — the body is a
stream that can only be consumed once.

### `params` — Route Parameters

`params` is synchronous and populated from the file-system route:

```ts
// For route: app/api/users/[id]/route.ts
// URL: /api/users/42
params = { id: '42' }

// For route: app/api/proxy/[...path]/route.ts
// URL: /api/proxy/a/b/c
params = { path: ['a', 'b', 'c'] }
```

---

## Supported Methods

Export any of these named functions:

| Export    | HTTP Method | Typical Use                      |
| --------- | ----------- | -------------------------------- |
| `GET`     | GET         | Read/retrieve resources          |
| `POST`    | POST        | Create resources                 |
| `PUT`     | PUT         | Full update/replace              |
| `DELETE`  | DELETE      | Remove resources                 |
| `PATCH`   | PATCH       | Partial update                   |
| `HEAD`    | HEAD        | Headers only (no body)           |
| `OPTIONS` | OPTIONS     | CORS preflight, capability check |

Each handler receives the same arguments:

```ts
export async function GET({
  request,
  params,
}: {
  request: Request // Standard Web API Request
  params: Record<string, string | string[]>
}) {
  // ...
}
```

---

## Response Types

### JSON Response

```ts
// app/api/users/route.ts
import { db } from '../../server/db'

export async function GET() {
  const users = await db.query('SELECT id, name, email FROM users')
  return Response.json(users)
}
```

`Response.json(data, init?)` sets `Content-Type: application/json` automatically. Second argument
accepts `status`, `headers`, `statusText`.

### Custom Status Code

```ts
export async function POST({ request }: { request: Request }) {
  const body = await request.json()

  if (!body.name) {
    return Response.json({ error: 'Name is required' }, { status: 400 })
  }

  const user = await db.query('INSERT INTO users (name) VALUES (?) RETURNING *', [body.name])
  return Response.json(user, { status: 201 })
}
```

Common status codes for API routes:

| Code | Meaning               | When to Use                        |
| ---- | --------------------- | ---------------------------------- |
| 200  | OK                    | Successful GET, PUT, PATCH, DELETE |
| 201  | Created               | Successful POST (new resource)     |
| 204  | No Content            | Successful DELETE (no body needed) |
| 400  | Bad Request           | Invalid input, missing fields      |
| 401  | Unauthorized          | Missing/invalid authentication     |
| 403  | Forbidden             | Authenticated but not allowed      |
| 404  | Not Found             | Resource doesn't exist             |
| 409  | Conflict              | Resource state conflict            |
| 413  | Payload Too Large     | Body exceeds `apiLimit`            |
| 422  | Unprocessable Entity  | Validation failure                 |
| 429  | Too Many Requests     | Rate limit hit                     |
| 500  | Internal Server Error | Unhandled server error             |

### Redirect

```ts
export async function GET() {
  return Response.redirect('https://example.com', 302)
}
```

`Response.redirect(url, status)` — status must be in 3xx range (default 302).

### Text / HTML

```ts
export async function GET() {
  return new Response('Hello, world!', {
    headers: { 'Content-Type': 'text/plain' },
  })
}
```

### Binary Data

```ts
export async function GET() {
  const image = await fs.promises.readFile('./public/image.png')
  return new Response(image, {
    headers: { 'Content-Type': 'image/png' },
  })
}
```

Binary response bodies are passed through as `Uint8Array`/`ArrayBuffer` without base64 encoding.

### Streaming

```ts
// app/api/stream/route.ts
export async function GET() {
  const stream = new ReadableStream({
    async start(controller) {
      controller.enqueue(new TextEncoder().encode('data: hello\n\n'))
      await new Promise((r) => setTimeout(r, 1000))
      controller.enqueue(new TextEncoder().encode('data: world\n\n'))
      controller.close()
    },
  })

  return new Response(stream, {
    headers: {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
    },
  })
}
```

#### Streaming Behavior

| Aspect          | Detail                                             |
| --------------- | -------------------------------------------------- |
| Frame size      | 64 KiB chunks                                      |
| Default timeout | `RUVYXA_WORKER_TIMEOUT_MS` (default 30s)           |
| Backpressure    | Consumer slow → producer pauses (stream protocol)  |
| Compression     | **Skipped** for live streams (no Content-Encoding) |
| Compression     | **Applied** for buffered responses                 |
| Error handling  | Stream error → 500 with Connection: close          |

### Error Response

```ts
export async function GET() {
  return Response.error()
  // Returns a 500 response with network error semantics
}
```

---

## Dynamic Segments in API Routes

Dynamic params work the same as in pages.

```
app/api/
  users/
    [id]/
      route.ts   →  /api/users/:id
```

```ts
// app/api/users/[id]/route.ts
export async function GET({ params }: { params: { id: string } }) {
  const user = await db.query('SELECT * FROM users WHERE id = ?', [params.id])

  if (!user) {
    return Response.json({ error: 'Not found' }, { status: 404 })
  }

  return Response.json(user)
}
```

| URL              | `params.id` |
| ---------------- | ----------- |
| `/api/users/42`  | `"42"`      |
| `/api/users/abc` | `"abc"`     |

### Catch-all segments

```
app/api/
  proxy/
    [...path]/
      route.ts   →  /api/proxy/*
```

```ts
// app/api/proxy/[...path]/route.ts
export async function GET({ params }: { params: { path: string[] } }) {
  const targetPath = params.path.join('/')
  const response = await fetch(`https://api.example.com/${targetPath}`)
  return new Response(response.body, {
    status: response.status,
    headers: { 'Content-Type': 'application/json' },
  })
}
```

### Parameter Extraction

Params are extracted from the URL path using the Radix trie router:

```
For pattern: /api/users/[id]/posts/[postId]
URL: /api/users/42/posts/7
→ params = { id: "42", postId: "7" }

For pattern: /api/files/[...segments]
URL: /api/files/a/b/c.txt
→ params = { segments: ["a", "b", "c.txt"] }
```

### Parameter Type Semantics

- Single `[param]`: always `string`
- Catch-all `[...param]`: `string[]` (never undefined)
- Optional catch-all `[[...param]]`: `string[]` or absent from params object (if path ends before
  it)

---

## Full CRUD Example

```ts
// app/api/todos/route.ts
import { db } from '../../server/db'

// GET /api/todos — list all todos
export async function GET() {
  const todos = await db.query('SELECT * FROM todos ORDER BY created_at DESC')
  return Response.json(todos)
}

// POST /api/todos — create a todo
export async function POST({ request }: { request: Request }) {
  const body = await request.json()

  if (typeof body.text !== 'string' || body.text.trim().length === 0) {
    return Response.json({ error: 'text is required' }, { status: 400 })
  }

  const todo = await db.query('INSERT INTO todos (text, done) VALUES (?, false) RETURNING *', [
    body.text.trim(),
  ])

  return Response.json(todo, { status: 201 })
}
```

```ts
// app/api/todos/[id]/route.ts
import { db } from '../../../server/db'

// GET /api/todos/:id
export async function GET({ params }: { params: { id: string } }) {
  const todo = await db.query('SELECT * FROM todos WHERE id = ?', [params.id])
  if (!todo) return Response.json({ error: 'Not found' }, { status: 404 })
  return Response.json(todo)
}

// PUT /api/todos/:id
export async function PUT({ request, params }: { request: Request; params: { id: string } }) {
  const body = await request.json()

  const todo = await db.query(
    'UPDATE todos SET text = COALESCE(?, text), done = COALESCE(?, done) WHERE id = ? RETURNING *',
    [body.text ?? null, body.done ?? null, params.id],
  )

  if (!todo) return Response.json({ error: 'Not found' }, { status: 404 })
  return Response.json(todo)
}

// DELETE /api/todos/:id
export async function DELETE({ params }: { params: { id: string } }) {
  const deleted = await db.query('DELETE FROM todos WHERE id = ? RETURNING id', [params.id])
  if (!deleted) return Response.json({ error: 'Not found' }, { status: 404 })
  return Response.json({ ok: true })
}
```

---

## Input Validation

Use the standard `Request.json()` or `Request.formData()` methods, then validate manually.

```ts
export async function POST({ request }: { request: Request }) {
  let body: unknown
  try {
    body = await request.json()
  } catch {
    return Response.json({ error: 'Invalid JSON' }, { status: 400 })
  }

  if (typeof body !== 'object' || body === null) {
    return Response.json({ error: 'Body must be an object' }, { status: 400 })
  }

  const data = body as Record<string, unknown>

  const errors: string[] = []

  if (typeof data.email !== 'string') errors.push('email must be a string')
  if (typeof data.age !== 'number') errors.push('age must be a number')

  if (errors.length > 0) {
    return Response.json({ error: 'Validation failed', details: errors }, { status: 422 })
  }

  // data is validated
  const user = await createUser(data.email as string, data.age as number)
  return Response.json(user, { status: 201 })
}
```

### Validation Libraries

You can use any validation library:

```ts
import { z } from 'zod'

const CreateUserSchema = z.object({
  email: z.string().email(),
  age: z.number().min(18).max(120),
})

export async function POST({ request }: { request: Request }) {
  const body = await request.json()
  const result = CreateUserSchema.safeParse(body)

  if (!result.success) {
    return Response.json(
      { error: 'Validation failed', details: result.error.flatten() },
      { status: 422 },
    )
  }

  const user = await createUser(result.data)
  return Response.json(user, { status: 201 })
}
```

---

## Body Size Limits

| Endpoint Type | Default Limit | Config Key                    | Hard Maximum |
| ------------- | ------------- | ----------------------------- | ------------ |
| API Routes    | 10 MiB        | `apiLimit` (in `security`)    | 256 MiB      |
| Actions       | 1 MiB         | `actionLimit` (in `security`) | 16 MiB       |

Configure in `ruvyxa.config.ts`:

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'

export default config({
  security: {
    apiLimit: 50 * 1024 * 1024, // 50 MiB — for file upload APIs
  },
})
```

If a request exceeds the limit, it returns **413 Content Too Large**:

```json
{
  "error": "Request body exceeded the API body limit or could not be read: ..."
}
```

The body size is checked **before** the request reaches your handler — the entire body is buffered
to enforce the limit.

---

## Unsupported Methods

If a method is not exported, Ruvyxa returns **405 Method Not Allowed**.

```
curl -X PATCH http://localhost:3000/api/todos
→ 405 Method Not Allowed (Allow: GET, POST)
```

The `Allow` header is set automatically based on exported handlers:

```
Allow: GET, POST, OPTIONS
```

### 405 Response Details

- **Status**: 405 Method Not Allowed
- **Headers**: `Allow` with comma-separated list of supported methods
- **Body**: Empty

---

## Middleware and Security Headers

Ruvyxa applies security headers automatically to all API responses:

| Header                    | Value                  | Overridable         |
| ------------------------- | ---------------------- | ------------------- |
| `X-Content-Type-Options`  | `nosniff`              | No (always applied) |
| `X-Frame-Options`         | `DENY`                 | No (always applied) |
| `Content-Security-Policy` | Configurable           | Yes — via config    |
| `X-RateLimit-Limit`       | Per-route (if enabled) | No                  |

Custom headers are merged on top of defaults:

```ts
export async function GET() {
  return Response.json(
    { ok: true },
    {
      headers: {
        'X-Custom-Header': 'my-value',
        // These take precedence over defaults
      },
    },
  )
}
```

### Disabling Security Headers

```ts
export default config({
  security: {
    headers: false,
  },
})
```

### Middleware

API routes automatically go through the same middleware stack as page routes:

- Rate limiting (if configured)
- CORS (if configured)
- Custom headers
- Compression
- Plugin middleware

There is **no per-endpoint rate limiting** for API routes by default — rate limiting is opt-in via
the `middleware` config.

---

## Set-Cookie Handling

Multiple `Set-Cookie` headers are preserved correctly:

```ts
export async function GET() {
  return new Response(null, {
    status: 302,
    headers: {
      Location: '/',
      'Set-Cookie': ['session=abc123; HttpOnly; Path=/', 'theme=dark; Path=/'].join(', '),
    },
  })
}
```

**Note**: `Set-Cookie` is the one header where multiple values are semantically meaningful. Axum and
the Web API handle this via the `Headers` multi-value mechanism. Use
`headers.append("Set-Cookie", "...")` if available, or join with `\n` for multi-value support.

---

## Binary Data and Plugin Transport

When API routes pass through TypeScript plugin middleware, binary bodies are base64-encoded:

```ts
interface PluginHttpRequest {
  method: string
  path: string
  headers: [string, string][] // headerPairs
  body_base64?: string // base64-encoded body
}
```

For direct API route handling (no plugin middleware), binary data flows as raw bytes.

---

## Streaming — In-Depth

### Frame Size

Streams send data in 64 KiB chunks. This is the internal buffer size for the underlying transport.

### Timeout

The worker timeout (`RUVYXA_WORKER_TIMEOUT_MS`, default 30 seconds) applies to the **entire stream
duration**. If the stream takes longer than the timeout, the worker is terminated.

### Backpressure

```
Producer ──→ [64KiB buffer] ──→ Consumer

If consumer is slow:
  - Buffer fills → backpressure signal → producer pauses
  - When consumer catches up → resume producing
```

This is standard Web Streams API behavior. Use `controller.desiredSize` to check backpressure.

### Compression

| Scenario                              | Compression                                      |
| ------------------------------------- | ------------------------------------------------ |
| Buffered response (small, known size) | Compressed with gzip/brotli                      |
| Live stream (chunked transfer)        | **No compression** (end-to-end stream preserved) |

### Error Handling in Streams

```ts
export async function GET() {
  const stream = new ReadableStream({
    start(controller) {
      // If this throws:
      //   - Stream is errored
      //   - HTTP response becomes 500 with Connection: close
      //   - Client receives whatever was sent + connection drop
    },
  })
  return new Response(stream)
}
```

---

## Edge Cases

| Scenario                                    | Behavior                                                                            |
| ------------------------------------------- | ----------------------------------------------------------------------------------- |
| **File not found in app/api/**              | Routes must match existing files; otherwise 404                                     |
| **Method exported but async**               | Works — Ruvyxa awaits the returned Promise                                          |
| **Handler throws synchronously**            | Caught and returned as 500                                                          |
| **Handler returns `null` or `undefined`**   | Treated as empty 200 response                                                       |
| **Response with no body**                   | Returns 204 No Content (or 200 with 0 Content-Length)                               |
| **Query string on request**                 | Available via `request.url` — params object does NOT include query                  |
| **Request body on GET**                     | Allowed but unusual; use `request.text()` or `request.json()`                       |
| **Multiple headers with same name**         | `request.headers.get()` returns first; `request.headers.getSetCookie()` for cookies |
| **`params` out of sync**                    | Parameter names come from the matched route pattern, not the trie                   |
| **Response headers already set**            | Merged with security headers; your values take precedence                           |
| **HEAD request with GET handler**           | Ruvyxa calls GET handler internally, strips response body                           |
| **OPTIONS request without OPTIONS handler** | Returns 405 with Allow header listing exported methods                              |

---

## Performance Characteristics

| Operation                       | Overhead                         |
| ------------------------------- | -------------------------------- |
| Route matching                  | O(path depth) — Radix trie       |
| Body buffering (up to apiLimit) | Memory proportional to body size |
| Handler dispatch                | ~1ms (IPC to Node worker)        |
| Response streaming              | Negligible per chunk             |
| Security headers                | ~0.01ms                          |

### Comparison: API Route vs Direct Handler

| Aspect           | API Route         | Direct Handler |
| ---------------- | ----------------- | -------------- |
| Body limit       | 10 MiB (default)  | N/A            |
| Security headers | Automatic         | Manual         |
| Middleware       | Automatic         | Manual         |
| Routing          | File-system based | Manual         |
| Streaming        | Full support      | Full support   |
| Worker dispatch  | Yes (~1ms)        | Native         |

---

## Error Codes

| Code | Condition                 | HTTP Status               |
| ---- | ------------------------- | ------------------------- |
| N/A  | Body exceeds `apiLimit`   | 413 Payload Too Large     |
| N/A  | Method not exported       | 405 Method Not Allowed    |
| N/A  | Invalid request path      | 400 Bad Request           |
| N/A  | Handler throws            | 500 Internal Server Error |
| N/A  | Stream error mid-response | 500 + Connection: close   |

---

## Troubleshooting

| Symptom                              | Cause                                   | Fix                                                       |
| ------------------------------------ | --------------------------------------- | --------------------------------------------------------- |
| 405 on POST                          | POST not exported from route.ts         | Add `export async function POST(...)`                     |
| 413 on file upload                   | Body exceeds `apiLimit`                 | Increase `apiLimit` in config                             |
| Empty response                       | Handler returns `null`                  | Return explicit `Response`                                |
| `request.json()` throws              | Body not valid JSON                     | Wrap in try/catch, return 400                             |
| CORS errors in browser               | No CORS middleware configured           | Add `cors` config to middleware                           |
| 404 on valid path                    | Route file deleted or not discovered    | Check app/api/ structure matches path                     |
| Params object empty                  | Wrong export name or path name          | Match `[param]` naming to `params.param`                  |
| `params` is `string[]` for catch-all | Expected for `[...path]`                | Join with `.join("/")` for string                         |
| Stream cuts off early                | Worker timeout exceeded                 | Increase `middleware.timeoutMs` or reduce stream duration |
| Headers not appearing in response    | Security headers override (unlikely)    | Merge headers in Response init object                     |
| Set-Cookie not working               | Multiple Set-Cookie not properly joined | Use `headers.append` or join with comma                   |

---

## HTTP Query Parameters

```tsx
// app/api/products/route.ts
export function GET({ request }: { request: Request }) {
  const url = new URL(request.url)
  const category = url.searchParams.get('category')
  const inStock = url.searchParams.get('inStock')
  const page = parseInt(url.searchParams.get('page') ?? '1', 10)
  const limit = parseInt(url.searchParams.get('limit') ?? '20', 10)

  // pagination
  const start = (page - 1) * limit
  const result = products
    .filter((p) => !category || p.category === category)
    .filter((p) => inStock === null || p.inStock === (inStock === 'true'))
    .slice(start, start + limit)

  return Response.json({
    data: result,
    page,
    limit,
    total: products.length,
  })
}
```

---

## Under the Hood: API Route Execution

```
Request → /api/products
    │
    └── router.find(path)
        │
        ├── if no route found → 404
        │
        ├── if route is a page → use render API? (No)
        │   (API routes must be route.ts, not page.tsx)
        │
        ├── if route is an API → find route.ts
        │
        ├── worker_pool.render_api()
        │   ├── serialize request → WorkerRequest::Api
        │   │   { route_file, method, request_path,
        │   │     headers, header_pairs, body, body_base64,
        │   │     stream_response, params }
        │   ├── worker selects export function by method
        │   ├── if no export → return 405
        │   ├── execute handler
        │   └── return WorkerApiResponse
        │
        └── render_api_pooled()
            ├── if !ok → RUV1200 diagnostic
            ├── streamed_body? → streaming body
            ├── headers → append to response
            └── security_headers → final response
```

### Worker Pool Timeouts

| Context    | Default Timeout     | Environment Variable       |
| ---------- | ------------------- | -------------------------- |
| Dev server | 30,000 ms (30s)     | `RUVYXA_WORKER_TIMEOUT_MS` |
| Build      | 300,000 ms (5 mins) | `RUVYXA_WORKER_TIMEOUT_MS` |

A timeout of 0 or an invalid value will be reset to the default value.

**Concurrency per worker:** A single worker can handle concurrent requests up to
`RUVYXA_WORKER_MAX_CONCURRENCY` (default: number of cores bounded to 2–8). Rendering consumes CPU,
and each execution holds a React tree, a compiled bundle, and a response buffer. If a massive burst
is received concurrently, it may exhaust the heap or cause CPU thrashing, leading to a timeout that
looks like a hung request. Excess requests are queued and processed sequentially as slots become
available.

---

## Routing Integration

```
app/
  api/
    route.ts          ← API route at /api
    users/
      route.ts        ← API route at /api/users
      [id]/
        route.ts      ← API route at /api/users/123
    blog/
      [slug]/
        route.ts      ← API route at /api/blog/hello-world
  products/
    page.tsx          ← Page route at /products (UI)
    route.ts          ← API route at /products (data) — can co-exist with page.tsx
```

`route.ts` can co-exist with `page.tsx` in the same directory — Ruvyxa handles this automatically.

---

## Under the Hood: Route Kind Detection

Ruvyxa determines whether a route is an API route or a page route based on:

- File named `route.ts` → API route
- File named `page.tsx` → page route
- If both exist → route supports both (API + page)

Only page routes support server actions (RUV1501 is returned if an action is called on a route
without `action.ts`).

---

## Best Practices

1. **One file per resource.** `todos/route.ts` for all `/api/todos` handlers, `todos/[id]/route.ts`
   for single-todo operations.

2. **Validate input early.** Check types and return 400 with a clear error message.

3. **Return consistent error shapes.** Use `{ error: string }` or
   `{ error: string, details: string[] }`.

4. **Use appropriate status codes.** 201 for created, 400 for bad request, 404 for not found, 422
   for validation failure.

5. **Set `apiLimit` for file uploads.** Default 10 MiB is fine for JSON APIs, but file uploads need
   more.

6. **Leverage dynamic params.** `[id]` segments keep your URLs clean and RESTful.

7. **Stream for long responses.** Use `ReadableStream` if returning large datasets or real-time
   data.

8. **Use middleware for cross-cutting concerns.** Rate limiting, authentication, logging — apply
   globally via middleware config.

9. **Avoid heavy computation in API routes.** Long-running handlers block the worker thread. Offload
   to background jobs or streaming.

10. **Never trust `params` without validation.** Even though params come from the URL pattern,
    validate them if they're used in queries or business logic.

---

## Webhook Handling

API routes are ideal for webhook endpoints:

```ts
// app/api/webhooks/stripe/route.ts
export async function POST({ request }: { request: Request }) {
  const signature = request.headers.get('stripe-signature')
  if (!signature) {
    return Response.json({ error: 'Missing signature' }, { status: 401 })
  }

  const body = await request.text()

  // Verify webhook signature
  let event: Stripe.Event
  try {
    event = stripe.webhooks.constructEvent(body, signature, process.env.STRIPE_WEBHOOK_SECRET!)
  } catch {
    return Response.json({ error: 'Invalid signature' }, { status: 401 })
  }

  // Handle event
  switch (event.type) {
    case 'payment_intent.succeeded':
      await handlePaymentSuccess(event.data.object)
      break
    case 'customer.subscription.updated':
      await handleSubscriptionUpdate(event.data.object)
      break
  }

  return Response.json({ received: true }, { status: 200 })
}
```

### Webhook Best Practices

| Practice               | Reason                            |
| ---------------------- | --------------------------------- |
| Verify signature       | Prevent spoofed events            |
| Respond quickly (≤10s) | Webhook senders timeout and retry |
| Idempotency keys       | Prevent duplicate processing      |
| Queue heavy work       | Don't block the webhook response  |
| Log raw payloads       | Debugging webhook issues          |
| Return 200 quickly     | Acknowledge receipt immediately   |

## Rate Limiting API Routes

Apply rate limiting via middleware config:

```ts
export default config({
  middleware: {
    builtin: {
      rate: {
        max: 100,
        window: 60, // 100 requests per 60 seconds
        key: 'ip', // Rate limit by IP address
      },
    },
  },
})
```

### Per-Route Rate Limiting

For route-specific limits, use custom logic:

```ts
// app/api/rate-limited/route.ts
const requestCounts = new Map<string, { count: number; resetAt: number }>()

export async function GET({ request }: { request: Request }) {
  const ip = request.headers.get('x-forwarded-for') || 'unknown'
  const now = Date.now()
  const entry = requestCounts.get(ip)

  if (entry && now < entry.resetAt) {
    if (entry.count >= 10) {
      return Response.json(
        { error: 'Rate limit exceeded' },
        {
          status: 429,
          headers: {
            'Retry-After': String(Math.ceil((entry.resetAt - now) / 1000)),
            'X-RateLimit-Limit': '10',
            'X-RateLimit-Remaining': '0',
          },
        },
      )
    }
    entry.count++
  } else {
    requestCounts.set(ip, { count: 1, resetAt: now + 60_000 })
  }

  return Response.json({ ok: true })
}
```

## Authentication Patterns

### API Key Auth

```ts
// app/api/secure/route.ts
export async function GET({ request }: { request: Request }) {
  const apiKey = request.headers.get('x-api-key')
  if (!apiKey || apiKey !== process.env.API_KEY) {
    return Response.json({ error: 'Unauthorized' }, { status: 401 })
  }
  return Response.json({ secret: 'data' })
}
```

### Bearer Token Auth

```ts
export async function GET({ request }: { request: Request }) {
  const auth = request.headers.get('authorization')
  if (!auth?.startsWith('Bearer ')) {
    return Response.json({ error: 'Missing or invalid authorization header' }, { status: 401 })
  }

  const token = auth.slice(7)
  try {
    const payload = await verifyJwt(token)
    const user = await db.query('SELECT * FROM users WHERE id = ?', [payload.sub])
    if (!user) {
      return Response.json({ error: 'User not found' }, { status: 401 })
    }
    return Response.json({ user })
  } catch {
    return Response.json({ error: 'Invalid or expired token' }, { status: 401 })
  }
}
```

### Session Auth

```ts
export async function GET({ request }: { request: Request }) {
  const cookie = request.headers.get('cookie')
  const sessionId = cookie
    ?.split(';')
    .find((c) => c.trim().startsWith('session='))
    ?.split('=')[1]
    ?.trim()

  if (!sessionId) {
    return Response.json({ error: 'Not authenticated' }, { status: 401 })
  }

  const session = await db.query('SELECT * FROM sessions WHERE id = ?', [sessionId])
  if (!session || session.expiresAt < Date.now()) {
    return Response.json({ error: 'Session expired' }, { status: 401 })
  }

  return Response.json({ user: session.user })
}
```

## Testing API Routes

Test handlers in isolation:

```ts
// tests/api/users.test.ts
import { GET, POST } from '../../app/api/users/route'

describe('GET /api/users', () => {
  it('returns user list', async () => {
    const response = await GET({
      request: new Request('http://localhost/api/users'),
      params: {},
    })
    expect(response.status).toBe(200)
    const data = await response.json()
    expect(Array.isArray(data)).toBe(true)
  })
})

describe('POST /api/users', () => {
  it('creates a user', async () => {
    const response = await POST({
      request: new Request('http://localhost/api/users', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: 'Test', email: 'test@example.com' }),
      }),
      params: {},
    })
    expect(response.status).toBe(201)
  })

  it('rejects invalid input', async () => {
    const response = await POST({
      request: new Request('http://localhost/api/users', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({}),
      }),
      params: {},
    })
    expect(response.status).toBe(400)
  })
})
```

## Advanced Streaming Patterns

### Server-Sent Events (SSE)

```ts
// app/api/events/route.ts
export async function GET() {
  const stream = new ReadableStream({
    start(controller) {
      // Send events every 5 seconds
      const interval = setInterval(() => {
        const data = JSON.stringify({ time: new Date().toISOString(), value: Math.random() })
        controller.enqueue(new TextEncoder().encode(`data: ${data}\n\n`))
      }, 5000)

      // Cleanup on cancellation
      controller.signal?.addEventListener('abort', () => {
        clearInterval(interval)
      })
    },
  })

  return new Response(stream, {
    headers: {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
    },
  })
}
```

### Streaming Large JSON Arrays

```ts
// app/api/large-dataset/route.ts
export async function GET() {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder()
      controller.enqueue(encoder.encode('['))

      const cursor = db.queryStream('SELECT * FROM large_table')
      let first = true

      for await (const row of cursor) {
        if (!first) controller.enqueue(encoder.encode(','))
        controller.enqueue(encoder.encode(JSON.stringify(row)))
        first = false
      }

      controller.enqueue(encoder.encode(']'))
      controller.close()
    },
  })

  return new Response(stream, {
    headers: { 'Content-Type': 'application/json' },
  })
}
```

### Progress Streaming

```ts
// app/api/progress/route.ts
export async function GET() {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder()
      const total = 100

      for (let i = 0; i <= total; i += 10) {
        controller.enqueue(encoder.encode(JSON.stringify({ progress: i, total }) + '\n'))
        await new Promise((r) => setTimeout(r, 500))
      }

      controller.enqueue(encoder.encode(JSON.stringify({ done: true }) + '\n'))
      controller.close()
    },
  })

  return new Response(stream, {
    headers: { 'Content-Type': 'application/x-ndjson' },
  })
}
```

## Error Handling Patterns

### Unified Error Handler

```ts
function apiError(status: number, message: string, details?: string[]) {
  return Response.json({ error: message, ...(details ? { details } : {}) }, { status })
}

export async function GET() {
  try {
    const data = await riskyOperation()
    return Response.json(data)
  } catch (error) {
    if (error instanceof NotFoundError) {
      return apiError(404, error.message)
    }
    if (error instanceof ValidationError) {
      return apiError(422, 'Validation failed', error.details)
    }
    console.error('Unhandled error:', error)
    return apiError(500, 'Internal server error')
  }
}
```

### Idempotency Middleware

```ts
const processedIds = new Set<string>()

export async function POST({ request }: { request: Request }) {
  const idempotencyKey = request.headers.get('Idempotency-Key')
  if (!idempotencyKey) {
    return Response.json({ error: 'Idempotency-Key header required' }, { status: 400 })
  }

  if (processedIds.has(idempotencyKey)) {
    return Response.json({ error: 'Request already processed' }, { status: 409 })
  }

  processedIds.add(idempotencyKey)
  // Process request...
  return Response.json({ ok: true }, { status: 201 })
}
```

## CORS Configuration

API routes that need to be accessed from different origins need CORS headers:

```ts
export default config({
  middleware: {
    builtin: {
      cors: {
        origins: ['https://myapp.com', 'https://admin.myapp.com'],
        methods: ['GET', 'POST', 'PUT', 'DELETE'],
        headers: ['Content-Type', 'Authorization'],
        credentials: true,
        maxAge: 86400,
      },
    },
  },
})
```

Manual CORS in a route:

```ts
export async function OPTIONS({ request }: { request: Request }) {
  return new Response(null, {
    headers: {
      'Access-Control-Allow-Origin': request.headers.get('origin') || '*',
      'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type, Authorization',
      'Access-Control-Max-Age': '86400',
    },
  })
}

export async function GET() {
  const origin = request.headers.get('origin') || '*'
  const data = await getData()
  return Response.json(data, {
    headers: { 'Access-Control-Allow-Origin': origin },
  })
}
```

## Environment Variables

Access server-side env vars in API routes:

```ts
export async function GET() {
  return Response.json({
    // Private: only available server-side
    databaseUrl: process.env.DATABASE_URL?.slice(0, 10) + '...',
    // Public: prefixed with RUVYXA_PUBLIC_, available in client
    publicApiUrl: process.env.RUVYXA_PUBLIC_API_URL,
  })
}
```

---

## Try It Yourself

Build a simple notes API.

**Step 1:** Create `app/api/notes/route.ts`:

```ts
// In-memory store (replace with DB in real app)
interface Note {
  id: number
  title: string
  body: string
}
let notes: Note[] = []
let nextId = 1

export async function GET() {
  return Response.json(notes)
}

export async function POST({ request }: { request: Request }) {
  const body = await request.json()
  if (typeof body.title !== 'string') {
    return Response.json({ error: 'title is required' }, { status: 400 })
  }

  const note: Note = { id: nextId++, title: body.title, body: body.body ?? '' }
  notes.push(note)
  return Response.json(note, { status: 201 })
}
```

**Step 2:** Create `app/api/notes/[id]/route.ts`:

```ts
interface Note {
  id: number
  title: string
  body: string
}
let notes: Note[] = [] // Same store (use DB in real app)

export async function GET({ params }: { params: { id: string } }) {
  const note = notes.find((n) => n.id === Number(params.id))
  if (!note) return Response.json({ error: 'Not found' }, { status: 404 })
  return Response.json(note)
}

export async function DELETE({ params }: { params: { id: string } }) {
  const idx = notes.findIndex((n) => n.id === Number(params.id))
  if (idx === -1) return Response.json({ error: 'Not found' }, { status: 404 })
  notes.splice(idx, 1)
  return Response.json({ ok: true })
}
```

**Step 3:** Test with curl:

```bash
curl http://localhost:3000/api/notes
# → []

curl -X POST http://localhost:3000/api/notes \
  -H "Content-Type: application/json" \
  -d '{"title": "My Note", "body": "Hello world"}'
# → {"id":1,"title":"My Note","body":"Hello world"}

curl http://localhost:3000/api/notes/1
# → {"id":1,"title":"My Note","body":"Hello world"}
```

---

## What an API Route Receives and Returns

An API entry is `route.ts` or `route.js`. The runtime looks up a named export matching the uppercase
HTTP method and invokes it with one object containing `request` and `params`. Returning a `Response`
preserves its status and headers; returning any other value is normalized to JSON.

```ts
// app/api/products/[id]/route.ts
export async function GET({ request, params }: { request: Request; params: { id: string } }) {
  const url = new URL(request.url)
  return Response.json({ id: params.id, include: url.searchParams.get('include') })
}

export async function DELETE({ params }: { params: { id: string } }) {
  await removeProduct(params.id)
  return new Response(null, { status: 204 })
}
```

If the matching named method is absent, the current runtime returns a
`405 Method <METHOD> is not allowed` response. That is different from a missing route, which is a
routing problem. Use method exports rather than a framework-specific request class: the handler
receives the standard Web `Request` and can return the standard Web `Response`.

### Body Limits Apply Before the Handler Runs

For methods that can carry a body, the dev server reads the body under `security.apiLimit` before
dispatching the API module. A request that exceeds that limit receives a payload-too-large response
without executing the handler. Parse and validate application data inside the handler as well:

```ts
export async function POST({ request }: { request: Request }) {
  const body: unknown = await request.json()
  if (
    !body ||
    typeof body !== 'object' ||
    typeof (body as { title?: unknown }).title !== 'string'
  ) {
    return Response.json({ error: 'title is required' }, { status: 400 })
  }

  return Response.json(
    { id: crypto.randomUUID(), title: (body as { title: string }).title },
    { status: 201 },
  )
}
```

Body-size enforcement limits transport size; it does not validate JSON shape, authorization, or
ownership. Keep those decisions in the handler or a server-only service it calls.

### Route Diagnostics Before Endpoint Debugging

When an endpoint returns an unexpected result, establish which route was discovered before changing
handler code:

```bash
ruvyxa routes
ruvyxa trace /api/products/[id]
ruvyxa analyze --format human
```

The route manifest separates page and API entries, and the analyzer treats API import graphs as
server graphs. Keep browser-only imports out of `route.ts`; a server graph that imports
`client-only` receives `RUV1009`.

---

## Next Steps

- **[06-server-actions.md](./06-server-actions.md)** — Server actions for mutations
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — Caching server-side data
- **[03-server-client-components.md](./03-server-client-components.md)** — Server vs client
  components
