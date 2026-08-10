# Routing และ rendering

> **เป้าหมายของ tutorial:** เพิ่ม dynamic route และเลือกวิธี render อย่างตั้งใจ **เริ่มจาก:** กติกา
> route ใน [โครงสร้างโปรเจกต์](03-project-structure.md) **Checkpoint:** เปิด dynamic URL
> จริงหนึ่งรายการในโหมด development แล้วรัน application check

route discovery แปลง file tree เป็น manifest รัน `npm run routes` ระหว่างพัฒนาเพื่อดู manifest
และใช้ `npm run routes:json` เมื่อ script ต้องการข้อมูลที่เครื่องอ่านได้ strategy ของหน้าถูกเลือกจาก
export ของหน้าและ configuration `render`

| Strategy | การเลือกที่ยืนยันจาก source                | เวลาสร้าง HTML                                          |
| -------- | ------------------------------------------ | ------------------------------------------------------- |
| SSR      | ค่าเริ่มต้น หรือ `render.strategy: 'ssr'`  | ทุก request                                             |
| SSG      | static route/static parameter discovery    | build time                                              |
| ISR      | `export const revalidate = 60`             | build time แล้ว revalidate หลัง TTL                     |
| CSR      | หน้า `'use client'`                        | browser หลัง minimal shell                              |
| PPR      | `export const ppr = true` พร้อม `Suspense` | static shell ตอน build; dynamic slot stream ตอน request |

## Markdown, MDX และ component ที่ใช้ร่วมกัน

สร้าง `page.md` สำหรับ Markdown หรือ `page.mdx` เมื่อต้องใช้ JSX, expression และ import ทั้งสองแบบ
ไม่ต้องตั้ง compiler เพิ่ม หากต้องการตกแต่ง element มาตรฐานของ MDX หรือให้ component ร่วมกัน
ให้เพิ่ม `mdx-components.tsx` ที่ใกล้ page ที่สุด (รองรับ `.ts`, `.jsx`, `.js`, `.mts` และ `.mjs`
ด้วย) ไว้ใน directory ของ page หรือ ancestor:

```tsx
// app/mdx-components.tsx
export function useMDXComponents(components = {}) {
  return {
    ...components,
    h1: (props) => <h1 className="docs-title" {...props} />,
  }
}
```

provider ที่ใกล้ที่สุดจะชนะ ดังนั้น `app/docs/mdx-components.tsx` จะปรับเฉพาะ route ใต้ `app/docs/`
ก็ได้ `components` ที่ส่งให้ page โดยตรงจะถูกส่งต่อเข้า `useMDXComponents`; ให้ merge มันใน object
ที่คืนมาหากยังต้องการใช้ provider เป็น module ใน client graph ปกติ ดังนั้น `ruvyxa check` จะปฏิเสธ
server-only import และ private environment variable

## Dynamic SSG

สำหรับ dynamic SSG/ISR page ให้ export `getStaticParams` มันรับ route ทั้งหมดที่ค้นพบและรายละเอียด
route ปัจจุบัน แล้วคืน object (หรือ string/number shorthand สำหรับ route ที่มี dynamic segment
เดียว) ผลลัพธ์ห่อด้วย `{ params, cache }` ได้ โดย `cache` รับวินาทีหรือข้อความอย่าง `"10m"`

```tsx
// app/blog/[slug]/page.tsx
import type { GetStaticParams, PageProps } from 'ruvyxa'

export const getStaticParams: GetStaticParams<{ slug: string }> = () => [
  { slug: 'first-post' },
  { slug: 'release-notes' },
]

export default function Post({ params }: PageProps<{ slug: string }>) {
  return (
    <article>
      <h1>{params.slug}</h1>
    </article>
  )
}
```

## Route metadata และ boundary

`export const meta` รับ `Meta` object หรือ `MetaFactory` metadata จาก layout merge จาก root ไป leaf;
ค่าที่เฉพาะที่สุดชนะ title ระดับล่างจะถูก format โดย `titleTemplate` ของ ancestor ที่ใกล้ที่สุด

```tsx
// app/layout.tsx
import type { Meta } from '@ruvyxa/react'
export const meta: Meta = { titleTemplate: '%s — Example', siteName: 'Example' }

// app/blog/[slug]/page.tsx
export const meta = ({ params }: { params: Record<string, string> }) => ({
  title: params.slug,
  canonical: `https://example.test/blog/${params.slug}`,
})
```

`error.tsx` รับ `{ error, reset }`; `loading.tsx` และ `not-found.tsx` เป็น component ปกติ
หากต้องการเลือก `not-found.tsx` ที่ใกล้ที่สุด ให้ import `notFound` จาก `@ruvyxa/react` แล้วเรียกมัน
(มัน throw tagged signal) อย่าสับสนกับ `notFound` จาก `ruvyxa/server` ซึ่งสร้าง HTTP `Response`
สถานะ 404

### boundary ของสถานะ route แบบครบชุด

วาง special file ทั้งสามไว้ข้าง segment ที่ต้องการครอบคลุม ระบบจะเลือกไฟล์ที่อยู่ใกล้ที่สุด ดังนั้น
โครงสร้างนี้ทำให้ทุกหน้า product มี loading UI, ปุ่มลองใหม่เมื่อ error และ 404 เฉพาะส่วน โดยไม่ต้อง
แก้ทุก page

```text
app/products/
├── [slug]/
│   └── page.tsx
├── error.tsx
├── loading.tsx
└── not-found.tsx
```

```tsx
// app/products/[slug]/page.tsx
import { notFound } from '@ruvyxa/react'

const products = { notebook: 'A plain notebook' }

export default function Product({ params }: { params: { slug: string } }) {
  const product = products[params.slug as keyof typeof products]
  if (!product) notFound()
  return (
    <main>
      <h1>{product}</h1>
    </main>
  )
}
```

```tsx
// app/products/loading.tsx
export default function Loading() {
  return <main aria-busy="true">กำลังโหลดสินค้า…</main>
}

// app/products/not-found.tsx
export default function ProductNotFound() {
  return (
    <main>
      <h1>ไม่พบสินค้า</h1>
    </main>
  )
}
```

```tsx
// app/products/error.tsx
'use client'
import type { RouteErrorProps } from '@ruvyxa/react'

export default function ProductError({ error, reset }: RouteErrorProps) {
  return (
    <main>
      <h1>ไม่สามารถโหลดสินค้านี้ได้</h1>
      <p>{error.message}</p>
      <button type="button" onClick={reset}>
        ลองใหม่
      </button>
    </main>
  )
}
```

`loading.tsx` ถูกใช้เป็น Suspense fallback ของ route ส่วน `not-found.tsx` render ฝั่ง server ได้ แต่
`reset()` จะ render ใหม่หลัง hydration ดังนั้น `error.tsx` ที่มีปุ่มลองใหม่ต้องเป็น client component
ควรแสดงข้อความ error ที่ปลอดภัยสำหรับผู้ใช้ และบันทึกรายละเอียดวินิจฉัยใน server หรือ integration
ด้าน observability แทนการ render secret หรือ stack trace

## นโยบาย i18n route

`i18n.locales` และ `i18n.defaultLocale` เป็น configuration field locale routing เป็นแบบ file-system
(เช่น `app/[lang]/about/page.tsx`); ชื่อ parameter ปริยายคือ `lang` เมื่อเปิด locale detection
server จะพิจารณา cookie ที่ตั้งค่าไว้ (ปริยาย `RUVYXA_LOCALE`) และ `Accept-Language`

**ก่อนหน้า:** [โครงสร้างโปรเจกต์](03-project-structure.md) · **ถัดไป:**
[ข้อมูล, action และ API route](05-data-actions-api.md)
