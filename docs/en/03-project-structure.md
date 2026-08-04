# Project structure

Ruvyxa discovers routes from the configured `appDir` (`app` when configured as in the templates).
The file name expresses route behavior; JavaScript exports express rendering behavior.

```text
app/
├── layout.tsx                 # shared shell
├── page.tsx                   # GET /
├── about/
│   └── page.tsx               # GET /about
├── blog/
│   ├── page.tsx               # GET /blog
│   └── [slug]/page.tsx        # GET /blog/:slug
├── api/
│   └── health/route.ts        # API handler at /api/health
└── showcase/
    ├── error.tsx              # nearest render-error boundary
    ├── loading.tsx            # loading boundary
    └── not-found.tsx          # nearest not-found UI
```

## Route files

| File            | Implemented purpose                                   |
| --------------- | ----------------------------------------------------- |
| `page.tsx`      | A page route component.                               |
| `layout.tsx`    | A layout composed along the route path.               |
| `route.ts`      | An API route module exporting HTTP method functions.  |
| `loading.tsx`   | Loading component discovered with the route boundary. |
| `error.tsx`     | Error component; it receives `{ error, reset }`.      |
| `not-found.tsx` | Nearest UI for the framework not-found signal.        |

Dynamic folders use `[name]`, catch-all `[...name]`, and optional catch-all `[[...name]]`. The demo
has `[slug]` and `[...slug]` examples. Keep route-specific server code beside the route only if it
is not imported into client modules; validation enforces server/client boundaries.

## Small complete page

```tsx
// app/about/page.tsx
import type { PageProps } from 'ruvyxa'

export default function About({ requestPath }: PageProps) {
  return (
    <main>
      <h1>About</h1>
      <p>Rendered for {requestPath}</p>
    </main>
  )
}
```

`PageProps.params` contains dynamic segment values; `requestPath` is the concrete request path. Use
a `layout.tsx` for document-wide composition, not duplicated page markup.

**Previous:** [Create your first app](02-create-your-first-app.md) · **Next:**
[Routing and rendering](04-routing-rendering.md)
