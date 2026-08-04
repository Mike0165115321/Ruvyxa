# UI, navigation, metadata, and assets

`@ruvyxa/react` exports framework-aware React helpers. They are optional; normal React components
continue to work.

## Navigation and route state

Use `Link` for application navigation and `useRouter()` for imperative navigation. `usePathname()`,
`useParams()`, `useSearchParams()`, `useSelectedRoute()`, and `useRouteContext()` expose the current
client route state.

```tsx
'use client'
import { Link, useRouter, useSearchParams } from '@ruvyxa/react'

export function SearchControls() {
  const router = useRouter()
  const query = useSearchParams().get('q') ?? ''
  return (
    <>
      <Link href="/about">About</Link>
      <button onClick={() => router.push(`/search?q=${query}`)}>Search</button>
    </>
  )
}
```

`useSearchParams()` returns an empty set during SSR when the query is unavailable; do not use it for
markup that must be identical in server HTML. `useRouter().pending` tracks a route-bundle
navigation.

### Choose prefetch deliberately

`Link` renders a normal anchor first, then enhances eligible same-window clicks. It preserves new
tab, modified-click, download, and non-`_self` link behavior. Its `prefetch` default is `'hover'`.
Choose the mode by the likelihood and cost of the next navigation rather than enabling eager
prefetching everywhere.

```tsx
import { Link } from '@ruvyxa/react'

export function ProductLinks() {
  return (
    <nav>
      {/* The default: warm only when a visitor shows intent. */}
      <Link href="/products/notebook">Notebook</Link>

      {/* Good for a prominent next step likely to enter the viewport. */}
      <Link href="/checkout" prefetch="viewport">
        Checkout
      </Link>

      {/* Avoid warming a large, low-probability destination. */}
      <Link href="/reports" prefetch="none">
        Reports
      </Link>

      {/* Replace a transient URL; keep scroll position if the view needs it. */}
      <Link href="/search?q=paper" replace scroll={false}>
        Apply filter
      </Link>

      {/* Keep external destinations as ordinary anchors. */}
      <a href="https://status.example.com" target="_blank" rel="noreferrer">
        Status
      </a>
    </nav>
  )
}
```

Use `prefetch="viewport"` sparingly on above-the-fold or clearly next-step links; it loads a route
when its link becomes visible. Use `'none'` (or `false`) for low-intent destinations. `replace`
replaces the current history entry, `scroll` defaults to `true`, and `viewTransition` opts into the
browser View Transitions API when available.

## Metadata and error UI

Use a route `meta` export for hierarchy-aware metadata ([Routing](04-routing-rendering.md)), or use
`<Seo>` inside a component for per-render tags. `<Seo>` can emit Open Graph, X card, Article
JSON-LD, breadcrumb JSON-LD, and custom JSON-LD. Its `twitterCard` prop is deprecated in favor of
`card`.

```tsx
import { Seo, RuvyxaErrorBoundary } from '@ruvyxa/react'

export default function Product() {
  return (
    <RuvyxaErrorBoundary
      fallback={({ error, resetError }) => (
        <button onClick={resetError}>Retry: {error.message}</button>
      )}
    >
      <Seo
        title="Product"
        description="A documented product"
        canonical="https://example.test/product"
      />
      <main>...</main>
    </RuvyxaErrorBoundary>
  )
}
```

`RuvyxaErrorBoundary` catches descendant React render errors, calls optional `onError`, and passes
`resetError` to its fallback. It does not replace route-level `error.tsx` boundaries.

## Images, CSS, and static files

`Image` accepts React image props plus Ruvyxa options. Local public PNG/JPEG assets are optimized to
WebP during a production build by default. `image.variantWidths` controls responsive variants;
`Image` uses those widths for local images when `sizes` is supplied. `image.onDemand` enables
same-origin runtime transformations at `/__ruvyxa/image` and has a default maximum width of 3840
when configured as an object.

```tsx
import { Image } from '@ruvyxa/react'
export function Hero() {
  return (
    <Image
      src="/hero.jpg"
      alt="Team at work"
      width={1200}
      height={630}
      sizes="(max-width: 768px) 100vw, 1200px"
      priority
    />
  )
}
```

Imported project CSS may live outside `app/`. To include global styles not imported by a module,
list project-relative files/directories in `css.entries`. The runtime recognizes Sass as a package
dependency; use styles that your build can resolve and run `npm run check` after changing
boundaries.

**Previous:** [Data, actions, and API routes](05-data-actions-api.md) · **Next:**
[Configuration and environment](07-configuration.md)
