# API Routes

API routes คือ HTTP endpoints ที่สร้างจากไฟล์ `route.ts` ในโฟลเดอร์ `app/` — เหมาะสำหรับ REST API,
webhooks, หรือ endpoint ที่ client components fetch

---

## route.ts พื้นฐาน

ไฟล์ `route.ts` export ฟังก์ชันตาม HTTP method ที่ต้องการรองรับ แต่ละฟังก์ชันรับ
`{ request, params }` และคืน `Response` object

```tsx
// app/api/health/route.ts
export function GET() {
  return Response.json({
    ok: true,
    framework: 'Ruvyxa',
    timestamp: new Date().toISOString(),
  })
}
```

เปิด `http://localhost:3000/api/health` → ได้ JSON:

```json
{
  "ok": true,
  "framework": "Ruvyxa",
  "timestamp": "2026-07-29T12:00:00.000Z"
}
```

---

## Handler Signature

```tsx
interface ApiHandlerContext {
  /** Request object ดั้งเดิม (URL, headers, method, body) */
  request: Request
  /** Dynamic route parameters จาก URL segments */
  params: Record<string, string | string[] | undefined>
}

// export ตาม method name — ไม่จำกัดแค่ GET/POST
export async function GET({ request, params }: ApiHandlerContext) {
  return Response.json({/* ... */})
}

export async function POST({ request }: { request: Request }) {
  return Response.json({/* ... */}, { status: 201 })
}
```

---

## Named HTTP Handlers

export ฟังก์ชันตาม HTTP method:

| Method    | ฟังก์ชัน export             | ใช้สำหรับ                   | Request Body         |
| --------- | --------------------------- | --------------------------- | -------------------- |
| `GET`     | `export function GET()`     | อ่านข้อมูล                  | ❌ ไม่มี body        |
| `POST`    | `export function POST()`    | สร้างข้อมูล                 | ✅ มี body           |
| `PUT`     | `export function PUT()`     | แทนที่ข้อมูลทั้งหมด         | ✅ มี body           |
| `PATCH`   | `export function PATCH()`   | แก้ไขบางส่วน                | ✅ มี body           |
| `DELETE`  | `export function DELETE()`  | ลบข้อมูล                    | ❌ ไม่มี body (ปกติ) |
| `HEAD`    | `export function HEAD()`    | headers เท่านั้น (metadata) | ❌ ไม่มี body        |
| `OPTIONS` | `export function OPTIONS()` | CORS preflight              | ❌ ไม่มี body        |

**Method ที่ไม่ได้ export → Ruvyxa คืน 405 Method Not Allowed อัตโนมัติ พร้อม header `Allow` ที่แสดง
methods ที่รองรับ**

```tsx
// app/api/products/route.ts
export function GET() {
  return Response.json([
    { id: '1', name: 'สินค้า A', price: 199 },
    { id: '2', name: 'สินค้า B', price: 299 },
  ])
}

export async function POST({ request }: { request: Request }) {
  const body = await request.json()
  return Response.json({ id: crypto.randomUUID(), ...body }, { status: 201 })
}

// DELETE ไม่ได้ export → 405
```

### 405 Error Format

เมื่อ client ส่ง method ที่ไม่รองรับ:

```
HTTP/1.1 405 Method Not Allowed
Allow: GET, POST, HEAD, OPTIONS
Content-Type: text/plain

Method Not Allowed
```

Header `Allow` จะถูกเพิ่มอัตโนมัติ โดย Ruvyxa ตรวจสอบ exports ที่มีในไฟล์

---

## Dynamic Segments

ใช้ dynamic segments ได้เหมือน pages:

```
app/
  api/
    products/
      route.ts          → GET /api/products
      [id]/
        route.ts        → GET /api/products/123
    users/
      [userId]/
        posts/
          route.ts      → GET /api/users/abc/posts
          [postId]/
            route.ts    → GET /api/users/abc/posts/456
```

```tsx
// app/api/products/[id]/route.ts
import type { PageProps } from 'ruvyxa/config'

export async function GET({ params }: { params: { id: string } }) {
  const product = await db.findProduct(params.id)

  if (!product) {
    return new Response('ไม่พบสินค้า', { status: 404 })
  }

  return Response.json(product)
}

export async function PATCH({ request, params }: { request: Request; params: { id: string } }) {
  const body = await request.json()
  const updated = await db.updateProduct(params.id, body)

  return Response.json(updated)
}

export async function DELETE({ params }: { params: { id: string } }) {
  await db.deleteProduct(params.id)
  return new Response(null, { status: 204 })
}
```

---

## Response Types

### Response.json()

```tsx
export function GET() {
  return Response.json({
    message: 'สำเร็จ',
    data: { id: 1, name: 'test' },
  })
  // → Content-Type: application/json
  // → status: 200
}
```

### new Response()

```tsx
export function GET() {
  return new Response(JSON.stringify({ error: 'ไม่พบข้อมูล' }), {
    status: 404,
    headers: {
      'Content-Type': 'application/json',
      'X-Custom-Header': 'value',
    },
  })
}
```

### redirect() จาก ruvyxa/server

```tsx
import { redirect } from 'ruvyxa/server'

export function GET() {
  return redirect('/login', 302) // status ต้องเป็น 3xx เท่านั้น
}
```

`redirect()` จะ throw error ถ้า status ไม่อยู่ในช่วง 300-399

### notFound() จาก ruvyxa/server

```tsx
import { notFound } from 'ruvyxa/server'

export function GET() {
  return notFound('ไม่พบข้อมูล')
  // → HTTP 404, body = "ไม่พบข้อมูล"
}
```

### json() จาก ruvyxa/server

```tsx
import { json } from 'ruvyxa/server'

export function GET() {
  return json({ data: 'ok' }, { status: 200 })
  // เหมือน Response.json()
}
```

---

## Input Validation

```tsx
// app/api/users/route.ts
export async function POST({ request }: { request: Request }) {
  try {
    const body = await request.json()

    // validate ทุก field
    if (!body.email || typeof body.email !== 'string') {
      return Response.json({ error: 'ต้องระบุอีเมล' }, { status: 400 })
    }

    if (!body.email.includes('@')) {
      return Response.json({ error: 'รูปแบบอีเมลไม่ถูกต้อง' }, { status: 400 })
    }

    if (body.age !== undefined && (typeof body.age !== 'number' || body.age < 0)) {
      return Response.json({ error: 'อายุไม่ถูกต้อง' }, { status: 400 })
    }

    const user = await db.createUser({
      email: body.email.trim(),
      name: body.name ?? '',
      age: body.age ?? 0,
    })

    return Response.json(user, { status: 201 })
  } catch (err) {
    return Response.json({ error: 'ข้อมูล JSON ไม่ถูกต้อง' }, { status: 400 })
  }
}
```

---

## Body Size Limits

| ประเภท               | ค่าเริ่มต้น               | Config Key             | ขีดจำกัดสูงสุด |
| -------------------- | ------------------------- | ---------------------- | -------------- |
| API routes (JSON)    | 10 MiB (10,485,760 bytes) | `security.apiLimit`    | 100 MiB        |
| Actions              | 1 MiB (1,048,576 bytes)   | `security.actionLimit` | 100 MiB        |
| Plugin response body | 32 MiB (33,554,432 bytes) | `security.pluginLimit` | 256 MiB        |

ปรับได้ใน `ruvyxa.config.ts`:

```tsx
import { config, type RuvyxaConfig } from 'ruvyxa/config'

const settings: RuvyxaConfig = {
  server: {},
  security: {
    apiLimit: 20 * 1024 * 1024, // 20 MiB
    actionLimit: 2 * 1024 * 1024, // 2 MiB
    pluginLimit: 64 * 1024 * 1024, // 64 MiB
  },
}

export default config(settings)
```

ถ้า body เกิน limit → `413 Payload Too Large`

---

## Streaming Responses

ใช้ `ReadableStream` สำหรับ streaming ข้อมูลยาว — logs, events, large datasets

### Streaming พื้นฐาน

```tsx
// app/api/stream/route.ts
export async function GET() {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder()

      controller.enqueue(encoder.encode('เริ่มต้น\n'))

      for (let i = 0; i < 5; i++) {
        await new Promise((r) => setTimeout(r, 1000))
        controller.enqueue(encoder.encode(`ข้อมูลชุดที่ ${i + 1}\n`))
      }

      controller.enqueue(encoder.encode('เสร็จสิ้น\n'))
      controller.close()
    },
  })

  return new Response(stream, {
    headers: {
      'Content-Type': 'text/plain',
      'Transfer-Encoding': 'chunked',
    },
  })
}
```

### SSE (Server-Sent Events)

```tsx
// app/api/events/route.ts
export async function GET() {
  let cleanup: (() => void) | null = null

  const stream = new ReadableStream({
    start(controller) {
      const encoder = new TextEncoder()

      // ส่ง event แรก
      controller.enqueue(encoder.encode('event: connected\ndata: {}\n\n'))

      const interval = setInterval(() => {
        const data = JSON.stringify({
          time: new Date().toISOString(),
          value: Math.random(),
        })
        controller.enqueue(encoder.encode(`event: update\ndata: ${data}\n\n`))
      }, 2000)

      // cleanup เมื่อ client disconnect
      cleanup = () => {
        clearInterval(interval)
        controller.close()
      }
    },
    cancel() {
      cleanup?.()
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

### Streaming Details (Under the Hood)

| Parameter    | ค่า                   | คำอธิบาย                                                    |
| ------------ | --------------------- | ----------------------------------------------------------- |
| Frame size   | 64 KiB                | แต่ละ chunk ที่ enqueue ควรไม่เกิน 64 KiB เพื่อ performance |
| Timeout      | 30s (configurable)    | worker timeout สำหรับ API routes                            |
| Backpressure | Native (backpressure) | ReadableStream รองรับ backpressure อัตโนมัติ                |
| Compression  | ขึ้นกับ proxy         | Ruvyxa ไม่บีบอัด stream เอง (ฝากให้ nginx/CDN)              |

**Body limits:** Streaming responses ไม่นับรวมใน body limit check — limit ตรวจสอบเฉพาะตอน receive
request body ไม่ใช่ตอน send response

**Binary Data:** สำหรับ streaming binary data, ใช้ `Uint8Array` โดยตรง:

```tsx
export async function GET() {
  const stream = new ReadableStream({
    start(controller) {
      const buffer = new Uint8Array([0x00, 0xff, 0xab])
      controller.enqueue(buffer)
      controller.close()
    },
  })

  return new Response(stream, {
    headers: { 'Content-Type': 'application/octet-stream' },
  })
}
```

---

## Binary Data and bodyBase64

เมื่อ worker pool ส่งข้อมูล binary ไปยัง worker, Ruvyxa ใช้ base64 encoding สำหรับ body:

```rust
struct WorkerRequest::Api {
    body: Option<Vec<u8>>,       // UTF-8 text
    body_base64: Option<String>,  // binary data → base64
    stream_response: bool,       // true = streaming
}
```

- `body` — สำหรับ JSON/text payloads
- `bodyBase64` — สำหรับ binary payloads (ไฟล์, images)
- `streamResponse` — flag ที่บอก worker ว่า response ควรเป็น streaming

วิธีรับ binary data ใน route handler:

```tsx
export async function POST({ request }: { request: Request }) {
  const blob = await request.blob() // binary data
  const buffer = await request.arrayBuffer() // หรือ ArrayBuffer

  return Response.json({ size: blob.size })
}
```

---

## Set-Cookie Multiple Values

Ruvyxa สนับสนุน multiple `Set-Cookie` headers โดยใช้ `header_pairs` (array of tuples) แทน `headers`
map:

```tsx
export async function GET() {
  const headers = new Headers()

  headers.append('Set-Cookie', 'token=abc123; HttpOnly; Path=/')
  headers.append('Set-Cookie', 'theme=dark; Path=/')
  headers.append('Content-Type', 'application/json')

  return Response.json({ ok: true }, { headers })
}
```

การ append `Set-Cookie` หลายครั้งด้วย `headers.append()` จะถูก serialize เป็น `header_pairs` ใน
worker request:

```json
{
  "headerPairs": [
    ["set-cookie", "token=abc123; HttpOnly; Path=/"],
    ["set-cookie", "theme=dark; Path=/"]
  ]
}
```

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

## ตัวอย่างเต็ม: CRUD API

```tsx
// app/api/products/route.ts
interface Product {
  id: string
  name: string
  price: number
  category: string
  inStock: boolean
}

const products: Product[] = [
  { id: '1', name: 'สินค้า A', price: 199, category: 'general', inStock: true },
  { id: '2', name: 'สินค้า B', price: 299, category: 'general', inStock: false },
]

// GET /api/products?category=general&page=1
export function GET({ request }: { request: Request }) {
  const url = new URL(request.url)
  const category = url.searchParams.get('category')
  const inStock = url.searchParams.get('inStock')
  const page = parseInt(url.searchParams.get('page') ?? '1', 10)
  const limit = parseInt(url.searchParams.get('limit') ?? '20', 10)

  let result = [...products]

  if (category) {
    result = result.filter((p) => p.category === category)
  }
  if (inStock !== null) {
    result = result.filter((p) => p.inStock === (inStock === 'true'))
  }

  const paginated = result.slice((page - 1) * limit, page * limit)

  return Response.json({
    data: paginated,
    pagination: {
      page,
      limit,
      total: result.length,
      totalPages: Math.ceil(result.length / limit),
    },
  })
}

// POST /api/products
export async function POST({ request }: { request: Request }) {
  try {
    const body = await request.json()

    if (!body.name || typeof body.name !== 'string') {
      return Response.json({ error: 'ต้องระบุชื่อสินค้า' }, { status: 400 })
    }

    const product: Product = {
      id: crypto.randomUUID(),
      name: body.name.trim(),
      price: Number(body.price) || 0,
      category: body.category ?? 'general',
      inStock: body.inStock !== false,
    }

    products.push(product)

    return Response.json(product, { status: 201 })
  } catch {
    return Response.json({ error: 'ข้อมูล JSON ไม่ถูกต้อง' }, { status: 400 })
  }
}
```

```tsx
// app/api/products/[id]/route.ts
interface Product {
  id: string
  name: string
  price: number
  category: string
  inStock: boolean
}

declare const products: Product[]

export async function GET({ params }: { params: { id: string } }) {
  const product = products.find((p) => p.id === params.id)

  if (!product) {
    return new Response('ไม่พบสินค้า', { status: 404 })
  }

  return Response.json(product)
}

export async function PATCH({ request, params }: { request: Request; params: { id: string } }) {
  const product = products.find((p) => p.id === params.id)
  if (!product) {
    return new Response('ไม่พบสินค้า', { status: 404 })
  }

  const body = await request.json()

  // อัปเดตเฉพาะ field ที่ส่งมา
  if (body.name !== undefined) product.name = String(body.name).trim()
  if (body.price !== undefined) product.price = Number(body.price)
  if (body.category !== undefined) product.category = String(body.category)
  if (body.inStock !== undefined) product.inStock = Boolean(body.inStock)

  return Response.json(product)
}

export async function DELETE({ params }: { params: { id: string } }) {
  const idx = products.findIndex((p) => p.id === params.id)
  if (idx === -1) {
    return new Response('ไม่พบสินค้า', { status: 404 })
  }

  products.splice(idx, 1)

  return new Response(null, { status: 204 }) // No Content
}
```

---

## Middleware + Security Headers

### CORS

```tsx
export async function GET({ request }: { request: Request }) {
  const origin = request.headers.get('Origin') ?? '*'

  // ถ้ามี OPTIONS request → export OPTIONS handler
  const headers: Record<string, string> = {
    'Access-Control-Allow-Origin': origin,
    'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, PATCH',
    'Access-Control-Allow-Headers': 'Content-Type, Authorization',
    'Access-Control-Max-Age': '86400',
  }

  return Response.json({ data: 'ok' }, { headers })
}

// OPTIONS handler สำหรับ CORS preflight
export async function OPTIONS({ request }: { request: Request }) {
  const origin = request.headers.get('Origin') ?? '*'

  return new Response(null, {
    status: 204,
    headers: {
      'Access-Control-Allow-Origin': origin,
      'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, PATCH',
      'Access-Control-Allow-Headers': 'Content-Type, Authorization',
      'Access-Control-Max-Age': '86400',
    },
  })
}
```

### Security Headers ใน Config

```tsx
// ruvyxa.config.ts
import { config, type RuvyxaConfig } from 'ruvyxa/config'

const settings: RuvyxaConfig = {
  server: {
    headers: {
      'X-Frame-Options': 'DENY',
      'X-Content-Type-Options': 'nosniff',
      'Referrer-Policy': 'strict-origin-when-cross-origin',
      'Permissions-Policy': 'camera=(), microphone=(), geolocation=()',
      'Strict-Transport-Security': 'max-age=31536000; includeSubDomains',
    },
  },
}

export default config(settings)
```

เมื่อตั้ง `security.headers: true` (default), Ruvyxa จะเพิ่ม security headers อัตโนมัติ:

| Header                   | ค่า                               |
| ------------------------ | --------------------------------- |
| `X-Frame-Options`        | `DENY`                            |
| `X-Content-Type-Options` | `nosniff`                         |
| `Referrer-Policy`        | `strict-origin-when-cross-origin` |

---

## Under the Hood: API Route Execution

```
Request → /api/products
    │
    └── router.find(path)
        │
        ├── ถ้าไม่เจอ route → 404
        │
        ├── ถ้า route เป็น page → ใช้ render API? (ไม่)
        │   (API routes ต้องเป็น route.ts ไม่ใช่ page.tsx)
        │
        ├── ถ้า route เป็น API → find route.ts
        │
        ├── worker_pool.render_api()
        │   ├── serialize request → WorkerRequest::Api
        │   │   { route_file, method, request_path,
        │   │     headers, header_pairs, body, body_base64,
        │   │     stream_response, params }
        │   ├── worker เลือก export function ตาม method
        │   ├── ถ้าไม่มี export → return 405
        │   ├── execute handler
        │   └── return WorkerApiResponse
        │
        └── render_api_pooled()
            ├── ถ้า !ok → RUV1200 diagnostic
            ├── streamed_body? → streaming body
            ├── headers → append to response
            └── security_headers → final response
```

### Worker Pool Timeouts

| Context    | Default Timeout     | Environment Variable       |
| ---------- | ------------------- | -------------------------- |
| Dev server | 30,000 ms (30s)     | `RUVYXA_WORKER_TIMEOUT_MS` |
| Build      | 300,000 ms (5 นาที) | `RUVYXA_WORKER_TIMEOUT_MS` |

Timeout ที่ 0 หรือ invalid จะถูกรีเซ็ตเป็นค่า default

**Concurrency ต่อ worker:** worker หนึ่งตัวรับ request พร้อมกันได้ไม่เกิน
`RUVYXA_WORKER_MAX_CONCURRENCY` (default: จำนวน core จำกัดที่ 2–8) การ render กิน CPU
และแต่ละครั้งถือ React tree, bundle ที่ compile แล้ว และ buffer ของ response ไว้ ถ้ารับทั้ง burst
เข้ามาพร้อมกันจะทำให้ heap หมดหรือ CPU thrash จนเกิด timeout ที่ดูเหมือน hang request
ที่เกินจะเข้าคิวและทยอยทำงานเมื่อมี slot ว่าง

---

## การทำงานร่วมกับ Routing

```
app/
  api/
    route.ts          ← API route ที่ /api
    users/
      route.ts        ← API route ที่ /api/users
      [id]/
        route.ts      ← API route ที่ /api/users/123
    blog/
      [slug]/
        route.ts      ← API route ที่ /api/blog/hello-world
  products/
    page.tsx          ← หน้า /products (UI)
    route.ts          ← API /products (data) — อยู่ร่วมกับ page.tsx ได้
```

route.ts อยู่ร่วมกับ page.tsx ได้ใน directory เดียวกัน — Ruvyxa จัดการให้โดยอัตโนมัติ

---

## Under the Hood: Route Kind Detection

Ruvyxa ตรวจสอบว่า route เป็น API route หรือ page route จาก:

- ไฟล์ชื่อ `route.ts` → API route
- ไฟล์ชื่อ `page.tsx` → page route
- ถ้ามีทั้งสอง → route รองรับทั้งสองแบบ (API + page)

เฉพาะ page routes เท่านั้นที่รองรับ server actions (RUV1501 ถ้าเรียก action ผ่าน route ที่ไม่มี
action.ts)

---

## Best Practices

1. **ใช้ HTTP methods ให้ถูก**: GET=read, POST=create, PUT=replace, PATCH=update, DELETE=delete
2. **validate input ทุกครั้ง**: return 400 + error message ที่ชัดเจน
3. **ใช้ status codes ให้ถูก**: 200=ok, 201=created, 204=no content, 400=bad request, 404=not found,
   413=too large, 429=rate limit, 500=server error
4. **streaming สำหรับข้อมูลยาว**: logs, events, large datasets
5. **ตั้ง security headers**: ใน config หรือ per-response
6. **แยก API routes ด้วย dynamic segments**: แทนการใช้ query params สำหรับ hierarchical data
7. **ใช้ OPTIONS handler**: สำหรับ CORS preflight เมื่อรองรับ cross-origin
8. **จำกัด body size**: ตั้ง `security.apiLimit` ให้เหมาะสมกับ use case

---

## ข้อผิดพลาดทั่วไป

| ปัญหา                      | สาเหตุ                               | วิธีแก้                                      |
| -------------------------- | ------------------------------------ | -------------------------------------------- |
| 404 API route              | ไฟล์ชื่อ `route.ts` หรือ path ไม่ตรง | ตรวจสอบ spelling และ path                    |
| 405 Method Not Allowed     | method ไม่ได้ export                 | เพิ่ม export function สำหรับ method นั้น     |
| 413 Request Too Large      | body เกิน `security.apiLimit`        | เพิ่ม `apiLimit` ใน config                   |
| 400 Bad Request            | JSON parse error                     | เช็ค JSON format ใน request body             |
| CORS error                 | cross-origin request ไม่มี headers   | ตั้ง CORS headers หรือ export OPTIONS        |
| streaming ไม่มา            | ลืม `controller.close()`             | ปิด stream เมื่อเสร็จ                        |
| streaming ตัดกลางคัน       | worker timeout                       | เพิ่ม RUVYXA_WORKER_TIMEOUT_MS               |
| 500 Internal Server Error  | handler throw error                  | ใช้ try-catch, return status code ที่เหมาะสม |
| `RUV1200` API route failed | route execution error                | ดู diagnostic message ใน dev overlay         |

---

## สิ่งที่ API Route รับและคืนจริง

API entry คือ `route.ts` หรือ `route.js` runtime จะหา named export ที่ตรงกับ HTTP method
ตัวพิมพ์ใหญ่ แล้วเรียกด้วย object เดียวที่มี `request` และ `params` การคืน `Response` จะเก็บ
status/headers ไว้ ส่วน การคืนค่าแบบอื่นจะถูกแปลงเป็น JSON

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

หากไม่มี named method ที่ตรงกัน runtime ปัจจุบันคืน `405 Method <METHOD> is not allowed` ซึ่งต่างจาก
route หายที่เป็นปัญหาของ routing ใช้ method exports แทน request class เฉพาะ framework เพราะ handler
ได้รับ Web `Request` มาตรฐานและคืน Web `Response` มาตรฐานได้

### Body Limit ถูกตรวจก่อนเข้า Handler

สำหรับ methods ที่ส่ง body ได้ dev server จะอ่าน body ภายใต้ `security.apiLimit` ก่อน dispatch API
module request ที่เกิน limit ได้รับ payload-too-large response โดย handler ไม่ทำงาน แต่ยังต้อง parse
และ validate ข้อมูลของแอปใน handler:

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

การบังคับ body size จำกัดขนาด transport เท่านั้น ไม่ได้ validate JSON shape, authorization หรือ
ownership ให้เก็บการตัดสินใจเหล่านี้ใน handler หรือ server-only service ที่ handler เรียก

### ตรวจ Route ก่อน Debug Endpoint

เมื่อ endpoint ตอบกลับไม่ตรงคาด ให้ยืนยันก่อนว่า route ใดถูกค้นพบ แล้วจึงแก้ handler:

```bash
ruvyxa routes
ruvyxa trace /api/products/[id]
ruvyxa analyze --format human
```

route manifest แยก page และ API entries และ analyzer มอง API import graph เป็น server graph
จึงควรเก็บ browser-only imports ออกจาก `route.ts`; server graph ที่ import `client-only` จะได้รับ
`RUV1009`

---

## ขั้นตอนถัดไป

- **[08-styling.md](./08-styling.md)** — CSS, SCSS, CSS Modules
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — การโหลดข้อมูลและ Cache
