# Routing และ rendering

route discovery แปลง file tree เป็น manifest รัน `pnpm routes` ระหว่างพัฒนาเพื่อดู manifest และใช้
`pnpm routes:json` เมื่อ script ต้องการข้อมูลที่เครื่องอ่านได้ strategy ของหน้าถูกเลือกจาก export
ของหน้าและ configuration `render`

| Strategy | การเลือกที่ยืนยันจาก source                | เวลาสร้าง HTML                                          |
| -------- | ------------------------------------------ | ------------------------------------------------------- |
| SSR      | ค่าเริ่มต้น หรือ `render.strategy: 'ssr'`  | ทุก request                                             |
| SSG      | static route/static parameter discovery    | build time                                              |
| ISR      | `export const revalidate = 60`             | build time แล้ว revalidate หลัง TTL                     |
| CSR      | หน้า `'use client'`                        | browser หลัง minimal shell                              |
| PPR      | `export const ppr = true` พร้อม `Suspense` | static shell ตอน build; dynamic slot stream ตอน request |

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

## นโยบาย i18n route

`i18n.locales` และ `i18n.defaultLocale` เป็น configuration field locale routing เป็นแบบ file-system
(เช่น `app/[lang]/about/page.tsx`); ชื่อ parameter ปริยายคือ `lang` เมื่อเปิด locale detection
server จะพิจารณา cookie ที่ตั้งค่าไว้ (ปริยาย `RUVYXA_LOCALE`) และ `Accept-Language`

**ก่อนหน้า:** [โครงสร้างโปรเจกต์](03-project-structure.md) · **ถัดไป:**
[ข้อมูล, action และ API route](05-data-actions-api.md)
