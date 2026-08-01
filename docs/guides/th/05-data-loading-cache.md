# การโหลดข้อมูลและ Cache

Ruvyxa มี server-side cache แบบ in-memory ที่มี TTL, stale-while-revalidate (SWR), LRU eviction และ
error isolation ส่วน `loader(handler)` เป็น server loader ที่รับ context ของ route และใช้
`cache(key)` ภายใน handler ได้

## API ที่มีอยู่จริง

```ts
import { cache, cacheStats, invalidateCache, loader } from '@ruvyxa/core/server'
```

```ts
interface LoaderContext {
  params: Record<string, string>
  request: Request
  cache: typeof cache
}

type LoaderHandler<TResult> = (ctx: LoaderContext) => TResult | Promise<TResult>

interface Loader<TResult> {
  (ctx?: Partial<LoaderContext>): Promise<TResult>
  ruvyxa: { kind: 'loader' }
}

function loader<TResult>(handler: LoaderHandler<TResult>): Loader<TResult>
function cache(key: string): CacheBuilder
function invalidateCache(keyOrPrefix?: string): void
function cacheStats(): { size: number; maxEntries: number }
```

`ruvyxa/server` เป็น subpath ของ package `ruvyxa` ที่ re-export server API เดียวกัน หากใช้ package
ระดับ core ให้ import จาก `@ruvyxa/core/server`

## Server Loader

`loader(handler)` รับ context object ไม่ใช่ builder ที่เรียก `.key().ttl().get()`:

```tsx
import { loader } from 'ruvyxa/server'

export const getProducts = loader(async ({ params, cache }) => {
  const category = params.category ?? 'all'
  return cache(`products:${category}`)
    .ttl('5m')
    .swr('1m')
    .get(async () => {
      const response = await fetch('https://api.example.com/products')
      if (!response.ok) throw new Error(`API failed: ${response.status}`)
      return response.json()
    })
})
```

เรียก loader ได้ด้วย context บางส่วน เช่น `getProducts({ params: { category: 'books' } })` โดย
`params`, `request` และ `cache` ที่ไม่ส่งมาจะได้รับค่า default จาก runtime

ถ้า key ขึ้นกับ argument ของฟังก์ชัน ให้สร้าง key อย่างชัดเจนด้วย `cache(key)`:

```ts
import { cache } from 'ruvyxa/server'

export const getUser = (userId: string) =>
  cache(`user:profile:${userId}`)
    .ttl('5m')
    .swr('1m')
    .get(() => fetchUser(userId))
```

## Cache Builder

```ts
interface CacheBuilder {
  ttl(value: string): CacheBuilder
  swr(value: string): CacheBuilder
  get<T>(producer: () => T | Promise<T>): Promise<T>
}
```

ตัวอย่าง:

```ts
const users = await cache('users:all')
  .ttl('30s')
  .swr('10s')
  .get(() => db.users.findMany())
```

TTL รองรับรูปแบบ `ms`, `s`, `m`, `h` และ `d` เช่น `500ms`, `30s`, `5m`, `1h`, `1d` ค่าเริ่มต้น คือ
60 วินาที และค่าที่ไม่ตรงรูปแบบหรือเกิน safe integer จะทำให้เกิด `TypeError`

SWR ทำงานโดยให้ค่า stale กลับระหว่างที่ producer refresh แบบ background เมื่อหมดทั้ง TTL และ SWR
แล้ว request ถัดไปจะเรียก producer ใหม่ หาก producer ล้มเหลวระหว่างที่มี stale value ระบบสามารถคืน
stale value ได้ ส่วน cache miss ที่ไม่มีค่าเดิมจะส่ง error กลับ

## Cache Key และ Invalidation

เลือก key ให้รวมทุก input ที่มีผลต่อผลลัพธ์ โดยเฉพาะ user หรือ tenant dimension:

```ts
cache('products:featured')
cache(`tenant:${tenantId}:settings`)
cache(`user:${userId}:profile`)
```

`invalidateCache(keyOrPrefix)` จะลบ key ที่ตรงกันหรือมี prefix ตามรูปแบบ `key:` และการไม่ส่ง
argument จะลบ cache ทั้งหมด:

```ts
import { invalidateCache } from 'ruvyxa/server'

invalidateCache('products:featured')
invalidateCache('products') // products และ products:*
invalidateCache() // ลบทั้งหมด
```

ใน server action ใช้ `invalidate` จาก context ได้:

```ts
import { action } from 'ruvyxa/server'

export const createProduct = action
  .input({
    parse(value: unknown) {
      if (!value || typeof value !== 'object') throw new Error('invalid input')
      return value as { name: string }
    },
  })
  .handler(async ({ input, invalidate }) => {
    const product = await db.products.create({ data: { name: input.name } })
    invalidate('products')
    return product
  })
```

## ขอบเขตของ Cache

cache store เป็น in-memory singleton ต่อ process/worker และมีเพดาน 1,024 entries พร้อม LRU eviction
ค่าเหล่านี้เป็น implementation detail ของ core ไม่ใช่ distributed cache:

- worker แต่ละตัวมี cache ของตัวเอง
- การเขียนบน worker หนึ่งไม่ทำให้ worker อื่นเห็นค่าใหม่ทันที
- อย่า cache ข้อมูล user-specific ด้วย key ที่ไม่มี user/tenant dimension
- ไม่มี `RedisCacheProvider`, `revalidateTag` หรือ `revalidatePath` ใน package API ปัจจุบัน

ถ้าต้องการ external cache ให้เขียน integration ของ application เองที่ boundary ของ data access และ
กำหนด serialization, TTL, invalidation และ failure behavior ให้ชัดเจน เอกสารนี้ไม่อ้างว่ามี adapter
Redis/Memcached ของ Ruvyxa ในตัว

ตรวจสถิติ local cache ได้ด้วย:

```ts
import { cacheStats } from 'ruvyxa/server'

const stats = cacheStats()
console.log(stats.size, stats.maxEntries)
```

## Client Loader Hook

`useRuvyxaLoader` อยู่ใน `@ruvyxa/react` และใช้โหลดข้อมูลจาก browser; มันไม่ใช่ server cache
provider:

```tsx
'use client'

import { useRuvyxaLoader } from '@ruvyxa/react'

export function ProductList() {
  const { data, loading, error, refetch } = useRuvyxaLoader(
    async () => {
      const response = await fetch('/api/products')
      if (!response.ok) throw new Error(`request failed: ${response.status}`)
      return response.json() as Promise<Array<{ id: string; name: string }>>
    },
    { deps: [] },
  )

  if (loading) return <p>กำลังโหลด...</p>
  if (error) return <p>{error.message}</p>
  return <button onClick={refetch}>{data?.length ?? 0} รายการ</button>
}
```

`enabled` ใช้ปิดการ fetch ชั่วคราว และ `deps` ใช้กำหนดค่าที่ทำให้ hook refetch เมื่อเปลี่ยน อย่าใส่
object ใหม่ทุก render ใน `deps` หากไม่ต้องการ refetch ทุกครั้ง

## พฤติกรรมที่ควรตรวจสอบก่อนใช้งานจริง

- เลือก TTL/SWR ตามความสดของข้อมูล ไม่ใช่ตามตัวเลข performance ที่คาดเดา
- ทดสอบ producer failure ทั้งกรณีมีและไม่มี stale value
- ทดสอบหลาย worker หากระบบต้องการ cache coherence ข้าม process
- ห้ามใส่ secret หรือข้อมูลส่วนตัวลงใน key ที่แสดงใน log หรือ diagnostics
- ใช้ `ruvyxa trace` และ `ruvyxa analyze` ตรวจ flow รอบ cache; ไม่มี cache-inspection CLI เฉพาะ

## สรุป

1. ใช้ `loader(handler)` สำหรับ server loader ที่รับ context
2. ใช้ `cache(key).ttl().swr().get()` สำหรับ cache producer
3. ใช้ `invalidateCache()` หรือ `invalidate()` ใน action สำหรับ invalidation
4. Cache เป็น in-memory ต่อ worker และไม่ใช่ distributed Redis/Memcached provider
5. วัดผลบน workload จริงด้วย `ruvyxa bench` แทนการใช้ค่า latency สำเร็จรูปในเอกสาร

---

## Production contract and retained detail

The section above is the current, source-backed contract for this release. The original long-form
draft is retained below to preserve instructional context and audit history. It is non-normative: do
not copy its API snippets or capability claims unless they are revalidated against the current
source and package export map. This boundary is intentional so the document can retain its original
depth without presenting unsupported historical design as production behavior.

### Thai cache draft — historical draft (non-normative)

> **คำเตือน archive:** เนื้อหาด้านล่างเก็บไว้เพื่อประวัติเท่านั้น ไม่ใช่ cache API ปัจจุบัน
> ตัวอย่างอาจเก่าหรือไม่รองรับ และห้ามนำไปใช้เป็น code จริง production contract
> ด้านบนเป็นแหล่งอ้างอิงหลัก

# การโหลดข้อมูลและ Cache

Ruvyxa มีระบบ cache ในตัวสองชั้น: **server-side cache** (`cache(key).ttl().swr().get()`)
สำหรับลดการเรียกข้อมูลซ้ำบนเซิร์ฟเวอร์ และ **client-side hook** (`useRuvyxaLoader()`)
สำหรับจัดการสถานะการโหลดข้อมูลฝั่งเบราว์เซอร์ การออกแบบนี้ครอบคลุมตั้งแต่การป้องกัน cache stampede,
LRU eviction, ไปจนถึงการป้องกัน race condition ใน React component

---

## Type Definitions ที่มีอยู่จริง

```ts
// @ruvyxa/core/server
export function loader<TResult>(handler: LoaderHandler<TResult>): Loader<TResult>
export function cache(key: string): CacheBuilder
export function invalidateCache(keyOrPrefix?: string): void
export function cacheStats(): { size: number; maxEntries: number }

interface CacheBuilder {
  ttl(value: string): CacheBuilder
  swr(value: string): CacheBuilder
  get<T>(producer: () => T | Promise<T>): Promise<T>
}
```

`loader(handler)` เป็น callable loader ส่วน `cache(key)` เป็น builder แยกกัน ไม่มี chain API แบบ
`loader().key().ttl().get()` และไม่มี export `revalidateTag` หรือ `revalidatePath` ใน server entry
ปัจจุบัน

---

## Server Loaders

`loader` คือฟังก์ชันที่รันบนเซิร์ฟเวอร์เท่านั้น สามารถเข้าถึง database โดยตรง, API keys ส่วนตัว,
private environment variables (`RUVYXA_*` ที่ไม่มี prefix `RUVYXA_PUBLIC_`), และ cache system

### Type Signature

```tsx
interface LoaderContext {
  params: Record<string, string> // พารามิเตอร์จาก dynamic route segments
  request: Request // Request object ดั้งเดิม
  cache: typeof cache // ฟังก์ชัน cache builder
}

type LoaderHandler<TResult> = (ctx: LoaderContext) => TResult | Promise<TResult>

interface Loader<TResult> {
  (ctx?: Partial<LoaderContext>): Promise<TResult>
  ruvyxa: {
    kind: 'loader'
  }
}
```

### การประกาศและเรียกใช้

```tsx
import { loader, cache } from 'ruvyxa/server'

interface Product {
  id: string
  name: string
  price: number
  category: string
}

// ประกาศ loader — รับ context, คืน Promise<TResult>
export const getProducts = loader(async ({ params, request, cache }) => {
  const category = params.category ?? 'all'

  // ใช้ cache builder เพื่อลดภาระ database
  const data = await cache(`products:${category}`)
    .ttl('5m') // cache อยู่ 5 นาที
    .swr('1m') // เสิร์ฟข้อมูลเก่าไป 1 นาทีระหว่าง refresh
    .get(async () => {
      const res = await fetch('https://api.example.com/products')
      if (!res.ok) throw new Error('API failed')
      return res.json() as Promise<Product[]>
    })

  return data
})
```

### การเรียกใช้ loader จาก server component:

```tsx
// app/products/page.tsx
import { getProducts } from './products.loader'

export default async function ProductsPage() {
  const products = await getProducts()

  return (
    <main>
      <h1>สินค้าทั้งหมด</h1>
      <div className="grid">
        {products.map((p) => (
          <div key={p.id}>
            <h2>{p.name}</h2>
            <p>ราคา: {p.price} บาท</p>
          </div>
        ))}
      </div>
    </main>
  )
}
```

loader สามารถเรียกจาก server component, server action, หรือ API route ก็ได้ — ทุกการเรียกผ่าน cache
system ทำให้ลดการทำงานซ้ำโดยอัตโนมัติ

### Loader พร้อม Parameters

```tsx
// app/products/[category]/page.tsx
import { getProducts } from '../loader'
import type { PageProps } from 'ruvyxa/config'

export default async function CategoryPage({ params }: PageProps<{ category: string }>) {
  // params.category ถูกส่งต่อผ่าน LoaderContext.params
  const products = await getProducts({ params })

  return (
    <main>
      <h1>หมวด {params.category}</h1>
      {products.map((p) => (
        <div key={p.id}>
          <h3>{p.name}</h3>
          <p>{p.price} บาท</p>
        </div>
      ))}
    </main>
  )
}
```

---

## Cache API

`cache(key)` เป็น cache ในหน่วยความจำพร้อม TTL (Time-To-Live), SWR (Stale-While-Revalidate), และ LRU
(Least Recently Used) eviction รองรับสูงสุด 1024 entries โดยค่าเริ่มต้น

### การใช้งานพื้นฐาน

```tsx
import { cache } from 'ruvyxa/server'

const data = await cache('users:all')
  .ttl('30s') // cache อยู่ 30 วินาที
  .get(async () => {
    return await db.query('SELECT * FROM users')
  })
```

### CacheBuilder Methods

```tsx
interface CacheBuilder {
  /** กำหนดอายุ cache รองรับทั้ง string ('30s', '5m') และตัวเลข (ms) */
  ttl(value: string | number): CacheBuilder
  /** กำหนดช่วงเวลาที่ยอมให้เสิร์ฟข้อมูลเก่าระหว่าง refresh */
  swr(value: string | number): CacheBuilder
  /** ดึงหรือคำนวณค่า เมื่อ producer error จะคืน stale data ถ้ามี */
  get<T>(producer: () => T | Promise<T>): Promise<T>
}
```

### TTL Formats

รองรับทั้งสตริงและตัวเลข:

| รูปแบบ  | ความหมาย        | มิลลิวินาที | ตัวอย่าง                        |
| ------- | --------------- | ----------- | ------------------------------- |
| `500ms` | 500 มิลลิวินาที | 500         | `.ttl('500ms')`                 |
| `30s`   | 30 วินาที       | 30,000      | `.ttl('30s')`                   |
| `1m`    | 1 นาที          | 60,000      | `.ttl('1m')` หรือ `.ttl(60000)` |
| `5m`    | 5 นาที          | 300,000     | `.ttl('5m')`                    |
| `1h`    | 1 ชั่วโมง       | 3,600,000   | `.ttl('1h')`                    |
| `6h`    | 6 ชั่วโมง       | 21,600,000  | `.ttl('6h')`                    |
| `1d`    | 1 วัน           | 86,400,000  | `.ttl('1d')`                    |

**หมายเหตุ:** ค่าเริ่มต้นของ TTL คือ 60 วินาที ถ้าไม่เรียก `.ttl()` เลย

### TTL Validation

ฟังก์ชัน `parseTtl()` ตรวจสอบความถูกต้องของสตริงด้วย regex `^(\d+)\s*(ms|s|m|h|d)$` —
ถ้ารูปแบบไม่ตรง จะ throw `TypeError` พร้อมข้อความ "Invalid cache duration" ค่าต้องเป็น safe integer
และมากกว่า 0

### SWR (Stale-While-Revalidate) Algorithm

SWR เป็นกลไกที่ให้ cache service ข้อมูลเก่าได้ในขณะที่รีเฟรชข้อมูลใหม่ในเบื้องหลัง
โดยไม่ต้องให้ผู้ใช้รอ การทำงานภายใน `CacheStore.get()` เป็นดังนี้:

```
                    ┌─ request ──┐
                    │            │
                    ▼            │
              ┌──────────┐      │
              │  มี cache? │──No─┘
              └─────┬────┘
                    │ Yes
                    ▼
          ┌──────────────────┐
          │ expiresAt > now?  │──Yes──► return cached (fresh hit)
          └────────┬─────────┘
                   │ No (expired)
                   ▼
          ┌──────────────────┐
          │ staleUntil > now? │──No──► ไป producer (miss)
          └────────┬─────────┘
                   │ Yes (stale)
                   ▼
          ┌──────────────────────┐
          │ refreshing = true?   │──Yes──► return stale (รอ refresh)
          └──────────┬───────────┘
                     │ No
                     ▼
          ┌──────────────────────┐
          │ set refreshing=true  │
          │ beginWrite(key)      │
          │ fire-and-forget:     │
          │   producer()         │
          │   .then(commitWrite) │
          │   .catch(unlock)     │
          │ return stale value   │
          └──────────────────────┘
```

**กุญแจสำคัญ:**

- ผู้เรียกคนแรกที่เจอ stale value จะเป็นคนเริ่ม refresh ในเบื้องหลัง (`fire-and-forget`)
- ผู้เรียกคนอื่นๆ ที่มาในช่วง refresh กำลังทำงาน จะเห็น `refreshing = true` และได้รับ stale value
  เช่นกันโดยไม่ต้องเริ่ม refresh ซ้ำ
- ถ้า producer ล้มเหลวระหว่าง refresh, `refreshing` จะถูก reset เป็น `false`
  ทำให้ผู้เรียกครั้งถัดไปสามารถลอง refresh อีกครั้ง
- ใช้ `writeToken` (Symbol) เพื่อป้องกัน race condition ในการ commit write

**Error Isolation:** ถ้า producer throw error ในขณะที่ cache มี stale data อยู่ ระบบจะคืน stale data
แทนการ propagate error ถ้าไม่มี stale data เลย (cache miss จริงๆ) error จะถูก throw ไปยังผู้เรียก

```tsx
// ตัวอย่าง: SWR สำหรับ dashboard stats
const data = await cache('dashboard:stats')
  .ttl('1m') // cache อยู่ 1 นาที
  .swr('5m') // เสิร์ฟข้อมูลเก่าไปอีก 5 นาทีระหว่าง refresh
  .get(async () => {
    return await computeExpensiveStats()
  })

// timeline:
// T+0m    → fresh, expiresAt=T+1m, staleUntil=T+6m
// T+1m    → expired, แต่ staleUntil=T+6m → เสิร์ฟเก่า, refresh เบื้องหลัง
// T+1–6m  → stale (อาจมี refresh เกิดขึ้น 0 ครั้งหรือหลายครั้ง)
// T+6m+   → expired อย่างสมบูรณ์ → ต้องรอ producer
```

### Cache Key Conventions

ใช้รูปแบบ prefix-based: `domain:entity:identifier`

```
users:profile:123
products:featured
blog:posts:recent
products:category:electronics
session:token:abc123
```

ข้อดีของ prefix-based key คือการ invalidate เป็นกลุ่ม:

```tsx
invalidateCache('products') // ลบ key 'products' และทุก key ที่ขึ้นต้นด้วย 'products:'
invalidateCache('products:featured') // ลบ key เดียว
invalidateCache() // ลบ cache ทั้งหมดทุก key
```

### Internal Cache Entry Structure

```tsx
interface CacheEntry {
  value: unknown // ค่าที่ cache ไว้
  expiresAt: number // timestamp (ms) ที่ cache หมดอายุ (TTL)
  staleUntil: number // timestamp (ms) ที่ cache หมดสภาพ stale สิ้นสุด
  refreshing: boolean // flag บอกว่ากำลัง refresh อยู่หรือไม่
}
```

### LRU Eviction

CacheStore รักษาจำนวน entries ไม่เกิน `CACHE_MAX_ENTRIES = 1024` เมื่อจำนวน entries
ถึงขีดจำกัดและมีการเพิ่ม key ใหม่, entry ที่ถูกใช้นานที่สุด (`#evictOldest()`) จะถูกลบออก

การ update key ที่มีอยู่แล้วไม่ทำให้เกิด eviction — eviction จะเกิดขึ้นเฉพาะเมื่อเพิ่ม key ใหม่ใน
cache ที่เต็มแล้ว

### Periodic Cleanup

ทุก 60 วินาที `cacheStore.prune()` จะถูกรันเพื่อลบ entries ที่ `staleUntil` ผ่านไปแล้ว (fully
expired) ช่วยคืนหน่วยความจำ Timer นี้ใช้ `.unref()` เพื่อไม่ให้ขัดขวางการปิด process

### cacheStats API

```tsx
import { cacheStats } from 'ruvyxa/server'

const stats = cacheStats()
console.log(stats.size) // จำนวน entries ปัจจุบัน
console.log(stats.maxEntries) // ขีดจำกัดสูงสุด (1024)
```

---

## Cache Invalidation

### invalidateCache(key?)

ฟังก์ชัน global สำหรับลบ cache entries จากที่ใดก็ได้:

```tsx
import { invalidateCache } from 'ruvyxa/server'

// ลบ key เดียว
invalidateCache('products:featured')

// ลบทุก key ที่ขึ้นต้นด้วย products: (และ key 'products' เอง)
invalidateCache('products')

// ลบ cache ทั้งหมด
invalidateCache()
```

**เงื่อนไขการ match:** key จะถูกลบถ้า `key === keyOrPrefix` หรือ `key.startsWith(keyOrPrefix + ':')`

### invalidate() ใน Action Context

เมื่อทำงานใน action, ฟังก์ชัน `invalidate` จะถูกส่งผ่าน context:

```tsx
import { action } from 'ruvyxa/server'

export const createProduct = action
  .input({
    parse(value: unknown) {
      if (!value || typeof value !== 'object') throw new Error('invalid input')
      const { name, category } = value as Record<string, unknown>
      if (!name || typeof name !== 'string') throw new Error('name is required')
      return {
        name: name.trim(),
        category: String(category ?? 'general').trim(),
      }
    },
  })
  .handler(async ({ input, invalidate }) => {
    const product = await db.insert('products', {
      name: input.name,
      category: input.category,
    })

    // ลบ cache ที่เกี่ยวข้องทันทีหลัง mutation
    invalidate('products') // ลบ products ทั้งหมด
    invalidate(`products:${product.id}`) // ลบ product ตัวนี้โดยเฉพาะ
    invalidate('categories') // ลบ categories cache

    return { success: true, product }
  })
```

---

## Client-Side: useRuvyxaLoader

`useRuvyxaLoader` เป็น React hook สำหรับโหลดข้อมูลจากฝั่ง client จัดการสถานะ loading, error, data,
refetch พร้อมระบบป้องกัน race condition และ unmount safety

### Type Signature

```tsx
interface UseLoaderOptions {
  /** ถ้า false จะไม่ fetch จนกว่าจะเป็น true ค่าเริ่มต้น: true */
  enabled?: boolean
  /** dependencies ที่เมื่อเปลี่ยนแล้วจะ refetch อัตโนมัติ */
  deps?: unknown[]
}

interface UseLoaderResult<T> {
  /** ข้อมูลที่โหลดมา หรือ undefined ถ้ายังโหลดหรือมี error */
  data: T | undefined
  /** กำลังโหลดอยู่หรือไม่ */
  loading: boolean
  /** error ที่เกิดขึ้น ถ้ามี */
  error: Error | undefined
  /** เรียกใช้เพื่อ refetch ด้วยตัวเอง */
  refetch: () => void
}

function useRuvyxaLoader<T>(
  loader: () => Promise<T>,
  options?: UseLoaderOptions,
): UseLoaderResult<T>
```

### ตัวอย่างพื้นฐาน

```tsx
'use client'

import { useRuvyxaLoader } from '@ruvyxa/react'

interface Product {
  id: string
  name: string
  price: number
}

export default function ProductList() {
  const { data, loading, error, refetch } = useRuvyxaLoader<Product[]>(
    async () => {
      const res = await fetch('/api/products')
      if (!res.ok) throw new Error(`error: ${res.status}`)
      return res.json()
    },
    { deps: [] }, // deps = [] → โหลดครั้งเดียวตอน mount
  )

  if (loading) return <p>กำลังโหลด...</p>
  if (error) return <p>Error: {error.message}</p>

  return (
    <div>
      <button onClick={refetch}>โหลดใหม่</button>
      {data?.map((p) => (
        <div key={p.id}>
          {p.name} — {p.price} บาท
        </div>
      ))}
    </div>
  )
}
```

### Behavior ของ deps

`deps` เป็น array ที่ควบคุมว่าเมื่อไหร่ควร refetch:

| deps            | พฤติกรรม                                                  |
| --------------- | --------------------------------------------------------- |
| `[]`            | fetch ครั้งเดียวตอน mount ไม่ refetch อีก                 |
| `[someVar]`     | fetch ตอน mount และ refetch ทุกครั้งที่ `someVar` เปลี่ยน |
| `[query, page]` | refetch เมื่อ query หรือ page เปลี่ยน                     |

**วิธีการเปรียบเทียบ:** ใช้ `Object.is()` เปรียบเทียบสมาชิกแต่ละตัว ถ้าตัวใดตัวหนึ่งไม่เท่ากัน
(`!Object.is(old, new)`) จะถือว่า deps เปลี่ยน

**ข้อควรระวัง:** ถ้าสร้าง object/array ใหม่ทุก render ใน deps (เช่น `deps={[{ id: userId }]}`)
จะทำให้ refetch ทุก render เนื่องจาก object ใหม่ไม่มีทาง `Object.is` เท่ากับ object เก่า

### enabled (Conditional Fetch)

```tsx
'use client'

import { useState } from 'react'
import { useRuvyxaLoader } from '@ruvyxa/react'

export default function UserProfile({ userId }: { userId?: string }) {
  const { data, loading } = useRuvyxaLoader(
    async () => {
      const res = await fetch(`/api/users/${userId}`)
      return res.json()
    },
    {
      deps: [userId],
      enabled: !!userId, // ไม่ fetch ถ้า userId เป็น undefined/falsy
    },
  )

  if (!userId) return <p>กรุณาเลือกผู้ใช้</p>
  if (loading) return <p>กำลังโหลด...</p>
  return <div>{data?.name}</div>
}
```

เมื่อ `enabled` เปลี่ยนจาก `false` เป็น `true` — จะเริ่ม fetch ทันที

เมื่อ `enabled` เปลี่ยนจาก `true` เป็น `false` — จะ cancel request ที่กำลังทำอยู่ (request ID จะถูก
increment)

### Under the Hood: Race Condition Protection

```tsx
// ภายใน useRuvyxaLoader
const requestIdRef = useRef(0) // request ID ป้องกัน stale closure
const mountedRef = useRef(true) // unmount safety
const loaderRef = useRef(loader) // อ้างอิง loader function ล่าสุด

// ทุกครั้งที่ execute:
const currentId = ++requestIdRef.current // increment request ID

// ผลลัพธ์จะถูกนำไปใช้เฉพาะเมื่อ:
if (mountedRef.current && currentId === requestIdRef.current) {
  setData(result) // ปลอดภัย: component ยัง mount และเป็น request ล่าสุด
}
```

**Unmount Safety:** เมื่อ component unmount, `mountedRef.current` จะเป็น `false` ทำให้ callback
ที่ค้างอยู่ไม่ update state (ป้องกัน React warning "Can't perform a React state update on an
unmounted component")

**Stale Closure Protection:** ถ้ามีการเรียก `execute` ซ้ำก่อนที่ request ก่อนหน้าจะเสร็จ, request ID
จะ increment ทำให้ผลลัพธ์ของ request เก่าถูก ignore

**Inline Loader Handling:** loader function ถูกเก็บใน `loaderRef` ซึ่ง update ทุก render แต่ไม่
trigger refetch — การ refetch ขึ้นกับ `deps` เท่านั้น

### Request Deduplication

useRuvyxaLoader ไม่มีการ deduplicate request โดยอัตโนมัติ — ถ้าต้องการ deduplication ควรใช้
server-side cache ร่วมด้วย:

```tsx
// server loader + client hook = deduplication อัตโนมัติ
export const getUser = loader(async ({ params, cache }) => {
  return cache(`user:${params.id}`)
    .ttl('30s')
    .get(async () => {
      return db.findUser(params.id)
    })
})

// client: fetch ผ่าน server loader
const { data } = useRuvyxaLoader(
  async () => {
    const res = await fetch(`/api/user/${id}`)
    return res.json()
  },
  { deps: [id] },
)
```

---

## Full Example: ระบบแสดงสินค้า

### Server Loader

```tsx
// app/products/loader.ts
import { loader, cache } from 'ruvyxa/server'

interface Product {
  id: string
  name: string
  price: number
  category: string
  inStock: boolean
}

export const getProducts = loader(async ({ params }) => {
  const category = params.category ?? 'all'
  const cacheKey = category === 'all' ? 'products:all' : `products:category:${category}`

  return cache(cacheKey)
    .ttl('5m')
    .swr('1m')
    .get(async () => {
      const res = await fetch(`https://api.example.com/products?cat=${category}`)
      if (!res.ok) throw new Error('API failed')
      const data = (await res.json()) as Product[]

      // cache ว่าง → throw เพื่อให้ cache คืน stale data ถ้ามี
      if (data.length === 0) throw new Error('No products found')

      return data
    })
})

export const getCategories = loader(async () => {
  return cache('categories')
    .ttl('1h')
    .get(async () => {
      const res = await fetch('https://api.example.com/categories')
      return res.json() as Promise<string[]>
    })
})
```

### Server Page

```tsx
// app/products/page.tsx
import { getCategories } from './loader'

export default async function CategoriesPage() {
  const categories = await getCategories()

  return (
    <main>
      <h1>หมวดหมู่สินค้า</h1>
      <ul>
        {categories.map((cat) => (
          <li key={cat}>
            <a href={`/products/${cat}`}>{cat}</a>
          </li>
        ))}
      </ul>
    </main>
  )
}
```

```tsx
// app/products/[category]/page.tsx
import { getProducts } from '../loader'
import type { PageProps } from 'ruvyxa/config'

export default async function CategoryPage({ params }: PageProps<{ category: string }>) {
  const products = await getProducts({ params })

  return (
    <main>
      <h1>หมวด {params.category}</h1>
      <div className="grid">
        {products?.map((p) => (
          <div key={p.id} className="card">
            <h3>{p.name}</h3>
            <p>{p.price} บาท</p>
            <span>{p.inStock ? 'ในสต็อก' : 'หมด'}</span>
          </div>
        )) ?? <p>ไม่มีสินค้าในหมวดนี้</p>}
      </div>
    </main>
  )
}
```

---

## Under the Hood: CacheStore Architecture

```
┌────────────────────────────────────────────────────────┐
│                    CacheStore                           │
│                                                        │
│  entries: Map<string, CacheEntry>   (ข้อมูล cache)      │
│  accessOrder: string[]             (LRU ordering)      │
│  pendingWrites: Map<string, Set<symbol>>  (write locks) │
│  maxEntries: 1024                   (ขีดจำกัด)          │
│                                                        │
│  Methods:                                              │
│    get(key)           → CacheEntry | undefined          │
│    peek(key)          → CacheEntry (ไม่ update LRU)     │
│    set(key, entry)    → evict LRU ถ้าเต็ม               │
│    delete(key)        → ลบ entry                        │
│    clear()            → ลบทั้งหมด                        │
│    invalidate(prefix) → ลบแบบ prefix                    │
│    beginWrite(key)    → ขอ write token                  │
│    commitWrite(...)   → commit แบบ optimistic           │
│    finishWrite(key)   → ปล่อย write lock                │
│    prune()            → ลบ fully expired entries        │
└────────────────────────────────────────────────────────┘
```

**LRU Eviction Algorithm:**

- `accessOrder` เป็น array ที่เก็บ key เรียงตามลำดับการใช้งานล่าสุด
- ทุกครั้งที่มีการ get/set, key จะถูกย้ายไปท้าย array (`#touchAccessOrder`)
- เมื่อต้อง evict, entry ตัวแรกใน `accessOrder` (ถูกใช้นานที่สุด) จะถูกลบ (`#evictOldest`)

**Write Token System:**

- ป้องกัน race condition ระหว่าง concurrent writes
- `beginWrite(key)` → ได้ Symbol token
- `commitWrite(key, token, entry, expectedEntry?)` → จะ commit ก็ต่อเมื่อ token ยัง valid และ
  expectedEntry ตรง
- `finishWrite(key, token)` → ปล่อย lock ให้ write อื่นทำต่อ

---

## Under the Hood: parseTtl Algorithm

```tsx
function parseTtl(value: string): number {
  // regex: ^(\d+)\s*(ms|s|m|h|d)$
  const match = value.match(/^(\d+)\s*(ms|s|m|h|d)$/)
  if (!match) throw invalidCacheDuration(value)

  const amount = Number(match[1])
  if (!Number.isSafeInteger(amount) || amount <= 0) throw invalidCacheDuration(value)

  const multiplier = match[2] === 'ms' ? 1
                   : match[2] === 's'  ? 1000
                   : match[2] === 'm'  ? 60_000
                   : match[2] === 'h'  ? 3_600_000
                   : match[2] === 'd'  ? 86_400_000

  const duration = amount * multiplier
  if (!Number.isSafeInteger(duration)) throw invalidCacheDuration(value)

  return duration   // คืนค่าเป็นมิลลิวินาที
}
```

---

## Error Codes

| รหัสข้อผิดพลาด | สถานะใน source ปัจจุบัน                                                       | วิธีตรวจสอบ                                |
| -------------- | ----------------------------------------------------------------------------- | ------------------------------------------ |
| ไม่มีรหัสเฉพาะ | cache API ปัจจุบันไม่ได้กำหนด public code สำหรับ TTL format                   | ตรวจ option และผลลัพธ์จาก cache API โดยตรง |
| ไม่มีรหัสเฉพาะ | `revalidateTag` และ circular dependency ไม่ใช่ code ที่ยืนยันจาก cache source | ตรวจ stack trace และ source ของ loader     |

---

## ข้อผิดพลาดทั่วไป (Troubleshooting)

| ปัญหา                           | สาเหตุ                              | วิธีแก้                                                            |
| ------------------------------- | ----------------------------------- | ------------------------------------------------------------------ |
| Cache ไม่ refresh               | TTL ยังไม่หมด หรือไม่ได้ invalidate | เรียก `invalidateCache(key)` หรือรอให้ TTL หมด                     |
| ข้อมูลเก่าเกินไป                | SWR window ยาวเกินไป                | ลดค่า `.swr()` หรือเพิ่ม frequency ของ background job              |
| Cache usage สูงมาก              | entries เกิน 1024 ถูก LRU evict     | ใช้ cacheStats() ตรวจสอบ, เพิ่ม maxEntries หรือ reduce cache scope |
| `useRuvyxaLoader` ไม่ fetch     | `enabled` เป็น `false`              | set `enabled: true` หรือลบ option                                  |
| refetch ไม่ทำงาน                | deps ไม่เปลี่ยน                     | ตรวจสอบว่าค่าใน deps เปลี่ยนจริงด้วย Object.is                     |
| server loader error             | producer throw error (API/DB ล่ม)   | เพิ่ม `.swr()` เพื่อให้มี stale data เป็น fallback                 |
| Race condition ข้อมูลสลับ       | deps เปลี่ยนเร็วเกินไป              | ใช้ request ID protection (built-in)                               |
| Component unmount แล้ว setState | unmount โดยไม่ cleanup              | useRuvyxaLoader มี unmount safety ในตัว                            |
| cache key collision             | key ซ้ำกัน                          | ใช้ prefix-based naming convention                                 |
| `Invalid cache duration` error  | TTL string format ไม่ถูก            | ใช้รูปแบบ `30s`, `5m`, `1h`, `1d` เท่านั้น                         |
| Memory leak                     | สร้าง cache keys ไม่จำกัด           | prune() ทำงานทุก 60s, LRU evict ที่ 1024 entries                   |

### RUV Error Codes ที่เกี่ยวข้อง

| Code    | ความหมาย                         | สาเหตุ                                          |
| ------- | -------------------------------- | ----------------------------------------------- |
| RUV1200 | API route execution failed       | API route handler throw error                   |
| RUV1300 | Client hydration bundling failed | Component มี imports ไม่ compatible กับ browser |
| RUV1500 | Server action execution failed   | Action handler throw error                      |
| RUV1501 | Route action file not found      | ไม่มีไฟล์ `action.ts` ใน route directory        |
| RUV1600 | General config error             | ruvyxa.config.ts มีค่าผิด                       |
| RUV1601 | Config validation error          | ค่าใน config ต้องมากกว่า 0 หรือไม่เว้นว่าง      |
| RUV1602 | Config limit exceeded            | ค่าเกินขีดจำกัดสูงสุด                           |
| RUV1700 | TypeScript plugin timeout        | Plugin hook ทำงานนานเกิน timeout                |
| RUV1701 | Plugin protocol error            | Plugin ส่งข้อมูลผิดรูปแบบ                       |

---

## Best Practices

```
┌────────────────────────────────────┐
│  Server Loader  vs  useRuvyxaLoader│
│                                    │
│  Server Loader:                    │
│    ✅ อ่าน DB โดยตรง               │
│    ✅ ใช้ private env               │
│    ✅ cache server-side            │
│    ✅ ทำงานตอน render              │
│    ✅ zero client JS               │
│                                    │
│  useRuvyxaLoader:                  │
│    ✅ ทำงานหลัง mount               │
│    ✅ refetch ได้ตามต้องการ         │
│    ✅ ใช้กับ API ที่ต้องการ auth     │
│    ✅ conditional fetch (enabled)   │
│    ✅ real-time update             │
└────────────────────────────────────┘
```

### หลักการ

1. **ใช้ server loader ทุกครั้งที่ทำได้** — ลด client bundle, ได้ cache ฟรี
2. **cache ข้อมูลที่ซ้ำๆ** — products, categories, settings, config
3. **ใช้ SWR สำหรับข้อมูลสำคัญ** — ถ้า API ล่ม ยังมีข้อมูลเก่าให้ใช้
4. **prefix keys ด้วย namespace** — จะได้ invalidate เป็นกลุ่ม
5. **ไม่ cache ข้อมูลเฉพาะ user** — profile, cart, session ใช้ per-user key เช่น `cart:user:123`
6. **ตั้ง deps ให้ถูกต้อง** — ถ้าไม่เปลี่ยน, ใช้ `[]`, ถ้าต้องการ refetch, ใส่ค่าที่ monitor
7. **ใช้ enabled สำหรับ conditional fetch** — ไม่ต้องเช็คเงื่อนไขใน loader function

---

## Performance Characteristics

| กระบวนการ            | ความซับซ้อน | หมายเหตุ                                 |
| -------------------- | ----------- | ---------------------------------------- |
| `cache(fn)` lookup   | `O(1)`      | Map lookup ผ่าน memory เร็วมาก           |
| `revalidateTag`      | `O(k)`      | เมื่อ `k` = จำนวน entries ที่มี tag นั้น |
| `revalidatePath`     | `O(p)`      | เมื่อ `p` = จำนวน entries ภายใต้ path    |
| GC (Garbage Collect) | `O(n)`      | ทำงานบน background thread แยกต่างหาก     |

การทำ Caching แทบจะไม่มี overhead ในเชิงของ CPU แต่อาจใช้หน่วยความจำเพิ่มขึ้นหากมีการแคช response
ขนาดใหญ่

---

## Security Considerations

### การรั่วไหลของข้อมูลระหว่าง Tenant (Cross-Tenant Data Leaks)

ฟังก์ชัน `cache()` เก็บข้อมูลแชร์กันในระดับ **Process** หากคุณแคชข้อมูลที่ผูกกับ user-specific (เช่น
ตะกร้าสินค้า หรือ หน้าโปรไฟล์) ให้มั่นใจว่าได้รวม User ID เป็นส่วนหนึ่งของ arguments หรือ tag
เพื่อหลีกเลี่ยงการแสดงข้อมูลของ User A ให้ User B เห็น

```tsx
// ❌ ไม่ปลอดภัย: ทุกคนจะเห็นโปรไฟล์ของผู้ใช้คนแรก
const getProfile = cache(async () => db.profile.find(), ['profile'])

// ✅ ปลอดภัย: โปรไฟล์ถูกแยกตาม user ID
const getProfile = cache(async (userId: string) => db.profile.find(userId), ['profile'])
```

---

## Advanced Loader Patterns

### การดึงข้อมูลแบบคู่ขนาน (Parallel Data Fetching)

คุณสามารถเรียกใช้ loaders หลายตัวพร้อมกันโดยไม่ต้องรอตัวใดตัวหนึ่งเสร็จก่อน:

```tsx
export default async function Dashboard() {
  // เริ่มดึงข้อมูลพร้อมกัน
  const usersPromise = getUsers()
  const statsPromise = getStats()

  // รอจนกว่าจะเสร็จทั้งหมด
  const [users, stats] = await Promise.all([usersPromise, statsPromise])

  return <DashboardView users={users} stats={stats} />
}
```

---

## Multi-Tenant Caching

ในแอปพลิเคชันแบบ Multi-Tenant คุณสามารถป้องกันแคชชนกันได้โดยรวม Tenant ID ไว้ใน Cache Key และ Tags
เสมอ:

```tsx
import { cache } from '@ruvyxa/core/server'

export const getTenantSettings = cache(
  async (tenantId: string) => {
    return db.settings.findByTenant(tenantId)
  },
  ['settings'], // ❌ ไม่ดี: อาจทำให้ invalidate ของ tenant อื่น
)

export const getTenantSettings = cache(
  async (tenantId: string) => {
    return db.settings.findByTenant(tenantId)
  },
  (tenantId) => [`settings:${tenantId}`], // ✅ ปลอดภัย: tag แยกตาม tenant
)
```

---

## External Cache Integration

cache ในตัวเป็น in-memory ต่อ worker และ repository ไม่ได้ export `CacheProvider`, Redis adapter
หรือ Memcached adapter ให้ config โดยตรง หาก application ต้องใช้ external cache ให้ห่อไว้ที่
data-access boundary ของ application เอง และกำหนด serialization, TTL, invalidation, failure behavior
และ tenant key isolation ให้ชัดเจน ตัวอย่างนี้เป็น pseudocode ของ application ไม่ใช่ Ruvyxa API:

```ts
async function getFromApplicationCache<T>(key: string, producer: () => Promise<T>): Promise<T> {
  // เรียก client ของ provider ที่ application เลือกเอง
  const cached = await applicationCache.get<T>(key)
  if (cached !== undefined) return cached
  const value = await producer()
  await applicationCache.set(key, value, { ttlSeconds: 300 })
  return value
}
```

ไม่ควรสรุปว่า cache ภายนอกจะทำงานข้าม process หรือ serialize object ทุกชนิดได้จนกว่าจะตรวจ contract
ของ provider ที่ application เลือกเอง

---

## การ invalidate cache ใน server code

ใช้ `invalidateCache` จาก server entry เมื่อข้อมูลเปลี่ยนแปลง โดยเลือก exact key หรือ prefix ให้
ตรงกับ key ที่สร้างไว้:

```ts
import { invalidateCache } from 'ruvyxa/server'

invalidateCache('products:featured') // exact key
invalidateCache('tenant:acme:') // exact key หรือ key ที่ขึ้นต้นด้วย prefix นี้
invalidateCache() // clear cache ของ worker ปัจจุบัน
```

การ invalidate นี้ไม่ใช่ distributed invalidation และไม่ควรอ้างว่าเคลียร์ cache ของทุก worker
หรือทุก instance โดยอัตโนมัติ

---

## Cache Warmup Strategies

การทำให้ข้อมูลถูกแคชไว้ล่วงหน้า (Warmup) มีประโยชน์มากสำหรับเพจที่มีผู้เข้าชมบ่อย
สามารถทำได้ผ่านสคริปต์ตอนเริ่มเซิร์ฟเวอร์:

```ts
// server.ts (หรือ entrypoint ของเซิร์ฟเวอร์)
import { getTopProducts } from './app/loaders'

async function warmup() {
  console.log('Warming up cache...')
  await getTopProducts() // โหลดขึ้น Memory ทันที
}

warmup()
```

---

## Cache Debugging

Ruvyxa มีเครื่องมือในการตรวจสอบสถานะแคช (Cache Hits / Misses) ในโหมดพัฒนา:

```bash
# รันโหมดพัฒนาพร้อมดู Cache Logs
RUVYXA_DEBUG_CACHE=1 npm run dev
```

คุณจะเห็น Logs ลักษณะนี้ใน Terminal:

```
[CACHE HIT]   tags: ['products'] time: 0.1ms
[CACHE MISS]  tags: ['user:123'] time: 145ms
[INVALIDATE]  tags: ['products']
```

---

## Integration with ISR and SSG

`cache()` ทำงานร่วมกับ ISR และ SSG ได้อย่างสมบูรณ์แบบ:

- **SSG**: ข้อมูลที่ถูก `cache()` ตอน Build จะถูกแช่แข็งไว้ใน Static HTML
- **ISR**: เมื่อถึงรอบ revalidate ข้อมูลจะดึงผ่านตัว `cache()` อีกครั้ง

หากทั้งคู่มี TTL (`revalidate` ใน Page และ `ttl` ใน `cache()`):

- เพจจะถูก Rebuild ตามเวลาของ **Page `revalidate`**
- แต่ข้อมูลภายในหน้าจะถูก Re-fetch ใหม่ก็ต่อเมื่อ **`cache()` TTL** หมดอายุลงด้วย

---

## Thundering Herd Prevention

Ruvyxa แก้ปัญหา "Thundering Herd" (รุมดึงข้อมูลพร้อมกันเมื่อแคชหมดอายุ) ให้อัตโนมัติ:

- หากมีผู้ใช้ 1,000 คนเข้าหน้าเดียวกันตอนที่แคชหมด
- Ruvyxa จะส่ง Query เข้าฐานข้อมูลเพียงแค่ **ครั้งเดียว** (Promise Deduplication)
- ผู้ใช้อีก 999 คนจะรอและได้รับผลลัพธ์จาก Request เดียวกันนี้

---

## ลองทำดู

มาลองสร้าง Loader อย่างง่ายพร้อมการ Cache ดู:

**1. สร้างไฟล์ `app/products/loader.ts`**

```ts
import { cache } from '@ruvyxa/core/server'

export const getProduct = cache(
  async (id: string) => {
    console.log('--- Fetching from DB ---')
    const res = await fetch(`https://dummyjson.com/products/${id}`)
    return res.json()
  },
  ['product'],
  { ttl: '10s' },
)
```

**2. สร้างหน้าเพจ `app/products/[id]/page.tsx`**

```tsx
import { getProduct } from './loader'

export default async function Page({ params }) {
  const product = await getProduct(params.id)

  return (
    <div>
      <h1>{product.title}</h1>
      <p>{product.description}</p>
    </div>
  )
}
```

ลองรีเฟรชหน้าเบราว์เซอร์ติดกัน 5 ครั้ง คุณจะเห็น `--- Fetching from DB ---`
โผล่ในเซิร์ฟเวอร์เพียงครั้งเดียว!

---

## ขอบเขตของ Cache และพฤติกรรมเมื่อเกิดข้อผิดพลาด

`cache(key)` จาก `@ruvyxa/core/server` เป็น application-data cache ใน process เดียว แตกต่างจาก
route-render cache และ generated build cache จึงไม่ควรใช้ชื่อเดียวแล้วคาดว่าจะ invalidate กันทั้งหมด
contract ปัจจุบันมีขอบเขตชัดเจน:

- เก็บได้สูงสุด 1,024 keys; เมื่อเต็มจะ evict key ที่ใช้งานล่าสุดน้อยที่สุด
- `ttl()` รับ duration ที่เป็นบวกพร้อมหน่วย `ms`, `s`, `m`, `h` หรือ `d`
- `swr()` เพิ่มช่วง stale-while-revalidate หลัง TTL
- ถ้า producer ล้มเหลวแต่ยังมี stale value ในช่วง SWR ระบบคืนค่านั้นได้; cache ว่างจะไม่กลายเป็น
  success
- `invalidateCache()` invalidate ได้ทั้ง key เดียว, prefix ที่คั่นด้วย colon หรือทุก key เมื่อไม่ส่ง
  argument

เพราะฉะนั้นการตั้งชื่อ key จึงเกี่ยวกับความถูกต้อง ไม่ใช่แค่รูปแบบ:

```ts
import { cache, invalidateCache } from '@ruvyxa/core/server'

export const listProducts = () =>
  cache('catalog:products')
    .ttl('5m')
    .swr('30s')
    .get(() => fetch('https://catalog.example.test/products').then((r) => r.json()))

export function refreshCatalog() {
  invalidateCache('catalog') // invalidate catalog และ keys ที่ขึ้นต้น catalog:
}
```

เลือก key จากทุก input ที่ทำให้ผลลัพธ์เปลี่ยน ข้อมูลเฉพาะ user ต้องมี user/tenant ที่เหมาะสมใน key
มิฉะนั้น caller หนึ่งอาจได้ข้อมูลของอีก caller หนึ่ง Cache นี้อยู่ใน process เดียว ดังนั้น
deployment หลาย instances ที่ต้องการข้อมูลตรงกันต้องใช้ data/cache strategy แบบ shared นอกเหนือจาก
helper นี้

### Loader กับ Client Fetch ใช้คนละช่วงของ Flow

`loader(handler)` ห่อ server-side handler และให้ context ที่มี `params`, `request` และ cache helper
เดียวกัน จึงเหมาะกับการอ่านข้อมูลตอน render page:

```ts
import { loader } from '@ruvyxa/core/server'

export const productLoader = loader(async ({ params, cache }) => {
  const id = params.id
  return cache(`catalog:product:${id}`)
    .ttl('1m')
    .get(async () => {
      const response = await fetch(`https://catalog.example.test/products/${id}`)
      if (!response.ok) throw new Error(`catalog returned ${response.status}`)
      return response.json()
    })
})
```

เก็บ initial page read ไว้บน server เมื่อใช้ private credentials หรือต้องการให้ข้อมูลอยู่ใน HTML
response ส่วน client hook หรือ browser `fetch` เหมาะกับ refresh ที่เกิดจาก interaction และต้องเรียก
endpoint ที่ เปิดเผยต่อ browser ได้อย่างปลอดภัย

### Debug Cache โดยไม่เดา CLI Flag

ไม่มี CLI flag สำหรับ inspect cache โดยตรง ให้ทำให้ behavior เห็นได้ที่ producer boundary แล้วตรวจ
route รอบ ๆ ตามปกติ:

```bash
ruvyxa trace /products/[id]
ruvyxa analyze --format human
```

log เฉพาะ metadata ของ key ที่ไม่ sensitive ใน development Cache key อาจมี account identifier
หรือข้อมูล routing ที่อ่อนไหว จึงไม่ควรใส่ secrets ใน key หรือใน diagnostics ที่ browser เห็นได้

---

## ขั้นตอนถัดไป

- **[06-server-actions.md](./06-server-actions.md)** — Server actions สำหรับ mutation
- **[07-api-routes.md](./07-api-routes.md)** — API routes สำหรับ REST endpoints
