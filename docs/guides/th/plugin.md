คู่มือภาษาไทย

## Plugin คืออะไร

Plugin คือแพ็กเกจ TypeScript/JavaScript ที่นำ logic ของคุณมาเสียบเข้ากับ Framework ผ่าน
`definePlugin({ name, register })` เพียง contract เดียว ระบบเดียวใช้แนวคิด “ปลั๊กเสียบกับเต้ารับ”:
คุณเลือก socket ที่ต้องใช้ แล้วลงทะเบียน hook ใน `register()`

| Socket        | ใช้ทำอะไร                                                        |
| ------------- | ---------------------------------------------------------------- |
| `http`        | request, response และ endpoint ของ plugin                        |
| `build`       | ตรวจค่า, resolve/load module, transform source และสร้าง artifact |
| `dev`         | รับเหตุการณ์ file change ตอน development                         |
| `diagnostics` | รายงาน info, warning และ error ตอนเริ่มระบบ                      |
| `native`      | ขอใช้ capability ที่ Framework มีให้แบบ versioned                |

Plugin เป็น trusted server/build code ไม่ใช่ sandbox จึงเขียน logic JavaScript/TypeScript ได้อิสระ
แต่ต้องรับผิดชอบ dependency, secret, side effect และความปลอดภัยของตัวเอง

## ขั้นที่ 0: เตรียมเครื่องมือ

ต้องมี Node.js, npm/pnpm และโปรเจกต์ Ruvyxa ที่รัน `ruvyxa dev` ได้ก่อน คำสั่งด้านล่างใช้ npm
เป็นตัวอย่าง เปลี่ยนเป็น pnpm ได้ตาม package manager ของโปรเจกต์

## ขั้นที่ 1: สร้าง plugin

ใช้คำสั่งเดียวนี้ ไม่ต้องเลือกชนิด plugin

```bash
npx ruvyxa plugin create request-logger
cd request-logger
npm install
npm test
```

ถ้าต้องการสร้างไว้ในโฟลเดอร์ย่อยของ monorepo:

```bash
npx ruvyxa plugin create request-logger --dir packages/request-logger
```

โครงสร้างที่ได้:

```text
request-logger/
├─ src/index.ts          # โค้ด plugin หลัก
├─ test/plugin.test.mjs  # test ของ register contract
├─ package.json
├─ tsconfig.json
├─ README.md
└─ .gitignore
```

## ขั้นที่ 2: เขียน plugin ตัวแรก

เปิด `src/index.ts` แล้วเขียนดังนี้:

```ts
import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: 'request-logger',
  register({ http }) {
    http.onResponse({
      handler({ response }) {
        const headers = new Headers(response.headers)
        headers.set('x-request-logger', 'active')
        return new Response(response.body, {
          status: response.status,
          statusText: response.statusText,
          headers,
        })
      },
    })
  },
})
```

สิ่งที่เกิดขึ้นคือ `definePlugin()` จะตรวจชื่อและคืน plugin object ที่พร้อมใช้งาน แล้ว
`register({ http })` จะได้เฉพาะเต้ารับ HTTP ที่ plugin นี้ใช้

## ขั้นที่ 3: ติดตั้งและลงทะเบียนในแอป

ถ้า plugin อยู่ในเครื่องเดียวกับแอป:

```bash
cd my-app
pnpm add ../packages/request-logger
```

ถ้า plugin publish แล้ว:

```bash
npm install ruvyxa-plugin-request-logger
```

เพิ่ม plugin ใน `ruvyxa.config.ts`:

```ts
import { config } from 'ruvyxa/config'
import requestLogger from 'ruvyxa-plugin-request-logger'

export default config({
  plugins: [requestLogger],
})
```

ลำดับใน `plugins` คือ registration order และชื่อ plugin ต้องไม่ซ้ำกัน

## ขั้นที่ 4: เปิดแอปและตรวจผล

```bash
npx ruvyxa dev
```

เปิดหน้าใดก็ได้ แล้วตรวจ response header `x-request-logger: active` ด้วย browser DevTools หรือ:

```bash
curl -I http://localhost:3000/
```

ถ้าเห็น header แสดงว่า plugin ถูกโหลด, register และทำงานผ่าน host จริงแล้ว

## ตัวอย่าง HTTP ที่ใช้บ่อย

### ป้องกัน route

```ts
register({ http }) {
  http.onRequest({
    match: ['/admin/*'],
    handler({ request }) {
      if (request.headers.get('authorization') !== `Bearer ${process.env.ADMIN_TOKEN}`) {
        return new Response('Unauthorized', { status: 401 })
      }
    },
  })
}
```

ไม่ return คือปล่อยให้ request ไปต่อ, return `Request` คือแทน request, return `Response` คือหยุด
chain ทันที และสามารถใช้ `next()` หรือ `next(replacement)` เพื่อควบคุมลำดับเอง

### สร้าง endpoint ของ plugin

```ts
http.route({
  method: 'GET',
  path: '/plugin/status',
  handler({ plugin }) {
    return Response.json({ plugin, ready: true })
  },
})
```

`method + path` ห้ามชนกับ plugin อื่น ระบบจะ fail ตอนเริ่มแทนการเลือกเจ้าของแบบเงียบ ๆ

### กำหนด `match`

| Pattern          | ความหมาย                 |
| ---------------- | ------------------------ |
| ไม่ระบุ หรือ `*` | ทุก path                 |
| `/api/users`     | path นี้เท่านั้น         |
| `/api/*`         | `/api/` และ path ใต้ลงไป |

Match จาก pathname ไม่รวม query string และ wildcard ใช้ได้ที่ท้าย pattern หนึ่งตัว

## ขั้นที่ 5: ใช้ Build socket

### Transform source

```ts
register({ build }) {
  build.onTransform(({ code, id, environment }) => {
    if (environment !== 'client' || !id.endsWith('.tsx')) return
    return code.replaceAll('__BUILD_CHANNEL__', JSON.stringify(process.env.CHANNEL ?? 'local'))
  })
}
```

คืน string หรือ `{ code, map }`; ไม่คืนค่าคือไม่ transform ไฟล์นั้น Hook ทำงานตามลำดับที่ลงทะเบียน

### Alias และ virtual module

```ts
import path from 'node:path'

register({ build }) {
  build.onResolve(({ id, root }) => {
    if (id === 'virtual:feature-flags') {
      return path.join(root, '.ruvyxa-virtual', 'feature-flags.ts')
    }
  })

  build.onLoad(({ id }) => {
    if (id.endsWith('feature-flags.ts')) {
      return { code: `export const flags = ${JSON.stringify({ checkoutV2: true })}` }
    }
  })
}
```

ในแอป import ได้ตามปกติ:

```ts
import { flags } from 'virtual:feature-flags'
```

`onResolve` ต้องคืน absolute path ส่วน `onLoad` สามารถคืน source โดยไม่ต้องมีไฟล์จริง

### ตรวจ config และสร้าง artifact

```ts
import { writeFile } from 'node:fs/promises'
import path from 'node:path'

register({ build }) {
  build.onStart(({ root }) => {
    if (!process.env.SEARCH_API_KEY) throw new Error(`SEARCH_API_KEY is required for ${root}`)
  })

  build.onComplete(({ outDir, manifest }) =>
    writeFile(path.join(outDir, 'plugin-manifest.json'), JSON.stringify(manifest, null, 2)),
  )
}
```

`onStart` ทำงานก่อน build output ส่วน `onComplete` ทำงานหลัง core output พร้อมแล้ว

## ขั้นที่ 6: ใช้ Dev, Diagnostics และ Native

รับ file change ตอน dev:

```ts
register({ dev }) {
  dev.onFileChange({
    match: ['content/*'],
    handler({ root, paths }) {
      console.log('changed', root, paths)
    },
  })
}
```

`paths` เป็น project-relative และใช้ `/` ทุกระบบปฏิบัติการ

รายงาน diagnostics:

```ts
register({ diagnostics }) {
  diagnostics.report({
    level: process.env.ANALYTICS_KEY ? 'info' : 'warning',
    code: 'ANL001',
    message: 'Analytics configuration checked',
  })
}
```

ระดับมี `info`, `warning`, `error`; `error` จะหยุด startup ห้ามใส่ secret ใน message

ใช้ native capability ที่ Framework รองรับเท่านั้น:

```ts
register({ native }) {
  native.claim('realtime@1', {
    path: '/__ruvyxa/realtime',
    heartbeatMs: 25_000,
    capacity: 256,
  })
}
```

Native capability มีเจ้าของได้ตัวเดียวและ plugin ไม่สามารถโหลด Rust code ใหม่จาก npm ได้

## ขั้นที่ 7: รับ options ให้ผู้ใช้ตั้งค่า

สร้าง factory ที่คืน `RuvyxaPlugin`:

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
    register({ http }) {
      http.onRequest({
        match,
        handler({ request }) {
          if (!request.headers.has(header)) {
            return new Response(`Missing ${header}`, { status: 400 })
          }
        },
      })
    },
  })
}
```

ผู้ใช้จะตั้งค่าใน config ได้แบบนี้:

```ts
import { audit } from 'ruvyxa-plugin-audit'

export default config({
  plugins: [audit({ match: ['/api/*'], header: 'x-trace-id' })],
})
```

## ขั้นที่ 8: ทดสอบ plugin

เริ่มจาก test ที่ไม่ต้องเปิดแอป โดยเรียก `register()` กับ socket spy:

```js
import assert from 'node:assert/strict'
import test from 'node:test'
import plugin from '../dist/index.js'

test('registers a response hook', async () => {
  let registration
  await plugin.register({
    http: {
      onRequest() {},
      onResponse(value) {
        registration = value
      },
      route() {},
    },
    build: { onStart() {}, onResolve() {}, onLoad() {}, onTransform() {}, onComplete() {} },
    dev: { onFileChange() {} },
    diagnostics: { report() {} },
    native: { claim() {} },
  })
  assert.equal(plugin.name, 'request-logger')
  assert.ok(registration)
})
```

จากนั้นทดสอบกับแอปจริง:

```bash
npm test
npm pack --dry-run
npx ruvyxa check --root ../my-app
npx ruvyxa test:parity --root ../my-app
```

## ขั้นที่ 9: เตรียม publish

ก่อน publish ให้ตรวจรายการนี้:

1. `peerDependencies` มี `ruvyxa` หรือ `@ruvyxa/core` ตาม API ที่ใช้
2. `dist` และ declaration ถูกสร้างครบ
3. tarball ไม่รวม test, `node_modules`, `.ruvyxa` หรือ dependency แบบ `workspace:`
4. package เป็น ESM และไม่ต้องมี metadata เฉพาะของ Ruvyxa ใน `package.json`
5. README อธิบาย install, registration, options, security และ deployment limits
6. รัน `npm test` และ `npm pack --dry-run` ก่อน `npm publish`

## ความปลอดภัยและข้อจำกัด

- Plugin มีสิทธิ์ระดับ server/build และไม่ใช่ sandbox
- Secret ต้องอยู่ server-side ห้ามฝังลง client bundle
- อย่าใช้ memory ของ plugin เป็น session, database, lock หรือ durable state
- Response hook มี buffer limit จึงไม่เหมาะกับ download ไฟล์ใหญ่มาก
- Hook ที่ timeout จะไม่ถูก retry อัตโนมัติ เพราะ side effect อาจเกิดไปแล้ว
- ใช้ `console` ได้ แต่ protocol ใช้ stdout จึงไม่ควรเขียนข้อมูล protocol เองลง stdout

## แก้ปัญหาเบื้องต้น

| อาการ                     | วิธีตรวจ                                                                     |
| ------------------------- | ---------------------------------------------------------------------------- |
| Plugin ไม่ผ่าน validation | ตรวจว่ามี `name` ที่ไม่ว่างและ `register(api)` เป็น function                 |
| Plugin ไม่ถูกโหลด         | ตรวจ `plugins` ใน `ruvyxa.config.ts` และ default export                      |
| ชื่อ plugin ซ้ำ           | ตั้ง `name` ให้ไม่ซ้ำทุกตัว                                                  |
| Route conflict            | ตรวจคู่ `method + path`                                                      |
| Alias หาไม่พบ             | `onResolve` ต้องคืน absolute path และ `onLoad` ต้องคืน source หรือมีไฟล์จริง |
| Hook ไม่ทำงาน             | ตรวจ `match`, `environment` และลำดับใน `config.plugins`                      |
| Dev change ไม่ match      | ใช้ project-relative path เช่น `content/*`                                   |
| Response ใหญ่เกิน         | ลดการใช้ response hook หรือปรับ `security.pluginLimit` ภายในเพดาน            |

---

#
