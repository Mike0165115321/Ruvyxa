# Create your first Ruvyxa app

## Create an application

The workspace publishes `create-ruvyxa`, and its source contains `minimal`, `blog`, `crud`, and
`api-backend` templates. Use the generator for a complete, package-manager-neutral starter.

```bash
pnpm create ruvyxa my-app
cd my-app
pnpm install
pnpm dev
```

The generated project scripts invoke the installed `ruvyxa` binary. `dev` discovers routes and
starts hot reload; its default root is the current directory. Visit the URL printed by the command
(the default server configuration is `localhost:3000` when no override is supplied).

## Install into an existing React project

The templates prove the minimum runtime dependencies below. Keep compatible React versions together.

```bash
pnpm add ruvyxa @ruvyxa/react react react-dom
pnpm add -D typescript @types/react @types/react-dom
```

Create `ruvyxa.config.ts`:

```ts
import { config } from 'ruvyxa/config'

export default config({
  appDir: 'app',
  outDir: '.ruvyxa',
  server: { host: 'localhost', port: 3000 },
})
```

Then add the files in [Project structure](03-project-structure.md). Do not put an application secret
in a `RUVYXA_PUBLIC_` variable: that prefix is deliberately exposed to browser code.

## Build one working vertical slice

Create these files after installing the dependencies. This is deliberately small: it proves page
routing, a layout, and an API route before you introduce database, auth, or plugins.

```text
app/
├── layout.tsx
├── page.tsx
└── api/
    └── health/
        └── route.ts
```

```tsx
// app/layout.tsx
import type { ReactNode } from 'react'

export const meta = { title: 'My Ruvyxa app', description: 'First Ruvyxa app' }

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
```

```tsx
// app/page.tsx
export default function Home() {
  return (
    <main>
      <h1>Ruvyxa is running</h1>
      <p>Edit app/page.tsx and save.</p>
    </main>
  )
}
```

```ts
// app/api/health/route.ts
export function GET() {
  return Response.json({ status: 'ok' })
}
```

Run `pnpm dev`, open `/`, then open `/api/health`. The first request renders the page; the health
route returns JSON with `status: "ok"`. Save a change to `app/page.tsx` to confirm hot reload, then
verify discovery and production behavior:

```bash
pnpm routes
pnpm check
pnpm build
pnpm test:parity
```

If any command fails, stop at that command and use [Troubleshooting](16-troubleshooting-upgrades.md)
before deploying. `test:parity` compares dev/prod routes and smoke-renders page routes; it is not a
replacement for application tests.

## Scripts

```json
{
  "scripts": {
    "dev": "ruvyxa dev",
    "build": "ruvyxa build",
    "start": "ruvyxa start",
    "preview": "ruvyxa preview",
    "check": "ruvyxa check",
    "routes": "ruvyxa routes"
  }
}
```

`start` and `preview` operate on an existing production build; run `build` first. `check` is the
application-level readiness command. See exact CLI flags in [CLI reference](10-cli.md).

**Previous:** [Introduction](01-introduction.md) · **Next:**
[Project structure](03-project-structure.md)
