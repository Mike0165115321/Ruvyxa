'use client'

/**
 * Client router hooks — exercises every hook the router exposes:
 * useRouter, usePathname, useParams, useSearchParams, useSelectedRoute.
 *
 * The page reads the live routing state and drives imperative navigation, which
 * is the path that loads route bundles and emits `modulepreload` hints on
 * prefetch (see `packages/@ruvyxa/react/src/router.ts`).
 */

import {
  Link,
  useParams,
  usePathname,
  useRouter,
  useSearchParams,
  useSelectedRoute,
} from '@ruvyxa/react'

export default function HooksPage() {
  const router = useRouter()
  const pathname = usePathname()
  const params = useParams()
  const searchParams = useSearchParams()
  const selected = useSelectedRoute()

  return (
    <main className="page-wide">
      <p className="eyebrow">Client router hooks</p>
      <h1>Router hooks</h1>

      <section>
        <h2>Live routing state</h2>
        <ul>
          <li>
            <code>usePathname()</code> → <strong>{pathname}</strong>
          </li>
          <li>
            <code>useSelectedRoute()</code> → <strong>{selected ?? '(none)'}</strong>
          </li>
          <li>
            <code>useParams()</code> → <code>{JSON.stringify(params)}</code>
          </li>
          <li>
            <code>useSearchParams()</code> → <code>{searchParams.toString() || '(empty)'}</code>
          </li>
        </ul>
      </section>

      <section>
        <h2>Imperative navigation</h2>
        <p>
          <code>useRouter()</code> pushes without a document load and warms bundles ahead of a
          click.
        </p>
        <button type="button" onClick={() => router.push('/about')}>
          router.push(&quot;/about&quot;)
        </button>
        <button type="button" onClick={() => router.push('/hooks?tab=state')}>
          push with a query
        </button>
        <button type="button" onClick={() => router.refresh()}>
          router.refresh()
        </button>
      </section>

      <section>
        <h2>Declarative navigation</h2>
        <p>
          <Link href="/showcase" prefetch="hover">
            Prefetch /showcase on hover
          </Link>
        </p>
      </section>

      <p className="badge">Feature: client router hooks</p>
    </main>
  )
}
