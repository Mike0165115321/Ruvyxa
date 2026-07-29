# การโหลดข้อมูลและ Cache

Ruvyxa มีระบบ cache ในตัวสองชั้น: **server-side cache** (`cache(key).ttl().swr().get()`)
สำหรับลดการเรียกข้อมูลซ้ำบนเซิร์ฟเวอร์ และ **client-side hook** (`useRuvyxaLoader()`)
สำหรับจัดการสถานะการโหลดข้อมูลฝั่งเบราว์เซอร์ การออกแบบนี้ครอบคลุมตั้งแต่การป้องกัน cache stampede,
LRU eviction, ไปจนถึงการป้องกัน race condition ใน React component

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

## ขั้นตอนถัดไป

- **[06-server-actions.md](./06-server-actions.md)** — Server actions สำหรับ mutation
- **[07-api-routes.md](./07-api-routes.md)** — API routes สำหรับ REST endpoints
