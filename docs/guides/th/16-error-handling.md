# การจัดการ Error ใน Ruvyxa

Ruvyxa ใช้ diagnostics แบบ `RUV####` สำหรับความผิดพลาดของ framework หลายส่วน โดย diagnostic จะมี
code และ title เสมอ ส่วน file, คำอธิบาย, วิธีแก้ และ affected routes จะแสดงเมื่อ subsystem นั้นส่ง
ข้อมูลมาเท่านั้น; runtime exception ทั่วไปอาจเป็นข้อความปกติโดยไม่มี `Diagnostic` code

ระบบ error codes ที่บันทึกในหน้านี้ครอบคลุม code ที่ยืนยันจาก source ตั้งแต่ build-time (boundary,
config, compilation) ไปจนถึง runtime (server, worker และ plugin) ไม่ได้หมายความว่า runtime error
ทุกชนิดจะมี code สาธารณะถาวร

---

## Error Code ช่วง

```
RUV1001-1099  →  Boundary / Graph      — server/client boundary, env leak, route discovery
RUV1100-1199  →  SSR / Render           — React SSR, renderer discovery
RUV1200-1299  →  API / Server Runtime   — API route, port binding, renderer
RUV1300-1399  →  Bundle / Compilation   — hydration bundling, client route, MDX
RUV1400-1499  →  Style                  — Tailwind, Sass, CSS entries
RUV1500-1599  →  Worker / Static Params — render worker, actions, static params, PPR
RUV1600-1603  →  Config / Adapter       — config loading, validation, shape/limits, adapter build()
RUV1700-1799  →  Plugin Bridge          — plugin hook timeout, protocol, worker pool
RUV1800-1899  →  JS Runtime             — module resolution, Oxc transform, circular deps
RUV2000-2200  →  Adapter / Plugin Def   — BuildContext, options, definePlugin, build hook
RUV3000-3201  →  Official Packages      — database, auth, realtime
```

---

## RUV1001-1099: Boundary Violations

ข้อผิดพลาดเกี่ยวกับ server/client boundary violation — ป้องกันไม่ให้ server code รั่วไหลไปยัง client

### RUV1001: ไม่พบไดเรกทอรี app

**Title**: App directory not found

**คำอธิบาย**: Ruvyxa ไม่พบไดเรกทอรี `app/` ใน project root — ไม่สามารถค้นหา route ได้

**Error text**: `RUV1001: App directory not found at <path>`

**วิธีแก้**: สร้างไดเรกทอรี `app/` ใน project root หรือกำหนด `appDir` ใน config

### RUV1002: Segment route dynamic ไม่ถูกต้อง

**Title**: Invalid dynamic route segment

**คำอธิบาย**: ชื่อ segment dynamic ใน route ไม่ถูกต้อง — เช่น ใช้อักขระพิเศษหรือรูปแบบผิด

```
RUV1002: Invalid dynamic route segment

  Segment: [slug#bad]
  Route: /blog/[slug#bad]
  File: app/blog/[slug#bad]/page.tsx

  Fix: ใช้รูปแบบ [param], [...param], หรือ [[...param]] เท่านั้น
```

**วิธีแก้**: เปลี่ยนชื่อ segment ให้ใช้รูปแบบที่รองรับ: `[param]`, `[...param]`, `[[...param]]`

### RUV1003: เส้นทาง route ขัดแย้งกัน

**Title**: Ambiguous route path

**คำอธิบาย**: สองไฟล์ match URL shape เดียวกัน — Ruvyxa ไม่รู้ว่าจะใช้ไฟล์ไหน

```
RUV1003: Ambiguous route

  Route: /products/[id]
  Files:
    app/products/[id]/page.tsx
    app/products/[slug]/page.tsx

  Both files match the same URL pattern like /products/123.

  Fix: Remove one route or give one route a static discriminator,
       for example /products/by-slug/[slug]. Parameter names alone
       do not make dynamic route shapes distinct.
```

**Edge cases**:

- `page.tsx` + `index.tsx` ในโฟลเดอร์เดียวกัน
- Group route ที่ซ้อนทับ
- Static route + dynamic route ที่ path เดียวกัน

**วิธีแก้**: ลบ route ตัวหนึ่งออก หรือเพิ่ม static segment ให้ต่างกัน เช่น
`/products/by-slug/[slug]` การเปลี่ยนชื่อ `[id]` เป็น `[slug]` เพียงอย่างเดียวไม่ได้ทำให้ route
shape ต่างกัน

### RUV1004: Page ไม่มี default export

**Title**: Page missing default export

**คำอธิบาย**: ไฟล์ page ไม่มี `export default` component — Ruvyxa ไม่สามารถ render หน้านี้ได้

```
RUV1004: Page is missing a default export

  File: app/about/page.tsx

  Fix: Add `export default function Page() { ... }`
```

**วิธีแก้**: เพิ่ม default export component ในไฟล์ page

```tsx
// app/about/page.tsx
export default function AboutPage() {
  return <div>About</div>
}
```

default export ทุกรูปแบบผ่านการตรวจนี้ รวมถึงการ re-export — `export { Page as default }`,
`export { default } from './page-impl'` และ `export * as default from './page-impl'` นับทั้งหมด
ยกเว้น `export type { X as default }` ที่ไม่นับ เพราะ type export ถูกลบตอน compile
จึงไม่เหลืออะไรให้ render

### RUV1007: โมดูล server-only ถูก import ใน client graph

**Title**: Server-only module imported in client bundle

**คำอธิบาย**: Client component import โมดูลที่ใช้ server-only code (database, env, server modules)

```
RUV1007: Private import

  Package: @ruvyxa/database
  File: app/components/UserList.tsx:1
  Import chain:
    app/components/UserList.tsx (client)
    app/lib/db.ts

  Server-only packages cannot be imported in client bundles.

  Fix: Move the database access to a server action or
       use @ruvyxa/auth/client instead of @ruvyxa/auth.
```

**Edge cases**:

- Import chain: page.tsx → utils/helpers.ts → server/db.ts — Ruvyxa ตรวจทั้ง chain
- Dynamic import (`import()`) ก็โดนตรวจ
- Re-export (`export from`) ก็โดน

**วิธีแก้**:

- ย้าย import ที่ผิดออกไป
- ใช้ server action (`'use server'`) แทน direct import
- ใช้ `/client` subpath สำหรับ packages ที่มี

### RUV1008: ตัวแปร env ส่วนบุคคลรั่วไหลไปยัง client bundle

**Title**: Private environment variable leaked to client bundle

**คำอธิบาย**: ตัวแปร environment ที่ไม่มี `RUVYXA_PUBLIC_` prefix ถูกใช้ใน client —
เสี่ยง泄露 secret

```
RUV1008: Private environment variable leaked to client bundle

  Variable: DATABASE_URL
  File: app/components/UserCard.tsx:12

  ⚠ This variable is NOT prefixed with RUVYXA_PUBLIC_.
    Private environment access is not allowed in a client-reachable module.

  Fix:
    1. If this value is safe for clients, rename it to
       RUVYXA_PUBLIC_DATABASE_URL in your .env file.
    2. If this value must remain secret, move the usage
       to a server component, API route, or server action.
```

**วิธีแก้**:

- เปลี่ยนเป็น `RUVYXA_PUBLIC_` prefix: `RUVYXA_PUBLIC_API_URL`
- หรือย้ายไป server side (`'use server'`, server action, API route)
- ใช้ `npm run doctor` เช็ค env var security

```tsx
'use client'

// ❌ อันตราย
const dbUrl = process.env.DATABASE_URL // RUV1008

// ✅ ปลอดภัย
const apiUrl = process.env.RUVYXA_PUBLIC_API_URL
```

**Detection**: `npm run analyze` รายงาน diagnostic นี้และจบด้วยความล้มเหลว; `npm run build`
หยุดในขั้น prebuild validation ด้วยเหตุผลเดียวกัน Ruvyxa ไม่ inline ค่า private env เพื่อแสดง
diagnostic และ static check นี้ไม่ประเมินเงื่อนไข `typeof window`.

### RUV1009: โมดูล client-only ถูก import ใน server graph

**Title**: Client-only module imported in SSR graph

**คำอธิบาย**: Server component import โมดูลที่ใช้ browser APIs — ไม่สามารถ render บน server ได้

```
RUV1009: Client-only module imported into SSR graph

  File: app/components/Map.tsx:1
  Import: ./client-map

  The imported module declares `import 'client-only'` and cannot be reached
  by the server graph.

  Fix: Move the import behind a client component boundary, or remove the
       `client-only` marker if the module is genuinely server-safe.
```

**วิธีแก้**: ให้ module ที่มี `import 'client-only'` อยู่หลัง client component boundary หรือ เอา
marker ออกหาก module นั้น server-safe จริง Ruvyxa ไม่มี Next.js-style import option
`{ ssr: false }`.

### RUV1010: ไฟล์ในไดเรกทอรี server/ ถึง client graph ได้

**Title**: File inside server/ directory reachable by client graph

**คำอธิบาย**: ไฟล์ที่อยู่ใน `server/` directory ถู import จาก client component — `server/` ต้องเป็น
server-only

```
RUV1010: File inside server/ directory reachable by client graph

  File: app/server/db.ts
  Imported in: app/components/List.tsx

  Files inside server/ directories must only be imported
  from server components.

  Fix: Move the shared logic to a file outside server/,
       or restructure to avoid importing it from client code.
```

**วิธีแก้**: ย้าย shared logic ไปไว้นอก `server/` หรือปรับโครงสร้าง import

---

## RUV1100-1199: SSR / Render Errors

ข้อผิดพลาดเกี่ยวกับ server-side rendering และ renderer discovery

### RUV1100: React SSR ล้มเหลว

**Title**: React SSR failed

**คำอธิบาย**: React server-side rendering ล้มเหลว — component error ระหว่าง render บน server

**Error text**: `RUV1100: React SSR failed for route <path>: <detail>`

**สาเหตุทั่วไป**:

- Component throw error ระหว่าง render
- Data fetching error ใน async component
- React error boundary ถึงขีดจำกัด

**วิธีแก้**: ตรวจ component, เพิ่ม error boundary (`error.tsx`), ใช้ `loading.tsx`

### RUV1101: SSR renderer ต้องการ projectRoot, appDir และ pageFile

**Title**: SSR renderer missing required parameters

**คำอธิบาย**: SSR renderer ถูกเรียกโดยไม่มีพารามิเตอร์ที่จำเป็น — projectRoot, appDir, pageFile

**Error text**: `RUV1101: SSR renderer requires projectRoot, appDir and pageFile`

**วิธีแก้**: นี่คือ framework bug — รายงานที่ https://github.com/anomalyco/ruvyxa/issues

### RUV1102: ไม่พบ SSR renderer

**Title**: SSR renderer not found

**คำอธิบาย**: Route มี layout แต่ไม่มี SSR renderer ที่ตรงกัน — อาจเกิดจาก page ไม่มี default export

```
RUV1102: SSR renderer was not found

  Route: /dashboard

  The route has a layout but no matching SSR renderer.

  Fix: Ensure the page file exports a default component.
```

**วิธีแก้**: ตรวจสอบว่า page file มี `export default function Page()`

---

## RUV1200-1299: API / Server Runtime

ข้อผิดพลาดเกี่ยวกับ API routes, port binding, และ API renderer

### RUV1200: การเรียก API route ล้มเหลว

**Title**: API route call failed

**คำอธิบาย**: API route handler เรียกใช้แล้วล้มเหลว — unhandled exception หรือ network error

**Error text**: `RUV1200: API route <path> failed: <detail>`

**วิธีแก้**: เพิ่ม try/catch ใน route handler:

```ts
// app/api/users/route.ts
export async function GET() {
  try {
    const users = await db.user.findMany()
    return Response.json({ users })
  } catch (error) {
    console.error('API error:', error)
    return Response.json({ error: 'RUV1200', message: 'Internal server error' }, { status: 500 })
  }
}
```

### RUV1201: ไม่พบพอร์ตเซิร์ฟเวอร์ที่ว่าง

**Title**: No available server port

**คำอธิบาย**: ไม่พบพอร์ตที่ว่างสำหรับ dev server หรือ production server — port ถูกใช้หมดช่วง

**Error text**: `RUV1201: No available port found in range <start>-<end>`

**วิธีแก้**: ระบุ port ที่แน่นอนใน config (`server.port`) หรือตรวจสอบว่าไม่มี process ค้างอยู่

### RUV1202: ไม่พบ API renderer

**Title**: API renderer not found

**คำอธิบาย**: ไม่พบ API renderer สำหรับ route — อาจเกิดจาก route structure ไม่ถูกต้อง

**Error text**: `RUV1202: API renderer not found for route <path>`

**วิธีแก้**: ตรวจสอบว่าไฟล์ API route (`route.ts`) มี export ฟังก์ชัน HTTP method (`GET`, `POST`,
ฯลฯ)

---

## RUV1300-1399: Bundle / Compilation Errors

ข้อผิดพลาดเกี่ยวกับ client hydration bundling, module resolution, และ MDX

### RUV1300: Client hydration bundling ล้มเหลว

**Title**: Client hydration bundling failed

**คำอธิบาย**: ไม่สามารถ bundle client-side JavaScript สำหรับ hydration — compilation error หรือ
missing module

**Error text**: `RUV1300: Client hydration bundling failed: <detail>`

**วิธีแก้**: ตรวจสอบ dependency, รัน `npm run clean` แล้ว `npm run build` ใหม่

### RUV1303: ไม่พบ client route

**Title**: Client route not found

**คำอธิบาย**: client bundle สำหรับ CSR route ไม่พบใน build output

```
RUV1303: Client route was not found

  Route: /dashboard (type: csr)

  The client bundle for this CSR route was not found in the
  build output.

  Fix: Rebuild the application.
```

**วิธีแก้**: รัน `npm run clean` แล้ว `npm run build` ใหม่

### RUV1304: Client bundle ถูกเรียกสำหรับ route ที่ไม่ใช่ page

**Title**: Client bundle requested for non-page route

**คำอธิบาย**: มีการร้องขอ client bundle สำหรับ route ที่ไม่ใช่ page (เช่น API route)

```
RUV1304: Client bundle requested for a non-page route

  Route: /api/hello

  API routes do not have client bundles.

  Fix: This is likely a framework bug — report it.
```

**วิธีแก้**: นี่คือ framework bug — รายงานที่ https://github.com/anomalyco/ruvyxa/issues

### RUV1311: MDX compilation error

**Title**: MDX compilation error

**คำอธิบาย**: ไฟล์ MDX (.mdx) มี syntax error — compile ไม่ผ่าน

**Error text**: `RUV1311: MDX compilation error in <file>:<line>: <detail>`

**วิธีแก้**: ตรวจสอบ syntax MDX — ดู error message ที่ระบุ

### RUV1312: Frontmatter YAML error

**Title**: Frontmatter YAML error

**คำอธิบาย**: Frontmatter YAML ในไฟล์ MD/MDX มี syntax error

**Error text**: `RUV1312: Frontmatter YAML error in <file>: <detail>`

**วิธีแก้**: ตรวจสอบ YAML frontmatter — indent, colon, quotes

```mdx
---
title: 'บทความ'
date: 2024-01-01
tags: ['react', 'ruvyxa']
---

เนื้อหา...
```

---

## RUV1400-1499: Style Errors

ข้อผิดพลาดเกี่ยวกับ CSS compilation — Tailwind, Sass, CSS entries

### RUV1400: Tailwind CSS compilation ล้มเหลว

**Title**: Tailwind CSS compilation failed

**คำอธิบาย**: Tailwind CSS CLI compilation ล้มเหลว — config error หรือ content path ไม่ถูกต้อง

```
RUV1400: Tailwind CSS compilation failed

  Error: Tailwind CSS CLI exited with code 1

  Fix: Check tailwind.config.ts for errors, ensure all
       configured content paths exist.
```

**วิธีแก้**: ตรวจ `tailwind.config.ts`, ตรวจสอบ content paths

### RUV1401: ไม่พบ Tailwind CSS CLI

**Title**: Tailwind CSS CLI not found

**คำอธิบาย**: Ruvyxa ใช้ Tailwind CSS CLI สำหรับ production build แต่ไม่พบใน node_modules

```
RUV1401: Tailwind CSS CLI was not found

  Ruvyxa uses the Tailwind CSS CLI directly for production
  builds, but it was not found in node_modules.

  Fix: Install Tailwind CSS:
       npm install -D tailwindcss @tailwindcss/postcss
```

**วิธีแก้**: ติดตั้ง Tailwind CSS:

```bash
npm install -D tailwindcss @tailwindcss/postcss
```

### RUV1402: Sass compilation ล้มเหลว

**Title**: Sass compilation failed

**คำอธิบาย**: ไฟล์ SCSS/Sass มี syntax error — compile ไม่ผ่าน

```
RUV1402: Sass compilation failed

  File: app/styles/custom.scss:24
  Error: Expected "{" after selector

  Fix: Check the SCSS syntax around line 24.
```

**วิธีแก้**: ตรวจสอบ syntax SCSS/Sass

### RUV1403: ไม่พบ CSS entry ที่กำหนดค่าไว้

**Title**: Configured CSS entry not found

**คำอธิบาย**: ไฟล์ CSS ที่กำหนดใน `css.entries` ไม่พบในระบบ

```
RUV1403: Configured CSS entry was not found at: ...

  CSS entry: ./src/styles/main.css

  The file specified in css.entries could not be found.

  Fix: Check that the CSS file exists at the specified path.
```

**วิธีแก้**: ตรวจสอบว่าไฟล์ CSS มีอยู่ที่ path ที่ระบุ

### RUV1404: CSS entry ต้องอยู่ภายใน project root

**Title**: CSS entry must stay inside project root

**คำอธิบาย**: CSS entry path ต้องอยู่ภายใน project root — ห้ามใช้ `../` ที่ออกนอก project

```
RUV1404: CSS entry must stay inside the project root

  Path: ../shared/styles.css

  CSS entries must be inside the project directory tree.

  Fix: Move the CSS file into the project or use a symlink.
```

**วิธีแก้**: ย้ายไฟล์ CSS เข้ามาใน project หรือใช้ symlink

---

## RUV1500-1599: Worker / Static Params Errors

ข้อผิดพลาดเกี่ยวกับ render worker, route action, static params, และ PPR

### RUV1500: SSG/action render ล้มเหลว

**Title**: SSG/action render failed

**คำอธิบาย**: Static generation หรือ action render ล้มเหลว — worker crash หรือ runtime error

```
RUV1500: Worker crash

  Worker: render-worker-2
  Status: exit code 1

  A render worker process crashed while handling a request.

  Fix: Check server logs for the crash reason. Common causes:
       - Out of memory
       - Unhandled exception in route handler
       - Native module incompatibility
```

**วิธีแก้**: ตรวจ server logs, ลด workload, ใช้ try/catch

### RUV1501: ไม่พบไฟล์ route action

**Title**: Route action file not found

**คำอธิบาย**: Route มี action reference แต่ไม่พบไฟล์ action

```
RUV1501: Route action file was not found

  Route: /contact
  Expected: app/contact/action.ts

  The action file for this route does not exist.

  Fix: Create the action file at the expected path.
```

**วิธีแก้**: สร้างไฟล์ action ที่ path ที่คาดหวัง

### RUV1510: Static params ต้องเป็น array หรือ object ที่มี params

**Title**: Static params resolution failed

**คำอธิบาย**: ค่า return จาก static params ไม่ถูกต้อง — ต้องเป็น array ของ objects

```
RUV1510: Static params resolution failed

  Route: /blog/[slug]
  getStaticParams returned: [{ slug: null }]

  Static params values must be strings or numbers, not null.

  Fix: Filter out null/undefined values before returning.
```

**วิธีแก้**: return array ของ parameter objects:

```ts
export const getStaticParams = async () => {
  const posts = await fetchPosts()
  return posts.filter((p) => p.slug).map((p) => ({ slug: p.slug }))
}
```

### RUV1511: String shorthand ใช้ได้เฉพาะ route ที่มี segment dynamic ตัวเดียว

**Title**: Static params shorthand invalid for multi-segment route

**คำอธิบาย**: ใช้ string shorthand สำหรับ route ที่มีหลาย dynamic segments — ต้องใช้ object form

```
RUV1511: Static params shorthand invalid

  Route: /products/[category]/[id]
  getStaticParams returned: ["electronics"]

  String shorthand is only valid for routes with exactly
  one dynamic segment.

  Fix: Use object form: [{ category: "electronics", id: "123" }]
```

**วิธีแก้**: ใช้ object form สำหรับ multi-segment routes

### RUV1512: Static params entry ต้องเป็น object หรือ scalar

**Title**: Static params entry must be object or scalar

**คำอธิบาย**: แต่ละ entry ใน static params array ต้องเป็น object หรือ scalar value

```
RUV1512: Static params shape invalid

  Route: /posts/[slug]
  getStaticParams returned: "not-an-array"

  getStaticParams must return an array of parameter objects.

  Fix: Return an array, e.g., [{ slug: "hello" }, { slug: "world" }]
```

**วิธีแก้**: return array ของ parameter objects หรือ scalars

### RUV1513: Static params cache duration ไม่ถูกต้อง

**Title**: Static params cache duration invalid

**คำอธิบาย**: ค่า cache duration ไม่ถูกต้อง — ต้องเป็น number (seconds) หรือ string pattern

```
RUV1513: Static params duration invalid

  Route: /blog/[slug]
  cache: "forever"

  Cache duration must be a number (seconds) or a string like
  "10m", "1h", "1d".

  Fix: "forever" is not valid. Use "365d" or 31536000.
```

**รูปแบบที่รองรับ**: `"10m"`, `"1h"`, `"1d"`, `"30d"`, หรือ number (seconds)

**วิธีแก้**: ใช้รูปแบบที่ถูกต้อง เช่น `"365d"` หรือ `31536000`

### RUV1550: PPR render ล้มเหลว

**Title**: PPR render failed

**คำอธิบาย**: Partial Pre-Rendering (PPR) ล้มเหลวระหว่าง static shell generation

```
RUV1550: PPR render failed

  Route: /dashboard

  Partial pre-rendering encountered an error during the
  static shell generation.

  Fix: Check the component for dynamic data access during
       the static shell phase.
```

**วิธีแก้**: ตรวจสอบ component สำหรับ dynamic data access ใน static shell phase

---

## RUV1600-1699: Config / Adapter Definition Errors

ข้อผิดพลาดเกี่ยวกับ config loading, validation, และ adapter definition

### RUV1600: การโหลด config ล้มเหลว

**Title**: Config load failed

**คำอธิบาย**: ไม่สามารถโหลด `ruvyxa.config.ts` — runtime error หรือ import ผิด

**Error text**: `RUV1600: Failed to load config file: <error>`

**สาเหตุทั่วไป**:

- Import path ผิด
- Runtime error ใน config
- Module not found

**วิธีแก้**: ตรวจ import, syntax, ติดตั้ง dependencies ที่จำเป็น

### RUV1601: ค่าฟิลด์ config ไม่ถูกต้อง

**Title**: Config field value invalid

**คำอธิบาย**: ค่าใน config field ไม่ถูกต้อง — เช่น type ผิด หรือค่านอกช่วง

**Error text**: `RUV1601: Config field <field> value <value> is invalid: <detail>`

**วิธีแก้**: แก้ไขค่าฟิลด์ให้ถูกต้องตาม schema

### RUV1602: รูปแบบ ฟิลด์ หรือค่าของ config ไม่ถูกต้อง

**Title**: Config shape, field, or limit is invalid

**คำอธิบาย**: รูปแบบหรือชื่อ field ไม่รองรับ หรือค่าเกินขีดจำกัดที่ source กำหนด

**Error text**: `RUV1602: Config field <field> is invalid, unknown, or exceeds the source limit`

**วิธีแก้**: ปรับค่าให้อยู่ในช่วงที่อนุญาต

### RUV1603: Adapter ต้องมีฟังก์ชัน build()

**Title**: Adapter must implement build()

**คำอธิบาย**: Adapter object ไม่มีฟังก์ชัน `build()` — Ruvyxa ต้องการ `build()` สำหรับ deployment

**Error text**: `RUV1603: Adapter <name> must have a build() function`

**วิธีแก้**: เพิ่มฟังก์ชัน `build()` ใน adapter:

```ts
export default {
  name: 'my-adapter',
  build: async (context) => {
    // implementation
  },
}
```

---

## RUV1700-1799: Plugin Bridge Errors

ข้อผิดพลาดเกี่ยวกับ communication ระหว่าง Rust host และ JS plugin worker

### RUV1700: Plugin hook timeout / host หยุดทำงาน

**Title**: Plugin hook timeout or host crashed

**คำอธิบาย**: Plugin hook ใช้เวลาเกินกำหนด หรือ host process หยุดทำงานกะทันหัน

```
RUV1700: TypeScript plugin hook timed out after 30000 ms

  Plugin: my-plugin
  Hook: http.onRequest

  The plugin exceeded middleware.timeoutMs.

  Fix: Reduce plugin work or increase middleware.timeoutMs.
```

**วิธีแก้**: ลดงานใน plugin hook หรือเพิ่ม `middleware.timeoutMs`

### RUV1701: Plugin protocol error

**Title**: Plugin protocol error

**คำอธิบาย**: Communication protocol ระหว่าง Rust host และ JS worker ผิดพลาด — invalid JSON หรือ
message format

```
RUV1701: TypeScript plugin host returned invalid JSON

  The plugin host sent malformed JSON over the IPC channel.

  Fix: This is likely a framework or plugin bug — report it.
```

**วิธีแก้**: นี่คือ framework หรือ plugin bug — รายงานที่ GitHub issues

### RUV1702: ไม่พบ Worker pool script

**Title**: Worker pool script not found

**คำอธิบาย**: ไม่พบ runtime script สำหรับ TypeScript plugin host (`plugin-runtime.mjs`)

```
RUV1702: Worker pool script was not found

  Script: plugin-runtime.mjs

  The TypeScript plugin host runtime script is missing from
  the installed Ruvyxa package.

  Fix: Reinstall ruvyxa: npm install ruvyxa
```

**วิธีแก้**: ติดตั้ง ruvyxa ใหม่: `npm install ruvyxa`

### RUV1704: Worker pool stream error

**Title**: Worker pool stream error

**คำอธิบาย**: IPC stream error ระหว่าง main process และ worker — communication ล้มเหลว

**Error text**: `RUV1704: Worker pool stream error: <detail>`

**วิธีแก้**: รีสตาร์ท dev server, ตรวจสอบ system resources

---

## RUV1800-1899: JS Runtime Errors

ข้อผิดพลาดใน JavaScript runtime — module resolution, transform, circular dependencies

### RUV1801: ไม่สามารถ resolve โมดูลได้

**Title**: Module resolution failed

**คำอธิบาย**: ไม่พบโมดูลที่ import — import path ไม่ถูกต้อง หรือไม่ได้ติดตั้ง package

**Error text**: `RUV1801: Cannot resolve module <source> from <importer>`

**วิธีแก้**: ตรวจสอบ import path หรือติดตั้ง package ที่ขาด:

```bash
# ตรวจ path
import { something } from './correct/path'

# ติดตั้ง package
npm install <package-name>
```

### RUV1802: Oxc transform ล้มเหลว

**Title**: Oxc transform failed

**คำอธิบาย**: Oxc (Rust JavaScript/TypeScript transformer) ไม่สามารถ transform ไฟล์ได้ — syntax
error

**Error text**: `RUV1802: Oxc transform failed for <file>: <detail>`

**วิธีแก้**: ตรวจสอบ syntax error ในไฟล์ที่ระบุ

### RUV1803: ตรวจพบ circular dependency

**Title**: Circular dependency detected

**คำอธิบาย**: สองโมดูล import ซึ่งกันและกัน — สร้าง loop ใน dependency graph

**Error text**: `RUV1803: Circular dependency detected: <chain>`

**วิธีแก้**: แยก shared logic ไปไว้ในไฟล์ที่สาม หรือปรับโครงสร้าง import

```
// ❌ A → B → A
// a.ts imports b.ts imports a.ts

// ✅ A → C ← B
// a.ts imports c.ts, b.ts imports c.ts
```

### RUV1804: JSX runtime ต้องเป็น classic หรือ automatic

**Title**: JSX runtime must be 'classic' or 'automatic'

**คำอธิบาย**: ค่า jsxRuntime ใน config ไม่ถูกต้อง — ต้องเป็น `"classic"` หรือ `"automatic"`

**Error text**: `RUV1804: JSX runtime <value> is invalid. Must be "classic" or "automatic"`

**วิธีแก้**: ตั้งค่า jsxRuntime เป็น `"classic"` หรือ `"automatic"` ใน config

---

## RUV2000-2200: Adapter / Plugin Definition Errors

ข้อผิดพลาดเกี่ยวกับ adapter API, plugin definition, และ build hooks

### RUV2000: BuildContext validation ล้มเหลว

**Title**: BuildContext validation failed

**คำอธิบาย**: BuildContext ที่ส่งให้ adapter ไม่ผ่าน validation — field ที่จำเป็นหายไป

```
RUV2000: BuildContext.root is required and must be a non-empty string

  Adapter: vercelAdapter

  Fix: Ensure the adapter receives a valid BuildContext.
```

**วิธีแก้**: ตรวจสอบว่า adapter ได้รับ BuildContext ที่ถูกต้อง

### RUV2001: ค่า options ของ adapter ไม่ถูกต้อง

**Title**: Adapter option invalid

**คำอธิบาย**: ค่า options ที่ส่งให้ adapter ไม่ถูกต้อง — แต่ละ adapter มี validation ของตัวเอง

```
[RUV2001] vercelAdapter: "regions" must be a non-empty array of region codes, such as ["sin1"]
[RUV2001] netlifyAdapter: "functionsDir" must not be an empty string
[RUV2001] cloudflareAdapter: "workerEntry" must be a string
[RUV2001] staticAdapter: "outputDir" overlaps protected build output
```

**วิธีแก้**: ดู error message และแก้ไข options ให้ถูกต้องตาม schema ของ adapter

### RUV2102: Plugin definition ไม่ถูกต้อง

**Title**: Invalid plugin definition

**คำอธิบาย**: ฟังก์ชัน `definePlugin()` ส่งค่าที่ไม่ถูกต้อง — validation error จาก plugin schema

```
RUV2102: Ruvyxa plugin must be an object.
RUV2102: Ruvyxa plugin must have a non-empty name.
RUV2102: Ruvyxa plugin "my-plugin" register must be a function.
RUV2102: Ruvyxa plugin "my-plugin" http.onRequest must be a function.
```

**วิธีแก้**: ดู error message ที่ระบุฟิลด์ที่ผิดพลาดและแก้ไข

### RUV2200: Adapter build hook ล้มเหลว

**Title**: Adapter build hook failed

**คำอธิบาย**: Adapter `build()` hook ล้มเหลวระหว่าง build process

**Error text**: `RUV2200: Adapter <name> build hook failed: <detail>`

**วิธีแก้**: ตรวจสอบ error details, ตรวจ compatibility ของ adapter กับ project

---

## RUV3000-3201: Official Package Errors

### @ruvyxa/database errors

| Code        | Title                    | Cause                           | Fix                    |
| ----------- | ------------------------ | ------------------------------- | ---------------------- |
| **RUV3001** | Database operation error | Invalid args, model name unsafe | Check query parameters |
| **RUV3002** | Adapter error            | Adapter-specific failure        | Check adapter logs     |
| **RUV3003** | Connection failed        | Database unreachable            | Check DATABASE_URL     |

### @ruvyxa/auth errors

| Code        | Title                  | Cause                        | Fix                      |
| ----------- | ---------------------- | ---------------------------- | ------------------------ |
| **RUV3100** | Auth service error     | Magic link delivery failed   | Check email provider     |
| **RUV3101** | Auth request invalid   | Cross-origin, body too large | Fix request, reduce body |
| **RUV3102** | Too many attempts      | Rate limit bucket หมดโควตา   | รอตาม `Retry-After`      |
| **RUV3103** | OAuth state invalid    | State mismatch or expired    | Re-authenticate          |
| **RUV3104** | OAuth provider error   | Token/profile request failed | Check provider           |
| **RUV3105** | Production store error | Non-durable store            | Use persistent store     |

### @ruvyxa/realtime errors

| Code        | Title          | Cause              | Fix                  |
| ----------- | -------------- | ------------------ | -------------------- |
| **RUV3201** | Realtime error | Protocol violation | Check message format |

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
app/blog/error.tsx    → เฉพาะ /blog/* routes
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
│  ⚠  RUV1008: Private environment variable                      │
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

// === Action ที่จัดการ error ครบถ้วน ===
export const createUser = action
  .input({
    parse(value: unknown): { email: string; name: string } {
      if (!value || typeof value !== 'object') throw new Error('รูปแบบข้อมูลไม่ถูกต้อง')
      const { email, name } = value as Record<string, unknown>
      if (typeof email !== 'string' || !email.includes('@')) {
        throw new Error('กรุณากรอกอีเมลที่ถูกต้อง')
      }
      if (typeof name !== 'string' || name.length < 2) {
        throw new Error('ชื่อต้องมีอย่างน้อย 2 ตัวอักษร')
      }
      return { email, name }
    },
  })
  .handler(async ({ input }) => {
    try {
      const { email, name } = input

      // 1. Business logic
      const existing = await db.user.findUnique({ where: { email } })
      if (existing) {
        return { error: 'อีเมลนี้มีผู้ใช้แล้ว', code: 'RUV1200' }
      }

      const user = await db.user.create({
        data: { email, name },
        select: { id: true, email: true, name: true },
      })

      await sendWelcomeEmail(email, name)

      // 2. Success
      return { success: true, user }
    } catch (error) {
      // 3. Error handling: Ruvyxa does not export a RuvyxaError class to applications.
      console.error('Unexpected error in createUser:', error)
      return {
        error: 'เกิดข้อผิดพลาดที่ไม่ทราบสาเหตุ',
        code: 'RUV1200',
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
    const result = await createUser({
      email: String(formData.get('email') ?? ''),
      name: String(formData.get('name') ?? ''),
    })

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

    return Response.json({
      data: users,
      pagination: { total, limit, offset },
    })
  } catch (error) {
    console.error('GET /api/users error:', error)
    return Response.json(
      {
        error: 'RUV1200',
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
      return Response.json(
        { error: 'RUV1200', message: 'ข้อมูลไม่ครบ: email และ name' },
        { status: 400 },
      )
    }

    const user = await db.user.create({ data: body })
    return Response.json({ user }, { status: 201 })
  } catch (error) {
    const isValidation = error instanceof SyntaxError
    const status = isValidation ? 400 : 500

    return Response.json(
      {
        error: 'RUV1200',
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
npm run doctor # ตรวจสอบทุกอย่าง — config, routes, boundary, env
npm run doctor -- --json     # รับ compatibility report เป็น JSON
npm run check # TypeScript check (เมื่อมี tsconfig) และ parity test
npm run analyze # ตรวจ routes, imports และ server/client boundary
npm run trace -- /           # ดู route manifest entry ของ path ที่ระบุ
```

**Output `npm run doctor`**:

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

| Code        | Title (Thai)                              | ช่วง              | Severity |
| ----------- | ----------------------------------------- | ----------------- | -------- |
| **RUV1001** | ไม่พบไดเรกทอรี app                        | Boundary / Graph  | Error    |
| **RUV1002** | Segment route dynamic ไม่ถูกต้อง          | Boundary / Graph  | Error    |
| **RUV1003** | เส้นทาง route ขัดแย้งกัน                  | Boundary / Graph  | Error    |
| **RUV1004** | Page ไม่มี default export                 | Boundary / Graph  | Error    |
| **RUV1007** | โมดูล server-only ใน client graph         | Boundary / Graph  | Error    |
| **RUV1008** | ตัวแปร env ส่วนบุคคลรั่วไหล               | Boundary / Graph  | Error    |
| **RUV1009** | โมดูล client-only ใน server graph         | Boundary / Graph  | Error    |
| **RUV1010** | ไฟล์ใน server/ ถึง client graph           | Boundary / Graph  | Error    |
| **RUV1100** | React SSR ล้มเหลว                         | SSR / Render      | Error    |
| **RUV1101** | SSR renderer ขาดพารามิเตอร์               | SSR / Render      | Error    |
| **RUV1102** | ไม่พบ SSR renderer                        | SSR / Render      | Error    |
| **RUV1200** | การเรียก API route ล้มเหลว                | API / Server      | Error    |
| **RUV1201** | ไม่พบพอร์ตเซิร์ฟเวอร์ที่ว่าง              | API / Server      | Error    |
| **RUV1202** | ไม่พบ API renderer                        | API / Server      | Error    |
| **RUV1300** | Client hydration bundling ล้มเหลว         | Bundle / Compile  | Error    |
| **RUV1303** | ไม่พบ client route                        | Bundle / Compile  | Error    |
| **RUV1304** | Client bundle สำหรับ non-page route       | Bundle / Compile  | Error    |
| **RUV1311** | MDX compilation error                     | Bundle / Compile  | Error    |
| **RUV1312** | Frontmatter YAML error                    | Bundle / Compile  | Error    |
| **RUV1400** | Tailwind CSS compilation ล้มเหลว          | Style             | Error    |
| **RUV1401** | ไม่พบ Tailwind CSS CLI                    | Style             | Error    |
| **RUV1402** | Sass compilation ล้มเหลว                  | Style             | Error    |
| **RUV1403** | ไม่พบ CSS entry ที่กำหนด                  | Style             | Error    |
| **RUV1404** | CSS entry ต้องอยู่ใน project root         | Style             | Error    |
| **RUV1500** | SSG/action render ล้มเหลว                 | Worker / Params   | Error    |
| **RUV1501** | ไม่พบไฟล์ route action                    | Worker / Params   | Error    |
| **RUV1510** | Static params รูปแบบผิด                   | Worker / Params   | Error    |
| **RUV1511** | String shorthand ไม่ถูกต้อง               | Worker / Params   | Error    |
| **RUV1512** | Static params entry ผิด                   | Worker / Params   | Error    |
| **RUV1513** | Static params cache duration ผิด          | Worker / Params   | Error    |
| **RUV1550** | PPR render ล้มเหลว                        | Worker / Params   | Error    |
| **RUV1600** | การโหลด config ล้มเหลว                    | Config / Adapter  | Error    |
| **RUV1601** | ค่าฟิลด์ config ไม่ถูกต้อง                | Config / Adapter  | Error    |
| **RUV1602** | รูปแบบ ฟิลด์ หรือค่าของ config ไม่ถูกต้อง | Config / Adapter  | Error    |
| **RUV1603** | Adapter ต้องมี build()                    | Config / Adapter  | Error    |
| **RUV1700** | Plugin hook timeout / host หยุด           | Plugin Bridge     | Error    |
| **RUV1701** | Plugin protocol error                     | Plugin Bridge     | Error    |
| **RUV1702** | ไม่พบ Worker pool script                  | Plugin Bridge     | Error    |
| **RUV1704** | Worker pool stream error                  | Plugin Bridge     | Error    |
| **RUV1801** | ไม่สามารถ resolve โมดูล                   | JS Runtime        | Error    |
| **RUV1802** | Oxc transform ล้มเหลว                     | JS Runtime        | Error    |
| **RUV1803** | Circular dependency                       | JS Runtime        | Error    |
| **RUV1804** | JSX runtime ไม่ถูกต้อง                    | JS Runtime        | Error    |
| **RUV2000** | BuildContext validation ล้มเหลว           | Adapter / Plugin  | Error    |
| **RUV2001** | ค่า options ของ adapter ไม่ถูกต้อง        | Adapter / Plugin  | Error    |
| **RUV2102** | Plugin definition ไม่ถูกต้อง              | Adapter / Plugin  | Error    |
| **RUV2200** | Adapter build hook ล้มเหลว                | Adapter / Plugin  | Error    |
| **RUV3001** | Database operation error                  | Official Packages | Error    |
| **RUV3002** | Database adapter error                    | Official Packages | Error    |
| **RUV3003** | Database connection failed                | Official Packages | Error    |
| **RUV3100** | Auth service error                        | Official Packages | Error    |
| **RUV3101** | Auth request invalid                      | Official Packages | Error    |
| **RUV3102** | Too many authentication attempts          | Official Packages | Error    |
| **RUV3103** | OAuth state invalid                       | Official Packages | Error    |
| **RUV3104** | OAuth provider error                      | Official Packages | Error    |
| **RUV3105** | Production store error                    | Official Packages | Error    |
| **RUV3201** | Realtime error                            | Official Packages | Error    |

---

## Troubleshooting — Quick Reference

| Error Code(s)       | ปัญหาที่พบบ่อย      | วิธีแก้ด่วน                                            |
| ------------------- | ------------------- | ------------------------------------------------------ |
| RUV1007-1010        | Boundary violations | ตรวจ `'use client'`, imports, env vars                 |
| RUV1001-1004        | Route discovery     | ตรวจ `app/` directory, route names, default exports    |
| RUV1100-1102        | SSR / Render        | ตรวจ component, `error.tsx`, default export            |
| RUV1200-1202        | API / Port          | ตรวจ route handler, port config, try/catch             |
| RUV1300, 1303-1304  | Bundle / Hydration  | `npm run clean` แล้ว `npm run build`, ดู error message |
| RUV1311-1312        | MDX / Frontmatter   | ตรวจ syntax MDX และ YAML                               |
| RUV1400-1404        | Style / CSS         | ตรวจ Tailwind config, SCSS syntax, css.entries         |
| RUV1500-1501        | Render / Action     | ดู server logs, ตรวจ action files                      |
| RUV1510-1513        | Static params       | ตรวจ `getStaticParams` return shape                    |
| RUV1550             | PPR                 | ตรวจ component ใน static shell phase                   |
| RUV1600-1603        | Config / Adapter    | ตรวจ config fields, adapter implements build()         |
| RUV1700, 1702, 1704 | Plugin bridge       | รีสตาร์ท dev server, `npm install ruvyxa`              |
| RUV1701             | Plugin protocol     | อัปเดต Ruvyxa, ตรวจ plugin compatibility               |
| RUV1801-1804        | JS Runtime          | ตรวจ import paths, syntax, circular deps               |
| RUV2000-2001        | Adapter config      | ตรวจ BuildContext และ adapter options                  |
| RUV2102             | Plugin definition   | ตรวจ `definePlugin()` return value                     |
| RUV2200             | Build hook          | ตรวจ adapter compatibility                             |
| RUV3001-3003        | Database            | ตรวจ DATABASE_URL, adapter logs                        |
| RUV3100-3105        | Auth                | ตรวจ provider config, OAuth state                      |
| RUV3201             | Realtime            | ตรวจ message format                                    |

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
9. รัน `npm run doctor` — ดู error ทั้งหมดในแอป
10. ทดสอบ API route error handling — ส่ง POST ผิด format
11. ตรวจสอบ bundle budget — เพิ่ม `bundleBudget` plugin
12. ดู error overlay ใน dev mode — ทำ intentional error

---

## สรุป

- Error codes: RUV1001-3201 — แบ่งเป็น 11 ช่วง: boundary/graph, SSR/render, API/server,
  bundle/compile, style, worker/params, config/adapter, plugin bridge, JS runtime, adapter/plugin
  def, official packages
- รวม 50+ error codes แต่ละตัวมี: code, title, คำอธิบาย, error text, วิธีแก้
- Error boundary: `error.tsx` (catch errors), `not-found.tsx` (404), `loading.tsx` (loading state)
- Error overlay ใน dev mode — แสดง file, line, why, fix, dismiss, reload, open in editor
- Server actions และ API routes: จัดการด้วย try/catch + `RuvyxaError`
- `notFound()` และ `redirect()` (301, 302, 307, 308) สำหรับควบคุม flow
- `npm run doctor` ตรวจทุกอย่าง — config, routes, boundary, env
- 2 ตาราง: error code ทั้งหมด + cross-reference

---

## อ่าน Diagnostic เป็น Boundary Signal

diagnostics ของ Ruvyxa ถูกสร้างจาก subsystem ที่เห็นปัญหา ให้เริ่มจาก code/message แล้วไปที่
boundary ที่เป็นเจ้าของปัญหา แทนการมอง numeric ranges ว่าทุกเลขต้องมีอยู่จริง กลุ่มสำคัญปัจจุบันคือ:

| Area                    | ตัวอย่าง                                              | จุดที่ควรตรวจแรก                                                       |
| ----------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------- |
| Route discovery         | `RUV1001`, `RUV1002`, `RUV1004`                       | `appDir`, route entry filename, รูปแบบ dynamic segment, default export |
| Client/server boundary  | `RUV1007`, `RUV1008`, `RUV1009`, `RUV1010`            | ไฟล์ที่ diagnostic ระบุและ reachable relative imports                  |
| Content และ styles      | `RUV1310`–`RUV1312`, `RUV1402`, `RUV1403`             | Markdown/MDX frontmatter, Sass source หรือ stylesheet import path      |
| Configuration           | `RUV1601`, `RUV1602`                                  | config field ที่ระบุและ range/path constraint ในเอกสาร                 |
| Plugin/adapter contract | `RUV2102`, `RUV2200`, `RUV2202`, `RUV2203`, `RUV2210` | plugin definition หรือ target/adapter package ที่เลือก                 |

ใช้คำสั่งที่เล็กที่สุดที่เปิดเผย boundary เดียวกัน:

```bash
npm run routes
npm run trace -- /the-route-pattern
npm run analyze -- --format human
npm run doctor -- --json
```

`routes` ตอบเรื่อง discovery, `trace` ตอบ manifest entry เดียว, `analyze` ตอบ route/import
validation และ `doctor` ตอบ environment/configuration/adapter compatibility คำสั่งเหล่านี้ไม่ได้แก้
source/config ให้อัตโนมัติ จึงควรเก็บ file/path context ของ diagnostic แล้วแก้ที่สาเหตุที่เล็กที่สุด

### ลำดับ Escalation ที่ปลอดภัย

1. reproduce ด้วยคำสั่งที่เป็นเจ้าของ failure
2. อ่านไฟล์และ import/config edge ที่ result ระบุ
3. เปลี่ยนสาเหตุเดียว ไม่เปลี่ยน settings ที่ไม่เกี่ยวหลายอย่างพร้อมกัน
4. รัน focused command ซ้ำ
5. รัน `npm run check` เมื่อ failure ข้าม route, render หรือ type boundary

อย่า log secrets, environment dump ทั้งหมด หรือ private request payload เพื่อ "ดูรายละเอียดเพิ่ม"
boundary diagnostics ตั้งใจให้ชี้ source location และ safe direction โดยไม่ต้องใช้ข้อมูล sensitive

## รูปแบบการป้องกัน Error (Error Prevention Patterns)

### Guard Against RUV1007 (Boundary Violations)

```typescript
// ❌ Bad — imports server-only package in client component
'use client'
import { db } from '../lib/db' // RUV1007

// ✅ Good — server action handles DB, client calls it
;('use client')
import { fetchUsers } from '../actions/users'

// ✅ Good — use /client subpath for auth
;('use client')
import { createAuthClient } from '@ruvyxa/auth/client'
```

### Guard Against RUV1008 (Env Leaks)

```typescript
// ❌ Bad — private env in client-reachable code
const url = process.env.DATABASE_URL // RUV1008

// ✅ Good — public env (prefixed)
const apiUrl = process.env.RUVYXA_PUBLIC_API_URL

// ✅ Good — private env accessed only in server actions
;('use server')
export const getData = action.handler(async () => {
  const url = process.env.DATABASE_URL // safe here
})
```

### Guard Against RUV1510-1513 (Static Params)

```typescript
// ✅ Good — always return valid params
export const getStaticParams: GetStaticParams = async () => {
  const posts = await fetchPosts()
  return posts.filter((p) => p.slug).map((p) => ({ slug: p.slug }))
}

// ❌ Bad — returns null slugs
export const getStaticParams = async () => {
  return [{ slug: null }] // RUV1510
}

// ❌ Bad — string shorthand for multi-segment route
// Route: /[category]/[id]
export const getStaticParams = async () => {
  return ['electronics'] // RUV1511 — use [{ category: 'electronics', id: '123' }]
}
```

### Guard Against Config Errors

```typescript
// ✅ Good — use the config() helper for type checking
import { config } from 'ruvyxa/config'
export default config({
  server: { port: 3000 },
})

// ❌ Bad — typos are not caught
export default {
  server: { port: '3000' }, // RUV1602 — shape/type mismatch
  build: { split: 'none' }, // RUV1601 — unknown value
}
```

---

## รูปแบบ Error ที่พบบ่อยตามระยะการพัฒนา (Common Error Patterns)

### During `npm run dev`

| Symptom                    | Likely Error                                     | Action                                   |
| -------------------------- | ------------------------------------------------ | ---------------------------------------- |
| Error overlay on page load | RUV1300 (hydration bundling), RUV1007 (boundary) | Check file indicated in overlay          |
| Page renders blank         | RUV1100 (SSR error in component)                 | Add `error.tsx`, check browser console   |
| HMR not updating           | Refresh browser, check network                   | —                                        |
| Slow page loads            | RUV1100 (SSR render slow)                        | Optimize component, reduce data fetching |
| 404 for existing route     | Route not exported                               | Check route naming, file location        |
| 500 on form submit         | RUV1500 (action error)                           | Check action code, add error handling    |
| Plugin not running         | RUV1602 (plugin/config shape invalid)            | Check plugin configuration               |

### During `npm run build`

| Symptom                                  | Likely Error                           | Action                                 |
| ---------------------------------------- | -------------------------------------- | -------------------------------------- |
| Build fails immediately                  | RUV1600 (config load failure)          | Run `npm run doctor`                   |
| Build fails during compilation           | RUV1802 (Oxc transform), RUV1311 (MDX) | Fix indicated syntax error             |
| Build fails at module resolution         | RUV1801 (module not resolved)          | Install package or fix import path     |
| Build succeeds but output missing routes | RUV1002 (invalid route segment)        | Check filenames for invalid characters |
| Build succeeds but circular deps found   | RUV1803 (circular dependency)          | Break the dependency cycle             |
| Build OOM                                | Not a numbered error                   | Reduce `build.workers`, increase RAM   |

### During `npm run check`

| Symptom                      | Likely Error          | Action                                   |
| ---------------------------- | --------------------- | ---------------------------------------- |
| Boundary violations reported | RUV1007-1010          | Restructure imports                      |
| Route conflicts reported     | RUV1003 (conflicting) | Remove one route or add a static segment |
| Config validation errors     | RUV1600-1603          | Fix config file                          |
| SSG params missing           | RUV1510-1513          | Add/export `getStaticParams`             |

### During Deployment

| Symptom                  | Likely Error                  | Action                       |
| ------------------------ | ----------------------------- | ---------------------------- |
| Build fails in CI        | RUV2200 (adapter hook failed) | Check adapter logs           |
| 502 on all routes        | RUV2200 (adapter build error) | Check adapter configuration  |
| Static site has no pages | RUV2200 (adapter mismatch)    | Use correct adapter          |
| Functions timeout        | RUV1700 (plugin timeout)      | Increase timeout or optimize |
| Cold starts are slow     | Not a numbered error          | Use `build.warm: true`       |

---

## สรุปแบบย่อ (Quick Reference)

### Error Code Ranges

```
Range       Category          Where to look
──────────  ────────────────  ──────────────────────
RUV1001     Route discovery   03-server-client-components.md
RUV1007     Boundary          bundler boundary check
RUV1010     Boundary          bundler boundary check
RUV1101     SSR               ssr-renderer.mjs
RUV1205     Prerender         prerender.rs
RUV1400     Style             dev_server style path
RUV1550     PPR               dev_server render path
RUV1600-1603 Config            config renderer and CLI validation
RUV1700     Plugin host       plugin_host.rs, worker_pool.rs
RUV1801-1804 Compiler          compiler.mjs
RUV2000     Adapter           @ruvyxa/core/utils.ts
RUV2102     Plugin def        @ruvyxa/core/plugin.ts
RUV2200     Adapter build     runtime_config.rs, adapter-runner.mjs
RUV3001     Database          15-official-packages.md
RUV3100     Auth              15-official-packages.md
RUV3201     Realtime          15-official-packages.md
```

### File Locations

| Error source            | File                                                                         |
| ----------------------- | ---------------------------------------------------------------------------- |
| Bundler boundary checks | `crates/ruvyxa_bundler/src/boundary.rs`                                      |
| Graph route validation  | `crates/ruvyxa_graph/src/lib.rs`                                             |
| Dev server rendering    | `crates/ruvyxa_dev_server/src/render_pipeline.rs`                            |
| Worker pool             | `crates/ruvyxa_dev_server/src/worker_pool.rs`                                |
| Style compilation       | `crates/ruvyxa_dev_server/src/style.rs`                                      |
| Plugin host             | `crates/ruvyxa_middleware/src/plugin_host.rs`                                |
| Config validation       | `crates/ruvyxa_cli/src/config.rs`, `crates/ruvyxa_cli/src/runtime_config.rs` |
| Plugin bridge           | `crates/ruvyxa_dev_server/src/plugin_bridge.rs`                              |
| Plugin validation       | `packages/@ruvyxa/core/src/plugin.ts`                                        |
| Compiler (JS)           | `packages/ruvyxa/runtime/compiler.mjs`                                       |
| Config renderer (JS)    | `packages/ruvyxa/runtime/config-renderer.mjs`                                |
| Worker pool (JS)        | `packages/ruvyxa/runtime/worker-pool.mjs`                                    |
| SSR renderer (JS)       | `packages/ruvyxa/runtime/ssr-renderer.mjs`                                   |
| API renderer (JS)       | `packages/ruvyxa/runtime/api-renderer.mjs`                                   |
| Auth errors             | `packages/@ruvyxa/auth/src/index.ts`                                         |
| Database errors         | `packages/@ruvyxa/database/src/index.ts`                                     |
| Realtime errors         | `packages/@ruvyxa/realtime/src/plugin.ts`                                    |
| Adapter errors          | `packages/@ruvyxa/adapter-*/src/index.ts`                                    |

---

## การแก้ไขปัญหาเบื้องต้น (Troubleshooting)

| Symptom                        | Likely cause                    | Fix                                      |
| ------------------------------ | ------------------------------- | ---------------------------------------- |
| Blank white page               | Uncaught client error           | Check browser console, add `error.tsx`   |
| RUV1003 after adding file      | Two files match same URL        | Remove duplicate route file              |
| RUV1008 on build               | Private env in client component | Rename to `RUVYXA_PUBLIC_*`              |
| RUV1600 on project creation    | Typo in config                  | Run `npm run doctor`                     |
| RUV1801 for local import       | Incorrect import path           | Use relative path or alias               |
| RUV1100 after deploy           | SSR render failure              | Check page component, dependencies       |
| RUV1500 on worker pool         | SSG / action render failed      | Check server logs, restart               |
| RUV1501 on render              | Route action file missing       | Create action file at expected path      |
| RUV1700 with plugins           | Plugin timeout or crash         | Increase timeout or fix plugin code      |
| RUV2200 on build               | Adapter build hook failed       | Check adapter logs, configuration        |
| 404 on existing route          | Route not exported              | Add `export default function Page()`     |
| 500 on server action           | Validation failed               | Check action error return                |
| Error overlay not showing      | `debug.overlay: false`          | Enable in config                         |
| Error overlay always showing   | Persistent compile error        | Fix the error in your code               |
| `notFound()` does nothing      | No `not-found.tsx`              | Create the file or check hierarchy       |
| `reset()` does not clear error | State persists                  | Use `key` prop to force remount          |
| WebSocket (HMR) disconnects    | Network proxy/firewall          | Check WebSocket path, restart dev server |

---

## อ่าน Diagnostic เป็น Boundary Signal

Ruvyxa diagnostics are emitted by the subsystem that observed the problem. Start from the code and
message, then move to the owning boundary rather than treating numeric ranges as a promise that
every number exists. The current high-value groups include:

| Area                    | Examples                                              | First place to inspect                                                |
| ----------------------- | ----------------------------------------------------- | --------------------------------------------------------------------- |
| Route discovery         | `RUV1001`, `RUV1002`, `RUV1004`                       | `appDir`, route entry filename, dynamic-segment shape, default export |
| Client/server boundary  | `RUV1007`, `RUV1008`, `RUV1009`, `RUV1010`            | The diagnostic file and its reachable relative imports                |
| Content and styles      | `RUV1310`–`RUV1312`, `RUV1402`, `RUV1403`             | Markdown/MDX frontmatter, Sass source, or stylesheet import path      |
| Configuration           | `RUV1601`, `RUV1602`                                  | The named config field and its documented range/path constraint       |
| Plugin/adapter contract | `RUV2102`, `RUV2200`, `RUV2202`, `RUV2203`, `RUV2210` | Plugin definition or selected target/adapter package                  |

Use the smallest command that exposes the same boundary:

```bash
npm run routes
npm run trace -- /the-route-pattern
npm run analyze -- --format human
npm run doctor -- --json
```

`routes` answers discovery, `trace` answers one manifest entry, `analyze` answers route/import
validation, and `doctor` answers environment/configuration/adapter compatibility. None of these
commands automatically repairs source or config; preserve the diagnostic's file/path context while
making the smallest correction.

### A Safe Escalation Sequence

1. Reproduce with the narrow command that owns the failure.
2. Read the exact file and import/config edge named by the result.
3. Change one cause, not several unrelated settings.
4. Re-run the focused command.
5. Run `npm run check` when the failure crosses route, render, or type boundaries.

Avoid logging secrets, full environment dumps, or private request payloads to "get more detail". The
framework's boundary diagnostics are intentionally designed to identify a source location and a safe
direction without requiring sensitive data.

---
