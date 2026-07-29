# Routing

> 🟢 **Beginner friendly** · ⏱️ ~7 min read
>
> **You'll learn:** how folders become URLs, pages with changing parts (`[slug]`), catch-all routes,
> and how to group files without affecting URLs.

Routes come directly from folders under `app/`. There is no route configuration file, no second
pattern to learn, and nothing to keep in sync: **the folder structure is the route table**.

## The mental model in 30 seconds

| You create                    | Browser URL    | What it is                    |
| ----------------------------- | -------------- | ----------------------------- |
| `app/page.tsx`                | `/`            | The home page                 |
| `app/about/page.tsx`          | `/about`       | A static page                 |
| `app/blog/[slug]/page.tsx`    | `/blog/hello`  | A dynamic page (`slug` param) |
| `app/docs/[...path]/page.tsx` | `/docs/a/b/c`  | A catch-all page              |
| `app/api/items/route.ts`      | `/api/items`   | An API endpoint (no HTML)     |
| `app/posts/intro/page.md`     | `/posts/intro` | A Markdown content page       |

A folder becomes a URL segment. A `page.tsx` (or `page.jsx`, `page.md`, `page.mdx`) inside it makes
that URL render a page; a `route.ts` makes it an API endpoint. Everything else in the folder is
private implementation detail.

## Your first routes

```text
app/
├── layout.tsx          → wraps every page
├── page.tsx            → /
├── about/
│   └── page.tsx        → /about
└── blog/
    ├── page.tsx        → /blog
    └── [slug]/
        └── page.tsx    → /blog/:slug
```

Every page default-exports a React component. Navigation between pages is plain HTML:

```tsx
// app/page.tsx
export default function Home() {
  return (
    <main>
      <h1>Welcome</h1>
      <a href="/about">About us</a>
    </main>
  )
}
```

## Dynamic Segments

`[name]` captures exactly one required segment as a `string`:

```tsx
// app/blog/[slug]/page.tsx → matches /blog/hello, not /blog or /blog/a/b
import type { PageProps } from 'ruvyxa/config'

export default function BlogPost({ params }: PageProps<{ slug: string }>) {
  return <h1>Post: {params.slug}</h1>
}
```

`[...path]` (catch-all) captures **one or more** remaining segments as `string[]`:

```tsx
// app/docs/[...path]/page.tsx → matches /docs/a and /docs/a/b/c, not /docs itself
export default function Docs({ params }: PageProps<{ path: string[] }>) {
  return <h1>{params.path.join('/')}</h1>
}
```

`[[...path]]` (optional catch-all) **also** matches the parent URL. Its value is `undefined` at the
parent and `string[]` when segments are present:

```tsx
// app/shop/[[...path]]/page.tsx → matches /shop AND /shop/clothes/shirts
export default function Shop({ params }: PageProps<{ path?: string[] }>) {
  return <h1>{params.path?.join('/') ?? 'All products'}</h1>
}
```

> **Note for Next.js users** — Ruvyxa keeps `params` synchronous. There is no Promise-based params
> API to await; `params.slug` is just a string.

## Matching priority

When several routes could match one URL, the most specific wins — always in this order:

1. **Static** segment (`app/blog/featured/`)
2. **Dynamic** segment (`app/blog/[slug]/`)
3. **Catch-all** (`app/blog/[...path]/`)
4. **Optional catch-all** (`app/blog/[[...path]]/`) — lowest priority

So `/blog/featured` renders the static page even when `[slug]` exists beside it. You never need to
order anything manually.

## Layouts nest automatically

Each folder may carry its own `layout.tsx`; layouts wrap from the outside in:

```text
app/layout.tsx            → wraps everything
app/blog/layout.tsx       → additionally wraps every /blog/* page
app/blog/[slug]/page.tsx  → rendered inside both layouts
```

```tsx
// app/blog/layout.tsx
export default function BlogLayout({ children }: { children: React.ReactNode }) {
  return (
    <section>
      <nav>Blog navigation</nav>
      {children}
    </section>
  )
}
```

## Page metadata

A page or layout can export `meta`. Ruvyxa merges every `meta` on the route — root layout first,
page last — and renders the result into `<head>`:

```tsx
// app/layout.tsx
import type { Meta } from '@ruvyxa/react'

export const meta: Meta = {
  titleTemplate: '%s · Acme',
  title: 'Acme',
  description: 'Everything Acme builds.',
  siteName: 'Acme',
  lang: 'en',
}
```

```tsx
// app/blog/[slug]/page.tsx
export const meta = ({ params }: { params: { slug: string } }) => ({
  title: params.slug,
  canonical: `https://acme.dev/blog/${params.slug}`,
})
```

`/blog/hello` gets `<title>hello · Acme</title>`, the layout's description and `og:site_name`, and
its own canonical URL. A level's own `title` is never formatted by its own `titleTemplate`, so the
layout above stays `Acme` at `/`.

| Field                                | Renders                                                              |
| ------------------------------------ | -------------------------------------------------------------------- |
| `title`, `titleTemplate`             | `<title>`, `og:title`, `twitter:title`                               |
| `description`                        | `<meta name="description">`, `og:description`, `twitter:description` |
| `canonical`                          | `<link rel="canonical">`, `og:url`                                   |
| `robots`, `noindex`                  | `<meta name="robots">`                                               |
| `lang`                               | the `lang` attribute of the server-rendered `<html>` element         |
| `alternates`                         | `<link rel="alternate" hreflang>` per entry                          |
| `image`, `imageAlt`                  | `og:image`, `twitter:image` and their alt tags                       |
| `siteName`, `type`, `locale`, `card` | the matching Open Graph and X preview tags                           |

`meta` may be an object or a **synchronous** function of `{ path, params }`. It is resolved during
render, so an async function would land after the streamed shell had already been sent without a
title.

Two limits worth knowing:

- `lang` is applied to the document each server render produces. Client-side navigation does not
  change it, so a locale-segmented tree (`app/[lang]/…`) gets the right value on every crawlable
  document but not on a soft navigation between locales.
- Do not set the same field in both `meta` and [`<Seo>`](official-packages.md). React hoists both
  `<title>` elements and the last one wins, which is rarely what you meant. Use `meta` for what a
  route always is, and `<Seo>` when the value only exists inside a component's render.

## Route Groups

Use `(name)` to organize files **without** adding a URL segment:

```text
app/(marketing)/pricing/page.tsx   → /pricing   (not /marketing/pricing)
app/(marketing)/contact/page.tsx   → /contact
app/(app)/dashboard/page.tsx       → /dashboard
```

Groups are ideal for giving different areas of a site different layouts — put a `layout.tsx` inside
each group folder.

## Ignored folders

Folders beginning with `_` or `@` are never routed. Use them for co-located helpers:

```text
app/blog/_components/PostCard.tsx   → not a route; import it from your pages
```

## Client-side navigation

A plain `<a href>` always works and triggers a full page load. To navigate between routes without
reloading the document, use `<Link>` from `@ruvyxa/react`:

```tsx
import { Link } from '@ruvyxa/react'

export default function Nav() {
  return (
    <nav>
      <Link href="/">Home</Link>
      <Link href="/blog/hello" prefetch="viewport">
        Hello
      </Link>
    </nav>
  )
}
```

`<Link>` renders a real `<a href>`, so it stays crawlable and works before hydration or with
JavaScript disabled — the soft navigation is a progressive enhancement. Modifier-clicks, new-tab
clicks, `target`, and `download` fall through to the browser untouched. Prefetch warms the target
bundle ahead of the click: `"hover"` (the default) on pointer or focus, `"viewport"` when the link
scrolls into view, or `false` to disable.

Read and control the current route with hooks:

```tsx
import { useRouter, usePathname, useParams, useSearchParams } from '@ruvyxa/react'

function Example() {
  const router = useRouter() // push / replace / back / forward / refresh / prefetch, plus `pending`
  const pathname = usePathname() // "/blog/hello"
  const params = useParams() // { slug: "hello" }
  const query = useSearchParams() // URLSearchParams

  return <button onClick={() => router.push('/about')}>About</button>
}
```

`useSearchParams` returns an empty set during server rendering and the real query string after
hydration — routing that must be identical in the server HTML belongs in the path, not the query.

A URL no client route owns (an API route, a redirect, a rewrite) falls back to a full document load,
so navigation never silently does nothing.

## Validation catches mistakes at build time

Ruvyxa rejects ambiguous route shapes instead of guessing, including dynamic siblings such as `[id]`
and `[slug]` in the same folder. Inspect what the framework discovered at any time:

```bash
npx ruvyxa routes    # print the resolved route table
npx ruvyxa analyze   # full graph analysis with rendering strategies
```

If a URL renders a 404 you did not expect, run `npx ruvyxa routes` first — the answer is almost
always a missing `page.tsx` or a folder name typo.

## Common beginner mistakes

| Symptom                            | Cause                                        | Fix                                        |
| ---------------------------------- | -------------------------------------------- | ------------------------------------------ |
| Folder exists but URL 404s         | No `page.tsx` inside the folder              | Add `page.tsx` (folders alone don't route) |
| `/blog` 404s but `/blog/x` works   | Only `[slug]/page.tsx` exists                | Add `app/blog/page.tsx` for the index      |
| Two files both claim one URL       | `page.tsx` and `route.ts` in the same folder | Keep one — a URL is a page or an endpoint  |
| Build error about ambiguous routes | `[id]` and `[slug]` siblings                 | Use one dynamic name per folder level      |

## Next steps

- [API Routes](api-routes.md) — `route.ts` handlers receive the same parameter shapes
- [Rendering Strategies](rendering-strategies.md) — make any route SSG, ISR, CSR, or PPR
- [Markdown, MDX, Images & Metadata](markdown-mdx-images.md) — `page.md` / `page.mdx` content routes
