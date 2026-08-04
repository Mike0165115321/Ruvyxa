# ข้อมูล, action และ API route

## Loader และ in-memory cache

`loader(handler)` สร้าง async callable ที่ติดเครื่องหมายเป็น Ruvyxa loader handler รับ
`{ params, request, cache }` `cache(key)` คือ cache ภายใน process ที่จำกัด LRU 1024 entry, TTL
ปริยาย 60 วินาที, รองรับ stale-while-revalidate และ prefix invalidation มันไม่ใช่ distributed cache

```ts
// app/products/server.ts
import { cache, loader } from 'ruvyxa/server'

export const products = loader(async ({ cache }) =>
  cache('products:list')
    .ttl('5m')
    .swr('1m')
    .get(async () => {
      const response = await fetch('https://example.test/products')
      if (!response.ok) throw new Error(`Upstream returned ${response.status}`)
      return response.json()
    }),
)
```

ระยะเวลา cache รับจำนวนเต็มบวกตามด้วย `ms`, `s`, `m`, `h` หรือ `d` `invalidateCache('products')` ลบ
`products` และ key ที่ขึ้นต้นด้วย `products:`; หากไม่ส่ง argument จะล้าง cache ทั้ง process เรียก
`cacheStats()` เพื่อได้ `{ size, maxEntries }`

## Server action

สร้าง action ด้วย `action.input(schema).handler(handler)` schema ต้องมี synchronous `parse(value)`
action handler รับ `input` ที่ parse แล้ว, request, user data (หากมี) และ `invalidate(key)`
`.realtime(channels?)` จะ publish หลังเรียกสำเร็จเมื่อ realtime capability ถูกตั้งค่า

```ts
// app/todos/action.ts
import { action } from 'ruvyxa/server'

export const createTodo = action
  .input({
    parse(value: unknown) {
      if (!value || typeof value !== 'object' || !('title' in value))
        throw new Error('title required')
      return { title: String(value.title).trim() }
    },
  })
  .realtime('todos')
  .handler(async ({ input, invalidate }) => {
    if (!input.title) throw new Error('title required')
    invalidate('todos')
    return { id: crypto.randomUUID(), ...input, completed: false }
  })
```

action รับ realtime channel ได้สูงสุด 16 ช่อง ชื่อ channel ใช้ตัวอักษร, ตัวเลข, `:`, `.`, `_`, `/`
หรือ `-` ความยาว 1–128 กำหนด payload และ rate restriction ของ action ใต้ `security`; ดู
[Security](13-security.md)

## API route

วาง `route.ts` ใน folder เป้าหมาย และ export HTTP method function ตัวพิมพ์ใหญ่
`app/api/echo/route.ts` ใน demo export `POST({ request })`, อ่าน JSON และคืน `Response.json` ใช้
response helper มาตรฐานได้: `json(data, init)`, `redirect(location, status)` และ `notFound(message)`
จาก `ruvyxa/server`

```ts
// app/api/health/route.ts
export function GET() {
  return Response.json({ ok: true })
}
```

route handler ต้อง validate body ที่ไม่น่าเชื่อถือก่อนใช้ API payload limit อยู่ที่
`security.apiLimit`; action payload ใช้ `security.actionLimit`

**ก่อนหน้า:** [Routing และ rendering](04-routing-rendering.md) · **ถัดไป:**
[UI, navigation, metadata และ asset](06-ui-navigation-metadata-and-assets.md)
