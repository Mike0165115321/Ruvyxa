# Data Loading and Caching

Ruvyxa provides a built-in caching layer for server-side data. Use it to reduce database load, speed
up responses, and keep data fresh. The cache is an in-memory, LRU-bounded store with
stale-while-revalidate semantics and error isolation.

```
  Request
     │
     ▼
  ┌─────────────┐   cache hit   ┌──────────┐
  │   Cache     │ ─────────────→ │  Serve   │
  │   Check     │               │  Cached  │
  │             │               │  Data    │
  └──────┬──────┘               └──────────┘
         │ cache miss
         ▼
  ┌─────────────┐   fresh data  ┌──────────┐
  │   Your      │ ─────────────→ │  Store   │
  │   Fetcher   │               │  in      │
  │   (fn)      │               │  Cache   │
  └─────────────┘               └──────────┘
```

---

## Type Definitions

All exports from `ruvyxa/server`:

```ts
// file: packages/@ruvyxa/core/src/server.ts

export interface LoaderContext {
  params: Record<string, string>
  request: Request
  cache: typeof cache
}

export type LoaderHandler<TResult> = (ctx: LoaderContext) => TResult | Promise<TResult>

export interface Loader<TResult> {
  (ctx?: Partial<LoaderContext>): Promise<TResult>
  ruvyxa: {
    kind: 'loader'
  }
}

export function loader<TResult>(handler: LoaderHandler<TResult>): Loader<TResult>
```

### CacheBuilder Interface

```ts
export interface CacheBuilder {
  ttl(value: string): CacheBuilder
  swr(value: string): CacheBuilder
  get<T>(producer: () => T | Promise<T>): Promise<T>
}

export interface CacheEntry {
  value: unknown
  expiresAt: number // Date.now() + ttl
  staleUntil: number // Date.now() + ttl + swr
  refreshing: boolean // background refresh in progress
}
```

### Invalidation

```ts
export function cache(key: string): CacheBuilder
export function invalidateCache(keyOrPrefix?: string): void
export function cacheStats(): { size: number; maxEntries: number }
```

### Client-Side

```ts
// file: packages/@ruvyxa/react/src/use-loader.ts

export interface UseLoaderOptions {
  enabled?: boolean // default: true
  deps?: unknown[] // default: []
}

export interface UseLoaderResult<T> {
  data: T | undefined
  loading: boolean
  error: Error | undefined
  refetch: () => void
}

export function useRuvyxaLoader<T>(
  loader: () => Promise<T>,
  options?: UseLoaderOptions,
): UseLoaderResult<T>
```

---

## Server Loaders

Loaders are async functions that fetch data on the server, wrapped with caching.

```tsx
import { loader } from 'ruvyxa/server'

const getUser = loader()
  .key('user:profile')
  .ttl('5m')
  .swr('1m')
  .get(async (userId: string) => {
    const user = await db.query('SELECT * FROM users WHERE id = ?', [userId])
    return user
  })
```

Then use it in any server component:

```tsx
// app/profile/[id]/page.tsx
import { getUser } from './loaders'

export default async function ProfilePage({ params }: { params: { id: string } }) {
  const user = await getUser(params.id)

  return (
    <div>
      <h1>{user.name}</h1>
      <p>Email: {user.email}</p>
    </div>
  )
}
```

### Loader Accepted Formats

Loaders accept both a handler argument at construction time and arguments at call time:

| Pattern                                   | Cache Key  | Description                    |
| ----------------------------------------- | ---------- | ------------------------------ |
| `loader().get(async (id: string) => ...)` | `key:arg1` | Arguments appended to base key |
| `loader().get(async () => ...)`           | `key`      | No arguments, single value     |
| `loader(ctx => ...)` (direct)             | N/A        | No caching, bare execution     |

---

## Cache API

### Basic Syntax

```ts
loader()
  .key('resource:scope') // Unique cache key
  .ttl('5m') // How long until stale
  .swr('1m') // Stale-while-revalidate window
  .get(async (...args) => {
    // Your fetch logic here
    return data
  })
```

### TTL (Time-To-Live)

Controls how long the cache entry is considered **fresh**. Uses `parseTtl()` internally.

```ts
function parseTtl(value: string): number {
  // regex: /^(\d+)\s*(ms|s|m|h|d)$/
  // returns milliseconds
}
```

| Format | Example        | Unit         | Milliseconds |
| ------ | -------------- | ------------ | ------------ |
| `ms`   | `ttl("500ms")` | milliseconds | 500          |
| `s`    | `ttl("30s")`   | seconds      | 30,000       |
| `m`    | `ttl("5m")`    | minutes      | 300,000      |
| `h`    | `ttl("1h")`    | hours        | 3,600,000    |
| `d`    | `ttl("1d")`    | days         | 86,400,000   |

Default TTL: **60 seconds** if not specified.

Invalid format throws:
`Invalid cache duration "X". Use a positive value within JavaScript's safe integer range, such as "30s", "5m", "1h", or "1d".`

### SWR (Stale-While-Revalidate)

Controls how long a **stale** entry is served while a background refresh happens.

```
Timeline:

  t=0    Cache entry created (fresh)
  t=5m   TTL expires (stale)
         ↓
  t=5m – t=6m   SWR window
         ├── Requests served stale entry
         └── Background refresh triggered
               │
               ▼
  t=6m   SWR expires
         Cache considered empty
         Next request fetches fresh
```

If `swr` is not set, stale entries are evicted immediately when TTL expires (swrMs = 0).

### SWR Algorithm — Exact Behavior

When `.get()` is called:

```
now = Date.now()
cached = cacheStore.get(key)

if cached AND cached.expiresAt > now:
    → FRESH HIT: return cached.value immediately (no producer call)

if cached AND cached.staleUntil > now:
    → STALE HIT: return cached.value
    → if NOT cached.refreshing:
        → set cached.refreshing = true
        → fire-and-forget background refresh:
            → await producer()
            → commitWrite(key, token, { value, expiresAt, staleUntil })
            → if commitWrite returns false (entry was replaced):
                → cached.refreshing = false (allow next refresh)
    → CONCURRENT CALLERS: all receive stale value; only first triggers refresh

if NO cached OR cached.staleUntil <= now:
    → MISS: await producer()
    → commitWrite(key, token, { value, expiresAt, staleUntil })
    → if producer throws:
        → if stale data exists → return stale value (fallback)
        → else → propagate error
```

### Cache Key Convention

Use `resource:scope` format for clarity:

| Key                        | Meaning                          |
| -------------------------- | -------------------------------- |
| `user:profile:42`          | User profile for ID 42           |
| `blog:recent`              | Recent blog posts                |
| `product:list:electronics` | Products in electronics category |
| `settings:global`          | Global app settings              |

### Cache Key Rules

- **Max length**: No hard limit (bounded by memory)
- **Allowed characters**: Any string — encoded as UTF-8 for hashing
- **Namespacing**: Keys starting with `prefix:` matched by `invalidateCache("prefix")` via prefix
  scan
- **Case sensitivity**: Exact match (`"User:42"` ≠ `"user:42"`)
- **Empty key**: Allowed but strongly discouraged; collisions guaranteed

Keys are automatically parameterized when you pass arguments to `.get()`:

```ts
const getUser = loader()
  .key('user:profile')
  .get(async (userId: string) => {
    //                           ^^ argument is appended to key
    // Cache key becomes: user:profile:42
    return await fetchUser(userId)
  })
```

### Direct `cache()` API

Use `cache()` for ad-hoc caching without the loader pattern:

```ts
import { cache } from 'ruvyxa/server'

const data = await cache('users:list')
  .ttl('5m')
  .swr('1m')
  .get(async () => {
    return await db.users.findMany()
  })
```

The `cache()` function returns a `CacheBuilder` with the same `.ttl()`, `.swr()`, and `.get()`
methods.

---

## Cache Invalidation

### From Server Code

```ts
import { invalidateCache } from 'ruvyxa/server'

// Invalidate a specific key
await invalidateCache('user:profile:42')

// Invalidate all keys matching prefix
await invalidateCache('user:profile') // matches user:profile:*, user:profile:settings:*, etc.

// Invalidate entire cache
await invalidateCache() // clears ALL entries
```

**Prefix matching algorithm**: Caches are invalidated when
`key === keyOrPrefix || key.startsWith(keyOrPrefix + ':')`. This means `invalidateCache("user")`
matches `user:profile:42` but NOT `username:42`.

### From Action Handlers

```ts
import { action } from 'ruvyxa/server'

export const updateProfile = action()
  .input({
    parse(value: unknown) {
      if (typeof value !== 'object' || value === null) throw new Error('Invalid')
      return value as { name: string; userId: string }
    },
  })
  .handler(async ({ input, invalidate }) => {
    await db.query('UPDATE users SET name = ? WHERE id = ?', [input.name, input.userId])
    invalidate('user:profile:' + input.userId)
    return { ok: true }
  })
```

The `invalidate` function in action handlers calls the same `cacheStore.invalidate()` internally.

---

## Full Loader Example

```ts
// app/lib/loaders.ts
import { loader } from 'ruvyxa/server'
import { db } from './db'

// Fetch a single post
export const getPost = loader()
  .key('blog:post')
  .ttl('5m')
  .swr('30s')
  .get(async (slug: string) => {
    console.log(`[Cache Miss] Fetching post: ${slug}`)
    const post = await db.query('SELECT * FROM posts WHERE slug = ?', [slug])
    return post
  })

// Fetch recent posts
export const getRecentPosts = loader()
  .key('blog:recent')
  .ttl('1m')
  .get(async (limit: number = 10) => {
    console.log(`[Cache Miss] Fetching recent posts`)
    const posts = await db.query('SELECT * FROM posts ORDER BY created_at DESC LIMIT ?', [limit])
    return posts
  })

// Fetch user profile
export const getUserProfile = loader()
  .key('user:profile')
  .ttl('1h')
  .swr('10m')
  .get(async (userId: string) => {
    const user = await db.query('SELECT * FROM users WHERE id = ?', [userId])
    return user
  })
```

Use in a page:

```tsx
// app/blog/[slug]/page.tsx
import { getPost, getRecentPosts } from '../lib/loaders'

export default async function BlogPost({ params }: { params: { slug: string } }) {
  const [post, recentPosts] = await Promise.all([
    getPost(params.slug), // Cached for 5m, SWR 30s
    getRecentPosts(5), // Cached for 1m
  ])

  return (
    <div>
      <h1>{post.title}</h1>
      <div>{post.content}</div>
      <aside>
        <h2>Recent Posts</h2>
        {recentPosts.map((p) => (
          <p key={p.id}>{p.title}</p>
        ))}
      </aside>
    </div>
  )
}
```

The first request fetches from the database. Subsequent requests within 5 minutes hit the cache.
Between 5 and 5.5 minutes, stale data is served while the background refreshes.

---

## Client-Side Loading

For client components that need to fetch data, use `useRuvyxaLoader` from `@ruvyxa/react`.

```tsx
'use client'

import { useRuvyxaLoader } from '@ruvyxa/react'

export function PostList() {
  const { data, loading, error, refetch } = useRuvyxaLoader(
    () => fetch('/api/posts').then((r) => r.json()),
    {
      deps: [], // Refetch when deps change
      enabled: true, // Set to false to skip initial fetch
    },
  )

  if (loading) return <p>Loading...</p>
  if (error) return <p>Error: {error.message}</p>

  return (
    <div>
      <button onClick={refetch}>Refresh</button>
      {data.map((post) => (
        <div key={post.id}>
          <h3>{post.title}</h3>
        </div>
      ))}
    </div>
  )
}
```

### Options Reference

| Option    | Type        | Default | Description                                                                                                                     |
| --------- | ----------- | ------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `enabled` | `boolean`   | `true`  | If `false`, loader does not execute. Mid-flight: increments request counter, sets loading to `false`, discards in-flight result |
| `deps`    | `unknown[]` | `[]`    | Triggers automatic refetch when any element changes (compared via `Object.is` reference equality, not deep equality)            |

### Return Value

| Field     | Type                 | Description                                                                      |
| --------- | -------------------- | -------------------------------------------------------------------------------- |
| `data`    | `T \| undefined`     | Successful response data                                                         |
| `loading` | `boolean`            | `true` during fetch, including initial                                           |
| `error`   | `Error \| undefined` | Caught error; non-`Error` throws are wrapped via `new Error(String(err))`        |
| `refetch` | `() => void`         | Manual refetch — calls `execute()` which clears error state and starts new fetch |

### Deps Comparison Algorithm

```ts
// Uses Object.is (reference equality) — NOT deep equality nor JSON serialization
if (
  depsRef.current.length !== deps.length ||
  depsRef.current.some((value, index) => !Object.is(value, deps[index]))
) {
  // deps changed → increment version → trigger useEffect → execute()
}
```

**Key behavior**: Pass stable references in `deps`. Creating a new object/array every render with
the same "contents" will appear to have changed. Use `useMemo` or primitive values.

### Edge Cases

| Scenario                                 | Behavior                                                                                      |
| ---------------------------------------- | --------------------------------------------------------------------------------------------- |
| **Unmount during fetch**                 | `mountedRef.current = false` in cleanup; result discarded via `if (mountedRef.current)` guard |
| **Deps change during in-flight request** | Stale request ID (requestIdRef.current !== currentId); stale result discarded                 |
| **Disabled mid-flight**                  | `requestIdRef.current++` retires in-flight; sets `loading = false`                            |
| **Rapid refetch calls**                  | Each call gets incrementing requestId; only latest result updates state                       |
| **Loader function changes**              | `loaderRef.current = loader` captures latest; closure avoids stale function reference         |
| **Throws synchronously**                 | Wrapped in `Promise.resolve().then(() => ...)` so synchronous throw follows same error path   |
| **De error state**                       | Only the most recent error is stored; superseded by new fetch                                 |

---

## Cache Store — Under the Hood

### Internal Architecture

```ts
class CacheStore {
  #entries = new Map<string, CacheEntry>() // key → CacheEntry
  #accessOrder: string[] = [] // LRU tracking
  #pendingWrites = new Map<string, Set<symbol>>() // write-lock per key
  #maxEntries: number // default: 1024
}
```

### Cache Limits and Eviction

| Property           | Value                                                |
| ------------------ | ---------------------------------------------------- |
| Max entries        | 1024 (`CACHE_MAX_ENTRIES`)                           |
| Eviction policy    | LRU — oldest accessed entry evicted when at capacity |
| Periodic cleanup   | Every 60 seconds, fully-expired entries pruned       |
| Per-key write lock | Symbol-based token prevents stale write races        |

### Pruning Behavior

```ts
prune(): number {
  // Removes entries where staleUntil < Date.now()
  // Also cleans up accessOrder list
  // Called every 60s via setInterval (unref'd — doesn't hold process open)
}
```

### Cache Stats

```ts
import { cacheStats } from 'ruvyxa/server'

const stats = cacheStats()
// → { size: 42, maxEntries: 1024 }
```

### Error Isolation

If the producer function throws:

- If stale data exists in cache → returns stale data silently
- If no stale data → propagates the error
- Background refresh failures are swallowed (keep serving stale)

### Concurrency

- Multiple concurrent calls to the same key with stale data: all served stale, only ONE triggers
  background refresh
- Background refresh uses `beginWrite(key)` / `commitWrite(key, token, entry, expectedEntry)`
  pattern
- `commitWrite` checks `expectedEntry` hasn't been replaced; rejects if another write happened
- If commit rejected, `cached.refreshing = false` so next reader can retry

### Per-Worker, Not Shared

The cache store is a module-level singleton (`const cacheStore = new CacheStore()`). Each Node.js
worker process has its own independent cache. This means:

- **Development**: Single process, single cache — consistent
- **Production multi-worker**: Each worker has separate cache; a write on worker A is not visible to
  worker B
- **Solution for production**: Use external cache (Redis, Memcached) via custom loader, or rely on
  SWR to minimize stale windows

---

## Performance Characteristics

| Operation                   | Complexity                                | Overhead                          |
| --------------------------- | ----------------------------------------- | --------------------------------- |
| Cache hit (fresh)           | O(1) map lookup + 1 LRU touch             | ~0.01ms                           |
| Cache hit (stale + SWR)     | O(1) lookup + background Promise          | ~0.01ms sync, producer runs async |
| Cache miss                  | O(1) lookup + producer execution          | Producer time + write overhead    |
| Cache invalidation (key)    | O(1) delete                               | ~0.001ms                          |
| Cache invalidation (prefix) | O(n) scan over all keys                   | Scales with entry count           |
| Cache invalidation (all)    | O(1) map clear                            | ~0.001ms                          |
| LRU eviction                | O(1) shift from front                     | ~0.001ms                          |
| Serialization cost          | User-defined (JSON.stringify in producer) | Varies by payload size            |

---

## Security Considerations

- **No cache key injection**: Keys are arbitrary strings, not user-controlled directly. If building
  keys from user input, sanitize to avoid `invalidateCache("*")` style attacks.
- **No data isolation**: Cache is shared across all users in a process. Don't cache sensitive data
  without appropriate access control on the consuming side.
- **Cache poisoning**: If producer depends on unvalidated input, an attacker could trigger caching
  of poisoned data. Validate inputs before passing to loader.

---

## Error Codes

| Code | Condition                           | Message                                               |
| ---- | ----------------------------------- | ----------------------------------------------------- |
| N/A  | Invalid TTL string                  | `Invalid cache duration "X". Use a positive value...` |
| N/A  | Producer throws (no stale fallback) | Propagated as-is                                      |

No formal RUV error codes for cache operations; errors are thrown directly.

---

## Troubleshooting

| Symptom                              | Cause                                              | Fix                                                   |
| ------------------------------------ | -------------------------------------------------- | ----------------------------------------------------- |
| Cache never hits                     | TTL too short or no `.ttl()` set                   | Set explicit TTL                                      |
| Stale data served too long           | SWR window too wide                                | Reduce `.swr()` value                                 |
| Background refresh never happens     | `cached.refreshing` stuck `true`                   | Entry was replaced externally; next stale hit resets  |
| Memory growing unbounded             | Excessive unique keys                              | Reduce key cardinality; increase TTL; use shared keys |
| Cross-worker inconsistency           | Production multi-process                           | Add external Redis/SharedCache layer                  |
| `useRuvyxaLoader` returns stale data | `deps` array using inline objects                  | Use stable references or primitives                   |
| Loader called on every render        | `deps` omitted from `useRuvyxaLoader`              | Provide empty `deps: []`                              |
| Error not updating after refetch     | Error state cleared on `enabled: false` transition | Normal behavior — re-enable to retry                  |

---

## Best Practices

1. **Use loaders in server components.** Data fetching on the server is faster and more secure than
   fetching from the client.

2. **Set sensible TTLs.** Short TTLs (30s-5m) for frequently changing data. Long TTLs (1h-1d) for
   stable data.

3. **Always set SWR.** It prevents thundering herd problems when TTL expires and multiple requests
   hit at once.

4. **Invalidate on mutations.** When a server action modifies data, invalidate the relevant cache
   keys.

5. **Parameterize cache keys naturally.** Pass arguments to `.get(fn)` — the framework handles key
   construction.

6. **Use `useRuvyxaLoader` sparingly.** Prefer fetching in server components and passing data down.

7. **Cache aggressively in production.** More caching = faster responses = happier users.

8. **Monitor `cacheStats().size`.** If it's consistently at `maxEntries` (1024), your cache is
   thrashing — consider restructuring keys.

9. **Prefix all keys by resource domain.** `user:*`, `blog:*`, `product:*` — makes invalidation
   predictable.

10. **Avoid caching per-user data globally.** The cache is shared across all users in a worker. For
    user-specific data, include the user ID in the key and never serve stale data to the wrong
    consumer.

---

## Advanced Loader Patterns

### Loader Composition

Compose multiple loaders in parallel:

```tsx
// app/dashboard/page.tsx
import { getProfile, getNotifications, getRecentOrders } from '../lib/loaders'

export default async function DashboardPage() {
  const [profile, notifications, recentOrders] = await Promise.all([
    getProfile(),
    getNotifications(),
    getRecentOrders(10),
  ])

  return (
    <div>
      <UserCard user={profile} />
      <NotificationList items={notifications} />
      <OrderTable orders={recentOrders} />
    </div>
  )
}
```

### Conditional Caching

Skip cache for admin users who need fresh data:

```ts
export const getOrders = loader()
  .key('admin:orders')
  .ttl('30s')
  .swr('10s')
  .get(async (fresh: boolean = false) => {
    if (fresh) {
      // Skip cache by calling directly
      return await db.query('SELECT * FROM orders')
    }
    return await db.query('SELECT * FROM orders LIMIT 100')
  })
```

### Pagination with Caching

```ts
export const getPaginatedPosts = loader()
  .key('blog:paginated')
  .ttl('5m')
  .swr('1m')
  .get(async (page: number, pageSize: number = 20) => {
    const offset = (page - 1) * pageSize
    const posts = await db.query('SELECT * FROM posts ORDER BY created_at DESC LIMIT ? OFFSET ?', [
      pageSize,
      offset,
    ])
    const [{ count }] = await db.query('SELECT COUNT(*) as count FROM posts')
    return { posts, total: count, page, pageSize }
  })
```

### Database Query Caching

Cache expensive database aggregations:

```ts
export const getDashboardStats = loader()
  .key('dashboard:stats')
  .ttl('15m')
  .swr('5m')
  .get(async () => {
    const [totalUsers, totalOrders, revenue] = await Promise.all([
      db.query('SELECT COUNT(*) as count FROM users'),
      db.query("SELECT COUNT(*) as count FROM orders WHERE status = 'completed'"),
      db.query("SELECT SUM(amount) as total FROM payments WHERE status = 'settled'"),
    ])
    return {
      totalUsers: totalUsers[0].count,
      totalOrders: totalOrders[0].count,
      revenue: revenue[0].total,
    }
  })
```

### API Fetch Caching

Cache external API calls with timeout:

```ts
export const getExchangeRate = loader()
  .key('fx:rate')
  .ttl('1h')
  .swr('10m')
  .get(async (from: string, to: string) => {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 5000)
    try {
      const res = await fetch(`https://api.exchangerate.com/v1/${from}/${to}`, {
        signal: controller.signal,
      })
      const data = await res.json()
      return data.rate
    } finally {
      clearTimeout(timeout)
    }
  })
```

## Multi-Tenant Caching

For multi-tenant apps, namespace keys by tenant:

```ts
export const getTenantUsers = loader()
  .key('tenant') // base key
  .ttl('5m')
  .get(async (tenantId: string) => {
    // Cache key becomes: tenant:acme-corp
    return await db.query('SELECT * FROM users WHERE tenant_id = ?', [tenantId])
  })
```

Invalidate on tenant update:

```ts
import { invalidateCache } from 'ruvyxa/server'

// Invalidate all caches for this tenant
invalidateCache('tenant:' + tenantId)
```

## External Cache Integration

The built-in in-memory cache is per-worker. For production deployments with multiple workers,
integrate an external cache:

### Redis Adapter Pattern

```ts
import { createClient } from 'redis'

const redis = createClient({ url: process.env.REDIS_URL })

export function redisLoader(key: string, ttlSeconds: number) {
  return {
    async get<T>(producer: () => Promise<T>): Promise<T> {
      const cached = await redis.get(key)
      if (cached) {
        return JSON.parse(cached) as T
      }
      const value = await producer()
      await redis.set(key, JSON.stringify(value), { EX: ttlSeconds })
      return value
    },
  }
}

// Usage
const userData = await redisLoader('user:42', 300).get(async () =>
  db.query('SELECT * FROM users WHERE id = ?', [42]),
)
```

### Cache-Aside Pattern

```ts
async function getFromCacheOrDb<T>(
  cacheKey: string,
  fetchFromDb: () => Promise<T>,
  ttlMs: number = 300_000,
): Promise<T> {
  const cached = cacheStore.get(cacheKey)
  if (cached && cached.expiresAt > Date.now()) {
    return cached.value as T
  }
  const value = await fetchFromDb()
  cacheStore.set(cacheKey, {
    value,
    expiresAt: Date.now() + ttlMs,
    staleUntil: Date.now() + ttlMs,
    refreshing: false,
  })
  return value
}
```

## Middleware-Based Cache Invalidation

Invalidate cache keys from middleware on specific events:

```ts
// app/server/middleware.ts
import { invalidateCache } from 'ruvyxa/server'

export function onResponse(response: Response) {
  if (response.status >= 200 && response.status < 300) {
    // After successful mutations, invalidate relevant caches
    invalidateCache('user:*')
    invalidateCache('dashboard:*')
  }
  return response
}
```

## Cache Warmup Strategies

For critical endpoints, warm the cache on server start:

```ts
// app/lib/warmup.ts
import { getDashboardStats, getRecentPosts, getUserProfile } from './loaders'

export async function warmCriticalCaches() {
  await Promise.all([getDashboardStats(), getRecentPosts(20), getUserProfile('default')])
  console.log('[Warmup] Critical caches populated')
}
```

## Cache Debugging

Log cache hit/miss ratio:

```ts
let hits = 0
let misses = 0

export const debugLoader = loader()
  .key('debug:example')
  .ttl('5m')
  .get(async () => {
    misses++
    return await fetchExpensiveData()
  })

// When used, check:
// In the response or middleware:
console.log(`Cache ratio: ${hits}/${hits + misses}`)
```

## Integration with ISR and SSG

For ISR pages, coordinate cache TTL with revalidation interval:

```ts
// app/blog/[slug]/page.tsx
export const revalidate = 300 // ISR: revalidate every 5 minutes

// In your loader, set matching TTL
export const getBlogPost = loader()
  .key('blog:post')
  .ttl('5m') // matches ISR revalidate
  .swr('1m') // serve stale while revalidating
  .get(async (slug: string) => {
    return await db.query('SELECT * FROM posts WHERE slug = ?', [slug])
  })
```

This ensures both the page cache and data cache have aligned expiration windows.

## Thundering Herd Prevention

The SWR pattern is the primary thundering herd prevention. Here's the exact sequence:

```
1. TTL expires at t=5m
2. 50 concurrent requests arrive at t=5m:01s
3. All 50 see stale entry (expired but within SWR window)
4. First request starts background refresh (cached.refreshing = true)
5. Remaining 49 served stale data immediately
6. Background refresh completes at t=5m:02s
7. Cache populated with fresh data
8. All subsequent requests see fresh data
```

Without SWR:

```
1. TTL expires at t=5m
2. 50 concurrent requests arrive
3. All 50 see empty cache
4. All 50 hit the database simultaneously
5. Database overload → slow responses or crash
```

---

## Try It Yourself

Build a todo app that uses caching.

**Step 1:** Create `app/lib/loaders.ts`:

```ts
import { loader } from 'ruvyxa/server'

export interface Todo {
  id: number
  text: string
  done: boolean
}

// Simulate DB
let todos: Todo[] = [
  { id: 1, text: 'Learn Ruvyxa', done: true },
  { id: 2, text: 'Build something', done: false },
]

export const getTodos = loader()
  .key('todos:all')
  .ttl('30s')
  .swr('10s')
  .get(async () => {
    console.log('[Cache Miss] Fetching todos')
    return [...todos]
  })
```

**Step 2:** Create `app/page.tsx`:

```tsx
import { getTodos } from './lib/loaders'
import { AddTodo } from './AddTodo'

export default async function Home() {
  const todos = await getTodos()

  return (
    <main>
      <h1>Todo List</h1>
      <ul>
        {todos.map((todo) => (
          <li key={todo.id}>
            <span style={{ textDecoration: todo.done ? 'line-through' : 'none' }}>{todo.text}</span>
          </li>
        ))}
      </ul>
      <AddTodo />
    </main>
  )
}
```

**Step 3:** Refresh the page — see the cache log in the terminal. Refresh again — notice the cache
hit (no log). After 30 seconds, the cache expires and the log reappears.

---

## Next Steps

- **[06-server-actions.md](./06-server-actions.md)** — Mutate data with server actions
- **[07-api-routes.md](./07-api-routes.md)** — Build REST APIs
- **[04-rendering-strategies.md](./04-rendering-strategies.md)** — When pages render
