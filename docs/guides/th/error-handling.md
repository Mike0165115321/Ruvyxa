# การจัดการข้อผิดพลาด

> 🔴 **เอกสารอ้างอิง** · ครอบคลุม framework diagnostics, route recovery, HTTP/API contract, server
> actions, client failures และ official packages

Ruvyxa ใช้ **RUV####** สำหรับปัญหาจาก framework, runtime, build และ official packages ส่วน
application error response คือ HTTP status กับข้อความปลอดภัยที่ API route หรือ action
ของแอปตั้งใจคืนให้ผู้เรียก จึงไม่ใช่สิ่งเดียวกัน

ห้ามแสดง compiler path, stack, token, database error หรือ RUV diagnostic ดิบแก่ผู้ใช้ production
ทั่วไป development แสดงบริบท diagnostic ได้ แต่ native server จะปกปิด unhandled production
diagnostic ด้วยหน้า 500 ทั่วไปโดยตั้งใจ

## Triage แบบเร็ว

1. เก็บรหัสและข้อความเต็มไว้ ห้ามลดเหลือเพียง “build failed”
2. แยกประเภทเป็น build/validation, render, API/action, client, adapter หรือ package
3. แก้ไฟล์และ import chain ที่ระบุก่อน แล้วรันคำสั่งที่ทำให้เกิดปัญหาซ้ำ
4. ใน CI ใช้ **ruvyxa analyze --format sarif --output reports/ruvyxa.sarif** เพื่อเก็บตำแหน่งและ
   suggested fix
5. เมื่อส่งต่อ incident ให้แนบรหัส, command, framework version, Node/Bun, target, route และ stack
   ที่ตัดข้อมูลลับแล้ว—ห้ามแนบ secret/cookie

## เลือกชั้น recovery ให้ถูก

| ปัญหาอยู่ที่                | ใช้                                              | ผลลัพธ์                                   |
| --------------------------- | ------------------------------------------------ | ----------------------------------------- |
| Build/contract              | แก้ diagnostic แล้วรัน check/build               | ไม่ deploy; dev overlay อาจแสดงรายละเอียด |
| Page render throw           | error.tsx ใกล้สุด หรือ RuvyxaErrorBoundary       | fallback เฉพาะส่วนและ retry ได้           |
| ไม่มี resource ตามปกติ      | notFound() และ not-found.tsx                     | not-found UI รวม server recovery          |
| Client data request         | error/refetch ของ useRuvyxaLoader                | loading/error/retry ชัดเจน                |
| Hydration mismatch          | hydrate({ onError })                             | report แบบปลอดภัย                         |
| API input ผิด               | คืน Response 4xx ที่ตั้งใจ                       | public contract เสถียร                    |
| Action ถูกบล็อก/payload ผิด | action security และ handler validation           | 400/403/413/415/429                       |
| DB/auth/realtime ล้มเหลว    | catch ที่ app boundary, log code, map แบบปลอดภัย | public error ของผลิตภัณฑ์                 |

## Route recovery

วาง special file ไว้ข้าง route segment ที่ต้องป้องกัน ระบบเลือกไฟล์ที่ใกล้ที่สุดและ layouts
ยังแสดงอยู่

```tsx
// app/products/error.tsx
'use client'
import type { RouteErrorProps } from '@ruvyxa/react'

export default function ProductsError({ error, reset }: RouteErrorProps) {
  console.error('products route failed', { message: error.message })
  return (
    <main>
      <h1>โหลดสินค้าไม่สำเร็จ</h1>
      <button onClick={reset}>ลองอีกครั้ง</button>
    </main>
  )
}
```

**reset** remount subtree ที่ล้มหลัง hydration ไม่ใช่ server rollback ดังนั้น mutation ต้อง
idempotent หรือบอกผู้ใช้ว่าคำขอก่อนหน้าอาจสำเร็จแล้ว

```tsx
// app/posts/[slug]/page.tsx
import { notFound } from '@ruvyxa/react'
const post = await getPost(params.slug)
if (!post) notFound()
```

ใช้ **notFound()** สำหรับ “ไม่มีข้อมูล” ที่คาดไว้ ไม่ใช่ generic throw ระบบ render not-found.tsx
ที่ใกล้ที่สุดและกู้คืนบน server ได้ก่อน JavaScript ส่วน error.tsx ทั่วไปกู้คืนฝั่ง client สำหรับ
streamed SSR จึงควรมี shell/loading ที่ดีและไม่พึ่งปุ่ม retry ก่อน hydration

## Client data และ hydration

useRuvyxaLoader แปลง synchronous throw และ Promise rejection เป็น **error**, ป้องกัน request
เก่ามาทับ request ใหม่ และมี **refetch**:

```tsx
'use client'
import { useRuvyxaLoader } from '@ruvyxa/react'
const { data, loading, error, refetch } = useRuvyxaLoader(
  async () => {
    const response = await fetch('/api/account')
    if (!response.ok) throw new Error('Account request failed')
    return response.json() as Promise<{ name: string }>
  },
  { deps: [] },
)
```

ลงทะเบียน hydration reporter หนึ่งจุดใกล้ client bootstrap โดยรับ unknown error และ
componentStack/digest แบบ optional exception ภายใน reporter จะถูกกลืนเพื่อไม่ให้การรายงานทำ UI ล้ม

```ts
import { hydrate } from '@ruvyxa/react'
hydrate({
  onError(error, context) {
    reportToObservability({
      kind: 'hydration',
      message: error instanceof Error ? error.message : String(error),
      ...context,
    })
  },
})
```

## API routes และ server actions

API route เป็นเจ้าของ public HTTP contract ต้อง validate ใกล้ handler และ catch dependency failure
ที่คาดไว้ route ที่ throw จะเป็น framework runtime failure (มัก RUV1200 ใน native worker) ไม่ใช่
validation feedback ที่ดีแก่ client

```ts
export async function POST({ request }: { request: Request }) {
  let body: unknown
  try {
    body = await request.json()
  } catch {
    return Response.json({ error: 'invalid_json' }, { status: 400 })
  }
  if (!body || typeof body !== 'object')
    return Response.json({ error: 'invalid_input' }, { status: 400 })
  try {
    return Response.json({ data: await createItem(body) }, { status: 201 })
  } catch (error) {
    logServerError(error)
    return Response.json({ error: 'temporarily_unavailable' }, { status: 503 })
  }
}
```

| Status  | ใช้เมื่อ                                            |
| ------- | --------------------------------------------------- |
| 400     | syntax หรือ validation input ไม่ผ่าน                |
| 401     | ต้องยืนยันตัวตน                                     |
| 403     | ไม่มีสิทธิ์; Ruvyxa ใช้บล็อก cross-site action ด้วย |
| 404     | ไม่มี resource/action route                         |
| 405     | ไม่ export method หรือ action ชี้ non-page route    |
| 413     | payload เกิน limit                                  |
| 415     | action body ไม่ใช่ JSON/URL-encoded form            |
| 429     | เกิน rate limit; ต้องใช้ retry timing               |
| 500/503 | internal/dependency failure; public text ต้องทั่วไป |

ก่อน action เริ่ม Ruvyxa อาจปฏิเสธ payload ใหญ่ (413), type ไม่รองรับ (415), cross-origin/cross-site
(403), UTF-8/JSON ผิด (400) หรือ rate limit (429) สิ่งนี้เสริม—not replaces—action.input({ parse })
และ authorization ใน handler parser/handler ที่ throw คือ execution failure บน server: ให้ log
และคืน feedback ปลอดภัย

## สัญญา Diagnostic

Rust Diagnostic มี code, title, explanation, file/line/column เมื่อทราบ, import chain, suggested fix
และ affected routes โดย human, JSON และ SARIF ออกมาจาก object เดียวใน
**crates/ruvyxa_diagnostics/src/lib.rs**

คำว่า “ส่งต่อ/สงวน” หมายถึง native host รู้จักรหัส แต่ static search ของ workspace นี้ยังไม่พบ
direct emitter ให้เก็บ worker output และตรวจ runtime version ก่อนสรุป RUV9999 เป็น test-only
sentinel สำหรับ redaction ไม่ใช่ public runtime diagnostic

## รายการรหัส RUV ครบถ้วน

### Routes, boundaries, SSR, APIs และ content

| รหัส    | ความหมาย / trigger ที่เป็นไปได้                               | วิธีเริ่มกู้คืน                                            |
| ------- | ------------------------------------------------------------- | ---------------------------------------------------------- |
| RUV1001 | ไม่พบ app directory                                           | สร้าง app/page.tsx (หรือ page.md/page.mdx) หรือตั้ง appDir |
| RUV1002 | dynamic segment syntax ไม่ถูกต้อง                             | ใช้ [name], [...name] หรือ [[...name]]                     |
| RUV1003 | routes มี URL match shape เดียวกัน                            | ทำ static path shape ให้ต่างกัน; ชื่อ parameter ไม่พอ      |
| RUV1004 | หน้า TS/JS ไม่มี default export                               | export default page component                              |
| RUV1007 | server-only module ถึง client graph                           | ย้ายงานไป server และส่ง serializable data                  |
| RUV1008 | private env ถึง browser code                                  | อ่านบน server; เปิดเผยเฉพาะ RUVYXA_PUBLIC_ ที่ตั้งใจ       |
| RUV1009 | client-only module ถึง server graph                           | ย้ายไป client module/component                             |
| RUV1010 | module ใต้ server/ ถึง client graph                           | ย้าย code ที่ browser ใช้ได้ออกนอก server/                 |
| RUV1100 | SSR renderer จับ render exception                             | อ่าน nested JS cause/stack; เพิ่ม route fallback           |
| RUV1101 | SSR renderer ไม่มี internal arguments                         | ซ่อม/ติดตั้ง runtime ใหม่ ไม่ใช่ app input ปกติ            |
| RUV1102 | ไม่พบ SSR renderer script                                     | ติดตั้ง ruvyxa/runtime dependencies                        |
| RUV1200 | API renderer handler/bundle throw                             | handle route error ที่คาดไว้และตรวจ server cause           |
| RUV1201 | API renderer ไม่มี arguments **หรือ** native port bind ไม่ได้ | ซ่อม invocation; กรณี port ให้ปล่อย/ตั้งช่วงพอร์ต          |
| RUV1202 | ไม่พบ API renderer script                                     | ติดตั้ง ruvyxa/runtime dependencies ใหม่                   |
| RUV1205 | prerender path ออกนอก safe output                             | คืน static params เป็น plain URL segments                  |
| RUV1300 | compile client hydration bundle ไม่สำเร็จ                     | แก้ page, browser-safe imports, JSX หรือ React dependency  |
| RUV1303 | ขอ client bundle ของ route ที่ไม่อยู่ manifest                | reload แล้วตรวจ deployment/cache                           |
| RUV1304 | ขอ client bundle สำหรับ non-page route                        | hydrate เฉพาะ page route                                   |
| RUV1310 | content extension ไม่รองรับ                                   | ใช้ page.md/page.mdx ที่รองรับ                             |
| RUV1311 | Markdown/MDX/content compile ล้มเหลว                          | แก้ syntax และ embedded imports/expressions                |
| RUV1312 | frontmatter เปิด --- แต่ไม่มีตัวปิด                           | ปิดด้วย --- หรือ ...                                       |

### Styles, rendering, actions และ configuration

| รหัส    | ความหมาย / trigger ที่เป็นไปได้                         | วิธีเริ่มกู้คืน                                            |
| ------- | ------------------------------------------------------- | ---------------------------------------------------------- |
| RUV1400 | Tailwind CLI compile ล้มเหลว                            | ตรวจ directives, content sources และ Tailwind versions     |
| RUV1401 | import Tailwind แต่ไม่มี @tailwindcss/cli               | ติดตั้ง Tailwind และ CLI                                   |
| RUV1402 | Sass compile ล้มเหลว                                    | แก้ Sass syntax/import ที่ระบุ                             |
| RUV1403 | resolve CSS/Sass import ไม่ได้                          | แก้ path หรือติดตั้ง dependency                            |
| RUV1404 | css.entries ออกนอก project root                         | ใช้ project-relative entry                                 |
| RUV1500 | SSG/action/PPR/action-realtime execution failure ทั่วไป | เก็บ nested worker message; แก้ route/action contract ก่อน |
| RUV1501 | ไม่มี action.ts/action.js ข้าง page                     | สร้างและ export action ที่ต้องการ                          |
| RUV1502 | server-action worker code แบบส่งต่อ/สงวน                | เก็บ worker message และตรวจ runtime ก่อนวิเคราะห์          |
| RUV1503 | server-action worker code แบบส่งต่อ/สงวน                | เก็บ worker message และตรวจ runtime ก่อนวิเคราะห์          |
| RUV1510 | staticParams ไม่คืน array หรือ { params }               | คืน array/object ตาม contract                              |
| RUV1511 | scalar static-param ใช้กับหลาย segments                 | คืน object ที่ key ทุก segment                             |
| RUV1512 | static-param entry ไม่ใช่ object/scalar ที่ใช้ได้       | คืน object; scalar ใช้ได้แค่ segment เดียว                 |
| RUV1513 | static-param cache duration ไม่ถูกต้อง                  | ใช้ positive number หรือ duration เช่น 10m                 |
| RUV1550 | Partial prerender ล้มเหลว                               | ตรวจ nested render error; แยก static/dynamic work          |
| RUV1600 | ruvyxa.config load/evaluate ไม่สำเร็จ                   | แก้ syntax, imports และ config side effects                |
| RUV1601 | config-renderer invocation/config validation ล้มเหลว    | ตาม field/message แล้วแก้ config/runtime invocation        |
| RUV1602 | bounded config value นอกช่วง                            | ตั้งค่าตาม min/max ที่รายงาน                               |
| RUV1603 | config.adapter หรือ adapter.build contract ผิด          | ให้ build(context) และ valid output object                 |

### Workers, compilation, middleware, adapters และ packages

| รหัส    | ความหมาย / trigger ที่เป็นไปได้                    | วิธีเริ่มกู้คืน                                                  |
| ------- | -------------------------------------------------- | ---------------------------------------------------------------- |
| RUV1700 | TypeScript plugin hook timeout                     | ลด blocking work หรือปรับ timeout ที่เกี่ยวข้อง                  |
| RUV1701 | plugin host คืน protocol/registry ไม่ถูกต้อง       | ตรวจ plugin runtime/version และ hook result                      |
| RUV1702 | ไม่พบ worker-pool script                           | ติดตั้ง ruvyxa/runtime dependencies ใหม่                         |
| RUV1704 | worker stream/API protocol ส่ง error frame         | เก็บ frame message แล้วตรวจ stream handler/logs                  |
| RUV1801 | runtime compiler resolve relative import ไม่ได้    | แก้ import path/dependency                                       |
| RUV1802 | Oxc TS/JSX transform ล้มเหลว                       | แก้ source syntax/transform issue ที่รายงาน                      |
| RUV1803 | runtime compiler พบ circular dependency            | ตัด cycle ที่ shared lower-level module                          |
| RUV1804 | JSX runtime ไม่ใช่ classic/automatic               | ใช้ค่าที่รองรับหนึ่งค่า                                          |
| RUV2000 | middleware configuration diagnostic                | แก้ invalid setting ที่ระบุ                                      |
| RUV2001 | middleware execution diagnostic                    | ตรวจ custom middleware/dependencies; ป้องกัน response boundary   |
| RUV2200 | adapter runner/build/artifact contract ล้มเหลว     | ตรวจ build output, artifact paths/kinds และ runner mode          |
| RUV2202 | adapter ไม่รองรับ route strategy                   | เลือก target/adapter ที่รองรับหรือเปลี่ยน strategy               |
| RUV2203 | resolve adapter package/factory ไม่ได้             | ติดตั้ง adapter หรือ export factory ให้ถูก                       |
| RUV2210 | platform serve route render strategy ไม่ได้        | deploy platform ที่รองรับหรือเปลี่ยน strategy                    |
| RUV3001 | DB query/options/private DB env ผิด                | แก้ model/operation/args และเก็บ DB env private                  |
| RUV3002 | DB adapter ไม่มี model/operation mapping           | แก้ Prisma/Dynamo mapping หรือ transport operation               |
| RUV3003 | ขอ transaction จาก adapter ที่ไม่รองรับ            | ใช้ transactional adapter หรือออกแบบใหม่                         |
| RUV3100 | auth runtime/provider delivery ล้มเหลว             | ตรวจ provider credentials/service; คืน temporary failure ปลอดภัย |
| RUV3101 | auth input/config ผิด                              | validate input/provider config                                   |
| RUV3102 | เกิน auth rate limit                               | ใช้ retry timing ห้าม retry เป็น loop                            |
| RUV3103 | OAuth/magic-link token/state ผิดหรือหมดอายุ        | เริ่ม sign-in ใหม่ ห้ามใช้ token เดิม                            |
| RUV3104 | OAuth token exchange ล้มเหลว/ไม่มี access token    | ตรวจ endpoint, credentials และ provider response                 |
| RUV3105 | production auth stores ไม่ durable                 | ตั้ง durable session/token/rate-limit stores                     |
| RUV3201 | native realtime อยู่บน target/adapter ที่ไม่รองรับ | ใช้ long-lived Node/Bun หรือนำ realtime ออก                      |
| RUV9999 | test-only sentinel สำหรับ production redaction     | ห้ามพึ่งเป็น public diagnostic                                   |

## Log เทียบกับสิ่งที่แสดง

ให้ log code, title, route, method, correlation ID ที่ปลอดภัย, framework version, target และ
cause/stack ที่ตัดข้อมูลลับแล้ว ให้แสดงข้อความภาษาผลิตภัณฑ์และ retry guidance ใช้ public message
เดียวกันกับ auth, authorization, database และ internal-rendering failure
ไม่ว่าทราบสาเหตุภายในหรือไม่ เพื่อลดการเปิดเผยบัญชี topology และ secrets

ผู้พัฒนา framework ต้องเพิ่ม stable code, explanation, span เมื่อทราบ, suggested fix และ tests
และแก้หน้านี้เมื่อ recovery behavior เปลี่ยน ห้ามสร้าง JSON/SARIF scanner แยก เพราะ Diagnostic
serializer ปัจจุบันคือ output contract เดียว
