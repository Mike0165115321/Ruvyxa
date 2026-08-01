# Routing ใน Ruvyxa

Routing คือหัวใจของ Ruvyxa หลักการคือ:

```
โฟลเดอร์ใน app/ = URL path
ไฟล์ page.tsx   = หน้าที่แสดง
ไฟล์ layout.tsx = layout ที่หุ้ม
ไฟล์ route.ts   = API endpoint
```

ไม่ต้อง config router แยก — file system คือ router framework ใช้ **Radix Trie** 数据结构 (radix
tree) สำหรับจับคู่ URL กับ route ด้วยความเร็ว O(k) โดย k = ความยาว path

---

## File Conventions

Ruvyxa จับคู่ไฟล์พิเศษในโฟลเดอร์ app/:

| ไฟล์        | ชนิด          | Extension                     | หน้าที่                                       |
| ----------- | ------------- | ----------------------------- | --------------------------------------------- |
| `page`      | หน้า          | `.tsx`, `.jsx`, `.md`, `.mdx` | แสดง content ของ route                        |
| `layout`    | Layout        | `.tsx`, `.jsx`                | หุ้ม child pages, persist ข้าม navigation     |
| `loading`   | Loading       | `.tsx`, `.jsx`                | UI ขณะ route โหลด                             |
| `error`     | Error         | `.tsx`, `.jsx`                | UI เมื่อ route error                          |
| `not-found` | 404           | `.tsx`, `.jsx`                | UI เมื่อหา route ไม่พบ                        |
| `route`     | API           | `.ts`, `.js`                  | HTTP endpoint (GET, POST, PUT, DELETE, PATCH) |
| `action`    | Action        | `.ts`, `.js`                  | Server action สำหรับ mutations                |
| `client`    | Client module | `.tsx`                        | Client-side module (hydration)                |
| `server`    | Server module | `.ts`, `.js`                  | Server-only module                            |

### page.tsx — type signature

```ts
// app/page.tsx
import type { PageProps } from 'ruvyxa/config'

export default function Page(props: PageProps): React.ReactElement
// หรือ
export default async function Page(props: PageProps): Promise<React.ReactElement>
```

### PageProps interface

```ts
interface PageProps<TParams extends RouteParams = RouteParams> {
  params: TParams // dynamic segment values
  requestPath: string // URL path ปัจจุบัน
}
```

### RouteParams type

```ts
type RouteParamValue = string | string[] | undefined
type RouteParams = Record<string, RouteParamValue>
```

### route.ts — method handlers

```ts
// app/api/users/route.ts
import type { RouteParams } from 'ruvyxa/config'

export async function GET(request: Request, { params }: { params: RouteParams }): Promise<Response>
export async function POST(request: Request, { params }: { params: RouteParams }): Promise<Response>
export async function PUT(request: Request, { params }: { params: RouteParams }): Promise<Response>
export async function DELETE(
  request: Request,
  { params }: { params: RouteParams },
): Promise<Response>
export async function PATCH(
  request: Request,
  { params }: { params: RouteParams },
): Promise<Response>
```

### ข้อกำหนดของ route.ts

- ต้อง export named functions: `GET`, `POST`, `PUT`, `DELETE`, `PATCH`
- ไม่จำเป็นต้อง export ทั้งหมด — export เฉพาะ method ที่ต้องการ
- ไม่สามารถ export `default` ใน route.ts
- request body size ถูกจำกัดโดย `security.apiLimit` (default 10 MiB)

### error.tsx — type signature

```ts
'use client'

import type { RouteErrorProps } from '@ruvyxa/react'

export default function ErrorPage({ error, reset }: RouteErrorProps): React.ReactElement
```

### RouteErrorProps

```ts
interface RouteErrorProps {
  error: Error & { digest?: string } // error object
  reset: () => void // ลอง render ใหม่
}
```

### loading.tsx — type signature

```ts
export default function Loading(): React.ReactElement
```

---

## Static Routes

Static route = URL path ตายตัว สร้างโฟลเดอร์ตาม path ที่ต้องการ:

```
app/
  page.tsx          → /
  about/
    page.tsx        → /about
  contact/
    page.tsx        → /contact
  dashboard/
    settings/
      page.tsx      → /dashboard/settings
```

ตัวอย่าง `app/contact/page.tsx`:

```tsx
export default function ContactPage() {
  return (
    <main>
      <h1>ติดต่อเรา</h1>
      <p>อีเมล: hello@ruvyxa.dev</p>
    </main>
  )
}
```

---

## Dynamic Segments [slug]

เมื่อ URL ไม่ตายตัว ใช้ `[folder]` เพื่อรับค่า dynamic:

```
app/
  blog/
    [slug]/
      page.tsx   → /blog/อะไรก็ได้
```

`params.slug` มีค่าจาก URL:

```tsx
// app/blog/[slug]/page.tsx
export default function BlogPost({ params }: { params: { slug: string } }) {
  return (
    <main>
      <h1>โพสต์: {params.slug}</h1>
      <p>slug นี้มาจาก URL</p>
    </main>
  )
}
```

| URL                 | params.slug     |
| ------------------- | --------------- |
| `/blog/hello-world` | `"hello-world"` |
| `/blog/ruvyxa-101`  | `"ruvyxa-101"`  |

ชื่อ segment ตั้งอะไรก็ได้: `[id]`, `[productId]`, `[lang]`, `[username]`

### หลาย dynamic segments

```
app/
  blog/
    [year]/
      [month]/
        [slug]/
          page.tsx   → /blog/2026/07/hello-world
```

```tsx
export default function Post({
  params,
}: {
  params: { year: string; month: string; slug: string }
}) {
  return (
    <h1>
      {params.year}/{params.month}/{params.slug}
    </h1>
  )
}
```

---

## Catch-All Routes [...slug]

จับทุก segment ที่เหลือ:

```
app/
  docs/
    [...slug]/
      page.tsx   → /docs/a, /docs/a/b, /docs/a/b/c
```

`params.slug` เป็น **array** ของ string:

```tsx
export default function DocsPage({ params }: { params: { slug: string[] } }) {
  return (
    <main>
      <h1>Docs: /{params.slug.join(' / ')}</h1>
      <pre>{JSON.stringify(params, null, 2)}</pre>
    </main>
  )
}
```

| URL                             | params.slug                         | Match?                         |
| ------------------------------- | ----------------------------------- | ------------------------------ |
| `/docs`                         | —                                   | ❌ (ต้องมีอย่างน้อย 1 segment) |
| `/docs/getting-started`         | `["getting-started"]`               | ✅                             |
| `/docs/guides/routing/overview` | `["guides", "routing", "overview"]` | ✅                             |

Catch-all ต้องเป็น segment **สุดท้ายของ path** เสมอ ไม่เช่นนั้น RUV1002

### ข้อจำกัดของ [...slug]

1. ต้องเป็น segment สุดท้ายของ path
2. ต้องมีอย่างน้อย 1 segment ใน URL (ต่างจาก `[[...slug]]`)
3. ไม่ match ถ้าไม่มี segment เลย

---

## Optional Catch-All [[...slug]]

เหมือน catch-all แต่ไม่มี segment ก็ match:

```
app/
  [[...slug]]/
    page.tsx   → /, /a, /a/b, /a/b/c
```

```tsx
export default function Page({ params }: { params: { slug?: string[] } }) {
  return (
    <main>
      <h1>slug: {params.slug?.join('/') ?? '(หน้าแรก)'}</h1>
    </main>
  )
}
```

| URL      | params.slug       | Match?                   |
| -------- | ----------------- | ------------------------ |
| `/`      | `undefined`       | ✅ (ต่างจาก `[...slug]`) |
| `/docs`  | `["docs"]`        | ✅                       |
| `/a/b/c` | `["a", "b", "c"]` | ✅                       |

Optional catch-all เหมาะสำหรับ:

- หน้าแรกที่มี route หลายระดับ
- Documentation viewer
- File browser UI

---

## ลำดับ Priority

เมื่อหลาย route match URL เดียวกัน Ruvyxa เลือกตาม priority:

```
Static route > [dynamic] > [...catchall] > [[...optional]]
```

ตัวอย่าง:

```
app/
  page.tsx           # 1. / (สูงสุด)
  blog/
    page.tsx         # 2. /blog
    [slug]/
      page.tsx       # 3. /blog/อะไรก็ได้
    [...path]/
      page.tsx       # 4. /blog/a/b/c (ต่ำสุด)
```

Priority ladder:

```
            ┌─ Static (/about)
            │
Priority    ├─ Dynamic (/blog/[slug])
(สูง→ต่ำ)   │
            ├─ Catch-all (/docs/[...slug])
            │
            └─ Optional (/[[...slug]])
```

### Priority algorithm

Ruvyxa ใช้ `route_match_shape()` function ใน `crates/ruvyxa_graph/src/lib.rs`:

- Static segment → รักษาชื่อเดิม
- Dynamic `[param]` → แทนที่ด้วย `:`
- Catch-all `[...param]` → แทนที่ด้วย `*`
- Optional `[[...param]]` → แทนที่ด้วย `*?`

```
/product/[id]      → /product/:     (dynamic)
/docs/[...slug]    → /docs/*        (catch-all)
/[[...slug]]       → /*?            (optional)
/static            → /static        (static)
```

### Conflict detection

ถ้ามีสอง route พยายาม match URL shape เดียวกันด้วย priority เท่ากัน Ruvyxa แจ้ง error
`RUV1003: Conflicting route paths`:

```bash
npm run routes  # ดู routes ที่มี
npm run check   # validate ทั้งหมด
```

ตัวอย่าง conflict:

```
app/blog/[slug]/page.tsx    → /blog/:   (dynamic)
app/blog/[postId]/page.tsx  → /blog/:   (dynamic) ← CONFLICT!
```

แก้: เปลี่ยนชื่อ dynamic segment ไม่ช่วย — `[slug]` และ `[postId]` คือ shape เดียวกัน
ต้องเปลี่ยนโครงสร้าง route หรือรวมไว้ในไฟล์เดียว

---

## Radix Trie — Under the Hood

Ruvyxa ใช้ **Radix Tree** (compressed trie) สำหรับ route matching อยู่ที่
`crates/ruvyxa_dev_server/src/router.rs`

### โครงสร้าง

```
Register routes: /, /blog, /blog/[slug], /docs/[...slug]

Root
├── "" → page (static)
└── "blog"
    ├── "" → page (static)
    └── ":" → [slug] page (dynamic)
└── "docs"
    └── "*" → [...slug] page (catch-all)
```

### ข้อดีของ Radix Trie

1. **O(k)** matching time — k = path length, ไม่ใช่ O(n) routes
2. **Prefix compression** — node ที่มี prefix ร่วมกันถูกแบ่งปัน
3. **Priority sorting** — static > dynamic > catch-all > optional
4. **Memory efficient** — ไม่ต้องเก็บ route table ซ้ำซ้อน

### Edge case: empty segment

```
app/
  page.tsx         → /
  blog/
    page.tsx       → /blog
    [slug]/
      page.tsx     → /blog/anything
```

`/blog` → match `app/blog/page.tsx` `/blog/` → 404 (trailing slash ไม่ถูก strip โดยอัตโนมัติ)

---

## Layout Nesting

Layout ซ้อนกันได้ไม่จำกัดระดับ แต่ละ layout หุ้ม child pages และ persist ขณะ navigate ภายใน subtree
เดียวกัน:

```tsx
// app/layout.tsx — ROOT LAYOUT (required)
import './globals.css'

export const meta = {
  title: 'ร้านค้าของฉัน',
  description: 'ร้านค้าออนไลน์ที่ดีที่สุด',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="th">
      <body>
        <header>เมนูหลัก</header>
        {children}
        <footer>© 2026</footer>
      </body>
    </html>
  )
}
```

```tsx
// app/blog/layout.tsx — LAYOUT สำหรับ /blog/*
export default function BlogLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="blog-container">
      <aside> sidebar </aside>
      <article>{children}</article>
    </div>
  )
}
```

```
Request: /blog/hello-world

RootLayout
  └─ <header>เมนูหลัก</header>
  └─ BlogLayout
       └─ <aside>sidebar</aside>
       └─ BlogPost(page)
  └─ <footer>© 2026</footer>
```

### Layout inheritance rules

1. **Root layout จำเป็น** — ต้องมี `app/layout.tsx` ทุกโปรเจค
2. **Layout ใช้ได้ทุกระดับ** — `app/blog/layout.tsx` หุ้ม `/blog/*`
3. **Layout persist** — ไม่ถูก unmount เมื่อ navigate ภายใน subtree เดียวกัน จาก `/blog/a` ไป
   `/blog/b` → BlogLayout ยังอยู่, มีแต่ BlogPost ที่เปลี่ยน
4. **Layout หลายระดับซ้อนกัน** — แต่ละระดับเพิ่ม wrapper
5. **Route groups มี layout ของตัวเอง** — `(marketing)/layout.tsx` ใช้แค่ routes ในกลุ่ม
6. **Layout ไม่ถูก inherit** — layout ที่ระดับหนึ่งไม่มีผลกับ sibling routes

### Layout chain algorithm

ใน `crates/ruvyxa_graph/src/lib.rs`, layout chain ถูกสร้างโดย `layout_chain()` function:

```
route: app/blog/[slug]/page.tsx

layout chain:
1. app/layout.tsx (root)
2. app/blog/layout.tsx (ถ้ามี)
```

สำหรับ `app/blog/[slug]/page.tsx` layout chain คือ `["layout.tsx", "blog/layout.tsx"]` ลำดับจาก root
→ leaf

### Root layout ต้องมี <html> และ <body>

ผิด:

```tsx
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return <div>{children}</div> // ❌ ไม่มี <html>, <body>
}
```

ถูก:

```tsx
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="th">
      <body>{children}</body>
    </html>
  )
}
```

---

## Meta Export

แต่ละ route และ layout สามารถ export `meta` เพื่อกำหนด metadata:

```tsx
import type { Meta, MetaExport, MetaFactory, MetaContext, MetaAlternate } from '@ruvyxa/react'

// Static meta
export const meta: Meta = {
  title: 'สินค้า',
  // ...
}

// Dynamic meta (function of request)
export const meta: MetaFactory = ({ params, path }) => ({
  title: `สินค้า ${params.id}`,
  // ...
})
```

### Meta interface

```ts
interface Meta {
  title?: string // Document title
  titleTemplate?: string // Format: '%s · Site' — %s = child title
  description?: string // <meta name="description">
  canonical?: string // <link rel="canonical">
  robots?: string // <meta name="robots"> verbatim
  noindex?: boolean // Shorthand: 'noindex, nofollow'
  lang?: string // <html lang> attribute
  alternates?: readonly MetaAlternate[] // hreflang entries
  image?: string // OG image URL
  imageAlt?: string // OG image alt
  siteName?: string // og:site_name
  type?: 'website' | 'article' | 'profile' // og:type
  locale?: string // og:locale (e.g. th_TH)
  card?: 'summary' | 'summary_large_image' // twitter:card
}
```

### MetaAlternate interface

```ts
interface MetaAlternate {
  hreflang: string // BCP 47 tag, หรือ 'x-default'
  href: string // Absolute URL
}
```

### MetaContext interface

```ts
interface MetaContext {
  path: string // URL path ปัจจุบัน (ex: /blog/hello)
  params: Record<string, string> // Dynamic segment values
}
```

### MetaExport type

```ts
type MetaExport = Meta | MetaFactory // สองรูปแบบที่ export meta ได้
type MetaFactory = (context: MetaContext) => Meta
```

### Meta merge algorithm

Ruvyxa merge metadata จาก layout chain root → leaf:

```
1. Root layout:    meta = { titleTemplate: '%s — Site', siteName: 'Ruvyxa' }
2. Nested layout:  meta = { description: 'Blog section' }
3. Page:           meta = { title: 'Hello World' }

Result:
  title:           'Hello World — Site'    ← titleTemplate ใช้ %s = title
  siteName:        'Ruvyxa'                ← จาก root layout
  description:     'Blog section'          ← จาก nested layout
```

กฏการ merge:

| Field           | Behavior                                                                              |
| --------------- | ------------------------------------------------------------------------------------- |
| `title`         | **Child ชนะ** — ใช้ title ของหน้าล่าสุด                                               |
| `titleTemplate` | **แม่ที่ใกล้ที่สุดมีผล** — ใช้ template จาก layout ancestor ที่ใกล้สุดที่มี field นี้ |
| `description`   | **Child ชนะ**                                                                         |
| `canonical`     | **Child ชนะ**                                                                         |
| `robots`        | **Child ชนะ**                                                                         |
| `noindex`       | **Child ชนะ** (override robots)                                                       |
| `lang`          | **Root เท่านั้น** — ตั้งจาก root layout, child ไม่ควรเปลี่ยน                          |
| `alternates`    | **รวมทุก level**                                                                      |
| `image`         | **Child ชนะ**                                                                         |
| `imageAlt`      | **Child ชนะ**                                                                         |
| `siteName`      | **Root เท่านั้น**                                                                     |
| `type`          | **Child ชนะ**                                                                         |
| `locale`        | **Child ชนะ**                                                                         |
| `card`          | **Child ชนะ**                                                                         |

### Meta with titleTemplate example

```
Root layout:   title = "Ruvyxa"           titleTemplate = "%s · Site"
Blog page:     title = "Blog Posts"
ผลลัพธ์:       <title>Blog Posts · Site</title>
```

Root layout's `title` (Ruvyxa) ถูกใช้เมื่อมี child ที่ไม่ได้ตั้ง title:

```
Child page ไม่มี title → <title>Ruvyxa · Site</title>
```

### SEO component

นอกจาก `meta` export, Ruvyxa มี `<Seo>` component สำหรับ per-render metadata:

```tsx
import { Seo } from '@ruvyxa/react'

export default function Page() {
  return (
    <main>
      <Seo
        title="Dynamic Title"
        description="This changes per render"
        jsonLd={{
          '@context': 'https://schema.org',
          '@type': 'Article',
          headline: 'Dynamic Title',
        }}
      />
      <h1>Content</h1>
    </main>
  )
}
```

ข้อควรระวัง: อย่า set field เดียวกันทั้ง `meta` export และ `<Seo>` — React จะ hoist ทั้งคู่
และตัวสุดท้ายที่ถูก mount จะชนะ

---

## Route Groups

บางครั้งอยากจัดโฟลเดอร์โดยไม่กระทบ URL ใช้ `(name)`:

```
app/
  (marketing)/
    page.tsx          → /  (ไม่ใช่ /marketing)
    about/
      page.tsx        → /about
  (dashboard)/
    settings/
      page.tsx        → /settings
    profile/
      page.tsx        → /profile
```

### Route groups semantics

1. `(name)` **ไม่มีผลต่อ URL** — ไม่เพิ่ม segment ใน path
2. Route groups **สามารถมี layout ของตัวเอง**: `(marketing)/layout.tsx` ใช้แค่ในกลุ่ม
3. Route groups **สามารถซ้อนกันได้**: `(marketing)/(blog)/...`
4. Route groups **ช่วยแบ่งทีม** — แต่ละ group มี scope ของตัวเอง
5. Route groups **สามารถมี route path ซ้ำกันไม่ได้** — ถ้า `(a)/about` และ `(b)/about` พร้อมกัน →
   RUV1003 conflict

### Route group with layout example

```
app/
  (marketing)/
    layout.tsx          ← layout สำหรับ marketing pages
    page.tsx            → /
    about/
      page.tsx          → /about
  (dashboard)/
    layout.tsx          ← layout สำหรับ dashboard pages
    settings/
      page.tsx          → /settings
```

### กฎสำคัญของ route groups

```
ถูก:  app/(marketing)/about/page.tsx   → /about
ผิด:  app/(marketing)/about/(section)/info/page.tsx  ← ซ้อน group → ทำงานได้ แต่ซับซ้อน

route groups + dynamic segments:
app/(marketing)/blog/[slug]/page.tsx   → /blog/:slug  ✅
```

---

## Private Folders และ Parallel Slots

### Private folders (_prefix)

Ruvyxa ไม่นับโฟลเดอร์ที่ขึ้นต้นด้วย `_` เป็น routes:

| Prefix           | ตัวอย่าง                        | เหตุผล                                     |
| ---------------- | ------------------------------- | ------------------------------------------ |
| `_` (underscore) | `app/_components/`, `app/_lib/` | โฟลเดอร์ส่วนตัวสำหรับ utilities            |
| `@` (at sign)    | `app/@modal/`, `app/@sidebar/`  | สงวนสำหรับ future feature (parallel slots) |

`app/_components/Button.tsx` → ไม่มี route, import ใช้ใน component อื่น

```tsx
// app/page.tsx
import Button from './_components/Button' // ✅ import ได้

export default function Page() {
  return <Button>คลิก</Button>
}
```

### Parallel slots (@prefix)

`@` สงวนสำหรับ future feature — ยังไม่ใช้งาน แต่สงวนไว้เพื่อ:

- Parallel routes (เหมือน Next.js)
- Modal/sidebar slots
- Dashboard widgets

ถ้าคุณใช้ `@` ในชื่อโฟลเดอร์ โฟลเดอร์นั้นจะถูก **ละเว้น** ไม่นับเป็น route:

```
app/
  @modal/           ← ละเว้น (future feature)
  @sidebar/         ← ละเว้น (future feature)
  page.tsx          ← /
```

---

## Link Component

ใช้ `<Link>` แทน `<a>` เพื่อ navigate แบบ client-side:

```tsx
import { Link } from '@ruvyxa/react'

export default function Nav() {
  return (
    <nav>
      <Link href="/">หน้าแรก</Link>
      <Link href="/about">เกี่ยวกับ</Link>
      <Link href="/blog/hello-world" prefetch="viewport">
        โพสต์
      </Link>
    </nav>
  )
}
```

### LinkProps interface

```ts
import type { LinkProps, LinkPrefetch } from '@ruvyxa/react'

type LinkPrefetch = boolean | 'hover' | 'viewport' | 'none'

interface LinkProps extends Omit<AnchorHTMLAttributes<HTMLAnchorElement>, 'href'> {
  href: string // ปลายทาง URL
  replace?: boolean // ใช้ history.replaceState แทน pushState
  scroll?: boolean // scroll to top หลัง navigate (default: true)
  prefetch?: LinkPrefetch // เมื่อ warm bundle (default: 'hover')
  children?: ReactNode
  ref?: Ref<HTMLAnchorElement>
}
```

### Prefetch behavior

| ค่า                 | พฤติกรรม                                                      |
| ------------------- | ------------------------------------------------------------- |
| `'hover'` (default) | Warm target bundle เมื่อ hover หรือ focus                     |
| `'viewport'`        | Warm target bundle เมื่อ link อยู่ใน viewport (+200px margin) |
| `true`              | เหมือน `'hover'`                                              |
| `'none'`            | ไม่ prefetch                                                  |
| `false`             | ไม่ prefetch                                                  |

### Prefetch mechanism

```tsx
// Link prefetch ทำงานผ่าน modulepreload hint:
<link rel="modulepreload" href="/__ruvyxa/client/route-blog-[slug].js">

// Shared chunks ก็ถูก preload ด้วย:
<link rel="modulepreload" href="/__ruvyxa/client/shared-vendors.js">
```

Prefetch ไม่ execute bundle — แค่ warm network cache และ module graph

### Edge case: modifier keys

Link ส่งต่อ browser ทันทีเมื่อ:

- `event.defaultPrevented === true`
- `event.button !== 0` (ไม่ใช่ปุ่มซ้าย)
- มี meta/ctrl/shift/alt key
- `target` ไม่ใช่ `_self`
- มี `download` attribute

---

## Router Hooks

ต้องใช้ใน `'use client'` component เท่านั้น:

```tsx
'use client'

import { useRouter, usePathname, useParams, useSearchParams, useSelectedRoute } from '@ruvyxa/react'

export default function NavigationButtons() {
  const router = useRouter()
  const pathname = usePathname()
  const params = useParams()
  const searchParams = useSearchParams()

  return (
    <div>
      <p>Path: {pathname}</p>
      <p>Params: {JSON.stringify(params)}</p>
      <p>Query: {searchParams.toString()}</p>

      <button onClick={() => router.push('/about')}>ไปหน้าเกี่ยวกับ</button>
      <button onClick={() => router.refresh()}>refresh เฉพาะ content</button>
    </div>
  )
}
```

### Type signatures

```ts
// useRouter
function useRouter(): RuvyxaRouter

interface RuvyxaRouter {
  push(href: string, options?: NavigateOptions): Promise<void>
  replace(href: string, options?: NavigateOptions): Promise<void>
  back(): void
  forward(): void
  refresh(): void // Rerender route จาก bundle ที่โหลดแล้ว
  prefetch(href: string): void // Warm route bundle
  readonly pending: boolean // กำลังโหลด bundle อยู่?
}

interface NavigateOptions {
  replace?: boolean // history.replaceState
  scroll?: boolean // scroll to top (default: true)
}

// usePathname
function usePathname(): string // pathname ปัจจุบัน

// useParams
function useParams(): RouteParams // { slug: "hello" }

// useSearchParams
function useSearchParams(): URLSearchParams // query string

// useSelectedRoute
function useSelectedRoute(): string | null // route pattern ที่ match
```

### Hook constraint

```ts
// ❌ ผิด — ต้องมี 'use client'
import { useRouter } from '@ruvyxa/react'
// → RUV1008: Server-only hook

// ✅ ถูก
;('use client')
import { useRouter } from '@ruvyxa/react'
```

### useRouter vs window.location

| Action          | useRouter                   | window.location                     |
| --------------- | --------------------------- | ----------------------------------- |
| ไปหน้า /about   | `router.push('/about')`     | `window.location.assign('/about')`  |
| แทนที่ history  | `router.replace('/about')`  | `window.location.replace('/about')` |
| กลับ            | `router.back()`             | `window.history.back()`             |
| ไปข้างหน้า      | `router.forward()`          | `window.history.forward()`          |
| Refresh content | `router.refresh()`          | `window.location.reload()`          |
| Preload bundle  | `router.prefetch('/about')` | —                                   |

---

## Route Navigation — Under the Hood

เมื่อ router.navigate(href) ถูกเรียก:

```
router.navigate('/blog/hello')
    │
    ▼
1. resolveInternalUrl(href)
   │— ตรวจว่าเป็น same-origin หรือไม่
   │— ถ้า cross-origin → window.location.assign()
   ▼
2. ensureManifest()
   │— fetch /__ruvyxa/client/route-manifest.json
   │— (ทำครั้งเดียว, cache ไว้)
   ▼
3. match(url.pathname)
   │— Radix matcher จับคู่ URL กับ route
   │— ถ้าไม่ match → hardNavigate()
   ▼
4. bundle loaded?
   │— ถ้ายังไม่โหลด → loadRoute()
   │   │— dynamic import(entry.src)
   │   │— รอจนกว่าจะมี __RUVYXA_ROUTES__[routePath]
   │— ถ้า load failed → hardNavigate()
   ▼
5. history.pushState/replaceState
   ▼
6. renderRoute(context)
   │— เรียก __RUVYXA_ROOT__.render(factory(context))
   ▼
7. scrollTo(0, 0) (ถ้า scroll !== false)
```

### Route manifest URL

```
GET /__ruvyxa/client/route-manifest.json

Response:
{
  "routes": [
    { "path": "/", "src": "/__ruvyxa/client/page-home.js", "sharedChunks": [...] },
    { "path": "/blog/[slug]", "src": "/__ruvyxa/client/page-blog-[slug].js", ... }
  ]
}
```

---

## Route Validation (RUV1100-1199)

Ruvyxa ตรวจสอบ routes โดยอัตโนมัติ:

```bash
npm run check
```

หรือ `ruvyxa check` validate:

| การตรวจสอบ                      | Error code                |
| ------------------------------- | ------------------------- |
| routes ซ้ำกัน                   | RUV1003                   |
| missing layouts                 | — (warning)               |
| imports ผิดพลาด                 | RUV1010                   |
| config ไม่ถูกต้อง               | RUV1600                   |
| server/client boundary          | RUV1007, RUV1008, RUV1009 |
| page missing default export     | RUV1004                   |
| dynamic segment ผิดรูปแบบ       | RUV1002                   |
| catch-all ไม่อยู่ตำแหน่งสุดท้าย | RUV1002                   |
| app directory ไม่มีอยู่         | RUV1001                   |

### Route validation algorithm

```
validate_app(root, manifest)
  │
  ├── แต่ละ route (page)
  │   ├── ตรวจสอบ default export (RUV1004)
  │   ├── collect_relative_graph (import tree)
  │   └── validate_client_module
  │       ├── ตรวจ 'server-only' import → RUV1007
  │       ├── ตรวจ private env → RUV1008
  │       └── ตรวจ server/ directory → RUV1010
  │
  ├── แต่ละ route (API)
  │   └── validate_server_module
  │       └── ตรวจ 'client-only' import → RUV1009
  │
  ├── server_modules
  │   └── validate_server_module
  │
  └── client_modules
      └── validate_client_module
```

---

## Route Parameters — Special Characters

Dynamic segment names ใช้ได้เฉพาะ `[a-zA-Z0-9_]`:

```
✅  app/blog/[slug]/page.tsx
✅  app/product/[productId]/page.tsx
✅  app/user/[user_name]/page.tsx
❌  app/blog/[my-slug]/page.tsx      ← hyphen ไม่ได้
❌  app/blog/[slug!]/page.tsx        ← special chars ไม่ได้
❌  app/blog/[my.slug]/page.tsx      ← dot ไม่ได้
❌  app/blog/[123]/page.tsx          ← ตัวเลขอย่างเดียวไม่ได้
```

### Slug encoding

URL-decoded values ถูกส่งไปยัง `params` โดยอัตโนมัติ:

```
URL: /blog/hello%20world
params.slug = "hello world"   ← URL decoded
```

---

## Troubleshooting

| ปัญหา                      | สาเหตุ                       | วิธีแก้                            |
| -------------------------- | ---------------------------- | ---------------------------------- |
| 404 ไม่เจอหน้า             | URL ไม่ตรงกับโฟลเดอร์        | เช็ค spelling, case-sensitive      |
| Route ซ้ำ                  | สองไฟล์ match URL เดียวกัน   | `npm run routes` หาตัวซ้ำ          |
| Dynamic segment ไม่ทำงาน   | ลืมวงเล็บ `[slug]`           | ใช้ `[square]` ไม่ใช่ `:param`     |
| Layout ไม่ซ้อน             | layout.tsx ผิดตำแหน่ง        | layout ต้องอยู่ในโฟลเดอร์ของ route |
| `useRouter()` error        | ไม่ได้ใส่ `'use client'`     | เพิ่ม directive                    |
| import .tsx ไม่เจอ         | path พิมพ์ผิด                | เช็ค relative path                 |
| params.slug เป็น undefined | ไม่ match dynamic segment    | ดูที่ `params` object ทั้งตัว      |
| ไม่เห็น route group        | ใช้ `()name` ไม่ใช่ `(name)` | วงเล็บเท่านั้น                     |
| 404 route ซ้ำกับ catch-all | Static กินก่อน catch-all     | จัด priority ใหม่                  |
| Link ไม่ navigate          | Cross-origin URL             | ใช้ relative path                  |
| router.refresh() ไม่ทำงาน  | Bundle ยังไม่ load           | รอให้ bundle โหลดก่อน              |

### ตัวอย่างข้อผิดพลาด

```tsx
// ผิด: ใช้ colon แทน bracket
// app/blog/:slug/page.tsx ← Ruvyxa ไม่รู้จัก → ไม่มี route

// ถูก: ใช้ bracket
// app/blog/[slug]/page.tsx ← ใช้ได้
```

```tsx
// ผิด: ใช้ useRouter โดยไม่มี 'use client'
import { useRouter } from '@ruvyxa/react'
// → RUV1008: Server-only hook

// ถูก:
;('use client')
import { useRouter } from '@ruvyxa/react'
```

```tsx
// ผิด: dynamic segment ชื่อซ้ำกับชื่อพิเศษ
// app/[action]/page.tsx ← action เป็น reserved name
// → RUV... (warning)

// ถูก: ใช้ชื่ออื่น
// app/[actionId]/page.tsx
```

---

## สัญญาของ Route Discovery

Route discovery ตั้งใจให้เป็น file-based และมีขอบเขตชัดเจน ภายใต้ `appDir` ที่ตั้งค่าไว้ (ปกติคือ
`app`) implementation ปัจจุบันรู้จัก route entry files ต่อไปนี้:

| ชื่อไฟล์               | ชนิด route | หมายเหตุ                                                        |
| ---------------------- | ---------- | --------------------------------------------------------------- |
| `page.tsx`, `page.jsx` | Page       | page JavaScript/TypeScript ต้องมี default export                |
| `page.md`, `page.mdx`  | Page       | เป็น content page ที่ content compiler สร้าง page component ให้ |
| `route.ts`, `route.js` | API route  | API renderer เรียก named export ตาม HTTP method                 |

ขณะเดิน tree ระบบจะข้าม directory ที่ชื่อขึ้นต้นด้วย `_` หรือ `@` ส่วน group ที่อยู่ในวงเล็บ เช่น
`(marketing)` จะถูกเดินเข้าไป แต่ไม่กลายเป็นส่วนของ URL จึงใช้จัดระเบียบไฟล์ ไม่ใช่ URL segment:

```text
app/(marketing)/pricing/page.tsx  ->  /pricing
app/(marketing)/layout.tsx        ->  อยู่ใน layout chain
```

### กฎ Dynamic Segment รวมกรณีที่ใช้ไม่ได้

ใช้ `[name]` สำหรับ segment เดียว, `[...name]` สำหรับ segments ที่เหลืออย่างน้อยหนึ่ง segment และ
`[[...name]]` สำหรับ catch-all ที่ว่างได้ catch-all ต้องเป็น visible segment สุดท้าย เพราะมันกิน
ส่วนที่เหลือของ path ทั้งหมด:

```text
app/products/[id]/page.tsx              -> /products/[id]
app/docs/[...parts]/page.tsx             -> /docs/[...parts]
app/docs/[[...parts]]/page.tsx           -> /docs/[[...parts]]
app/docs/[...parts]/edit/page.tsx        -> ใช้ไม่ได้: catch-all ไม่ได้อยู่ท้ายสุด
```

ชื่อ parameter ห้ามว่าง, ห้ามมี bracket ซ้อน และห้ามขึ้นต้นด้วย `.` ข้อผิดพลาดเหล่านี้เกิดตอน route
discovery ดังนั้นให้แก้ชื่อ directory ก่อนเริ่ม debug เรื่อง rendering

### Layouts และ Modules เฉพาะ Route

สำหรับทุก route Ruvyxa จะรวม `layout.tsx` ตั้งแต่ root ของแอปลงมาถึง directory ของ route นั้น
directory ของ route สามารถมี `server.ts`/`server.js`, `action.ts`/`action.js` และ `client.tsx`
ได้ด้วย ไฟล์เหล่านี้จะเข้าสู่ server หรือ client module list ของ manifest เก็บ concern ที่ใช้กับ
route เดียวไว้ด้วยกัน และย้าย code ที่ใช้ร่วมกันกว้าง ๆ ออกนอก directory ของ route

```text
app/blog/[slug]/
  layout.tsx       # เพิ่ม nested layout ให้ branch นี้
  page.tsx         # page entry
  action.ts        # actions ของ page route นี้
  client.tsx       # client entry เฉพาะ route เมื่อจำเป็น
```

### ตรวจ Manifest ก่อนสรุปว่า URL มีอยู่

เรียก CLI โดยตรง ไม่ต้องสมมติว่ามี package script:

```bash
ruvyxa routes
ruvyxa trace /blog/[slug]
```

`routes` พิมพ์ route table ที่ค้นพบ ส่วน `trace` รับ route pattern ที่อยู่ในตาราง ไม่ใช่ชื่อ
component file หรือ parameter จริง หาก directory ใหม่ไม่ปรากฏ ให้ตรวจชื่อ entry file และดูว่า
ancestor directory มีชื่อขึ้นต้นด้วย `_` หรือ `@` หรือไม่ก่อน

---

## ขั้นตอนถัดไป

- **[03-server-client-components.md](./03-server-client-components.md)** — Server vs Client
  components
- **[04-rendering-strategies.md](./04-rendering-strategies.md)** — SSR, SSG, ISR, PPR, CSR
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — โหลดข้อมูลและ cache
