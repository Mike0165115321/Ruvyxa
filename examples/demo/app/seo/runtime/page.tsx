/**
 * Runtime metadata — the `Seo` component.
 *
 * `Seo` renders React 19 document metadata from inside a component, which is
 * the escape hatch for metadata that depends on rendered data rather than on
 * the route. It also emits Article JSON-LD and a breadcrumb trail, neither of
 * which the static `meta` export covers.
 *
 * This page declares no `meta` export of its own. It still renders two title
 * elements, because `Seo` always emits one and the root layout's `meta` already
 * did: React hoists both and the last one wins. That is a real constraint of
 * combining the two APIs, not a demo artifact, and it is visible here on
 * purpose rather than hidden behind a page with no layout title.
 */

import { Link, Seo } from '@ruvyxa/react'

export default function SeoRuntimePage() {
  return (
    <main className="page">
      <Seo
        title="Runtime metadata"
        description="Document metadata rendered by the Seo component."
        canonical="https://ruvyxa.dev/seo/runtime"
        image="/ruvyxa-card.png"
        imageAlt="Ruvyxa Kitchen Sink"
        siteName="Ruvyxa Kitchen Sink"
        type="article"
        locale="en_US"
        card="summary_large_image"
        article={{
          type: 'BlogPosting',
          publishedAt: '2026-01-01T00:00:00.000Z',
          updatedAt: '2026-07-29T00:00:00.000Z',
          authors: [{ name: 'Ruvyxa', url: 'https://ruvyxa.dev', type: 'Organization' }],
          section: 'Framework',
          tags: ['seo', 'metadata'],
        }}
        breadcrumbs={[
          { name: 'Home', url: 'https://ruvyxa.dev/' },
          { name: 'SEO', url: 'https://ruvyxa.dev/seo' },
          { name: 'Runtime', url: 'https://ruvyxa.dev/seo/runtime' },
        ]}
      />
      <p className="eyebrow">Runtime metadata</p>
      <h1>Seo component</h1>
      <p>
        This page renders its metadata from a component instead of a route export, which is what a
        page needs when the values depend on data fetched during the render.
      </p>
      <ul>
        <li>Open Graph, Twitter, and canonical tags are emitted by the component.</li>
        <li>Article facts become Article JSON-LD.</li>
        <li>The breadcrumb trail becomes BreadcrumbList JSON-LD.</li>
        <li>
          The document carries two title elements: one from the root layout&apos;s <code>meta</code>{' '}
          and one from this component. React keeps the last.
        </li>
      </ul>
      <p>
        <Link href="/seo">Back to the static metadata pipeline</Link>
      </p>
      <p className="badge">Feature: Seo component</p>
    </main>
  )
}
