# Server Actions

Server actions คือฟังก์ชันที่รันบนเซิร์ฟเวอร์แต่เรียกจาก client ได้เหมือนเรียกฟังก์ชันปกติ —
เหมาะกับ form submission, database mutations, cache invalidation, และ real-time event publishing

---

## Server Action คืออะไร?

```
Client (browser)
    │
    │  POST /__ruvyxa/action?path=/todos&name=createTodo
    │  Content-Type: application/json
    │  { "title": "ซื้อของ" }
    ▼
Server (Ruvyxa)
    │
    ├── 1. ตรวจสอบ content type (JSON / urlencoded เท่านั้น)
    ├── 2. ตรวจสอบ body size (default 1 MiB)
    ├── 3. ตรวจสอบ same-origin / fetch metadata
    ├── 4. rate limit check (default 600/60s per client-action)
    ├── 5. resolve route + action file
    ├── 6. เรียก input.parse(value) — validate
    ├── 7. execute handler
    ├── 8. invalidate cache
    ├── 9. publish realtime event (ถ้ามี)
    ├── 10. return result → JSON response
    ▼
Client ได้ผลลัพธ์
```

Ruvyxa สร้าง form action URL ในรูปแบบ: `/__ruvyxa/action?path=/todos&name=createTodo`

- `path` — path ของ route ที่ action อยู่ (required)
- `name` — ชื่อ export ของ action function (required)

---

## Type Signature

```tsx
// ---- Action Schema ----
interface Schema<TInput> {
  /** รับ unknown, ต้อง throw ถ้า invalid, คืนค่าที่ validate แล้ว */
  parse(value: unknown): TInput
}

// ---- Action Builder ----
interface ActionBuilder<TInput = unknown> {
  /** เพิ่ม input validation schema */
  input<TNextInput>(schema: Schema<TNextInput>): ActionBuilder<TNextInput>
  /** เพิ่ม realtime event publishing (ไม่เรียก = ไม่ publish) */
  realtime(channels?: string | readonly string[]): ActionBuilder<TInput>
  /** กำหนด handler function */
  handler<TResult>(
    handler: (ctx: ActionContext<TInput>) => TResult | Promise<TResult>,
  ): ServerAction<TInput, TResult>
}

// ---- Action Context ----
interface ActionContext<TInput> {
  /** input ที่ผ่าน parse() แล้ว — type-safe */
  input: TInput
  /** Request object ดั้งเดิมจาก HTTP request */
  request: Request
  /** ข้อมูล user (optional, ขึ้นกับ middleware) */
  user?: unknown
  /** ฟังก์ชัน invalidate cache (prefix-based) */
  invalidate(key: string): void
}

// ---- Server Action Object ----
interface ServerAction<TInput, TResult> {
  (input: TInput, ctx?: Partial<ActionContext<TInput>>): Promise<TResult>
  ruvyxa: {
    kind: 'action'
    realtime?: ActionRealtimeOptions // มีก็ต่อเมื่อเรียก .realtime()
  }
}

interface ActionRealtimeOptions {
  /** ช่องทางที่ publish event ถ้าไม่ระบุ = route:<pathname> */
  channels: readonly string[]
}
```

---

## สร้าง Action แรก

ไฟล์ `app/todos/action.ts`:

```tsx
import { action } from 'ruvyxa/server'

interface TodoInput {
  title: string
}

export const createTodo = action
  .input({
    // parse() รับ unknown → ตรวจสอบ → คืน type ที่ต้องการ
    parse(value: unknown): TodoInput {
      if (!value || typeof value !== 'object' || !('title' in value)) {
        throw new Error('ต้องระบุชื่อ todo')
      }
      const title = String(value.title).trim()
      if (!title) throw new Error('ชื่อ todo ต้องไม่เว้นว่าง')
      return { title }
    },
  })
  .handler(async ({ input, invalidate }) => {
    const todo = {
      id: crypto.randomUUID(),
      title: input.title,
      completed: false,
      createdAt: new Date().toISOString(),
    }

    await db.insert('todos', todo)

    // ลบ cache ที่เกี่ยวข้อง
    invalidate('todos')

    return todo
  })
```

---

## การเรียก Action

### 1. HTML Form (แบบไม่มี JavaScript)

```tsx
// app/todos/page.tsx
export default function TodosPage() {
  return (
    <main>
      <h1>รายการสิ่งที่ต้องทำ</h1>

      <form method="post" action="/__ruvyxa/action?path=/todos&name=createTodo">
        <label>
          ชื่อ
          <input name="title" defaultValue="ซื้อของ" />
        </label>
        <button type="submit">เพิ่ม</button>
      </form>
    </main>
  )
}
```

เมื่อกด submit:

1. Browser ส่ง `POST` ไป `/__ruvyxa/action?path=/todos&name=createTodo`
2. Content-Type เป็น `application/x-www-form-urlencoded` โดยอัตโนมัติ
3. ข้อมูลจาก form fields ถูกแปลงเป็น `{ "title": "ซื้อของ" }`
4. Ruvyxa เรียก `createTodo` action
5. ถ้าสำเร็จ → browser redirect (ตาม default behavior)
6. ถ้าล้มเหลว → แสดง error

### 2. fetch API (JavaScript)

```tsx
'use client'

export default function TodosClient() {
  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()

    const res = await fetch('/__ruvyxa/action?path=/todos&name=createTodo', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title: 'ซื้อของ' }),
    })

    if (!res.ok) {
      const text = await res.text()
      throw new Error(text)
    }

    const data = await res.json()
    console.log('created:', data)
  }

  return (
    <form onSubmit={handleSubmit}>
      <input name="title" placeholder="ชื่อ todo..." />
      <button type="submit">เพิ่ม</button>
    </form>
  )
}
```

---

## Action Input Validation

### Basic Validation

```tsx
export const createUser = action
  .input({
    parse(value: unknown) {
      if (!value || typeof value !== 'object') {
        throw new Error('ข้อมูลไม่ถูกต้อง')
      }

      const obj = value as Record<string, unknown>

      if (!obj.email || typeof obj.email !== 'string') {
        throw new Error('ต้องระบุอีเมล')
      }

      const email = obj.email.trim()
      if (!email.includes('@')) {
        throw new Error('รูปแบบอีเมลไม่ถูกต้อง')
      }

      const name = obj.name ? String(obj.name).trim() : ''

      return { email, name }
    },
  })
  .handler(async ({ input }) => {
    const user = await db.insert('users', {
      email: input.email,
      name: input.name,
    })
    return user
  })
```

### Validation หลายฟิลด์ (รวม error ทั้งหมด)

```tsx
export const register = action
  .input({
    parse(value: unknown) {
      if (!value || typeof value !== 'object') throw new Error('invalid')

      const { username, password, age } = value as Record<string, unknown>

      const errors: string[] = []

      if (!username || String(username).length < 3) {
        errors.push('ชื่อผู้ใช้ต้องมีอย่างน้อย 3 ตัวอักษร')
      }
      if (!password || String(password).length < 8) {
        errors.push('รหัสผ่านต้องมีอย่างน้อย 8 ตัวอักษร')
      }
      if (typeof age !== 'number' || age < 18) {
        errors.push('ต้องมีอายุ 18 ปีขึ้นไป')
      }

      if (errors.length > 0) {
        throw new Error(errors.join('\n'))
      }

      return {
        username: String(username).trim(),
        password: String(password),
        age,
      }
    },
  })
  .handler(async ({ input }) => {
    return await db.createUser(input)
  })
```

**หมายเหตุ:** `parse()` รับ `unknown` — ทุกค่าที่ส่งจาก client เป็น `unknown` เสมอในตอนเริ่ม
ต้องตรวจสอบ type ทุกครั้ง

---

## Content-Type Handling

Ruvyxa action รองรับ Content-Type เพียง 2 ชนิด:

| Content-Type                        | วิธีส่ง         | ตัวอย่าง             |
| ----------------------------------- | --------------- | -------------------- |
| `application/json`                  | `fetch` / axios | `{ "title": "..." }` |
| `application/x-www-form-urlencoded` | HTML form       | `title=...`          |

**Content-Type อื่นที่ไม่ใช่ 2 ชนิดนี้จะถูก reject ทันที (415 Unsupported Media Type)**

```tsx
// 🔴 ไม่รองรับ
// Content-Type: multipart/form-data
// Content-Type: text/plain
// Content-Type: application/xml

// ✅ รองรับ
;<form method="post" action="/__ruvyxa/action?...">
  <input name="title" />
  <input name="quantity" type="number" />
  <button>ส่ง</button>
</form>
// → parse ได้ { title: "...", quantity: "1" }

fetch('/__ruvyxa/action?path=/products&name=create', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ title: 'สินค้า', price: 299 }),
})
// → parse ได้ { title: "สินค้า", price: 299 }
```

**การแปลง form data → JSON:**

- `application/x-www-form-urlencoded`: `name=value&name2=value2` →
  `{ name: "value", name2: "value2" }`
- ค่าที่เป็นตัวเลขจะยังคงเป็น string (ต้องแปลงใน `parse()`)
- fields ที่ไม่ระบุค่า → `""`

**Payload ที่ว่างเปล่า:**

- ถ้า Content-Type เป็น `application/json` และ body ว่าง, Ruvyxa จะแปลงเป็น `{}` อัตโนมัติ
- ถ้า Content-Type เป็น `urlencoded` และ body ว่าง → `{ }`

---

## Security Defaults

Ruvyxa มีระบบรักษาความปลอดภัยหลายชั้นสำหรับ action:

| มาตรการ               | ค่าเริ่มต้น             | รายละเอียด                                           |
| --------------------- | ----------------------- | ---------------------------------------------------- |
| **Body size limit**   | 1,048,576 bytes (1 MiB) | request body เกิน → 413 Payload Too Large            |
| **Same-origin**       | `true`                  | ตรวจสอบ `Origin` header ตรงกับ `Host`                |
| **Fetch Metadata**    | `true`                  | ตรวจสอบ `Sec-Fetch-Site` header                      |
| **Rate limit**        | 600 requests / 60s      | sliding-window counter ต่อ client+action, เกิน → 429 |
| **Method**            | POST เท่านั้น           | actions ไม่รับ GET/DELETE/etc.                       |
| **Content-Type**      | JSON / urlencoded       | 415 สำหรับ type อื่น                                 |
| **Trusted proxy IPs** | loopback เท่านั้น       | X-Forwarded-For เชื่อถือได้จาก localhost             |

### การตั้งค่าใน ruvyxa.config.ts

```tsx
// ruvyxa.config.ts
import { config, type RuvyxaConfig } from 'ruvyxa/config'

const settings: RuvyxaConfig = {
  server: {
    port: 3000,
  },
  security: {
    // ขนาด body สูงสุดของ action (bytes)
    actionLimit: 2_097_152, // 2 MiB

    // Rate limit: client+action, 1000 requests ต่อ 30 วินาที
    actionRateLimit: {
      max: 1000,
      window: 30, // วินาที
    },

    // ปิดการตรวจสอบ same-origin (ไม่แนะนำ)
    sameOrigin: false,

    // ปิดการตรวจสอบ fetch metadata (ไม่แนะนำ)
    fetchMeta: false,

    // reverse proxy ที่เชื่อถือได้ — ระบุเป็น address ตรงตัว หรือ CIDR range ก็ได้
    trustedProxyIps: ['10.0.0.1', '172.16.0.0/12', '2001:db8::/32'],
  },
}

export default config(settings)
```

### Under the Hood: Request Validation Flow

```
Request → /__ruvyxa/action
    │
    ├── validate_action_request()
    │   ├── bodyLen > actionBodyLimit?     → 413
    │   ├── content-type supported?        → 415
    │   ├── same-origin check?             → 403
    │   └── fetch-metadata check?          → 403
    │
    ├── validate_action_payload()
    │   ├── UTF-8 valid?                   → 400
    │   ├── JSON syntax (ถ้า json)?        → 400
    │   └── return (content_type, payload)
    │
    ├── action_rate_limit_key()
    │   └── format: "{clientIP}:{path}:{name}"
    │
    ├── rate_limiter.allow(key)?
    │   ├── false → 429 + Retry-After
    │   └── true → continue
    │
    └── render_server_action_pooled()
        ├── find route by path             → 404
        ├── check route kind == Page       → 405
        ├── find action.ts                 → RUV1501
        ├── worker_pool.render_action()
        └── return Response
```

### Under the Hood: Same-Origin Validation

```rust
fn action_origin_is_cross_site(headers, config, peer) -> bool {
    let origin = headers.get("Origin");
    let host = headers.get("Host");

    // ถ้าไม่มี Origin → ใช้ Sec-Fetch-Site แทน
    // ถ้าไม่มีทั้งสอง → fail closed (block)

    // การเทียบ host คือด่านที่หยุด CSRF ได้จริง เพราะ browser เป็นผู้ตั้ง
    // Origin เอง หน้าเว็บจาก origin อื่นจึงปลอม host ให้ตรงไม่ได้
    if origin_host != host {
        return true;
    }

    // scheme จะถูกเทียบเฉพาะเมื่อมีแหล่งที่เชื่อถือได้ระบุมา — Ruvyxa
    // ไม่ terminate TLS เอง หลักฐานเดียวของ scheme ฝั่ง browser คือ
    // X-Forwarded-Proto จาก peer ที่เชื่อถือได้ (loopback หรือรายการใน
    // security.trustedProxyIps) ถ้าไม่มีหลักฐาน จะถือว่า scheme ไม่ทราบค่า
    // และไม่นำมาตัดสิน
    if peer_is_trusted && forwarded_proto in ("http", "https") {
        return origin_scheme != forwarded_proto;
    }

    return false;
}
```

> รุ่นก่อนหน้าจะสมมติว่าเป็น `http` เสมอเมื่อไม่มี trusted proxy รายงาน scheme มา ผลคือ deployment
> ใดที่ proxy ผู้ terminate TLS ไม่ใช่ loopback และไม่ได้อยู่ใน `security.trustedProxyIps` —
> ซึ่งเป็นรูปแบบปกติของ Docker Compose, Kubernetes และ edge ของ managed platform — จะถูกตอบ
> `403 Cross-origin action request blocked` ทุก action ยังคงแนะนำให้ตั้ง `trustedProxyIps`
> เพราะเป็นตัวเปิด การอ่าน client IP จาก header สำหรับ rate limiter และทำให้การเทียบ scheme
> แบบเข้มงวดกลับมาทำงาน

### Under the Hood: Rate Limiter Algorithm

Rate limiter ใช้ sliding window counter โดย hash แต่ละ key ลงใน slot ที่มีจำนวนคงที่
หน่วยความจำที่ใช้จึงไม่ขึ้นกับจำนวน client ที่เคยเข้ามา:

```rust
const ACTION_RATE_LIMIT_SLOTS: usize = 8192;

struct RateSlot {
    window_start: Instant,
    current: u32,   // จำนวน request ใน window ที่เริ่มที่ window_start
    previous: u32,  // จำนวน request ใน window ก่อนหน้า
}

struct ActionRateLimiter {
    slots: Vec<Option<RateSlot>>,  // ACTION_RATE_LIMIT_SLOTS ช่อง
    hasher: RandomState,           // seed ใหม่ทุก process
    max_hits: usize,               // default 600
    window: Duration,              // default 60s
}

fn allow(key: &str) -> bool {
    let slot = slots[hash(key) % ACTION_RATE_LIMIT_SLOTS];

    // 1. เลื่อน window ไปข้างหน้าให้ window_start อยู่ห่างจาก now ไม่เกินหนึ่ง window
    // 2. ประมาณค่าแบบ sliding: ถ่วงน้ำหนัก window ก่อนหน้าด้วยสัดส่วนที่ยังอยู่ในช่วงย้อนหลัง
    let overlap = 1.0 - (now - slot.window_start) / window;
    let estimated = slot.previous * overlap + slot.current;

    // 3. ถ้า estimated >= max_hits → deny
    // 4. slot.current += 1 → allow
}
```

สองคุณสมบัติที่สำคัญ:

- **client จะไม่ถูกปฏิเสธเพราะโควตาของ client อื่น** ระบบไม่เคยปฏิเสธเพราะ "ไม่มีที่ว่าง"
  รุ่นก่อนหน้าเก็บ key map ที่จำกัดไว้ 10,000 รายการ และปฏิเสธ key ที่รับเพิ่มไม่ได้
  ผู้โจมตีที่หมุนเวียน source address — ทำได้ง่ายมากด้วย IPv6 `/64` — จึงถมเต็ม map แล้วล็อก client
  ที่เข้ามาครั้งแรกออกทั้งหมดจนจบ window
- **การชน slot ทำให้ถูกจำกัดเร็วขึ้นได้ แต่ไม่เคยได้โควตาเพิ่ม** สอง client ที่ชน slot
  กันจะใช้โควตาร่วมกัน และเพราะ hasher ถูก seed ใหม่ทุก process จึงสร้าง key
  ให้ชนกับเป้าหมายที่เจาะจงไม่ได้

**Key format:** `"{clientIP}:{path}:{name}"` — ทำให้ rate limit แยกกันสำหรับแต่ละ client และ action

**Forwarded Client IP:** ถ้า peer IP เป็น trusted proxy, ระบบจะตรวจสอบ `X-Forwarded-For` header
(จากขวาไปซ้าย) เพื่อหา IP จริงของ client โดยข้าม proxy IPs ที่ trust แล้ว

**Trusted proxy:**

- loopback (`127.0.0.1`, `::1`) เชื่อถือได้เสมอ ไม่ต้องระบุใน config
- เพิ่ม address ตรงตัวหรือ CIDR range ได้ผ่าน `trustedProxyIps`
- range แบบ IPv4 จะ match peer แบบ IPv4-mapped (`::ffff:10.0.0.9`) ด้วย ซึ่งเป็นรูปแบบที่ dual-stack
  listener รายงาน client IPv4
- ถ้าไม่ตั้ง trusted proxy ไว้เลย จะใช้ peer IP ตรงๆ เป็น key ของ rate limiter

---

## Real-time Actions

Action สามารถ publish realtime events ผ่าน WebSocket/SSE ได้ทันทีหลัง execution สำเร็จ

### การใช้งาน

```tsx
export const sendMessage = action
  .input({
    parse(value: unknown) {
      if (!value || typeof value !== 'object') throw new Error('invalid')
      return { text: String((value as any).text ?? '').trim() }
    },
  })
  .realtime('chat:messages') // publish event ไปยัง channel 'chat:messages'
  .handler(async ({ input }) => {
    const msg = {
      id: crypto.randomUUID(),
      text: input.text,
      timestamp: new Date().toISOString(),
    }
    await db.insert('messages', msg)
    return msg
  })
```

### Channel Name Rules

- ความยาว: 1-128 ตัวอักษร
- ตัวอักษรที่ใช้ได้: `[A-Za-z0-9:._/-]`
- สูงสุด 16 channels ต่อ action
- ถ้าไม่ระบุ channel name — จะใช้ `route:<pathname>` อัตโนมัติ

```tsx
// .realtime() ไม่มี argument → ใช้ route path
.realtime()              // → channels: ["route:/todos"]

// .realtime() หลาย channels
.realtime(['chat:room:1', 'chat:room:2', 'notifications'])
```

### Real-time Event Format

Event ที่ publish จะถูกเข้ารหัส base64url และส่งผ่าน header `x-ruvyxa-realtime-event`:

```
x-ruvyxa-realtime-event: <base64url(json)>
```

ขนาด event metadata สูงสุด 24 KiB (RUV1500 ถ้าเกิน)

ข้อความที่ client จะได้รับประกอบด้วย:

- path, name ของ action ที่ trigger
- timestamp
- result data จาก handler

---

## Cache Invalidation

เมื่อ action สำเร็จ ควร invalidate cache ที่เกี่ยวข้องเสมอ:

```tsx
export const updateProduct = action
  .input({
    parse(value: unknown) {
      const obj = value as Record<string, unknown>
      if (!obj.id) throw new Error('ต้องระบุ id')
      return {
        id: String(obj.id),
        name: String(obj.name ?? '').trim(),
        price: Number(obj.price) || 0,
      }
    },
  })
  .handler(async ({ input, invalidate }) => {
    await db.update('products', input.id, {
      name: input.name,
      price: input.price,
    })

    // ลบ cache ต่างๆ ที่เกี่ยวข้อง
    invalidate('products') // ลบ products ทั้งหมด
    invalidate(`products:${input.id}`) // ลบ product นี้โดยเฉพาะ
    invalidate('categories') // ลบ categories cache

    return { success: true }
  })
```

`invalidate(key)` ทำงานเหมือน `invalidateCache(key)` — ลบ key ที่ตรงทั้งหมดหรือขึ้นต้นด้วย prefix

---

## ตัวอย่างเต็ม: Todo App

```tsx
// app/todos/action.ts
import { action } from 'ruvyxa/server'

interface Todo {
  id: string
  title: string
  completed: boolean
}

// CREATE
export const createTodo = action
  .input({
    parse(value: unknown) {
      if (!value || typeof value !== 'object') throw new Error('invalid')
      const title = String((value as any).title ?? '').trim()
      if (!title) throw new Error('ต้องระบุชื่อ')
      return { title }
    },
  })
  .handler(async ({ input, invalidate }) => {
    const todo: Todo = {
      id: crypto.randomUUID(),
      title: input.title,
      completed: false,
    }
    await db.insert('todos', todo)
    invalidate('todos')
    return todo
  })

// DELETE
export const deleteTodo = action
  .input({
    parse(value: unknown) {
      if (!value || typeof value !== 'object') throw new Error('invalid')
      return { id: String((value as any).id) }
    },
  })
  .handler(async ({ input, invalidate }) => {
    await db.delete('todos', input.id)
    invalidate('todos')
    return { success: true }
  })

// TOGGLE
export const toggleTodo = action
  .input({
    parse(value: unknown) {
      if (!value || typeof value !== 'object') throw new Error('invalid')
      return {
        id: String((value as any).id),
        completed: Boolean((value as any).completed),
      }
    },
  })
  .handler(async ({ input, invalidate }) => {
    await db.update('todos', input.id, { completed: input.completed })
    invalidate('todos')
    return { success: true }
  })
```

### หน้า UI พร้อม Error Handling

```tsx
'use client'

import { useState } from 'react'

export default function TodosClient() {
  const [title, setTitle] = useState('')
  const [result, setResult] = useState<Todo | null>(null)
  const [error, setError] = useState('')

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError('')
    setResult(null)

    try {
      const res = await fetch('/__ruvyxa/action?path=/todos&name=createTodo', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ title }),
      })

      if (!res.ok) {
        const text = await res.text()
        throw new Error(text)
      }

      const data = await res.json()
      setResult(data)
      setTitle('')
    } catch (err: any) {
      setError(err.message)
    }
  }

  return (
    <div>
      <form onSubmit={handleSubmit}>
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="ชื่อ todo..."
        />
        <button type="submit">เพิ่ม</button>
      </form>

      {error && <p style={{ color: 'red' }}>{error}</p>}
      {result && <pre>{JSON.stringify(result, null, 2)}</pre>}
    </div>
  )
}
```

---

## Under the Hood: Action Response Format

Response ที่ return จาก worker pool ไปยัง client:

```
HTTP/1.1 200 OK
Content-Type: application/json
X-Ruvyxa-Realtime-Event: <base64url event> (ถ้ามี)

{"id":"abc...","title":"ซื้อของ","completed":false}
```

**Error responses:**

| Status                     | เหตุผล                                 |
| -------------------------- | -------------------------------------- |
| 200 OK                     | สำเร็จ                                 |
| 400 Bad Request            | JSON parse error หรือ validation error |
| 403 Forbidden              | Cross-origin หรือ cross-site           |
| 404 Not Found              | Route หรือ action file ไม่พบ           |
| 405 Method Not Allowed     | Route ไม่ใช่ page                      |
| 413 Payload Too Large      | Body เกิน actionLimit                  |
| 415 Unsupported Media Type | Content-Type ไม่รองรับ                 |
| 429 Too Many Requests      | Rate limit เกิน                        |
| 500 Internal Server Error  | Handler error                          |

### Action File Resolution

Ruvyxa ค้นหาไฟล์ action จาก route directory:

```rust
pub(crate) fn action_file_for(route: &RouteEntry) -> Option<PathBuf> {
    let route_dir = route.file.parent()?;
    ["action.ts", "action.js"]
        .into_iter()
        .map(|name| route_dir.join(name))
        .find(|path| path.is_file())
}
```

ลำดับการค้นหา:

1. `app/todos/action.ts`
2. `app/todos/action.js`
3. ถ้าไม่พบ → RUV1501

---

## Under the Hood: ServerConfig for Actions

ฟิลด์สำคัญใน `ServerConfig` ที่ควบคุมพฤติกรรมของ action:

| ฟิลด์                      | Type           | ค่าเริ่มต้น       | ขีดจำกัดสูงสุด |
| -------------------------- | -------------- | ----------------- | -------------- |
| `action_body_limit_bytes`  | usize          | 1,048,576 (1 MiB) | 100 MiB        |
| `action_rate_limit_max`    | usize          | 600               | 100,000        |
| `action_rate_limit_window` | Duration       | 60s               | 3,600s         |
| `same_origin_actions`      | bool           | true              | —              |
| `fetch_metadata_actions`   | bool           | true              | —              |
| `trusted_proxies`          | TrustedProxies | ว่าง              | —              |

---

## ข้อผิดพลาดทั่วไป

| ปัญหา                       | สาเหตุ                              | วิธีแก้                                         |
| --------------------------- | ----------------------------------- | ----------------------------------------------- |
| Action 404                  | path/name ไม่ตรง                    | เช็ค action URL: `?path=/todos&name=createTodo` |
| 413 Payload Too Large       | body เกิน 1MB                       | เพิ่ม `security.actionLimit` ใน config          |
| 415 Unsupported Media Type  | Content-Type ไม่ใช่ JSON/urlencoded | เปลี่ยนเป็น `application/json`                  |
| 429 Too Many Requests       | rate limit เกิน                     | รอหรือเพิ่ม `security.actionRateLimit.max`      |
| 403 Forbidden (same-origin) | Origin ผิด                          | ใช้ same-origin หรือตั้ง CORS                   |
| 403 Forbidden (fetch-meta)  | Cross-site request                  | ใช้ same-site context                           |
| validation error            | parse() throw                       | ตรวจสอบ input format                            |
| invalidate ไม่ทำงาน         | key ไม่ตรงกับ cache                 | เช็ค cache key prefix                           |
| realtime event ไม่มา        | .realtime() ไม่ได้เรียก             | เพิ่ม `.realtime('channel')` ก่อน `.handler()`  |
| realtime event เกินขนาด     | metadata > 24 KiB                   | ลดขนาดข้อมูลที่ส่ง                              |
| Action error 500            | handler throw                       | ตรวจสอบ error message จาก response              |

### RUV Error Codes

| Code        | ความหมาย                        | คำอธิบาย                               |
| ----------- | ------------------------------- | -------------------------------------- |
| **RUV1500** | Server action execution failed  | Handler throw error (generic)          |
| **RUV1501** | Route action file was not found | ไม่มี `action.ts`/`action.js` ใน route |
| **RUV1502** | (reserved)                      | —                                      |
| **RUV1503** | (reserved)                      | —                                      |
| **RUV1601** | Config validation error         | security.actionLimit/etc. = 0          |
| **RUV1602** | Config limit exceeded           | ค่าเกิน max ที่อนุญาต                  |

---

## Best Practices

1. **validate ทุก input** — `parse()` รับ `unknown`, return type ที่ต้องการ, throw เมื่อ invalid
2. **invalidate cache ที่เกี่ยวข้อง** — หลัง mutation ทุกครั้ง ใช้ prefix matching
3. **ใช้ `crypto.randomUUID()` สำหรับ id** — ไม่ต้องพึ่ง database auto-increment
4. **อย่าเก็บ secret ใน input** — action code เป็น server-side ไม่ leak ไป client
5. **limit body size** — ป้องกัน request ใหญ่เกิน (ปรับได้ที่ `security.actionLimit`)
6. **ใช้ same-origin + fetchMeta** — ป้องกัน CSRF โดยค่าเริ่มต้น
7. **ตั้ง trusted proxy IPs** — ถ้าใช้ reverse proxy (nginx, Cloudflare)
8. **ใช้ realtime สำหรับ event-driven features** — chat, notification, live update

---

## Contract ของ Action Builder

Server-action API ที่รองรับคือ builder จาก `@ruvyxa/core/server` ซึ่งแยกการตัดสินใจเป็น 3 ส่วน:
validation ของ input (เลือกใช้ได้), การ publish realtime (เลือกใช้ได้) และ server handler schema
ต้องมีเพียงเมทอด `parse(value)` จึงไม่บังคับให้ใช้ validation library ใด library หนึ่ง

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

handler context มี `input` ที่ validate แล้ว, `request` ที่เข้ามา, `user` ที่ runtime integration
อาจ ใส่ให้ และ `invalidate(key)` แต่ไม่ได้สร้าง authentication หรือ persistent storage ให้อัตโนมัติ
แอปยัง ต้อง authenticate request และเขียนข้อมูลลง data store ของตนเอง

### Realtime Channels มีขอบเขตชัดเจน

`.realtime()` รับ channel เดียวหรือรายการ channels ระบบ trim, ตัดค่าซ้ำ, จำกัดไม่เกิน 16 ค่า และชื่อ
ต้องยาว 1–128 ตัว ใช้ได้เฉพาะ letters, digits, `:`, `.`, `_`, `/` หรือ `-` การไม่ส่ง argument จะใช้
route channel ให้มอง realtime event เป็นสัญญาณให้ refresh/invalidate state ไม่ใช่หลักฐานว่า
subscriber ทุกคนได้รับ database change ที่ durable แล้ว

### ความปลอดภัยของ Request เกิดก่อน Handler

action endpoint ตรวจ request และ rate limiter ที่ตั้งค่าไว้ก่อนส่งงานให้ worker รัน action ดังนั้น
`security.actionLimit`, `security.actionRateLimit`, same-origin checks, Fetch Metadata checks และ
trusted-proxy settings ต้องอยู่ในการทบทวน deployment/security; schema `parse` ไม่ได้แทนการป้องกันนี้

ใช้ `ruvyxa analyze --format human` ตรวจว่า dependencies ของ action ไม่ไปอยู่ใน client graph แล้วใช้
`npm run check` เป็น project-level type/parity gate ไม่ควรอธิบายหรือเรียก `'use server'` ว่าเป็นกลไก
register action: route-local `action.ts`/`action.js` และ exported action values คือ convention
ปัจจุบัน

---

## ขั้นตอนถัดไป

- **[07-api-routes.md](./07-api-routes.md)** — API routes สำหรับ REST endpoints
- **[08-styling.md](./08-styling.md)** — CSS, SCSS, CSS Modules
