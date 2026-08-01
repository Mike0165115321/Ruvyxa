# Server & Client Components

Ruvyxa แบ่ง component เป็น 2 โลก:

```
┌─────────────────────────────────────────────┐
│          Server Components                  │
│  (default — ไม่ต้องเขียนอะไร)               │
│                                             │
│  • อ่าน database ได้โดยตรง                  │
│  • import 'server-only'                     │
│  • ใช้ env ตัวไหนก็ได้ (public + private)   │
│  • ไม่มี useState, useEffect, onClick       │
│  • ไม่มี interactivity                      │
│  • ลด JavaScript ที่ส่งไป browser           │
│  • async ได้ (await ใน component)           │
│  • JSON-serializable props only             │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│          Client Components                  │
│  ('use client' directive)                   │
│                                             │
│  • มี useState, useEffect, onClick          │
│  • interactivity เต็มที่                    │
│  • import 'server-only' ไม่ได้              │
│  • ใช้ได้เฉพาะ RUVYXA_PUBLIC_* env         │
│  • ต้อง JSON-serializable props             │
│  • ต้อง sync (hooks แทน async)             │
└─────────────────────────────────────────────┘
```

---

## Server Components (ค่าเริ่มต้น)

ทุก component ใน Ruvyxa เป็น server component โดยอัตโนมัติ ไม่ต้องเพิ่มอะไร:

```tsx
// app/page.tsx — SERVER COMPONENT
// สามารถอ่าน database, file system, env ได้โดยตรง

export default function HomePage() {
  const now = new Date().toISOString()

  return (
    <main>
      <h1>สวัสดี</h1>
      <p>สร้างเมื่อ: {now}</p>
    </main>
  )
}
```

### สิ่งที่ server components ทำได้

```tsx
import { cache } from 'ruvyxa/server'

export default async function ProductPage() {
  // อ่าน database ได้โดยตรง
  const products = await db.query('SELECT * FROM products')

  // อ่านไฟล์ system
  const content = await fs.readFile('./content.md', 'utf-8')

  // ใช้ environment variables — ทั้ง public และ private
  const apiKey = process.env.STRIPE_SECRET_KEY
  const appName = process.env.RUVYXA_PUBLIC_APP_NAME

  // ใช้ cache system
  const data = await cache('products')
    .ttl('5m')
    .get(async () => {
      return await fetch('https://api.example.com/products').then((r) => r.json())
    })

  return (
    <main>
      <h1>สินค้า</h1>
      {products.map((p) => (
        <div key={p.id}>{p.name}</div>
      ))}
    </main>
  )
}
```

### สิ่งที่ server components ทำไม่ได้

```tsx
// ❌ ERROR — RUV1008
export default function BadComponent() {
  const [count, setCount] = useState(0) // ไม่ได้ — ต้องใช้ client component
  useEffect(() => {}, []) // ไม่ได้ — ต้องใช้ client component

  return <button onClick={() => setCount((c) => c + 1)}>คลิก</button> // ไม่ได้ — JS event handler
}
```

### ข้อจำกัดของ server components

1. **ไม่มี state** — `useState`, `useReducer`, `useRef` ใช้ไม่ได้
2. **ไม่มี effects** — `useEffect`, `useLayoutEffect`, `useInsertionEffect` ใช้ไม่ได้
3. **ไม่มี event handlers** — `onClick`, `onSubmit`, `onChange` ใช้ไม่ได้
4. **ไม่มี context** ที่ client-specific — `useContext` ของ client context
5. **ไม่มี custom hooks ที่ใช้ client features**
6. **ต้อง async ผ่าน `async function`** — ไม่มี `useEffect` สำหรับ side effects
7. **Props ต้อง JSON-serializable** — function, Date, RegExp ส่งข้าม boundary ไม่ได้
8. **ไม่ใช้ browser APIs** — `window`, `document`, `localStorage` เข้าถึงไม่ได้

### Server component ใช้ custom hook ได้ไหม?

ได้ ถ้า custom hook นั้นไม่ได้ใช้ client features:

```tsx
// server-safe custom hook
function useServerTime() {
  // ✅ ไม่มี useState, useEffect
  // ✅ แค่คำนวณค่าจาก server
  return new Date().toISOString()
}

export default function Page() {
  const time = useServerTime()
  return <p>{time}</p>
}
```

ผิด:

```tsx
function useWindowWidth() {
  // ❌ ใช้ useState และ effect
  const [width, setWidth] = useState(0)
  useEffect(() => {
    setWidth(window.innerWidth)
  }, [])
  return width
}
```

---

## การตรวจจับ 'use client' directive

Ruvyxa ตรวจหา `'use client'` directive ใน source code:

### ตำแหน่งที่ตรวจ

Directive ต้องอยู่ **บรรทัดแรก** หรือ **บรรทัดที่สอง** (หลัง shebang หรือ comment):

```tsx
// ✅ ถูก — บรรทัดแรก
'use client'
import { useState } from 'react'
```

```tsx
// ✅ ถูก — หลัง shebang (rare)
#!/usr/bin/env node
'use client'
```

```tsx
// ❌ ผิด — ไม่ใช่บรรทัดแรก
import { useState } from 'react'
;('use client') // ← จะไม่ถูกตรวจจับเป็น directive
```

### อัลกอริทึมตรวจจับ

ใน `crates/ruvyxa_graph/src/lib.rs:1269-1277`:

```rust
let trimmed = source.trim_start();
if trimmed.starts_with("\"use client\"") || trimmed.starts_with("'use client'") {
    return RenderMeta {
        strategy: RenderStrategy::Csr,
        ..Default::default()
    };
}
```

1. `source.trim_start()` — เอา whitespace และ newline ด้านหน้าออก
2. เช็คว่าขึ้นต้นด้วย `"use client"` หรือ `'use client'`
3. ถ้าใช่ → component นั้นเป็น CSR

### กรณีพิเศษ: MDX

ไฟล์ `.mdx` และ `.md` ถูกตรวจด้วย algorithm พิเศษ `markdown_without_code_examples()`:

- Fenced code blocks ถูก blank out
- Inline code spans ถูก blank out
- ESM imports (`import`/`export`) นอก fence ถูกเก็บไว้
- ทำให้ `'use client'` ใน code block ตัวอย่างไม่ถูกตรวจจับเป็น directive

---

## Client Components ('use client')

เมื่อต้องการ interactivity — event handlers, state, effects — เพิ่ม `'use client'` บรรทัดแรก:

```tsx
'use client'

import { useState } from 'react'

export default function Counter() {
  const [count, setCount] = useState(0)

  return (
    <div>
      <p>นับ: {count}</p>
      <button onClick={() => setCount((c) => c + 1)}>เพิ่ม</button>
    </div>
  )
}
```

### ตัวอย่าง client component เต็มรูปแบบ

```tsx
'use client'

import { useState, useEffect } from 'react'

export default function Dashboard() {
  const [data, setData] = useState(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetch('/api/dashboard')
      .then((r) => r.json())
      .then((d) => {
        setData(d)
        setLoading(false)
      })
  }, []) // [] = effect ทำงานครั้งเดียว

  if (loading) return <p>กำลังโหลด...</p>

  return (
    <div>
      <h1>Dashboard</h1>
      <pre>{JSON.stringify(data, null, 2)}</pre>
      <button onClick={() => setLoading(true)}>โหลดใหม่</button>
    </div>
  )
}
```

### Client component ที่เป็น async ไม่ได้

```tsx
// ❌ ผิด — client component ห้าม async
'use client'
export default async function ClientPage() {
  // Error: client component ไม่สามารถเป็น async function
  const data = await fetch('/api/data').then((r) => r.json())
  return <div>{data}</div>
}

// ✅ ถูก — ใช้ useEffect แทน
;('use client')
export default function ClientPage() {
  const [data, setData] = useState(null)

  useEffect(() => {
    fetch('/api/data')
      .then((r) => r.json())
      .then(setData)
  }, [])

  return <div>{data}</div>
}
```

---

## Composition Pattern

วิธีที่ถูกต้อง: ใช้ wrapper component ที่เป็น server แล้วใส่ client component ข้างใน:

```tsx
// ✅ ถูก: server wrapper + client child
// app/page.tsx
import { Counter } from './Counter'

export default function HomePage() {
  return (
    <main>
      <h1>หน้าแรก</h1>
      <p>เนื้อหา static — ไม่มี JS</p>

      {/* เฉพาะ Counter เท่านั้นที่ถูกส่ง JS ไป browser */}
      <Counter />
    </main>
  )
}
```

```tsx
'use client'

// app/Counter.tsx
import { useState } from 'react'

export function Counter() {
  const [count, setCount] = useState(0)
  return <button onClick={() => setCount((c) => c + 1)}>{count}</button>
}
```

### Children Pattern

ส่ง children จาก server ไป client component:

```tsx
'use client'

export default function Wrapper({ children }: { children: React.ReactNode }) {
  const [visible, setVisible] = useState(true)

  return (
    <div>
      <button onClick={() => setVisible((v) => !v)}>สลับ</button>
      {visible && children}
    </div>
  )
}
```

```tsx
// server component
import Wrapper from './Wrapper'

export default function Page() {
  return (
    <Wrapper>
      {/* children นี้ยังเป็น server component */}
      <div>เนื้อหาที่จะถูกซ่อน/แสดง</div>
    </Wrapper>
  )
}
```

### ข้อสำคัญของ Children Pattern

1. **Children ถูก server-render** — ถึงแม้ parent เป็น client component
2. **Children ถูกส่งเป็น JSX** — ไม่ใช่ client bundle
3. **Children ไม่ interactive** ถ้าไม่มี `'use client'` ใน children
4. **Children สามารถเป็น async** — server component ใน children

### Pattern: Server component import client component

```tsx
// ✅ correct: server → client import (one-way)
// app/page.tsx (server)
import InteractiveButton from './InteractiveButton' // client component

export default function Page() {
  return <InteractiveButton label="คลิกสิ" />
}
```

```tsx
// app/InteractiveButton.tsx (client)
'use client'

export default function InteractiveButton({ label }: { label: string }) {
  return <button onClick={() => alert('สวัสดี')}>{label}</button>
}
```

### Pattern ที่ผิด: Client component import server component

```tsx
// ❌ ผิด — อย่า import server component ใน client component โดยตรง
'use client'
import ServerMessage from './ServerMessage' // server component

export default function ClientPage() {
  return <ServerMessage /> // ServerMessage จะโดน bundle เป็น client code
  // → อาจ error ถ้า ServerMessage ใช้ server-only features
}
```

สิ่งที่เกิดขึ้นเมื่อ client import server component:

1. Bundler มองว่า `ServerMessage` อยู่ใน client dependency graph
2. `ServerMessage` ถูก compiled เป็น client bundle
3. ถ้า `ServerMessage` ใช้ `process.env.PRIVATE_KEY` → RUV1008
4. ถ้า `ServerMessage` import `server-only` → RUV1007

**วิธีแก้ที่ถูกต้อง**: ส่ง `<ServerMessage />` เป็น children (Children Pattern)

---

## Props ต้อง JSON-serializable

การส่ง props จาก server → client component:

```tsx
// ใช้ได้: string, number, boolean, object, array, null
<ClientCard
  title="สินค้า"           // string ✓
  price={299}             // number ✓
  inStock={true}          // boolean ✓
  tags={['ใหม่', 'ลดราคา']} // array ✓
  metadata={{ sku: 'A001' }} // object ✓
  description={null}      // null ✓
/>

// ใช้ไม่ได้:
<ClientCard
  onClick={() => {}}      // function ✗ — RUV1007
  date={new Date()}       // Date ✗ — ไม่ serialize
  regex={/pattern/}       // RegExp ✗
  style={{ display }}     // CSSStyleDeclaration ✗
/>
```

### JSON-serializable types ที่ยอมรับ

| Type             | Serializable? | หมายเหตุ                       |
| ---------------- | ------------- | ------------------------------ |
| `string`         | ✅            |                                |
| `number`         | ✅            | รวม NaN, Infinity              |
| `boolean`        | ✅            |                                |
| `null`           | ✅            |                                |
| `object` (plain) | ✅            |                                |
| `array`          | ✅            |                                |
| `undefined`      | ❌            | ถ้าเป็น top-level prop → error |
| `function`       | ❌            | RUV1007                        |
| `Date`           | ❌            | ใช้ string แทน                 |
| `RegExp`         | ❌            | ใช้ string pattern แทน         |
| `Symbol`         | ❌            |                                |
| `BigInt`         | ❌            |                                |
| `Map`, `Set`     | ❌            | ใช้ object แทน                 |
| `Promise`        | ❌            |                                |
| `ReactElement`   | ✅            | children is serializable       |
| cyclic reference | ❌            | stack overflow                 |

---

## Server-Only Code

### import 'server-only'

ใช้ guard ป้องกัน client import:

```tsx
// lib/database.ts
import 'server-only'

export const db = {
  async query(sql: string) {
    // นี้คือ database query
  },
}
```

ถ้ามี client component พยายาม import:

```tsx
'use client'
import { db } from '../lib/database'
// → RUV1007: Client boundary violation
//   "server-only" module imported in client context
```

### Modules ที่ถือว่าเป็น server-only โดยอัตโนมัติ

นอกจาก `import 'server-only'`, Ruvyxa ถือว่า import specifier ต่อไปนี้เป็น server-only:

```rust
// จาก crates/ruvyxa_graph/src/lib.rs:437-442
fn is_server_only_specifier(specifier: &str) -> bool {
    matches!(
        specifier,
        "server-only" | "@ruvyxa/auth" | "@ruvyxa/database"
    )
}
```

- `server-only` — แบบ manual
- `@ruvyxa/auth` — auth module (มี secret keys)
- `@ruvyxa/database` — database module (มี connection string)

### server/ directory

โฟลเดอร์ `server/` ถูกห้าม import จาก client โดยอัตโนมัติ ไม่ต้องใส่ `'server-only'`:

```
app/
  lib/
    server/
      config.ts          # ห้าม import จาก client component
      auth.ts            # ห้าม import จาก client component
    utils.ts             # ใช้ได้ทั้ง server และ client
```

### อัลกอริทึมตรวจจับ server/ directory

ใน `crates/ruvyxa_graph/src/lib.rs:726-731`:

```rust
fn relative_starts_with_server(relative: &Path) -> bool {
    relative
        .components()
        .next()
        .is_some_and(|component| component.as_os_str() == "server")
}
```

1. รับ `canonical_root` (project root ที่ normalize)
2. ใช้ `strip_prefix` ตัด root เพื่อให้ได้ relative path
3. เช็คว่า component แรกของ relative path = `server` หรือไม่
4. ถ้าใช่ → `RUV1010: Server directory module reached by client graph`

### Private env detection algorithm

Ruvyxa ตรวจจับการอ่าน private env vars ใน client graph:

ใน `crates/ruvyxa_graph/src/lib.rs:675-707`:

```
1. อ่าน source code
2. ลบ string literals, comments, template literals
3. ค้นหา "process.env"
4. หลังจาก process.env:
   a. ถ้าเป็น "." (dot notation): process.env.SECRET
      → เก็บชื่อ "SECRET"
   b. ถ้าเป็น "[" (bracket notation): process.env["SECRET"]
      → เก็บชื่อ "SECRET"
5. ตรวจว่าชื่อขึ้นต้นด้วย "RUVYXA_PUBLIC_" หรือไม่
6. ถ้าไม่ขึ้นต้น → RUV1008: Private env var
```

```tsx
// ✅ ใช้ได้ใน client:
console.log(import.meta.env.RUVYXA_PUBLIC_APP_NAME)
console.log(process.env.RUVYXA_PUBLIC_APP_NAME)

// ❌ Error ใน client:
console.log(process.env.DATABASE_URL) // RUV1008
console.log(process.env.STRIPE_SECRET) // RUV1008
console.log(process.env.AWS_SECRET_KEY) // RUV1008
```

### 'client-only' guard

ในทางกลับกัน ถ้า server component import module ที่มี `'client-only'`:

```tsx
// lib/browser-utils.ts
import 'client-only'

export const getWindowSize = () => ({
  width: window.innerWidth,
  height: window.innerHeight,
})
```

```tsx
// server component
import { getWindowSize } from '../lib/browser-utils'
// → RUV1009: Client-only module imported into server graph
```

---

## Hydration และ Zero-JS Pages

### Hydration scheduling

ควบคุมเวลา hydration ของ client components:

```tsx
'use client'

export const hydrate = 'idle' // 'load' | 'idle' | 'visible' | false
```

| ค่า                | ค่าใน source                       | Behavior                                           |
| ------------------ | ---------------------------------- | -------------------------------------------------- |
| `'load'` (default) | `export const hydrate = 'load'`    | Hydrate ทันทีที่ document parser ถึง module        |
| `'idle'`           | `export const hydrate = 'idle'`    | Hydrate เมื่อ browser idle (`requestIdleCallback`) |
| `'visible'`        | `export const hydrate = 'visible'` | Hydrate เมื่อ component อยู่ใน viewport            |
| `false`            | `export const hydrate = false`     | ไม่ hydrate เลย — zero-JS                          |
| `'none'`           | `export const hydrate = 'none'`    | เหมือน false                                       |

### HydrationMode enum

ใน `crates/ruvyxa_graph/src/lib.rs:1337-1363`:

```rust
fn parse_hydration_mode(source: &str) -> HydrationMode {
    // ค้นหา "export const hydrate"
    // อ่านค่าหลัง =
    // match:
    //   "false" | "none" → HydrationMode::None
    //   "idle" → HydrationMode::Idle
    //   "visible" → HydrationMode::Visible
    //   default → HydrationMode::Load
}
```

### Zero-JS Pages

```tsx
export const hydrate = false

export default function StaticPage() {
  return <p>หน้านี้ไม่มี JavaScript เลย — เร็วสุดๆ</p>
}
```

### ใช้กับ

- Landing page
- Marketing site
- Blog content
- หน้า static ที่ไม่ต้อง interactive

### Hydrate prop behavior

```tsx
// app/page.tsx — hydrate = false, ไม่มี JS
export const hydrate = false

// ถึงแม้จะมี 'use client' component อยู่
// ถ้า hydrate = false → component จะไม่ถูก hydrate
```

**ข้อควรระวัง**: `export const hydrate` บน route = opt-out การส่ง client bundle ถ้ามี `'use client'`
component ใน route นั้น component จะไม่ถูก hydrate

---

## Boundary Validation Error Codes

Ruvyxa ตรวจสอบ server/client boundary โดยอัตโนมัติตอน check/build:

| Code      | ความหมาย                    | สาเหตุ                              | วิธีแก้                                           |
| --------- | --------------------------- | ----------------------------------- | ------------------------------------------------- |
| `RUV1007` | Client boundary violation   | import server-only module ใน client | ย้าย import ไป server component หรือใช้ API route |
| `RUV1008` | Private env in client graph | ใช้ `process.env.PRIVATE` ใน client | ใช้ `RUVYXA_PUBLIC_*` หรือย้าย env read           |
| `RUV1009` | Client-only in server graph | import `client-only` ใน server      | ย้าย browser-only code ไป client module           |
| `RUV1010` | Server directory in client  | import จาก `server/` directory      | ย้าย shared code ไว้นอก `server/`                 |

### Error example: RUV1007

```
RUV1007: Server-only module imported into client graph

This module is reachable from a hydrated page or client module
but declares 'server-only'.

File: app/Counter.tsx:3
  import 'server-only'

Suggestion: Move server-only work behind a route handler/server
module and pass serializable data to the client.
```

### Error example: RUV1008

```
RUV1008: Private environment variable used in client graph

`process.env.DATABASE_URL` is reachable from browser code.
Only `RUVYXA_PUBLIC_*` env vars may be exposed to client modules.

File: app/page.tsx:5
  const dbUrl = process.env.DATABASE_URL

Suggestion: Move the env read into server-only code or rename
it to `RUVYXA_PUBLIC_*` if it is safe to expose.
```

### Error example: RUV1009

```
RUV1009: Client-only module imported into server graph

This module is reachable from server runtime code but
declares 'client-only'.

File: app/api/route.ts:3
  import { getWindowSize } from './browser-utils'

Suggestion: Move browser-only code into a client component
or client.tsx module.
```

### Error example: RUV1010

```
RUV1010: Server directory module reached by client graph

Files under server/ are reserved for server-only code.

File: app/ClientComponent.tsx:4
  import { config } from './lib/server/config'

Suggestion: Move shared browser-safe code outside server/,
or import it from a server route only.
```

---

## Module Graph Collection

Ruvyxa ใช้ `collect_relative_graph()` เพื่อหา dependency tree ทั้งหมด:

### อัลกอริทึม

ใน `crates/ruvyxa_graph/src/lib.rs:532-559`:

```
1. เริ่มจาก entry file
2. Normalize path (canonical)
3. BFS traversal:
   a. Mark current file as visited
   b. อ่าน source code
   c. หา import specifiers ทั้งหมด
   d. กรองเฉพาะ relative imports (ขึ้นต้นด้วย .)
   e. resolve relative import → full path
   f. ถ้ายังไม่เคย visit → ใส่ queue
4. Return set ของทุกไฟล์ที่ reachable
```

### Resolver: resolve_relative_import

ใน `crates/ruvyxa_graph/src/lib.rs:651-673`:

```
% ลำดับการ resolve:
1. exact path
2. + .ts
3. + .tsx
4. + .js
5. + .jsx
6. + .md
7. + .mdx
8. /index.ts
9. /index.tsx
10. /index.js
11. /index.jsx
12. /index.md
13. /index.mdx
```

---

## Module-level Exports ที่ถูกต้อง

### Server component

```tsx
export default function Page(props: PageProps): React.ReactElement
export async function Page(props: PageProps): Promise<React.ReactElement>
export const meta: Meta | MetaFactory
export const revalidate: number // ISR TTL
export const ppr: boolean // PPR opt-in
export function getStaticParams(): StaticParamsResult
export const hydrate: 'load' | 'idle' | 'visible' | false
```

### Client component

```tsx
'use client'
export default function Page(props: PageProps): React.ReactElement
export const meta: Meta | MetaFactory
export const hydrate: 'load' | 'idle' | 'visible' | false
```

### API route

```ts
export function GET(request: Request, context: { params: RouteParams }): Promise<Response>
export function POST(request: Request, context: { params: RouteParams }): Promise<Response>
export function PUT(request: Request, context: { params: RouteParams }): Promise<Response>
export function DELETE(request: Request, context: { params: RouteParams }): Promise<Response>
export function PATCH(request: Request, context: { params: RouteParams }): Promise<Response>
```

---

## Best Practices

```
┌────────────────────────────────────────────────┐
│              Decision Tree                      │
│                                                │
│  ต้องการ interactivity หรือไม่?                │
│         │              │                       │
│        ใช่             ไม่ใช่                  │
│         │              │                       │
│         ▼              ▼                       │
│  'use client'    Server component              │
│  + hooks         (default)                     │
│  + events                                      │
│  + effects        ✅ เล็ก, เร็ว, ไม่มี JS เกิน │
│                                                 │
│  ⚠ แต่ JS ถูกส่งไป browser                    │
└────────────────────────────────────────────────┘
```

### ข้อควรจำ

1. **Server component = default** — ใช้ไปก่อน เติม `'use client'` เมื่อจำเป็นเท่านั้น
2. **แยก client logic** — component เล็กๆ ที่ interactive ไว้ในไฟล์แยก `Counter.tsx`, `Form.tsx`
3. **Children pattern** — ส่ง server JSX เป็น children ให้ client component เพื่อลด client bundle
4. **ไม่ต้อง 'use client' ทุกไฟล์** — มีแต่ไฟล์ที่ใช้ hooks, events, state จริงๆ
5. **Server components สามารถ async** — `async function Page()` ใช้ `await` ได้เลย
6. **Client components ต้อง sync** — `useEffect` สำหรับ side effects
7. **Private env ต้อง server-only** — ขึ้นต้นด้วย `RUVYXA_PUBLIC_` สำหรับ client-safe
8. **Props ต้อง JSON** — function, Date, RegExp ส่งข้าม boundary ไม่ได้

### ลองทำดู

```tsx
// สร้างโปรเจคใหม่แล้วลอง:
// app/page.tsx
import ServerMessage from './ServerMessage'
import ClientCounter from './ClientCounter'

export default function HomePage() {
  return (
    <div>
      <ServerMessage /> {/* ไม่มี JS ใน browser */}
      <ClientCounter /> {/* มี JS เฉพาะปุ่มนี้ */}
    </div>
  )
}
```

```tsx
'use client'
// app/ClientCounter.tsx
import { useState } from 'react'

export default function ClientCounter() {
  const [count, setCount] = useState(0)
  return (
    <div>
      <p>นับ: {count}</p>
      <button onClick={() => setCount((c) => c + 1)}>+1</button>
    </div>
  )
}
```

```tsx
// app/ServerMessage.tsx — ไม่มี 'use client'
export default function ServerMessage() {
  return <p>ข้อความนี้มาจาก server ไม่มี JavaScript เลย</p>
}
```

---

## การแก้ไขปัญหาที่พบบ่อย

### "RUV1008: Server-only hook"

```tsx
// ผิด:
export default function Page() {
  const [x, setX] = useState(0) // ← RUV1008
  return <button onClick={() => setX(1)}>...</button>
}

// ถูก: เพิ่ม 'use client'
;('use client')
export default function Page() {
  const [x, setX] = useState(0)
  return <button onClick={() => setX(1)}>...</button>
}
```

### "RUV1007: Client boundary violation"

```tsx
'use client'
import { db } from './database' // ← database ใช้ 'server-only'
// → RUV1007

// วิธีแก้:
// 1. สร้าง API route ที่เรียก database
// 2. client component fetch จาก API นั้น
```

```tsx
// วิธีแก้: สร้าง API route
// app/api/users/route.ts
import { db } from '../database' // server-only OK ใน route.ts

export async function GET() {
  const users = await db.query('SELECT * FROM users')
  return Response.json(users)
}

// client component
;('use client')
export default function UsersList() {
  const [users, setUsers] = useState([])

  useEffect(() => {
    fetch('/api/users')
      .then((r) => r.json())
      .then(setUsers)
  }, [])

  return (
    <div>
      {users.map((u) => (
        <p key={u.id}>{u.name}</p>
      ))}
    </div>
  )
}
```

### Client component import server component ไม่ได้

```tsx
// ผิด:
'use client'
import ServerComponent from './ServerComponent' // ← import ได้ แต่...
// ServerComponent จะถูก bundle เป็น client ด้วย → อาจ error

// ถูก: ส่งเป็น children
;('use client')
export default function Client({ children }: { children: React.ReactNode }) {
  return <div className="card">{children}</div>
}
```

### onClick ไม่ทำงาน

```tsx
// ผิด: (server component)
export default function Page() {
  return <button onClick={() => alert('สวัสดี')}>คลิก</button>
  // JS ไม่ถูกส่งไป browser, onClick หาย
}

// ถูก:
;('use client')
export default function Page() {
  return <button onClick={() => alert('สวัสดี')}>คลิก</button>
}
```

### Client component async ไม่ได้

```tsx
// ผิด:
'use client'
export default async function Page() {
  // client component cannot be async
}

// ถูก:
;('use client')
export default function Page() {
  const [data, setData] = useState(null)
  useEffect(() => {
    fetchData().then(setData)
  }, [])
}
```

---

## Boundary Validation ดูจาก Reachability

สิ่งที่สำคัญไม่ใช่ชื่อไฟล์ แต่คือ module นั้นถูกเข้าถึงจาก hydrated page/client graph หรือจาก server
runtime หรือไม่ Route validation เดินตาม relative imports ด้วย parser เดียวกับที่ bundler ใช้ แล้ว
รายงาน boundary diagnostics ที่สำคัญดังนี้:

| Code      | สิ่งที่พบ                                                                                 | แนวทางที่ถูกต้อง                                                                            |
| --------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `RUV1007` | module ที่ client เข้าถึงได้ import `server-only`, `@ruvyxa/auth` หรือ `@ruvyxa/database` | ย้ายงาน server ไปอยู่หลัง loader, API route หรือ action แล้วส่งเฉพาะข้อมูลที่ serialize ได้ |
| `RUV1008` | `process.env.NAME` ที่ไม่ public ไปถึง client code                                        | เก็บไว้ฝั่ง server หรือใช้ `RUVYXA_PUBLIC_` เฉพาะค่าที่ตั้งใจเปิดเผย                        |
| `RUV1010` | module ใต้ directory `server/` ของ project ถูก client เข้าถึง                             | ย้าย helper ที่ browser ใช้ได้ออกนอก `server/`                                              |
| `RUV1009` | module ฝั่ง server import `client-only`                                                   | ย้าย browser-only code ไปยัง client module/entry                                            |

ตัวอย่างนี้เก็บ environment read ไว้ใน server-only module และคืนเฉพาะข้อมูลที่ UI ต้องใช้:

```ts
// app/server/catalog.ts
import 'server-only'

export async function loadCatalog() {
  const endpoint = process.env.CATALOG_API_URL
  if (!endpoint) throw new Error('CATALOG_API_URL is required')
  return fetch(endpoint).then((response) => response.json())
}
```

```tsx
// app/page.tsx
import { loadCatalog } from './server/catalog'
import { CatalogList } from './components/CatalogList'

export default async function Page() {
  return <CatalogList items={await loadCatalog()} />
}
```

page สามารถ render data นี้บน server ได้เอง; มีเพียง module ที่ต้องทำงานใน browser
เท่านั้นที่ควรเพิ่ม `'use client'` ค่า environment แบบ public อ่านผ่าน
`process.env.RUVYXA_PUBLIC_NAME` ไม่ควรสมมติว่า มี compatibility layer แบบ `import.meta.env`

### ลำดับ Debug Boundary

เมื่อ build หรือ check รายงาน boundary diagnostic อย่าเพิ่งย้าย directive จนกว่าจะเข้าใจ import
path:

```bash
ruvyxa analyze --format human
ruvyxa trace /the-affected-route
npm run check
```

เริ่มจากไฟล์ที่ diagnostic ระบุ แล้วตรวจ relative imports ของไฟล์นั้น Analyzer cache import edges
เหล่านี้ ข้าม routes ได้ แต่ผลยังถูกประเมินให้ทุก route ที่เข้าถึง module นั้น จึงควรระวังเมื่อย้าย
shared helper

---

## ขั้นตอนถัดไป

- **[04-rendering-strategies.md](./04-rendering-strategies.md)** — SSR, SSG, ISR, PPR, CSR
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — โหลดข้อมูลและ cache
