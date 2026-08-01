# 17 — เอกสารอ้างอิง API

**Ruvyxa 1.0**

เอกสารอ้างอิงครบถ้วนสำหรับแพ็กเกจ `@ruvyxa/react`, `@ruvyxa/core` และแพ็กเกจที่เกี่ยวข้อง
ครอบคลุมทุก component, hook, function, type และ constant ที่ถูก export ออกมา

---

## สารบัญ

1. [@ruvyxa/react — Components](#1-ruvyxareact--components)
2. [@ruvyxa/react — Hooks](#2-ruvyxareact--hooks)
3. [@ruvyxa/react — Functions & Constants](#3-ruvyxareact--functions--constants)
4. [@ruvyxa/core/server](#4-ruvyxacoreserver)
5. [@ruvyxa/core/config](#5-ruvyxacoreconfig)
6. [@ruvyxa/core — Utilities](#6-ruvyxacore--utilities)
7. [@ruvyxa/core/plugin-harness](#7-ruvyxacoreplugin-harness)
8. [@ruvyxa/core/plugin](#8-ruvyxacoreplugin)
9. [Full Type Reference](#9-full-type-reference)

---

## 1. @ruvyxa/react — Components

นำเข้าจาก `@ruvyxa/react`

### RuvyxaErrorBoundary

Error boundary สำหรับ React component จับ error ระหว่าง render และแสดง fallback UI แทนการพังทั้งหน้า

```tsx
import { RuvyxaErrorBoundary } from '@ruvyxa/react'

;<RuvyxaErrorBoundary
  fallback={({ error, resetError }) => (
    <div>
      <p>เกิดข้อผิดพลาด: {error.message}</p>
      <button onClick={resetError}>ลองอีกครั้ง</button>
    </div>
  )}
  onError={(error, info) => {
    sendToLoggingService(error, info.componentStack)
  }}
>
  <App />
</RuvyxaErrorBoundary>
```

**Props:**

| Prop       | Type                                       | คำอธิบาย                    |
| ---------- | ------------------------------------------ | --------------------------- |
| `children` | `ReactNode`                                | เนื้อหาที่ต้องการป้องกัน    |
| `fallback` | `(props: ErrorFallbackProps) => ReactNode` | UI ที่แสดงเมื่อเกิด error   |
| `onError`  | `(error: Error, info: ErrorInfo) => void`  | callback เมื่อจับ error ได้ |

### Image

แสดงผลภาพพร้อมรองรับ WebP จาก build output และ responsive srcset อัตโนมัติ

```tsx
import { Image } from '@ruvyxa/react'

<Image
  src="/hero.png"
  alt="ภาพหลัก"
  width={1200}
  height={600}
  priority
  sizes="(max-width: 768px) 100vw, 1200px"
/>

// แบบ fill (ต้องมี parent ที่มี position: relative)
<Image src="/bg.jpg" alt="พื้นหลัง" fill />
```

**Props ที่สำคัญ:**

| Prop          | Type          | ค่าเริ่มต้น | คำอธิบาย                             |
| ------------- | ------------- | ----------- | ------------------------------------ |
| `src`         | `string`      | —           | URL ภาพ รองรับเส้นทาง local PNG/JPEG |
| `alt`         | `string`      | —           | ข้อความทดแทน (required)              |
| `width`       | `number`      | —           | ความกว้างจริง (required ยกเว้น fill) |
| `height`      | `number`      | —           | ความสูงจริง (required ยกเว้น fill)   |
| `unoptimized` | `boolean`     | `false`     | ไม่แปลงเป็น WebP                     |
| `priority`    | `boolean`     | `false`     | eager-load + fetchPriority high      |
| `fill`        | `boolean`     | `false`     | เติมพื้นที่ parent                   |
| `loader`      | `ImageLoader` | —           | ใช้ image CDN loader                 |
| `quality`     | `number`      | —           | ส่งให้ custom loader                 |

### Picture

`<picture>` element สำหรับ art direction พร้อมการแปลง URL เหมือน `Image`

```tsx
import { Picture } from '@ruvyxa/react'

;<Picture
  src="/photo.jpg"
  alt="รูปถ่าย"
  width={800}
  height={600}
  sources={[
    {
      media: '(min-width: 768px)',
      srcSet: '/photo-wide.jpg 800w, /photo-wide-640w.webp 640w',
    },
    {
      media: '(max-width: 767px)',
      srcSet: '/photo-square.jpg 400w',
    },
  ]}
/>
```

### Seo

Component สำหรับใส่ metadata SEO, Open Graph, Twitter Card และ JSON-LD structured data

```tsx
import { Seo, type SeoArticle } from '@ruvyxa/react'

;<Seo
  title="หน้าแรก — เว็บไซต์ของฉัน"
  description="คำอธิบายเว็บไซต์"
  canonical="https://example.com"
  image="/og-image.png"
  siteName="เว็บไซต์ของฉัน"
  type="website"
  locale="th_TH"
  noindex={false}
  card="summary_large_image"
  article={{
    publishedAt: '2025-01-15',
    authors: [{ name: 'สมชาย', url: 'https://example.com/author/somchai' }],
    tags: ['เทคโนโลยี', 'React'],
  }}
  breadcrumbs={[
    { name: 'หน้าแรก', url: '/' },
    { name: 'บล็อก', url: '/blog' },
  ]}
  jsonLd={{
    '@type': 'WebSite',
    name: 'เว็บไซต์ของฉัน',
  }}
/>
```

**SeoArticle:**

```tsx
interface SeoArticle {
  type?: 'Article' | 'BlogPosting' | 'NewsArticle'
  publishedAt?: string
  updatedAt?: string
  authors?: readonly SeoAuthor[]
  section?: string
  tags?: readonly string[]
}
```

**SeoBreadcrumb:**

```tsx
interface SeoBreadcrumb {
  name: string
  url: string
}
```

### Link

Component สำหรับ navigation ระหว่าง route แบบ client-side โดยยังคง `<a>` HTML element จริง

```tsx
import { Link } from '@ruvyxa/react'

<Link href="/">หน้าแรก</Link>
<Link href="/blog/post-1" prefetch="viewport" replace={false} scroll>
  อ่านบทความ
</Link>
```

**Props:**

| Prop       | Type           | ค่าเริ่มต้น | คำอธิบาย                    |
| ---------- | -------------- | ----------- | --------------------------- |
| `href`     | `string`       | —           | URL ปลายทาง                 |
| `replace`  | `boolean`      | `false`     | แทนที่ history entry        |
| `scroll`   | `boolean`      | `true`      | เลื่อนไปด้านบนหลัง navigate |
| `prefetch` | `LinkPrefetch` | `'hover'`   | โหมด warming bundle         |

**LinkPrefetch:** `boolean \| 'hover' \| 'viewport' \| 'none'`

- `true` / `'hover'` — prefetch เมื่อ hover หรือ focus
- `'viewport'` — prefetch เมื่อ link ปรากฏใน viewport
- `false` / `'none'` — ปิด prefetch

### Answer

Component แสดง Q&A พร้อม Schema.org microdata และแหล่งอ้างอิง

```tsx
import { Answer } from '@ruvyxa/react'

;<Answer
  question="Ruvyxa คืออะไร?"
  answer="Ruvyxa คือเฟรมเวิร์ก React ที่ออกแบบมาเพื่อความเร็วและการใช้งานจริง"
  sources={[{ name: 'เอกสารทางการ', url: 'https://ruvyxa.dev' }]}
/>
```

**Props:**

| Prop           | Type             | คำอธิบาย                                        |
| -------------- | ---------------- | ----------------------------------------------- |
| `question`     | `string`         | คำถาม                                           |
| `answer`       | `ReactNode`      | คำตอบ (หรือใช้ children แทน)                    |
| `children`     | `ReactNode`      | เนื้อหาตอบแทนถ้าไม่ใช้ `answer`                 |
| `sources`      | `AnswerSource[]` | แหล่งอ้างอิง                                    |
| `sourcesLabel` | `ReactNode`      | ป้ายหัวข้อแหล่งอ้างอิง (ค่าเริ่มต้น: "Sources") |
| `id`           | `string`         | ID สำหรับ anchor                                |
| `className`    | `string`         | CSS class                                       |

---

## 2. @ruvyxa/react — Hooks

### useRouteContext

อ่านค่า route context ปัจจุบัน

```tsx
import { useRouteContext } from '@ruvyxa/react'

function Component() {
  const { pathname, params, route } = useRouteContext()
  return <p>เส้นทาง: {pathname}</p>
}
```

**คืนค่า:** `RouteContextValue`

```tsx
interface RouteContextValue {
  pathname: string
  params: RouteParams
  route: string
}
```

### usePathname

คืนค่า pathname ปัจจุบัน

```tsx
import { usePathname } from '@ruvyxa/react'

const pathname = usePathname()
```

### useParams

คืนค่า parameters จาก route pattern ที่ match

```tsx
import { useParams } from '@ruvyxa/react'

const params = useParams()
// params.slug สำหรับ route `/blog/[slug]`
```

### useSearchParams

คืนค่า query string เป็น `URLSearchParams`

```tsx
import { useSearchParams } from '@ruvyxa/react'

const searchParams = useSearchParams()
const page = searchParams.get('page') // "2"
```

### useSelectedRoute

คืนค่า route pattern ที่ match ปัจจุบัน เช่น `/blog/[slug]`

```tsx
import { useSelectedRoute } from '@ruvyxa/react'

const route = useSelectedRoute()
```

### useRouter

คืนค่า API สำหรับ imperative navigation

```tsx
import { useRouter } from '@ruvyxa/react'

function Nav() {
  const router = useRouter()
  return (
    <>
      <span>{router.pending ? 'กำลังโหลด...' : 'พร้อม'}</span>
      <button onClick={() => router.push('/about')}>เกี่ยวกับ</button>
      <button onClick={() => router.replace('/')}>กลับหน้าแรก</button>
      <button onClick={router.back}>ย้อนกลับ</button>
      <button onClick={() => router.prefetch('/blog')}>อุ่นแคช blog</button>
    </>
  )
}
```

**RuvyxaRouter:**

```tsx
interface RuvyxaRouter {
  push(href: string, options?: NavigateOptions): Promise<void>
  replace(href: string, options?: NavigateOptions): Promise<void>
  back(): void
  forward(): void
  refresh(): void
  prefetch(href: string): void
  readonly pending: boolean
}
```

### useRuvyxaLoader

เรียกใช้ loader ฝั่ง client พร้อม state data/loading/error

```tsx
import { useRuvyxaLoader } from '@ruvyxa/react'

function UserProfile({ userId }: { userId: string }) {
  const { data, loading, error, refetch } = useRuvyxaLoader(
    () => fetch(`/api/users/${userId}`).then((r) => r.json()),
    { deps: [userId] },
  )

  if (loading) return <p>กำลังโหลด...</p>
  if (error) return <p>ข้อผิดพลาด: {error.message}</p>
  return <div>{data.name}</div>
}
```

**UseLoaderOptions:**

| Prop      | Type        | ค่าเริ่มต้น | คำอธิบาย                    |
| --------- | ----------- | ----------- | --------------------------- |
| `enabled` | `boolean`   | `true`      | ถ้า `false` ไม่เรียก loader |
| `deps`    | `unknown[]` | `[]`        | dependencies สำหรับ refetch |

**UseLoaderResult\<T\>:**

```tsx
interface UseLoaderResult<T> {
  data: T | undefined
  loading: boolean
  error: Error | undefined
  refetch: () => void
}
```

---

## 3. @ruvyxa/react — Functions & Constants

### hydrate()

ส่งสัญญาณว่า hydration เสร็จสมบูรณ์ ใช้ลงทะเบียน error handler

```tsx
import { hydrate } from '@ruvyxa/react'

hydrate({
  onError: (error, { componentStack }) => {
    reportError(error, componentStack)
  },
})
```

### reportHydrationError()

รายงาน hydration mismatch error ไปยัง global error handler

```tsx
import { reportHydrationError } from '@ruvyxa/react'

try {
  // logic ที่อาจ mismatch
} catch (e) {
  reportHydrationError(e, { componentStack: stack })
}
```

### notFound()

โยน error เพื่อแสดง `not-found.tsx`

```tsx
import { notFound } from '@ruvyxa/react'

async function Page({ params }: { params: { slug: string } }) {
  const post = await getPost(params.slug)
  if (!post) notFound() // แสดง not-found.tsx
  return <Article post={post} />
}
```

### isNotFoundError()

ตรวจสอบว่า error ค่าที่ได้รับมาจาก `notFound()` หรือไม่

```tsx
import { isNotFoundError } from '@ruvyxa/react'

if (isNotFoundError(error)) {
  // handle not-found
}
```

### compilePattern()

แปลง route pattern เป็น RegExp สำหรับ matching

```tsx
import { compilePattern } from '@ruvyxa/react'

const pattern = compilePattern('/blog/[slug]')
// -> { regex: /^\/blog\/([^/]+)\/?$/, paramNames: ['slug'], catchAll: null }
```

### routeSpecificity()

คำนวณ specificity vector สำหรับ route pattern

```tsx
import { routeSpecificity } from '@ruvyxa/react'

routeSpecificity('/blog/new') // [0, 0]  (static segment)
routeSpecificity('/blog/[slug]') // [0, 1]  (dynamic segment)
```

Return: `number[]` โดย static = 0, dynamic = 1, catch-all = 2, optional catch-all = 3

### compareSpecificity()

เปรียบเทียบ specificity vector สองค่า ใช้เรียงลำดับ route

```tsx
import { compareSpecificity, routeSpecificity } from '@ruvyxa/react'

const a = routeSpecificity('/blog/new')
const b = routeSpecificity('/blog/[slug]')
compareSpecificity(a, b) // -1 เพราะ a ชนะ (เจาะจงกว่า)
```

### normalizeMatchPath()

ทำให้ pathname เป็นมาตรฐานสำหรับการ match

```tsx
import { normalizeMatchPath } from '@ruvyxa/react'

normalizeMatchPath('/docs//a/') // "/docs/a"
normalizeMatchPath('/') // "/"
```

### createRouteMatcher()

สร้าง matcher function จากรายการ route

```tsx
import { createRouteMatcher } from '@ruvyxa/react'

const matcher = createRouteMatcher([
  { path: '/', src: '/routes/index.js' },
  { path: '/blog/[slug]', src: '/routes/blog/[slug].js' },
])

const match = matcher('/blog/hello')
if (match) {
  console.log(match.route.path) // "/blog/[slug]"
  console.log(match.params.slug) // "hello"
}
```

### Constants

```tsx
import { DEFAULT_DEVICE_WIDTHS, NOT_FOUND_PROPERTY, RouteContext } from '@ruvyxa/react'
```

| Constant                | Type                                 | ค่า                                             |
| ----------------------- | ------------------------------------ | ----------------------------------------------- |
| `DEFAULT_DEVICE_WIDTHS` | `readonly number[]`                  | `[640, 750, 828, 1080, 1200, 1920, 2048, 3840]` |
| `NOT_FOUND_PROPERTY`    | `'__ruvyxaNotFound'`                 | ใช้ตรวจสอบ not-found error                      |
| `RouteContext`          | `Context<RouteContextValue \| null>` | React context สำหรับ route                      |

### RouteErrorProps

Props ที่ส่งให้ `error.tsx`

```tsx
interface RouteErrorProps {
  error: Error
  reset: () => void
}
```

### Meta, MetaFactory, MetaExport

Types สำหรับ `export const meta` ของแต่ละ route

```tsx
interface Meta {
  title?: string
  titleTemplate?: string
  description?: string
  canonical?: string
  robots?: string
  noindex?: boolean
  lang?: string
  alternates?: readonly MetaAlternate[]
  image?: string
  imageAlt?: string
  siteName?: string
  type?: 'website' | 'article' | 'profile'
  locale?: string
  card?: 'summary' | 'summary_large_image'
}

type MetaFactory = (context: MetaContext) => Meta
type MetaExport = Meta | MetaFactory
```

---

## 4. @ruvyxa/core/server

นำเข้าจาก `@ruvyxa/core/server`

### loader()

สร้าง server loader ที่เรียกใช้ระหว่าง render

```tsx
import { loader } from '@ruvyxa/core/server'

export const getUsers = loader(async ({ params, request, cache }) => {
  const users = await db.users.findMany()
  return { users }
})

// เรียกใช้ใน component
const data = await getUsers({ params: { id: '1' } })
```

**LoaderContext:**

```tsx
interface LoaderContext {
  params: Record<string, string>
  request: Request
  cache: typeof cache
}
```

### action()

สร้าง server action

```tsx
import { action } from '@ruvyxa/core/server'

export const createUser = action
  .input(z.object({ name: z.string() }))
  .handler(async ({ input, request, user, invalidate }) => {
    const user = await db.users.create({ data: { name: input.name } })
    invalidate('users')
    return { success: true, user }
  })
```

**ActionBuilder chain:** `action.input(schema).realtime(channels?).handler(fn)`

**ActionContext:**

```tsx
interface ActionContext<TInput> {
  input: TInput
  request: Request
  user?: unknown
  invalidate(key: string): void
}
```

### cache()

สร้าง cache entry แบบในหน่วยความจำ พร้อม TTL, stale-while-revalidate และ LRU eviction

```tsx
import { cache } from '@ruvyxa/core/server'

const data = await cache('users:list')
  .ttl('5m')
  .swr('1m')
  .get(async () => {
    return db.users.findMany()
  })
```

**CacheBuilder:**

```tsx
interface CacheBuilder {
  ttl(value: string): CacheBuilder // "30s", "5m", "1h", "1d"
  swr(value: string): CacheBuilder // stale-while-revalidate window
  get<T>(producer: () => T | Promise<T>): Promise<T>
}
```

### invalidateCache()

ล้าง cache ทั้งหมด หรือตาม key/prefix

```tsx
import { invalidateCache } from '@ruvyxa/core/server'

invalidateCache() // clear ทั้ง cache
invalidateCache('users') // ลบ key "users" และ "users:*"
```

### cacheStats()

ดูสถานะ cache สำหรับ observability

```tsx
import { cacheStats } from '@ruvyxa/core/server'

const stats = cacheStats()
// { size: 42, maxEntries: 1024 }
```

### redirect()

สร้าง Response สำหรับ redirect

```tsx
import { redirect } from '@ruvyxa/core/server'

export const loader = () => {
  return redirect('/login', 302)
}
```

### notFound()

สร้าง Response 404

```tsx
import { notFound } from '@ruvyxa/core/server'

export const loader = () => {
  return notFound('ไม่พบหน้าที่ขอ')
}
```

### json()

สร้าง Response แบบ JSON

```tsx
import { json } from '@ruvyxa/core/server'

export const loader = () => {
  return json({ success: true, data: [...] }, { status: 200 })
}
```

### Types

```tsx
import type {
  LoaderContext,
  LoaderHandler,
  Loader,
  ActionContext,
  ActionBuilder,
  ServerAction,
  CacheBuilder,
  CacheEntry,
  Schema,
} from '@ruvyxa/core/server'
```

---

## 5. @ruvyxa/core/config

นำเข้าจาก `@ruvyxa/core/config`

### config()

ฟังก์ชันสำหรับสร้าง typed config ใน `ruvyxa.config.ts`

```tsx
import { config } from '@ruvyxa/core/config'
import type { RuvyxaConfig } from '@ruvyxa/core/config'

export default config({
  appDir: 'app',
  server: { port: 3000 },
  site: { url: 'https://example.com' },
} satisfies RuvyxaConfig)
```

### Re-exported Types (19 types)

| Type                        | คำอธิบาย                                                   |
| --------------------------- | ---------------------------------------------------------- |
| `RuvyxaConfig`              | การตั้งค่า config หลัก                                     |
| `SiteConfig`                | การตั้งค่า site URL, sitemap, robots                       |
| `MiddlewareConfig`          | การตั้งค่า middleware                                      |
| `BuiltinMiddlewareConfig`   | การตั้งค่า built-in middleware (CORS, rate limit, headers) |
| `CorsConfig`                | การตั้งค่า CORS                                            |
| `RateLimitConfig`           | การตั้งค่า rate limiting                                   |
| `RenderConfig`              | การตั้งค่ารูปแบบ render                                    |
| `RenderStrategy`            | `'ssr' \| 'ssg' \| 'isr' \| 'csr' \| 'ppr'`                |
| `ImageConfig`               | การตั้งค่ารูปภาพ                                           |
| `PageProps`                 | Props ที่ส่งให้ page component                             |
| `RouteParams`               | `Record<string, RouteParamValue>`                          |
| `RouteParamValue`           | `string \| string[] \| undefined`                          |
| `StaticParamsContext`       | Context สำหรับ `getStaticParams`                           |
| `StaticParamSegment`        | Segment แบบ dynamic                                        |
| `StaticParamsResult`        | ค่าที่ส่งกลับจาก `getStaticParams`                         |
| `StaticParamsValues`        | ค่า static params                                          |
| `StaticParamsCacheDuration` | ระยะเวลา cache                                             |
| `CachedStaticParams`        | ค่า static params แบบมี cache                              |
| `TransformResult`           | ผลลัพธ์จากการ transform โค้ด                               |

---

## 6. @ruvyxa/core — Utilities

นำเข้าจาก `@ruvyxa/core`

### Constants

```tsx
import {
  STATIC_ASSET_EXTENSIONS,
  CLIENT_BUNDLE_PREFIX,
  IMMUTABLE_CACHE_CONTROL,
  PUBLIC_ASSET_CACHE_CONTROL,
  DEFAULT_SECURITY_HEADERS,
} from '@ruvyxa/core'
```

| Constant                     | Type                | ค่า                                                                      |
| ---------------------------- | ------------------- | ------------------------------------------------------------------------ |
| `STATIC_ASSET_EXTENSIONS`    | `readonly string[]` | extensions สำหรับ static asset (`'apng'`, `'css'`, `'js'`, `'png'`, ฯลฯ) |
| `CLIENT_BUNDLE_PREFIX`       | `string`            | `'/__ruvyxa/client/'`                                                    |
| `IMMUTABLE_CACHE_CONTROL`    | `string`            | `'public, max-age=31536000, immutable'`                                  |
| `PUBLIC_ASSET_CACHE_CONTROL` | `string`            | `'public, max-age=3600, must-revalidate'`                                |
| `DEFAULT_SECURITY_HEADERS`   | `object`            | security headers เริ่มต้น                                                |

### Functions

#### staticAssetPattern()

คืนค่า PCRE regex pattern สำหรับ match static asset URL

```tsx
import { staticAssetPattern } from '@ruvyxa/core'

staticAssetPattern()
// ^/(?!__ruvyxa/).+\.(?:apng|avif|bmp|css|...)$
```

#### staticAssetGlobs()

คืนค่ารายการ glob สำหรับ static assets

```tsx
import { staticAssetGlobs } from '@ruvyxa/core'

staticAssetGlobs()
// ['/*.apng', '/*.css', '/*.js', '/*.png', ...]
```

#### publicAssetGlobs()

คืนค่ารายการ glob สำหรับ public assets (ไม่รวม js/css/mjs/map)

```tsx
import { publicAssetGlobs } from '@ruvyxa/core'

publicAssetGlobs()
// ['/*.png', '/*.jpg', '/*.svg', '/*.woff2', ...]
```

#### headersFileContents()

สร้างเนื้อหาไฟล์ `_headers` สำหรับ static hosting

```tsx
import { headersFileContents } from '@ruvyxa/core'

const contents = headersFileContents()
// /*
//   X-Content-Type-Options: nosniff
//   ...
```

#### clientBuildOutput()

คืนค่าเส้นทาง client build output สำหรับ deployment adapters

```tsx
import { clientBuildOutput } from '@ruvyxa/core'

const output = clientBuildOutput({ root: '/project', outDir: '/project/.ruvyxa' })
// { clientDir: '/project/.ruvyxa/client', chunkManifest: '/project/.ruvyxa/client/chunk-manifest.json' }
```

#### projectRelativeOutDir()

แปลง `outDir` เป็น path สัมพัทธ์กับ project root แบบ POSIX

```tsx
import { projectRelativeOutDir } from '@ruvyxa/core'

const relative = projectRelativeOutDir({
  root: '/project',
  outDir: '/project/.ruvyxa',
})
// ".ruvyxa"
```

#### validateBuildContext()

ตรวจสอบ `BuildContext` ว่ามี `root` และ `outDir` ที่ถูกต้อง

```tsx
import { validateBuildContext } from '@ruvyxa/core'

validateBuildContext(ctx, 'my-adapter')
// throws หาก root หรือ outDir ไม่ถูกต้อง
```

#### standaloneServerSource()

สร้าง source code สำหรับ standalone HTTP server (Node/Bun)

```tsx
import { standaloneServerSource } from '@ruvyxa/core'

const source = standaloneServerSource({ isrCache: 'tmp' })
// string ที่เป็น JavaScript source ของ server
```

#### withResponseHeader()

คืนค่า Response ใหม่ที่เพิ่มหรือแทนที่ header

```tsx
import { withResponseHeader } from '@ruvyxa/core'

const response = withResponseHeader(originalResponse, 'X-Custom', 'value')
```

---

## 7. @ruvyxa/core/plugin-harness

นำเข้าจาก `@ruvyxa/core/plugin-harness`

### createPluginHarness()

สร้าง test harness สำหรับ plugin โดยไม่ต้องรัน server จริง

```tsx
import { createPluginHarness } from '@ruvyxa/core/plugin-harness'
import type { RuvyxaPlugin } from '@ruvyxa/core'

const plugin: RuvyxaPlugin = {
  name: 'my-plugin',
  register(api) {
    api.http.onResponse(({ response }) => withResponseHeader(response, 'x-plugin', 'active'))
  },
}

const harness = await createPluginHarness(plugin)

// ทดสอบ response hooks
const response = await harness.respond(new Response('ok'), '/api/test')
console.log(response.headers.get('x-plugin')) // "active"

// ทดสอบ request hooks
const { request, response: shortCircuit } = await harness.request('/test')

// ทดสอบ route handler
const routeResponse = await harness.route('/api/hello', { method: 'GET' })

// ส่ง event file change
await harness.fileChange(['src/file.ts'])

// ทดสอบ build hooks
await harness.build.start()
await harness.build.transform('const x = 1', 'src/file.ts')
await harness.build.complete()
```

### PluginHarness Members

```tsx
interface PluginHarness {
  readonly head: readonly PluginHeadEntry[]
  readonly routes: readonly PluginHttpRouteRegistration[]
  readonly diagnostics: readonly HarnessDiagnostic[]
  readonly nativeClaims: readonly HarnessNativeClaim[]

  request(input, options?): Promise<{ response?: Response; request: Request }>
  respond(response, input?, options?): Promise<Response>
  route(input, options?): Promise<Response | undefined>
  fileChange(change): Promise<void>

  readonly build: {
    start(options?): Promise<void>
    resolve(id, importer?, options?): Promise<string | null>
    load(id, options?): Promise<TransformResult | null>
    transform(code, id, options?): Promise<TransformResult | null>
    complete(options?): Promise<void>
  }
}
```

### Harness Types

```tsx
interface HarnessNativeClaim {
  plugin: string
  capability: PluginNativeCapability
  options: RealtimePluginOptions
}

interface HarnessDiagnostic extends PluginDiagnostic {
  plugin: string
}

interface HarnessRequestOptions {
  path?: string
  method?: string
  headers?: HeadersInit
  body?: BodyInit | null
}

interface HarnessBuildOptions {
  root?: string
  outDir?: string
  manifest?: Record<string, unknown>
  environment?: PluginEnvironment
}
```

---

## 8. @ruvyxa/core/plugin

นำเข้าจาก `@ruvyxa/core/plugin`

### definePlugin()

สร้าง RuvyxaPlugin จาก concise declaration หรือ advanced socket API

```tsx
import { definePlugin } from '@ruvyxa/core/plugin'
import type { RuvyxaPluginDefinition } from '@ruvyxa/core/plugin'

const plugin = definePlugin({
  name: 'analytics',
  headers: { 'X-Analytics': 'enabled' },
  head: [
    {
      tag: 'script',
      attrs: { src: 'https://cdn.analytics.com/script.js', async: true },
    },
  ],
  http: {
    match: ['/api/*'],
    onRequest: (ctx) => {
      console.log(`${ctx.request.method} ${ctx.request.url}`)
    },
  },
  build: {
    onStart: (ctx) => console.log(`Build started: ${ctx.outDir}`),
    onComplete: (ctx) => console.log(`Build complete: ${ctx.outDir}`),
  },
  dev: {
    onFileChange: { match: ['*.ts'], handler: (ctx) => console.log(ctx.paths) },
  },
  diagnostics: { level: 'info', code: 'MY001', message: 'plugin loaded' },
  native: { realtime: { path: '/ws', heartbeatMs: 30000 } },
  register: async (api) => {
    // advanced API สำหรับลงทะเบียน hooks ซ้ำกัน
    api.http.onRequest({ match: ['/admin/*'], handler: (ctx) => ctx.request })
  },
})
```

### RuvyxaPluginDefinition

```tsx
interface RuvyxaPluginDefinition {
  name: string
  headers?: HeadersInit
  head?: PluginHeadEntry | readonly PluginHeadEntry[]
  http?: PluginHttpDefinition
  build?: PluginBuildDefinition
  dev?: PluginDevDefinition
  diagnostics?: PluginDiagnostic | readonly PluginDiagnostic[]
  native?: PluginNativeDefinition
  register?(api: PluginRegistrationApi): void | Promise<void>
}
```

### PluginHeadEntry

```tsx
interface PluginHeadEntry {
  tag: 'link' | 'meta' | 'noscript' | 'script' | 'style'
  attrs?: Record<string, string | number | boolean>
  children?: string // สำหรับ script, style, noscript เท่านั้น
}
```

---

## 9. Full Type Reference

### Adapter

```tsx
interface Adapter {
  name: string
  target: 'node' | 'edge' | 'serverless' | 'static'
  supports?: Array<RenderStrategy | 'api'>
  build(ctx: BuildContext): AdapterOutput | Promise<AdapterOutput>
}
```

### AdapterOutput

```tsx
interface AdapterOutput {
  name: string
  target: Adapter['target']
  entry: string
  assetsDir: string
  clientDir?: string
  chunkManifest?: string
  platform?:
    | 'node'
    | 'vercel'
    | 'cloudflare'
    | 'netlify'
    | 'bun'
    | 'static'
    | 'railway'
    | 'render'
    | 'firebase'
    | 'aws'
  runtime?: 'node' | 'bun'
  configFiles?: string[]
  functionsDir?: string
  artifacts?: AdapterArtifact[]
}
```

### AdapterArtifact

```tsx
interface AdapterArtifact {
  kind: 'file' | 'static-site' | 'function'
  path: string
  contents?: string
  handlerSource?: string
  scope?: 'build' | 'project'
  skipIfExists?: boolean
  optional?: boolean
  excludeStrategies?: string[]
}
```

### BuildContext

```tsx
interface BuildContext {
  root: string
  outDir: string
  chunkManifest?: string
}
```

### RuvyxaConfig

```tsx
interface RuvyxaConfig {
  appDir?: string
  outDir?: string
  runtime?: 'node' | 'bun' | 'edge' | 'static'
  react?: boolean
  typescript?: { strict?: boolean }
  css?: { entries?: string[] }
  server?: { port?: number; host?: string }
  build?: {
    minify?: boolean
    map?: boolean
    treeShake?: boolean
    split?: 'single' | 'route' | 'manual'
    workers?: number
    jsx?: 'classic' | 'automatic'
    target?: 'es2018' | 'es2019' | 'es2020' | 'es2022' | 'esnext'
    manifest?: boolean
    warm?: boolean
    prerenderCache?: boolean
  }
  render?: RenderConfig
  debug?: { overlay?: boolean; traces?: boolean }
  image?: ImageConfig
  security?: {
    actionLimit?: number
    apiLimit?: number
    pluginLimit?: number
    actionRateLimit?: { max?: number; window?: number }
    sameOrigin?: boolean
    fetchMeta?: boolean
    trustedProxyIps?: string[]
    headers?: boolean
  }
  cache?: { routes?: boolean; css?: boolean; dir?: string }
  site?: SiteConfig
  middleware?: MiddlewareConfig
  adapter?: Adapter
  adapterOptions?: Record<string, unknown>
  plugins?: RuvyxaPlugin[]
}
```

### SiteConfig

```tsx
interface SiteConfig {
  url?: string
  sitemap?: boolean | SiteSitemapConfig
  robots?: boolean | SiteRobotsConfig
}
```

### MiddlewareConfig

```tsx
interface MiddlewareConfig {
  builtin?: BuiltinMiddlewareConfig
  workers?: number
  timeoutMs?: number
}
```

### BuiltinMiddlewareConfig

```tsx
interface BuiltinMiddlewareConfig {
  cors?: CorsConfig
  timing?: boolean
  log?: boolean
  rate?: RateLimitConfig
  headers?: Record<string, string>
}
```

### RenderConfig

```tsx
interface RenderConfig {
  strategy?: RenderStrategy // ค่าเริ่มต้น "ssr"
  revalidate?: number
}
```

### ImageConfig

```tsx
interface ImageConfig {
  optimize?: boolean
  quality?: number
  lossless?: boolean
  keepOriginal?: boolean
  variantWidths?: number[]
  workers?: number
}
```

### Plugin Sockets

```tsx
interface PluginRegistrationApi {
  readonly http: PluginHttpSocket
  readonly build: PluginBuildSocket
  readonly dev: PluginDevSocket
  readonly diagnostics: PluginDiagnosticsSocket
  readonly native: PluginNativeSocket
}
```

- **PluginHttpSocket** — `onRequest`, `onResponse`, `route`
- **PluginBuildSocket** — `onStart`, `onResolve`, `onLoad`, `onTransform`, `onComplete`
- **PluginDevSocket** — `onFileChange`
- **PluginDiagnosticsSocket** — `report`
- **PluginNativeSocket** — `claim`

### RuvyxaPlugin

```tsx
interface RuvyxaPlugin {
  readonly name: string
  readonly head?: readonly PluginHeadEntry[]
  register(api: PluginRegistrationApi): void | Promise<void>
}
```

### ImageLoader

```tsx
interface ImageLoaderProps {
  src: string
  width?: number
  quality?: number
}

type ImageLoader = (props: ImageLoaderProps) => string
```

### StandaloneServerOptions

```tsx
interface StandaloneServerOptions {
  isrCache?: 'bundle' | 'tmp'
}
```

### ImageLoaderProps

```tsx
interface ImageLoaderProps {
  src: string
  width?: number
  quality?: number
}
```

---

_เอกสารอ้างอิงสำหรับ Ruvyxa 1.0 — อัปเดตล่าสุดตาม codebase ณ เวลาที่เผยแพร่_

## วิธีใช้ API Reference อย่างปลอดภัย

public export barrels เป็น source ที่ผูกกับ implementation สำหรับบทนี้: `@ruvyxa/react` export
UI/runtime helpers เช่น `Image`, `Picture`, `Link`, `Seo`, `Answer`, route-context hooks,
`useRuvyxaLoader`, hydration helpers และ error/not-found helpers ส่วน `@ruvyxa/core` export
`config`, plugin helpers, server helpers `action`, `cache`, `loader`, `json`, `notFound`,
`redirect`, `invalidateCache` รวมถึง public configuration และ route types

ควร import จาก public package paths เหล่านี้ ไม่ควรเข้าถึง `src/` หรือ runtime files โดยตรง:

```ts
import { config } from 'ruvyxa/config'
import { action, cache, loader } from '@ruvyxa/core/server'
import { Image, Link, useParams } from '@ruvyxa/react'
```

types และ runtime behavior เปลี่ยนไปด้วยกันได้ เมื่อเพิ่มตัวอย่างใหม่หรือ upgrade package ให้ตรวจ
exported symbol ที่ตรงกันและรัน TypeScript/project check อย่าถือว่า type snippet
ในหน้านี้เป็นหลักฐานของ private implementation contract
