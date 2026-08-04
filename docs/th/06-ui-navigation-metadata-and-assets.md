# UI, navigation, metadata และ asset

`@ruvyxa/react` export React helper ที่รู้จัก framework helper เหล่านี้เป็น optional; React
component ปกติยังทำงานได้

## Navigation และ route state

ใช้ `Link` สำหรับ application navigation และ `useRouter()` สำหรับ imperative navigation
`usePathname()`, `useParams()`, `useSearchParams()`, `useSelectedRoute()` และ `useRouteContext()`
เปิดเผย client route state ปัจจุบัน

```tsx
'use client'
import { Link, useRouter, useSearchParams } from '@ruvyxa/react'

export function SearchControls() {
  const router = useRouter()
  const query = useSearchParams().get('q') ?? ''
  return (
    <>
      <Link href="/about">About</Link>
      <button onClick={() => router.push(`/search?q=${query}`)}>Search</button>
    </>
  )
}
```

`useSearchParams()` คืน set ว่างระหว่าง SSR เมื่อ query ใช้ไม่ได้ จึงอย่าใช้มันสำหรับ markup
ที่ต้องเหมือนกันใน server HTML `useRouter().pending` ติดตาม route-bundle navigation

## Metadata และ error UI

ใช้ route `meta` export สำหรับ metadata แบบ hierarchy-aware ([Routing](04-routing-rendering.md))
หรือใช้ `<Seo>` ใน component สำหรับ tag ต่อ render `<Seo>` สามารถสร้าง Open Graph, X card, Article
JSON-LD, breadcrumb JSON-LD และ custom JSON-LD prop `twitterCard` ถูก deprecate และแทนด้วย `card`

```tsx
import { Seo, RuvyxaErrorBoundary } from '@ruvyxa/react'

export default function Product() {
  return (
    <RuvyxaErrorBoundary
      fallback={({ error, resetError }) => (
        <button onClick={resetError}>Retry: {error.message}</button>
      )}
    >
      <Seo
        title="Product"
        description="A documented product"
        canonical="https://example.test/product"
      />
      <main>...</main>
    </RuvyxaErrorBoundary>
  )
}
```

`RuvyxaErrorBoundary` ดัก React render error ของ descendant, เรียก `onError` เมื่อมี และส่ง
`resetError` ให้ fallback มันไม่แทน route-level `error.tsx` boundary

## Image, CSS และ static file

`Image` รับ React image prop พร้อม Ruvyxa option asset PNG/JPEG ใน public แบบ local จะถูก optimize
เป็น WebP ระหว่าง production build โดยปริยาย `image.variantWidths` ควบคุม responsive variant;
`Image` ใช้ width เหล่านั้นกับ local image เมื่อระบุ `sizes` `image.onDemand` เปิด same-origin
runtime transformation ที่ `/__ruvyxa/image` และมี maximum width ปริยาย 3840 เมื่อกำหนดเป็น object

```tsx
import { Image } from '@ruvyxa/react'
export function Hero() {
  return (
    <Image
      src="/hero.jpg"
      alt="Team at work"
      width={1200}
      height={630}
      sizes="(max-width: 768px) 100vw, 1200px"
      priority
    />
  )
}
```

imported project CSS อาจอยู่นอก `app/` ได้ หากต้อง include global style ที่ module ไม่ได้ import
ให้ใส่ file/directory แบบ project-relative ใน `css.entries` runtime รู้จัก Sass เป็น package
dependency ให้ใช้ style ที่ build resolve ได้ และรัน `ruvyxa check` หลังเปลี่ยน boundary

**ก่อนหน้า:** [ข้อมูล, action และ API route](05-data-actions-api.md) · **ถัดไป:**
[Configuration และ environment](07-configuration.md)
