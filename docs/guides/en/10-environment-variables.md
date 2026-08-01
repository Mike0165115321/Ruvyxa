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

## Prefix Detection Algorithm — Under the Hood

Ruvyxa uses this algorithm to determine which env vars are safe to send to the client:

```ts
function isClientAccessible(varName: string): boolean {
  // Step 1: Special cases
  if (varName === 'NODE_ENV') return true
  if (varName === 'RUVYXA_RUNTIME') return false // server-only runtime info

  // Step 2: Prefix check
  if (varName.startsWith('RUVYXA_PUBLIC_')) return true

  // Step 3: Explicit allowlist (Ruvyxa internal)
  const ALLOWED_CLIENT_PREFIXES = [
    'NEXT_PUBLIC_', // Next.js compatibility
    'PUBLIC_', // SvelteKit compatibility
    'VITE_', // Vite compatibility
  ]
  if (ALLOWED_CLIENT_PREFIXES.some((prefix) => varName.startsWith(prefix))) {
    return true
  }

  // Step 4: Everything else is server-only
  return false
}

function collectClientEnvVars(allVars: Record<string, string>): Record<string, string> {
  const clientVars: Record<string, string> = {}

  for (const [key, value] of Object.entries(allVars)) {
    if (isClientAccessible(key)) {
      clientVars[key] = value
    }
  }

  return clientVars
}
```

### Live Example

```bash
# Input env vars
RUVYXA_PUBLIC_API_URL=https://api.example.com    → client ✅
RUVYXA_PUBLIC_GA_ID=G-XXXXXXXXXX                 → client ✅
NODE_ENV=development                             → client ✅ (special)
DATABASE_URL=postgres://localhost/db             → server-only ❌
AUTH_SECRET=sk-xxxx                              → server-only ❌
STRIPE_API_KEY=sk_live_xxxxx                     → server-only ❌
MY_APP_SECRET=secret                             → server-only ❌
PUBLIC_STRIPE_KEY=pk_test_xxxxx                  → client ✅ (Vite compat)
```

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

## process.env vs import.meta.env — Differences

| Feature                             | `process.env`                                    | `import.meta.env`                |
| ----------------------------------- | ------------------------------------------------ | -------------------------------- |
| Runtime                             | Node.js (server) + Browser (client, public only) | ESM (both server and client)     |
| Server Components                   | ✅                                               | ✅                               |
| Client Components                   | ✅ (only RUVYXA_PUBLIC_*)                        | ✅ (only RUVYXA_PUBLIC_*)        |
| Type Safety                         | `NodeJS.ProcessEnv` interface                    | `ImportMetaEnv` interface        |
| Auto-complete                       | ✅ if declarations exist                         | ✅ if declarations exist         |
| Build-time replacement              | ✅ Ruvyxa replaces at build time                 | ✅ Ruvyxa replaces at build time |
| Dynamic access (`process.env[var]`) | ✅ (but not recommended)                         | ❌ (must be static string)       |
| Tree-shaking                        | ✅                                               | ✅ Better (static analysis)      |

### Comparison Example

```tsx
// Server Component — Both are fine
export default function ServerPage() {
  // process.env (Node.js style)
  console.log(process.env.RUVYXA_PUBLIC_API_URL)
  console.log(process.env.DATABASE_URL) // ✅ server-only

  // import.meta.env (ESM style)
  console.log(import.meta.env.RUVYXA_PUBLIC_API_URL)
  console.log(import.meta.env.DATABASE_URL) // ✅ server-only
  console.log(import.meta.env.MODE) // 'development' | 'production'

  return <div>Server Component</div>
}
```

```tsx
// Client Component
'use client'

export default function ClientPage() {
  // process.env — Only RUVYXA_PUBLIC_* + NODE_ENV
  console.log(process.env.RUVYXA_PUBLIC_API_URL) // ✅
  console.log(process.env.NODE_ENV) // ✅
  console.log(process.env.DATABASE_URL) // ❌ RUV1008

  // import.meta.env — Only public variables
  console.log(import.meta.env.RUVYXA_PUBLIC_API_URL) // ✅
  console.log(import.meta.env.MODE) // ✅
  console.log(import.meta.env.DATABASE_URL) // ❌ RUV1008

  return <div>Client Component</div>
}
```

---

## Public Variables (`RUVYXA_PUBLIC_*`) — Deep Dive

### Variables Safe for Client

| Variable                              | Example Value               | Usage                  |
| ------------------------------------- | --------------------------- | ---------------------- |
| `RUVYXA_PUBLIC_API_URL`               | `https://api.example.com`   | API endpoint           |
| `RUVYXA_PUBLIC_SITE_URL`              | `https://example.com`       | Site URL               |
| `RUVYXA_PUBLIC_GA_ID`                 | `G-XXXXXXXXXX`              | Google Analytics ID    |
| `RUVYXA_PUBLIC_SENTRY_DSN`            | `https://xxx@sentry.io/xxx` | Sentry DSN (public)    |
| `RUVYXA_PUBLIC_GTM_ID`                | `GTM-XXXXXXX`               | Google Tag Manager     |
| `RUVYXA_PUBLIC_STRIPE_KEY`            | `pk_live_xxxxx`             | Stripe publishable key |
| `RUVYXA_PUBLIC_ALGOLIA_ID`            | `XXXXX`                     | Algolia app ID         |
| `RUVYXA_PUBLIC_MAPBOX_TOKEN`          | `pk.xxxxx`                  | Mapbox public token    |
| `RUVYXA_PUBLIC_POSTHOG_KEY`           | `phc_xxxxx`                 | PostHog public key     |
| `RUVYXA_PUBLIC_CLERK_PUBLISHABLE_KEY` | `pk_test_xxxxx`             | Clerk auth key         |
| `RUVYXA_PUBLIC_VERCEL_ANALYTICS_ID`   | `xxxxx`                     | Vercel Analytics       |
| `RUVYXA_PUBLIC_ENVIRONMENT`           | `production`                | Custom env flag        |

### TypeScript Declarations (All Public)

```ts
// ruvyxa-env.d.ts
declare namespace NodeJS {
  interface ProcessEnv {
    // Public — client-safe
    RUVYXA_PUBLIC_API_URL: string
    RUVYXA_PUBLIC_SITE_URL: string
    RUVYXA_PUBLIC_GA_ID: string
    RUVYXA_PUBLIC_GTM_ID: string
    RUVYXA_PUBLIC_SENTRY_DSN: string
    RUVYXA_PUBLIC_STRIPE_KEY: string
    RUVYXA_PUBLIC_ENVIRONMENT: 'development' | 'staging' | 'production'
  }
}
```

### Usage in Client Components

```tsx
'use client'

export default function AnalyticsProvider({ children }: { children: React.ReactNode }) {
  const gaId = process.env.RUVYXA_PUBLIC_GA_ID
  const gtmId = process.env.RUVYXA_PUBLIC_GTM_ID

  useEffect(() => {
    if (typeof window !== 'undefined' && gaId) {
      // Load Google Analytics
      const script = document.createElement('script')
      script.src = `https://www.googletagmanager.com/gtag/js?id=${gaId}`
      script.async = true
      document.head.appendChild(script)

      window.dataLayer = window.dataLayer || []
      function gtag(...args: unknown[]) {
        window.dataLayer.push(args)
      }
      gtag('js', new Date())
      gtag('config', gaId)
    }
  }, [gaId])

  return <>{children}</>
}
```

---

## Private Variables (Server-Only) — Deep Dive

### Variables Strictly Prohibited on Client

| Category       | Examples                                                | Risk of Leak            |
| -------------- | ------------------------------------------------------- | ----------------------- |
| Database       | `DATABASE_URL`, `MONGODB_URI`, `PGHOST`                 | Data loss or breach     |
| Authentication | `AUTH_SECRET`, `JWT_SECRET`, `AUTH_GOOGLE_SECRET`       | Session hijacking       |
| API Keys       | `STRIPE_API_KEY`, `OPENAI_API_KEY`, `AWS_ACCESS_KEY_ID` | Financial loss, attacks |
| Encryption     | `ENCRYPTION_KEY`, `SALT`                                | Data decryption         |
| Infrastructure | `REDIS_URL`, `SQS_QUEUE_URL`, `CLOUDAMQP_URL`           | Infrastructure attacks  |
| Email          | `SMTP_PASS`, `SENDGRID_API_KEY`                         | Email spoofing/spam     |

### How to Safely Use Private Variables

#### ✅ Server Component (Safe)

```tsx
// app/dashboard/page.tsx — Server Component
import { PrismaClient } from '@prisma/client'

const prisma = new PrismaClient({
  datasources: {
    db: {
      url: process.env.DATABASE_URL, // ✅ Safe
    },
  },
})

export default async function Dashboard() {
  const users = await prisma.user.findMany()
  return <div>Users: {users.length}</div>
}
```

#### ✅ API Route (Safe)

```ts
// app/api/chat/route.ts
import OpenAI from 'openai'

const openai = new OpenAI({
  apiKey: process.env.OPENAI_API_KEY, // ✅ Safe
})

export async function POST(req: Request) {
  // ...
}
```

#### ❌ Client Component (Unsafe -> RUV1008)

```tsx
'use client'

export default function ClientComponent() {
  // ❌ Will throw RUV1008 Environment Boundary Violation
  const apiKey = process.env.OPENAI_API_KEY
  return <div>{apiKey}</div>
}
```

---

## RUV1008 Error — Environment Boundary Violation

If you reference a private environment variable in a Client Component, Ruvyxa's compiler will block
the build:

```
RUV1008: Environment Boundary Violation
  └─ In client component: app/components/ChatBox.tsx
  └─ Private environment variable accessed: process.env.OPENAI_API_KEY

Client components are shipped to the browser. Accessing private variables
would leak your secrets to users.

To fix:
  1. If this is a public variable, prefix it: RUVYXA_PUBLIC_OPENAI_API_KEY
  2. If this is a secret, move the logic to a Server Component or Server Action.
```

---

## Client Scanner — RUV1008 Detection Mechanism

Ruvyxa uses Oxc to statically analyze AST during compilation:

1. Identify files starting with `"use client"` or `'use client'`
2. Scan for AST nodes matching:
   - `MemberExpression`: `process.env.XXX`
   - `MemberExpression`: `import.meta.env.XXX`
   - Destructuring: `const { XXX } = process.env`
3. Evaluate the identifier `XXX`
4. Pass it to `isClientAccessible(XXX)`
5. If it returns `false`, throw RUV1008.

Because this is statically analyzed, dynamic access like `process.env[getVarName()]` is strictly
blocked in Client Components to prevent runtime leaks.

---

## Allowed Client Variables — Full List

The following environment variables are allowed in Client Components implicitly or natively:

- `NODE_ENV` — `development`, `production`, `test`
- `RUVYXA_PUBLIC_*` — Any variable starting with this prefix
- `NEXT_PUBLIC_*` — Next.js compatibility fallback
- `PUBLIC_*` — SvelteKit compatibility fallback
- `VITE_*` — Vite compatibility fallback
- `import.meta.env.MODE` — 'development' | 'production'
- `import.meta.env.DEV` — boolean
- `import.meta.env.PROD` — boolean
- `import.meta.env.SSR` — boolean

---

## Runtime Environment Variables

Docker deployments require variables to be supplied at runtime (not build time).

```dockerfile
# Dockerfile
ENV PORT=3000
CMD ["npm", "run", "start"]
```

**Rule of Thumb:**

- `RUVYXA_PUBLIC_*`: Baked in at **build time**. If changed, you must rebuild.
- `DATABASE_URL` (Server-only): Read dynamically at **runtime**. You can change it without
  rebuilding.

If you absolutely need dynamic public variables at runtime, you must expose them through an API
route and fetch them on the client:

```ts
// app/api/config/route.ts
export function GET() {
  return Response.json({
    theme: process.env.DYNAMIC_THEME_COLOR || 'blue',
  })
}
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

## Platform Detection — Environment Variables

Ruvyxa provides built-in variables to detect the current deployment platform:

```ts
if (process.env.VERCEL) {
  console.log('Running on Vercel!')
} else if (process.env.NETLIFY) {
  console.log('Running on Netlify!')
} else if (process.env.CLOUDFLARE_PAGES) {
  console.log('Running on Cloudflare!')
}
```

These are automatically populated by the respective deployment environments.

---

## Production Deployment — Environment Variable Setup

When deploying your Ruvyxa app:

1. **Do not commit `.env` or `.env.local`**. Add them to `.gitignore`.
2. **Vercel/Netlify**: Add your variables in their web dashboards (Settings -> Environment
   Variables).
3. **Docker**: Pass them using `docker run -e DATABASE_URL=...` or Docker Compose.
4. **CI/CD**: Add them to GitHub Secrets / GitLab CI Variables.

---

## `ruvyxa doctor` — Inspecting Environment Variables

You can use the `doctor` command to verify which environment variables Ruvyxa has loaded and how
they are classified:

```bash
npm run doctor -- --env
```

Output:

```
🩺 Ruvyxa Doctor - Environment Variable Report

Loaded from: .env, .env.local

Public Variables (Client-safe):
  ✅ RUVYXA_PUBLIC_API_URL
  ✅ RUVYXA_PUBLIC_GA_ID

Private Variables (Server-only):
  🔒 DATABASE_URL
  🔒 AUTH_SECRET
  🔒 STRIPE_SECRET_KEY

System Variables:
  ℹ️ NODE_ENV = development
  ℹ️ RUVYXA_RUNTIME = node
```

---

## CI/CD Integration

If your CI pipeline runs `ruvyxa build` (e.g. GitHub Actions), it needs access to all
`RUVYXA_PUBLIC_*` variables so they can be baked into the bundle.

```yaml
# .github/workflows/build.yml
steps:
  - name: Build
    run: npm run build
    env:
      RUVYXA_PUBLIC_API_URL: ${{ secrets.RUVYXA_PUBLIC_API_URL }}
      # Server-only variables like DATABASE_URL are not strictly needed
      # for the build step unless a plugin strictly requires them.
```

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

## Try It Yourself

1. Add `RUVYXA_PUBLIC_TEST=hello` to `.env`
2. Add `SECRET_TEST=world` to `.env`
3. Inside `app/page.tsx`, `console.log` both variables. (It will work on the server console).
4. Inside a Client Component (`'use client'`), `console.log(process.env.SECRET_TEST)`.
5. Observe the `RUV1008` error in your terminal.
6. Change the console log to `process.env.RUVYXA_PUBLIC_TEST`. It will work in the browser console.

---

## Summary

- Use `.env`, `.env.local`, `.env.development`, `.env.production` for managing environment
  variables.
- Prefix with `RUVYXA_PUBLIC_` to expose variables to the browser.
- **Server Components** can access all variables.
- **Client Components** can only access public variables (enforced by RUV1008).
- Public variables are baked in at build time; private variables are read at runtime.
- Never commit private secrets to version control.

---

## Current Environment-loading Contract

For project configuration and JavaScript runtimes, Ruvyxa currently loads `.env` and then
`.env.local` from the project root; a value in `.env.local` wins over the same key in `.env`. The
parser accepts blank lines, comments, `KEY=value`, quoted values, and dotenv-style
`export KEY=value`. It does not, by itself, implement an environment-name matrix such as
`.env.production.local` or an `import.meta.env` API.

```dotenv
# .env -- safe defaults shared by the project
RUVYXA_PUBLIC_SITE_NAME=Catalog
CATALOG_API_URL=https://catalog.example.test

# .env.local -- developer-specific override; keep secrets out of version control
CATALOG_API_URL=http://localhost:4010
```

Use normal process environment variables for platform-provided production configuration. A command
or platform environment can supply those values without requiring a new env-file naming convention.

### Public Exposure Is a Build Boundary

The client-boundary scanner permits `NODE_ENV` and names beginning with `RUVYXA_PUBLIC_` in modules
reachable from the browser. Other statically known `process.env.NAME` reads in that graph produce
`RUV1008`. Prefixing a value is therefore a disclosure decision, not a convenience mechanism.

```tsx
'use client'

export function ProductApiStatus() {
  const apiBase = process.env.RUVYXA_PUBLIC_API_BASE
  return <p>Using {apiBase ?? 'the default API'}</p>
}
```

```ts
// app/server/database.ts
import 'server-only'

export function databaseUrl() {
  const value = process.env.DATABASE_URL
  if (!value) throw new Error('DATABASE_URL is required')
  return value
}
```

Never rename a secret with `RUVYXA_PUBLIC_` merely to silence a diagnostic. Move the read behind a
server module, action, loader, or API route instead.

### Verify the Boundary Rather Than Printing Values

Avoid commands that dump environment contents into logs. The useful checks are structural:

```bash
ruvyxa analyze --format human
npm run check
```

`analyze` reports a private variable that reaches the client graph; `check` exercises the project
readiness flow. Keep a committed `.env.example` with variable names and non-secret placeholders, and
exclude `.env.local` when it contains credentials.

---

## Next Steps

- [03-server-client-components.md](./03-server-client-components.md) -- Understanding the
  server/client boundary
- [11-configuration.md](./11-configuration.md) -- Full config reference for ruvyxa.config.ts
- [14-plugins.md](./14-plugins.md) -- requireEnv plugin and env validation
- [16-error-handling.md](./16-error-handling.md) -- RUV1008 and related errors
