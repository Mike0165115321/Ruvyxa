# Environment Variables

Every app needs configuration that changes between environments: API keys, database URLs, feature
flags. Ruvyxa gives you a clean, secure system for managing environment variables -- with one hard
rule: **secrets stay on the server**.

---

## What You Will Learn

- The .env file hierarchy and load order
- Public vs private variables and the RUVYXA_PUBLIC_ prefix
- Accessing variables at runtime and build time (process.env vs import.meta.env)
- TypeScript type declarations (ImportMetaEnv, ProcessEnv)
- Client bundle validation and the RUV1008 error
- Build-time inlining and dead-code elimination
- requireEnv plugin for startup validation
- Security best practices and .gitignore patterns
- CI/CD adapter-specific env configuration

---

## Environment File Hierarchy

Ruvyxa loads environment variables from these files, in order of precedence (later files override
earlier ones):

```
.ruvyxa/.env                # Generated build metadata (do not edit)
.env                        # Default values (committed to repo)
.env.local                  # Local overrides (NOT committed)
.env.development            # Dev-server specific
.env.production             # Production specific
.env.local.development      # Local dev overrides
.env.local.production       # Local prod overrides
```

### Load Order

```
         .ruvyxa/.env          (lowest priority -- build metadata)
              |
              v
            .env
              |
              v
        .env.local
              |
              v
     .env.development        or    .env.production
              |
              v
.env.local.development       or   .env.local.production
              |
              v
      process.env / import.meta.env       (highest priority -- shell vars)
```

Variables set in your shell session or OS environment always win over any .env file. This is used by
CI/CD pipelines that inject secrets directly.

### What to Commit

| File                     | Commit? | Purpose                     |
| ------------------------ | ------- | --------------------------- |
| `.env`                   | Yes     | Defaults with dummy values  |
| `.env.example`           | Yes     | Template for new developers |
| `.env.local`             | No      | Local machine overrides     |
| `.env.development`       | Yes     | Shared dev config           |
| `.env.production`        | Yes     | Shared production config    |
| `.env.local.development` | No      | Personal dev overrides      |
| `.env.local.production`  | No      | Personal prod overrides     |

Rule of thumb: anything with `.local` in the name stays out of your repository. Add `*.local` to
`.gitignore`.

---

## Public vs Private Variables

This is the most important concept.

### RUVYXA_PUBLIC_* -- The Public Prefix

Only variables prefixed with `RUVYXA_PUBLIC_` are available in client-side JavaScript bundles.

```bash
# .env
RUVYXA_PUBLIC_SITE_URL=https://example.com
RUVYXA_PUBLIC_GA_ID=G-XXXXXXXXXX
DATABASE_URL=postgres://user:pass@localhost:5432/db
STRIPE_SECRET_KEY=sk_live_...
```

In this example:

- `RUVYXA_PUBLIC_SITE_URL` and `RUVYXA_PUBLIC_GA_ID` -> available everywhere
- `DATABASE_URL` and `STRIPE_SECRET_KEY` -> server only

```
+---------------------------------------------------+
|              Server Bundle                         |
|  RUVYXA_PUBLIC_SITE_URL  OK                       |
|  RUVYXA_PUBLIC_GA_ID     OK                       |
|  DATABASE_URL            OK                       |
|  STRIPE_SECRET_KEY       OK                       |
+-------------------------+-------------------------+
                          |
                          | Bundler boundary
                          |
                          v
+---------------------------------------------------+
|              Client Bundle                         |
|  RUVYXA_PUBLIC_SITE_URL  OK                       |
|  RUVYXA_PUBLIC_GA_ID     OK                       |
|  DATABASE_URL            RUV1008 ERROR            |
|  STRIPE_SECRET_KEY       RUV1008 ERROR            |
+---------------------------------------------------+
```

### Prefix Detection Algorithm

The client bundle scanner in `crates/ruvyxa_bundler/src/boundary.rs` uses a token-aware scan:

```rust
fn find_private_env_reads(source: &str) -> Vec<String> {
    // Scans source bytes for `process.env.NAME` and `process.env['NAME']`
    // Skips strings, comments, template literals, regex literals
    // Allows: NODE_ENV, RUVYXA_PUBLIC_*
    // Reports: everything else as private
}
```

The scanner handles:

- Dot access: `process.env.DATABASE_URL`
- Bracket access: `process.env['API_KEY']`
- Template expressions: `` `db://${process.env.DATABASE_URL}` ``
- String literals: `"process.env.FOO"` (ignored -- inside string)
- Comments: `// process.env.FOO` (ignored)
- Regex literals: `/["']/` (correctly handled, not confused with strings)
- Division: `total / count` (not mistaken for regex)

Allowed client variables:

- `NODE_ENV` -- always allowed
- `RUVYXA_PUBLIC_*` -- any variable starting with this prefix

### What Happens If You Leak a Private Variable?

Ruvyxa's bundler scans client code for non-public environment variable references. If found, it
throws **RUV1008**:

```
RUV1008: Private environment variable used in client bundle

  Variable: DATABASE_URL
    File: app/components/UserProfile.tsx:12
    Explanation: `process.env.DATABASE_URL` is reachable from browser code.
    Only `RUVYXA_PUBLIC_*` env vars may be exposed to client modules.

    Fix: Rename `DATABASE_URL` to `RUVYXA_PUBLIC_DATABASE_URL` if it is safe
    to expose, or move the env read into server-only code.

  For more information, see:
  https://ruvyxa.dev/docs/errors/RUV1008
```

This check happens at **build time** and **dev time**, so you catch it before deployment.

### RUV1008 -- Full Error Specification

| Field             | Value                                                                               |
| ----------------- | ----------------------------------------------------------------------------------- |
| Error Code        | `RUV1008`                                                                           |
| Severity          | Non-fatal diagnostic (build continues, but reported)                                |
| Trigger           | `process.env.VAR` or `process.env['VAR']` in client-bundle code                     |
| Condition         | VAR is not `NODE_ENV` and does not start with `RUVYXA_PUBLIC_`                      |
| File Location     | `boundary.rs` in `find_private_env_reads()`                                         |
| Also triggers for | `@ruvyxa/auth`, `@ruvyxa/database` imports (RUV1007), `server/` directory (RUV1010) |

### Related Boundary Violations

| Code    | Message                                            | Severity   | Trigger                                           |
| ------- | -------------------------------------------------- | ---------- | ------------------------------------------------- |
| RUV1007 | Server-only module imported into client bundle     | Hard error | `import "server-only"` or `import "@ruvyxa/auth"` |
| RUV1008 | Private environment variable used in client bundle | Diagnostic | `process.env.SECRET` in client code               |
| RUV1009 | Client-only module imported into SSR graph         | Diagnostic | `import "client-only"` in server bundle           |
| RUV1010 | Server directory module reached by client graph    | Hard error | File under `server/` reachable from client        |

---

## Accessing Environment Variables

### At Runtime

```tsx
// Works everywhere (server + client)
const siteUrl = process.env.RUVYXA_PUBLIC_SITE_URL
const siteUrl = import.meta.env.RUVYXA_PUBLIC_SITE_URL // ESM alias

// Server only -- RUV1008 in client code
const dbUrl = process.env.DATABASE_URL
```

### In Server Components

```tsx
// app/page.tsx -- server component, safe
export default async function HomePage() {
  const dbUrl = process.env.DATABASE_URL
  const data = await fetchData(dbUrl)
  return <div>{/* render */}</div>
}
```

### In Client Components

```tsx
// 'use client' -- only public vars allowed
'use client'

export default function AnalyticsTracker() {
  const gaId = process.env.RUVYXA_PUBLIC_GA_ID
  return <Script src={`https://www.googletagmanager.com/gtag/js?id=${gaId}`} />
}
```

### In API Routes

```tsx
// app/api/payment/route.ts -- server only, safe
export async function POST(request: Request) {
  const stripeKey = process.env.STRIPE_SECRET_KEY
  // ... process payment
  return Response.json({ success: true })
}
```

### In Server Actions

```ts
// app/actions/email/action.ts
'use server'

import { action } from 'ruvyxa/server'

export const sendNewsletter = action(async (formData: FormData) => {
  const apiKey = process.env.SENDGRID_API_KEY
  // ... send email
})
```

---

## TypeScript Declarations

Ruvyxa generates a `ruvyxa-env.d.ts` file in your project root. Extend it with your own types:

```ts
// ruvyxa-env.d.ts
/// <reference types="ruvyxa/client" />

declare namespace NodeJS {
  interface ProcessEnv {
    RUVYXA_PUBLIC_SITE_URL: string
    RUVYXA_PUBLIC_GA_ID: string
    DATABASE_URL: string
    STRIPE_SECRET_KEY: string
    SENDGRID_API_KEY: string
  }
}
```

Or use `ImportMetaEnv`:

```ts
// env.d.ts
/// <reference types="ruvyxa/client" />

interface ImportMetaEnv {
  readonly RUVYXA_PUBLIC_SITE_URL: string
  readonly RUVYXA_PUBLIC_GA_ID: string
  readonly DATABASE_URL: string
  readonly STRIPE_SECRET_KEY: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
```

With these declarations, `process.env.DATABASE_URL` is fully typed.

---

## Build-Time Variables

Environment variables are inlined at build time:

```ts
// This is replaced at build time with the actual value
const apiUrl = process.env.RUVYXA_PUBLIC_API_URL
// After build: const apiUrl = "https://api.example.com";
```

### Implications

1. **You must rebuild** to pick up changed environment variables (for public vars)
2. **Dead code elimination** works -- conditional branches using env vars can be stripped:

```ts
if (process.env.RUVYXA_PUBLIC_FEATURE_FLAG === 'enabled') {
  // This branch may be entirely removed in production if the flag is off
  registerFeature()
}
```

For server-only vars, no inlining happens -- they are read from the actual environment at runtime.

### Stability

The build uses `stable_process_env()` to create a deterministic snapshot of environment variables
for cache key computation:

```rust
fn prerender_context_hash(
    root: &Path,
    styles: &str,
    client_assets: &BTreeMap<...>,
    build: &BuildConfigOptions,
    project_env: &BTreeMap<String, String>,
) -> String {
    // Includes RUVYXA_PUBLIC_* in hash so cache invalidates when env changes
}
```

---

## Validation & Defaults

### Using requireEnv Plugin

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'

export default config({
  plugins: [
    {
      name: 'requireEnv',
      options: {
        variables: ['DATABASE_URL', 'STRIPE_SECRET_KEY', 'SENDGRID_API_KEY'],
        strict: true, // fail startup if missing
      },
    },
  ],
})
```

### Default Values

```ts
const port = process.env.PORT ?? '3000'
const logLevel = process.env.LOG_LEVEL || 'info'
```

### Config Renderer Environment

When Ruvyxa evaluates `ruvyxa.config.ts`, it sets:

```bash
RUVYXA_RUNTIME=node     # or bun, etc.
```

This is injected by `run_config_renderer()`:

```rust
ProcessCommand::new(runtime.executable())
    .arg(renderer)
    .arg(root)
    .env("RUVYXA_RUNTIME", runtime.command())
    .output()?;
```

---

## Security Best Practices

### Do Not Commit Secrets

```bash
# .gitignore
.env.local
.env*.local
*.pem
secrets.*
```

### Use .env.example

```bash
# .env.example (committed, safe)
RUVYXA_PUBLIC_SITE_URL=http://localhost:3000
RUVYXA_PUBLIC_GA_ID=UA-XXXXX-Y
DATABASE_URL=postgres://user:password@localhost:5432/db
STRIPE_SECRET_KEY=sk_test_...
SENDGRID_API_KEY=SG.example...
```

New developers copy this to `.env.local` and fill in real values.

### Server-Only Validation Module

```ts
// app/lib/env.server.ts -- server-only
function requireEnv(name: string): string {
  const value = process.env[name]
  if (!value) {
    throw new Error(`Missing required environment variable: ${name}`)
  }
  return value
}

export const DATABASE_URL = requireEnv('DATABASE_URL')
export const STRIPE_SECRET_KEY = requireEnv('STRIPE_SECRET_KEY')
```

Import this from server components, API routes, and server actions only. Never import in a
`'use client'` file.

### Environment-Specific Config

```bash
# .env.development
RUVYXA_PUBLIC_API_URL=http://localhost:4000
LOG_LEVEL=debug

# .env.production
RUVYXA_PUBLIC_API_URL=https://api.example.com
LOG_LEVEL=warn
```

### Platform-Specific Environment Variables

Ruvyxa detects deployment platform from build environment variables:

```rust
const PLATFORM_ADAPTER_ENV: [(&str, &str); 6] = [
    ("VERCEL", "vercel"),
    ("NETLIFY", "netlify"),
    ("CF_PAGES", "cloudflare"),
    ("RAILWAY_PROJECT_ID", "railway"),
    ("RENDER", "render"),
    ("AWS_APP_ID", "aws"),
];
```

The `RUVYXA_ADAPTER` env var overrides auto-detection.

---

## Environment in CI/CD

### Setting Environment Variables

| Platform         | How to set                                   |
| ---------------- | -------------------------------------------- |
| Vercel           | Project Settings > Environment Variables     |
| Netlify          | Site Settings > Build & Deploy > Environment |
| Cloudflare Pages | Project Settings > Environment Variables     |
| Railway          | Project > Variables                          |
| Render           | Dashboard > Environment                      |
| AWS (manual)     | Lambda environment variables or SSM          |

### Adapter Configuration

```ts
// ruvyxa.config.ts
export default config({
  adapter: 'vercel',
  adapterOptions: {
    regions: ['iad1'],
  },
})
```

The adapter emits platform-specific configuration that references environment variables. The adapter
runner (`runtime/adapter-runner.mjs`) reads `RUVYXA_RUNTIME` to select the JavaScript runtime.

---

## Troubleshooting

| Symptom                             | Likely cause                    | Fix                                                   |
| ----------------------------------- | ------------------------------- | ----------------------------------------------------- |
| `process.env.VAR` is undefined      | File not loaded or var missing  | Check .env exists, var name correct                   |
| RUV1008 in build output             | Private var in client code      | Prefix with `RUVYXA_PUBLIC_` or move to server file   |
| Public var is empty string          | Value not set in any .env       | Add to .env or .env.local                             |
| TS error on `process.env`           | Missing type declaration        | Create `ruvyxa-env.d.ts`                              |
| Build succeeds, runtime var missing | Not set in deployment env       | Set in hosting platform dashboard                     |
| Variable leaked to client bundle    | Unintended string interpolation | Search for `process.env.VAR` without `RUVYXA_PUBLIC_` |
| Config fails to load                | RUV1600 config error            | Check ruvyxa.config.ts syntax                         |
| `RUVYXA_RUNTIME` not recognized     | CLI --runtime flag mismatch     | Use `node` or `bun`                                   |

### Full Error: RUV1008

```
RUV1008: Private environment variable used in client bundle

  Variable: DATABASE_URL
    File: app/components/UserProfile.tsx:12
    Explanation: `process.env.DATABASE_URL` is reachable from browser code.
    Only `RUVYXA_PUBLIC_*` env vars may be exposed to client modules.

    Fix: Rename `DATABASE_URL` to `RUVYXA_PUBLIC_DATABASE_URL` if it is safe
    to expose, or move the env read into server-only code.
```

### Full Error: RUV1007

```
RUV1007: Server-only module imported into client bundle

  File: app/components/UserProfile.tsx:5
  Explanation: This module is reachable from the browser hydration bundle
    but declares `server-only`.

    Fix: Move server-only code behind a loader/API route, or pass
    serialized data to the page.
```

### Full Error: RUV1009

```
RUV1009: Client-only module imported into SSR graph

  File: app/lib/browser-utils.ts
  Explanation: This module is reachable from server runtime code but
    declares `client-only`.

    Fix: Move browser-only code into a client component or client.tsx module.
```

---

## Full Example: Multi-Environment Setup

```bash
# .env (defaults -- committed)
RUVYXA_PUBLIC_SITE_URL=http://localhost:3000
RUVYXA_PUBLIC_GA_ID=
DATABASE_URL=postgres://postgres:postgres@localhost:5432/myapp

# .env.local (local overrides -- NOT committed)
DATABASE_URL=postgres://admin:realpassword@localhost:5432/myapp

# .env.production (shared prod config -- committed)
RUVYXA_PUBLIC_SITE_URL=https://myapp.com
RUVYXA_PUBLIC_GA_ID=G-MEASUREMENT123
```

```ts
// ruvyxa-env.d.ts
/// <reference types="ruvyxa/client" />

declare namespace NodeJS {
  interface ProcessEnv {
    RUVYXA_PUBLIC_SITE_URL: string
    RUVYXA_PUBLIC_GA_ID: string
    DATABASE_URL: string
  }
}
```

```ts
// app/lib/db.ts (server only -- never imported from client)
export function getDbUrl(): string {
  return process.env.DATABASE_URL
}
```

```tsx
// app/components/Analytics.tsx ('use client')
'use client'

export function Analytics() {
  const gaId = process.env.RUVYXA_PUBLIC_GA_ID
  if (!gaId) return null
  return <Script src={`https://www.googletagmanager.com/gtag/js?id=${gaId}`} />
}
```

---

## Next Steps

- [03-server-client-components.md](./03-server-client-components.md) -- Understanding the
  server/client boundary
- [11-configuration.md](./11-configuration.md) -- Full config reference for ruvyxa.config.ts
- [14-plugins.md](./14-plugins.md) -- requireEnv plugin and env validation
- [16-error-handling.md](./16-error-handling.md) -- RUV1008 and related errors
