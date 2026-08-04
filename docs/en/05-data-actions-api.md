# Data, actions, and API routes

## Loaders and the in-memory cache

`loader(handler)` creates an async callable marked as a Ruvyxa loader. Its handler receives
`{ params, request, cache }`. `cache(key)` is an in-process cache with an LRU limit of 1024 entries,
default TTL of 60 seconds, optional stale-while-revalidate, and prefix invalidation. It is not a
distributed cache.

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

Cache durations accept a positive integer plus `ms`, `s`, `m`, `h`, or `d`.
`invalidateCache('products')` removes `products` and keys beginning `products:`; no argument clears
the complete process cache. Call `cacheStats()` to obtain `{ size, maxEntries }`.

## Server actions

Build an action with `action.input(schema).handler(handler)`. The schema only needs a synchronous
`parse(value)` method. The action handler receives the parsed `input`, the request, optional user
data, and `invalidate(key)`. `.realtime(channels?)` publishes after successful invocation when the
realtime capability is configured.

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

An action accepts at most 16 realtime channels. Channel names use 1–128 letters, digits, `:`, `.`,
`_`, `/`, or `-`. Set action payload and rate restrictions under `security`; see
[Security](13-security.md).

## API routes

Put a `route.ts` in the target folder and export an upper-case method function. The demo's
`app/api/echo/route.ts` exports `POST({ request })`, reads JSON, and returns `Response.json`. Use
the standards-based response helpers when useful: `json(data, init)`, `redirect(location, status)`,
and `notFound(message)` from `ruvyxa/server`.

```ts
// app/api/health/route.ts
export function GET() {
  return Response.json({ ok: true })
}
```

Route handlers must validate untrusted bodies before using them. API payload limits are governed by
`security.apiLimit`; action payloads use `security.actionLimit`.

**Previous:** [Routing and rendering](04-routing-rendering.md) · **Next:**
[UI, navigation, metadata, and assets](06-ui-navigation-metadata-and-assets.md)
