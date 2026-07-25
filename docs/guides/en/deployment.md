# Deployment

> 🟢 **Quick Deploy is beginner friendly** · ⏱️ ~8 min read (2 min for Quick Deploy alone)
>
> **You'll learn:** put your app online in 2-3 steps on any platform, what an adapter does, and — if
> you want it — the advanced adapter system at the very end.

New to deploying? Start with **Quick Deploy** below — most apps ship in two or three steps. The
deeper sections explain how it works, and the **Advanced** section at the end covers the adapter
system for power users and adapter authors.

## Quick Deploy

Pick your platform. Every path assumes your `package.json` has the standard scripts
(`"build": "ruvyxa build"` — the starter templates already do).

| Platform                       | Steps                                                                                                         |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| **Vercel**                     | Push your repo → import it on Vercel → done. Ruvyxa detects Vercel and emits the right output.                |
| **Netlify**                    | Push your repo → import it on Netlify → set **Publish directory** to `.ruvyxa/deploy/netlify/publish` → done. |
| **Railway**                    | Push your repo → create a Railway service → done. Ruvyxa detects Railway and emits a standalone server.       |
| **Render**                     | Push your repo → create a Render Web Service → done. Ruvyxa detects Render and honors its `PORT`.             |
| **Firebase Hosting**           | `ruvyxa build --adapter firebase` → `firebase deploy --only hosting,functions`                                |
| **AWS Amplify Hosting**        | Push your repo → import it in Amplify → done. Ruvyxa emits `.amplify-hosting/` automatically.                 |
| **Cloudflare**                 | `ruvyxa build --adapter cloudflare` → `npx wrangler deploy -c .ruvyxa/deploy/cloudflare/wrangler.jsonc`       |
| **Your own server / Docker**   | `ruvyxa build --adapter node` → `node .ruvyxa/deploy/node/server/index.mjs`                                   |
| **Static host (GitHub Pages)** | `ruvyxa build --adapter static` → upload `.ruvyxa/static/`                                                    |

That's it for most projects. Vercel, Netlify, Railway, Render, Cloudflare Pages, and AWS Amplify
builds select their adapter automatically. Generated provider config never overwrites a file you
already own.

## How It Works (in one minute)

`ruvyxa build` compiles your app into `.ruvyxa/`. An **adapter** then repackages that output into
the exact shape a hosting platform expects — a serverless function for Netlify, a Build Output
directory for Vercel, a standalone server for a VPS. You choose an adapter one of three ways:

1. **Automatically** — building on Vercel, Netlify, Cloudflare Pages, Railway, Render, or AWS
   Amplify CI selects the right adapter from the platform's environment. Zero configuration.
2. **Command line** — `ruvyxa build --adapter node` (no config changes, uses adapter defaults).
3. **Config** — set `adapter` in `ruvyxa.config.ts` when you need adapter options.

All ten official adapters (`node`, `bun`, `static`, `vercel`, `netlify`, `cloudflare`, `railway`,
`render`, `firebase`, `aws`) ship with the `ruvyxa` package — nothing extra to install.

### Setup

Use the standard npm scripts:

```json
{
  "scripts": {
    "dev": "ruvyxa dev",
    "build": "ruvyxa build",
    "start": "ruvyxa start",
    "check": "ruvyxa check"
  }
}
```

### Zero-config platform detection

When neither `config.adapter` nor `--adapter` selects an adapter, `ruvyxa build` detects the hosting
platform from its build environment and picks the matching adapter automatically:

| Environment variable | Adapter      |
| -------------------- | ------------ |
| `VERCEL`             | `vercel`     |
| `NETLIFY`            | `netlify`    |
| `CF_PAGES`           | `cloudflare` |
| `RAILWAY_PROJECT_ID` | `railway`    |
| `RENDER`             | `render`     |
| `AWS_APP_ID`         | `aws`        |

Set `RUVYXA_ADAPTER=<name>` to override detection, or set it to a specific adapter on any other CI.
A configured adapter always wins over detection.

Firebase Hosting deploys through the Firebase CLI rather than a hosted build environment, so select
it with `--adapter firebase` (or `RUVYXA_ADAPTER=firebase`). Authentication and project selection
remain Firebase CLI responsibilities.

An adapter's post-build lifecycle runs while the build is still in the staging directory, so a
failed adapter cannot replace a previously successful `.ruvyxa/` build. Generated deploy output
lands in `.ruvyxa/deploy/<platform>/`.

Every adapter ships the same two cache policies, expressed in whatever config the host reads
(`config.json` routes on Vercel, `.netlify/v1/config.json` headers plus `netlify.toml` on Netlify,
an `_headers` file on Cloudflare and the static adapter, response headers in the standalone Node
server):

- Content-hashed `/__ruvyxa/client/*` bundles — `public, max-age=31536000, immutable`.
- Everything else from `public/` — `public, max-age=3600, must-revalidate`, the same header
  `ruvyxa dev` and `ruvyxa start` send. Without it Vercel, Netlify, and Cloudflare all default to
  `max-age=0, must-revalidate` and re-fetch every image and font on each navigation.

## Platform Guides

### Vercel

Connect the repository and deploy — nothing else to configure. During the build, the adapter emits
Vercel's Build Output API layout (`.vercel/output/static` and `.vercel/output/config.json`) at the
project root, which Vercel picks up automatically. `.vercel/` is a generated build artifact; the
starter templates already gitignore it.

To choose the adapter explicitly in config:

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'
import { vercelAdapter } from '@ruvyxa/adapter-vercel'

export default config({
  adapter: vercelAdapter(),
})
```

Pass `vercelAdapter({ projectOutput: false })` to write only under `.ruvyxa/deploy/vercel/` and
deploy that directory manually with the “Other” preset.

Static pages are served from Vercel's edge everywhere, but SSR pages, API routes, and ISR
revalidation run in the **function region** — `iad1` (US East) unless your account says otherwise.
If your users are far from it, pin the function closer:

```ts
export default config({
  adapter: vercelAdapter({ regions: ['sin1'] }), // Singapore
})
```

### Netlify

Connect the repository, then set two fields once in the Netlify dashboard:

- **Build command**: `npm run build`
- **Publish directory**: `.ruvyxa/deploy/netlify/publish`

No file is written at your project root. The build emits Netlify's Frameworks API directory
(`.netlify/v1/`, a gitignored build artifact) containing the SSR/API function and the immutable
cache headers — Netlify picks it up automatically on deploy.

To choose the adapter explicitly in config:

```ts
import { netlifyAdapter } from '@ruvyxa/adapter-netlify'

export default config({
  adapter: netlifyAdapter(),
})
```

Prefer a committed config file instead of dashboard fields? Pass
`netlifyAdapter({ projectConfig: true })` to generate a project-root `netlify.toml` (with
project-relative paths) on the next build; an existing `netlify.toml` is **never overwritten**. Pass
`frameworksApi: false` to skip the `.netlify/v1/` output.

### Cloudflare

No file is written at your project root. The deploy directory is self-sufficient — deploy it
directly:

```bash
ruvyxa build --adapter cloudflare
npx wrangler deploy -c .ruvyxa/deploy/cloudflare/wrangler.jsonc
```

To choose the adapter explicitly in config:

```ts
import { cloudflareAdapter } from '@ruvyxa/adapter-cloudflare'

export default config({
  adapter: cloudflareAdapter(),
})
```

Prefer a committed root config? Pass `cloudflareAdapter({ projectConfig: true })` to generate a
project-root `wrangler.jsonc` (with project-relative paths); an existing `wrangler.jsonc` is **never
overwritten**.

### Railway

Connect the repository as a Railway service. Railpack runs the standard `build` script, the
`RAILWAY_PROJECT_ID` environment variable selects `railway`, and the generated standalone server
uses Railway's `PORT` automatically.

```ts
import { railwayAdapter } from '@ruvyxa/adapter-railway'

export default config({
  adapter: railwayAdapter(),
})
```

The adapter emits `.ruvyxa/deploy/railway/server/index.mjs` and a safe `railway.json` with the build
and start commands. An existing project `railway.json` is never overwritten. Pass
`railwayAdapter({ projectConfig: false })` when dashboard settings are your source of truth.

### Render

Create a Render Web Service from the repository. The standard `build` script runs on Render,
`RENDER=true` selects the adapter, and the standalone server binds to `0.0.0.0:$PORT` (Render's
default is currently port 10000).

```ts
import { renderAdapter } from '@ruvyxa/adapter-render'

export default config({
  adapter: renderAdapter({ serviceName: 'my-web-app' }),
})
```

The adapter emits `.ruvyxa/deploy/render/server/index.mjs` plus a `render.yaml` Blueprint. Existing
Blueprints are preserved. Pass `projectConfig: false` to keep configuration in the Render dashboard.

### Firebase Hosting

Firebase serves SSG/CSR pages and assets from its CDN, then rewrites SSR, ISR, PPR, and API requests
to a generated second-generation HTTPS function:

```bash
ruvyxa build --adapter firebase
firebase deploy --only hosting,functions
```

The first build creates `firebase.json` if one does not exist. Select or pass a Firebase project
with the Firebase CLI before deploying; Ruvyxa never writes credentials or a project ID. Dynamic
deployments require a Firebase project with billing enabled because they use Cloud Functions.

```ts
import { firebaseAdapter } from '@ruvyxa/adapter-firebase'

export default config({
  adapter: firebaseAdapter({ region: 'asia-east1' }),
})
```

The generated Hosting rewrite uses `pinTag` so Hosting and the function deploy together. Runtime ISR
cache is ephemeral per warm function instance; use durable application storage when cross-instance
cache consistency matters.

### AWS Amplify Hosting

Import the repository in AWS Amplify Hosting. `AWS_APP_ID` selects the adapter and `ruvyxa build`
emits Amplify's native deployment specification:

```text
.amplify-hosting/
├── static/
├── compute/default/
│   └── server.js
└── deploy-manifest.json
```

No `amplify.yml` is required when Amplify uses its standard `.amplify-hosting` fallback. The compute
server listens on port 3000 and stores runtime ISR refreshes in `/tmp`, the writable Amplify compute
location.

```ts
import { awsAdapter } from '@ruvyxa/adapter-aws'

export default config({
  adapter: awsAdapter({ runtime: 'nodejs22.x' }),
})
```

`aws` means AWS Amplify Hosting support. It does not provision arbitrary ECS, Lambda, API Gateway,
RDS, IAM, or VPC resources.

### Self-Hosted (Node.js, Docker, VPS, PaaS)

```bash
npm run build
npm run start          # serve from .ruvyxa/ using the ruvyxa CLI
```

Or build a standalone server that runs without the ruvyxa CLI at runtime:

```bash
ruvyxa build --adapter node
node .ruvyxa/deploy/node/server/index.mjs
```

On Bun, build with `--adapter bun` and run `bun .ruvyxa/deploy/bun/server/index.mjs`. Both adapters
emit the same server, so ordering, static fallbacks, and cache headers are identical on either
runtime.

The `deploy/node/` directory is self-contained (server + `public/` assets). Copy it into a Docker
image, a VPS, PM2, systemd, or any PaaS (Render, Railway, Fly.io, Heroku) and run the same command —
no `node_modules` and no native binary needed at runtime. The server honors `PORT` (default 3000)
and `HOST` (default 0.0.0.0), and supports SSR, API, ISR, PPR, SSG, and CSR.

### Static Hosting

```bash
ruvyxa build --adapter static
# upload .ruvyxa/static/ to your static host
```

Static hosting works for apps whose pages are all SSG/CSR. Pages that need a server (SSR, ISR, PPR,
API routes) are rejected at build time with a clear per-route error — pick a serverless or Node
target for those.

### What Each Platform Supports

| Strategy | Vercel | Netlify | Cloudflare | Railway/Render | Firebase | AWS Amplify | Static |
| -------- | ------ | ------- | ---------- | -------------- | -------- | ----------- | ------ |
| SSG      | Yes    | Yes     | Yes        | Yes            | Yes      | Yes         | Yes    |
| CSR      | Yes    | Yes     | Yes        | Yes            | Yes      | Yes         | Yes    |
| SSR      | Yes    | Yes     | Yes        | Yes            | Yes      | Yes         | No     |
| API      | Yes    | Yes     | Yes        | Yes            | Yes      | Yes         | No     |
| ISR      | Yes    | Yes     | No*        | Yes            | Yes†     | Yes†        | No     |
| PPR      | Yes    | Yes     | No*        | Yes            | Yes†     | Yes†        | No     |

\* Cloudflare Workers lack persistent server-side storage for ISR cache. ISR and PPR routes are
rejected with `RUV2210` on Cloudflare. Use KV or Durable Objects bindings manually if needed.

† Firebase Functions and Amplify compute use instance-local ephemeral caches. Revalidation works,
but cache entries are not shared across cold starts or scaled instances.

Static-only deployments (SSG/CSR pages without API or SSR routes) work everywhere. The serverless
adapters emit both static assets and a serverless function; platforms serve static files directly
and forward unmatched requests to the function handler.

### How Requests Are Routed on a Host

Every serverless adapter follows the same order, which mirrors what `ruvyxa dev` and `ruvyxa start`
do locally:

1. **Hashed client bundles** under `/__ruvyxa/client/` — served from the CDN, cached immutably.
2. **Public assets** (everything from `public/`) — served from the CDN with
   `public, max-age=3600, must-revalidate`.
3. **Pre-rendered SSG/CSR pages** — served from the CDN.
4. **Everything else** — the function handler.

Two consequences worth knowing:

- A request for a missing asset (`/logo.png`, `/favicon.ico`) returns **404**, never a rendered
  page. Without that rule a bare dynamic route such as `/[lang]` captures the filename and answers
  `200` with an HTML body, which browsers show as a broken image while billing a function invocation
  per request. Routes that declare the extension themselves (`/sitemap.xml`) still match.
- **ISR and PPR pages are deliberately not published as static files.** The host would serve the
  build-time snapshot before the function is ever reached, so the page could never revalidate. The
  deploy-time HTML still ships inside the function bundle and is used as the first cache entry.

## Troubleshooting

### CSS / images / JS 404 on Netlify (unstyled page)

```
Failed to load resource: the server responded with a status of 404 ()
```

**Symptom.** The page renders as plain text — no styles, broken images — and the browser console
shows 404s for `.css`, `.png`, and `/__ruvyxa/client/*.js`. The HTML itself loads fine.

**Cause.** The **publish directory is not set**. Ruvyxa's static asset layer (CSS, images, hashed
client bundles) is written to `.ruvyxa/deploy/netlify/publish`. The SSR/API function is discovered
automatically through the Frameworks API (`.netlify/v1/`), but **the Frameworks API cannot declare a
publish directory** — Netlify resolves that from `netlify.toml` (or the dashboard) _before_ the
build runs. With no publish directory pointing at the output, Netlify serves nothing static, every
asset request falls through to the function, and the function 404s it (it serves routes, not files).
That is why the HTML renders — the function works — but everything static is missing.

**Fix — pick one:**

- **Dashboard** (no committed file): Site configuration → Build & deploy → set **Publish directory**
  to `.ruvyxa/deploy/netlify/publish`, then redeploy.
- **Committed config**: set `netlifyAdapter({ projectConfig: true })` in `ruvyxa.config.ts`, run
  `npx --no-install ruvyxa build` to generate a project-root `netlify.toml`, then commit it and
  push. An existing `netlify.toml` is never overwritten — add
  `publish = ".ruvyxa/deploy/netlify/publish"` to it by hand in that case.

**Confirm the fix** by requesting an asset directly — it must return `200`, not `404`:

```bash
curl -I https://YOUR-SITE.netlify.app/__ruvyxa/client/
```

> **Not a Ruvyxa error:** a console line like
> `A listener indicated an asynchronous response by returning true, but the message channel closed`
> comes from a browser extension, not your site. It is unrelated to the 404 and can be ignored.

Vercel and Cloudflare do not hit this: their deploy directory is self-sufficient (`.vercel/output`,
`.ruvyxa/deploy/cloudflare/`) and carries the static layer with it.

### Realtime Fails to Build on a Serverless/Static Adapter

```
RUV3201 native WebSocket realtime requires a self-hosted Node/Bun build; received target=node adapter=netlify
```

Ruvyxa's native WebSocket transport needs one long-lived process holding the connections. It is
available on Node, Bun, Railway, and Render, but **not** on serverless (Vercel, Netlify, Cloudflare,
Firebase, AWS Amplify) or static adapters — the guard fails the build on purpose rather than
deploying a socket that can never connect. Options:

- Deploy the realtime app with **Node**, **Bun**, **Railway**, or **Render** on a host that keeps
  the process alive.
- Or drop the native realtime plugin from that build if the route set does not need live
  connections.

The demo app enables native realtime, so it cannot be built for a serverless adapter — this is by
design, not a bug.

### Unsupported Routes on the Static Adapter

```
RUV2202 adapter static supports ssg, csr; unsupported routes: /api/x (api), /dashboard (ssr)
```

The static adapter publishes files only — it has no server, so it cannot host SSR pages, API routes,
ISR, or PPR. Either convert those routes to `ssg`/`csr`, or switch to an adapter that ships a
function or server (node, bun, vercel, netlify, cloudflare, railway, render, firebase, aws).

### Permission Denied Error

```
node_modules/.bin/ruvyxa: Permission denied
```

This means the installed Ruvyxa launcher was published without executable permission. Upgrade to a
Ruvyxa release that includes the executable launcher.

### GLIBC Version Error

```
ruvyxa: /lib64/libc.so.6: version `GLIBC_2.39' not found
```

Ruvyxa releases before 1.0.19 shipped dynamically linked Linux binaries that required the build
machine's glibc, which broke on hosts with an older glibc (for example Vercel's Amazon Linux build
image). Since 1.0.19 the Linux CLI binaries are fully static musl builds and run on any Linux —
upgrade the `ruvyxa` package to fix this error.

### Node Version

Pin Node 22 for reproducible CI builds:

```json
{
  "engines": {
    "node": "22.x"
  }
}
```

---

## CI/CD

### Recommended Pipeline

```yaml
# .github/workflows/deploy.yml
- run: npm ci
- run: npx ruvyxa analyze
- run: npm run typecheck
- run: npm run check
- run: npm run build
```

### Build Artifacts

After `npm run build`, the normal runtime output remains in `.ruvyxa/` and an adapter may add a
deployment directory:

```text
.ruvyxa/
├── server/         # Server-side source
├── client/         # Client bundles with manifest
├── assets/         # Static assets + WebP images
├── prerender/      # Pre-rendered HTML pages
├── manifest.json   # Route manifest
├── build.json      # Build metadata
└── deploy/         # Adapter-specific artifacts, when configured
```

For a static adapter, use its generated publish directory instead of deploying all of `.ruvyxa/`.

---

## Production Checklist

Before deploying:

- [ ] `npx ruvyxa analyze` — no errors
- [ ] `npm run typecheck` — type-safe
- [ ] `npm run check` — readiness checks pass
- [ ] `.env.example` — lists required variable names without real values
- [ ] Security headers — `security.headers: true`
- [ ] CORS origins — explicit, not wildcard
- [ ] Body limits — `security.apiLimit` and `security.actionLimit` are appropriate
- [ ] Reverse proxy — forward `X-Forwarded-Proto` and add its exact non-loopback IP to
      `security.trustedProxyIps` when behind an HTTPS proxy

## Learn from the Demo

`examples/demo/` is an integration app containing static, dynamic, and catch-all routes; API routes;
server actions; MDX; public environment variables; external CSS; and SSR, SSG, ISR, CSR, and PPR
examples. Read its [README](../../../examples/demo/README.md), run the diagnostic commands, and copy
a proven pattern before adding a new feature to your own app.

---

## Advanced: The Adapter System

Everything below is for power users and adapter authors — deploying an app never requires it.

<details>
<summary><strong>Expand the adapter system reference</strong> (resolution rules, writing your own adapter, lifecycle)</summary>

### Available Adapters

| Adapter                      | Target                                                    |
| ---------------------------- | --------------------------------------------------------- |
| `@ruvyxa/adapter-node`       | Standalone server: `.ruvyxa/deploy/node/server/index.mjs` |
| `@ruvyxa/adapter-bun`        | Standalone server: `.ruvyxa/deploy/bun/server/index.mjs`  |
| `@ruvyxa/adapter-static`     | Static files: `.ruvyxa/static/`                           |
| `@ruvyxa/adapter-cloudflare` | Cloudflare Workers: `.ruvyxa/deploy/cloudflare/`          |
| `@ruvyxa/adapter-netlify`    | Netlify functions + static: `.netlify/v1/` + deploy dir   |
| `@ruvyxa/adapter-vercel`     | Vercel Build Output API: `.vercel/output/`                |
| `@ruvyxa/adapter-railway`    | Railway standalone server + `railway.json`                |
| `@ruvyxa/adapter-render`     | Render standalone server + `render.yaml`                  |
| `@ruvyxa/adapter-firebase`   | Firebase Hosting + Cloud Functions v2                     |
| `@ruvyxa/adapter-aws`        | AWS Amplify Hosting static + compute primitives           |

All official adapters are bundled with the `ruvyxa` package — `--adapter <name>` and platform
auto-detection work without installing anything. Install the individual `@ruvyxa/adapter-*` package
only when you need to pass adapter options in `ruvyxa.config.ts`.

### `--adapter` Resolution

`--adapter` accepts two kinds of value, and overrides `config.adapter` for that build only:

**1. Built-in names** — `node`, `bun`, `static`, `vercel`, `netlify`, `cloudflare`, `railway`,
`render`, `firebase`, `aws`. These work with `ruvyxa` alone installed and always use the adapter's
defaults.

**2. Any adapter package name** — opens the ecosystem to platforms without an official adapter (Deno
Deploy, Fastly, and so on):

```bash
ruvyxa build --adapter @acme/ruvyxa-adapter-deno   # scoped names are used verbatim
ruvyxa build --adapter fastly                       # short names try the conventions
```

Resolution order:

1. A scoped name (`@scope/name`) or one containing `/` resolves as that exact package.
2. A short name tries `@ruvyxa/adapter-<name>`, then `ruvyxa-adapter-<name>`, then `<name>`.
3. Each candidate resolves from your project's `node_modules` first, then falls back to the copies
   bundled with `ruvyxa` — **a project-installed version always wins**, so you can pin an adapter
   version per project.

When no candidate resolves, the build fails with `RUV2203` listing every package name that was
tried, so the missing install is obvious.

### Writing an Adapter

An adapter package has a single contract: its default export must be a factory function returning an
object matching the `Adapter` interface from `@ruvyxa/core` (`name`, `target`, `supports?`,
`build(ctx)`) — exactly what every official adapter does:

```ts
// ruvyxa-adapter-fastly/src/index.ts
import type { Adapter, BuildContext } from '@ruvyxa/core'

export default function fastlyAdapter(): Adapter {
  return {
    name: 'fastly',
    target: 'edge',
    supports: ['ssr', 'ssg', 'csr', 'api'],
    build(ctx: BuildContext) {
      return {
        name: 'fastly',
        target: 'edge',
        entry: `${ctx.outDir}/server/app`,
        assetsDir: `${ctx.outDir}/assets`,
        artifacts: [/* ... */],
      }
    },
  }
}
```

The framework does the heavy lifting: it compiles every route into an executable `.mjs` registry,
copies the shared serverless handler runtime (`serverless-handler.mjs` — SSR, API dispatch, ISR
revalidation, PPR), and materializes the artifacts an adapter declares (`file`, `static-site`,
`function`). The adapter only describes the platform's expected layout and wraps the handler in the
platform's function signature.

### Adapter Lifecycle Notes

- An adapter's `build()` function runs both during configuration loading and during the post-build
  artifact step.
- The post-build step may create only files inside `.ruvyxa/` (plus an allowlist of platform
  discovery paths at the project root, such as `.vercel/output` and `.netlify/v1`); its result is
  recorded as `adapterArtifacts` in `.ruvyxa/build.json`.
- Static adapters deliberately reject dynamic request handling until a platform request handler
  exists. This is a safety boundary, not a fallback.
- Function output contains a compiled `.mjs` static route registry bundle, not raw TypeScript/TSX.
  This makes the emitted artifact executable as-is and lets Wrangler discover edge modules during
  bundling. On Vercel and Netlify, ISR cache age is checked against `revalidate`; only stale entries
  regenerate, and concurrent stale hits are coalesced within a warm function instance.

</details>
