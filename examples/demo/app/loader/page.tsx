'use client'

/**
 * Client data loading — exercises useRuvyxaLoader, including the two behaviors
 * its contract guarantees: a `deps`-driven refetch and a manual `refetch()`.
 *
 * The `deps` array here is fixed-length, but the hook tolerates a length that
 * changes between renders (see `packages/@ruvyxa/react/src/use-loader.ts`); this
 * page is the runtime companion to that unit contract.
 */

import { useState } from 'react'

import { useRuvyxaLoader } from '@ruvyxa/react'

interface Health {
  ok: boolean
  framework: string
}

export default function LoaderPage() {
  const [reloads, setReloads] = useState(0)

  const { data, loading, error, refetch } = useRuvyxaLoader<Health>(
    async () => {
      const response = await fetch('/api/health')
      if (!response.ok) throw new Error(`Request failed: ${response.status}`)
      return (await response.json()) as Health
    },
    { deps: [reloads] },
  )

  return (
    <main className="page">
      <p className="eyebrow">Client loader</p>
      <h1>useRuvyxaLoader</h1>
      <p>
        Fetches <code>/api/health</code> on mount and whenever <code>deps</code> change. The badge
        below reflects the current request state.
      </p>

      {loading && <p className="badge">Loading…</p>}
      {error && <p className="badge">Error: {error.message}</p>}
      {data && <pre>{JSON.stringify(data, null, 2)}</pre>}

      <button type="button" onClick={() => setReloads((count) => count + 1)}>
        Refetch via deps ({reloads})
      </button>
      <button type="button" onClick={() => refetch()}>
        Refetch manually
      </button>

      <p className="badge">Feature: useRuvyxaLoader</p>
    </main>
  )
}
