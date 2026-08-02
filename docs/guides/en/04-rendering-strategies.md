# Rendering Strategies

Ruvyxa supports five rendering strategies. The right one depends on one question:

> **When should the HTML for this page be produced?**

```
                         When is HTML produced?
                                  │
              ┌───────────────────┼───────────────────┐
              ▼                   ▼                   ▼
         At request?         At build?           In browser?
              │                   │                   │
         ┌────┴────┐        ┌────┴────┐              CSR
         ▼         ▼        ▼         ▼
        SSR       ISR      SSG       PPR
    (default)  (revalidate) (static)  (partial)
```

---

## Decision Tree

```
Incoming Request
    │
    ▼
Has 'use client' without data-loading server components?
    │         │
   Yes        No
    │         │
    ▼         ▼
   CSR      Has export const ppr = true?
                │         │
               Yes        No
                │         │
                ▼         ▼
               PPR      Has export const revalidate = N?
                           │         │
                          Yes        No
                           │         │
                           ▼         ▼
                          ISR      Has getStaticParams or staticParams?
                           │         │         │
                           │        Yes        No
                           │         │         │
                           │         ▼         ▼
                           │      SSG          Route has dynamic segment?
                           │      (dynamic)    │         │
                           │                  Yes        No
                           │                   │         │
                           │                   ▼         ▼
                           │                 SSR      Has dynamic data marker?
                           │                   │      (fetch, cookies, headers,
                           │                   │       Date.now, Math.random,
                           │                   │       process.env, searchParams)
                           │                   │         │
                           │                   │         │
                           │                   ◄─────────┤
                           │                   │ Yes     │ No
                           │                   ▼         ▼
                           │                SSR        SSG
                           │                           (static)
◄───────────────────────────┘───────────────────────────►

SSG ─── static HTML, Node runtime not required
ISR ─── SSG + cache expiration + background refresh
SSR ─── server render on every request
PPR ─── static shell + live content streaming
CSR ─── purely browser-rendered
```

---

## Detection Algorithm — Exact Priority

Source: `crates/ruvyxa_graph/src/lib.rs`, function `detect_render_strategy`

Ruvyxa detects the strategy automatically based on what you export from your page file. The
detection runs at route discovery time (during `dev` and `build`).

### Priority Order (1 = highest)

| Priority | Strategy          | Detection condition                        | Export required                    |
| -------- | ----------------- | ------------------------------------------ | ---------------------------------- |
| 1        | **CSR**           | File starts with `'use client'`            | None                               |
| 2        | **PPR**           | `export const ppr = true`                  | `ppr`                              |
| 3        | **ISR**           | `export const revalidate = <number>`       | `revalidate`                       |
| 4        | **SSG** (dynamic) | `getStaticParams` or `staticParams` export | `getStaticParams` / `staticParams` |
| 5        | **SSG** (auto)    | Static route, no dynamic markers           | None                               |
| 6        | **SSR**           | Default — none of the above                | None                               |

### Pseudocode

```rust
fn detect_render_strategy(file, layout_chain) -> RenderMeta {
    // 1. Check 'use client' directive (must be at top, before imports)
    if source.trim_start().starts_with("\"use client\"") {
        return CSR
    }

    // Check hydration mode (affects all non-CSR strategies)
    let hydration = parse_hydration_mode(&source)

    // 2. Check PPR: export const ppr = true
    if has_export_const_bool(&code, "ppr", true) {
        return PPR
    }

    // 3. Check ISR: export const revalidate = <number>
    if let Some(seconds) = parse_export_const_number(&code, "revalidate") {
        return ISR { revalidate: seconds }
    }

    // 4. Check static params export
    if has_static_params_export(&code) {
        return SSG
    }

    // 5. Auto-SSG: static route with no dynamic data markers
    let reachable_code = render_reachable_code(app_dir, file, layout_chain)
    if !route_has_dynamic_segments(route_path)
        && !has_dynamic_data_markers(&reachable_code)
    {
        return SSG
    }

    // 6. Default: SSR
    return SSR
}
```

### What Disqualifies Auto-SSG

A route is NOT auto-detected as SSG if any of its reachable code (including layouts and
dependencies) contains:

```rust
const MARKERS: &[&str] = &[
    "fetch(",         // Dynamic data fetching
    "headers(",       // Reading request headers
    "cookies(",       // Reading cookies
    "searchParams",   // Reading query parameters
    "Date.now(",      // Timestamp — changes per call
    "Math.random(",   // Random value — changes per call
    "process.env.",   // Env var access (server-private)
];
```

If any of these strings appear in the page source or its imported dependencies (after stripping
strings and comments), the route defaults to SSR.

### Render Reachable Code Algorithm

The detector walks the full dependency graph:

```rust
fn render_reachable_code(app_dir, file, layout_chain) -> String {
    let mut files = collect_relative_graph(file)     // BFS from page
    for layout in layout_chain {
        files.extend(collect_relative_graph(layout)) // BFS from layouts
    }

    let mut code = String::new()
    for path in files {
        let source = read_file(path)
        let source = strip_mdx_code_examples(source) // for MDX/MD files
        code.push_str(strip_strings_and_comments(source))
    }
    return code // all reachable code, strings/comments stripped
}
```

Only **relative imports** (starting with `.` or `..`) are followed. NPM package imports are not
traversed.

---

## Render Strategy Types

```rust
pub enum RenderStrategy {
    Ssr,  // Server-Side Rendering (default)
    Ssg,  // Static Site Generation
    Isr,  // Incremental Static Regeneration
    Csr,  // Client-Side Rendering
    Ppr,  // Partial Pre-Rendering
}
```

### Rendering Strategy Detection Code

```rust
fn parse_export_const_number(code: &str, name: &str) -> Option<u64> {
    // Looks for: export const revalidate = 300
    // Parses the numeric value after =
}

fn has_export_const_bool(code: &str, name: &str, expected: bool) -> bool {
    // Looks for: export const ppr = true
    // Exact match on the boolean value
}

fn has_static_params_export(code: &str) -> bool {
    // Looks for: export const staticParams = ...
    // Or: export async function getStaticParams ...
    // Or: export function getStaticParams ...
}
```

---

## Render Config (Global Defaults)

Set default rendering behavior in `ruvyxa.config.ts`:

```ts
export interface RenderConfig {
  /** Default rendering strategy for pages without explicit exports. @default "ssr" */
  strategy?: 'ssr' | 'ssg' | 'isr' | 'csr' | 'ppr'
  /** Default ISR revalidation interval in seconds. @default 60 */
  revalidate?: number
}
```

Applied in `apply_rendering_defaults`:

```rust
fn apply_rendering_defaults(render, default_strategy, default_revalidate) -> RenderMeta {
    if render.strategy != SSR { return render }  // explicit strategy wins
    if default_strategy is None { return render } // no global default

    render.strategy = default_strategy
    if strategy == ISR {
        render.revalidate = default_revalidate.unwrap_or(60)
    }
    return render
}
```

---

## RenderMeta — Full Schema

```rust
pub struct RenderMeta {
    /// Rendering strategy for this route
    pub strategy: RenderStrategy,
    /// ISR revalidation interval in seconds
    pub revalidate: Option<u64>,
    /// Whether the page exports getStaticParams or staticParams
    pub has_static_params: bool,
    /// Static paths discovered from getStaticParams at build time
    pub static_paths: Vec<String>,
    /// Whether PPR page uses <Suspense> boundaries
    pub has_dynamic_slots: bool,
    /// Whether served HTML includes client hydration bundle
    pub hydrate: bool,
    /// Scheduling mode for client bundle
    pub hydration: HydrationMode,
}
```

---

## SSR — Server-Side Rendering (Default)

HTML is generated on every request. Best for personalized or frequently changing content.

```tsx
// app/profile/page.tsx — SSR (default)
import { db } from '../server/db'

export default async function ProfilePage({ params }: { params: { id: string } }) {
  const user = await db.query('SELECT * FROM users WHERE id = ?', [params.id])
  //        ^^ runs on every request

  return (
    <div>
      <h1>{user.name}</h1>
      <p>Member since {user.createdAt}</p>
    </div>
  )
}
```

```
SSR Flow:

  Request → Server renders HTML → Send HTML to browser
     │                              │
     └── includes JSON data ────────┘
                                    │
                              Hydrate on client
```

### Render Cache Behavior

SSR uses a render cache to avoid re-rendering identical pages:

| Cache config                   | Behavior                                          |
| ------------------------------ | ------------------------------------------------- |
| `cache.routes: true` (default) | Rendered HTML cached, served on repeat requests   |
| `cache.routes: false`          | Every request triggers a fresh render             |
| Default TTL                    | 60 seconds (configurable via `render.revalidate`) |

Cache key: full URL including query parameters.

### SSR Error Code

```
RUV1100: React SSR failed
  └─ Error during server-side rendering
  └─ Component stack trace included
```

### SSR Error Code

```
RUV1102: SSR renderer was not found
  └─ Page module missing or invalid
```

**Use SSR when:** content is user-specific, requires auth, or changes every visit.

---

## SSG — Static Site Generation

HTML is generated at build time. No server work at request time.

### For Static Routes (Auto-SSG)

If a route has no dynamic segments and no dynamic features, Ruvyxa automatically SSGs it.

```tsx
// app/about/page.tsx — auto-SSG
export default function AboutPage() {
  return <h1>About Us (built once)</h1>
}
```

### Conditions for Auto-SSG Detection

A route qualifies for auto-SSG if ALL of these are true:

1. No `'use client'` directive
2. No `export const ppr = true`
3. No `export const revalidate = <number>`
4. No `getStaticParams` or `staticParams` export
5. Route path has **no dynamic segments** (`[slug]`, `[...path]`, `[[...path]]`)
6. Reachable code has **no dynamic markers** (`fetch(`, `headers(`, `cookies(`, `searchParams`,
   `Date.now(`, `Math.random(`, `process.env.`)

### For Dynamic Routes with `getStaticParams`

```tsx
// app/blog/[slug]/page.tsx
import { db } from '../server/db'

export async function getStaticParams() {
  const slugs = await db.query('SELECT slug FROM posts')
  return slugs.map((row: any) => ({ slug: row.slug }))
}

export default async function BlogPost({ params }: { params: { slug: string } }) {
  const post = await db.query('SELECT * FROM posts WHERE slug = ?', [params.slug])
  return <h1>{post.title}</h1>
}
```

### `GetStaticParams` — Full Type

```ts
/** Context passed to getStaticParams at build time */
export interface StaticParamsContext {
  /** All route entries discovered in the app */
  routes: Array<{ path: string; id: string }>
  /** The dynamic route currently requesting parameters */
  route: {
    path: string
    segments: StaticParamSegment[]
  }
}

/** A dynamic segment included in the route being statically generated */
export interface StaticParamSegment {
  name: string
  catchAll: boolean
  optional: boolean
}

/** Duration accepted by persistent SSG parameter discovery caching */
export type StaticParamsCacheDuration = number | `${number}${'s' | 'm' | 'h' | 'd'}`

/** Static parameter values. String shorthand allowed for single-segment routes. */
export type StaticParamsValues<TParams extends RouteParams = RouteParams> = ReadonlyArray<
  TParams | string | number
>

/** Opt-in cache metadata for parameter discovery results */
export interface CachedStaticParams<TParams extends RouteParams = RouteParams> {
  params: StaticParamsValues<TParams>
  /** Cache duration in seconds or compact string like "10m" */
  cache: StaticParamsCacheDuration
}

/** Value accepted from getStaticParams or staticParams page export */
export type StaticParamsResult<TParams extends RouteParams = RouteParams> =
  StaticParamsValues<TParams> | CachedStaticParams<TParams>

export type GetStaticParams<TParams extends RouteParams = RouteParams> = (
  ctx: StaticParamsContext,
) => StaticParamsResult<TParams> | Promise<StaticParamsResult<TParams>>
```

### `staticParams` — Scalar Shorthand

For routes with a single dynamic segment, use `staticParams` as a string array:

```tsx
export const staticParams = ['hello-world', 'another-post', 'third-post']

export default function BlogPost({ params }: { params: { slug: string } }) {
  return <h1>{params.slug}</h1>
}
```

This is equivalent to `getStaticParams` returning
`[{ slug: "hello-world" }, { slug: "another-post" }, { slug: "third-post" }]`.

**Shorthand rules:**

- Only valid for routes with exactly **one** dynamic segment
- String values are mapped to an object with the segment name as key
- `staticParams: ["a", "b"]` → `[{ slug: "a" }, { slug: "b" }]`
- Also accepts numbers: `staticParams: [1, 2, 3]` → `[{ id: 1 }, { id: 2 }, { id: 3 }]`

### `getStaticParams` with Context

```tsx
export async function getStaticParams(context: StaticParamsContext) {
  console.log(context.route.path) // "/blog/[slug]"
  console.log(context.routes.length) // total routes in app

  const slugs = await db.query('SELECT slug FROM posts')
  return slugs.map((row: any) => ({ slug: row.slug }))
}
```

### Persistent Parameter Cache

Return `{ params, cache }` from `getStaticParams` to avoid recomputing params on every build:

```tsx
export async function getStaticParams() {
  const rows = await db.query('SELECT slug FROM posts')
  return {
    params: rows.map((r: any) => ({ slug: r.slug })),
    cache: '10m', // cache the parameter list for 10 minutes
  }
}
```

**Cache duration formats:**

| Format  | Meaning    |
| ------- | ---------- |
| `60`    | 60 seconds |
| `"60s"` | 60 seconds |
| `"10m"` | 10 minutes |
| `"2h"`  | 2 hours    |
| `"1d"`  | 1 day      |

### SSG Build Output

```
.ruvyxa/prerender/
  index.html                     → /
  about.html                     → /about
  blog/
    hello-world.html             → /blog/hello-world
    index.html                   → /blog
```

### Pre-render Output Manifest

The prerender output includes a manifest:

```json
.ruvyxa/prerender/manifest.json
{
  "version": 1,
  "generated": "2026-07-29T12:00:00Z",
  "routes": [
    {
      "path": "/",
      "file": "index.html",
      "strategy": "ssg",
      "params": {}
    },
    {
      "path": "/blog/hello-world",
      "file": "blog/hello-world.html",
      "strategy": "ssg",
      "params": { "slug": "hello-world" }
    }
  ]
}
```

### SSG Error Code

```
RUV1500: SSG render failed
  └─ Error during static generation of /blog/hello-world
  └─ TypeError: Cannot read properties of undefined
```

Also:

```
RUV1205: Prerender path `/` for route `/` cannot be written inside the build output.
  └─ Route path conflicts with build output directory structure
```

---

## ISR — Incremental Static Regeneration

Like SSG, but the page revalidates after a set time. Uses stale-while-revalidate.

```tsx
// app/blog/[slug]/page.tsx — ISR
export const revalidate = 60 // Revalidate every 60 seconds

export async function getStaticParams() {
  return ['post-1', 'post-2', 'post-3']
}

export default async function BlogPost({ params }: { params: { slug: string } }) {
  const data = await fetch(`https://api.example.com/posts/${params.slug}`).then((r) => r.json())
  return <h1>{data.title}</h1>
}
```

### Revalidate Behavior

| `revalidate` value | Behavior                                           |
| ------------------ | -------------------------------------------------- |
| `0`                | Never cache — always fetch fresh (effectively SSR) |
| `60`               | Cache for 60 seconds, stale-while-revalidate after |
| `3600`             | Cache for 1 hour                                   |
| `86400`            | Cache for 1 day                                    |
| Default            | 60 seconds (from config)                           |

### ISR Flow

```
ISR Flow:

  Request #1 (t=0s):
    ┌─ Cache empty? → Render page → Store in cache → Serve
    └─ Cache fresh? → Serve cached response

  Request #2 (t=65s, TTL expired):
    ┌─ Cache stale? → Serve stale HTML immediately
    │                  └── Trigger background re-render
    │                                        ↓
    │                                 New HTML stored in cache
    │                                        ↓
    └─ Next request gets fresh HTML
```

The cache uses a **stale-while-revalidate** pattern:

- Within `revalidate` seconds: serve cached HTML
- After `revalidate` seconds: serve stale HTML, re-render in background
- Once re-render completes: update cache

### Cache Coalescing

When multiple concurrent requests arrive for a stale ISR page:

1. First request triggers background re-render
2. Subsequent requests receive the stale HTML (not queued)
3. Only one background re-render runs at a time per route

### ISR with Render Cache

```
RUV1500: ISR render failed
  └─ Background revalidation error for /blog/my-post
```

**Use ISR when:** content changes occasionally but does not need to be real-time (blogs, marketing
pages, docs).

---

## PPR — Partial Prerendering

Static shell + dynamic slots. The best of SSG and SSR combined.

```tsx
// app/dashboard/page.tsx
export const ppr = true

import { Suspense } from 'react'
import { SlowWidget } from './SlowWidget'
import { FastWidget } from './FastWidget'

export default function Dashboard() {
  return (
    <div className="dashboard">
      <h1>Dashboard</h1>

      {/* Static shell — pre-rendered at build time */}
      <FastWidget />

      {/* Dynamic slot — rendered per request, streamed in */}
      <Suspense fallback={<div>Loading user data...</div>}>
        <SlowWidget />
      </Suspense>
    </div>
  )
}
```

### PPR Flow

```
PPR Flow:

  Build time:
    ┌──────────────────────────────────────┐
    │ Static Shell (pre-rendered)          │
    │  <h1>Dashboard</h1>                  │
    │  <FastWidget /> HTML                 │
    │  <div id="slot-1">                   │
    │    Loading user data...              │
    │  </div>                              │
    └──────────────────────────────────────┘

  Request time:
    ┌──────────────────────────────────────┐
    │ Send static shell immediately        │
    │ While streaming dynamic slot #1      │
    │ While streaming dynamic slot #2      │
    └──────────────────────────────────────┘
```

### Suspense Boundary Rules

Every `<Suspense>` boundary becomes a **dynamic slot** that streams in after the static shell.

| Boundary placement                       | Behavior                            |
| ---------------------------------------- | ----------------------------------- |
| `<Suspense>` at top level                | Dynamic slot, streamed after shell  |
| `<Suspense>` inside another `<Suspense>` | Nested slot, streamed independently |
| No `<Suspense>`                          | Content is part of static shell     |

### PPR with Streaming

PPR streaming behavior depends on the runtime:

| Runtime | Streaming support                                   |
| ------- | --------------------------------------------------- |
| Node.js | Full streaming via `ReadableStream`                 |
| Bun     | Full streaming via `ReadableStream`                 |
| Edge    | Streaming via `ReadableStream` (platform-dependent) |
| Static  | PPR not supported (fallback to SSG)                 |

### PPR Error Code

```
RUV1550: PPR render failed
  └─ Dynamic slot rendering error
  └─ Suspense boundary: #slot-1
```

**Use PPR when:** you have a mostly static page with some dynamic sections (dashboards, product
pages with personalized recommendations).

---

## CSR — Client-Side Rendering

Minimal HTML shell. Everything renders in the browser.

```tsx
'use client'

import { useState, useEffect } from 'react'

export default function Dashboard() {
  const [data, setData] = useState(null)

  useEffect(() => {
    fetch('/api/data')
      .then((r) => r.json())
      .then(setData)
  }, [])

  if (!data) return <p>Loading...</p>

  return <h1>{data.title}</h1>
}
```

### CSR HTML Output

CSR pages receive a minimal shell HTML:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
  </head>
  <body>
    <div id="__ruvyxa"></div>
    <script type="module" src="/__ruvyxa/client?path=/dashboard"></script>
  </body>
</html>
```

No page content is server-rendered into a CSR shell; the client bundle renders it in the browser.

### CSR Auto-Detection

CSR is triggered by the `'use client'` directive at the top of the page file:

```rust
// Detection in detect_render_strategy:
// 1. Check 'use client' directive (highest priority)
let trimmed = source.trim_start();
if trimmed.starts_with("\"use client\"") || trimmed.starts_with("'use client'") {
    return RenderMeta { strategy: RenderStrategy::Csr, ... }
}
```

CSR detection is **top priority** — it overrides all other strategy exports.

### CSR vs SSR with Client Component

| Aspect       | CSR page                              | SSR (default) + client island     |
| ------------ | ------------------------------------- | --------------------------------- |
| HTML content | Minimal shell                         | Full server-rendered HTML         |
| JS bundle    | Whole page ships to client            | Only client islands ship          |
| Hydration    | Full page hydration                   | Island hydration                  |
| SEO          | Limited (crawlers may not execute JS) | Full content accessible           |
| Initial load | Fast HTML, then wait for JS           | Full HTML visible, JS progressive |

**Use CSR when:** the page is behind a login wall, or you are building a highly interactive app that
cannot be effectively server-rendered.

---

## Hydration Scheduling

Control **when** a client component hydrates. Useful for deferring non-critical interactivity.

```tsx
'use client'

import { HeavyChart } from './HeavyChart'

export default function Page() {
  return (
    <div>
      <h1>Dashboard</h1>

      {/* Hydrate immediately (default) */}
      <HeavyChart hydrate="load" />

      {/* Hydrate after browser idle */}
      <HeavyChart hydrate="idle" />

      {/* Hydrate when scrolled into viewport */}
      <HeavyChart hydrate="visible" />

      {/* No hydration — pure server HTML, no JS */}
      <HeavyChart hydrate={false} />
    </div>
  )
}
```

| `hydrate`          | When JS runs                 | Implementation                | Use case                     |
| ------------------ | ---------------------------- | ----------------------------- | ---------------------------- |
| `"load"`           | Immediately                  | Normal hydration after render | Critical interactivity       |
| `"idle"`           | After `requestIdleCallback`  | Browser idle callback         | Non-urgent UI                |
| `"visible"`        | When element enters viewport | `IntersectionObserver`        | Below-the-fold content       |
| `false` / `"none"` | Never                        | No hydration bundle shipped   | Static content, no JS needed |

### Route-Level Hydration

Export `hydrate` from a page to set the default for all its client components:

```tsx
// app/page.tsx
export const hydrate = 'idle' // all client components defer hydration
```

### `HydrationMode` Enum

```rust
pub enum HydrationMode {
    Load,    // Default
    Idle,    // requestIdleCallback
    Visible, // IntersectionObserver, 200px rootMargin
    None,    // Zero-JS
}
```

### Zero-JS Pages

Set `hydrate={false}` to ship **zero JavaScript** for a component. Combine with SSG for truly static
pages.

```tsx
// app/docs/page.tsx
import { TableOfContents } from './TableOfContents'

export default function DocsPage() {
  return (
    <div>
      <h1>Documentation</h1>
      <TableOfContents hydrate={false} /> {/* 0 JS */}
    </div>
  )
}
```

### Hydration Inheritance

| Parent                 | Child prop                 | Result                                |
| ---------------------- | -------------------------- | ------------------------------------- |
| Route `hydrate="idle"` | Component default          | Component hydrates "idle"             |
| Route `hydrate="idle"` | Component `hydrate="load"` | Component hydrates "load" (overrides) |
| Route `hydrate=false`  | Component default          | Component never hydrates              |
| Route `hydrate=false`  | Component `hydrate="load"` | Component hydrates "load"             |
| CSR page               | Any hydrate prop           | Always hydrates (CSR ignores hydrate) |

### Module Preload Behavior for Deferred Routes

When a route uses deferred hydration, the client module is **not** eagerly loaded:

- `hydrate="load"` — module included in initial bundle
- `hydrate="idle"` — module loaded after `requestIdleCallback`
- `hydrate="visible"` — module loaded when element observed
- `hydrate=false` — module never loaded

---

## Prerender Output

SSG and ISR pages write HTML to `.ruvyxa/prerender/`:

```
.ruvyxa/prerender/
  index.html                     → /
  about.html                     → /about
  blog/
    hello-world.html             → /blog/hello-world
    index.html                   → /blog
```

These files are served directly by your adapter (Vercel, Netlify, Cloudflare, Node).

### Build Output Schema

After `ruvyxa build`, the `.ruvyxa/` directory contains:

```
.ruvyxa/
├── prerender/
│   ├── manifest.json           # Prerender manifest
│   ├── index.html              # Pre-rendered root page
│   ├── about.html              # Pre-rendered about page
│   └── blog/
│       ├── hello-world.html    # Pre-rendered blog post
│       └── index.html          # Pre-rendered blog listing
├── client/
│   ├── manifest.json           # Client bundle manifest
│   ├── pages/
│   │   ├── index.mjs           # Client entry for /
│   │   └── blog/[slug].mjs     # Client entry for /blog/:slug
│   └── shared/
│       └── vendor.mjs          # Shared vendor bundle
├── server/
│   ├── handler.mjs             # Server handler
│   └── route-modules.mjs       # Route module registry
└── build.json                  # Build report
```

### `build.json` Full Schema

```json
{
  "version": 1,
  "timestamp": "2026-07-29T12:00:00Z",
  "duration": 4523,
  "routes": {
    "total": 12,
    "pages": 10,
    "api": 2
  },
  "strategies": {
    "ssg": 5,
    "ssr": 3,
    "isr": 1,
    "ppr": 1,
    "csr": 2
  },
  "sizes": {
    "clientBundle": 245760,
    "prerendered": 102400,
    "shared": 189440
  },
  "diagnostics": []
}
```

---

## Rendering Configuration

### Config Fields (`ruvyxa.config.ts`)

```ts
export default config({
  render: {
    strategy: 'ssr', // default strategy for all pages
    revalidate: 60, // default ISR TTL in seconds
  },
  build: {
    minify: true,
    map: false,
    treeShake: true,
    split: 'route', // 'single' | 'route' | 'manual'
    workers: 4,
    prerenderCache: true, // reuse valid prerender HTML between builds
  },
  cache: {
    routes: true, // cache SSR/ISR HTML
    css: true, // cache compiled CSS
  },
})
```

### `split` Strategies

| Value               | Behavior                             | Best for                             |
| ------------------- | ------------------------------------ | ------------------------------------ |
| `'single'`          | Single bundle for all routes         | Small apps, no code splitting needed |
| `'route'` (default) | One bundle per route + shared chunks | Most apps                            |
| `'manual'`          | Manual code splitting via `import()` | Advanced optimization                |

---

## Per-Strategy Comparison

| Property        | SSR              | SSG               | ISR                    | PPR                         | CSR           |
| --------------- | ---------------- | ----------------- | ---------------------- | --------------------------- | ------------- |
| HTML generation | Every request    | Build time        | Build + revalidate     | Build shell + request slots | Minimal shell |
| Server load     | High             | None              | Low (background only)  | Medium                      | None          |
| Freshness       | Always fresh     | Static            | Stale-while-revalidate | Shell static, slots fresh   | N/A           |
| Build time      | Fast             | Slow (many pages) | Slow                   | Medium                      | Fast          |
| SEO             | Full             | Full              | Full                   | Full                        | Limited       |
| JS required     | No (progressive) | No                | No                     | No (for shell)              | Yes           |
| Use case        | User dashboards  | Marketing pages   | Blogs, docs            | Product pages               | Login walls   |
| Cache TTL       | None (or config) | Forever           | Configurable (seconds) | Shell forever               | N/A           |
| Dynamic data    | Yes              | No                | Yes (background)       | Yes (slots)                 | Yes (client)  |
| Personalization | Yes              | No                | No (stale)             | Yes (slots)                 | Yes           |

---

## Rendering Strategy Detection — Edge Cases

### `export const revalidate = 0`

This creates an SSR-like behavior with the ISR strategy. The page revalidates on every request
effectively, but the ISR caching infrastructure is used.

### Both `ppr` and `revalidate` exported

PPR takes priority (priority 2) over ISR (priority 3). The `revalidate` export is ignored.

### Both `'use client'` and `ppr`

CSR takes priority (priority 1). The `ppr` export is ignored. The page is fully client-rendered.

### MDX/MD Content Pages

Markdown and MDX pages follow the same detection rules. Fenced code blocks and inline code spans are
**blanked** before detection, so example code showing `fetch(` or `process.env.` doesn't
accidentally prevent auto-SSG.

### Route with `getStaticParams` but also dynamic markers

If a page exports `getStaticParams` but reachable code contains `fetch(`, the route is SSG
(priority 4) — the dynamic markers only affect auto-SSG detection (priority 5).

---

## Strategy Detection — What Runs When

```
                            BUILD TIME
                                │
                    ┌───────────┴───────────┐
                    │                       │
                   SSG                     ISR
              (pre-render all            (pre-render all
               pages + params)            pages + params)
                    │                       │
                    └───────────┬───────────┘
                                │
                            DEPLOY
                                │
                        ┌───────┴───────┐
                        │               │
                    REQUEST TIME    REQUEST TIME
                        │               │
                   ┌────┴────┐     ┌────┴────┐
                   │         │     │         │
                  SSR       PPR   ISR       CSR
              (render per   (shell  (serve    (boot JS
               request)     static, stale,     in
                            stream  re-render  browser)
                            slots)  bg)
```

---

## Best Practices

1. **Prefer SSG for eligible pages.** The framework's automatic detector still defaults to SSR when
   a route is dynamic or cannot be proven static; choose SSG deliberately when the route and its
   reachable dependencies are static.

2. **Use ISR for content that changes.** Blog posts, docs, marketing pages — these are perfect for
   ISR.

3. **Use PPR for personalized pages.** Dashboards, user profiles — static shell + dynamic slots.

4. **Avoid full-page CSR.** Extract interactive islands instead of making the whole page
   client-rendered.

5. **Match `revalidate` to your update frequency.** If you post once a day, `revalidate = 3600` (1
   hour) is fine.

6. **Lazy hydrate non-critical UI.** Use `hydrate="idle"` or `hydrate="visible"` for charts,
   comments, and analytics.

7. **Check `npm run routes` to verify strategy.** Each route shows its detected type.

8. **Use Prerender Cache for faster rebuilds.** Set `build.prerenderCache: true` to reuse unchanged
   prerendered HTML.

9. **Monitor ISR cache hit rate.** A low hit rate means your `revalidate` is too short.

10. **Match strategy to content lifecycle.**

| Content type                      | Recommended strategy     |
| --------------------------------- | ------------------------ |
| Marketing pages                   | SSG (auto)               |
| Blog posts                        | ISR (`revalidate: 3600`) |
| User dashboard                    | SSR                      |
| Product page with recommendations | PPR                      |
| Admin panel (behind auth)         | CSR                      |
| API documentation                 | SSG with `staticParams`  |

---

## Troubleshooting

| Problem                           | Cause                                                             | Solution                                   |
| --------------------------------- | ----------------------------------------------------------------- | ------------------------------------------ |
| SSG does not work, always SSR     | Dynamic APIs are used e.g. `cookies()`, `headers()`, `Date.now()` | Use ISR or SSR instead                     |
| ISR does not refresh              | `revalidate` time not reached, or error in background render      | Wait or reduce revalidate time             |
| PPR static shell has dynamic data | Missing `<Suspense>` boundary                                     | Wrap dynamic content with `<Suspense>`     |
| CSR has flash                     | Missing fallback state                                            | Add loading state                          |
| Slow build due to too much SSG    | getStaticParams returns too many paths                            | Use ISR or limit params                    |
| SSG takes long build time         | Too many SSG pages                                                | Convert some pages to ISR                  |
| ISR cache does not refresh        | Background render error                                           | Check server logs                          |
| PPR stream is slow                | Dynamic slots are heavy                                           | Use multi-level `<Suspense>` or add cache  |
| getStaticParams fails             | Database/API not ready at build time                              | Use try/catch or ISR instead               |
| Hydration mismatch                | Server HTML ≠ Client first render                                 | Check `useEffect` + `Date` / `Math.random` |

### Error Examples

```tsx
// SSG fails due to Date.now()
export default function Page() {
  // ❌ Date.now() → SSR (dynamic marker)
  const time = Date.now()
  return <p>{time}</p>
}

// ✅ Use new Date() → SSG (Not a dynamic marker)
export default function Page() {
  const time = new Date().toISOString()
  return <p>{time}</p>
}
```

```tsx
// PPR fails because there is no Suspense
export const ppr = true

export default function Page() {
  // ❌ PPR requires <Suspense> boundary
  return <DynamicContent />
}

// ✅ Correct
export const ppr = true

export default function Page() {
  return (
    <div>
      <p>Static shell</p>
      <Suspense fallback={<p>Loading...</p>}>
        <DynamicContent />
      </Suspense>
    </div>
  )
}
```

---

## Try It Yourself

Create a blog with mixed rendering:

```
app/
├── page.tsx                ← SSG (auto)
├── blog/
│   ├── page.tsx            ← SSG with getStaticParams
│   └── [slug]/
│       └── page.tsx        ← ISR (revalidate: 300)
└── dashboard/
    └── page.tsx            ← PPR
```

**Step 1:** `app/page.tsx` — SSG (auto):

```tsx
export default function Home() {
  return <h1>Welcome</h1>
}
```

**Step 2:** `app/blog/[slug]/page.tsx` — ISR:

```tsx
export const revalidate = 300

export async function getStaticParams() {
  return [{ slug: 'first-post' }, { slug: 'second-post' }]
}

export default function Post({ params }: { params: { slug: string } }) {
  return <h1>{params.slug}</h1>
}
```

**Step 3:** `app/dashboard/page.tsx` — PPR:

```tsx
export const ppr = true

import { Suspense } from 'react'

async function UserData() {
  const data = await fetch('https://api.example.com/user').then((r) => r.json())
  return <p>Hello, {data.name}</p>
}

export default function Dashboard() {
  return (
    <div>
      <h1>Dashboard</h1>
      <Suspense fallback={<p>Loading user...</p>}>
        <UserData />
      </Suspense>
    </div>
  )
}
```

Build and check the output:

```bash
npm run build
```

Look at `.ruvyxa/prerender/` to see what was pre-rendered.

---

## How the Current Strategy Detector Decides

Rendering strategy is derived from the page source plus the route and its reachable relative
dependencies. The detector uses this precedence order; the first matching rule wins:

1. A top-of-file `'use client'` directive makes the page CSR.
2. `export const ppr = true` selects PPR.
3. `export const revalidate = <number>` selects ISR with that interval.
4. A `getStaticParams` or `staticParams` export selects SSG.
5. A non-dynamic route with no reachable `fetch(` or `process.env.` marker is an automatic SSG
   candidate.
6. Otherwise, the route is SSR unless `render.strategy` supplies a configured default.

This means `Date.now()` alone is not the detector's SSR signal. A simple static route that needs a
specific contract should declare the intended strategy explicitly through the supported exports or
configuration rather than relying on incidental code shape.

### Explicit Examples and Their Consequences

```tsx
// app/docs/page.tsx -- no dynamic segments and no data markers: SSG candidate
export default function Docs() {
  return <main>Documentation</main>
}
```

```tsx
// app/blog/[slug]/page.tsx -- dynamic SSG requires concrete parameters
export const getStaticParams = async () => [{ slug: 'welcome' }, { slug: 'release-notes' }]

export default function Post({ params }: { params: { slug: string } }) {
  return <main>{params.slug}</main>
}
```

```tsx
// app/status/page.tsx -- build once, refresh in the background after 30 seconds
export const revalidate = 30

export default async function Status() {
  const response = await fetch('https://status.example.test/api')
  return <pre>{await response.text()}</pre>
}
```

For an ISR route, the interval belongs in `revalidate`; request-time caching behavior is handled by
the server or selected deployment adapter. Test the actual manifest instead of inferring it from the
generated directory layout:

```bash
ruvyxa routes
ruvyxa trace /status
```

### Hydration Is an Independent Choice

For server-rendered strategies, `export const hydrate` controls whether and when the client bundle
loads. The supported values are `false` (no client bundle), `'idle'`, `'visible'`, and the default
load behavior. A CSR page created by `'use client'` remains client-rendered even when a page also
contains a hydration export.

```tsx
// static, zero-JS content page
export const hydrate = false

export default function LegalNotice() {
  return <main>Terms and conditions</main>
}
```

Use zero-JS pages only when their rendered content does not rely on client interactivity. Client
islands still need a hydrated client bundle, so keep interactive behavior in a deliberately
client-reachable module.

---

## Next Steps

- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — Data fetching and caching
- **[06-server-actions.md](./06-server-actions.md)** — Server actions for mutations
- **[07-api-routes.md](./07-api-routes.md)** — Building API endpoints
