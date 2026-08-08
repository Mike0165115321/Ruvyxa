# Plugin และ middleware

> **เป้าหมายของ tutorial:** เพิ่มพฤติกรรมที่ใช้ร่วมกันครั้งเดียว แล้วใช้กับ route ที่ต้องการ
> **เริ่มจาก:** แอปที่กำหนดค่าแล้วใน [Configuration](07-configuration.md) **Checkpoint:** ตรวจ route
> ที่ตรงและไม่ตรงอย่างละหนึ่งรายการหลังเปิดใช้ plugin หรือ middleware rule

plugin คือ value ที่คืนจาก `definePlugin()` ใน `ruvyxa/plugin` (ถูก re-export โดย `ruvyxa` ด้วย)
เพิ่มมันใน `plugins` ของ `ruvyxa.config.ts` plugin ต้องมีชื่อที่ไม่ว่าง และต้องมี declarative
behavior หรือ `register(api)` definition ที่ไม่ถูกต้องล้มเหลวด้วย `RUV2102`

## Declarative plugin

```ts
// plugins/request-id.ts
import { definePlugin } from 'ruvyxa/plugin'

export const requestId = definePlugin({
  name: 'example:request-id',
  http: {
    match: ['/api/*'],
    onResponse({ response }) {
      const headers = new Headers(response.headers)
      headers.set('x-example', 'enabled')
      return new Response(response.body, { status: response.status, headers })
    },
  },
})
```

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'
import { requestId } from './plugins/request-id'
export default config({ plugins: [requestId] })
```

`http.match` ใช้ path แบบ exact หรือ prefix ที่ลงท้าย `*` request hook คืน `Request`, `Response`
หรือไม่คืนค่าได้ response hook คืน `Response` หรือไม่คืนค่าได้ `http.routes` ประกาศ plugin-owned
route แบบ exact และรับ method เดียว หลาย method หรือทุก method เมื่อไม่ระบุ advanced `register` API
เปิด socket `http`, `build`, `dev`, `diagnostics`, `native` และ `head`

## Build และ dev lifecycle

build hook คือ `onStart`, `onResolve`, `onLoad`, `onTransform` และ `onComplete`
resolve/load/transform hook รับ environment เป็น `client`, `server`, `edge`, `worker` หรือ `shared`;
transformation คืน code, `{ code, map }`, null หรือไม่คืนค่า dev เปิด `onFileChange` registration
plugin report diagnostic และเพิ่ม document-head entry ได้ อย่าพึ่งพา module-level middleware state
ข้าม worker: config ระบุชัดว่า worker ไม่ share state นี้

## First-party plugin

`ruvyxa/plugins` มี implementation ของ `redirects`, `headers`, `observability`, `securityHeaders`,
`cacheRules`, `sitemap`, `robots`, `alias` และ file-backed helper อื่นใน public entry point นั้น ใช้
validation ของมันแทนการเขียน behavior ซ้ำ ตัวอย่างเช่น redirect รับ `*`, path exact หรือ
trailing-prefix pattern และรับ destination เฉพาะ HTTP(S) URL แบบ absolute หรือ absolute path
ที่ปลอดภัย

```ts
import { redirects, securityHeaders } from 'ruvyxa/plugins'
export default config({
  plugins: [
    redirects([{ source: '/old/*', destination: '/new/*', permanent: true }]),
    securityHeaders({ contentSecurityPolicy: { 'default-src': ["'self'"] } }),
  ],
})
```

`permanent: true` ทำให้ `redirects` ส่ง 308; มิฉะนั้นส่ง 307 `securityHeaders` ให้ HSTS โดยปริยาย
แต่ไม่สามารถเลือก CSP ที่ปลอดภัยสำหรับ application ของคุณได้—ให้กำหนดอย่างตั้งใจและทดสอบ third-party
resource

## แค็ตตาล็อก first-party plugin

| Plugin                                | Output หรือ runtime behavior                                                                      |
| ------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `redirects`, `headers`, `cacheRules`  | route-scoped redirect, response header และ browser/CDN cache directive                            |
| `observability`, `securityHeaders`    | request ID/timing/structured log และ response security policy                                     |
| `pwa`                                 | manifest, service worker, registration script, optional precache/offline fallback และ HTML wiring |
| `sitemap`, `robots`, `feed`           | `sitemap.xml`, `robots.txt` และ RSS output ตอน build จาก metadata ที่ระบุ                         |
| `searchIndex`, `contentEngine`        | search index ตอน build และ content-derived answer/search artifact                                 |
| `openApi`                             | OpenAPI 3.1 JSON ที่ serve ตอน development และเขียนเข้า production output                         |
| `alias`, `bundleBudget`, `requireEnv` | import aliasing ตอน build, client JavaScript size limit และ required environment validation       |
| `fonts`                               | self-host Google Fonts stylesheet URL ที่ส่งให้ตอน build                                          |

ใช้ข้อมูลแบบ explicit กับ build-time plugin: มันไม่ค้นหา business content หรือ API semantic
ของคุณให้เอง ตัวอย่างนี้เป็น PWA declaration ที่สมบูรณ์พร้อม `name` ที่จำเป็น:

```ts
import { pwa, openApi } from 'ruvyxa/plugins'

export default config({
  plugins: [
    pwa({
      name: 'Example app',
      icons: [{ src: '/icon-192.png', sizes: '192x192', type: 'image/png' }],
    }),
    openApi({
      info: { title: 'Example API', version: '1.0.0' },
      operations: [
        { method: 'GET', path: '/api/health', responses: { '200': { description: 'Healthy' } } },
      ],
    }),
  ],
})
```

PWA plugin ใช้ `/manifest.webmanifest`, `/sw.js` และ `/pwa-register.js` โดยปริยาย; path
ทั้งสามต้องต่างกัน `openApi` ใช้ `/openapi.json` โดยปริยาย, ต้องมี title/version ที่ไม่ว่าง
และปฏิเสธ method/path กับ `operationId` ที่ซ้ำ รัน production build และตรวจ generated output
ทุกครั้งที่เพิ่ม build plugin

**ก่อนหน้า:** [Configuration และ environment](07-configuration.md) · **ถัดไป:**
[การเชื่อมต่อ](09-integrations-auth-data-and-realtime.md)
