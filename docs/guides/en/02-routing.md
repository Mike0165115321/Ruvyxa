# Routing in Ruvyxa

Ruvyxa uses a **file-system router**. The folder structure inside `app/` maps directly to URL paths.
No config file, no route registry — just files and folders.

```
          URL
       /blog/hello-world
           ↑
    app/blog/[slug]/page.tsx
           ↑
    Folders → URL segments
```

---

## Mental Model

Every file in `app/` that follows a convention becomes a route. The folder path determines the URL.

```
app/
├── page.tsx                → /
├── layout.tsx              → root layout (wraps all pages)
├── about/
│   ├── page.tsx            → /about
│   └── layout.tsx          → /about layout
├── blog/
│   ├── page.tsx            → /blog
│   └── [slug]/
│       └── page.tsx        → /blog/:slug
└── api/
    └── hello/
        └── route.ts        → /api/hello (GET, POST, etc.)
```

---

## File Conventions

| File            | Role                                                              |
| --------------- | ----------------------------------------------------------------- |
| `page.tsx`      | Page component — renders UI at this route                         |
| `page.jsx`      | Page component (JSX variant)                                      |
| `page.ts`       | Page component (no JSX, returns React element)                    |
| `page.js`       | Page component (JS variant)                                       |
| `page.mdx`      | MDX page — Markdown with React components                         |
| `page.md`       | Markdown page — plain markdown                                    |
| `layout.tsx`    | Layout component — wraps child pages, persists across navigations |
| `route.ts`      | API endpoint — exports HTTP method handlers                       |
| `route.js`      | API endpoint (JS variant)                                         |
| `loading.tsx`   | Loading UI — shown while the page loads                           |
| `error.tsx`     | Error UI — shown when an error occurs                             |
| `not-found.tsx` | 404 UI — shown for missing routes                                 |

### Extension Resolution Order

When multiple files define the same route, the priority is:

```
page.tsx > page.jsx > page.ts > page.js > page.mdx > page.md
route.ts > route.js
```

### Server-side Module Conventions

| File         | Role                                                    |
| ------------ | ------------------------------------------------------- |
| `action.ts`  | Server actions for this route (exports named functions) |
| `action.js`  | Server actions (JS variant)                             |
| `server.ts`  | Server-only data access module                          |
| `server.js`  | Server-only data access (JS variant)                    |
| `client.tsx` | Client-entry module for this route                      |

---

## Static Routes

The simplest kind. A folder with `page.tsx` becomes a static URL.

```
app/
  about/
    page.tsx  →  /about
```

```tsx
// app/about/page.tsx
export default function AboutPage() {
  return <h1>About Us</h1>
}
```

Visit `/about` — done.

Nesting works the same way:

```
app/
  docs/
    getting-started/
      page.tsx  →  /docs/getting-started
```

---

## Dynamic Routes

Use `[slug]` syntax for dynamic segments. The `params` object gives you the value **synchronously**.

```
app/
  blog/
    [slug]/
      page.tsx  →  /blog/:slug
```

```tsx
// app/blog/[slug]/page.tsx
export default function BlogPost({ params }: { params: { slug: string } }) {
  return (
    <article>
      <h1>Blog Post: {params.slug}</h1>
    </article>
  )
}
```

Visit `/blog/hello-world` → renders "Blog Post: hello-world".

### Multiple Dynamic Segments

```
app/
  docs/
    [category]/
      [article]/
        page.tsx  →  /docs/:category/:article
```

```tsx
// app/docs/[category]/[article]/page.tsx
export default function DocPage({ params }: { params: { category: string; article: string } }) {
  return (
    <h1>
      {params.category} / {params.article}
    </h1>
  )
}
```

### `PageProps` Type Definition

```ts
export interface PageProps<TParams extends RouteParams = RouteParams> {
  params: TParams
  requestPath: string
}

export type RouteParams = Record<string, RouteParamValue>
export type RouteParamValue = string | string[] | undefined
```

Usage:

```tsx
import type { PageProps } from 'ruvyxa'

export default function BlogPost({ params, requestPath }: PageProps<{ slug: string }>) {
  // params.slug → string
  // requestPath → "/blog/hello-world"
}
```

---

## Catch-all Routes

Use `[...path]` to match one or more segments. `params.path` is a **string array**.

```
app/
  docs/
    [...path]/
      page.tsx  →  /docs/*
```

| URL                   | `params.path`          |
| --------------------- | ---------------------- |
| `/docs`               | `[]`                   |
| `/docs/guide`         | `["guide"]`            |
| `/docs/guide/routing` | `["guide", "routing"]` |

```tsx
export default function DocsPage({ params }: { params: { path: string[] } }) {
  return <h1>Docs: {params.path.join(' / ')}</h1>
}
```

### Optional Catch-all

Use `[[...path]]` to match zero or more segments. `params.path` is `string[] | undefined`.

```
app/
  [[...slug]]/
    page.tsx  →  catches everything (including /)
```

| URL         | `params.path`     |
| ----------- | ----------------- |
| `/`         | `undefined`       |
| `/anything` | `["anything"]`    |
| `/a/b/c`    | `["a", "b", "c"]` |

**Edge case:** When `params.path` is `undefined` at the root route, the property is absent from the
params object entirely (not `[]`). This matches the server router behavior and the client-side
matcher.

---

## Matching Algorithm (Radix Trie Implementation)

Ruvyxa's router uses a segment-based regex matcher, not a radix trie. Each route pattern is compiled
into a regular expression at discovery time.

### `compilePattern` — Pattern to Regex

Source: `packages/@ruvyxa/react/src/route-match.ts`

```ts
interface CompiledPattern {
  regex: RegExp
  paramNames: string[]
  catchAll: { name: string; optional: boolean } | null
}

function compilePattern(routePath: string): CompiledPattern {
  if (routePath === '/') {
    return { regex: /^\/$/, paramNames: [], catchAll: null }
  }

  const segments = routePath.split('/').filter(Boolean)
  const paramNames: string[] = []
  let catchAll: CompiledPattern['catchAll'] = null
  let pattern = '^'

  for (const segment of segments) {
    // [[...slug]] → optional catch-all → (?:/(.*))?
    if (/^\[\[\.\.\.(\w+)\]\]$/.test(segment)) { ... }
    // [...slug] → required catch-all → /(.+)
    if (/^\[\.\.\.(\w+)\]$/.test(segment)) { ... }
    // [slug] → dynamic → /([^/]+)
    if (/^\[(\w+)\]$/.test(segment)) { ... }
    // static → literal → /segment-name
    pattern += `/${escapeRegex(segment)}`
  }

  pattern += '/?$'  // optional trailing slash
  return { regex: new RegExp(pattern), paramNames, catchAll }
}
```

### Match Execution

The matcher sorts routes by **specificity** (static before dynamic before catch-all), then checks
each compiled regex in order:

```
static segments     → specificity 0
dynamic [slug]      → specificity 1
catch-all [...path] → specificity 2
optional [[...path]] → specificity 3
Lower number = higher priority
```

### Specificity Comparison

```ts
function compareSpecificity(left: number[], right: number[]): number {
  const length = Math.max(left.length, right.length)
  for (let index = 0; index < length; index++) {
    const leftScore = left[index] ?? -1
    const rightScore = right[index] ?? -1
    if (leftScore !== rightScore) return leftScore - rightScore
  }
  return 0
}
```

A shorter route sorts before a longer one when all segments have equal specificity.

### Path Normalization

Before matching, the request path is normalized:

```ts
function normalizeMatchPath(pathname: string): string {
  const segments = pathname.split('/').filter(Boolean)
  return segments.length === 0 ? '/' : `/${segments.join('/')}`
}
```

This collapses:

- `/docs/a/` → `/docs/a`
- `/docs//a` → `/docs/a`
- `/docs/a` → `/docs/a`

### Canonical Path Validation

```ts
function canonicalRoutePath(pathname: string): string | null {
  // Decode each segment exactly once
  // Reject empty segments, . or .. traversal, encoded slashes, control characters
  // Returns null for invalid paths → 404
}
```

---

## Matching Priority

When multiple files could match the same URL, Ruvyxa uses this priority order:

```
1. Static    /blog/about       (highest)
2. Dynamic   /blog/[slug]
3. Catch-all /blog/[...path]
4. Optional  /blog/[[...path]] (lowest)
```

Example: if you have both `app/blog/about/page.tsx` and `app/blog/[slug]/page.tsx`, the static route
wins for `/blog/about`.

```
Priority diagram:

  URL: /blog/about
         ↓
  ┌─────────────────────────────┐
  │ app/blog/about/page.tsx     │  ← Static wins (exact match)
  │   (matched first)           │
  ├─────────────────────────────┤
  │ app/blog/[slug]/page.tsx    │  ← Dynamic would also match
  │   (skipped — static exists) │
  └─────────────────────────────┘
```

---

## Route Validation — Error Codes (RUV1100-1199)

| Code      | Message                       | When it fires                                                                          |
| --------- | ----------------------------- | -------------------------------------------------------------------------------------- |
| `RUV1001` | App directory not found       | `app/` doesn't exist or `appDir` config points to nonexistent path                     |
| `RUV1002` | Invalid dynamic route segment | Segment name has invalid characters, or catch-all `[...path]` is not the final segment |
| `RUV1002` | Catch-all must be final       | `[...slug]` or `[[...slug]]` followed by more segments                                 |
| `RUV1003` | Conflicting route paths       | Two files resolve to the same URL pattern                                              |
| `RUV1004` | Page missing default export   | `page.tsx`/`page.jsx` doesn't have `export default function`                           |
| `RUV1100` | React SSR failed              | Error during server-side rendering                                                     |
| `RUV1102` | SSR renderer not found        | Page module missing render                                                             |

### RUV1002 — Invalid Dynamic Segment Causes

```
// ❌ Invalid: segment name with hyphens
app/blog/[my-slug]/page.tsx → RUV1002

// ❌ Invalid: segment with special chars
app/blog/[slug!]/page.tsx → RUV1002

// ❌ Invalid: catch-all not final
app/docs/[...path]/[id]/page.tsx → RUV1002

// ✅ Valid: alphanumeric and underscore only
app/blog/[slug]/page.tsx
app/blog/[article_id]/page.tsx
```

### RUV1003 — Ambiguous Route Resolution

```
RUV1003: Conflicting route paths
  /about matched by:
    - app/about/page.tsx
    - app/[slug]/page.tsx
  Remove or rename one of them.
```

This fires when two routes with the same specificity level match the same URL. For example, two
static routes or two dynamic routes with the same pattern cannot coexist.

---

## Layout Nesting

Every folder can have a `layout.tsx`. Layouts wrap child pages and **persist** across client-side
navigations.

```
app/
├── layout.tsx         ← wraps everything
├── page.tsx           → /
├── blog/
│   ├── layout.tsx     ← wraps /blog/* pages
│   ├── page.tsx       → /blog
│   └── [slug]/
│       └── page.tsx   → /blog/:slug
```

```tsx
// app/layout.tsx — root layout (required)
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        <header>Site Header</header>
        {children}
        <footer>Site Footer</footer>
      </body>
    </html>
  )
}
```

```tsx
// app/blog/layout.tsx — blog section layout
export default function BlogLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="blog-layout">
      <aside>Blog Sidebar</aside>
      <main>{children}</main>
    </div>
  )
}
```

### LayoutProps Type Definition

```ts
export interface LayoutProps<TParams extends RouteParams = RouteParams> {
  children: React.ReactNode
  params: TParams
}
```

Usage:

```tsx
export default function BlogLayout({ children, params }: LayoutProps<{ slug: string }>) {
  return <div>{children}</div>
}
```

### Layout Chain Resolution

The layout chain is built by walking from the root `app/layout.tsx` down to the route's parent
folder. For `/blog/hello-world`:

```
Layout chain (bottom-up):
  1. app/blog/[slug]/page.tsx   (page content)
  2. app/blog/layout.tsx        (blog section layout)
  3. app/layout.tsx             (root layout)
```

### Layout Inheritance Rules

| Rule                                    | Behavior                                                                       |
| --------------------------------------- | ------------------------------------------------------------------------------ |
| **Nearest layout wins**                 | A layout only wraps routes in its folder and subfolders                        |
| **Missing layout**                      | If a folder has no `layout.tsx`, the parent layout wraps children directly     |
| **Root layout required**                | `app/layout.tsx` is mandatory and must contain `<html>` and `<body>`           |
| **Route group layouts**                 | `(group)/layout.tsx` only wraps routes inside that group                       |
| **No layout inheritance across groups** | Layouts in `(marketing)` don't affect `(dashboard)` even at same nesting level |

Layout nesting pattern:

```
URL: /blog/hello-world
                         ┌─────────────────┐
                         │ Root layout      │
                         │  ├ header        │
                         │  ├──────────────┤
                         │  │ Blog layout   │
                         │  │  ├ sidebar    │
                         │  │  ├───────────│
                         │  │  │ Hello Post │
                         │  │  └───────────│
                         │  │  └ sidebar    │
                         │  └──────────────┤
                         │  └ footer        │
                         └─────────────────┘
```

---

## Page Metadata

Export a `meta` object or function for SEO metadata.

### `Meta` Type — All Fields

```ts
export interface Meta {
  /** Document title. Formatted by the nearest ancestor `titleTemplate`. */
  title?: string
  /**
   * Format applied to titles declared *below* this level; `%s` is the child title.
   * A level's own `title` is never formatted by its own template.
   */
  titleTemplate?: string
  /** Meta description. Required for a perfect Lighthouse SEO score. */
  description?: string
  /** Absolute canonical URL for this document. */
  canonical?: string
  /** Written verbatim into `<meta name="robots">`. Overrides `noindex`. */
  robots?: string
  /** Shorthand for `robots: 'noindex, nofollow'`. */
  noindex?: boolean
  /** Document language, written onto the `<html lang>` attribute. */
  lang?: string
  /** `<link rel="alternate" hreflang>` entries for other locales. */
  alternates?: readonly MetaAlternate[]
  /** Preview image URL used by X and Open Graph consumers. */
  image?: string
  /** Alternative text for `image`. */
  imageAlt?: string
  /** `og:site_name`. */
  siteName?: string
  /** `og:type`. @default "website" */
  type?: 'website' | 'article' | 'profile'
  /** `og:locale`, e.g. `th_TH`. */
  locale?: string
  /** Twitter card type. @default "summary_large_image" when image is set. */
  card?: 'summary' | 'summary_large_image'
}

export interface MetaAlternate {
  /** BCP 47 tag, or `x-default`. */
  hreflang: string
  /** Absolute URL of the alternate document. */
  href: string
}

export interface MetaContext {
  /** Concrete request path, e.g. `/blog/hello`. */
  path: string
  /** Dynamic segment values for this request. */
  params: Record<string, string>
}

export type MetaFactory = (context: MetaContext) => Meta
export type MetaExport = Meta | MetaFactory
```

### Static Meta Example

```tsx
// app/blog/[slug]/page.tsx
import type { Metadata } from 'ruvyxa'

export const meta: Metadata = {
  title: 'My Blog',
  description: 'A blog about things',
  canonical: 'https://example.com/blog',
  robots: 'index, follow',
  openGraph: {
    title: 'My Blog',
    image: '/og-image.png',
  },
}

export default function BlogPage() {
  return <h1>Blog</h1>
}
```

### Dynamic Meta from Params

```tsx
export const meta = ({ params }: { params: { slug: string } }): Metadata => ({
  title: `Post: ${params.slug}`,
  description: `Read about ${params.slug}`,
  canonical: `https://example.com/blog/${params.slug}`,
  alternates: {
    canonical: `/blog/${params.slug}`,
  },
})
```

### Meta Merge Algorithm

When multiple layouts and the page export `meta`, they are merged with this algorithm:

```
1. Start with root layout `meta` (if any)
2. For each ancestor layout (closest to root → closest to page):
   └─ Deep merge: page-level fields overwrite layout-level fields
   └─ Arrays (alternates): fully replaced, not concatenated
   └─ `titleTemplate`: nearest ancestor wins, applied to page title
3. Page `meta` wins for all fields
```

**Specific rules:**

| Field           | Merge behavior                                                |
| --------------- | ------------------------------------------------------------- |
| `title`         | Most specific (page > layout > root) wins                     |
| `titleTemplate` | Applied to page title by the nearest ancestor that defines it |
| `description`   | Most specific wins                                            |
| `canonical`     | Most specific wins                                            |
| `robots`        | Most specific wins                                            |
| `noindex`       | Most specific wins                                            |
| `alternates`    | Page value replaces all layout alternates                     |
| `image`         | Most specific wins                                            |
| `lang`          | Root layout value — not changed by client navigation          |

**Example merge:**

```tsx
// app/layout.tsx
export const meta = {
  titleTemplate: '%s — My Site',
  siteName: 'My Site',
}

// app/blog/layout.tsx
export const meta = {
  titleTemplate: '%s — Blog',
}

// app/blog/[slug]/page.tsx
export const meta = ({ params }) => ({
  title: params.slug,
  description: `Read about ${params.slug}`,
})

// Result:
// title: "hello-world — Blog"
// siteName: "My Site"           (inherited from root)
// description: "Read about hello-world" (from page)
```

### Meta Fields Supported Table

| Field           | Type              | Description             | Rendered as                                        |
| --------------- | ----------------- | ----------------------- | -------------------------------------------------- |
| `title`         | `string`          | Page title              | `<title>`                                          |
| `titleTemplate` | `string`          | Format string with `%s` | Applied to child title                             |
| `description`   | `string`          | Meta description        | `<meta name="description">`                        |
| `canonical`     | `string`          | Canonical URL           | `<link rel="canonical">`                           |
| `robots`        | `string`          | Robots directive        | `<meta name="robots">`                             |
| `noindex`       | `boolean`         | Shorthand noindex       | `<meta name="robots" content="noindex, nofollow">` |
| `lang`          | `string`          | Document language       | `<html lang="...">`                                |
| `alternates`    | `MetaAlternate[]` | hreflang entries        | `<link rel="alternate" hreflang="..." href="...">` |
| `image`         | `string`          | OG image URL            | `<meta property="og:image">`                       |
| `imageAlt`      | `string`          | OG image alt text       | `<meta property="og:image:alt">`                   |
| `siteName`      | `string`          | OG site name            | `<meta property="og:site_name">`                   |
| `type`          | `string`          | OG type                 | `<meta property="og:type">`                        |
| `locale`        | `string`          | OG locale               | `<meta property="og:locale">`                      |
| `card`          | `string`          | Twitter card type       | `<meta name="twitter:card">`                       |

---

## Route Groups

Use `(name)` folders to organize routes without affecting the URL.

```
app/
├── (marketing)/
│   ├── page.tsx           → /
│   └── about/
│       └── page.tsx       → /about
├── (dashboard)/
│   ├── dashboard/
│   │   └── page.tsx       → /dashboard
│   └── settings/
│       └── page.tsx       → /settings
└── layout.tsx             ← root layout
```

### Layout Isolation Semantics

Route groups provide **layout isolation**:

- Each group can have its own `layout.tsx` that only applies to routes inside that group
- Layouts from one group **never** apply to routes in another group, even at the same nesting level
- Groups do not affect URL structure — `(marketing)/about/page.tsx` is just `/about`

### Multiple Groups at Same Level

```
app/
├── (marketing)/
│   ├── layout.tsx        ← applies to (marketing) routes only
│   ├── page.tsx          → /
│   └── about/
│       └── page.tsx      → /about
├── (dashboard)/
│   ├── layout.tsx        ← applies to (dashboard) routes only
│   └── settings/
│       └── page.tsx      → /settings
├── layout.tsx             ← root layout wraps all
```

Route groups are useful for:

- Different layouts for different sections
- Organizing many routes without deep nesting
- Separate loading or error UIs per section

---

## Ignored Folders

Folders starting with `_` or `@` are **ignored** by the router.

```
app/
├── _components/     ← ignored (put shared components here)
│   └── Header.tsx
├── @shared/         ← ignored (put shared utilities here)
│   └── utils.ts
└── page.tsx         ← still /
```

### Private Folders (`_` prefix)

The `_` prefix creates a **private folder**. Files inside `_components/`, `_utils/`, `_lib/` are:

- **Not routable** — ignored by file-system walker
- **Not included** in route manifests
- **Safe for colocation** — keep shared components, types, utilities in the same `app/` subtree

The walker filter in Rust:

```rust
.filter_entry(|entry| {
    let name = entry.file_name().to_string_lossy();
    !name.starts_with('_') && !name.starts_with('@')
})
```

### Parallel Slots (`@` prefix)

The `@` prefix is also **ignored** by the router. Unlike Next.js, Ruvyxa does not use `@` for
parallel routing slots. `@` prefixed folders function identically to `_` private folders:

```
app/
├── @modal/           ← ignored (no parallel slot behavior)
│   └── dialog.tsx
├── @feed/            ← ignored
│   └── feed.tsx
└── page.tsx
```

---

## Internationalization with Route Groups

Use route groups to organize locale-specific routes:

```
app/
├── (en)/
│   ├── layout.tsx         ← English layout
│   ├── page.tsx           → / (English home)
│   └── about/
│       └── page.tsx       → /about (English)
├── (th)/
│   ├── layout.tsx         ← Thai layout
│   ├── page.tsx           → / (Thai home)
│   └── about/
│       └── page.tsx       → /about (Thai)
└── layout.tsx             ← root layout (lang switcher, etc.)
```

This pattern allows:

- Separate layouts per locale
- Same URL structure
- Locale-specific meta (lang, alternates)

---

## Client Navigation

### `<Link>` Component

Use `<Link>` from `@ruvyxa/react` for client-side navigation. It prefetches pages in the background.

```tsx
import { Link } from '@ruvyxa/react'

export default function Nav() {
  return (
    <nav>
      <Link href="/">Home</Link>
      <Link href="/about">About</Link>
      <Link href="/blog/hello-world">Blog Post</Link>
    </nav>
  )
}
```

### `LinkProps` Type

```ts
export type LinkPrefetch = boolean | 'hover' | 'viewport' | 'none'

export interface LinkProps extends Omit<AnchorHTMLAttributes<HTMLAnchorElement>, 'href'> {
  /** Destination URL. Relative paths resolve against the current document. */
  href: string
  /** Replace the current history entry instead of pushing a new one. */
  replace?: boolean
  /** Scroll to the top after navigating. Defaults to `true`. */
  scroll?: boolean
  /**
   * Warm the destination bundle ahead of the click.
   * `"hover"` (default): prefetch on pointer enter or focus
   * `"viewport"`: prefetch when link enters viewport (200px margin)
   * `false` / `"none"`: no prefetch
   * `true`: same as "hover"
   */
  prefetch?: LinkPrefetch
  children?: ReactNode
  ref?: Ref<HTMLAnchorElement>
}
```

### Prefetch Behavior Table

| Prop                         | Behavior                                  | Implementation                                                   |
| ---------------------------- | ----------------------------------------- | ---------------------------------------------------------------- |
| `prefetch="hover"` (default) | Prefetch on mouse enter or keyboard focus | `onMouseEnter` / `onFocus` handler calls `router.prefetch(href)` |
| `prefetch="viewport"`        | Prefetch when link enters viewport        | Uses `IntersectionObserver` with 200px `rootMargin`              |
| `prefetch={true}`            | Same as `"hover"`                         | Identical to hover behavior                                      |
| `prefetch={false}`           | No prefetch                               | Skipped                                                          |
| `prefetch="none"`            | No prefetch                               | Skipped                                                          |

### Prefetch Implementation

The `<Link>` component uses `router.prefetch()` which:

1. Resolves the URL against the current document
2. Fetches the client route manifest (lazy, one-time)
3. Matches the URL against route patterns
4. If route bundle not already loaded, appends `<link rel="modulepreload">` for the route's entry
   and shared chunks

```ts
function prefetch(href: string): void {
  const url = resolveInternalUrl(href)
  if (!url) return
  void ensureManifest().then(() => {
    const matched = match(url.pathname)
    if (!matched?.route.src) return
    if (globals.__RUVYXA_ROUTES__?.[matched.route.path]) return
    if (!preloadModule(matched.route.src)) return
    for (const chunk of matched.route.sharedChunks ?? []) {
      preloadModule(chunk.src)
    }
  })
}
```

### Browser Navigation Handling

The `<Link>` component respects browser-native behavior:

```ts
function shouldLetBrowserHandle(event: MouseEvent, target?: string): boolean {
  if (event.defaultPrevented) return true
  if (event.button !== 0) return true // non-primary button
  if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return true
  if (target && target !== '_self') return true // external target
  return false
}
```

Links are **progressive enhancements**: they render as real `<a href>` elements, work without
JavaScript, crawlable by search engines, and middle-clickable.

### Navigation Hooks

```tsx
'use client'

import { useRouter, usePathname, useParams, useSearchParams } from '@ruvyxa/react'

export default function NavButtons() {
  const router = useRouter()
  const pathname = usePathname()
  const params = useParams()
  const searchParams = useSearchParams()

  return (
    <div>
      <p>Current path: {pathname}</p>
      <p>Search: {searchParams.get('q')}</p>
      <button onClick={() => router.push('/about')}>Go to About</button>
      <button onClick={() => router.back()}>Back</button>
    </div>
  )
}
```

### Hook Type Signatures

#### `useRouter()`

```ts
export interface RuvyxaRouter {
  /** Navigate to a new URL. Adds history entry. */
  push(href: string, options?: NavigateOptions): Promise<void>
  /** Navigate to a new URL. Replaces current history entry. */
  replace(href: string, options?: NavigateOptions): Promise<void>
  /** Navigate back in history. */
  back(): void
  /** Navigate forward in history. */
  forward(): void
  /** Re-render the current route from its already-loaded bundle. */
  refresh(): void
  /** Warm a route's bundle so a later navigation renders immediately. */
  prefetch(href: string): void
  /** `true` while a navigation is loading a bundle. */
  readonly pending: boolean
}

export interface NavigateOptions {
  /** Replace the current history entry instead of pushing a new one. */
  replace?: boolean
  /** Scroll to the top after navigating. Defaults to `true`. */
  scroll?: boolean
}
```

#### `usePathname()`

```ts
function usePathname(): string
// Returns current pathname, e.g. "/blog/hello-world"
// No search string or hash. Server-safe.
```

#### `useParams()`

```ts
function useParams(): RouteParams
// RouteParams = Record<string, string | string[] | undefined>
// e.g. { slug: "hello-world" } or { path: ["guide", "routing"] }
```

#### `useSearchParams()`

```ts
function useSearchParams(): URLSearchParams
// Returns empty set during SSR (server can't reliably know query string)
// Real values after hydration
// New instance per render (immutable pattern)
```

#### `useSelectedRoute()`

```ts
function useSelectedRoute(): string
// Returns matched route pattern, e.g. "/blog/[slug]"
```

#### `useRouteContext()`

```ts
interface RouteContextValue {
  pathname: string
  params: RouteParams
  route: string
}

function useRouteContext(): RouteContextValue
```

### Hook Behavior Edge Cases

| Hook                  | SSR value                     | Client initial value       | After navigation          |
| --------------------- | ----------------------------- | -------------------------- | ------------------------- |
| `usePathname()`       | `requestPath` from server     | `window.location.pathname` | Updated                   |
| `useParams()`         | From server render            | From route match           | Updated                   |
| `useSearchParams()`   | Empty (`URLSearchParams('')`) | From `location.search`     | Updated                   |
| `useSelectedRoute()`  | Pattern from server           | From route match           | Updated                   |
| `useRouter().pending` | `false`                       | `false`                    | `true` during bundle load |

---

## Router Internals — Under the Hood

### Route Context (`__RUVYXA_ROUTE_CONTEXT__`)

The route context is stored on `globalThis` rather than in a module import. This ensures the
generated route entry (which cannot import `@ruvyxa/react`) and the app's copy share one instance.

```ts
const CONTEXT_KEY = '__RUVYXA_ROUTE_CONTEXT__'
const store = globalThis as unknown as Record<string, unknown>
const existing = store[CONTEXT_KEY]
if (existing) return existing
const created = createContext<RouteContextValue | null>(null)
store[CONTEXT_KEY] = created
```

### Globals Contract

The router reads/writes these globals:

| Global                       | Type                               | Purpose                                              |
| ---------------------------- | ---------------------------------- | ---------------------------------------------------- |
| `__RUVYXA_ROUTES__`          | `Record<string, TreeFactory>`      | Route tree factories, registered by bundle execution |
| `__RUVYXA_ROOT__`            | `{ render(tree): void }`           | Ruvyxa root renderer (hydrate or re-render)          |
| `__RUVYXA_ROUTE_PARAMS__`    | `RouteParams`                      | Current route parameters                             |
| `__RUVYXA_REQUEST_PATH__`    | `string`                           | Current request path                                 |
| `__RUVYXA_ROUTE_PATTERN__`   | `string`                           | Route pattern the current bundle registered under    |
| `__RUVYXA_ROUTE_MANIFEST__`  | `{ routes: RouteManifestEntry[] }` | Inline client route table                            |
| `__RUVYXA_ROUTER_INSTANCE__` | `RouterInstance`                   | Singleton router instance                            |

### Route Manifest (Client)

Published at `__ruvyxa/client/route-manifest.json`:

```json
{
  "routes": [
    {
      "path": "/",
      "src": "/__ruvyxa/client/pages/index.mjs",
      "sharedChunks": [{ "src": "/__ruvyxa/client/shared/vendor.mjs" }],
      "strategy": "ssg"
    },
    {
      "path": "/blog/[slug]",
      "src": "/__ruvyxa/client/pages/blog/[slug].mjs",
      "sharedChunks": [],
      "strategy": "isr"
    }
  ]
}
```

This manifest is fetched lazily (first client navigation) and cached. It contains only
`{ path, src, sharedChunks }` — no absolute source paths or module graphs.

### Navigation Flow

```
1. User clicks <Link href="/blog/hello">
2. Link calls event.preventDefault()
3. Router.resolveInternalUrl() checks: same origin? http/https?
4. Router.ensureManifest() — lazy-fetches route table if needed
5. Router.match(url.pathname) — compiled regex matching
6. If no match → hard navigation (browser handles it)
7. If route bundle not loaded → router.loadRoute() → dynamic import()
8. window.history.pushState()
9. router.renderRoute() → __RUVYXA_ROOT__.render()
10. Scroll to top (unless scroll: false)
```

---

## Route Manifest (Build Output)

The full route manifest is written during build and contains:

```json
{
  "appDir": "app",
  "routes": [
    {
      "id": "/",
      "path": "/",
      "kind": "page",
      "file": "app/page.tsx",
      "layoutChain": ["app/layout.tsx"],
      "serverModules": ["app/actions.ts"],
      "clientModules": ["app/client.tsx"],
      "runtime": "node",
      "render": {
        "strategy": "ssg",
        "revalidate": null,
        "hasStaticParams": false,
        "staticPaths": [],
        "hasDynamicSlots": false,
        "hydrate": true,
        "hydration": "load"
      }
    }
  ]
}
```

### Route Entry Schema

| Field           | Type                           | Description                                    |
| --------------- | ------------------------------ | ---------------------------------------------- |
| `id`            | `string`                       | Human-readable route identifier                |
| `path`          | `string`                       | URL pattern (e.g., `/blog/[slug]`)             |
| `kind`          | `"page" \| "api"`              | Route type                                     |
| `file`          | `string`                       | Source file path                               |
| `layoutChain`   | `string[]`                     | Ordered ancestor layout files                  |
| `serverModules` | `string[]`                     | Server-only modules (`server.ts`, `action.ts`) |
| `clientModules` | `string[]`                     | Client entry modules                           |
| `runtime`       | `"node" \| "edge" \| "static"` | Runtime target                                 |
| `render`        | `RenderMeta`                   | Rendering strategy metadata                    |

---

## Route Validation

When you run `ruvyxa check` or `ruvyxa dev`, Ruvyxa validates your route tree. Ambiguous routes
produce an error.

```
RUV1003: Ambiguous route
  /about matched by:
    - app/about/page.tsx
    - app/[slug]/page.tsx
  Remove or rename one of them.
```

Run `npm run routes` to see the resolved route table:

```
┌──────────────────────────────────────┬──────────────────┬──────────┐
│ URL                                  │ File             │ Type     │
├──────────────────────────────────────┼──────────────────┼──────────┤
│ /                                    │ app/page.tsx     │ page     │
│ /about                               │ app/about/page.tsx│ page    │
│ /blog                                │ app/blog/page.tsx │ page    │
│ /blog/[slug]                         │ app/blog/[slug]/ │ page    │
│                                      │   page.tsx       │          │
│ /api/hello                           │ app/api/hello/   │ api      │
│                                      │   route.ts       │          │
└──────────────────────────────────────┴──────────────────┴──────────┘
```

---

## Common Mistakes

| Mistake                                 | Why it fails                                      | Fix                                    |
| --------------------------------------- | ------------------------------------------------- | -------------------------------------- |
| `[slug]` inside `(group)` folder        | Group does not affect params, but nesting is fine | Keep `[slug]` in normal folders        |
| Two files at same URL                   | `RUV1003` ambiguous route                         | Delete or rename one                   |
| `layout.tsx` missing `<html>`           | Root layout must have `<html>` and `<body>`       | Add them to root layout                |
| `page.tsx` in `_components/`            | Underscore folders are ignored                    | Move `page.tsx` outside                |
| Accessing `params` async                | `params` is synchronous in Ruvyxa                 | Use `params.slug` directly             |
| `meta` as a function returns wrong type | Type mismatch in dynamic metadata                 | Ensure return type is `Metadata`       |
| `<Link>` without `@ruvyxa/react` import | Unknown component                                 | Import `Link` from `@ruvyxa/react`     |
| Forgot `'use client'` for hooks         | `useState`/`useRouter` on server                  | Add directive or use `useRuvyxaLoader` |
| Using `[...slug]` not at end of path    | `RUV1002`: catch-all must be final                | Move to last segment                   |
| Hyphen in dynamic segment name          | `RUV1002`: invalid chars                          | Use underscores: `[article_id]`        |
| Two `layout.tsx` files in same folder   | Not allowed — one per folder                      | Remove duplicate                       |

---

## Performance Characteristics

| Operation           | Complexity                            | Notes                                 |
| ------------------- | ------------------------------------- | ------------------------------------- |
| Route discovery     | `O(n)` where `n` = files in `app/`    | Single recursive walk                 |
| Route matching      | `O(r)` where `r` = number of routes   | Linear scan of compiled regexes       |
| Specificity sort    | `O(r log r)`                          | One-time at build/discovery           |
| Compiled regex size | `O(s)` where `s` = segments per route | One capture group per dynamic segment |
| Manifest JSON size  | `O(r)`                                | Proportional to route count           |

---

## Security Implications

### Path Traversal Protection

The `canonicalRoutePath` function rejects:

- Empty segments
- `.` and `..` traversal segments
- Encoded slashes (`%2F`, `%5C`)
- Control characters (U+0000–U+001F, U+007F–U+009F)

### URL Decoding Safety

Segments are decoded exactly once with `decodeURIComponent`. Double-encoded paths are caught and
rejected, preventing path traversal via `%252F` encoding tricks.

### Link Security

The `<Link>` component only intercepts same-origin `http:`/`https:` navigations. External links,
`mailto:`, `tel:`, and downloads pass through untouched.

---

## Try It Yourself

Create this route structure and visit each URL:

```
app/
├── page.tsx
├── blog/
│   ├── page.tsx
│   └── [slug]/
│       └── page.tsx
├── docs/
│   └── [...path]/
│       └── page.tsx
└── (shop)/
    ├── layout.tsx
    ├── products/
    │   └── page.tsx
    └── cart/
        └── page.tsx
```

Expected routes:

```
/                  → app/page.tsx
/blog              → app/blog/page.tsx
/blog/my-post      → app/blog/[slug]/page.tsx
/docs              → app/docs/[...path]/page.tsx
/docs/guide        → app/docs/[...path]/page.tsx
/docs/guide/routing → app/docs/[...path]/page.tsx
/products          → app/(shop)/products/page.tsx
/cart              → app/(shop)/cart/page.tsx
```

Run `npm run routes` to verify.

---

## The Route-discovery Contract

Route discovery is intentionally file-based and narrow. Under the configured `appDir` (normally
`app`), the current implementation recognizes these route entry files:

| File name              | Route kind | Notes                                                                         |
| ---------------------- | ---------- | ----------------------------------------------------------------------------- |
| `page.tsx`, `page.jsx` | Page       | A JavaScript/TypeScript page must provide a default export.                   |
| `page.md`, `page.mdx`  | Page       | Compiled as a content page; the content compiler supplies the page component. |
| `route.ts`, `route.js` | API route  | Named HTTP-method exports are invoked by the API renderer.                    |

Directories whose name starts with `_` or `@` are ignored while walking the app tree. A parenthesis
group such as `(marketing)` is traversed but omitted from the URL. That makes it an organization
tool, not a URL segment:

```text
app/(marketing)/pricing/page.tsx  ->  /pricing
app/(marketing)/layout.tsx        ->  participates in the layout chain
```

### Dynamic Segment Rules, Including the Failure Case

Use `[name]` for one segment, `[...name]` for one-or-more remaining segments, and `[[...name]]` for
an optional catch-all. A catch-all must be the final visible segment because it consumes the rest of
the path.

```text
app/products/[id]/page.tsx              -> /products/[id]
app/docs/[...parts]/page.tsx             -> /docs/[...parts]
app/docs/[[...parts]]/page.tsx           -> /docs/[[...parts]]
app/docs/[...parts]/edit/page.tsx        -> invalid: catch-all is not final
```

The parameter name may not be empty, contain another bracket, or start with `.`. Those are route
discovery errors, so fix the directory name before debugging rendering code.

### Layouts and Route-local Modules

For each route, Ruvyxa collects `layout.tsx` from the application root down to that route's
directory. A route directory may also contain `server.ts`/`server.js`, `action.ts`/`action.js`, and
`client.tsx`; those files become part of the route manifest's server or client module lists. Keep
route-local concerns together when they are used by one route, and move broadly shared code outside
the route directory.

```text
app/blog/[slug]/
  layout.tsx       # adds a nested layout for this branch
  page.tsx         # page entry
  action.ts        # actions associated with this page route
  client.tsx       # route-local client entry, if needed
```

### Verify the Manifest Before Assuming a URL Exists

Use the CLI, not an assumed package script, to inspect discovery:

```bash
ruvyxa routes
ruvyxa trace /blog/[slug]
```

`routes` prints the discovered table. `trace` accepts the route pattern shown by the table rather
than a component filename or a concrete parameter value. If a new directory does not appear, first
check its entry-file name and whether an ancestor directory starts with `_` or `@`.

---

## Next Steps

- **[03-server-client-components.md](./03-server-client-components.md)** — Server vs client
  components
- **[04-rendering-strategies.md](./04-rendering-strategies.md)** — SSR, SSG, ISR, PPR, CSR
- **[06-server-actions.md](./06-server-actions.md)** — Server actions for mutations
- **[07-api-routes.md](./07-api-routes.md)** — Building API endpoints with `route.ts`
