/**
 * Types for the route metadata a page or layout declares with `export const meta`.
 *
 * The generated route entry merges every `meta` on the path — root layout to
 * page, most specific wins — and renders the result as `<title>`, `<meta>`, and
 * `<link>` elements that React 19 hoists into `<head>`. See `routeMetaPrelude()`
 * in `packages/ruvyxa/runtime/entry-templates.mjs` for the emitter and
 * `META_PRELUDE` in `crates/ruvyxa_bundler/src/output.rs` for its Rust mirror.
 *
 * ```tsx
 * // app/layout.tsx
 * export const meta: RouteMeta = {
 *   titleTemplate: '%s — Ruvyxa',
 *   siteName: 'Ruvyxa',
 *   description: 'The React framework with a native heart.',
 * }
 *
 * // app/blog/[slug]/page.tsx — receives the layout's template
 * export const meta: RouteMetaFactory = ({ params }) => ({
 *   title: params.slug,
 *   canonical: `https://ruvyxa.dev/blog/${params.slug}`,
 * })
 * ```
 *
 * `<Seo>` remains available for per-render metadata inside a component. Do not
 * set the same field in both: React hoists both `<title>` elements and the last
 * one wins, which is rarely what the author meant.
 */

/** One `<link rel="alternate" hreflang>` entry. */
export interface RouteMetaAlternate {
  /** BCP 47 tag, or `x-default`. */
  hreflang: string
  /** Absolute URL of the alternate document. */
  href: string
}

/** Render context passed to a `meta` function. */
export interface RouteMetaContext {
  /** Concrete request path, e.g. `/blog/hello`. */
  path: string
  /** Dynamic segment values for this request. */
  params: Record<string, string>
}

export interface RouteMeta {
  /** Document title. Formatted by the nearest ancestor `titleTemplate`. */
  title?: string
  /**
   * Format applied to titles declared *below* this level; `%s` is the child
   * title. A level's own `title` is never formatted by its own template.
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
  /**
   * Document language, written onto the `<html lang>` attribute of the rendered
   * document. Client-side navigation does not update it; a locale-segmented
   * route tree gets the correct value on every server-rendered document.
   */
  lang?: string
  /** `<link rel="alternate" hreflang>` entries for other locales of this page. */
  alternates?: readonly RouteMetaAlternate[]
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
  /**
   * Shape of the link preview card on X.
   *
   * Emitted as `<meta name="twitter:card">`: that is the attribute name X's
   * crawler still reads, so the tag keeps the platform's former name even
   * though the option does not. Defaults to `summary_large_image` when an image
   * is set.
   */
  card?: 'summary' | 'summary_large_image'
}

/** A `meta` export declared as a function of the current request. */
export type RouteMetaFactory = (context: RouteMetaContext) => RouteMeta

/** Either shape accepted by `export const meta`. */
export type RouteMetaExport = RouteMeta | RouteMetaFactory
