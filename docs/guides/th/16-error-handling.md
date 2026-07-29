# การจัดการ Error ใน Ruvyxa

Ruvyxa ใช้ระบบ error codes แบบ `RUV####` เพื่อให้นักพัฒนาแก้ไขปัญหาได้รวดเร็ว error ทุกตัวมี code,
ชื่อ, คำอธิบาย, error text ที่แสดง, และวิธีแก้ไขที่ชัดเจน

ระบบ error codes ครอบคลุมตั้งแต่ build-time (config, compilation, boundary) ไปจนถึง runtime (server,
worker, plugin, deploy)

---

## Error Code ช่วง

```
RUV1000-1099  →  Boundary errors     — server/client boundary, env, hooks
RUV1100-1199  →  Route errors        — ambiguous routes, parameters, conflicts
RUV1200-1299  →  Config errors       — validation, unknown fields, type mismatch
RUV1300-1399  →  Build errors        — compilation, resolution, bundle, timeout
RUV1400-1499  →  Server errors       — runtime, worker pool, cache, actions, API
RUV1500-1599  →  Worker errors       — crash, timeout, protocol, init, socket
RUV1600-1699  →  Plugin errors       — config invalid, out of range, not found, hook
RUV1700-1799  →  Deploy errors       — adapter, bundle budget, artifacts, incompatibility
```

---

## RUV1000-1099: Boundary Errors

ข้อผิดพลาดเกี่ยวกับ server/client boundary violation — ป้องกันไม่ให้ server code รั่วไหลไปยัง client

### RUV1000: Missing Use Client Directive

```tsx
// app/page.tsx
import { useState } from 'react' // RUV1000

export default function Page() {
  const [count, setCount] = useState(0) // ต้องใช้ 'use client'
  return <div>{count}</div>
}
```

**Title**: Missing `'use client'` directive

**คำอธิบาย**: Server component ใช้ React hooks (`useState`, `useEffect`, `useContext` ฯลฯ) โดยไม่มี
`'use client'` directive ที่ด้านบนของไฟล์

**Error text**: `RUV1000: Component uses React hooks but is missing 'use client' directive`

**วิธีแก้**: เพิ่ม `'use client'` เป็นบรรทัดแรกของไฟล์

```tsx
'use client' // ← เพิ่มตรงนี้

import { useState } from 'react'
// ...
```

### RUV1001: Private Import in Client Component

```tsx
'use client'

// app/page.tsx
import { serverOnlyFunction } from './server-only' // RUV1001
```

**Title**: Private server import in client component

**คำอธิบาย**: Client component import ไฟล์ที่ใช้ server-only code (database, env, server modules)

**Error text**: `RUV1001: Client component imports server-only module: <path>`

**Edge cases**:

- Import chain: page.tsx → utils/helpers.ts → server/db.ts — Ruvyxa ตรวจทั้ง chain
- Dynamic import (`import()`) ก็โดนตรวจ
- Re-export (`export from`) ก็โดน

**วิธีแก้**:

- ย้าย import ที่ผิดออกไป
- ใช้ server action (`'use server'`) แทน direct import
- ถ้าต้องการ import จริงๆ — แยก server logic ไว้ใน `server/` directory

### RUV1002: Client Hook in Server Component

```tsx
// app/page.tsx (server component — ไม่มี 'use client')
import { useState } from 'react' // RUV1002
```

**Title**: Client hook used in server component

**คำอธิบาย**: Server component ใช้ React hooks ที่ต้องทำงานใน client

**Error text**: `RUV1002: Client hook {hookName} cannot be used in a server component`

**วิธีแก้**:

- เพิ่ม `'use client'` directive ที่ด้านบนของไฟล์
- หรือแยกส่วนที่ใช้ hooks ออกเป็น client component ย่อย

### RUV1003: Ambiguous Route

```
app/
  blog/
    page.tsx    → /blog
    index.tsx   → /blog  (ซ้ำกับ page.tsx)
```

**Title**: Ambiguous route path

**คำอธิบาย**: สองไฟล์ match URL เดียวกัน — Ruvyxa ไม่รู้ว่าจะใช้ไฟล์ไหน

**Error text**: `RUV1003: Ambiguous route: <path> matched by both <file1> and <file2>`

**Edge cases**:

- `page.tsx` + `index.tsx` ในโฟลเดอร์เดียวกัน
- `route.ts` + `page.tsx` ใน path เดียวกัน
- Group route ที่ซ้อนทับ

**วิธีแก้**: ลบ หรือเปลี่ยนชื่อไฟล์ที่ซ้ำ — เหลือเพียงไฟล์เดียว

### RUV1004: Duplicate Route Parameter

```
app/
  blog/
    [slug]/
      [slug]/
        page.tsx  → /blog/:slug/:slug (ซ้ำ)
```

**Title**: Duplicate route parameter name

**คำอธิบาย**: พารามิเตอร์ซ้ำใน path เดียวกัน (ชื่อเหมือนกันใน 2 segments)

**Error text**: `RUV1004: Duplicate route parameter: <param> used in both <segment1> and <segment2>`

**Edge cases**:

- `[slug]/[slug]` — ชื่อซ้ำใน parent-child
- `[...slug]/[slug]` — catch-all + ปกติ ซ้ำ
- `[slug]/[slug]/page.tsx` — ชื่อซ้ำ

**วิธีแก้**: เปลี่ยนชื่อพารามิเตอร์ให้ต่างกัน เช่น `[slug]/[postId]`

### RUV1005: Missing SEO Metadata

**Title**: Missing SEO metadata

**คำอธิบาย**: หน้าไม่มี `Meta` หรือ `Seo` component — อาจส่งผลต่อ SEO

**Error text**: `RUV1005: Route <path> is missing SEO metadata` (warning — ไม่ fail build)

**วิธีแก้**: เพิ่ม `<Meta>` component หรือ export metadata object:

```ts
// app/blog/[slug]/page.tsx
import { Meta } from '@ruvyxa/react';

export const metadata = {
  title: 'บทความ',
  description: 'คำอธิบาย',
};

// หรือ
export default function Page() {
  return (
    <>
      <Meta title="บทความ" description="คำอธิบาย" />
      <article>...</article>
    </>
  );
}
```

### RUV1006: Missing Layout

**Title**: Missing layout file

**คำอธิบาย**: ต้องการ layout แต่ไม่พบไฟล์ `layout.tsx` ในโฟลเดอร์นั้น (เช่น route group ที่พยายามใช้
layout)

**Error text**: `RUV1006: Required layout not found in <path>`

**วิธีแก้**: สร้าง `layout.tsx`:

```tsx
// app/(marketing)/layout.tsx
export default function MarketingLayout({ children }: { children: React.ReactNode }) {
  return <div className="marketing">{children}</div>
}
```

### RUV1007: Client Boundary Violation (Import Chain)

```tsx
// app/page.tsx → utils/api.ts → server/db.ts
//                                  ↑ server-only ไปถึง client
```

**Title**: Client boundary violation via import chain

**คำอธิบาย**: Import chain จาก client component ไปถึง server-only module — Ruvyxa ตรวจสอบ dependency
graph ทั้งหมด

**Error text**: `RUV1007: Client boundary violation: <client> → <intermediate> → <server-only>`

**Edge cases**:

- Chain ยาว 10 ไฟล์ — Ruvyxa ตรวจทุก node
- Barrel imports (`index.ts` ที่ re-export)
- Dynamic imports (`const mod = await import('./server-only')`)

**วิธีแก้**:

- ใช้ `'use server'` action แทน direct import
- แยก server logic ไว้ใน `server/` directory ที่ Ruvyxa mark เป็น server-only
- ใช้ `ruvyxa check` เพื่อหา boundary violations ก่อน build

### RUV1008: Private Environment Variable in Client

```tsx
'use client'

// ❌
const dbUrl = process.env.DATABASE_URL // RUV1008
```

**Title**: Private environment variable exposed to client

**คำอธิบาย**: ตัวแปร environment ที่ไม่มี `RUVYXA_PUBLIC_` prefix ถูกใช้ใน client —
เสี่ยง泄露 secret

**Error text**: `RUV1008: Private environment variable <name> is exposed to client code`

**วิธีแก้**:

- เปลี่ยนเป็น `RUVYXA_PUBLIC_` prefix: `RUVYXA_PUBLIC_API_URL`
- หรือย้ายไป server side (`'use server'`, server action, API route)
- ใช้ `ruvyxa doctor` เช็ค env var security

```tsx
'use client'

// ✅ ปลอดภัย
const apiUrl = process.env.RUVYXA_PUBLIC_API_URL
```

### RUV1009: Server-only Hook in Server Component

```tsx
// app/page.tsx (server component)
export default function Page() {
  useEffect(() => {
    // RUV1009 — useEffect ต้องใช้ใน client
    console.log('mounted')
  }, [])
  return <div>Hello</div>
}
```

**Title**: Server-only hook used in component

**คำอธิบาย**: Server component ใช้ React hook ที่ต้องมี `'use client'`

**Error text**: `RUV1009: Hook <hookName> is not available in server components`

**Hooks ที่โดนตรวจ**: `useState`, `useEffect`, `useContext`, `useReducer`, `useCallback`, `useMemo`,
`useRef`, `useImperativeHandle`, `useLayoutEffect`, `useDebugValue`, `useDeferredValue`,
`useTransition`, `useSyncExternalStore`, `useId`

**วิธีแก้**: เพิ่ม `'use client'` directive หรือแยก client component

### RUV1010: Missing Server Action Directive

```ts
// app/actions.ts
export async function createUser(data: FormData) {
  'use server' // ต้องอยู่บรรทัดแรกของ function
}
```

**Title**: Missing `'use server'` directive in server action

**คำอธิบาย**: ฟังก์ชันที่ใช้ `action()` wrapper หรือเรียกจาก client ต้องมี `'use server'` directive

**Error text**: `RUV1010: Server action is missing 'use server' directive`

**วิธีแก้**: เพิ่ม `'use server'` เป็นบรรทัดแรกของ function หรือเป็นบรรทัดแรกของไฟล์

---

## RUV1100-1199: Route Errors

ข้อผิดพลาดเกี่ยวกับ route system — การค้นหาและ resolve route

### RUV1100: Route File Not Found

**Title**: Route file not found

**คำอธิบาย**: Ruvyxa ไม่พบ route file สำหรับ path ที่ร้องขอ — generic 404

**Error text**: `RUV1100: Route file not found for path <path>`

**วิธีแก้**: ตรวจสอบว่าไฟล์อยู่ใน `app/` directory ถูกต้อง

### RUV1101: Route Not Found (404)

**Title**: Route not found

**คำอธิบาย**: URL ไม่มี route ที่ตรงกัน — แสดง 404

**Error text**: `RUV1101: No matching route for URL <url>`

**วิธีแก้**: สร้าง `app/not-found.tsx` สำหรับกำหนดหน้า 404

```tsx
// app/not-found.tsx
import { Link } from '@ruvyxa/react'

export default function NotFound() {
  return (
    <main>
      <h1>404 — ไม่พบหน้า</h1>
      <p>หน้านี้ไม่มีอยู่ในระบบ</p>
      <Link href="/">กลับหน้าแรก</Link>
    </main>
  )
}
```

### RUV1102: Invalid Route Parameter

```
app/
  blog/
    [slug]/
      page.tsx  → /blog/:slug

แต่ slug มี `/` → /blog/a/b/c
```

**Title**: Invalid route parameter value

**คำอธิบาย**: Route parameter มีค่าที่ไม่ถูกต้อง — เช่น slug มี `/` ที่ทำให้ match หลาย segment

**Error text**: `RUV1102: Invalid parameter <param> value: <value> in route <route>`

**Edge cases**:

- `[slug]` มี `/` — ใช้ `[...slug]` แทน
- Parameter ไม่ตรง type (คาดหวัง number ได้ string)
- Parameter ว่าง

**วิธีแก้**:

- ใช้ catch-all route `[...slug]` แทนถ้าต้องการหลาย segment
- Validate parameter ใน page component
- ใช้ `generateStaticParams` เพื่อจำกัดค่าที่ถูกต้อง

### RUV1103: Static Path Conflict

**Title**: Static path conflicts with dynamic route

**คำอธิบาย**: เส้นทาง SSG (prerendered) ซ้อนทับกับ dynamic route

**Error text**: `RUV1103: Static path <path> conflicts with dynamic route <route>`

**ตัวอย่าง**:

```
app/
  blog/
    page.tsx        → /blog (static)
    [slug]/
      page.tsx      → /blog/:slug (dynamic)
    hello-world/
      page.tsx      → /blog/hello-world (conflict กับ [slug])
```

**วิธีแก้**: จัดลำดับ — static paths มี priority กว่า dynamic routes

### RUV1104: Page Not in Manifest

**Title**: Page missing from route manifest

**คำอธิบาย**: ไฟล์ใน `app/` ไม่ถูกเพิ่มใน route manifest — มักเกิดจาก cache เก่าหรือ build
ไม่สมบูรณ์

**Error text**: `RUV1104: File <path> is not included in route manifest`

**วิธีแก้**: รัน `ruvyxa clean && ruvyxa build` เพื่อ rebuild manifest

### RUV1105: API Route Conflict

**Title**: API route conflicts with page route

**คำอธิบาย**: API route (`app/api/users/route.ts`) ซ้อนทับกับ page route ใน path เดียวกัน

**Error text**: `RUV1105: API route <path> conflicts with page route <path>`

**วิธีแก้**: เปลี่ยน path ของ API route — ใช้ `/api/` prefix

### RUV1106: Route Group Misconfiguration

**Title**: Route group misconfiguration

**คำอธิบาย**: Route group (`(name)`) ไม่มี layout หรือใช้งานผิด

**Error text**: `RUV1106: Route group <group> misconfiguration: <detail>`

**วิธีแก้**: ตรวจสอบว่ามี `layout.tsx` ใน route group หรือไม่

### RUV1107: Interception Route Error

**Title**: Route interception error

**คำอธิบาย**: Route interception (parallel routes, intercepting routes) ผิดพลาด

**Error text**: `RUV1107: Route interception failed for <route>`

**วิธีแก้**: ตรวจสอบการตั้งค่า `(..)` หรือ `(...)` ใน route structure

---

## RUV1200-1299: Config Errors

ข้อผิดพลาดเกี่ยวกับ configuration — `ruvyxa.config.ts`

### RUV1200: Config Syntax Error

**Title**: Config file syntax error

**คำอธิบาย**: ไฟล์ config มี syntax error — parse ไม่ผ่าน

**Error text**: `RUV1200: Syntax error in config file: <detail>`

**วิธีแก้**: ตรวจ syntax — วงเล็บ, จุลภาค, เครื่องหมายคำพูด

### RUV1201: Config Load Failed

**Title**: Config file failed to load

**คำอธิบาย**: ไม่สามารถโหลด `ruvyxa.config.ts` — runtime error หรือ import ผิด

**Error text**: `RUV1201: Failed to load config file: <error>`

**สาเหตุทั่วไป**:

- Import path ผิด (`import { something } from 'wrong-package'`)
- Runtime error ใน config (`throw new Error('...')`)
- Circular dependency
- Module not found (`ts-node` หรือ `jiti` ไม่สามารถ resolve)

**วิธีแก้**: ตรวจ import, syntax, ติดตั้ง dependencies ที่จำเป็น

### RUV1202: Unknown Config Field

```ts
export default defineConfig({
  unknownField: true, // RUV1202
})
```

**Title**: Unknown configuration field

**คำอธิบาย**: ฟิลด์ที่ไม่มีใน schema ของ `defineConfig`

**Error text**: `RUV1202: Unknown config field: <field>`

**ค่าที่ถูกต้องทั้งหมด**:

```typescript
interface RuvyxaConfig {
  appDir?: string // default: 'app'
  output?: string // default: '.ruvyxa'
  adapter?: Adapter // 'vercel' | 'netlify' | 'cloudflare' | 'node' | 'bun' | 'static' | 'railway' | 'render' | 'firebase' | 'aws'
  runtime?: 'node' | 'bun' | 'workerd' | 'deno'
  target?: 'server' | 'serverless' | 'edge' | 'static'
  site?: SiteConfig
  security?: SecurityConfig
  images?: ImageConfig
  cache?: CacheConfig
  middleware?: MiddlewareConfig
  plugins?: PluginConfig[]
  debug?: DebugConfig
  css?: CSSConfig
  experimental?: Record<string, any>
}
```

**วิธีแก้**: ตรวจชื่อฟิลด์ใน TypeScript definition — ใช้ autocomplete จาก `defineConfig`

### RUV1203: Config Validation Error

```ts
export default defineConfig({
  appDir: '/absolute/path', // RUV1203 — ต้องเป็น relative path
})
```

**Title**: Configuration validation error

**คำอธิบาย**: ค่า config ไม่ผ่าน validation — เช่น path ผิด, ค่าไม่อยู่ใน range

**Error text**: `RUV1203: Validation error on field <field>: <detail>`

**Validations ที่ Ruvyxa ตรวจ**:

- `appDir`: ต้องเป็น relative path (ไม่เริ่มด้วย `/`)
- `adapter`: ต้องเป็นชื่อ adapter ที่รองรับ
- `site.url`: ต้องเป็น valid URL
- `middleware.workers`: 1-8
- `middleware.timeoutMs`: 1-300000
- `security.actionLimit`: 1-1048576
- `images.sizes`: แต่ละค่าต้อง between 32-4096

**วิธีแก้**: ดูรายละเอียด error และแก้ไขค่าที่ผิด

### RUV1204: Config Type Error

**Title**: Configuration type error

**คำอธิบาย**: ชนิดข้อมูลไม่ตรงกับ schema

**Error text**: `RUV1204: Type error on field <field>: expected <type>, got <type>`

**ตัวอย่าง**:

- `workers: '4'` → ต้องเป็น number
- `url: 123` → ต้องเป็น string
- `plugins: {...}` → ต้องเป็น array

**วิธีแก้**: แก้ไขชนิดข้อมูลให้ถูกต้อง — ใช้ TypeScript เพื่อ type checking

### RUV1205: Missing Config File

**Title**: Missing configuration file

**คำอธิบาย**: ไม่พบ `ruvyxa.config.ts` — Ruvyxa ใช้ default config แทน

**Error text**: `RUV1205: Config file not found, using defaults` (warning)

**วิธีแก้**: สร้างไฟล์ config ถ้าต้องการค่าเฉพาะ (deployment, plugins, security)

### RUV1206: Plugin Config Conflict

**Title**: Plugin configuration conflict

**คำอธิบาย**: สอง plugins มี config ที่ขัดแย้งกัน — เช่น กำหนดค่าเดียวกันทั้งคู่

**Error text**: `RUV1206: Plugin <a> and <b> have conflicting config for <field>`

**วิธีแก้**: รวม config หรือลบ plugin ที่ซ้ำซ้อน

### RUV1207: Environment Variable Validation Error

**Title**: Environment variable validation failed

**คำอธิบาย**: ตัวแปร environment ไม่ผ่าน validation — เช่น รูปแบบผิด, ค่าขาด

**Error text**: `RUV1207: Environment variable <name> validation failed: <detail>`

**วิธีแก้**: ตรวจว่าตัวแปรถูกตั้งค่าถูกต้อง — ใช้ `requireEnv` plugin

---

## RUV1300-1399: Build Errors

ข้อผิดพลาดระหว่าง build process — compilation, resolution, bundle

### RUV1300: Build Initialization Failed

**Title**: Build initialization failed

**คำอธิบาย**: ไม่สามารถเริ่มต้น build process — มักเกิดจาก dependency หาย

**Error text**: `RUV1300: Build initialization failed: <detail>`

**วิธีแก้**: ตรวจ dependencies, รัน `npm install`, ลบ node_modules แล้วติดตั้งใหม่

### RUV1301: Compilation Error

**Title**: TypeScript/JSX compilation error

**คำอธิบาย**: TypeScript หรือ JSX syntax error — transpile ไม่ผ่าน

**Error text**: `RUV1301: Compilation error in <file>:<line>:<column>: <message>`

**สาเหตุทั่วไป**:

- TypeScript type error
- JSX syntax ผิด
- Import ของไฟล์ที่ไม่มีอยู่
- Type mismatch

**วิธีแก้**: ดู stack trace — แก้ syntax error หรือ type error ที่ไฟล์นั้น

### RUV1302: Module Resolution Failed

**Title**: Module resolution failed

**คำอธิบาย**: ไม่พบ module ที่ import — import path ไม่ถูกต้องหรือไม่ได้ติดตั้ง

**Error text**: `RUV1302: Module <source> not found from <importer>`

**Edge cases**:

- npm package ไม่ได้ติดตั้ง — `npm install <package>`
- Path relative ผิด — ตรวจ `./` หรือ `../`
- Barrel export หาย — ตรวจ `index.ts` exports
- Alias ไม่ถูกต้อง — ตรวจ `alias` plugin
- Workspace protocol (`workspace:`) — ใช้ `"@ruvyxa/core": "workspace:*"` ใน dev

**วิธีแก้**: `npm install` หรือตรวจ path import ให้ถูกต้อง

### RUV1303: Bundle Failed

**Title**: Bundle process failed

**คำอธิบาย**: ไม่สามารถ bundle แอปพลิเคชัน — error ในระหว่าง tree-shaking, code-splitting, หรือ
minification

**Error text**: `RUV1303: Bundle failed: <detail>`

**สาเหตุทั่วไป**:

- Circular dependency
- Dynamic import ผิดพลาด
- Worker/bundle size เกิน limit
- Plugin transform ทำให้ bundle เสีย
- Side effect flag ผิด

**วิธีแก้**: ตรวจ dependencies, ใช้ `ruvyxa clean` แล้ว build ใหม่, ตรวจ circular dependency

### RUV1304: Image Optimization Failed

**Title**: Image optimization failed

**คำอธิบาย**: ไม่สามารถ optimize รูป — format ไม่รองรับ หรือไฟล์เสีย

**Error text**: `RUV1304: Image optimization failed for <path>: <detail>`

**Format ที่รองรับ**:

| Format | Optimize   | แปลงเป็น   |
| ------ | ---------- | ---------- |
| JPEG   | ✓          | WebP, AVIF |
| PNG    | ✓          | WebP, AVIF |
| GIF    | ✓ (static) | WebP       |
| SVG    | ✓ (minify) | —          |
| WebP   | ✓          | AVIF       |
| AVIF   | ✓          | —          |

**วิธีแก้**: ตรวจว่ารูปเสียหายหรือ format ไม่รองรับ — แปลงเป็น JPEG/PNG ก่อน

### RUV1305: Style Collection Failed

**Title**: CSS/Style collection failed

**คำอธิบาย**: ไม่สามารถ collect หรือ compile CSS — syntax error หรือ import ผิด

**Error text**: `RUV1305: Style collection failed: <detail>`

**สาเหตุทั่วไป**:

- CSS syntax error (`{` ไม่ปิด, `;` ขาด)
- PostCSS plugin error
- `@import` path ผิด
- TailwindCSS config error
- CSS Modules import ผิด

**วิธีแก้**: ตรวจ CSS syntax error หรือ import ที่ผิด

### RUV1306: Boundary Check Failed

**Title**: Server/client boundary check failed

**คำอธิบาย**: Server/client boundary violation ใน build time — รายละเอียดใน RUV1000-1010

**Error text**: `RUV1306: Boundary check failed: <count> violations found`

**วิธีแก้**: รัน `ruvyxa check` เพื่อดูรายละเอียด boundary violations

### RUV1307: Build Timeout

**Title**: Build process timed out

**คำอธิบาย**: Build ใช้เวลาเกินกำหนด — default timeout 300s (5 นาที)

**Error text**: `RUV1307: Build timed out after <duration>ms`

**สาเหตุทั่วไป**:

- Infinite loop ใน plugin hook
- Transform ไฟล์ใหญ่เกินไป (> 10MB)
- Image optimization รูปใหญ่เกินไป
- Module resolution ติด loop

**วิธีแก้**:

- เพิ่ม parallelism
- ลดขนาด bundle (exclude large deps)
- ตรวจ infinite loop ใน plugin
- ใช้ `RUVYXA_BUILD_TIMEOUT` env ตั้งค่า timeout

### RUV1308: Code Splitting Error

**Title**: Code splitting error

**คำอธิบาย**: Dynamic import (`import()`) ไม่สามารถ split ได้ — chunk ผิดพลาด

**Error text**: `RUV1308: Code splitting failed for <module>: <detail>`

**วิธีแก้**: ตรวจ dynamic import syntax, หลีกเลี่ยง dynamic import ที่มี expression

### RUV1309: Minification Error

**Title**: JavaScript/CSS minification error

**คำอธิบาย**: Minifier (SWC/Terser) ไม่สามารถ minify ไฟล์ได้

**Error text**: `RUV1309: Minification error in <file>: <detail>`

**วิธีแก้**: ตรวจ syntax, ปิด minification ชั่วคราวเพื่อ debug (`minify: false`)

---

## RUV1400-1499: Server Errors

ข้อผิดพลาด runtime บน server — production, dev, start

### RUV1400: Server Start Failed

**Title**: Server failed to start

**คำอธิบาย**: ไม่สามารถ start production server — port ถูกใช้ หรือ entry missing

**Error text**: `RUV1400: Server failed to start: <detail>`

**วิธีแก้**: ตรวจ port ไม่ซ้ำ, ตรวจ `build.json`, รัน `ruvyxa build` ก่อน `ruvyxa start`

### RUV1401: Runtime Error

**Title**: Server runtime error

**คำอธิบาย**: Unhandled exception ใน server runtime — error ที่ไม่ถูก try/catch

**Error text**: `RUV1401: Unhandled runtime error: <error>`

**Edge cases**:

- Async error ที่ไม่มี `.catch()`
- Error ใน `getServerSideProps`-equivalent
- Error ใน server component render
- Error ใน middleware

**วิธีแก้**: ดู stack trace ใน logs — เพิ่ม error boundary (`error.tsx`), ใช้ try/catch

### RUV1402: Worker Pool Exhausted

**Title**: Worker pool exhausted

**คำอธิบาย**: Worker processes ทั้งหมดกำลังทำงาน — ไม่มี worker ว่างให้ request ใหม่

**Error text**: `RUV1402: All <count> workers are busy, request <id> queued`

**สาเหตุทั่วไป**:

- Traffic spike
- Worker ตาย (OOM) และไม่ restart ทัน
- Request ใช้เวลานาน (> timeout)
- ตั้ง `middleware.workers` น้อยเกินไป

**วิธีแก้**:

- เพิ่ม `middleware.workers` ใน config (max 8)
- ลด request processing time
- เปิด load balancing (multiple instances)
- ตรวจสอบ memory leak

### RUV1403: Server Cache Error

**Title**: Server cache error

**คำอธิบาย**: Cache system error — ไม่สามารถ read/write cache

**Error text**: `RUV1403: Cache error: <detail>`

**วิธีแก้**: รัน `ruvyxa clean` ล้าง cache directory

### RUV1404: Action Execution Failed

```ts
'use server'
import { action } from 'ruvyxa/server'

export const doSomething = action(async () => {
  throw new Error('Something went wrong') // RUV1404
})
```

**Title**: Server action execution failed

**คำอธิบาย**: Server action (`'use server'`) throw error — unhandled exception ใน action function

**Error text**: `RUV1404: Server action <name> failed: <error>`

**Edge cases**:

- Validation error (input ไม่ถูก)
- Database error
- Authentication/authorization error
- Network error (external API)

**วิธีแก้**: จัดการ error ด้วย try/catch:

```ts
export const createUser = action(async (data: FormData) => {
  try {
    // validation
    if (!data.get('email')) {
      return { error: 'กรุณากรอกอีเมล', code: 'VALIDATION' }
    }
    // logic
    return { success: true }
  } catch (error) {
    console.error('Action error:', error)
    return { error: 'เกิดข้อผิดพลาด', code: 'RUV1404' }
  }
})
```

### RUV1405: API Route Error

```ts
// app/api/users/route.ts
export async function GET() {
  throw new Error('Database connection failed') // RUV1405
}
```

**Title**: API route handler error

**คำอธิบาย**: API route handler throw error — unhandled exception

**Error text**: `RUV1405: API route <path> handler failed: <error>`

**วิธีแก้**: เพิ่ม error handling ใน route handler:

```ts
export async function GET() {
  try {
    const users = await db.user.findMany()
    return Response.json({ users })
  } catch (error) {
    console.error('API error:', error)
    return Response.json({ error: 'RUV1405', message: 'Internal server error' }, { status: 500 })
  }
}
```

### RUV1406: Session Error

**Title**: Session validation error

**คำอธิบาย**: Session ไม่ถูกต้องหรือหมดอายุ — token expired, signature ผิด, หรือ user ถูกลบ

**Error text**: `RUV1406: Session invalid or expired: <detail>`

**สาเหตุ**:

- JWT หมดอายุ (`exp`)
- JWT signature ไม่ตรง
- Session ใน database ถูกลบ
- User ถูกลบจากระบบ
- Cookie ผิด

**วิธีแก้**: ให้ผู้ใช้ login ใหม่ — redirect ไป `/auth/login`

### RUV1407: Middleware Error

**Title**: Middleware execution error

**คำอธิบาย**: Middleware function throw error ระหว่าง request

**Error text**: `RUV1407: Middleware error in <file>:<line>: <detail>`

**วิธีแก้**: ตรวจ middleware code — ใช้ try/catch ใน middleware

### RUV1408: SSR Render Error

**Title**: SSR rendering error

**คำอธิบาย**: Server-side rendering ล้มเหลว — component error ระหว่าง render

**Error text**: `RUV1408: SSR render error for route <path>: <detail>`

**สาเหตุทั่วไป**:

- Component throw error
- Data fetching error
- React error boundary reached
- Memory limit (อย่าลืม `dangerouslySetInnerHTML`)

**วิธีแก้**: ตรวจ component, เพิ่ม error boundary, ใช้ `loading.tsx`

### RUV1409: Static Path Generation Error

**Title**: Static path generation error

**คำอธิบาย**: `generateStaticParams` ล้มเหลว — ไม่สามารถสร้าง static paths

**Error text**: `RUV1409: generateStaticParams failed for route <path>: <detail>`

**วิธีแก้**: ตรวจ `generateStaticParams` implementation — ตรวจ error ใน function

---

## RUV1500-1599: Worker Errors

ข้อผิดพลาดเกี่ยวกับ plugin worker process — crash, timeout, protocol

### RUV1500: Worker Pool Init Failed

**Title**: Worker pool initialization failed

**คำอธิบาย**: ไม่สามารถสร้าง worker pool — system resource ไม่พอ หรือ runtime ใช้งานไม่ได้

**Error text**: `RUV1500: Worker pool initialization failed: <detail>`

**วิธีแก้**: ตรวจ system resource (RAM), ตรวจ Node.js/Bun version

### RUV1501: Worker Crash

**Title**: Plugin worker process crashed

**คำอธิบาย**: Plugin worker process (Node.js/Bun) หยุดทำงานกะทันหัน — uncaught exception, OOM, หรือ
signal death

**Error text**: `RUV1501: Worker <id> crashed with signal <signal>: <detail>`

**Edge cases**:

- OOM (Out of Memory) — worker ใช้ RAM เกิน limit
- Segmentation fault — native module ปัญหา
- `process.exit()` ใน plugin code
- Unhandled promise rejection

**วิธีแก้**:

- ตรวจ plugin code — ใช้ try/catch รอบทุก hook
- ตรวจ memory usage — ลด plugin complexity
- ใช้ `workers: 1` เพื่อ debug
- เพิ่ม system memory

### RUV1502: Worker Timeout

**Title**: Plugin worker operation timed out

**คำอธิบาย**: Plugin middleware/hook ใช้เวลาเกิน timeout — default 30s

**Error text**: `RUV1502: Worker <id> timed out after <timeout>ms on hook <hook>`

**Timeouts ตาม hook**:

| Hook                                      | Default Timeout | Configurable               |
| ----------------------------------------- | --------------- | -------------------------- |
| `resolveId` / `onResolve`                 | 5s              | ✗                          |
| `transform` / `onTransform`               | 30s             | ✗                          |
| `middleware` / `onRequest` / `onResponse` | 30s             | ✓ (`middleware.timeoutMs`) |
| `buildStart` / `onStart`                  | 30s             | ✗                          |
| `buildEnd` / `onComplete`                 | 30s             | ✗                          |
| `serverStart`                             | 30s             | ✗                          |
| `serverEnd`                               | 10s             | ✗                          |

**วิธีแก้**:

- เพิ่ม `middleware.timeoutMs` ใน config (max 300,000ms)
- Optimize plugin — ลด blocking operations
- ใช้ async/await ให้ถูกต้อง

### RUV1503: Worker Protocol Error

**Title**: Plugin worker protocol error

**คำอธิบาย**: Communication protocol ระหว่าง Rust server และ JS worker ผิดพลาด — message format,
serialization, หรือ version mismatch

**Error text**: `RUV1503: Worker protocol error: <detail>`

**สาเหตุทั่วไป**:

- Plugin version ไม่ compatible กับ Ruvyxa version
- Message size เกิน limit (default 1MB)
- JSON serialization ล้มเหลว (circular reference)
- Socket registry version mismatch
- Worker ส่ง response ผิด format

**วิธีแก้**: อัปเดต Ruvyxa version และ plugin version ให้ตรงกัน

### RUV1504: Worker Initialization Failed

**Title**: Worker process initialization failed

**คำอธิบาย**: ไม่สามารถ start worker process — runtime ไม่พร้อม

**Error text**: `RUV1504: Worker initialization failed: <detail>`

**สาเหตุ**:

- Node.js ไม่ติดตั้ง
- Bun ไม่ติดตั้ง (แต่ config ใช้ `runtime: 'bun'`)
- Node.js version < 18
- Plugin path ไม่ถูกต้อง
- `node_modules` ขาด

**วิธีแก้**: ตรวจว่าระบบมี Node.js 22+ หรือ Bun ติดตั้ง — `node --version`, `bun --version`

### RUV1510: Socket Registry Connection Failed

**Title**: Socket registry connection failed

**คำอธิบาย**: ไม่สามารถเชื่อมต่อ socket registry ระหว่าง Rust และ JS worker

**Error text**: `RUV1510: Socket registry connection failed: <detail>`

**วิธีแก้**: รัน `ruvyxa clean && ruvyxa dev` ใหม่

### RUV1511: Socket Registry Timeout

**Title**: Socket registry operation timed out

**คำอธิบาย**: Socket registry ไม่ตอบกลับภายใน timeout — worker อาจจะ busy หรือ dead

**Error text**: `RUV1511: Socket registry timeout after <timeout>ms on <operation>`

**วิธีแก้**: เพิ่ม timeout, ตรวจ worker health

### RUV1512: Socket Registry Message Too Large

**Title**: Socket registry message exceeds size limit

**คำอธิบาย**: Message ที่ส่งระหว่าง Rust ↔ JS worker มีขนาดเกิน limit — default 1MB

**Error text**: `RUV1512: Socket message size <size> exceeds limit <limit>`

**วิธีแก้**: ลดขนาด message payload — หรือ split เป็น chunks

### RUV1513: Socket Registry Queue Full

**Title**: Socket registry message queue full

**คำอธิบาย**: Queue ของ pending messages เต็ม — worker รับไม่ทัน

**Error text**: `RUV1513: Socket registry queue full: <count> pending messages`

**วิธีแก้**: เพิ่ม worker count, ลด frequency ของ messages

---

## RUV1600-1699: Plugin Errors

ข้อผิดพลาดเกี่ยวกับ plugin system — config, not found, hook failure

### RUV1600: Plugin Registration Failed

**Title**: Plugin registration failed

**คำอธิบาย**: ไม่สามารถลงทะเบียน plugin — error ใน plugin constructor หรือ factory

**Error text**: `RUV1600: Plugin <name> registration failed: <detail>`

**วิธีแก้**: ตรวจ plugin code — constructor/factory throw error?

### RUV1601: Plugin Config Invalid

**Title**: Plugin configuration invalid

**คำอธิบาย**: ค่าใน plugin options ไม่ถูกต้อง — เช่น ค่า = 0, field ว่าง, type ผิด

**Error text**: `RUV1601: Plugin <name> config invalid on field <field>: <detail>`

**ตัวอย่าง**:

- `workers: 0` → ต้อง ≥ 1
- `timeoutMs: -1` → ต้อง ≥ 1
- `redirects: "string"` → ต้องเป็น array
- Plugin name ว่าง → ต้องมี name

**วิธีแก้**: ตั้งค่าให้ถูกต้องตาม schema ของ plugin

### RUV1602: Plugin Config Out of Range

**Title**: Plugin configuration out of range

**คำอธิบาย**: ค่าเกินขีดจำกัดที่ Ruvyxa อนุญาต

**Error text**:
`RUV1602: Plugin <name> config field <field> value <value> out of range [<min>, <max>]`

**Range สำหรับทุก field**:

| Field                     | ขั้นต่ำ | สูงสุด                | Default             |
| ------------------------- | ------- | --------------------- | ------------------- |
| `middleware.workers`      | 1       | 8                     | 1                   |
| `middleware.timeoutMs`    | 1       | 300,000               | 30,000              |
| `middleware.pluginLimit`  | 1       | 268,435,456 (256 MiB) | 33,554,432 (32 MiB) |
| `security.actionLimit`    | 1       | 1,048,576 (1 MiB)     | 262,144 (256 KiB)   |
| `security.apiLimit`       | 1       | 5,242,880 (5 MiB)     | 1,048,576 (1 MiB)   |
| `security.pluginLimit`    | 1       | 5,242,880 (5 MiB)     | 1,048,576 (1 MiB)   |
| `cache.ssr.ttl`           | 0       | 86,400 (1 วัน)        | 60                  |
| `cache.images.ttl`        | 0       | 3,153,6000 (1 ปี)     | 86,400              |
| `images.sizes` (per size) | 32      | 4,096                 | —                   |

**วิธีแก้**: ปรับค่าให้อยู่ในช่วงที่อนุญาต

### RUV1603: Plugin Not Found

**Title**: Plugin not found

**คำอธิบาย**: ไม่พบ plugin package ใน node_modules — Ruvyxa มองหา `ruvyxa-plugin-<name>`
หรือชื่อที่ระบุ

**Error text**: `RUV1603: Plugin <name> not found — searched in <paths>`

**วิธีแก้**:

```bash
# ตรวจว่าติดตั้งหรือยัง
npm ls ruvyxa-plugin-<name>

# ติดตั้ง
npm install ruvyxa-plugin-<name>

# หรือใช้ import โดยตรง
import myPlugin from 'ruvyxa-plugin-my-plugin';
// แทน name-based
```

### RUV1604: Plugin Hook Failure

```ts
hooks: {
  buildStart() {
    throw new Error('Plugin failed'); // RUV1604
  },
}
```

**Title**: Plugin hook execution failed

**คำอธิบาย**: Plugin hook throw error — unhandled exception ใน hook function

**Error text**: `RUV1604: Plugin <name> hook <hook> failed: <error>`

**Hooks ที่โดนตรวจ**: `onStart`, `onResolve`, `onTransform`, `onComplete`, `onRequest`,
`onResponse`, `resolveId`, `transform`, `buildStart`, `buildEnd`, `serverStart`, `serverEnd`,
`middleware`

**วิธีแก้**: ตรวจ plugin code — ใช้ try/catch ใน hook

```ts
hooks: {
  buildStart(ctx) {
    try {
      doRiskyOperation();
    } catch (e) {
      console.error('Plugin hook failed:', e);
      // Don't throw — or throw with proper code
    }
  },
}
```

### RUV1605: Plugin Dependency Conflict

**Title**: Plugin dependency conflict

**คำอธิบาย**: สอง plugins ต้องการ dependency version ที่ขัดแย้งกัน

**Error text**: `RUV1605: Plugin <a> and <b> have conflicting dependency <dep>`

**วิธีแก้**: ใช้ `overrides` ใน `package.json` หรือหันไปใช้ plugin ที่ compatible กัน

### RUV1606: Plugin Circular Dependency

**Title**: Plugin circular dependency detected

**คำอธิบาย**: Plugins มี circular dependency — A เรียก B, B เรียก A

**Error text**: `RUV1606: Circular plugin dependency: <chain>`

**วิธีแก้**: จัดโครงสร้าง plugin ใหม่ — merge หรือแยก dependencies

---

## RUV1700-1799: Deploy Errors

ข้อผิดพลาดเกี่ยวกับ deployment — adapter, build artifacts, compatibility

### RUV1700: Adapter Not Found

**Title**: Deploy adapter not found

**คำอธิบาย**: ไม่พบ adapter package ที่จะแปลง output สำหรับ platform

**Error text**: `RUV1700: Adapter <name> not found — install @ruvyxa/adapter-<name>`

**วิธีแก้**: ติดตั้ง adapter:

```bash
# Adapter packages
npm install @ruvyxa/adapter-vercel
npm install @ruvyxa/adapter-netlify
npm install @ruvyxa/adapter-cloudflare
npm install @ruvyxa/adapter-node
npm install @ruvyxa/adapter-bun
npm install @ruvyxa/adapter-static
npm install @ruvyxa/adapter-railway
npm install @ruvyxa/adapter-render
npm install @ruvyxa/adapter-firebase
npm install @ruvyxa/adapter-aws
```

### RUV1701: Adapter Build Failed

**Title**: Adapter build transformation failed

**คำอธิบาย**: Adapter ไม่สามารถแปลง build output สำหรับ platform — output ไม่ compatible
หรือมีข้อจำกัด

**Error text**: `RUV1701: Adapter <name> build failed: <detail>`

**สาเหตุทั่วไป**:

- Output ขนาดเกิน platform limit (Vercel: 50MB, Cloudflare: 1MB)
- Native module ที่ platform ไม่รองรับ
- Runtime API ที่ platform ไม่มี (เช่น `fs` ใน edge)
- Node.js version ไม่ตรง

**วิธีแก้**: ตรวจ compatibility, ลอง adapter อื่น, optimize output

### RUV1702: Bundle Budget Exceeded

**Title**: Bundle size exceeded budget

**คำอธิบาย**: Bundle size เกิน budget ที่กำหนดใน `bundleBudget` plugin

**Error text**: `RUV1702: Bundle budget exceeded: <type> <actual> > <limit>`

**วิธีแก้**:

- Tree shaking — ลบ unused exports
- Code splitting — ใช้ `import()` แทน static import
- ลด dependencies — ใช้ lightweight alternatives
- Optimize images — ลดขนาด, ใช้ WebP/AVIF
- ตรวจ bundle ด้วย `ruvyxa analyze`

### RUV1703: Missing Build Artifacts

**Title**: Required build artifacts are missing

**คำอธิบาย**: Build output ไม่สมบูรณ์ — ขาดไฟล์ที่จำเป็นสำหรับ deployment

**Error text**: `RUV1703: Missing required build artifact: <path>`

**วิธีแก้**: รัน `ruvyxa clean && ruvyxa build` ใหม่

### RUV1704: Adapter Incompatibility

**Title**: Adapter incompatible with project features

**คำอธิบาย**: Adapter ไม่รองรับฟีเจอร์ที่แอปใช้ — เช่น static adapter แต่แอป ใช้ server actions

**Error text**: `RUV1704: Adapter <name> does not support <feature>`

**Feature compatibility ตาราง**:

| Feature        | node | bun | vercel | netlify | cloudflare | static | railway | render | firebase | aws |
| -------------- | ---- | --- | ------ | ------- | ---------- | ------ | ------- | ------ | -------- | --- |
| SSR            | ✓    | ✓   | ✓      | ✓       | ✓          | ✗      | ✓       | ✓      | ✓        | ✓   |
| SSG            | ✓    | ✓   | ✓      | ✓       | ✓          | ✓      | ✓       | ✓      | ✓        | ✓   |
| ISR            | ✗    | ✗   | ✓      | ✓       | ✗          | ✗      | ✗       | ✗      | ✗        | ✓   |
| API Routes     | ✓    | ✓   | ✓      | ✓       | ✓          | ✗      | ✓       | ✓      | ✓        | ✓   |
| Server Actions | ✓    | ✓   | ✓      | ✓       | ✓          | ✗      | ✓       | ✓      | ✓        | ✓   |
| Middleware     | ✓    | ✓   | ✓      | ✓       | ✓          | ✗      | ✓       | ✓      | ✓        | ✓   |
| Image Opt      | ✓    | ✓   | ✓      | ✓       | ✓          | ✓      | ✓       | ✓      | ✗        | ✓   |
| Edge Functions | ✗    | ✗   | ✓      | ✓       | ✓          | ✗      | ✗       | ✗      | ✗        | ✓   |
| WebSocket      | ✓    | ✓   | ✗      | ✗       | ✓          | ✗      | ✓       | ✓      | ✗        | ✗   |

**วิธีแก้**: ใช้ adapter ที่รองรับฟีเจอร์นั้น หรือเปลี่ยนฟีเจอร์

### RUV1705: Missing Environment Variable for Deploy

**Title**: Required environment variable missing for deployment

**คำอธิบาย**: Environment variable ที่ adapter ต้องการสำหรับ deploy ไม่ได้ตั้งค่า

**Error text**: `RUV1705: Environment variable <name> is required for <adapter> deployment`

**วิธีแก้**: ตั้งค่า env var ใน platform dashboard หรือ CI/CD secrets

### RUV1706: Deploy Health Check Failed

**Title**: Deployment health check failed

**คำอธิบาย**: หลังจาก deploy แล้ว health endpoint ตอบกลับไม่สำเร็จ

**Error text**: `RUV1706: Deployment health check failed: <endpoint> returned <status>`

**วิธีแก้**: ตรวจ health endpoint, logs, environment variables

### RUV1707: Staging Swap Failed

**Title**: Blue-green staging swap failed

**คำอธิบาย**: การ swap จาก staging ไป production ล้มเหลว — staging build ไม่สมบูรณ์ หรือ health
check ไม่ผ่าน

**Error text**: `RUV1707: Staging swap failed: <detail>`

**วิธีแก้**: ตรวจ staging build, รัน `ruvyxa deploy:status`, rollback ถ้าจำเป็น

---

## Error Boundaries และ Special Pages — ละเอียด

### `error.tsx` — Error Boundary

ไฟล์ `error.tsx` ใช้ catch error ใน component tree และแสดง UI ทดแทน:

```tsx
'use client'

import { useEffect } from 'react'

export default function ErrorPage({
  error, // Error object
  reset, // ฟังก์ชัน reset — ลอง render ใหม่
}: {
  error: Error & { digest?: string }
  reset: () => void
}) {
  useEffect(() => {
    // Log error ไปยัง error tracking service
    console.error('Page error:', error)
    // เช่น Sentry.captureException(error);
  }, [error])

  return (
    <main
      style={{
        padding: '4rem',
        textAlign: 'center',
        maxWidth: '600px',
        margin: '0 auto',
      }}
    >
      <h1>เกิดข้อผิดพลาด</h1>
      <p style={{ color: '#666', margin: '1rem 0' }}>
        {error.message || 'ข้อผิดพลาดที่ไม่ทราบสาเหตุ'}
      </p>
      {error.digest && (
        <p style={{ fontSize: '0.8rem', color: '#999' }}>Error ID: {error.digest}</p>
      )}
      <div style={{ marginTop: '2rem', display: 'flex', gap: '1rem', justifyContent: 'center' }}>
        <button onClick={reset}>ลองอีกครั้ง</button>
        <a href="/">กลับหน้าแรก</a>
      </div>
    </main>
  )
}
```

**Error boundary hierarchy**:

```
app/error.tsx         → ระดับ root — ทุก route
app/blog/error.tsx    → เฉพาะ /blog/* รoutes
app/blog/[slug]/error.tsx → เฉพาะ /blog/:slug
```

**กฎ**:

- `error.tsx` ต้องมี `'use client'` (เพราะใช้ React state)
- Error boundary จะ catch error จาก child components ทั้งหมด
- ไม่ catch error ใน layout ที่อยู่ level เดียวกัน — ต้องมี error.tsx ที่ parent
- `reset()` ลอง re-render content — ไม่ reload หน้า

### `not-found.tsx` — 404 Page

```tsx
// app/not-found.tsx
import { Link } from '@ruvyxa/react'

export default function NotFoundPage() {
  return (
    <main style={{ textAlign: 'center', padding: '4rem' }}>
      <h1>404</h1>
      <p>ไม่พบหน้าที่คุณหา</p>
      <p>หน้านี้ถูกลบ ย้าย หรือไม่มีอยู่ในระบบ</p>
      <div style={{ marginTop: '2rem' }}>
        <Link href="/">กลับหน้าแรก</Link>
        {' | '}
        <Link href="/search">ค้นหา</Link>
      </div>
    </main>
  )
}
```

**Trigger not-found**:

- URL ไม่มี route ที่ match → auto 404
- `notFound()` function → แสดง not-found.tsx
- `app/not-found.tsx` ที่ root → 404 ทั้งแอป
- `app/blog/not-found.tsx` → 404 เฉพาะ /blog/*

```ts
// app/blog/[slug]/page.tsx
import { notFound } from 'ruvyxa/server';

export default async function BlogPost({ params }: { params: { slug: string } }) {
  const post = await db.post.findUnique({ where: { slug: params.slug } });

  if (!post) {
    notFound(); // → แสดง not-found.tsx ที่ใกล้ที่สุด
  }

  return <article>{post.title}</article>;
}
```

### `loading.tsx` — Loading State

```tsx
// app/loading.tsx
export default function LoadingPage() {
  return (
    <main style={{ textAlign: 'center', padding: '4rem' }}>
      <div
        className="spinner"
        style={{
          width: 40,
          height: 40,
          border: '4px solid #eee',
          borderTop: '4px solid #333',
          borderRadius: '50%',
          animation: 'spin 1s linear infinite',
          margin: '0 auto',
        }}
      />
      <p style={{ marginTop: '1rem', color: '#666' }}>กำลังโหลด...</p>

      <style>{`
        @keyframes spin {
          to { transform: rotate(360deg); }
        }
      `}</style>
    </main>
  )
}
```

**Loading hierarchy**:

```
app/loading.tsx           → ทุก route
app/blog/loading.tsx      → เฉพาะ /blog/*
app/dashboard/loading.tsx → เฉพาะ /dashboard/*
```

### Hierarchy — Full

```
📁 app/
├── layout.tsx              → root layout
├── page.tsx                → /
├── error.tsx               → error boundary (root)
├── not-found.tsx           → 404 (root)
├── loading.tsx             → loading (root)
│
├── blog/
│   ├── layout.tsx          → layout เฉพาะ blog
│   ├── page.tsx            → /blog
│   ├── error.tsx           → error เฉพาะ blog
│   ├── loading.tsx         → loading เฉพาะ blog
│   └── [slug]/
│       ├── page.tsx        → /blog/:slug
│       └── error.tsx       → error เฉพาะ /blog/:slug
│
├── dashboard/
│   ├── layout.tsx          → layout เฉพาะ dashboard
│   ├── page.tsx            → /dashboard
│   └── loading.tsx         → loading (dashboard)
│
└── api/
    └── users/
        └── route.ts        → /api/users (API — ไม่มี error UI)
```

**เมื่อ error เกิดขึ้น — ลำดับการหา error boundary**:

1. ไฟล์ `error.tsx` ในโฟลเดอร์เดียวกัน
2. ไฟล์ `error.tsx` ใน parent โฟลเดอร์
3. ไฟล์ `error.tsx` ที่ root (`app/error.tsx`)
4. ถ้าไม่มีเลย → Ruvyxa global error page

---

## Error Overlay — Dev Mode

เมื่อเกิด error ใน dev mode, Ruvyxa แสดง overlay ในเบราว์เซอร์:

```
┌────────────────────────────────────────────────────────────────┐
│  ⚠  Ruvyxa Error                                              │
│                                                                │
│  RUV1008: Private environment variable                         │
│                                                                │
│  ┌─── File ─────────────────────────────────────────────────┐  │
│  │  app/page.tsx:3:15                                        │  │
│  │  1 │ 'use client';                                        │  │
│  │  2 │                                                       │  │
│  │  3 │ const dbUrl = process.env.DATABASE_URL  ←─ error      │  │
│  │  4 │                                                       │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                │
│  Why:  DATABASE_URL is a private env var exposed to client     │
│                                                                │
│  Fix:  Prefix with RUVYXA_PUBLIC_ or move to server            │
│                                                                │
│  ┌────────────────────────────────────────────────────────┐    │
│  │  [Dismiss]    [Reload]    [Open in Editor]              │    │
│  └────────────────────────────────────────────────────────┘    │
└────────────────────────────────────────────────────────────────┘
```

**Features**:

- แสดง error code, file, line number, code context
- คำอธิบายสาเหตุ (`Why`) และวิธีแก้ (`Fix`)
- ปุ่ม Dismiss — ปิด overlay
- ปุ่ม Reload — refresh page
- ปุ่ม Open in Editor — เปิดไฟล์ใน IDE (ถ้าตั้ง `EDITOR` env)

**ปิด overlay**:

```ts
// ruvyxa.config.ts
debug: {
  overlay: false,     // ปิด error overlay
}
```

**Overlay จะแสดงเฉพาะใน dev mode** — production แสดง custom error page (`error.tsx`)

---

## Server Action Error Handling — แบบสมบูรณ์

```ts
'use server'
import { action } from 'ruvyxa/server'
import { RuvyxaError } from 'ruvyxa/errors'

// === Action ที่จัดการ error ครบถ้วน ===
export const createUser = action(async (data: FormData) => {
  try {
    // 1. Input validation
    const email = data.get('email') as string
    const name = data.get('name') as string

    if (!email || !email.includes('@')) {
      throw new RuvyxaError('RUV1404', 'กรุณากรอกอีเมลที่ถูกต้อง')
    }

    if (!name || name.length < 2) {
      throw new RuvyxaError('RUV1404', 'ชื่อต้องมีอย่างน้อย 2 ตัวอักษร')
    }

    // 2. Business logic
    const existing = await db.user.findUnique({ where: { email } })
    if (existing) {
      throw new RuvyxaError('RUV1404', 'อีเมลนี้มีผู้ใช้แล้ว')
    }

    const user = await db.user.create({
      data: { email, name },
      select: { id: true, email: true, name: true },
    })

    await sendWelcomeEmail(email, name)

    // 3. Success
    return { success: true, user }
  } catch (error) {
    // 4. Error handling
    if (error instanceof RuvyxaError) {
      return { error: error.message, code: error.code }
    }

    console.error('Unexpected error in createUser:', error)
    return {
      error: 'เกิดข้อผิดพลาดที่ไม่ทราบสาเหตุ',
      code: 'RUV1404',
    }
  }
})
```

**Client-side**:

```tsx
'use client'
import { createUser } from './actions'
import { useState } from 'react'

export function RegisterForm() {
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(formData: FormData) {
    setError(null)
    const result = await createUser(formData)

    if (result.error) {
      setError(`Error ${result.code}: ${result.error}`)
    } else {
      alert('สร้างผู้ใช้สำเร็จ!')
    }
  }

  return (
    <form action={handleSubmit}>
      {error && <div className="error-banner">⚠ {error}</div>}
      <input name="email" type="email" required placeholder="อีเมล" />
      <input name="name" required placeholder="ชื่อ" />
      <button type="submit">สมัครสมาชิก</button>
    </form>
  )
}
```

---

## API Route Error Handling — แบบสมบูรณ์

```ts
// app/api/users/route.ts
import { NextResponse } from 'ruvyxa/server'

// === GET /api/users ===
export async function GET(request: Request) {
  try {
    const { searchParams } = new URL(request.url)
    const limit = Math.min(Number(searchParams.get('limit')) || 10, 100)
    const offset = Number(searchParams.get('offset')) || 0

    const [users, total] = await Promise.all([
      db.user.findMany({ take: limit, skip: offset }),
      db.user.count(),
    ])

    return NextResponse.json({
      data: users,
      pagination: { total, limit, offset },
    })
  } catch (error) {
    console.error('GET /api/users error:', error)
    return NextResponse.json(
      {
        error: 'RUV1405',
        message: 'ไม่สามารถดึงข้อมูลผู้ใช้ได้',
        details:
          process.env.NODE_ENV === 'development'
            ? error instanceof Error
              ? error.message
              : undefined
            : undefined,
      },
      { status: 500 },
    )
  }
}

// === POST /api/users ===
export async function POST(request: Request) {
  try {
    const body = await request.json()

    // Validate
    if (!body.email || !body.name) {
      return NextResponse.json(
        { error: 'RUV1405', message: 'ข้อมูลไม่ครบ: email และ name' },
        { status: 400 },
      )
    }

    const user = await db.user.create({ data: body })
    return NextResponse.json({ user }, { status: 201 })
  } catch (error) {
    const isValidation = error instanceof SyntaxError
    const status = isValidation ? 400 : 500

    return NextResponse.json(
      {
        error: 'RUV1405',
        message: isValidation ? 'ข้อมูล JSON ไม่ถูกต้อง' : 'ไม่สามารถสร้างผู้ใช้ได้',
      },
      { status },
    )
  }
}
```

---

## การใช้ `notFound()` และ `redirect()`

```ts
// app/blog/[slug]/page.tsx
import { notFound, redirect } from 'ruvyxa/server';

export default async function BlogPost({ params }: { params: { slug: string } }) {
  const post = await db.post.findUnique({ where: { slug: params.slug } });

  if (!post) {
    notFound();              // → แสดง not-found.tsx (404)
  }

  if (post.redirectTo) {
    redirect(post.redirectTo, 301);  // → redirect (301 permanent)
  }

  if (!post.published) {
    if (!isAdmin) {
      redirect('/blog/drafts', 302);  // → redirect (302 temporary)
    }
  }

  return <article>{post.title}</article>;
}
```

**redirect status codes**:

- `redirect(path)` — 307 (temporary)
- `redirect(path, 301)` — 301 (permanent)
- `redirect(path, 302)` — 302 (found)
- `redirect(path, 308)` — 308 (permanent, preserve method)

---

## ดู Error Codes ทั้งหมด

```bash
# CLI tools สำหรับ debug
ruvyxa doctor            # ตรวจสอบทุกอย่าง — config, routes, boundary, env
ruvyxa doctor --verbose  # แสดงละเอียดทุก error
ruvyxa check             # เฉพาะ config + routes + boundary
ruvyxa analyze           # bundle analysis — dependencies, size
ruvyxa trace             # build trace — timing, dependencies
```

**Output `ruvyxa doctor`**:

```
━━━ Ruvyxa Doctor ━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Version:   0.1.0
  Node:      22.4.0
  Platform:  win32

  Config:
    ✓ ruvyxa.config.ts found
    ✓ All fields valid
    ✓ Site URL set

  Routes:
    ✓ 12 routes registered
    ✓ No ambiguous routes
    ✓ All pages have metadata

  Boundary:
    ✓ 0 violations (server → client)
    ✓ All server directives correct
    ✓ Private env vars not exposed

  Plugins:
    ✓ 3 plugins registered
    ✓ All plugin configs valid
    ✓ Adapter: vercel

  Ready for production ✓
```

---

## ตาราง Error Codes — ทุก code

| Code        | Title                             | ช่วง     | Severity |
| ----------- | --------------------------------- | -------- | -------- |
| **RUV1000** | Missing 'use client' directive    | Boundary | Error    |
| **RUV1001** | Private server import in client   | Boundary | Error    |
| **RUV1002** | Client hook in server component   | Boundary | Error    |
| **RUV1003** | Ambiguous route                   | Boundary | Error    |
| **RUV1004** | Duplicate route parameter         | Boundary | Error    |
| **RUV1005** | Missing SEO metadata              | Boundary | Warning  |
| **RUV1006** | Missing layout file               | Boundary | Warning  |
| **RUV1007** | Client boundary violation (chain) | Boundary | Error    |
| **RUV1008** | Private env var in client         | Boundary | Error    |
| **RUV1009** | Server-only hook in component     | Boundary | Error    |
| **RUV1010** | Missing server action directive   | Boundary | Error    |
| **RUV1100** | Route file not found              | Route    | Error    |
| **RUV1101** | Route not found (404)             | Route    | Info     |
| **RUV1102** | Invalid route parameter           | Route    | Error    |
| **RUV1103** | Static path conflict              | Route    | Warning  |
| **RUV1104** | Page not in manifest              | Route    | Error    |
| **RUV1105** | API route conflict                | Route    | Error    |
| **RUV1106** | Route group misconfiguration      | Route    | Warning  |
| **RUV1107** | Interception route error          | Route    | Error    |
| **RUV1200** | Config syntax error               | Config   | Error    |
| **RUV1201** | Config load failed                | Config   | Error    |
| **RUV1202** | Unknown config field              | Config   | Warning  |
| **RUV1203** | Config validation error           | Config   | Error    |
| **RUV1204** | Config type error                 | Config   | Error    |
| **RUV1205** | Missing config file               | Config   | Warning  |
| **RUV1206** | Plugin config conflict            | Config   | Error    |
| **RUV1207** | Env var validation error          | Config   | Error    |
| **RUV1300** | Build init failed                 | Build    | Error    |
| **RUV1301** | Compilation error                 | Build    | Error    |
| **RUV1302** | Module resolution failed          | Build    | Error    |
| **RUV1303** | Bundle failed                     | Build    | Error    |
| **RUV1304** | Image optimization failed         | Build    | Warning  |
| **RUV1305** | Style collection failed           | Build    | Error    |
| **RUV1306** | Boundary check failed             | Build    | Error    |
| **RUV1307** | Build timeout                     | Build    | Error    |
| **RUV1308** | Code splitting error              | Build    | Error    |
| **RUV1309** | Minification error                | Build    | Error    |
| **RUV1400** | Server start failed               | Server   | Error    |
| **RUV1401** | Runtime error                     | Server   | Error    |
| **RUV1402** | Worker pool exhausted             | Server   | Warning  |
| **RUV1403** | Server cache error                | Server   | Warning  |
| **RUV1404** | Action execution failed           | Server   | Error    |
| **RUV1405** | API route error                   | Server   | Error    |
| **RUV1406** | Session error                     | Server   | Warning  |
| **RUV1407** | Middleware error                  | Server   | Error    |
| **RUV1408** | SSR render error                  | Server   | Error    |
| **RUV1409** | Static path generation error      | Server   | Error    |
| **RUV1500** | Worker pool init failed           | Worker   | Error    |
| **RUV1501** | Worker crash                      | Worker   | Error    |
| **RUV1502** | Worker timeout                    | Worker   | Error    |
| **RUV1503** | Worker protocol error             | Worker   | Error    |
| **RUV1504** | Worker init failed                | Worker   | Error    |
| **RUV1510** | Socket registry connection failed | Worker   | Error    |
| **RUV1511** | Socket registry timeout           | Worker   | Error    |
| **RUV1512** | Socket registry message too large | Worker   | Error    |
| **RUV1513** | Socket registry queue full        | Worker   | Warning  |
| **RUV1600** | Plugin registration failed        | Plugin   | Error    |
| **RUV1601** | Plugin config invalid             | Plugin   | Error    |
| **RUV1602** | Plugin config out of range        | Plugin   | Error    |
| **RUV1603** | Plugin not found                  | Plugin   | Error    |
| **RUV1604** | Plugin hook failure               | Plugin   | Error    |
| **RUV1605** | Plugin dependency conflict        | Plugin   | Warning  |
| **RUV1606** | Plugin circular dependency        | Plugin   | Error    |
| **RUV1700** | Adapter not found                 | Deploy   | Error    |
| **RUV1701** | Adapter build failed              | Deploy   | Error    |
| **RUV1702** | Bundle budget exceeded            | Deploy   | Error    |
| **RUV1703** | Missing build artifacts           | Deploy   | Error    |
| **RUV1704** | Adapter incompatibility           | Deploy   | Error    |
| **RUV1705** | Missing env var for deploy        | Deploy   | Error    |
| **RUV1706** | Deploy health check failed        | Deploy   | Error    |
| **RUV1707** | Staging swap failed               | Deploy   | Error    |

---

## Troubleshooting — Quick Reference

| Error Code   | ปัญหาที่พบบ่อย      | วิธีแก้ด่วน                                           |
| ------------ | ------------------- | ----------------------------------------------------- |
| RUV1000-1010 | Boundary violations | ตรวจ `'use client'` และ imports                       |
| RUV1100-1105 | Route ไม่ match     | สร้าง `not-found.tsx`, ตรวจ route structure           |
| RUV1201-1205 | Config error        | ตรวจ `ruvyxa.config.ts` — syntax, import, field names |
| RUV1301-1304 | Build fails         | `ruvyxa clean && ruvyxa build` — ดู error message     |
| RUV1401-1406 | Runtime error       | ดู stack trace, เพิ่ม try/catch, ตรวจ env vars        |
| RUV1501-1504 | Worker failed       | รีสตาร์ท dev server, ตรวจ Node.js/Bun version         |
| RUV1510-1513 | Socket registry     | อัปเดต Ruvyxa, ตรวจ plugin compatibility              |
| RUV1600-1604 | Plugin config       | ตรวจ plugin options, range, npm install               |
| RUV1700-1705 | Deploy              | ติดตั้ง adapter, optimize bundle, ตรวจ artifacts      |

---

## ลองทำดู

1. สร้าง `app/error.tsx` พร้อม UI สวยงาม — แสดง error message, digest, reset button
2. สร้าง `app/not-found.tsx` พร้อมลิงก์กลับหน้าแรก และ search
3. สร้าง `app/loading.tsx` — spinner + skeleton
4. ทดลองสร้าง RUV1007 โดย import server module ใน client component — ดู error
5. ใช้ `notFound()` ใน dynamic route — ดู 404 page
6. จัดการ error ใน server action ด้วย try/catch — return error object
7. ใช้ `RuvyxaError` class ใน custom error
8. เปิด `debug.overlay: false` ถ้าไม่ต้องการ error overlay
9. รัน `ruvyxa doctor` — ดู error ทั้งหมดในแอป
10. ทดสอบ API route error handling — ส่ง POST ผิด format
11. ตรวจสอบ bundle budget — เพิ่ม `bundleBudget` plugin
12. ดู error overlay ใน dev mode — ทำ intentional error

---

## สรุป

- Error codes: RUV1000-1799 — แบ่งเป็น 8 ช่วง: boundary, route, config, build, server, worker,
  plugin, deploy
- รวม 65+ error codes แต่ละตัวมี: code, title, คำอธิบาย, error text, วิธีแก้
- Error boundary: `error.tsx` (catch errors), `not-found.tsx` (404), `loading.tsx` (loading state)
- Error overlay ใน dev mode — แสดง file, line, why, fix, dismiss, reload, open in editor
- Server actions และ API routes: จัดการด้วย try/catch + `RuvyxaError`
- `notFound()` และ `redirect()` (301, 302, 307, 308) สำหรับควบคุม flow
- `ruvyxa doctor` ตรวจทุกอย่าง — config, routes, boundary, env
- 2 ตาราง: error code ทั้งหมด + cross-reference
