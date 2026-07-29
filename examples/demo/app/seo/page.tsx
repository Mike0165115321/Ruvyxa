/**
 * Route metadata — the `export const meta` pipeline.
 *
 * `meta` is merged least-to-most specific from the root layout down to this
 * page (see `packages/@ruvyxa/react/src/meta.ts`) and emitted into the document
 * head during SSR. This page covers the full field surface: title formatting
 * through the layout's `titleTemplate`, description, canonical, robots,
 * hreflang alternates, and the Open Graph / Twitter fields.
 *
 * The `Seo` component is demonstrated separately at `/seo/runtime`. Setting the
 * same field in both places renders two title elements, which is never what an
 * author means, so each page here owns exactly one metadata source.
 */

import { Link } from '@ruvyxa/react'
import type { Meta } from '@ruvyxa/react'

export const meta: Meta = {
  title: 'SEO and metadata',
  description: 'Per-route metadata merged from the root layout down.',
  canonical: 'https://ruvyxa.dev/seo',
  robots: 'index, follow',
  alternates: [
    { hreflang: 'en', href: 'https://ruvyxa.dev/seo' },
    { hreflang: 'th', href: 'https://ruvyxa.dev/th/seo' },
    { hreflang: 'x-default', href: 'https://ruvyxa.dev/seo' },
  ],
  image: '/ruvyxa-card.png',
  imageAlt: 'Ruvyxa Kitchen Sink',
  siteName: 'Ruvyxa Kitchen Sink',
  type: 'website',
  locale: 'en_US',
  card: 'summary_large_image',
}

export default function SeoPage() {
  return (
    <main className="page">
      <p className="eyebrow">Metadata pipeline</p>
      <h1>Route metadata</h1>
      <p>
        This page declares a route <code>meta</code> export. It is merged with the root layout, so
        the rendered title is formatted by the layout&apos;s <code>titleTemplate</code>.
      </p>
      <ul>
        <li>Title, description, canonical, and robots come from the merged metadata.</li>
        <li>
          Three <code>hreflang</code> alternates are emitted as alternate links.
        </li>
        <li>Open Graph and Twitter tags come from the same object.</li>
      </ul>
      <p>
        <Link href="/seo/runtime">See the runtime metadata component instead</Link>
      </p>
      <p className="badge">Feature: route metadata</p>
    </main>
  )
}
