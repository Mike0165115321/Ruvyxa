# Routing

> 🟢 **เหมาะกับมือใหม่** · ⏱️ อ่าน ~7 นาที
>
> **จะได้เรียนรู้:** โฟลเดอร์กลายเป็น URL ยังไง, หน้าแบบมีตัวแปร (`[slug]`), catch-all route
> และการจัดกลุ่มไฟล์โดยไม่กระทบ URL

Route มาจากโฟลเดอร์ใต้ `app/` โดยตรง ไม่มีไฟล์คอนฟิก route ไม่มี pattern ที่สองให้จำ
และไม่มีอะไรต้อง sync: **โครงสร้างโฟลเดอร์คือตาราง route**

## เข้าใจใน 30 วินาที

| คุณสร้าง                      | URL ใน browser | มันคืออะไร                  |
| ----------------------------- | -------------- | --------------------------- |
| `app/page.tsx`                | `/`            | หน้าแรก                     |
| `app/about/page.tsx`          | `/about`       | หน้า static                 |
| `app/blog/[slug]/page.tsx`    | `/blog/hello`  | หน้า dynamic (param `slug`) |
| `app/docs/[...path]/page.tsx` | `/docs/a/b/c`  | หน้า catch-all              |
| `app/api/items/route.ts`      | `/api/items`   | API endpoint (ไม่มี HTML)   |
| `app/posts/intro/page.md`     | `/posts/intro` | หน้า content แบบ Markdown   |

โฟลเดอร์กลายเป็น URL segment มี `page.tsx` (หรือ `page.jsx`, `page.md`, `page.mdx`) ข้างใน URL
นั้นจะ render เป็นหน้า มี `route.ts` จะเป็น API endpoint ไฟล์อื่นในโฟลเดอร์เป็นเรื่องภายใน

## Route แรกของคุณ

```text
app/
├── layout.tsx          → ห่อทุกหน้า
├── page.tsx            → /
├── about/
│   └── page.tsx        → /about
└── blog/
    ├── page.tsx        → /blog
    └── [slug]/
        └── page.tsx    → /blog/:slug
```

ทุกหน้า default-export React component การนำทางระหว่างหน้าใช้ HTML ธรรมดา:

```tsx
// app/page.tsx
export default function Home() {
  return (
    <main>
      <h1>ยินดีต้อนรับ</h1>
      <a href="/about">เกี่ยวกับเรา</a>
    </main>
  )
}
```

## Dynamic Segments

`[name]` จับหนึ่ง segment (บังคับมี) เป็น `string`:

```tsx
// app/blog/[slug]/page.tsx → match /blog/hello ไม่ match /blog หรือ /blog/a/b
import type { PageProps } from 'ruvyxa/config'

export default function BlogPost({ params }: PageProps<{ slug: string }>) {
  return <h1>Post: {params.slug}</h1>
}
```

`[...path]` (catch-all) จับ **หนึ่ง segment ขึ้นไป** ที่เหลือเป็น `string[]`:

```tsx
// app/docs/[...path]/page.tsx → match /docs/a และ /docs/a/b/c ไม่ match /docs เอง
export default function Docs({ params }: PageProps<{ path: string[] }>) {
  return <h1>{params.path.join('/')}</h1>
}
```

`[[...path]]` (optional catch-all) match URL ของ parent **ด้วย** — ค่าเป็น `undefined` ที่ parent
และเป็น `string[]` เมื่อมี segment:

```tsx
// app/shop/[[...path]]/page.tsx → match /shop และ /shop/clothes/shirts
export default function Shop({ params }: PageProps<{ path?: string[] }>) {
  return <h1>{params.path?.join('/') ?? 'สินค้าทั้งหมด'}</h1>
}
```

> **สำหรับคนมาจาก Next.js** — Ruvyxa ให้ `params` แบบ synchronous ไม่มี Promise ให้ await;
> `params.slug` เป็น string ตรงๆ

## ลำดับความสำคัญตอน match

เมื่อหลาย route match URL เดียวกันได้ ตัวที่เจาะจงที่สุดชนะ — ตามลำดับนี้เสมอ:

1. **Static** segment (`app/blog/featured/`)
2. **Dynamic** segment (`app/blog/[slug]/`)
3. **Catch-all** (`app/blog/[...path]/`)
4. **Optional catch-all** (`app/blog/[[...path]]/`) — ต่ำสุด

ดังนั้น `/blog/featured` จะ render หน้า static แม้มี `[slug]` อยู่ข้างๆ ไม่ต้องจัดลำดับเองเลย

## Layout ซ้อนกันอัตโนมัติ

แต่ละโฟลเดอร์มี `layout.tsx` ของตัวเองได้ layout ห่อจากนอกเข้าใน:

```text
app/layout.tsx            → ห่อทุกอย่าง
app/blog/layout.tsx       → ห่อทุกหน้า /blog/* เพิ่ม
app/blog/[slug]/page.tsx  → render ในทั้งสอง layout
```

```tsx
// app/blog/layout.tsx
export default function BlogLayout({ children }: { children: React.ReactNode }) {
  return (
    <section>
      <nav>เมนูบล็อก</nav>
      {children}
    </section>
  )
}
```

## Metadata ของหน้า

page หรือ layout export `meta` ได้ Ruvyxa จะ merge `meta` ทุกชั้นของ route — root layout ก่อน page
ท้ายสุด — แล้ว render ลง `<head>`:

```tsx
// app/layout.tsx
import type { Meta } from '@ruvyxa/react'

export const meta: Meta = {
  titleTemplate: '%s · Acme',
  title: 'Acme',
  description: 'ทุกอย่างที่ Acme สร้าง',
  siteName: 'Acme',
  lang: 'th',
}
```

```tsx
// app/blog/[slug]/page.tsx
export const meta = ({ params }: { params: { slug: string } }) => ({
  title: params.slug,
  canonical: `https://acme.dev/blog/${params.slug}`,
})
```

`/blog/hello` จะได้ `<title>hello · Acme</title>`, description กับ `og:site_name` จาก layout และ
canonical ของตัวเอง `title` ของชั้นไหนจะไม่ถูกจัดรูปด้วย `titleTemplate` ของชั้นตัวเอง ดังนั้น
layout ข้างบนยังเป็น `Acme` ที่ `/`

| ฟิลด์                                | render เป็น                                                          |
| ------------------------------------ | -------------------------------------------------------------------- |
| `title`, `titleTemplate`             | `<title>`, `og:title`, `twitter:title`                               |
| `description`                        | `<meta name="description">`, `og:description`, `twitter:description` |
| `canonical`                          | `<link rel="canonical">`, `og:url`                                   |
| `robots`, `noindex`                  | `<meta name="robots">`                                               |
| `lang`                               | attribute `lang` ของ `<html>` ที่ server render                      |
| `alternates`                         | `<link rel="alternate" hreflang>` ต่อหนึ่งรายการ                     |
| `image`, `imageAlt`                  | `og:image`, `twitter:image` และ alt                                  |
| `siteName`, `type`, `locale`, `card` | tag Open Graph และการ์ดพรีวิวของ X ที่ตรงกัน                         |

`meta` เป็น object หรือฟังก์ชัน **แบบ synchronous** ของ `{ path, params }` ก็ได้ ค่าถูก resolve ตอน
render ฟังก์ชัน async จะได้ค่ามาหลัง shell ถูกส่งออกไปแล้วโดยไม่มี title

ข้อจำกัดที่ควรรู้สองข้อ:

- `lang` ถูกใส่ในเอกสารที่ server render แต่ละครั้ง การนำทางฝั่ง client ไม่เปลี่ยนค่านี้ route tree
  ที่แยกภาษา (`app/[lang]/…`) จึงได้ค่าถูกต้องบนทุกเอกสารที่ crawler เห็น แต่ไม่เปลี่ยนตอน soft
  navigation ข้ามภาษา
- อย่าตั้งฟิลด์เดียวกันทั้งใน `meta` และ [`<Seo>`](official-packages.md) React จะ hoist `<title>`
  ทั้งสองอันและอันหลังชนะ ซึ่งมักไม่ใช่สิ่งที่ตั้งใจ ใช้ `meta` กับสิ่งที่ route เป็นเสมอ และใช้
  `<Seo>` เมื่อค่ามีอยู่เฉพาะตอน component render

## Route Groups

ใช้ `(name)` จัดระเบียบไฟล์ **โดยไม่เพิ่ม** URL segment:

```text
app/(marketing)/pricing/page.tsx   → /pricing   (ไม่ใช่ /marketing/pricing)
app/(marketing)/contact/page.tsx   → /contact
app/(app)/dashboard/page.tsx       → /dashboard
```

Group เหมาะกับการให้แต่ละโซนของเว็บมี layout ต่างกัน — วาง `layout.tsx` ในแต่ละ group

## โฟลเดอร์ที่ถูกข้าม

โฟลเดอร์ขึ้นต้นด้วย `_` หรือ `@` ไม่ถูก route ใช้วาง helper ใกล้ๆ หน้า:

```text
app/blog/_components/PostCard.tsx   → ไม่ใช่ route; import จากหน้าได้ตามปกติ
```

## การนำทางฝั่ง client

`<a href>` ธรรมดาใช้งานได้เสมอและจะโหลดหน้าใหม่ทั้งหน้า ถ้าต้องการเปลี่ยน route โดยไม่โหลดเอกสารใหม่
ใช้ `<Link>` จาก `@ruvyxa/react`:

```tsx
import { Link } from '@ruvyxa/react'

export default function Nav() {
  return (
    <nav>
      <Link href="/">หน้าแรก</Link>
      <Link href="/blog/hello" prefetch="viewport">
        Hello
      </Link>
    </nav>
  )
}
```

`<Link>` render เป็น `<a href>` จริง จึงยัง crawl ได้ คลิกกลางเปิดแท็บใหม่ได้ และทำงานก่อน hydration
หรือแม้ปิด JavaScript — soft navigation เป็นส่วนเสริมทับบนนั้น การคลิกพร้อมปุ่มพิเศษ, เปิดแท็บใหม่,
`target`, และ `download` จะปล่อยให้เบราว์เซอร์จัดการเอง prefetch อุ่นบันเดิลปลายทางล่วงหน้า:
`"hover"` (ค่าเริ่มต้น), `"viewport"` เมื่อเลื่อนถึง, หรือ `false` เพื่อปิด

อ่านและควบคุม route ปัจจุบันด้วย hooks:

```tsx
import { useRouter, usePathname, useParams, useSearchParams } from '@ruvyxa/react'

function Example() {
  const router = useRouter() // push / replace / back / forward / refresh / prefetch และ `pending`
  const pathname = usePathname() // "/blog/hello"
  const params = useParams() // { slug: "hello" }
  const query = useSearchParams() // URLSearchParams
  return <button onClick={() => router.push('/about')}>เกี่ยวกับ</button>
}
```

`useSearchParams` คืนค่าว่างตอน server render และคืน query string จริงหลัง hydration — routing
ที่ต้องเหมือนกันใน HTML ฝั่ง server ควรอยู่ใน path ไม่ใช่ query URL ที่ไม่มี client route ใดรองรับ
(API route, redirect, rewrite) จะ fallback เป็นการโหลดเอกสารเต็ม การนำทางจึงไม่เงียบหาย

## Validation จับข้อผิดพลาดตอน build

Ruvyxa reject รูปแบบ route ที่กำกวมแทนการเดา รวมถึง dynamic siblings อย่าง `[id]` กับ `[slug]`
ในโฟลเดอร์เดียวกัน (`RUV1003`) ดูสิ่งที่ framework ค้นพบได้ตลอด:

```bash
npx ruvyxa routes    # พิมพ์ตาราง route ที่ resolve แล้ว
npx ruvyxa analyze   # วิเคราะห์กราฟเต็มพร้อม rendering strategy
```

ถ้า URL ขึ้น 404 ทั้งที่ไม่ควร รัน `npx ruvyxa routes` ก่อน — คำตอบมักเป็น `page.tsx` หายหรือ
พิมพ์ชื่อโฟลเดอร์ผิด

## ข้อผิดพลาดที่มือใหม่เจอบ่อย

| อาการ                            | สาเหตุ                                       | แก้                                           |
| -------------------------------- | -------------------------------------------- | --------------------------------------------- |
| มีโฟลเดอร์แต่ URL ขึ้น 404       | ไม่มี `page.tsx` ในโฟลเดอร์                  | เพิ่ม `page.tsx` (โฟลเดอร์เปล่าไม่เป็น route) |
| `/blog` 404 แต่ `/blog/x` ใช้ได้ | มีแค่ `[slug]/page.tsx`                      | เพิ่ม `app/blog/page.tsx` เป็นหน้า index      |
| สองไฟล์แย่ง URL เดียวกัน         | `page.tsx` กับ `route.ts` ในโฟลเดอร์เดียวกัน | เก็บตัวเดียว — URL หนึ่งเป็นได้อย่างเดียว     |
| Build error route กำกวม          | `[id]` กับ `[slug]` เป็น siblings            | ใช้ชื่อ dynamic เดียวต่อระดับโฟลเดอร์         |

## บทถัดไป

- [API Routes](api-routes.md) — handler `route.ts` รับ parameter รูปแบบเดียวกัน
- [Rendering Strategies](rendering-strategies.md) — ทำให้ route ใดก็ได้เป็น SSG, ISR, CSR, PPR
- [Markdown, MDX, Images & Metadata](markdown-mdx-images.md) — content route แบบ `page.md` /
  `page.mdx`
