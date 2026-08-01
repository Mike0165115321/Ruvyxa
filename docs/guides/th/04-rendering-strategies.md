# กลยุทธ์การ Render

Ruvyxa รองรับ 5 กลยุทธ์การ render แบบอัตโนมัติ: SSR, SSG, ISR, PPR, CSR

คุณไม่ต้อง config อะไรมาก — framework ตรวจสอบ source code ของคุณ
และเลือกกลยุทธ์ที่เหมาะสมให้อัตโนมัติ

---

## Decision Tree

```
หน้าเข้ามา
    │
    ▼
มี 'use client' โดยไม่มี server component โหลดข้อมูล?
    │         │
    ใช่       ไม่ใช่
    │         │
    ▼         ▼
   CSR      มี export const ppr = true?
                │         │
                ใช่       ไม่ใช่
                │         │
                ▼         ▼
               PPR      มี export const revalidate = N?
                           │         │
                           ใช่       ไม่ใช่
                           │         │
                           ▼         ▼
                          ISR      มี getStaticParams หรือ staticParams?
                           │         │         │
                           │         ใช่       ไม่ใช่
                           │         │         │
                           │         ▼         ▼
                           │      SSG          route มี dynamic segment?
                           │      (dynamic)    │         │
                           │                   ใช่       ไม่ใช่
                           │                   │         │
                           │                   ▼         ▼
                           │                 SSR      มี dynamic data marker?
                           │                   │      (fetch, cookies, headers,
                           │                   │       Date.now, Math.random,
                           │                   │       process.env, searchParams)
                           │                   │         │
                           │                   │         │
                           │                   ◄─────────┤
                           │                   │  มี     │  ไม่มี
                           │                   ▼         ▼
                           │                SSR        SSG
                           │                           (static)
◄───────────────────────────┘───────────────────────────►

SSG ─── static HTML, ไม่ต้องใช้ Node runtime
ISR ─── SSG + cache หมดอายุ + background refresh
SSR ─── server render ทุก request
PPR ─── static shell + live content streaming
CSR ─── browser render ล้วนๆ
```

---

## Detection Algorithm Priority

ที่ `crates/ruvyxa_graph/src/lib.rs:1247-1335`:

```rust
fn detect_render_strategy(app_dir, file, route_path, layout_chain) -> RenderMeta {
    // 1. ตรวจ 'use client' directive
    // 2. ตรวจ export const ppr = true
    // 3. ตรวจ export const revalidate = <number>
    // 4. ตรวจ getStaticParams / staticParams
    // 5. ตรวจ dynamic segments + dynamic data markers
    // 6. default → SSR
}
```

### ลำดับการตรวจสอบ

| Priority   | เงื่อนไข                                                             | กลยุทธ์       |
| ---------- | -------------------------------------------------------------------- | ------------- |
| 1 (สูงสุด) | มี `'use client'` + ไม่มี server component โหลดข้อมูล                | CSR           |
| 2          | `export const ppr = true` + มี `<Suspense>` boundary                 | PPR           |
| 3          | `export const revalidate = N` (N > 0)                                | ISR           |
| 4          | `export function getStaticParams()` หรือ `export const staticParams` | SSG (dynamic) |
| 5          | ไม่มี dynamic segment + ไม่มี dynamic data markers                   | SSG (static)  |
| 6 (ต่ำสุด) | default                                                              | SSR           |

### Dynamic data markers

Ruvyxa ตรวจจับ dynamic API calls ใน source code:

```rust
const MARKERS: &[&str] = &[
    "fetch(",        // HTTP requests
    "headers(",      // headers()
    "cookies(",      // cookies()
    "searchParams",  // URLSearchParams
    "Date.now(",     // current time
    "Math.random(",  // random value
    "process.env.",  // env variables
];
```

ถ้า marker ใดปรากฏใน route หรือ dependency → SSR (ยกเว้น `process.env.RUVYXA_PUBLIC_*` ซึ่งปลอดภัย)

### ข้อควรระวังของ detection algorithm

1. **Markers ถูกตรวจจับในทุกไฟล์ใน dependency tree** — layout + imports
2. **MDX/`.md` files**: code in fenced blocks ถูก blank out
3. **Code in strings/comments**: ถูก blank out (`code_without_strings_and_comments`)
4. **Regex literals**: ถูกตรวจจับและเว้น (`regex_can_start()`) — ป้องกัน `/['"]/` ถูกรู้ว่าเป็น
   string
5. **Circular imports**: ป้องกันด้วย `visited` set

### Detection edge cases

```tsx
// ไฟล์นี้จะถูกตรวจจับเป็น SSG (static) ในกรณีใด:
// ✅ ไม่มี 'use client'
// ✅ ไม่มี ppr, revalidate, getStaticParams
// ✅ ไม่มี dynamic segment ใน path
// ✅ ไม่มี fetch(), headers(), cookies(), Date.now(), Math.random()
export default function StaticPage() {
  return <p>สวัสดี</p>
}
```

```tsx
// ไฟล์นี้จะถูกตรวจจับเป็น SSR:
// ❌ มี process.env.DATABASE_URL (ไม่ใช่ RUVYXA_PUBLIC_*)
export default function Page() {
  const db = process.env.DATABASE_URL
  return <p>Hello</p>
}
```

---

## SSR — Server-Side Rendering (ค่าเริ่มต้น)

สร้าง HTML ทุกครั้งที่มี request เหมาะกับข้อมูลที่เปลี่ยนตลอด

```tsx
// SSR page — ไม่มี export พิเศษ, ใช้ default
export default function ProfilePage({ params }: { params: { id: string } }) {
  return (
    <main>
      <h1>โปรไฟล์ {params.id}</h1>
      <p>หน้านี้ render ทุกครั้งที่มีคนเปิด</p>
    </main>
  )
}
```

### เมื่อใช้ SSR

- ข้อมูลเปลี่ยนตลอด (user dashboard, real-time metrics)
- ต้อง authentication ทุก request
- personalization ตาม user
- ใช้ dynamic APIs: `cookies()`, `headers()`, `searchParams`
- ข้อมูลเฉพาะ session

### SSR Flow

```
Request 1                Request 2
    │                        │
    ▼                        ▼
  Server ──render──► HTML1   Server ──render──► HTML2
    │                        │
    ▼                        ▼
  HTML1 (ต่างจาก HTML2)     HTML2
```

### SSR Type signature

```tsx
export default async function SsrPage(props: PageProps): Promise<React.ReactElement> {
  // สามารถใช้ async/await
  const user = await getCurrentUser()
  return <h1>สวัสดี {user.name}</h1>
}
```

### SSR Edge case: async component

```tsx
// SSR async component — ใช้ await ได้
export default async function UserDashboard({ params }: { params: { id: string } }) {
  const [user, posts, notifications] = await Promise.all([
    db.query('SELECT * FROM users WHERE id = ?', [params.id]),
    db.query('SELECT * FROM posts WHERE user_id = ?', [params.id]),
    db.query('SELECT * FROM notifications WHERE user_id = ?', [params.id]),
  ])

  return (
    <div>
      <h1>{user.name}</h1>
      <p>โพสต์: {posts.length}</p>
      <p>แจ้งเตือน: {notifications.length}</p>
    </div>
  )
}
```

---

## SSG — Static Site Generation

สร้าง HTML ตอน build time ไม่ต้องใช้ server runtime เลย

### Static Route (ไม่มี dynamic segment)

```tsx
// app/static-page/page.tsx
// SSG โดยอัตโนมัติ — ไม่มี dynamic segment, ไม่มี dynamic API
export default function StaticPage() {
  const buildTime = new Date().toISOString() // ไม่ใช่ Date.now()

  return (
    <main>
      <h1>SSG Page</h1>
      <p>สร้างตอน build: {buildTime}</p>
      <p>ค่านี้จะไม่เปลี่ยนจนกว่า build ใหม่</p>
    </main>
  )
}
```

**ข้อแตกต่าง**: `new Date().toISOString()` != `Date.now()`

- `new Date()` — object instantiation, ไม่นับเป็น dynamic marker
- `Date.now()` — function call, นับเป็น dynamic marker → SSR

### SSG conditions

หน้าได้ SSG (static) เมื่อ **ทุกข้อตรง**:

1. ไม่มี `'use client'`
2. ไม่มี `export const ppr = true`
3. ไม่มี `export const revalidate = N`
4. ไม่มี `getStaticParams` / `staticParams`
5. Route path ไม่มี dynamic segment
6. ไม่มี dynamic data markers ใน route + layout dependencies

### Dynamic Route + getStaticParams

```tsx
// app/ssg-blog/[slug]/page.tsx
import type { GetStaticParams, PageProps } from 'ruvyxa/config'

const posts = [
  { slug: 'hello-world', title: 'Hello', content: 'สวัสดี!' },
  { slug: 'ruvyxa-guide', title: 'Guide', content: 'คู่มือ Ruvyxa' },
  { slug: 'performance', title: 'Perf', content: 'เทคนิค performance' },
]

export const getStaticParams: GetStaticParams<{ slug: string }> = async () => {
  return posts.map((p) => ({ slug: p.slug }))
}

export default function BlogPost({ params }: PageProps<{ slug: string }>) {
  const post = posts.find((p) => p.slug === params.slug)

  if (!post) {
    return <h1>ไม่พบโพสต์</h1>
  }

  return (
    <main>
      <h1>{post.title}</h1>
      <p>{post.content}</p>
      <p className="badge">SSG: สร้างตอน build time</p>
    </main>
  )
}
```

### Output schema

```
.ruvyxa/prerender/ssg-blog/hello-world/index.html
.ruvyxa/prerender/ssg-blog/ruvyxa-guide/index.html
.ruvyxa/prerender/ssg-blog/performance/index.html
```

### GetStaticParams type

```ts
import type {
  GetStaticParams,
  StaticParamsContext,
  StaticParamsSegment,
  StaticParamsResult,
  StaticParamsValues,
  CachedStaticParams,
  StaticParamsCacheDuration,
} from 'ruvyxa/config'

// Signature
type GetStaticParams<TParams extends RouteParams = RouteParams> = (
  ctx: StaticParamsContext,
) => StaticParamsResult<TParams> | Promise<StaticParamsResult<TParams>>
```

### StaticParamsContext

```ts
interface StaticParamsContext {
  routes: Array<{ path: string; id: string }> // routes ทั้งหมด
  route: {
    path: string // path ของ route นี้
    segments: StaticParamSegment[] // dynamic segments
  }
}

interface StaticParamSegment {
  name: string // ชื่อ segment
  catchAll: boolean // [...slug]?
  optional: boolean // [[...slug]]?
}
```

### StaticParamsResult

```ts
type StaticParamsResult<TParams> = StaticParamsValues<TParams> | CachedStaticParams<TParams>

type StaticParamsValues<TParams> = ReadonlyArray<TParams | string | number>

interface CachedStaticParams<TParams> {
  params: StaticParamsValues<TParams>
  cache: StaticParamsCacheDuration // 60 | '5m' | '1h' | '1d'
}

type StaticParamsCacheDuration = number | `${number}${'s' | 'm' | 'h' | 'd'}`
```

### Parameter รูปแบบที่ยอมรับ

```tsx
// 1. Object array (recommended)
export const getStaticParams = async () => {
  return [{ slug: 'hello' }, { slug: 'world' }]
}

// 2. String array (สำหรับ 1 dynamic segment)
export const getStaticParams = async () => {
  return ['hello', 'world'] // เหมือน [{ slug: 'hello' }, { slug: 'world' }]
}

// 3. Number array (สำหรับ 1 dynamic segment)
export const getStaticParams = async () => {
  return [1, 2, 3] // เหมือน [{ id: '1' }, { id: '2' }, { id: '3' }]
}

// 4. Cached version
export const getStaticParams = async () => {
  const posts = await fetchPosts()
  return {
    params: posts.map((p) => ({ slug: p.slug })),
    cache: '1h', // cache result 1 ชั่วโมง
  }
}
```

### SSG (static) Output structure

```
.ruvyxa/
├── prerender/
│   ├── static-page/
│   │   └── index.html          # SSG static page
│   ├── ssg-blog/
│   │   ├── hello-world/
│   │   │   └── index.html      # SSG dynamic page
│   │   ├── ruvyxa-guide/
│   │   │   └── index.html
│   │   └── performance/
│   │       └── index.html
│   ├── nested/
│   │   └── page/
│   │       └── index.html      # nested route
├── server/
│   └── page-ssr.js             # SSR fallback handler
└── client/
    └── page.js                 # client bundles
```

### SSG fallback behavior

เมื่อ SSG page ไม่มี prerender output:

- ถ้า route มี `export const revalidate` → ใช้ ISR fallback
- ถ้าไม่มี → 404

---

## ISR — Incremental Static Regeneration

SSG + cache หมดอายุ + refresh ใน background:

```tsx
// app/isr-page/page.tsx
export const revalidate = 60 // หมดอายุทุก 60 วินาที

export default function IsrPage() {
  const now = new Date().toISOString()

  return (
    <main>
      <h1>ISR Page</h1>
      <p>
        เวลาปัจจุบัน: <code>{now}</code>
      </p>
      <p>cache อยู่ 60s จากนั้น refresh เอง</p>
    </main>
  )
}
```

### ISR cache algorithm

```
เวลา        Event
──────────────────────────────────────────
T+0s       Request 1 → render → cache
T+10s      Request 2 → เสิร์ฟจาก cache (instant)
T+45s      Request 3 → เสิร์ฟจาก cache
T+70s      Request 4 → เสิร์ฟ cache (stale) + background refresh เริ่ม
T+70.1s    background render → cache ใหม่
T+75s      Request 5 → เสิร์ฟ cache ใหม่ (instant)
```

### อัลกอริทึมละเอียด

```
request เข้ามาที่ /isr-page
    │
    ▼
มี file ใน prerender/ หรือไม่?
    │         │
    มี         ไม่มี
    │         │
    ▼         ▼
เช็ค cache age:   render → cache → serve
  age < revalidate?
    │         │
    ใช่       ไม่ใช่
    │         │
    ▼         ▼
 serve cache    เสิร์ฟ cache (stale)
 (instant)     + เริ่ม background job
                เพื่อ render cache ใหม่
```

### ISR + getStaticParams

ISR สามารถใช้กับ dynamic routes ได้:

```tsx
// app/products/[id]/page.tsx
export const revalidate = 300 // 5 นาที

export const getStaticParams: GetStaticParams<{ id: string }> = async () => {
  const products = await db.query('SELECT id FROM products')
  return products.map((p) => ({ id: p.id.toString() }))
}

export default async function ProductPage({ params }: PageProps<{ id: string }>) {
  const product = await db.query('SELECT * FROM products WHERE id = ?', [params.id])

  return (
    <main>
      <h1>{product.name}</h1>
      <p>ราคา: {product.price} บาท</p>
      <p>อายุ cache: 5 นาที</p>
    </main>
  )
}
```

### ISR revalidate ค่าต่างๆ

| ค่า     | พฤติกรรม                 |
| ------- | ------------------------ |
| `60`    | 60 วินาที — cache 1 นาที |
| `300`   | 5 นาที                   |
| `3600`  | 1 ชั่วโมง                |
| `86400` | 1 วัน                    |

### เมื่อใช้ ISR

- e-commerce ราคาสินค้าเปลี่ยนบ้าง
- blog ที่มี comments
- dashboard refresh ทุก 5 นาที
- product catalog
- news site

### revalidate type

```ts
// page export
export const revalidate: number // seconds, > 0
```

- ค่า `0` → ถือว่าไม่มี revalidate ไม่ใช่ ISR
- ค่า null/undefined → ไม่ใช่ ISR
- ถ้า `render.strategy = 'isr'` ใน config → route inherit strategy

---

## PPR — Partial Pre-Rendering

static shell + dynamic slots:

```tsx
// app/ppr-page/page.tsx
import { Suspense } from 'react'

export const ppr = true

async function DynamicSection() {
  const timestamp = new Date().toISOString()
  await new Promise((r) => setTimeout(r, 1000)) // simulate async work
  return (
    <div>
      <h3>Dynamic Content (streamed)</h3>
      <p>
        เวลาปัจจุบัน: <code>{timestamp}</code>
      </p>
    </div>
  )
}

function Fallback() {
  return <div className="skeleton">กำลังโหลด...</div>
}

export default function PprPage() {
  return (
    <main>
      <h1>PPR Page</h1>
      <p>ส่วน static นี้ build พร้อม shell</p>

      {/* dynamic slot */}
      <Suspense fallback={<Fallback />}>
        <DynamicSection />
      </Suspense>

      <p>ส่วนนี้ก็ static ด้วย</p>
    </main>
  )
}
```

### PPR build time

```
ตอน build:
┌──────────────────────────────┐
│  static shell (HTML)         │  ← prerendered
│  ┌──────────────────────┐    │
│  │ <Suspense>           │    │  ← hole
│  │ fallback: skeleton   │    │
│  └──────────────────────┘    │
└──────────────────────────────┘
```

### PPR request time

```
ตอน request:
┌──────────────────────────────┐
│  static shell                │  ← served instantly
│  ┌──────────────────────┐    │
│  │ dynamic content      │    │  ← streamed
│  └──────────────────────┘    │
└──────────────────────────────┘
```

### PPR streaming behavior

1. **Build time**: static shell ถูก prerender ส่วนที่ไม่ใช่ `<Suspense>`
2. **Request time**: static shell ถูกส่งทันที
3. **Streaming**: dynamic content ภายใน `<Suspense>` ถูก render แบบ streaming
4. **Fallback**: `<Suspense fallback>` ถูกแสดงระหว่างรอ dynamic content
5. **Replace**: เมื่อ dynamic content พร้อม, fallback ถูกแทนที่

### PPR ต้องมี `<Suspense>` boundary

```tsx
// ❌ ผิด: ppr = true แต่ไม่มี Suspense
export const ppr = true

export default function Page() {
  return <p>ไม่มี Suspense — PPR ไม่มีผล</p>
}

// ✅ ถูก: ppr = true + Suspense
export const ppr = true

export default function Page() {
  return (
    <Suspense fallback={<p>โหลด...</p>}>
      <DynamicContent />
    </Suspense>
  )
}
```

### PPR output

```
.ruvyxa/
├── prerender/
│   └── ppr-page/
│       ├── index.html         ← static shell
│       └── ppr-shell.json     ← metadata (dynamic slots info)
```

### เมื่อใช้ PPR

- product page: shell static + ราคา/stock dynamic
- social feed: shell static + content personal
- dashboard: layout cached + metrics live
- รายการสินค้า: category static + inventory dynamic

---

## CSR — Client-Side Rendering

browser render ทั้งหมด:

```tsx
'use client'

import { useState, useEffect } from 'react'

export default function CsrPage() {
  const [count, setCount] = useState(0)
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
  }, [])

  return (
    <main>
      <h1>CSR Page</h1>
      <p>หน้านี้ render ใน browser เท่านั้น</p>

      {mounted && (
        <div>
          <p>
            นับ: <strong>{count}</strong>
          </p>
          <button onClick={() => setCount((c) => c + 1)}>เพิ่ม</button>
        </div>
      )}
    </main>
  )
}
```

### การตรวจจับ CSR

Route ถูกกำหนดเป็น CSR เมื่อ:

1. ไฟล์มี `'use client'` directive ที่บรรทัดแรก
2. ไม่มี server-side data loading (async, fetch, headers, cookies)
3. = ไฟล์ทั้งหมดเป็น client code

### CSR Flow

```
Server ส่ง:          Client:
<html>               1. โหลด bundle.js
  <body>             2. React hydrate/render
    <div id="root">  3. เริ่ม interactive
    </div>
    <script src="/bundle.js">
    </script>
  </body>
</html>
```

### เมื่อใช้ CSR

- admin dashboard หลัง login
- real-time editor
- canvas, WebGL apps
- หน้าไม่สนใจ SEO
- interactive-heavy application

### CSR Limitations

1. **SEO ไม่ดี** — crawler เห็นแค่ empty shell
2. **LCP (Largest Contentful Paint) ช้า** — ต้องรอ JS bundle
3. **FCP (First Contentful Paint) ช้า** — ต้องรอ JS
4. **ผู้ใช้ JS disabled → ไม่เห็นเนื้อหา**

---

## RenderMeta structure

ใน `crates/ruvyxa_graph/src/lib.rs:91-117`:

```rust
struct RenderMeta {
    strategy: RenderStrategy,         // Ssr | Ssg | Isr | Csr | Ppr
    revalidate: Option<u64>,          // ISR TTL (seconds)
    has_static_params: bool,          // มี getStaticParams?
    static_paths: Vec<String>,        // paths ที่ค้นพบจาก getStaticParams
    has_dynamic_slots: bool,          // มี <Suspense> สำหรับ PPR?
    hydrate: bool,                    // opt-out zero-JS?
    hydration: HydrationMode,         // Load | Idle | Visible | None
}
```

### RenderStrategy enum

```rust
enum RenderStrategy {
    Ssr,  // Server-Side Rendering (default)
    Ssg,  // Static Site Generation
    Isr,  // Incremental Static Regeneration
    Csr,  // Client-Side Rendering
    Ppr,  // Partial Pre-Rendering
}
```

### RouteEntry structure

ใน `crates/ruvyxa_graph/src/lib.rs:22-36`:

```rust
struct RouteEntry {
    id: String,
    path: String,
    kind: RouteKind,             // Page | Api
    file: PathBuf,
    layout_chain: Vec<String>,
    server_modules: Vec<String>,
    client_modules: Vec<String>,
    runtime: RuntimeTarget,      // Node | Edge | Static
    render: RenderMeta,
}
```

---

## Hydration Scheduling

ควบคุมเวลา hydration ของ client components:

```tsx
'use client'

export const hydrate = 'idle' // 'load' | 'idle' | 'visible' | false
```

| ค่า                | ภาษาไทย     | เมื่อทำงาน                                    |
| ------------------ | ----------- | --------------------------------------------- |
| `'load'`           | โหลดเสร็จ   | ทันทีที่ document parser ถึง module (default) |
| `'idle'`           | รอว่าง      | เมื่อ browser idle (`requestIdleCallback`)    |
| `'visible'`        | เห็นได้     | เมื่อ component อยู่ใน viewport               |
| `false` / `'none'` | ไม่ hydrate | Zero-JS page                                  |

### HydrationMode enum

ใน `crates/ruvyxa_graph/src/lib.rs:74-86`:

```rust
enum HydrationMode {
    Load,     // default: hydrate ทันที
    Idle,     // hydrate เมื่อ idle
    Visible,  // hydrate เมื่อ visible
    None,     // ไม่ hydrate
}
```

### Zero-JS Pages

```tsx
export const hydrate = false

export default function StaticPage() {
  return <p>หน้านี้ไม่มี JavaScript เลย — เร็วสุดๆ</p>
}
```

ใช้กับ:

- Landing pages
- Marketing pages
- Blog content
- หน้า static content

### Hydration + Client Component Interaction

```tsx
// app/page.tsx
export const hydrate = 'visible'

export default function Page() {
  return <InteractiveButton label="คลิก" />
}

// เมื่อ hydrate = 'visible':
// 1. HTML ถูก server render → ส่งไป browser
// 2. JS ไม่ถูกโหลดทันที — component ไม่ทำงาน
// 3. เมื่อ component อยู่ใน viewport → โหลด JS → hydrate
// 4. Button เริ่ม interactive
```

---

## Prerender Output Schema

```
.ruvyxa/
├── prerender/                      # pre-rendered pages
│   ├── [route-path]/
│   │   ├── index.html             # SSG/ISR HTML
│   │   └── ppr-shell.json         # PPR metadata (ถ้ามี)
│   ├── isr-page/
│   │   └── index.html             # ISR (cache first)
│   └── ppr-page/
│       ├── index.html             # PPR static shell
│       └── ppr-shell.json         # dynamic slots metadata
├── server/
│   ├── page-ssr.js                # SSR fallback handler
│   └── serverless-handler.mjs     # serverless entry
├── client/
│   ├── page-home.js               # client bundle pages
│   ├── page-blog-[slug].js
│   └── shared-*.js                # shared chunks
├── route-manifest.json             # build report
└── route-modules.mjs              # route module registry
```

### Prerender output file naming

```
Static route:    /about → prerender/about/index.html
Dynamic route:   /blog/[slug] → prerender/blog/[slug-value]/index.html
ISR:             /isr-page → prerender/isr-page/index.html
PPR shell:       /ppr-page → prerender/ppr-page/index.html + ppr-shell.json
```

---

## ตารางเปรียบเทียบ

| กลยุทธ์ | เวลา render     | Server runtime | Dynamic      | SEO    | เหมาะกับ              |
| ------- | --------------- | -------------- | ------------ | ------ | --------------------- |
| SSG     | build time      | ❌ ไม่ต้อง     | ❌ ไม่มี     | ✅ ดี  | blog, marketing, docs |
| SSR     | request time    | ✅ ต้องมี      | ✅ เต็มที่   | ✅ ดี  | dashboard, profile    |
| ISR     | build + refresh | ✅ ต้องมี      | ✅ พอได้     | ✅ ดี  | e-commerce, news      |
| PPR     | build + request | ✅ ต้องมี      | ✅ streaming | ✅ ดี  | product page, feeds   |
| CSR     | browser         | ❌ ไม่ต้อง     | ✅ เต็มที่   | ❌ แย่ | admin, editor         |

| กลยุทธ์ | TTFB                   | Time-to-Interactive | ข้อมูลที่ได้             |
| ------- | ---------------------- | ------------------- | ------------------------ |
| SSG     | instant (<1ms)         | instant             | data ตอน build           |
| SSR     | request time-dependent | หลัง hydration      | data real-time           |
| ISR     | instant (cache)        | instant (ถ้า cache) | data ตอน last revalidate |
| PPR     | instant (shell)        | หลัง stream เสร็จ   | shell + streaming        |
| CSR     | instant (shell)        | หลัง JS render      | data ตอน fetch           |

---

## Config Default Strategy

ตั้งค่า default render strategy ได้ใน config:

```ts
// ruvyxa.config.ts
import { config, type RuvyxaConfig } from 'ruvyxa/config'

const settings: RuvyxaConfig = {
  render: {
    strategy: 'ssr', // default: 'ssr'
    revalidate: 60, // default ISR TTL (seconds)
  },
}

export default config(settings)
```

### กลไก apply_rendering_defaults

ใน `crates/ruvyxa_graph/src/lib.rs:1389-1407`:

```rust
fn apply_rendering_defaults(render, default_strategy, default_revalidate) {
    // ใช้ default strategy เฉพาะเมื่อ detect strategy = SSR
    // ถ้า detect strategy ไม่ใช่ SSR → ใช้ของ route นั้น
    // ถ้า default strategy = ISR → ใช้ default_revalidate (หรือ 60)
}
```

### Per-route override

Route สามารถ override config default:

```tsx
// route นี้เป็น SSG แม้ config จะตั้งเป็น SSR
export function getStaticParams() {
  return posts.map((p) => ({ slug: p.slug }))
}
```

### Config strategy influences

| Config strategy   | Effect                                                      |
| ----------------- | ----------------------------------------------------------- |
| `'ssr'` (default) | ไม่มี effect — ทุก route ใช้ default detection              |
| `'ssg'`           | Routes ที่ detect เป็น SSR จะกลายเป็น SSG                   |
| `'isr'`           | Routes ที่ detect เป็น SSR จะกลายเป็น ISR (revalidate = 60) |
| `'csr'`           | Routes ที่ detect เป็น SSR จะกลายเป็น CSR                   |

---

## Best Practices

```tsx
// 1. เริ่มต้นเป็น SSG เสมอ ถ้าเป็นไปได้
//    ตรวจสอบก่อน: ข้อมูลเปลี่ยนบ่อยมั้ย? ไม่ → SSG

// 2. SSG + ISR สำหรับข้อมูลที่เปลี่ยนบ้าง
export const revalidate = 300 // 5 นาที

// 3. SSR สำหรับข้อมูลเฉพาะ user
export default async function ProfilePage() {
  const user = await getCurrentUser()
  return <h1>สวัสดี {user.name}</h1>
}

// 4. PPR framework ตรวจสอบให้เอง
export const ppr = true
// + Suspense boundaries

// 5. CSR สำหรับ interactive-heavy
;('use client')
// หน้า admin, settings, real-time
```

### แนวทางการเลือกกลยุทธ์

| สถานการณ์                  | กลยุทธ์ที่แนะนำ        |
| -------------------------- | ---------------------- |
| Blog post ที่ไม่เปลี่ยน    | SSG                    |
| Blog ที่มี comments        | ISR (revalidate = 300) |
| Dashboard ส่วนตัว          | SSR                    |
| หน้ารายการสินค้า           | ISR หรือ PPR           |
| Admin dashboard หลัง login | CSR                    |
| Landing page               | SSG + hydrate = idle   |
| Product detail             | PPR                    |
| API backend                | SSR (route.ts)         |

### Performance recommendations

```tsx
// Landing page: SSG + deferred hydration
export const hydrate = 'idle'

export default function LandingPage() {
  return (
    <main>
      <h1>ยินดีต้อนรับ</h1>
      <p>เนื้อหา static ทั้งหมด</p>
      {/* interactive parts จะ hydrate เมื่อ idle */}
    </main>
  )
}
```

---

## Troubleshooting

| ปัญหา                             | สาเหตุ                                                     | วิธีแก้                                      |
| --------------------------------- | ---------------------------------------------------------- | -------------------------------------------- |
| SSG ไม่ทำงาน ไป SSR ตลอด          | มี dynamic API เช่น `cookies()`, `headers()`, `Date.now()` | ใช้ ISR หรือ SSR แทน                         |
| ISR ไม่ refresh                   | `revalidate` ยังไม่ถึง หรือเกิด error ใน background render | รอหรือลดค่า revalidate                       |
| PPR static shell มีข้อมูล dynamic | ลืม `<Suspense>` boundary                                  | ห่อ dynamic content ด้วย `<Suspense>`        |
| CSR มี flash                      | ไม่มี fallback state                                       | เพิ่ม loading state                          |
| build ช้าเพราะ SSG มากเกินไป      | getStaticParams คืนค่ามาก                                  | ใช้ ISR หรือ limit params                    |
| SSG ใช้ build time นาน            | หน้า SSG มากเกินไป                                         | บางหน้าเปลี่ยนเป็น ISR                       |
| ISR cache ไม่ refresh             | background render error                                    | ดู server logs                               |
| PPR stream ทำงานช้า               | dynamic slot ทำงานหนัก                                     | ใช้ `<Suspense>` หลาย level หรือเพิ่ม cache  |
| getStaticParams fail              | database/API ไม่พร้อมตอน build                             | ใช้ try/catch หรือ ISR แทน                   |
| hydration mismatch                | server HTML ≠ client first render                          | ตรวจสอบ `useEffect` + `Date` / `Math.random` |

### ตัวอย่างข้อผิดพลาด

```tsx
// SSG ไม่ทำงานเพราะ Date.now()
export default function Page() {
  // ❌ Date.now() → SSR (dynamic marker)
  const time = Date.now()
  return <p>{time}</p>
}

// ✅ ใช้ new Date() → SSG (ไม่เป็น dynamic marker)
export default function Page() {
  const time = new Date().toISOString()
  return <p>{time}</p>
}
```

```tsx
// PPR ไม่ทำงานเพราะไม่มี Suspense
export const ppr = true

export default function Page() {
  // ❌ PPR ต้องการ <Suspense> boundary
  return <DynamicContent />
}

// ✅ ถูก
export const ppr = true

export default function Page() {
  return (
    <div>
      <p>Static shell</p>
      <Suspense fallback={<p>Loading...</p>}>
        <DynamicContent />
      </Suspense>
    </div>
  )
}
```

---

## ลองทำดู

```bash
# 1. สร้างหน้า SSG ง่ายๆ
mkdir -p app/ssg-test
New-Item app/ssg-test/page.tsx

# 2. เขียน:
# export default function SSGTest() {
#   return <p>build time: {Date.now()}</p>
# }
# → สนใจ: ใช้ Date.now() → SSR

# 3. สร้าง production build
npm run build

# 4. เช็ค output
Get-ChildItem .ruvyxa/prerender/ssg-test/

# 5. เปิดอีกครั้ง — time จะเท่าเดิม (static)
npm run start
# → http://localhost:3000/ssg-test
```

```bash
# ทดสอบ ISR
mkdir -p app/isr-demo
New-Item app/isr-demo/page.tsx

# เขียน:
# export const revalidate = 10
# export default function IsrDemo() {
#   return <p>{new Date().toISOString()}</p>
# }

npm run build
npm run start
# เปิด /isr-demo → เห็นเวลา
# รอ 10 วินาที → refresh → cache ใหม่
```

---

## วิธีที่ Strategy Detector ปัจจุบันตัดสินใจ

Rendering strategy ได้จาก source ของ page ร่วมกับ route และ relative dependencies ที่เข้าถึงได้
Detector ใช้ลำดับความสำคัญนี้ โดยกฎแรกที่ตรงจะชนะ:

1. `'use client'` ที่ต้นไฟล์ทำให้ page เป็น CSR
2. `export const ppr = true` เลือก PPR
3. `export const revalidate = <number>` เลือก ISR พร้อม interval นั้น
4. export `getStaticParams` หรือ `staticParams` เลือก SSG
5. route ที่ไม่มี dynamic segment และไม่มี marker `fetch(` หรือ `process.env.` ใน dependencies เป็น
   SSG candidate อัตโนมัติ
6. นอกนั้นเป็น SSR เว้นแต่ `render.strategy` ใน config กำหนดค่าเริ่มต้นไว้

ดังนั้น `Date.now()` เพียงอย่างเดียวไม่ใช่สัญญาณที่ detector ใช้ตัดสิน SSR หาก page ต้องการ contract
ชัดเจน ให้ประกาศ strategy ผ่าน export หรือ config ที่รองรับ แทนการพึ่งรูปร่างของโค้ดโดยบังเอิญ

### ตัวอย่าง Explicit และผลลัพธ์

```tsx
// app/docs/page.tsx -- ไม่มี dynamic segment และ data marker: SSG candidate
export default function Docs() {
  return <main>Documentation</main>
}
```

```tsx
// app/blog/[slug]/page.tsx -- dynamic SSG ต้องให้ concrete parameters
export const getStaticParams = async () => [{ slug: 'welcome' }, { slug: 'release-notes' }]

export default function Post({ params }: { params: { slug: string } }) {
  return <main>{params.slug}</main>
}
```

```tsx
// app/status/page.tsx -- build ครั้งเดียว แล้ว refresh เบื้องหลังเมื่อครบ 30 วินาที
export const revalidate = 30

export default async function Status() {
  const response = await fetch('https://status.example.test/api')
  return <pre>{await response.text()}</pre>
}
```

สำหรับ ISR interval อยู่ใน `revalidate`; พฤติกรรม cache ระหว่าง request เป็นหน้าที่ของ server หรือ
deployment adapter ที่เลือก ให้ตรวจ manifest จริง แทนการเดาจาก directory output:

```bash
ruvyxa routes
ruvyxa trace /status
```

### Hydration เป็นการตัดสินใจแยกกัน

สำหรับ server-rendered strategies, `export const hydrate` ควบคุมว่าจะโหลด client bundle หรือไม่และ
เมื่อไร ค่าที่รองรับคือ `false` (ไม่ส่ง client bundle), `'idle'`, `'visible'` และค่า default ที่โหลด
ทันที CSR page ที่มาจาก `'use client'` ยังเป็น client-rendered แม้จะมี hydration export ด้วย

```tsx
// static content page แบบ zero-JS
export const hydrate = false

export default function LegalNotice() {
  return <main>Terms and conditions</main>
}
```

ใช้ zero-JS page เมื่อ rendered content ไม่พึ่ง client interactivity เท่านั้น เพราะ client islands
ยังต้องมี hydrated client bundle จึงควรเก็บ behavior ที่ interactive ไว้ใน client-reachable module

---

## ขั้นตอนถัดไป

- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — โหลดข้อมูลและ cache
- **[06-server-actions.md](./06-server-actions.md)** — Server actions
