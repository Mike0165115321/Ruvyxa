# Routing and rendering

Route discovery turns the file tree into a manifest. Run `pnpm routes` while developing to inspect
it; use `pnpm routes:json` when a script needs machine-readable output. A page's strategy is
selected from its exports and the `render` configuration.

| Strategy | Selection evidenced in source             | When HTML is produced                                       |
| -------- | ----------------------------------------- | ----------------------------------------------------------- |
| SSR      | default, or `render.strategy: 'ssr'`      | every request                                               |
| SSG      | static route/static parameter discovery   | build time                                                  |
| ISR      | `export const revalidate = 60`            | build time, then revalidated after TTL                      |
| CSR      | `'use client'` page                       | browser after a minimal shell                               |
| PPR      | `export const ppr = true` with `Suspense` | static shell at build; dynamic slot streams at request time |

## Dynamic SSG

For a dynamic SSG/ISR page, export `getStaticParams`. It receives all discovered routes and the
current route description, and it returns objects (or a single-segment string/number shorthand). The
result can be wrapped with `{ params, cache }`, where `cache` accepts seconds or a string such as
`"10m"`.

```tsx
// app/blog/[slug]/page.tsx
import type { GetStaticParams, PageProps } from 'ruvyxa'

export const getStaticParams: GetStaticParams<{ slug: string }> = () => [
  { slug: 'first-post' },
  { slug: 'release-notes' },
]

export default function Post({ params }: PageProps<{ slug: string }>) {
  return (
    <article>
      <h1>{params.slug}</h1>
    </article>
  )
}
```

## Route metadata and boundaries

`export const meta` accepts a `Meta` object or `MetaFactory`. Layout metadata merges from root to
leaf; the most specific value wins. A lower-level title is formatted by the nearest ancestor
`titleTemplate`.

```tsx
// app/layout.tsx
import type { Meta } from '@ruvyxa/react'
export const meta: Meta = { titleTemplate: '%s — Example', siteName: 'Example' }

// app/blog/[slug]/page.tsx
export const meta = ({ params }: { params: Record<string, string> }) => ({
  title: params.slug,
  canonical: `https://example.test/blog/${params.slug}`,
})
```

`error.tsx` receives `{ error, reset }`; `loading.tsx` and `not-found.tsx` are plain components. To
select the nearest `not-found.tsx`, import `notFound` from `@ruvyxa/react` and call it (it throws a
tagged signal). Do not confuse it with `notFound` from `ruvyxa/server`, which creates an HTTP
`Response` with status 404.

## i18n route policy

`i18n.locales` and `i18n.defaultLocale` are configuration fields. Locale routing is file-system
based (for example `app/[lang]/about/page.tsx`); the default parameter name is `lang`. With locale
detection enabled, the server considers the configured cookie (default `RUVYXA_LOCALE`) and
`Accept-Language`.

**Previous:** [Project structure](03-project-structure.md) · **Next:**
[Data, actions, and API routes](05-data-actions-api.md)
