# Deployment

Ruvyxa builds application output and, when selected, converts it into a platform-specific artifact.
The adapter does not deploy to a provider, run a health-gated promotion, or provide production
rollback orchestration.

## Build and serve flow

```bash
# Validate the project and inspect a target
ruvyxa check
ruvyxa doctor --adapter node

# Build the framework output and invoke the selected adapter
ruvyxa build --adapter node

# Serve an existing production build
ruvyxa start
# Or inspect it locally
ruvyxa preview
```

`start` and `preview` serve an existing build. They do not mean “build, stage, health-check, and
swap”. The CLI's build commit writes staging output, renames it into place, and restores the
previous framework output if that commit fails; this is a local build-safety mechanism, not a
production deployment service.

## Adapter selection

The repository ships these first-party adapter names:

| Name         | Package                      |
| ------------ | ---------------------------- |
| `node`       | `@ruvyxa/adapter-node`       |
| `bun`        | `@ruvyxa/adapter-bun`        |
| `static`     | `@ruvyxa/adapter-static`     |
| `vercel`     | `@ruvyxa/adapter-vercel`     |
| `netlify`    | `@ruvyxa/adapter-netlify`    |
| `cloudflare` | `@ruvyxa/adapter-cloudflare` |
| `railway`    | `@ruvyxa/adapter-railway`    |
| `render`     | `@ruvyxa/adapter-render`     |
| `firebase`   | `@ruvyxa/adapter-firebase`   |
| `aws`        | `@ruvyxa/adapter-aws`        |

Selection can be explicit on the CLI or configured in the project. The runtime runner may use
platform detection as a fallback, but dependency presence alone is not the documented selection
contract.

## Adapter examples

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'
import { nodeAdapter } from '@ruvyxa/adapter-node'

export default config({ adapter: nodeAdapter() })
```

The framework adapter contract is `build(ctx) -> AdapterOutput`. The authoritative capabilities are
the selected adapter's `supports` field. A declared capability is a compatibility check; it is not
evidence that a provider's limits or cache semantics have been tested for every application.

### Node

`@ruvyxa/adapter-node` emits under `.ruvyxa/deploy/node/`:

```text
server/index.mjs
public/                 # optional prerendered pages
start.mjs
README.md
```

Run the standalone server with:

```bash
node .ruvyxa/deploy/node/server/index.mjs
```

It honors `PORT` and `HOST`. The generated output is suitable for a Node host, a container, PM2,
systemd, or a PaaS; the adapter does not perform the final provider deployment.

### Static

`@ruvyxa/adapter-static` emits a static-site artifact in `static/` by default, or in the validated
relative `outputDir`. It supports `ssg` and `csr`; a static host cannot execute SSR, ISR, PPR, or
API routes. Dynamic SSG routes use the framework's `getStaticParams`/`staticParams` metadata.

```bash
ruvyxa build --adapter static
# publish the generated .ruvyxa/deploy/static/ artifact according to the adapter output
```

### AWS Amplify Hosting

The AWS adapter can emit `.amplify-hosting/` project output, including the static site, the
`compute/default` handler, and `deploy-manifest.json`. Its static artifact excludes `isr` and `ppr`.
Verify the generated manifest and the current Amplify runtime before deployment.

### Cloud and serverless targets

Vercel, Netlify, Cloudflare, Firebase, Railway, Render, and Bun have first-party adapter packages.
Their exact artifact layout and provider constraints are defined in each package's `src/index.ts`
and generated README. Do not infer a platform's ISR/PPR, filesystem, WebSocket, or cold-start
behavior from the package name; inspect the adapter output and provider limits.

## Docker

Docker is not a separate built-in adapter. Build with the Node or Bun adapter and copy the generated
server into the image:

```dockerfile
FROM node:22-alpine
WORKDIR /app
COPY .ruvyxa/deploy/node/ .
EXPOSE 3000
CMD ["node", "server/index.mjs"]
```

Choose the base image and runtime version for the generated artifact and test the image in the
target environment. No repository benchmark establishes a universal image size or throughput.

## CI example

CI should validate and build the artifact; deployment is a separate provider-specific step:

```yaml
steps:
  - run: pnpm install --frozen-lockfile
  - run: pnpm exec ruvyxa check
  - run: pnpm exec ruvyxa doctor --adapter node
  - run: pnpm exec ruvyxa build --adapter node
  - uses: actions/upload-artifact@v4
    with:
      name: ruvyxa-node-build
      path: .ruvyxa/
```

## Verification and performance

Use `ruvyxa analyze` for bundle inspection and `ruvyxa bench` for measurements on the actual
application and deployment shape. The repository does not provide universal targets for TTFB,
throughput, bundle size, image size, ROI, or timeline; record workload, hardware, adapter, and
sample size with any project-specific result.

## Troubleshooting

| Symptom                     | First verification                                                                           |
| --------------------------- | -------------------------------------------------------------------------------------------- |
| Adapter not found           | Check the selected name and package installation; run `ruvyxa doctor --adapter <name>`.      |
| Strategy unsupported        | Compare the route strategy with the adapter's `supports` field.                              |
| Static route missing        | Confirm the route is `ssg`/`csr` and dynamic routes expose `getStaticParams`/`staticParams`. |
| Output is not served        | Run `build` first; `start` and `preview` do not create a build.                              |
| Realtime build failure      | Use a long-lived Node/Bun target; the native realtime plugin rejects unsupported targets.    |
| Provider deployment failure | Inspect the generated artifact and provider logs; this is outside the adapter build step.    |

## Source of truth

- `packages/ruvyxa/runtime/adapter-runner.mjs`
- `packages/@ruvyxa/core/src/types.ts`
- `packages/@ruvyxa/adapter-*/src/index.ts`
- `crates/ruvyxa_cli/src/main.rs`

---

## Production contract and retained detail

The section above is the current, source-backed contract for this release. The original long-form
draft is retained below to preserve instructional context and audit history. It is non-normative: do
not copy its API snippets or capability claims unless they are revalidated against the current
source and package export map. This boundary is intentional so the document can retain its original
depth without presenting unsupported historical design as production behavior.

### English deployment draft — historical draft (non-normative)

> **Archive warning:** The material below is retained for history only. It is not the current
> deployment contract; provider behavior, commands, and benchmarks shown there are not promises. The
> source-backed contract above is authoritative.

# Deployment

Build once, deploy anywhere. Ruvyxa adapter system translates single build output into
platform-native artifacts — serverless functions, edge workers, static sites, or standalone servers
— without changing application code.

---

## What You Will Learn

- Complete `.ruvyxa/` build output structure
- `build.json` schema and contract
- Adapter architecture: build → inspect → transform → stage → atomic commit
- All 10 adapters with exact options, types, and output
- Adapter auto-detection via platform config files and environment variables
- Staging directory and atomic deployment commit algorithm
- Docker deployment patterns
- Production readiness checklist
- CI/CD integration for GitHub Actions and GitLab CI
- Troubleshooting every known deployment failure

---

## Build Output

`ruvyxa build` writes everything into `.ruvyxa/`. This is the universal build contract — every
adapter reads from this structure.

### Complete `.ruvyxa/` Directory Tree

```
.ruvyxa/
├── build.json                    # Adapter contract — every adapter reads this
├── manifest.json                 # Route manifest (server-side)
├── server/
│   ├── app/                      # Copied application source
│   │   ├── page.tsx
│   │   ├── layout.tsx
│   │   ├── about/
│   │   │   └── page.tsx
│   │   └── api/
│   │       └── hello/
│   │           └── route.ts
│   ├── components/               # Copied shared components
│   │   ├── Header.tsx
│   │   └── Footer.tsx
│   ├── server/                   # Server runtime framework code
│   │   ├── index.js              # Node.js production entry point
│   │   ├── serverless-handler.mjs  # Platform-agnostic serverless handler
│   │   ├── route-modules.mjs     # Route module registry
│   │   ├── manifest.mjs          # JS-importable manifest
│   │   └── runtime.js            # Server runtime utilities
│   └── index.js                  # Main server bundle
├── client/
│   ├── manifest.json             # Client bundle manifest
│   ├── _entry.js                 # Application entry point (hashed)
│   ├── _shared.js                # Shared chunk (hashed)
│   ├── _entry-[hash].js          # Versioned entry
│   ├── _shared-[hash].js         # Versioned shared chunk
│   ├── index.js                  # Page-specific bundle
│   ├── about.js                  # Route-specific bundle
│   └── [hash].js                 # Code-split async chunks
├── prerender/
│   ├── manifest.json             # Pre-render manifest
│   ├── index.html                # Pre-rendered /
│   ├── about/
│   │   └── index.html            # Pre-rendered /about
│   └── blog/
│       └── hello-world/
│           └── index.html        # Pre-rendered /blog/hello-world
├── data/
│   ├── index.json                # Prerender data for /
│   └── about.json                # Prerender data for /about
├── assets/
│   ├── images/                   # Optimized public images
│   │   ├── logo.webp             # WebP-converted
│   │   ├── logo.png              # Original preserved (configurable)
│   │   └── logo-640w.webp        # Responsive variant
│   ├── fonts/                    # Subsetted/bundled fonts
│   │   ├── Inter.woff2
│   │   └── Inter-Bold.woff2
│   └── styles.css                # Global CSS bundle
└── cache/                        # Build caches (internal)
    ├── compile/                  # Compiled module cache
    └── image/                    # Image optimization cache
```

### `build.json` — Full Schema

```typescript
interface BuildJson {
  /** Schema version. Currently 1. */
  version: number
  /** Build timestamp as Unix epoch milliseconds. */
  timestamp: number
  /** Total number of discovered routes. */
  routesCount: number
  /** SHA-256 hash of resolved configuration (adapter, security, build options). */
  configHash: string
  /** Adapter name used for this build. */
  adapter: string
  /** Route entries. */
  routes: BuildRoute[]
  /** Static asset paths (relative to .ruvyxa/). */
  assets: string[]
  /** Pre-rendered pathnames. */
  prerendered: string[]
  /** All configured client bundles. */
  clientBundles: string[]
  /** Image optimization report. */
  imageReport?: ImageReport
  /** Full adapter output configuration (varies by adapter). */
  config: Record<string, unknown>
}

interface BuildRoute {
  /** URL pathname. */
  path: string
  /** Rendering strategy: "ssr" | "ssg" | "isr" | "csr" | "ppr" */
  type: string
  /** Relative path to handler file. */
  file: string
  /** ISR revalidation seconds (only for isr type). */
  revalidate?: number
}

interface ImageReport {
  totalOriginalBytes: number
  totalOptimizedBytes: number
  savingsPercent: number
  optimizedCount: number
}
```

### `build.json` Example

```json
{
  "version": 1,
  "timestamp": 1743212345678,
  "routesCount": 5,
  "configHash": "a1b2c3d4e5f6...",
  "adapter": "vercel",
  "routes": [
    { "path": "/", "type": "ssr", "file": "server/index.js" },
    { "path": "/about", "type": "ssg", "file": "prerender/about/index.html" },
    { "path": "/blog/[slug]", "type": "isr", "file": "server/blog/[slug].js", "revalidate": 300 },
    { "path": "/api/hello", "type": "api", "file": "server/api/hello.js" },
    { "path": "/dashboard", "type": "csr", "file": "client/dashboard.js" }
  ],
  "assets": [
    "client/_entry-[hash].js",
    "client/_shared-[hash].js",
    "client/index.js",
    "assets/styles.css",
    "assets/images/logo.webp"
  ],
  "prerendered": ["/about", "/blog/hello-world"],
  "clientBundles": ["client/manifest.json"],
  "imageReport": {
    "totalOriginalBytes": 2450000,
    "totalOptimizedBytes": 890000,
    "savingsPercent": 63.7,
    "optimizedCount": 12
  },
  "config": { "adapter": "vercel" }
}
```

---

## Adapter System Architecture

```
 ┌──────────────┐
 │  ruvyxa      │
 │  build       │
 └──────┬───────┘
        │
        ▼
 ┌──────────────────────┐
 │  .ruvyxa/            │  ← Universal build output
 │  build.json          │     (platform-agnostic)
 └──────┬───────────────┘
        │
        ▼
 ┌──────────────────────────────────┐
 │  Adapter                          │
 │                                   │
 │  1. read build.json               │
 │  2. validate route compatibility  │
 │  3. produce artifacts + config    │
 │  4. write to staging directory    │
 │  5. atomic commit (rename)        │
 │                                   │
 │  ┌────────────────────────────┐   │
 │  │ Vercel    → .vercel/output/│   │
 │  │ Netlify   → .netlify/      │   │
 │  │ Cloudflare→ .cloudflare/   │   │
 │  │ Node      → dist/          │   │
 │  │ Bun       → dist/          │   │
 │  │ Static    → dist/          │   │
 │  │ AWS       → .amplify/      │   │
 │  │ Firebase  → .firebase/     │   │
 │  │ Railway   → dist/          │   │
 │  │ Render    → dist/          │   │
 │  └────────────────────────────┘   │
 └──────────────────────────────────┘
```

### Adapter Interface

```typescript
// @ruvyxa/core
interface Adapter {
  /** Unique adapter name. */
  name: string
  /** Deployment target category. */
  target: 'node' | 'edge' | 'serverless' | 'static'
  /** Supported rendering strategies. Omit = all supported. */
  supports?: Array<'ssr' | 'ssg' | 'isr' | 'csr' | 'ppr' | 'api'>
  /** Build function: reads context, returns output configuration. */
  build(ctx: BuildContext): AdapterOutput | Promise<AdapterOutput>
}

interface BuildContext {
  root: string
  outDir: string
  /** Override chunk manifest path. */
  chunkManifest?: string
}

interface AdapterOutput {
  name: string
  target: Adapter['target']
  entry: string
  assetsDir: string
  clientDir?: string
  chunkManifest?: string
  platform?:
    | 'node'
    | 'vercel'
    | 'cloudflare'
    | 'netlify'
    | 'bun'
    | 'static'
    | 'railway'
    | 'render'
    | 'firebase'
    | 'aws'
  runtime?: 'node' | 'bun'
  configFiles?: string[]
  functionsDir?: string
  artifacts?: AdapterArtifact[]
}

interface AdapterArtifact {
  kind: 'file' | 'static-site' | 'function'
  path: string
  contents?: string
  handlerSource?: string
  scope?: 'build' | 'project'
  skipIfExists?: boolean
  optional?: boolean
  excludeStrategies?: string[]
}
```

### Adapter Selection Logic

Ruvyxa resolves adapter in this priority:

1. **Explicit `adapter` config** — highest priority
2. **Auto-detection via platform config file** — checks project root
3. **Environment variable `RUVYXA_ADAPTER`** — fallback
4. **Default to Node adapter** — when nothing matches

#### Auto-Detection Table

| Platform   | Config File     | Env Var               | Default Adapter |
| ---------- | --------------- | --------------------- | --------------- |
| Vercel     | `vercel.json`   | `VERCEL`              | vercel          |
| Netlify    | `netlify.toml`  | `NETLIFY`             | netlify         |
| Cloudflare | `wrangler.toml` | `CF_PAGES`            | cloudflare      |
| Firebase   | `firebase.json` | `FIREBASE_CONFIG`     | firebase        |
| AWS        | —               | `AWS_EXECUTION_ENV`   | aws             |
| Railway    | `railway.json`  | `RAILWAY_ENVIRONMENT` | railway         |
| Render     | `render.yaml`   | `RENDER`              | render          |
| Node       | —               | —                     | node            |
| Bun        | —               | —                     | bun             |
| Static     | —               | —                     | static          |

```typescript
// A named adapter is selected with the CLI; a string is not a valid config.adapter value.
// ruvyxa doctor --adapter vercel
// ruvyxa build --adapter vercel

// Using adapter function (TypeScript-safe options)
import { vercelAdapter } from '@ruvyxa/adapter-vercel'

export default config({
  adapter: vercelAdapter({
    regions: ['sin1', 'hnd1'],
    maxDuration: 30,
  }),
})
```

---

### Adapter Reference

Complete table of every adapter with its function, target, supported rendering strategies, options
type, and deployment platform:

| Package                      | Function              | Target     | Supports                     | Options                                                                                      |
| ---------------------------- | --------------------- | ---------- | ---------------------------- | -------------------------------------------------------------------------------------------- |
| `@ruvyxa/adapter-vercel`     | `vercelAdapter()`     | serverless | SSR, SSG, ISR, CSR, PPR, API | `functionsDir`, `projectOutput: true`, `runtime: 'nodejs20.x'`, `maxDuration: 10`, `regions` |
| `@ruvyxa/adapter-node`       | `nodeAdapter()`       | node       | SSR, SSG, ISR, CSR, PPR, API | `entry`                                                                                      |
| `@ruvyxa/adapter-static`     | `staticAdapter()`     | static     | SSG, CSR                     | `outputDir: 'static'`                                                                        |
| `@ruvyxa/adapter-aws`        | `awsAdapter()`        | serverless | SSR, SSG, ISR, CSR, PPR, API | `runtime: 'nodejs22.x'`, `projectOutput: true`                                               |
| `@ruvyxa/adapter-bun`        | `bunAdapter()`        | node       | SSR, SSG, ISR, CSR, PPR, API | `entry`                                                                                      |
| `@ruvyxa/adapter-cloudflare` | `cloudflareAdapter()` | edge       | SSR, SSG, CSR, API           | `workerEntry`, `projectConfig: false`, `compatibilityDate: '2025-09-01'`                     |
| `@ruvyxa/adapter-firebase`   | `firebaseAdapter()`   | serverless | SSR, SSG, ISR, CSR, PPR, API | `functionName: 'ruvyxaServer'`, `region: 'us-central1'`, `projectConfig: true`               |
| `@ruvyxa/adapter-netlify`    | `netlifyAdapter()`    | serverless | SSR, SSG, ISR, CSR, PPR, API | `functionsDir`, `projectConfig: false`, `frameworksApi: true`                                |
| `@ruvyxa/adapter-railway`    | `railwayAdapter()`    | node       | SSR, SSG, ISR, CSR, PPR, API | `projectConfig: true`                                                                        |
| `@ruvyxa/adapter-render`     | `renderAdapter()`     | node       | SSR, SSG, ISR, CSR, PPR, API | `serviceName: 'ruvyxa-app'`, `projectConfig: true`                                           |

#### Adapter Details

**@ruvyxa/adapter-vercel**

- **Output**: `.vercel/output/` — Build Output API v3 format. Serverless function in
  `functions/__ruvyxa.func/`, static assets under `static/`.
- **Auto-detection**: `vercel.json` at project root or `VERCEL` env var.
- **Runtime dependency**: None. Uses platform `nodejs20.x` (configurable).
- **Error codes**: `RUV1700` (not installed), `RUV1704` (incompatible route).
- **Notes**: ISR uses `os.tmpdir()` for cache — persists for function instance lifetime. Supports
  preview deployments via Git integration.

**@ruvyxa/adapter-node**

- **Output**: `dist/` — standalone Node.js HTTP server with `server.js`, route modules, client
  assets, prerendered HTML, and a generated `package.json` for production dependency install.
- **Auto-detection**: None (default fallback).
- **Runtime dependency**: `ruvyxa` runtime package in generated `package.json`.
- **Errors**: entry failures are reported by the adapter/runtime; there is no dedicated diagnostic
  code for this case.
- **Notes**: Supports WebSocket via realtime plugin and cluster mode via PM2 or `node:cluster`.

**@ruvyxa/adapter-static**

- **Output**: `dist/` — flat static HTML, assets, `_redirects`, `404.html`, `sitemap.xml`,
  `robots.txt`.
- **Auto-detection**: None. Must be explicitly configured.
- **Runtime dependency**: None. Pure static files.
- **Errors**: unsupported output is reported by the adapter/runtime; there is no dedicated
  diagnostic code for this case.
- **Notes**: Only SSG and CSR routes are included. Routes using SSR, ISR, or PPR are excluded at
  build time with a warning.

**@ruvyxa/adapter-aws**

- **Output**: `.amplify/` — `amplify.yml` build spec, `dist/` for static assets,
  `functions/ruvyxa-server/` with Lambda bundle including `index.mjs`, `package.json`, and
  `node_modules/`.
- **Auto-detection**: `AWS_EXECUTION_ENV` env var.
- **Runtime dependency**: Dependencies bundled into Lambda zip.
- **Error codes**: `RUV1700`, `RUV1704`.
- **Notes**: Supports Lambda@Edge for SSR. ISR is planned via Lambda + CloudFront cache
  invalidation.

**@ruvyxa/adapter-bun**

- **Output**: `dist/` — single-file Bun server (`server.js`) and client assets. No `package.json`
  needed — Bun reads `bun.lock` from project root.
- **Auto-detection**: None. Must be explicitly configured.
- **Runtime dependency**: None required — leverages Bun's built-in transpiler, SQLite, and fast
  `fetch`.
- **Error codes**: `RUV1700`, `RUV1704`.
- **Notes**: Performance 2-4x faster than Node.js. Output is a self-contained single-file server.

**@ruvyxa/adapter-cloudflare**

- **Output**: `.cloudflare/` — Pages Functions handler (`__ruvyxa.js`), `_routes.json`, `_headers`,
  `_redirects`, static assets, SSG fallback HTML.
- **Auto-detection**: `wrangler.toml` at project root or `CF_PAGES` env var.
- **Runtime dependency**: None. Uses Cloudflare Workers runtime (workerd).
- **Error codes**: `RUV2210` (ISR/PPR rejected — needs KV/Durable Objects), `RUV1704`.
- **Notes**: Only SSR, SSG, CSR, and API strategies supported. ISR and PPR are rejected at build
  time. Worker memory limit: 128MB.

**@ruvyxa/adapter-firebase**

- **Output**: `.firebase/` — `firebase.json` (hosting config), `.firebaserc` (project alias),
  `dist/` (static assets + SSG), `functions/` (Cloud Function entry, `package.json`,
  `node_modules/`).
- **Auto-detection**: `firebase.json` at project root or `FIREBASE_CONFIG` env var.
- **Runtime dependency**: Dependencies bundled into Cloud Functions.
- **Error codes**: `RUV1700`, `RUV1704`.
- **Notes**: Uses Firebase Hosting rewrites to route all requests to Cloud Function. ISR/PPR not
  supported (no writable filesystem in Cloud Functions).

**@ruvyxa/adapter-netlify**

- **Output**: `.netlify/` — `deploy.config`, `dist/` (publish directory), `functions/__ruvyxa/`
  (serverless handler + route modules + prerender). Auto-generates `netlify.toml`.
- **Auto-detection**: `netlify.toml` at project root or `NETLIFY` env var.
- **Runtime dependency**: None. Platform provides Node.js runtime.
- **Error codes**: `RUV1700`, `RUV1704`.
- **Notes**: ISR and PPR not supported — Netlify does not provide a writable filesystem. Edge
  Functions available with `edgeFunctions: true`.

**@ruvyxa/adapter-railway**

- **Output**: `dist/` — Node server (`server.js`) plus `railway.json` for platform configuration
  including build command, start command, and health check path.
- **Auto-detection**: `railway.json` at project root or `RAILWAY_ENVIRONMENT` env var.
- **Runtime dependency**: `ruvyxa` runtime package in `package.json`.
- **Error codes**: `RUV1700`, `RUV1704`.
- **Notes**: Built on top of Node adapter output. Health check path defaults to `/api/health`. Uses
  Nixpacks builder.

**@ruvyxa/adapter-render**

- **Output**: `dist/` — Node server (`server.js`) plus `render.yaml` Blueprint with service name,
  plan, region, health check, and environment variables.
- **Auto-detection**: `render.yaml` at project root or `RENDER` env var.
- **Runtime dependency**: `ruvyxa` runtime package in `package.json`.
- **Error codes**: `RUV1700`, `RUV1704`.
- **Notes**: Built on top of Node adapter output. Supports plans: starter, professional, advanced.
  Regions: oregon, frankfurt, singapore, virginia.

---

## Vercel

```bash
npm i -D @ruvyxa/adapter-vercel
```

### Type Definitions

```typescript
interface VercelAdapterOptions {
  /** Custom functions output directory. Defaults to `${outDir}/functions`. */
  functionsDir?: string
  /** Emit Build Output API at project root (.vercel/output/). @default true */
  projectOutput?: boolean
  /** Node.js runtime version. @default 'nodejs20.x' */
  runtime?: string
  /** Max serverless function duration in seconds. @default 10 */
  maxDuration?: number
  /** Vercel region codes (e.g. ['sin1', 'iad1']). */
  regions?: string[]
}

function vercelAdapter(options?: VercelAdapterOptions): Adapter
```

### Configuration

```typescript
import { vercelAdapter } from '@ruvyxa/adapter-vercel'

export default config({
  adapter: vercelAdapter({
    regions: ['iad1', 'hnd1', 'sin1'],
    maxDuration: 30,
    runtime: 'nodejs22.x',
    projectOutput: true,
  }),
})
```

### Output Structure

```
.vercel/output/
├── config.json                # Vercel Build Output API config
├── static/                    # Static assets
│   ├── _entry-[hash].js
│   ├── _shared-[hash].js
│   ├── index.html             # SSG pages
│   ├── about/
│   │   └── index.html
│   └── assets/
│       ├── images/
│       └── styles.css
└── functions/
    └── __ruvyxa.func/
        ├── .vc-config.json    # Serverless function config
        ├── serverless-handler.mjs
        ├── route-modules.mjs
        ├── manifest.mjs
        └── prerender/         # Bundled prerender output
```

### .vc-config.json

```json
{
  "runtime": "nodejs20.x",
  "handler": "serverless-handler.mjs",
  "launcherType": "Nodejs",
  "maxDuration": 10,
  "regions": ["iad1"]
}
```

### Feature Support

| Feature             | Support | Notes                            |
| ------------------- | ------- | -------------------------------- |
| SSR                 | ✅      | Via serverless function          |
| API Routes          | ✅      | Via serverless function          |
| SSG                 | ✅      | Served from edge CDN             |
| ISR                 | ✅      | Uses `os.tmpdir()` for ISR cache |
| PPR                 | ✅      | Partial pre-rendering            |
| CSR                 | ✅      | Minimal shell + client hydrate   |
| Edge Functions      | 🔜      | Coming in a future release       |
| Image Optimization  | ✅      | With `image.optimize: true`      |
| Preview Deployments | ✅      | Automatic with Git integration   |
| Server Actions      | ✅      | Via POST to function             |
| Middleware          | ✅      | Via function prefix              |

### ISR Behavior on Vercel

On first request after deploy, ISR reads bundled pre-rendered HTML. After revalidation, writes
updated HTML to `os.tmpdir()/ruvyxa-isr-cache/`. Cache persists for function instance lifetime. Cold
starts read bundled snapshot.

---

## Netlify

```bash
npm i -D @ruvyxa/adapter-netlify
```

### Type Definitions

```typescript
interface NetlifyAdapterOptions {
  /** Custom functions output directory. Defaults to `${outDir}/functions`. */
  functionsDir?: string
  /** Enable edge functions. @default false */
  edgeFunctions?: boolean
  /** Node.js version for serverless functions. @default 'nodejs20' */
  nodeVersion?: string
  /** Publish directory for static assets. @default 'dist' */
  publishDir?: string
}

function netlifyAdapter(options?: NetlifyAdapterOptions): Adapter
```

### Configuration

```typescript
import { netlifyAdapter } from '@ruvyxa/adapter-netlify'

export default config({
  adapter: netlifyAdapter({
    edgeFunctions: false,
    nodeVersion: 'nodejs20',
  }),
})
```

### Output Structure

```
.netlify/
├── deploy.config              # Netlify deploy configuration
├── dist/                      # Static assets + SSG pages
│   ├── _entry.js
│   ├── index.html
│   ├── about/index.html
│   └── assets/
└── functions/
    └── __ruvyxa/
        ├── serverless-handler.mjs
        ├── route-modules.mjs
        ├── manifest.mjs
        └── prerender/
```

### netlify.toml (auto-generated)

```toml
[build]
  command = "npx ruvyxa build"
  publish = ".netlify/dist"
  functions = ".netlify/functions"

[build.processing]
  skip_processing = false

[[redirects]]
  from = "/*"
  to = "/.netlify/functions/__ruvyxa"
  status = 200

[[headers]]
  for = "/assets/*"
  [headers.values]
    Cache-Control = "public, max-age=31536000, immutable"
```

### Feature Support

| Feature             | Support | Notes                                         |
| ------------------- | ------- | --------------------------------------------- |
| SSR                 | ✅      | Via serverless function                       |
| API Routes          | ✅      | Via serverless function                       |
| SSG                 | ✅      | Static files in publish dir                   |
| ISR                 | ❌      | Netlify does not support writeable filesystem |
| PPR                 | ❌      | Requires ISR support                          |
| CSR                 | ✅      | Client hydration                              |
| Edge Functions      | ✅      | With `edgeFunctions: true`                    |
| Netlify Forms       | ✅      | Compatible with server actions                |
| Split Testing       | ✅      | Branch-based deploys                          |
| Preview Deployments | ✅      | Automatic                                     |

### Deploy Commands

```bash
# Build
ruvyxa build

# Deploy via CLI
netlify deploy --prod

# Or push to Git with Netlify integration
```

---

## Cloudflare

```bash
npm i -D @ruvyxa/adapter-cloudflare
```

### Type Definitions

```typescript
interface CloudflareAdapterOptions {
  /** Custom worker entry path. Defaults to `${outDir}/server/app`. */
  workerEntry?: string
  /** Emit wrangler.jsonc at project root. @default false */
  projectConfig?: boolean
  /** Workers compatibility date. @default '2025-09-01' */
  compatibilityDate?: string
}

function cloudflareAdapter(options?: CloudflareAdapterOptions): Adapter
```

### Configuration

```typescript
import { cloudflareAdapter } from '@ruvyxa/adapter-cloudflare'

export default config({
  adapter: cloudflareAdapter({
    compatibilityDate: '2025-09-01',
    projectConfig: false,
  }),
})
```

### Output Structure

```
.cloudflare/
├── functions/                  # Cloudflare Pages Functions
│   └── __ruvyxa.js
├── _routes.json                # Route configuration
├── _headers                    # Custom headers
├── _redirects                  # Redirects
├── index.html                  # SSG fallback
└── assets/                     # Static files
    ├── _entry-[hash].js
    ├── _shared-[hash].js
    ├── images/
    └── styles.css
```

### Worker Handler (generated)

The adapter generates a Worker fetch handler that imports the generic serverless handler and route
manifest. Static assets are served by Cloudflare's `assets` binding — the Worker only handles
dynamic routes.

Supported strategies: `['ssr', 'ssg', 'csr', 'api']`

ISR and PPR are rejected at build time with error code **RUV2210** because they require persistent
storage (KV/Durable Objects) not yet integrated.

### Deploy Commands

```bash
ruvyxa build

# Deploy to Cloudflare Pages
wrangler pages deploy .cloudflare

# Deploy to Workers (with projectConfig: true)
wrangler deploy -c .ruvyxa/deploy/cloudflare/wrangler.jsonc
```

### _routes.json

```json
{
  "version": 1,
  "include": ["/*"],
  "exclude": ["/assets/*", "/_entry*", "/_shared*"]
}
```

### Feature Support

| Feature            | Support | Notes                      |
| ------------------ | ------- | -------------------------- |
| SSR via Pages Func | ✅      | Worker fetch handler       |
| API Routes         | ✅      | Via Worker                 |
| SSG                | ✅      | Static assets binding      |
| ISR                | ❌      | Needs KV integration       |
| PPR                | ❌      | Needs KV integration       |
| CSR                | ✅      | Client hydration           |
| Workers            | ✅      | Full Workers compatibility |
| Durable Objects    | 🔜      | Future release             |

---

## Node.js

```bash
npm i -D @ruvyxa/adapter-node
```

### Type Definitions

```typescript
interface NodeAdapterOptions {
  /** Entry point filename. @default 'server.js' */
  entry?: string
  /** Output directory. @default 'dist' */
  outputDir?: string
  /** Enable compression middleware. @default true */
  compress?: boolean
}

function nodeAdapter(options?: NodeAdapterOptions): Adapter
```

### Configuration

```typescript
import { nodeAdapter } from '@ruvyxa/adapter-node'

export default config({
  adapter: nodeAdapter({
    compress: true,
  }),
})
```

### Output Structure

```
dist/
├── server.js                  # Entry point — start this
├── package.json               # Runtime dependencies
├── server/                    # Route handlers
│   ├── index.js
│   ├── about.js
│   ├── blog/[slug].js
│   └── api/hello.js
├── client/                    # Static assets
│   ├── _entry-[hash].js
│   ├── _shared-[hash].js
│   └── manifest.json
├── assets/
│   ├── images/
│   └── styles.css
└── prerender/                 # Pre-rendered HTML
    ├── index.html
    └── about/index.html
```

### Generated package.json

```json
{
  "name": "ruvyxa-app",
  "private": true,
  "type": "module",
  "dependencies": {
    "ruvyxa": "^2.0.0"
  }
}
```

### Running

```bash
node dist/server.js
# Listens on PORT env var (default 3000)
```

### Production Deployment

```bash
# Set production environment
export NODE_ENV=production
export PORT=8080
export DATABASE_URL=postgres://...

# Start server
node dist/server.js
```

### Feature Support

| Feature        | Support | Notes                   |
| -------------- | ------- | ----------------------- |
| SSR            | ✅      | HTTP server             |
| API Routes     | ✅      | HTTP server             |
| SSG            | ✅      | Pre-rendered files      |
| ISR            | ✅      | In-memory + disk cache  |
| PPR            | ✅      | Streaming supported     |
| CSR            | ✅      | Client hydration        |
| WebSocket      | ✅      | Via realtime plugin     |
| Server Actions | ✅      | POST handler            |
| Cluster Mode   | ✅      | Via PM2 or node:cluster |

---

## Bun

```bash
npm i -D @ruvyxa/adapter-bun
```

### Type Definitions

```typescript
interface BunAdapterOptions {
  /** Entry point filename. @default 'server.js' */
  entry?: string
  /** Output directory. @default 'dist' */
  outputDir?: string
}

function bunAdapter(options?: BunAdapterOptions): Adapter
```

### Configuration

```typescript
import { bunAdapter } from '@ruvyxa/adapter-bun'

export default config({
  adapter: bunAdapter(),
})
```

### Output Structure

```
dist/
├── server.js                  # Single-file Bun server
└── client/                    # Static assets
    ├── _entry.js
    └── assets/
```

The Bun adapter generates a single-file server that leverages Bun's runtime (built-in transpiler,
SQLite, faster `fetch`). No `package.json` needed — Bun reads `bun.lock` from the project root.

### Running

```bash
bun run dist/server.js
```

### Feature Support

| Feature        | Support | Notes                |
| -------------- | ------- | -------------------- |
| SSR            | ✅      | Bun HTTP server      |
| API Routes     | ✅      | Bun HTTP server      |
| SSG            | ✅      | Static files served  |
| ISR            | ✅      | Bun filesystem cache |
| PPR            | ✅      | Streaming            |
| CSR            | ✅      | Client hydration     |
| Bun SQLite     | ✅      | Out of box           |
| Server Actions | ✅      | POST handler         |

---

## Static Hosting

Any platform serving static files: S3 + CloudFront, GitHub Pages, Surge.sh, Netlify (static mode).

```bash
npm i -D @ruvyxa/adapter-static
```

### Type Definitions

```typescript
interface StaticAdapterOptions {
  /** Output directory. @default 'dist' */
  outputDir?: string
  /** Trailing slash behavior. @default false */
  trailingSlash?: boolean
  /** 404 fallback page. @default '404.html' */
  notFoundPage?: string
  /** Clean output directory before writing. @default true */
  clean?: boolean
}

function staticAdapter(options?: StaticAdapterOptions): Adapter
```

### Configuration

```typescript
import { staticAdapter } from '@ruvyxa/adapter-static'

export default config({
  adapter: staticAdapter({
    trailingSlash: false,
    notFoundPage: '404.html',
  }),
})
```

### Output Structure

```
dist/
├── index.html              # /
├── about/
│   └── index.html          # /about (SSG)
├── blog/
│   └── hello-world/
│       └── index.html      # /blog/hello-world
├── _redirects              # Netlify-style redirects
├── 404.html                # Custom 404 page
├── sitemap.xml             # Auto-generated sitemap
├── robots.txt              # Auto-generated robots
├── assets/
│   ├── images/
│   │   ├── logo.webp
│   │   └── logo.png
│   ├── fonts/
│   └── styles.css
└── client/
    ├── _entry.js
    └── _shared.js
```

### Deploy Examples

```bash
# S3 + CloudFront
aws s3 sync dist/ s3://my-bucket --delete
aws cloudfront create-invalidation --distribution-id XYZ --paths "/*"

# GitHub Pages
npx gh-pages -d dist

# Surge.sh
surge dist/ my-site.surge.sh

# Netlify (static)
npx netlify deploy --prod --dir=dist
```

### Compatibility Requirements

Static mode requires SSG or CSR rendering strategy for every route. Routes using SSR, ISR, or PPR
are excluded from output with a warning.

```bash
# Verify route compatibility
ruvyxa check
# Shows: "Route /api/users: incompatible with static adapter (type: api)"
```

---

## AWS (Amplify Hosting)

```bash
npm i -D @ruvyxa/adapter-aws
```

### Type Definitions

```typescript
interface AwsAdapterOptions {
  /** Amplify app ID. */
  appId?: string
  /** AWS region. @default 'us-east-1' */
  region?: string
  /** Package version for Lambda. @default '1.0.0' */
  version?: string
}

function awsAdapter(options?: AwsAdapterOptions): Adapter
```

### Output Structure

```
.amplify/
├── amplify.yml                # Amplify build spec
├── dist/                      # Static assets
└── functions/
    └── ruvyxa-server/
        ├── index.mjs
        ├── package.json
        └── node_modules/
```

### Feature Support

| Feature | Support | Notes             |
| ------- | ------- | ----------------- |
| SSR     | ✅      | Lambda@Edge       |
| API     | ✅      | Lambda function   |
| SSG     | ✅      | CloudFront static |
| CSR     | ✅      | Client hydration  |
| ISR     | 🔜      | Lambda + CF cache |

---

## Firebase

```bash
npm i -D @ruvyxa/adapter-firebase
```

### Type Definitions

```typescript
interface FirebaseAdapterOptions {
  /** Cloud Function name. @default 'ruvyxa' */
  functionName?: string
  /** GCP region. @default 'us-central1' */
  region?: string
  /** Memory allocation. @default '512Mi' */
  memory?: string
  /** Min instances for cold-start mitigation. @default 0 */
  minInstances?: number
  /** Max instances. @default 100 */
  maxInstances?: number
}

function firebaseAdapter(options?: FirebaseAdapterOptions): Adapter
```

### Output Structure

```
.firebase/
├── firebase.json              # Firebase Hosting config
├── .firebaserc                # Project alias
├── dist/                      # Static assets + SSG
└── functions/
    ├── package.json
    ├── index.js               # Cloud Function entry
    └── node_modules/
```

### firebase.json

```json
{
  "hosting": {
    "public": "dist",
    "ignore": ["firebase.json", "**/.*"],
    "rewrites": [{ "source": "**", "function": "ruvyxa" }]
  },
  "functions": {
    "source": "functions"
  }
}
```

### Feature Support

| Feature | Support | Notes            |
| ------- | ------- | ---------------- |
| SSR     | ✅      | Cloud Function   |
| API     | ✅      | Cloud Function   |
| SSG     | ✅      | Firebase Hosting |
| CSR     | ✅      | Client hydrate   |
| ISR     | ❌      | No writeable fs  |
| PPR     | ❌      | No writeable fs  |

---

## Railway

```bash
npm i -D @ruvyxa/adapter-railway
```

### Type Definitions

```typescript
interface RailwayAdapterOptions {
  /** Start command override. Defaults to 'node dist/server.js' */
  startCommand?: string
  /** Health check path. @default '/api/health' */
  healthcheckPath?: string
}

function railwayAdapter(options?: RailwayAdapterOptions): Adapter
```

### Auto-Detection

Railway sets `RAILWAY_ENVIRONMENT` env var. If found, the Node adapter output is enhanced with
Railway-specific `railway.json`.

### Output

```
dist/
├── server.js
├── railway.json
├── client/
└── assets/
```

### railway.json

```json
{
  "$schema": "https://railway.app/railway.schema.json",
  "build": {
    "builder": "nixpacks",
    "buildCommand": "npx ruvyxa build"
  },
  "deploy": {
    "startCommand": "node dist/server.js",
    "healthcheckPath": "/api/health",
    "restartPolicyType": "on-failure"
  }
}
```

---

## Render

```bash
npm i -D @ruvyxa/adapter-render
```

### Type Definitions

```typescript
interface RenderAdapterOptions {
  /** Service name. Must be lowercase with hyphens. */
  serviceName?: string
  /** Start command. Defaults to 'node dist/server.js' */
  startCommand?: string
  /** Health check path. @default '/health' */
  healthCheckPath?: string
  /** Instance type. @default 'starter' */
  plan?: 'starter' | 'professional' | 'advanced'
  /** Region. @default 'oregon' */
  region?: 'oregon' | 'frankfurt' | 'singapore' | 'virginia'
}

function renderAdapter(options?: RenderAdapterOptions): Adapter
```

### Auto-Detection

Render sets `RENDER` env var. If found, the adapter auto-detects and writes `render.yaml`.

### Output

```
dist/
├── server.js
├── render.yaml                # Blueprint
├── client/
└── assets/
```

### render.yaml

```yaml
services:
  - type: web
    name: my-app
    env: node
    buildCommand: npx ruvyxa build
    startCommand: node dist/server.js
    healthCheckPath: /health
    plan: starter
    region: oregon
    envVars:
      - key: NODE_ENV
        value: production
```

---

## Staging + Atomic Commit

All adapters use a staging directory and atomic rename to prevent partial deployments:

### Algorithm

```
1. Adapter writes output to  .ruvyxa/.staging/<adapter>/
2. If target directory exists → rename to  .<name>.old/
3. Rename .staging/<adapter>/ → target directory
4. On failure → restore .<name>.old/ → target directory
5. Remove .<name>.old/ on success
```

This ensures a failed deployment never leaves a partially-written directory that causes 502 errors.

### Example (Vercel)

```
Step 1: Write to .ruvyxa/.staging/vercel/
Step 2: If .vercel/output/ exists, rename to .vercel/.output.old/
Step 3: Rename .ruvyxa/.staging/vercel/ → .vercel/output/
Step 4: Remove .vercel/.output.old/
```

---

## Docker

Use the Node adapter inside a multi-stage Docker build:

### Dockerfile

```dockerfile
# ---- Build Stage ----
FROM node:22-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npx ruvyxa build

# ---- Runner Stage ----
FROM node:22-alpine AS runner
WORKDIR /app
# Copy built output
COPY --from=builder /app/dist ./dist
# Copy production dependencies
COPY --from=builder /app/package*.json ./
RUN npm ci --production --ignore-scripts
# Expose port
EXPOSE 3000
# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD node -e "fetch('http://localhost:3000/api/health').then(r => process.exit(r.ok?0:1))"
# Start server
CMD ["node", "dist/server.js"]
```

### Build and Run

```bash
docker build -t my-app:latest .
docker run -p 3000:3000 \
  -e DATABASE_URL=postgres://... \
  -e AUTH_SECRET=... \
  -e RUVYXA_PUBLIC_URL=https://example.com \
  my-app:latest
```

### docker-compose.yml

```yaml
version: '3.9'
services:
  app:
    build: .
    ports:
      - '3000:3000'
    environment:
      - NODE_ENV=production
      - DATABASE_URL=${DATABASE_URL}
      - AUTH_SECRET=${AUTH_SECRET}
      - RUVYXA_PUBLIC_URL=https://example.com
    healthcheck:
      test:
        [
          'CMD',
          'node',
          '-e',
          "fetch('http://localhost:3000/api/health').then(r => process.exit(r.ok?0:1))",
        ]
      interval: 30s
      timeout: 3s
      retries: 3
    restart: unless-stopped
```

---

## Production Checklist

### Before You Deploy

- [ ] `ruvyxa check` — zero errors
- [ ] `ruvyxa test:parity` — dev/prod behavior matches
- [ ] `npm run typecheck` — TypeScript clean
- [ ] `ruvyxa build` — build succeeds
- [ ] `ruvyxa start` — production server responds
- [ ] `ruvyxa analyze` — bundle sizes acceptable
- [ ] `ruvyxa bench` — critical routes meet latency targets
- [ ] `debug.overlay: false` in production config
- [ ] `build.minify: true` for production
- [ ] `build.map: false` unless debugging source maps
- [ ] Load test with real traffic patterns

### Environment Variables

| What         | Convention          | How to Set                              |
| ------------ | ------------------- | --------------------------------------- |
| Public vars  | `RUVYXA_PUBLIC_*`   | Set in platform dashboard or CI         |
| Private vars | Any name            | Set in platform secret manager          |
| Secrets      | Any name            | Never commit — use vault/secret store   |
| Site URL     | `RUVYXA_PUBLIC_URL` | Full origin, e.g. `https://example.com` |

```bash
# Vercel
vercel env add DATABASE_URL
vercel env add RUVYXA_PUBLIC_URL

# Netlify
netlify env:set DATABASE_URL postgres://...
netlify env:set RUVYXA_PUBLIC_URL https://example.com

# CI
DATABASE_URL=${{ secrets.DATABASE_URL }} npx ruvyxa build
```

### Security Hardening

- [ ] `security.sameOrigin: true` — CSRF prevention
- [ ] `security.headers` — security headers on every response
- [ ] `security.actionLimit` — max action payload (default 1 MiB)
- [ ] `security.apiLimit` — max API payload (default 10 MiB)
- [ ] `security.actionRateLimit` — per-client rate limiting
- [ ] `requireEnv` plugin — validate required vars at startup
- [ ] `securityHeaders` plugin — CSP, HSTS, permissions policy
- [ ] `trustedProxyIps` — if behind load balancer
- [ ] `security.fetchMeta` — strip `X-Fetch-Meta` from requests

### Body Limits

| Config                 | Default    | Max         | Purpose                    |
| ---------------------- | ---------- | ----------- | -------------------------- |
| `security.actionLimit` | 1,048,576  | 10,485,760  | Server action payload      |
| `security.apiLimit`    | 10,485,760 | ∞           | API route payload          |
| `security.pluginLimit` | 33,554,432 | 268,435,456 | Response middleware buffer |

### SSL/TLS

- Vercel / Netlify / Cloudflare: automatic TLS
- Node standalone: use reverse proxy (nginx, Caddy, HAProxy)
- Docker: terminate TLS at load balancer
- Always use `https://` for `site.url`

### Monitoring

- Enable `observability` plugin for request IDs + W3C trace context
- Set up platform health checks (`/api/health`)
- Configure error reporting (Sentry, Datadog, etc.)
- Monitor cold starts: `RUV1704` if workers fail to initialize
- Watch ISR cache disk usage (Vercel: `os.tmpdir()`)

### Performance

- Set `build.warm: true` to precompile route modules in background
- Use `build.prerenderCache: true` (default) for fast rebuilds
- Configure `cache.dir` for shared compile cache across CI runs
- Set `build.workers` to CPU count for parallel compilation
- Enable `image.optimize` for WebP conversion

---

## Setting Environment Variables in Production

| Platform   | วิธีตั้งค่า                                            |
| ---------- | ------------------------------------------------------ |
| Vercel     | Dashboard → Project → Settings → Environment Variables |
| Netlify    | Site settings → Build & deploy → Environment variables |
| Cloudflare | Pages → Project → Settings → Environment variables     |
| Railway    | Dashboard → Variables → New Variable                   |
| Render     | Dashboard → Environment → Secret Files                 |
| Docker     | `-e` flags หรือ `--env-file`                           |
| AWS Lambda | AWS Console → Lambda → Environment variables           |
| Firebase   | Firebase Console → Functions → Environment variables   |

### Production-specific Variables File

```bash
# .env.production — ใช้ตอน build เท่านั้น
RUVYXA_PUBLIC_API_URL=https://api.production.com
RUVYXA_PUBLIC_SITE_URL=https://production.com
DATABASE_URL=postgres://prod-user:****@prod-host/db
AUTH_SECRET=production-secret
```

**ข้อสำคัญ**: อย่า commit `.env.production` ลง git — ใช้ platform's secret manager สำหรับ production
secrets

---

NOT FOUND: ## ruvyxa doctor --adapter

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  NODE_VERSION: 22

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
          cache: 'npm'
      - run: npm ci
      - run: npx ruvyxa check
      - run: npx ruvyxa test:parity
      - run: npm run typecheck

  build:
    needs: validate
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
          cache: 'npm'
      - run: npm ci
      - run: npx ruvyxa build
        env:
          DATABASE_URL: ${{ secrets.DATABASE_URL }}
          AUTH_SECRET: ${{ secrets.AUTH_SECRET }}
          RUVYXA_PUBLIC_URL: ${{ vars.RUVYXA_PUBLIC_URL }}
      - uses: actions/upload-artifact@v4
        with:
          name: build-output
          path: .ruvyxa/

  deploy-vercel:
    needs: build
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          name: build-output
          path: .ruvyxa/
      - name: Deploy to Vercel
        uses: amondnet/vercel-action@v25
        with:
          vercel-token: ${{ secrets.VERCEL_TOKEN }}
          vercel-org-id: ${{ secrets.VERCEL_ORG_ID }}
          vercel-project-id: ${{ secrets.VERCEL_PROJECT_ID }}
          vercel-args: '--prod'
```

### GitLab CI

```yaml
# .gitlab-ci.yml
image: node:22

variables:
  NPM_CONFIG_CACHE: '$CI_PROJECT_DIR/.npm'

cache:
  key: ${CI_COMMIT_REF_SLUG}
  paths:
    - .npm/
    - .ruvyxa/cache/

stages:
  - check
  - build
  - deploy

check:
  stage: check
  script:
    - npm ci
    - npx ruvyxa check
    - npx ruvyxa test:parity

build:
  stage: build
  script:
    - npm ci
    - npx ruvyxa build
  artifacts:
    paths:
      - .ruvyxa/
    expire_in: 1 hour

deploy:
  stage: deploy
  script:
    - npx ruvyxa build
    - npx ruvyxa plugin deploy
  only:
    - main
  environment:
    name: production
    url: https://example.com
```

---

## Troubleshooting

| Problem                             | Cause                               | Fix                                              |
| ----------------------------------- | ----------------------------------- | ------------------------------------------------ |
| Build fails in CI                   | Missing env vars                    | Set secrets in CI dashboard secrets              |
| 404 on all routes                   | Adapter not set / wrong output dir  | Verify `adapter` config, check `.ruvyxa/` output |
| Static site blank                   | SSR routes in static mode           | Use SSG strategy or switch to Node adapter       |
| Images not optimized                | `image.optimize` off                | Enable in config and rebuild                     |
| Vercel 502 timeout                  | Serverless function too slow        | Increase `maxDuration` or optimize route         |
| Netlify function cold start         | No warm requests                    | Enable `build.warm: true`                        |
| "Adapter not found"                 | Missing adapter package             | `npm install @ruvyxa/adapter-<name>`             |
| RUV1700: Adapter not found          | Adapter package not installed       | Install the matching adapter                     |
| RUV1701: Adapter build failed       | Adapter incompatibility             | Check adapter version, framework version         |
| RUV1702: Manifest failed            | Disk space / permissions            | Free space, check write permissions              |
| Adapter configuration issue         | Adapter is missing or misconfigured | Check the adapter package and `ruvyxa.config.ts` |
| RUV1704: Adapter incompatible       | Strategy not supported              | Use different strategy or adapter                |
| Docker: Connection refused          | Port mismatch                       | Check EXPOSE and `server.port` config            |
| Docker: Healthcheck fails           | Missing /api/health route           | Add health check API route                       |
| Static: Pre-rendered page not found | Missing `getStaticParams`           | Export params for dynamic routes                 |
| Cloudflare: 1014                    | Worker exceeds CPU                  | Optimize SSR path, enable caching                |
| Firebase: Function timeout          | Too much work in SSR/API            | Increase function timeout or optimize            |

---

## Production Performance Benchmarks

```bash
ruvyxa bench
```

| Metric                  | ค่าเป้าหมาย | ค่าที่ควรได้ |
| ----------------------- | ----------- | ------------ |
| SSR Response Time (p50) | < 100ms     | 45ms         |
| SSR Response Time (p99) | < 500ms     | 280ms        |
| TTFB (First Byte)       | < 200ms     | 120ms        |
| Throughput (req/s)      | > 1000      | 2450         |
| Bundle Size (initial)   | < 200KB     | 128KB        |
| Bundle Size (total)     | < 500KB     | 340KB        |
| Asset Size (images)     | < 1MB       | 680KB        |
| Prerender Time/page     | < 1s        | 0.3s         |

---

## Migrating Production URL

เมื่อย้าย production URL:

```ts
// ruvyxa.config.ts
// 1. อัปเดต site.url
site: {
  url: 'https://new-domain.com',
  previousUrl: 'https://old-domain.com',  // สำหรับ redirect
}

// 2. ตั้ง redirects plugin
plugins: [
  {
    name: 'redirects',
    options: {
      redirects: [
        { source: '/(.*)', destination: 'https://new-domain.com/$1', permanent: true },
      ],
    },
  },
]
```

ตรวจสอบ:

```bash
curl -I https://old-domain.com/about
# ควรได้: 301 → https://new-domain.com/about
```

---

## Error Codes (RUV1700-1799)

| Code    | Title                       | Source                                | Fix                          |
| ------- | --------------------------- | ------------------------------------- | ---------------------------- |
| RUV1700 | Adapter not found           | CLI adapter resolution                | Install adapter package      |
| RUV1701 | Adapter build failed        | Adapter `build()`                     | Check adapter compatibility  |
| RUV1702 | Adapter manifest failed     | Build JSON write                      | Check disk/permissions       |
| RUV2200 | Adapter runner failure      | Invalid adapter output                | Check adapter implementation |
| RUV2202 | Unsupported adapter target  | Target is incompatible                | Choose a supported target    |
| RUV2203 | Adapter package unavailable | Package cannot resolve                | Install or correct adapter   |
| RUV2210 | Strategy unsupported        | Platform cannot render route strategy | Choose a supported strategy  |

---

## Deployment Decisions That the CLI Can Verify

## Try It Yourself

1. รัน `ruvyxa build` แล้วดูโครงสร้าง `.ruvyxa/` — ทำความเข้าใจแต่ละ directory
2. รัน `ruvyxa build --adapter static` → เปิด `.ruvyxa/index.html` ใน browser
3. ทดสอบ `ruvyxa doctor --adapter vercel` — ดู warning ที่แนะนำ
4. สร้าง Dockerfile และ docker-compose.yml สำหรับ production
5. ตั้งค่า CI/CD ด้วย GitHub Actions — รวม quality + build + deploy
6. Deploy ไปยัง platform ที่เลือก — ใช้ staging ก่อน production
7. ทดสอบ `ruvyxa deploy:stage && ruvyxa deploy:swap`
8. ตรวจ production checklist ทุกข้อก่อน deploy จริง
9. รัน `ruvyxa bench` และ `ruvyxa analyze` หลัง deploy
10. ตั้ง monitoring: uptime check, error tracking, performance alert

---

## Summary

- Build output อยู่ที่ `.ruvyxa/` — 8 directories พร้อม metadata ใน `build.json`
- 10 adapters — vercel, netlify, cloudflare, node, bun, static, railway, render, firebase, aws
- Auto-detect จาก platform environment variables — 8 env vars ที่รู้จัก
- Adapter auto-detection algorithm — 6 ขั้นตอน จาก env → config → CLI → fallback
- Staging deploy system — blue-green, atomic swap, rollback
- Production checklist — 12 ข้อจาก env vars ถึง monitoring
- Docker multi-stage build — production image ~100MB
- CI/CD พร้อม GitHub Actions และ GitLab CI
- 12 adapter-specific troubleshooting entries
- Performance benchmarks — TTFB < 200ms, throughput > 1000 req/s

---

Ruvyxa supports the built-in adapter names `node`, `bun`, `static`, `vercel`, `netlify`,
`cloudflare`, `railway`, `render`, `firebase`, and `aws`. Each name has a corresponding first-party
adapter package in this repository, and `--adapter` may also accept a syntactically valid npm
package name for a third-party adapter. Selection is an explicit build decision:

```bash
ruvyxa doctor --target node --adapter node
ruvyxa build --target node --adapter node
ruvyxa doctor --target static --adapter static
ruvyxa build --target static --adapter static
```

Run `doctor` first when evaluating a target; it inspects compatibility without materializing adapter
artifacts. `build` performs the build and invokes the selected adapter. `start` and `preview` only
serve an existing production build, so neither is a substitute for the build command.

### Automatic Platform Selection Is a Fallback

When no adapter is set by the command or configuration, the CLI can select one from its build
environment: `VERCEL`, `NETLIFY`, `CF_PAGES`, `RAILWAY_PROJECT_ID`, `RENDER`, or `AWS_APP_ID`.
`RUVYXA_ADAPTER` has priority over those platform markers when it contains a valid adapter name.
This is a convenience for a known host, not a reason to omit explicit adapter checks from CI.

### A Small, Reproducible Deployment Gate

```bash
npm run check
ruvyxa analyze --format sarif --output reports/ruvyxa.sarif
ruvyxa doctor --adapter cloudflare --json
ruvyxa build --adapter cloudflare
```

Create `reports/` before writing an analysis file. Keep the target and adapter consistent with the
environment that will execute the output. An edge/static adapter may reject a route strategy that a
Node adapter can serve; treat that as a design constraint of the deployment, not as a warning to
ignore.

### Separate Framework Output From Host Configuration

The framework creates its configured output directory (the minimal starter uses `.ruvyxa`). Domain
names, TLS certificates, secret provisioning, DNS, traffic splitting, and provider dashboards are
owned by the hosting platform. Document and automate those platform operations in the host's own
repository configuration; do not claim that `ruvyxa build` performs an external deployment unless
the selected adapter's package explicitly does so.

---

## Next Steps

- **[11-configuration.md](./11-configuration.md)** — adapter and adapterOptions config
- **[12-cli-commands.md](./12-cli-commands.md)** — build and deploy commands
- **[14-plugins.md](./14-plugins.md)** — sitemap, robots, and deploy plugins
- **[16-error-handling.md](./16-error-handling.md)** — deploy error codes
