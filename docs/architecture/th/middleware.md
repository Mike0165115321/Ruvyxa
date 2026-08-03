# สถาปัตยกรรม Middleware

Ruvyxa มีตำแหน่ง middleware HTTP สองแบบ แต่ใช้ request pipeline ที่มีลำดับแน่นอนร่วมกัน:

1. นโยบายในตัวใช้ Rust/Tower สำหรับ CORS, rate limiting, timing, logging, compression และ security
   headers
2. ปลั๊กอินแอปหรือแพ็กเกจใช้ socket `http` ด้วย Fetch primitives มาตรฐาน

```ts
import { config } from 'ruvyxa/config'
import { definePlugin } from 'ruvyxa/plugin'

const auth = definePlugin({
  name: 'auth',
  register({ http }) {
    http.onRequest({
      match: ['/api/*'],
      handler({ request }) {
        if (!request.headers.has('authorization')) {
          return new Response('Unauthorized', { status: 401 })
        }
      },
    })
    http.onResponse({
      match: ['/api/*'],
      handler({ response, plugin }) {
        const headers = new Headers(response.headers)
        headers.set('x-plugin', plugin)
        return new Response(response.body, { status: response.status, headers })
      },
    })
  },
})

export default config({
  middleware: { builtin: { timing: true, log: true } },
  plugins: [auth],
})
```

การคืนค่าว่างหมายถึงทำต่อใน chain; `Request` ที่คืนมาจะแทน request เดิม และ `Response` จะ
short-circuit request hooks กับ application routing. Response hook สามารถแทน response ได้ `next()`
และ `next(replacement)` ใช้เพื่อทำต่อแบบชัดเจนใน branch ที่ซับซ้อนได้

## เลเยอร์ Tower ในตัว

เลเยอร์ในตัวถูกออกแบบให้เรียบง่ายโดยตั้งใจ — ไม่มี plugin ordering DSL, ไม่มี abstract compression
algorithm enum, ไม่มี `RateLimitStore` trait ลำดับถูกกำหนดตายตัวและนำไปใช้กับ Axum router จากล่าง
ขึ้นบน: compression (เปิดเสมอ, brotli + gzip, ใช้เฉพาะ response ที่รู้ content-length) → CORS
(เขียนขึ้นเองไม่ใช่ `tower_http::cors::CorsLayer`; short-circuit preflight `OPTIONS` เป็น `204`,
เติม `Vary: Origin` ให้ origin ที่ถูกปฏิเสธ) → rate limiting (token bucket แบบ in-process คีย์ด้วย
`ip` หรือ `header:<name>`, ไม่มี Redis backend, ครบ 10,000 คีย์ที่ติดตามจะ GC แบบ lazy) → timing
(`X-Response-Time`) → request logging (`x-request-id`) → custom headers → route handler
`middleware.workers` (1-8) และ `middleware.timeoutMs` (1-300000) ใช้กำหนดขนาด/ขอบเขตของ TypeScript
plugin pool เท่านั้น — เลเยอร์ในตัวทำงาน in-process เสมอและไม่มีขอบเขตจำกัด

## ขอบเขต runtime

Rust server เป็นเจ้าของ socket, Axum routing, body limits และ final response ส่วน worker pool
`PluginHost` ที่มีขอบเขตจะเรียก callback ใน Node หรือ Bun ผ่าน NDJSON:

```text
Rust -> { hook: "http.request", request: ... }
Node -> { ok: true, result: { kind: "request" | "response", ... } }

Rust -> { hook: "http.response", request: ..., response: ... }
Node -> { ok: true, result: { response: ... } }
```

Header ส่งเป็น ordered pairs และ body เป็น base64 Rust ตรวจผลลัพธ์ทุกค่าก่อนส่งเข้า Axum และ
บังคับใช้ `security.pluginLimit` ก่อน buffer response. Exact route ที่ปลั๊กอินเป็นเจ้าของจะ
ลงทะเบียนผ่าน `http.route()`; method/path ซ้ำกันจะถูกปฏิเสธตอน registry เริ่มทำงาน

## ลำดับและความล้มเหลว

Request handler ของปลั๊กอินทำงานตามลำดับใน config/source; response แรกที่ชัดเจนจะหยุด request chain.
Response handler ทำงานต่อเนื่องกับ response ปัจจุบัน Connection middleware ทำงานก่อน plugin request
และ native security headers จะเติมหลัง plugin response เพื่อคงค่าที่แอปหรือ ปลั๊กอินกำหนดชัดเจนไว้

ข้อยกเว้นหรือ return ที่ไม่รองรับทำให้ call ล้มเหลวพร้อม named diagnostic timeout จะไม่ retry
handler ที่อาจมี side effect แล้ว แต่ worker ที่เสียจะถูกแทนที่ Console output อยู่บน stderr ขณะที่
stdout สงวนไว้สำหรับ protocol

ไม่มี middleware-plugin object model แยกต่างหาก HTTP คือ socket หนึ่งของปลั๊กอินเดียวกันที่
สามารถลงทะเบียน build, dev, diagnostics หรือ native behavior ได้ด้วย

ปลั๊กอินที่ประกาศความสามารถ `realtime@1` จะเปิดเผย WebSocket path, ช่วง heartbeat และ capacity ผ่าน
registry descriptor เดียวกันที่ dev server อ่านตอนเริ่มทำงาน — ดูรายละเอียดระดับการเชื่อมต่อได้ที่
[Dev Server: Realtime Runtime](dev-server.md)
