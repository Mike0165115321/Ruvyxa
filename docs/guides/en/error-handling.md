# Error Handling

> 🔴 **Reference** · Framework diagnostics, route recovery, HTTP/API contracts, server actions,
> client failures, and official packages.

Ruvyxa uses **RUV####** for framework, runtime, build, and official-package failures. An application
response is different: it is the intentional HTTP status and safe message that an API route or
action returns to its caller.

Never show compiler paths, stacks, tokens, database errors, or raw RUV diagnostics to ordinary
production users. Development can show structured diagnostic context; unhandled native production
diagnostics are intentionally redacted to a generic 500 page.

## Fast triage

1. Keep the complete code and message; do not replace it with “build failed”.
2. Classify it as build/validation, render, API/action, client, adapter, or package.
3. Fix the named file and import chain first, then rerun the narrowest reproducing command.
4. In CI, use **ruvyxa analyze --format sarif --output reports/ruvyxa.sarif** to retain locations
   and suggested fixes.
5. When escalating, include code, command, framework version, Node/Bun, target, route, and a
   redacted stack—never secrets or cookies.

## Pick the recovery layer

| Failure belongs to        | Use                                              | Result                                      |
| ------------------------- | ------------------------------------------------ | ------------------------------------------- |
| Build/contract failure    | Fix diagnostic; run check/build                  | No deployment; dev overlay can show details |
| Page render throws        | nearest error.tsx or RuvyxaErrorBoundary         | Local fallback and optional retry           |
| Expected missing resource | notFound() and nearest not-found.tsx             | Not-found UI, including server recovery     |
| Client data request       | useRuvyxaLoader error and refetch                | Explicit loading/error/retry UI             |
| Hydration mismatch        | hydrate({ onError })                             | Safe observability report                   |
| Bad API input             | Return intentional 4xx Response                  | Stable public contract                      |
| Action blocked/malformed  | Built-in action security plus handler validation | 400/403/413/415/429                         |
| DB/auth/realtime failure  | Catch at app boundary, log code, map safely      | Product-specific public error               |

## Route recovery

Place special files beside the route segment they protect; the nearest one wins and layouts remain
visible.

```tsx
// app/products/error.tsx
'use client'
import type { RouteErrorProps } from '@ruvyxa/react'

export default function ProductsError({ error, reset }: RouteErrorProps) {
  console.error('products route failed', { message: error.message })
  return (
    <main>
      <h1>We could not load products</h1>
      <button onClick={reset}>Retry</button>
    </main>
  )
}
```

**reset** remounts the failed route subtree after hydration. It is not a server rollback, so
mutations should be idempotent or clearly report possible prior success.

```tsx
// app/posts/[slug]/page.tsx
import { notFound } from '@ruvyxa/react'
const post = await getPost(params.slug)
if (!post) notFound()
```

Use **notFound()** for expected absence, never a generic throw. It renders the nearest not-found.tsx
and is recoverable on the server before JavaScript. Ordinary error.tsx recovery is client-side for
streamed SSR, so provide a useful shell/loading UI and do not depend on retry before hydration.

## Client data and hydration

useRuvyxaLoader normalizes both a synchronous throw and rejected promise into **error**, prevents
stale requests from winning, and provides **refetch**. Render all states.

```tsx
'use client'
import { useRuvyxaLoader } from '@ruvyxa/react'
const { data, loading, error, refetch } = useRuvyxaLoader(
  async () => {
    const response = await fetch('/api/account')
    if (!response.ok) throw new Error('Account request failed')
    return response.json() as Promise<{ name: string }>
  },
  { deps: [] },
)
```

Register one hydration reporter near client bootstrap. The reporter receives unknown error plus
optional componentStack/digest; an exception in the reporter is swallowed so error reporting cannot
crash the UI.

```ts
import { hydrate } from '@ruvyxa/react'
hydrate({
  onError(error, context) {
    reportToObservability({
      kind: 'hydration',
      message: error instanceof Error ? error.message : String(error),
      ...context,
    })
  },
})
```

## API routes and server actions

API routes own the public HTTP contract. Validate close to the handler and catch expected dependency
failures. A throw becomes a framework runtime failure (commonly RUV1200 in the native worker), not
useful client validation feedback.

```ts
export async function POST({ request }: { request: Request }) {
  let body: unknown
  try {
    body = await request.json()
  } catch {
    return Response.json({ error: 'invalid_json' }, { status: 400 })
  }
  if (!body || typeof body !== 'object')
    return Response.json({ error: 'invalid_input' }, { status: 400 })
  try {
    return Response.json({ data: await createItem(body) }, { status: 201 })
  } catch (error) {
    logServerError(error)
    return Response.json({ error: 'temporarily_unavailable' }, { status: 503 })
  }
}
```

| Status  | Use it for                                                        |
| ------- | ----------------------------------------------------------------- |
| 400     | Invalid syntax or validated input                                 |
| 401     | Authentication required                                           |
| 403     | No permission; Ruvyxa also blocks cross-site actions with 403     |
| 404     | Missing resource/action route                                     |
| 405     | Method not exported or action targets a non-page route            |
| 413     | Payload exceeds configured limit                                  |
| 415     | Action body is not JSON or URL-encoded form                       |
| 429     | Rate limit; honor retry timing                                    |
| 500/503 | Unexpected/internal/dependency failure; public text stays generic |

Before an action runs, Ruvyxa can reject an oversized payload (413), unsupported type (415),
cross-origin/cross-site request (403), malformed UTF-8/JSON (400), or rate-limit breach (429). This
complements—not replaces—action.input({ parse }) validation and authorization in the handler.
Parser/handler throws are server execution failures: log them and return safe client feedback.

## Diagnostic contract

Each Rust Diagnostic has code, title, explanation, optional file/line/column, import chain,
suggested fix, and affected routes. Human, JSON, and SARIF output come from that same object in
**crates/ruvyxa_diagnostics/src/lib.rs**.

“Forwarded/reserved” below means the native host recognizes the code, but a static search of this
workspace found no current direct emitter. Preserve the worker output and verify the runtime version
before assigning a root cause. RUV9999 is a test-only redaction sentinel, not a public runtime
diagnostic.

## Complete RUV code catalogue

### Routes, boundaries, SSR, APIs, and content

| Code    | Meaning / likely trigger                                              | First recovery action                                                       |
| ------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| RUV1001 | App directory missing.                                                | Create app/page.tsx (or page.md/page.mdx), or set appDir.                   |
| RUV1002 | Invalid dynamic segment syntax.                                       | Use [name], [...name], or [[...name]].                                      |
| RUV1003 | Routes have the same URL match shape.                                 | Make static path shape distinct; parameter names do not distinguish routes. |
| RUV1004 | TS/JS page lacks default export.                                      | Export the default page component.                                          |
| RUV1007 | Server-only module reaches client graph.                              | Move work server-side; pass serializable data.                              |
| RUV1008 | Private env reaches browser code.                                     | Read it server-side; expose only deliberate RUVYXA_PUBLIC_ values.          |
| RUV1009 | Client-only module reaches server graph.                              | Move it to a client module/component.                                       |
| RUV1010 | Module under server/ reaches client graph.                            | Move browser-safe sharing outside server/.                                  |
| RUV1100 | SSR renderer caught render exception.                                 | Read nested JS cause/stack; add route fallback for users.                   |
| RUV1101 | SSR renderer lacks internal arguments.                                | Repair/reinstall runtime; not normal app input.                             |
| RUV1102 | SSR renderer script missing.                                          | Install ruvyxa/runtime dependencies.                                        |
| RUV1200 | API renderer handler/bundle threw.                                    | Handle expected route errors; inspect server cause.                         |
| RUV1201 | API renderer lacks arguments **or** native port fallback cannot bind. | Repair invocation; for bind failure free/configure scanned port range.      |
| RUV1202 | API renderer script missing.                                          | Reinstall ruvyxa/runtime dependencies.                                      |
| RUV1205 | Prerender path escapes safe output.                                   | Return static params mapping to plain URL segments.                         |
| RUV1300 | Client hydration bundle compilation failed.                           | Fix page, browser-safe imports, JSX, or React dependency.                   |
| RUV1303 | Client bundle requested for absent manifest route.                    | Reload; then verify route deployment/cache consistency.                     |
| RUV1304 | Client bundle requested for non-page route.                           | Hydrate only page routes.                                                   |
| RUV1310 | Unsupported content extension.                                        | Use supported page.md/page.mdx inputs.                                      |
| RUV1311 | Markdown/MDX/content compilation failed.                              | Fix content syntax and embedded imports/expressions.                        |
| RUV1312 | Frontmatter has opening --- but no closing delimiter.                 | Close with --- or ....                                                      |

### Styles, rendering, actions, and configuration

| Code    | Meaning / likely trigger                                   | First recovery action                                            |
| ------- | ---------------------------------------------------------- | ---------------------------------------------------------------- |
| RUV1400 | Tailwind CLI compilation failed.                           | Check directives, content sources, and Tailwind versions.        |
| RUV1401 | Tailwind import found but @tailwindcss/cli missing.        | Install Tailwind and its CLI.                                    |
| RUV1402 | Sass compilation failed.                                   | Fix named Sass syntax/import.                                    |
| RUV1403 | CSS/Sass import cannot resolve.                            | Correct path or install dependency.                              |
| RUV1404 | css.entries escapes project root.                          | Use project-relative entry.                                      |
| RUV1500 | General SSG/action/PPR/action-realtime execution failure.  | Preserve nested worker message; fix route/action contract first. |
| RUV1501 | Adjacent action.ts/action.js missing.                      | Create it beside page and export requested action.               |
| RUV1502 | Forwarded/reserved server-action worker code.              | Retain worker message; verify runtime before diagnosis.          |
| RUV1503 | Forwarded/reserved server-action worker code.              | Retain worker message; verify runtime before diagnosis.          |
| RUV1510 | staticParams result is not array or { params }.            | Return documented array/object shape.                            |
| RUV1511 | Scalar static-param shorthand used with multiple segments. | Return an object keyed by every segment.                         |
| RUV1512 | Static-param entry is neither object nor valid scalar.     | Return object; scalar only for one segment.                      |
| RUV1513 | Static-param cache duration invalid.                       | Use positive number or duration such as 10m.                     |
| RUV1550 | Partial prerender failed.                                  | Inspect nested render error; separate static/dynamic work.       |
| RUV1600 | ruvyxa.config failed to load/evaluate.                     | Fix syntax, imports, and config side effects.                    |
| RUV1601 | Config-renderer invocation/config validation failure.      | Follow detailed field/message; repair config/runtime invocation. |
| RUV1602 | Bounded configuration value outside allowed range.         | Set named value within reported min/max.                         |
| RUV1603 | config.adapter or adapter.build contract invalid.          | Provide build(context) and valid output object.                  |

### Workers, compilation, middleware, adapters, and packages

| Code    | Meaning / likely trigger                              | First recovery action                                              |
| ------- | ----------------------------------------------------- | ------------------------------------------------------------------ |
| RUV1700 | TypeScript plugin hook timed out.                     | Reduce blocking hook work or adjust relevant timeout.              |
| RUV1701 | Plugin host returned invalid protocol/registry data.  | Check plugin runtime/version and hook result.                      |
| RUV1702 | Worker-pool script missing.                           | Reinstall ruvyxa/runtime dependencies.                             |
| RUV1704 | Worker stream/API protocol sent error frame.          | Preserve frame message; inspect stream handler/logs.               |
| RUV1801 | Runtime compiler cannot resolve relative import.      | Fix import path or dependency.                                     |
| RUV1802 | Oxc TS/JSX transform failed.                          | Fix reported source syntax/transform issue.                        |
| RUV1803 | Runtime compiler found circular dependency.           | Break cycle through lower-level shared module.                     |
| RUV1804 | JSX runtime is neither classic nor automatic.         | Use one supported value.                                           |
| RUV2000 | Middleware configuration diagnostic.                  | Correct named invalid setting.                                     |
| RUV2001 | Middleware execution diagnostic.                      | Check custom middleware/dependencies; protect response boundary.   |
| RUV2200 | Adapter runner/build/artifact contract failed.        | Validate build output, artifact paths/kinds, and runner mode.      |
| RUV2202 | Adapter cannot support route strategy.                | Choose compatible target/adapter or change route strategy.         |
| RUV2203 | Adapter package/factory unresolved.                   | Install adapter or export factory correctly.                       |
| RUV2210 | Platform cannot serve route render strategy.          | Deploy compatible platform or choose supported strategy.           |
| RUV3001 | DB query/options/private DB env invalid.              | Correct model/operation/args; keep DB env private.                 |
| RUV3002 | DB adapter lacks model/operation mapping.             | Correct Prisma/Dynamo mapping or transport operation.              |
| RUV3003 | Transaction requested from non-transactional adapter. | Use transactional adapter or redesign safely.                      |
| RUV3100 | Auth runtime/provider delivery failed.                | Check provider credentials/service; return safe temporary failure. |
| RUV3101 | Auth input/config invalid.                            | Validate input and provider config.                                |
| RUV3102 | Auth rate limit exceeded.                             | Honor retry timing; do not loop retries.                           |
| RUV3103 | OAuth/magic-link token or state invalid/expired.      | Restart sign-in; never reuse token.                                |
| RUV3104 | OAuth token exchange failed/no access token.          | Check provider endpoint, credentials, response.                    |
| RUV3105 | Production auth stores are not durable.               | Configure durable session/token/rate-limit stores.                 |
| RUV3201 | Native realtime on unsupported target/adapter.        | Use long-lived Node/Bun or remove native realtime.                 |
| RUV9999 | Test-only production-redaction sentinel.              | Never depend on it as public diagnostic.                           |

## Log versus show

Log code, title, route, method, safe correlation ID, framework version, target, and redacted
cause/stack. Show a product-language message and retry guidance. Use the same public message for
auth, authorization, database, and internal-rendering failures whether or not the internal cause is
known; this avoids account, topology, and secret disclosure.

Framework contributors must add a stable code, explanation, span when known, suggested fix, and
tests. Update this guide when recovery behavior changes. Do not add separate JSON/SARIF scanners:
the existing Diagnostic serializer is the single output contract.
