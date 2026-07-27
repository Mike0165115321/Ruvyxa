# Plugins

Plugin ของ Ruvyxa คือ TypeScript/JavaScript ปกติ: สร้างแพ็กเกจหนึ่งชุด, export plugin หนึ่งตัว แล้ว
ลงทะเบียนจุดเดียวใน `ruvyxa.config.ts` API เดียวกันใช้ได้ตั้งแต่เพิ่ม header, auth, route,
transform, virtual module, งานตอน dev, diagnostics ไปจนถึง native capability ที่ Framework รองรับ

## เริ่มแบบสั้นที่สุด

```bash
npx ruvyxa plugin create request-logger
cd request-logger
npm install
npm test
```

คำสั่งสร้างโฟลเดอร์ `request-logger/` โดยตรง ถ้าต้องการวางใน path อื่นให้ใช้ relative path
ที่ปลอดภัย:

```bash
npx ruvyxa plugin create request-logger --dir packages/request-logger
```

โครงสร้างที่ได้:

```text
request-logger/
├─ src/index.ts
├─ test/plugin.test.mjs
├─ package.json
├─ tsconfig.json
├─ README.md
└─ .gitignore
```

ตัวอย่างเริ่มต้นจะเพิ่ม `x-request-logger: active` ใน response แก้โค้ดหลักที่ `src/index.ts` ได้เลย
ไม่ต้องสร้าง wrapper หรือเลือกชนิด plugin เพิ่ม

## ติดตั้งและนำไปใช้

ถ้า plugin อยู่ใน repository เดียวกัน:

```bash
pnpm add ./packages/request-logger
```

ถ้า publish แล้ว:

```bash
npm install ruvyxa-plugin-request-logger
```

เพิ่มค่าที่ export เข้า `ruvyxa.config.ts`:

```ts
import { config } from 'ruvyxa/config'
import requestLogger from 'ruvyxa-plugin-request-logger'

export default config({
  plugins: [requestLogger],
})
```

ลำดับใน array คือลำดับทำงาน และชื่อ plugin ต้องไม่ซ้ำกัน

## แนวคิดเดียวที่ต้องจำ

Plugin มี `name` และ `register()` หนึ่งฟังก์ชัน เลือก destructure เฉพาะกลุ่มเต้ารับที่ต้องใช้:

```ts
import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: 'my-plugin',
  register({ http, build, dev, diagnostics, native }) {
    // เสียบ handler เข้ากับ socket ที่ต้องใช้
  },
})
```

ภายใน callback เขียน TypeScript/JavaScript ได้อิสระ: ใช้ npm package, เรียก API, อ่าน environment
ฝั่ง server, ติดต่อฐานข้อมูล, สร้างไฟล์, parse source หรือสร้าง Fetch `Response` ก็ได้ Plugin เป็น
trusted server/build code ไม่ใช่ sandbox จึงต้องตรวจ dependency เหมือน dependency ของแอป และอย่า
import module ฝั่ง server ของ plugin เข้า Client Component

## ตัวอย่าง HTTP

### เพิ่มหรือแก้ response header

```ts
import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: 'cache-policy',
  register({ http }) {
    http.onResponse({
      match: ['/api/*'],
      handler({ response }) {
        const headers = new Headers(response.headers)
        headers.set('cache-control', 'no-store')
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

### ป้องกัน route

```ts
http.onRequest({
  match: ['/admin/*'],
  handler({ request }) {
    if (request.headers.get('authorization') !== `Bearer ${process.env.ADMIN_TOKEN}`) {
      return new Response('Unauthorized', { status: 401 })
    }
  },
})
```

ไม่ return คือไปต่อ, return `Request` คือแทน request ปัจจุบัน, return `Response` คือหยุด chain ทันที
จะใช้ `next()` หรือ `next(replacement)` เพื่อเขียน flow ให้ชัดก็ได้

### สร้าง endpoint ที่ plugin เป็นเจ้าของ

```ts
http.route({
  method: 'GET',
  path: '/plugin/status',
  handler({ plugin }) {
    return Response.json({ plugin, ready: true })
  },
})
```

Path ต้องเป็น absolute path แบบ exact ส่วน `method` เป็น string, array หรือไม่ระบุเพื่อรับทุก method
ก็ได้ ถ้ามีเจ้าของ `method + path` ซ้ำ ระบบจะ fail ตอนเริ่ม ไม่เลือกตัวใดตัวหนึ่งแบบเงียบๆ

### กฎ `match`

| Pattern      | ความหมาย                 |
| ------------ | ------------------------ |
| ไม่ระบุ      | ทุก path                 |
| `*`          | ทุก path                 |
| `/api/users` | path นี้เท่านั้น         |
| `/api/*`     | `/api/` และ path ใต้ลงไป |

รองรับ wildcard หนึ่งตัวที่ท้าย pattern เท่านั้น และ match จาก pathname ไม่รวม query string

## ตัวอย่าง Build

### Transform source

```ts
build.onTransform(({ code, id, environment }) => {
  if (environment !== 'client' || !id.endsWith('.tsx')) return
  return code.replaceAll('__BUILD_CHANNEL__', JSON.stringify(process.env.CHANNEL ?? 'local'))
})
```

คืนค่าเป็น string หรือ `{ code, map }`; ไม่คืนค่าคือข้าม Transform ทำงานต่อกันตามลำดับที่ลงทะเบียน

### Alias และ virtual module

```ts
import path from 'node:path'

build.onResolve(({ id, root }) => {
  if (id === 'virtual:feature-flags') {
    return path.join(root, '.ruvyxa-virtual', 'feature-flags.ts')
  }
})

build.onLoad(({ id }) => {
  if (id.endsWith('feature-flags.ts')) {
    return {
      code: `export const flags = ${JSON.stringify({ checkoutV2: true })}`,
    }
  }
})
```

ในแอปใช้ได้ทันที:

```ts
import { flags } from 'virtual:feature-flags'
```

`onResolve` ต้องคืน absolute path ส่วน `onLoad` ทำงานก่อนอ่าน filesystem ดังนั้น path สำหรับ virtual
module ไม่จำเป็นต้องมีไฟล์จริง ระบบจำ binding ระหว่างชื่อ import กับ path ที่ resolve แล้วให้โดยตรง

### ตรวจ config และสร้าง artifact ตอน build

```ts
import { writeFile } from 'node:fs/promises'
import path from 'node:path'

build.onStart(({ root }) => {
  if (!process.env.SEARCH_API_KEY) {
    throw new Error(`SEARCH_API_KEY is required to build ${root}`)
  }
})

build.onComplete(({ outDir, manifest }) =>
  writeFile(path.join(outDir, 'plugin-manifest.json'), JSON.stringify(manifest, null, 2)),
)
```

`onStart` ทำงานก่อน staging output ส่วน `onComplete` ทำงานหลัง core output commit และก่อน adapter
สร้าง artifact ของตัวเอง

## รับ file change ตอน Development

```ts
dev.onFileChange({
  match: ['content/*'],
  async handler({ root, paths }) {
    await rebuildSearchIndex(root, paths)
  },
})
```

Path เป็น project-relative และใช้ `/` ทุกระบบปฏิบัติการ Pattern เป็น exact หรือมี wildcard หนึ่งตัว
ท้ายสุด ควรทำ handler ให้เร็ว; ถ้างานหนักให้ส่งเข้า queue ของระบบที่เหมาะสม

## Diagnostics

```ts
diagnostics.report({
  level: process.env.ANALYTICS_KEY ? 'info' : 'warning',
  code: 'ANL001',
  message: process.env.ANALYTICS_KEY
    ? 'Analytics integration enabled'
    : 'ANALYTICS_KEY is missing; analytics is disabled',
})
```

Level คือ `info`, `warning`, `error` และ code เป็น uppercase identifier ห้ามใส่ secret ใน message
ถ้าเป็น `error` ระบบจะหยุด startup; อีกสองแบบจะแสดงผ่าน host

## Native capability

Plugin ส่วนใหญ่ไม่ต้องใช้ `native` กลุ่มนี้มีไว้ต่อ option ของ JavaScript เข้ากับความสามารถที่
Ruvyxa implement ไว้แล้ว ปัจจุบันมี `realtime@1`:

```ts
native.claim('realtime@1', {
  path: '/__ruvyxa/realtime',
  heartbeatMs: 25_000,
  capacity: 256,
})
```

มีเจ้าของได้ตัวเดียว Plugin สร้าง native id เองหรือโหลด Rust code จาก npm ไม่ได้ งานทั่วไปใช้
`@ruvyxa/realtime/plugin` ซึ่งยึด contract ทางการนี้ให้แล้ว

## Plugin ที่รับ options แบบมี type

ถ้าอยากให้ผู้ใช้ตั้งค่า ให้ export factory:

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

ตรวจ options ใน factory เพื่อให้ error ชี้กลับไปที่ config ก่อนเริ่ม hook

## ทดสอบโดยไม่ต้องเปิดแอป

เรียก `register()` ด้วย socket spy ขนาดเล็กเหมือน test ที่ generator สร้าง:

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
    build: {
      onStart() {},
      onResolve() {},
      onLoad() {},
      onTransform() {},
      onComplete() {},
    },
    dev: { onFileChange() {} },
    diagnostics: { report() {} },
    native: { claim() {} },
  })

  assert.equal(plugin.apiVersion, 2)
  assert.ok(registration)
})
```

ถ้าใช้ build resolution, response body, route หรือ native capability ควรมี fixture app
อย่างน้อยหนึ่ง ชุดเพื่อทดสอบกับ host จริงด้วย

## Checklist ก่อน publish

1. รัน `npm test` และดูไฟล์ด้วย `npm pack --dry-run`
2. ใส่ `ruvyxa` ใน `peerDependencies`; `devDependencies` ใช้เฉพาะ build/test ภายในแพ็กเกจ
3. Publish `dist` และ declaration; ไม่ใส่ `src`, tests, `node_modules`, `.ruvyxa`
4. ใช้ ESM exports และ metadata `ruvyxa: { kind: "plugin", apiVersion: 2 }`
5. เขียน install, registration, options, security assumptions และ deployment limits ให้ครบ

## State, Security และ Performance

- Plugin เป็น trusted code ที่มีสิทธิ์ระดับ server/build ของแอป ไม่ใช่ sandbox
- Environment ส่วนตัวต้องอยู่ฝั่ง server ห้ามฝังลง client source
- Build กับ dev ไม่แชร์ module global และ worker pool แต่ละ process ก็แยก global
- อย่าใช้ memory ใน plugin เป็น session, database, lock หรือ durable state
- Response hook ต้อง buffer แบบมีเพดาน จึงไม่เหมาะกับไฟล์ download ขนาดใหญ่มาก
- Hook ที่ timeout จะไม่ถูก retry เพราะอาจทำ side effect ไปแล้ว
- ใช้ `console` ได้ Ruvyxa จะส่งไป stderr เพื่อไม่รบกวน protocol stdout

## แก้ปัญหาที่พบบ่อย

| อาการ                   | ตรวจอะไร                                                                       |
| ----------------------- | ------------------------------------------------------------------------------ |
| Unsupported API version | import `definePlugin` จาก `ruvyxa/plugin` แล้ว build package ใหม่              |
| ชื่อ plugin ซ้ำ         | ทุก plugin ใน config ต้องมีชื่อถาวรที่ไม่ซ้ำ                                   |
| Route conflict          | ให้ `method + path` มีเจ้าของตัวเดียว                                          |
| Import resolve ไม่ได้   | `onResolve` คืน absolute path แล้วให้ `onLoad` คืน source หรือมีไฟล์จริง       |
| Hook ไม่ทำงาน           | ตรวจ `match`, environment และมี plugin ใน `config.plugins` หรือไม่             |
| Dev change ไม่ match    | ใช้ path แบบ project-relative เช่น `content/*`                                 |
| Response ใหญ่เกิน       | ปรับ `security.pluginLimit` ภายในเพดาน หรือไม่ใช้ response hook กับ route นั้น |
| State ไม่ตรงกัน         | ย้าย durable state ออกจาก memory ของ plugin process                            |

เอกสารนี้รวมทั้งวิธีใช้งานและสถาปัตยกรรมของระบบ plugin ปัจจุบันไว้ในไฟล์เดียว ส่วน integration
แพ็กเกจทางการดู [Official packages](guides/th/official-packages.md)
