import { Link } from '@ruvyxa/react'

/**
 * Root 404 boundary. A `notFound()` call or an unmatched URL that no nearer
 * `not-found.tsx` catches renders here.
 */
export default function NotFound() {
  return (
    <main className="page">
      <p className="eyebrow">404</p>
      <h1>Page not found</h1>
      <p>No route matches that URL.</p>
      <p>
        <Link href="/">Back home</Link>
      </p>
    </main>
  )
}
