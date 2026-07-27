# คู่มือเขียน Plugin สำหรับ Ruvyxa

Plugin คือโมดูล TypeScript/JavaScript ที่ export ค่า `definePlugin(...)` หนึ่งตัว
เพื่อเพิ่มความสามารถ ให้แอปผ่าน hook ของ HTTP, build, development, diagnostics และ native capability
ของ framework Plugin เป็น trusted server/build code ไม่ใช่ sandbox

## 1. เลือกรูปแบบการเขียน

เริ่มจาก declaration แบบสั้นก่อน แต่ละ declaration สร้าง hook ชนิดนั้นได้หนึ่งตัว ใช้
`register(api)` เมื่อจำเป็นต้องใช้ hook ชนิดเดิมหลายตัว, ต้องคุมลำดับละเอียด หรือสร้าง hook ตาม
เงื่อนไข/ลูป

| ความต้องการ                                         | แบบสั้น            | แบบเต็ม                   |
| --------------------------------------------------- | ------------------ | ------------------------- |
| เพิ่ม response header                               | `headers`          | `http.onResponse(...)`    |
| request/response hook หนึ่งตัว หรือ routes เล็กน้อย | `http`             | `http`                    |
| build hook แต่ละชนิดหนึ่งตัว                        | `build`            | `build`                   |
| file-change handler หนึ่งตัว                        | `dev.onFileChange` | `dev.onFileChange(...)`   |
| diagnostic คงที่                                    | `diagnostics`      | `diagnostics.report(...)` |
| realtime ที่กำหนดค่าตายตัว                          | `native.realtime`  | `native.claim(...)`       |
| hook ชนิดเดียวกันหลายตัว                            | —                  | `register(api)`           |

ถ้าใช้ร่วมกัน ระบบลงทะเบียน declaration แบบสั้นตามลำดับ HTTP → build → dev → diagnostics → native
แล้วจึงเรียก `register(api)` เป็นลำดับสุดท้าย

## 2. สร้าง package และติดตั้งเข้าแอป

```bash
npx ruvyxa plugin create request-logger
cd request-logger
npm install
npm test
```

ใน monorepo ระบุที่อยู่ด้วย `--dir`:

```bash
npx ruvyxa plugin create request-logger --dir packages/request-logger
```

สร้างเสร็จจะมี `src/index.ts`, `test/plugin.test.mjs`, `package.json`, `tsconfig.json`, `README.md`
และ `.gitignore` ติดตั้ง package ในแอปแล้วลงทะเบียน default export:

```bash
cd ../my-app
pnpm add ../packages/request-logger
```

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'
import requestLogger from 'ruvyxa-plugin-request-logger'

export default config({ plugins: [requestLogger] })
```

ชื่อ `name` ต้องไม่ว่างและห้ามซ้ำ ส่วนลำดับใน `plugins` คือลำดับการลงทะเบียน

## 3. Plugin ตัวแรก: `headers`

```ts
import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: 'request-logger',
  headers: { 'x-request-logger': 'active' },
})
```

`headers` รับค่า `HeadersInit` และเพิ่มหรือแทน header ที่ระบุใน response ให้เปิดแอปและตรวจผ่าน host
จริง:

```bash
npx ruvyxa dev
curl -I http://localhost:3000/
```

## 4. API แบบสั้นครบทุกส่วน

ตัวอย่างนี้ใช้ทุก section ที่รองรับ:

```ts
import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: 'site-tools',
  headers: { 'x-site-tools': 'enabled' },
  http: {
    match: ['/admin/*'],
    onRequest({ request }) {
      if (!request.headers.has('authorization'))
        return new Response('Unauthorized', { status: 401 })
    },
    onResponse({ response }) {
      return response
    },
    routes: [
      { method: 'GET', path: '/plugin/status', handler: ({ plugin }) => Response.json({ plugin }) },
    ],
  },
  build: {
    onStart({ root, outDir }) {
      console.log('building', root, outDir)
    },
    onComplete({ manifest }) {
      console.log('finished', manifest)
    },
  },
  dev: {
    onFileChange({ paths }) {
      console.log('changed', paths)
    },
  },
  diagnostics: { level: 'info', code: 'SITE001', message: 'Site tools enabled' },
  native: { realtime: true },
})
```

`http` และ `build` ห้ามเป็น object ว่าง; ต้องมี behavior อย่างน้อยหนึ่งตัว

### HTTP request และ `match`

`onRequest` รับ `{ plugin, root, request, next }` ไม่ return คือใช้ request เดิมต่อไป, return
`Request` คือแทน request, return `Response` คือหยุด request flow และตอบทันที

```ts
http: {
  match: ['/admin/*'],
  onRequest({ request }) {
    if (request.headers.get('authorization') !== `Bearer ${process.env.ADMIN_TOKEN}`) {
      return new Response('Unauthorized', { status: 401 })
    }
  },
},
```

ใช้ `next()` หรือ `next(replacementRequest)` เมื่อต้องการควบคุมการไปต่อแบบชัดเจน Pattern ใช้ exact
path, `*`, หรือ prefix ที่ลงท้าย `*` เช่น `/api/*`; ระบบ match กับ decoded pathname โดยไม่รวม query
string ไม่ระบุ `match` คือทุก path

### HTTP response และ route

`onResponse` รับ `{ plugin, root, request, response, next }` ไม่ return คือใช้ response เดิม; return
`Response` คือแทน response

```ts
http: {
  onResponse({ response }) {
    const headers = new Headers(response.headers)
    headers.set('x-request-checked', 'yes')
    return new Response(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers,
    })
  },
  routes: [
    {
      method: ['GET', 'HEAD'],
      path: '/plugin/health',
      handler({ plugin, request }) {
        return Response.json({ plugin, method: request.method })
      },
    },
  ],
},
```

Route มี `path` แบบ exact, `method` หนึ่งค่า/หลายค่า/ไม่ระบุ (ทุก method) และ handler ที่คืน
`Response` หรือ `Promise<Response>` คู่ method/path ต้องไม่ซ้ำข้าม plugin

### Build: lifecycle, virtual module และ transform

```ts
import path from 'node:path'

export default definePlugin({
  name: 'virtual-flags',
  build: {
    onStart({ root, outDir }) {
      console.log({ root, outDir })
    },
    onResolve({ id, root }) {
      return id === 'virtual:flags' ? path.join(root, '.virtual', 'flags.ts') : undefined
    },
    onLoad({ id }) {
      return id.endsWith('flags.ts') ? { code: 'export const checkoutV2 = true' } : undefined
    },
    onTransform({ code, id, environment }) {
      if (environment !== 'client' || !id.endsWith('.tsx')) return
      return { code: code.replaceAll('__CHANNEL__', JSON.stringify('stable')) }
    },
    onComplete({ outDir, manifest }) {
      console.log('output', outDir, manifest)
    },
  },
})
```

`onResolve` รับ `id`, `importer?`, `root`, `environment` และคืน absolute path, `null` หรือไม่คืนค่า
`onLoad`/`onTransform` คืน source string, `{ code, map }`, `null` หรือไม่คืนค่า Environment คือ
`client`, `server`, `edge`, `worker`, หรือ `shared`

### Dev, diagnostics และ native

```ts
export default definePlugin({
  name: 'content-tools',
  dev: {
    onFileChange: {
      match: ['content/*'],
      handler({ root, paths }) {
        console.log(root, paths)
      },
    },
  },
  diagnostics: [
    { level: 'info', code: 'CONTENT001', message: 'Content tools enabled' },
    { level: 'warning', code: 'CONTENT002', message: 'Remote sync is disabled' },
  ],
  native: { realtime: { path: '/events', heartbeatMs: 25_000, capacity: 256 } },
})
```

`dev.onFileChange` เขียนเป็น function ตรง ๆ ได้เช่นกัน; `match` เป็น project-relative pattern
`diagnostics` เป็นค่าเดียวหรือ array และ level คือ `info`, `warning`, `error` ส่วน
`native.realtime: true` ใช้ค่า default; options มี `path`, `heartbeatMs`, `capacity` และ realtime
มีเจ้าของได้เพียง plugin เดียว

## 5. `register(api)`: escape hatch สำหรับขั้นสูง

ใช้เมื่อ hook ชนิดเดียวกันต้องมีหลายตัว หรือต้องคุมลำดับ/สร้างตามเงื่อนไข:

```ts
import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: 'security',
  register({ http, build, diagnostics }) {
    http.onRequest(requireAuthentication)
    http.onRequest(rateLimit)
    http.onResponse({ handler: addAuditHeader })
    http.route({ method: 'GET', path: '/plugin/metrics', handler: metrics })

    build.onTransform(instrumentClientCode)
    build.onTransform(removeDebugCalls)
    diagnostics.report({ level: 'info', code: 'SEC001', message: 'Security enabled' })
  },
})
```

socket ที่ใช้ได้ทั้งหมด:

```ts
register({ http, build, dev, diagnostics, native }) {
  http.onRequest(handlerOrRegistration)
  http.onResponse(handlerOrRegistration)
  http.route({ path, method, handler })
  build.onStart(hook); build.onResolve(hook); build.onLoad(hook)
  build.onTransform(hook); build.onComplete(hook)
  dev.onFileChange(handlerOrRegistration)
  diagnostics.report({ level, code, message })
  native.claim('realtime@1', options)
}
```

ผสมแบบสั้นและเต็มได้ โดย header จะถูกลงทะเบียนก่อน audit hook:

```ts
definePlugin({
  name: 'hybrid',
  headers: { 'x-powered-by': 'ruvyxa' },
  register({ http }) {
    http.onResponse({ handler: addAuditHeader })
  },
})
```

## 6. Plugin ที่รับ options

export factory ที่คืน `RuvyxaPlugin` เพื่อให้ผู้ใช้กำหนดค่าได้:

```ts
import { definePlugin, type RuvyxaPlugin } from 'ruvyxa/plugin'

export interface AuditOptions {
  match?: readonly string[]
  header?: string
}

export function audit(options: AuditOptions = {}): RuvyxaPlugin {
  const match = options.match ?? ['/api/*']
  const header = options.header ?? 'x-audit-id'
  return definePlugin({
    name: 'audit',
    http: {
      match,
      onRequest({ request }) {
        if (!request.headers.has(header)) return new Response(`Missing ${header}`, { status: 400 })
      },
    },
  })
}
```

ผู้ใช้ใส่ใน config ได้เป็น `plugins: [audit({ header: 'x-trace-id' })]`

## 7. ทดสอบและ publish

ทดสอบ registration contract โดยส่ง socket spy เข้า `plugin.register` แล้วทดสอบ fixture app สำหรับ
request/response, route หรือ build behavior:

```bash
npm test
npm pack --dry-run
npx ruvyxa check --root ../my-app
npx ruvyxa test:parity --root ../my-app
```

ก่อน publish ให้มี `ruvyxa` หรือ `@ruvyxa/core` ใน peer dependency ตาม API ที่ใช้, publish ESM
output และ declaration, ตรวจ tarball และอย่าให้มี test, `node_modules`, `.ruvyxa` หรือ `workspace:`
dependency

## 8. Validation และแก้ปัญหา

| อาการ                     | ตรวจตรงจุด                                                                       |
| ------------------------- | -------------------------------------------------------------------------------- |
| Plugin validation ไม่ผ่าน | ต้องมี `name` ที่ไม่ว่าง และมี declaration อย่างน้อยหนึ่งตัวหรือ `register(api)` |
| Plugin ไม่ถูกโหลด         | ตรวจ `plugins` และ default import/export ใน config                               |
| ชื่อซ้ำ                   | เปลี่ยน `name` ให้ไม่ซ้ำ                                                         |
| Route ชนกัน               | ให้ทุกคู่ method/path ไม่ซ้ำ                                                     |
| Hook ไม่ทำงาน             | ตรวจ `match`, environment และลำดับ plugin                                        |
| Virtual import หาไม่พบ    | `onResolve` ต้องคืน absolute path แล้ว `onLoad` คืน source หรือมีไฟล์จริง        |
| Dev handler ไม่ทำงาน      | ใช้ project-relative match เช่น `content/*`                                      |
| Native claim ไม่ผ่าน      | ใช้ capability ที่รองรับและต้องมีเจ้าของคนเดียว                                  |

ห้ามส่ง private environment value ลง source ฝั่ง client หรือใส่ secret ใน diagnostic message
