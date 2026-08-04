# การเชื่อมต่อ: authentication, data, realtime, adapter และ testing

## Authentication

`@ruvyxa/auth` export `createAuth`, provider helper `google` และ `github`, memory store, type และ
`AuthError` package export `@ruvyxa/auth/client` และ `@ruvyxa/auth/plugin` แยกกัน provider contract
ที่รองรับมี credentials, OAuth, magic link และ WebAuthn memory store เป็น process-local; เลือก
durable shared store ก่อน scale authentication แบบหลาย instance

```ts
import { createAuth, memoryAuthStore, memoryRateLimitStore } from '@ruvyxa/auth'

const auth = createAuth({
  secret: process.env.RUVYXA_AUTH_SECRET!,
  origin: 'https://example.test',
  store: memoryAuthStore({ development: true }),
  rateLimitStore: memoryRateLimitStore({ development: true }),
  providers: {},
})
```

`AuthOptions` contract ที่แน่นอนถูก export โดย package อย่าใช้ placeholder ในตัวอย่างนี้เป็น secret
จริง register plugin ที่ auth runtime คืนมา แล้วใช้ browser entry point แยกเฉพาะใน client code:

```ts
// ruvyxa.config.ts
export default config({ plugins: [auth.plugin] })

// a client module
import { createAuthClient } from '@ruvyxa/auth/client'
const authClient = createAuthClient()
```

auth path ปริยายคือ `/__ruvyxa/auth` client มี `login`, `logout`, `session` และ `oauth`;
`createAuth` มี `handle`, `login`, `getSession` และ `logout` สำหรับ server-side integration memory
store ต้องการ `{ development: true }` และตั้งใจให้ production build ล้มเหลวด้วย `RUV3105`; ให้ใช้
durable implementation ของ `AuthStore` และ `AuthRateLimitStore` แทน `createAuthPlugin(bridge)`
ใช้ได้เมื่อ ต้องมี custom bridge

## Database

`@ruvyxa/database` เป็น typed normalized-operation layer ไม่ใช่ ORM migration system
`createDatabase<TSchema>(adapter)` สร้าง model delegate สำหรับ `findMany`, `findFirst`,
`findUnique`, `create`, `createMany`, `update`, `updateMany`, `delete`, `deleteMany` และ `count`
มันมี `prismaAdapter`, `dynamoAdapter` และ `defineDatabaseAdapter`; adapter error ใช้
`RUV3001`–`RUV3003`

```ts
import { createDatabase, defineDatabaseAdapter } from '@ruvyxa/database'
const adapter = defineDatabaseAdapter({
  name: 'example',
  execute: async (operation) => {
    throw new Error(`implement ${operation.kind}`)
  },
})
const db = createDatabase<{ todo: { id: string; title: string } }>(adapter)
```

framework ไม่มี database server, migration engine หรือ backup service
ส่วนเหล่านี้เป็นความรับผิดชอบของ application/infrastructure

## Realtime และ adapter

`@ruvyxa/realtime/plugin` export `realtime()` ซึ่ง claim native realtime capability มันปฏิเสธ build
ที่ไม่ใช่ long-lived Node/Bun output และปฏิเสธ adapter aws, cloudflare, firebase, netlify, static
และ vercel ด้วย `RUV3201` `@ruvyxa/realtime/client` export `createRealtimeClient`; จำกัด active
channel ที่ 16 และ reconnect ด้วย bounded exponential backoff

มี first-party adapter package สำหรับ Node, Bun, static, Vercel, Netlify, Cloudflare, Railway,
Render, Firebase และ AWS เลือก build ด้วย `npm run build -- --adapter <name>` หรือ config `adapter`;
ดู [Deploy, run และ operate](15-deploy-run-and-operate.md) `@ruvyxa/testing` export `mockLoader`,
`mockAction` และ `mockCache` สำหรับ unit test

**ก่อนหน้า:** [Plugin และ middleware](08-plugins-middleware.md) · **ถัดไป:**
[CLI reference](10-cli.md)
