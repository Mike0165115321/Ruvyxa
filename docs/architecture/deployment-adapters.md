# Deployment Adapters

Deployment adapters convert the framework build output into artifacts for a target runtime. The
contract and selection flow below are the implementation contract in this repository; platform
deployment commands remain the responsibility of the platform or the project CI pipeline.

## Adapter contract

The shared contract is exported by `@ruvyxa/core`:

```ts
interface Adapter {
  name: string
  target: string
  supports?: readonly string[]
  build(ctx: BuildContext): AdapterOutput | Promise<AdapterOutput>
}
```

`build()` returns an `AdapterOutput` containing the target, runtime metadata, and artifact
descriptors. An artifact can be a function, static site, or generated file. Adapters receive the
validated build context; they do not receive an invented `adapt(options)` contract.

## Selection and build flow

1. `ruvyxa doctor --adapter <name>` can inspect adapter compatibility.
2. `ruvyxa build --adapter <name>` selects an adapter explicitly and invokes its `build()` hook.
3. The bundled `runtime/adapter-runner.mjs` resolves built-in short names (or a valid npm package
   name), validates the route strategies against `supports`, and materializes the returned
   artifacts.
4. `ruvyxa start` and `ruvyxa preview` serve an existing production build; they do not perform the
   adapter build step.

The CLI also accepts an adapter configured by the project configuration. Automatic platform
selection is a fallback in the runtime runner, not dependency scanning of `package.json`.

## Built-in adapter names

The repository currently ships these ten first-party adapter packages:

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

The authoritative capability list is each adapter's `supports` field. It is a declaration used by
the runner; successful local compilation does not prove that a provider-side deployment will meet
all runtime requirements.

## Representative artifact layouts

The exact output is adapter-specific. Two source-backed examples are:

### Node

`@ruvyxa/adapter-node` emits under `.ruvyxa/deploy/node/`:

```text
server/index.mjs
public/                 # optional prerendered site
start.mjs
README.md
```

The generated standalone server uses `node:http` and honors `PORT` and `HOST`. It can be copied to a
Node-compatible host, Docker image, PM2, systemd, or a PaaS; the adapter itself does not deploy the
artifact.

### Static

`@ruvyxa/adapter-static` emits a static-site artifact in `static/` by default, or in the validated
relative `outputDir` supplied to `staticAdapter`. It supports only `ssg` and `csr`, because a pure
static host cannot execute server routes.

### AWS Amplify Hosting

`@ruvyxa/adapter-aws` can emit `.amplify-hosting/` project artifacts, including a static directory,
the `compute/default` handler, and `deploy-manifest.json`. Its static artifact explicitly excludes
`isr` and `ppr`; the adapter's declared capabilities and the provider's runtime behavior must be
verified together.

## Static parameters

Static generation uses the framework's `getStaticParams`/`staticParams` route metadata. The
framework does not expose a `getStaticPaths()` API in the current source tree.

## Docker and custom deployment

There is no native Docker adapter in the built-in list. Use the Node or Bun adapter, copy its
generated deployment directory into an image, and run the generated server according to that
adapter's README. A custom adapter must implement the `Adapter` contract and return valid
`AdapterOutput` artifacts; the runner does not provide a deployment control plane, blue-green swap,
or production rollback service.

## Source of truth

- `packages/@ruvyxa/core/src/types.ts` — adapter and artifact types
- `packages/ruvyxa/runtime/adapter-runner.mjs` — name resolution and runner flow
- `packages/@ruvyxa/adapter-*/src/index.ts` — adapter capabilities and artifacts
- `crates/ruvyxa_cli/src/main.rs` — CLI commands and adapter option

---

## Production contract and retained detail

The section above is the current, source-backed contract for this release. The original long-form
draft is retained below to preserve instructional context and audit history. It is non-normative: do
not copy its API snippets or capability claims unless they are revalidated against the current
source and package export map. This boundary is intentional so the document can retain its original
depth without presenting unsupported historical design as production behavior.

### Deployment adapter draft — historical draft (non-normative)

> **Archive warning:** The material below is retained for history only. It is not the current
> adapter contract; examples may be stale or unsupported and must not be copied as working commands.
> The source-backed contract above is authoritative.

# Deployment Adapters · อาดาปเตอร์สำหรับการปรับใช้

**Scope**: Platform adapter packages (`@ruvyxa/adapter-*`)

## สรุป

Adapters transform Ruvyxa build output into platform-specific formats. Each adapter implements a
common interface: receive compiled bundles + route manifest → produce deployable artifact.

---

## Adapter Interface

```typescript
// packages/ruvyxa/src/adapters/types.ts

export interface Adapter {
  name: string

  /**
   * Called during ruvyxa build. Transform build output to platform format.
   */
  adapt(options: AdaptOptions): Promise<AdaptResult>
}

export interface AdaptOptions {
  /** Route manifest */
  manifest: RouteManifest
  /** Client bundle output directory */
  clientOutDir: string
  /** Server bundle output directory */
  serverOutDir: string
  /** Project root */
  root: string
  /** Build configuration */
  config: BuildConfig
}

export interface AdaptResult {
  /** Directory containing deploy-ready output */
  outputDir: string
  /** Platform-specific configuration files written */
  generated: string[]
}
```

The CLI discovers the adapter from `package.json` dependencies:

```javascript
const adapterPackage = Object.keys(pkg.dependencies || {}).find((dep) =>
  dep.startsWith('@ruvyxa/adapter-'),
)
```

---

## Built-in Adapters

| Package                      | Platform            | Output                               |
| ---------------------------- | ------------------- | ------------------------------------ |
| `@ruvyxa/adapter-node`       | Node.js HTTP server | `server.js` + `client/` directory    |
| `@ruvyxa/adapter-static`     | Static hosting      | `out/` directory (SSG HTML + assets) |
| `@ruvyxa/adapter-vercel`     | Vercel Functions    | `.vercel/output/` config             |
| `@ruvyxa/adapter-netlify`    | Netlify Functions   | `netlify.toml` + functions           |
| `@ruvyxa/adapter-cloudflare` | Cloudflare Workers  | `wrangler.toml` + worker script      |
| `@ruvyxa/adapter-docker`     | Docker container    | `Dockerfile` + `nginx.conf`          |

---

## Adapter: Node

### Output Structure

```
.ruvyxa/output/
  ├── server.js              # CJS bundle with all server modules
  ├── client/                # Static assets
  │   ├── index.js
  │   ├── about.js
  │   └── ...
  ├── package.json           # Start script
  └── manifest.json          # Route manifest
```

### Runtime

`server.js` exports a `createServer()` function:

```javascript
module.exports.createServer = function (options) {
  // Start Axum/Axum-compatible HTTP server
  // Load route manifest
  // Serve client/ as static files
  // Handle SSR via embedded server modules
}
```

---

## Adapter: Static

### Output Structure

```
.ruvyxa/static/
  ├── index.html
  ├── about/index.html
  ├── blog/[slug]/index.html   # One per static param
  ├── _next/static/            # Client bundles
  └── manifest.json
```

### Build Process

1. Run `ruvyxa build` (full SSG detection)
2. For each SSG route: call `getStaticPaths()`, render each path to HTML
3. Write HTML to `out/` with directory structure matching the URL
4. Copy client bundles to `out/_next/static/`
5. Generate `404.html` (customizable)

---

## Adapter: Vercel

### Output Structure

```
.ruvyxa/vercel/
  └── .vercel/output/
      ├── config.json          # Vercel output config
      └── static/
          ├── index.html
          └── ...
      └── functions/
          ├── __ruvyxa.func/
          │   ├── .vc-config.json
          │   └── server.js
          └── blog/[slug].func/
              ├── .vc-config.json
              └── server.js
```

### `.vc-config.json`

```json
{
  "runtime": "nodejs18.x",
  "handler": "server.js",
  "launcherType": "Nodejs",
  "shouldAddHelpers": true
}
```

---

## Adapter: Cloudflare

### Output Structure

```
.ruvyxa/cloudflare/
  ├── wrangler.toml
  ├── worker.js              # Compiled worker (ESM)
  └── assets/                 # Static assets
```

### `wrangler.toml`

```toml
name = "ruvyxa-app"
main = "worker.js"
compatibility_date = "2024-01-01"

[site]
bucket = "assets"
```

`worker.js` is an ES module that exports `fetch`:

```javascript
export default {
  async fetch(request, env, ctx) {
    // Match route, render, return Response
  },
}
```

---

## Adapter: Docker

### Output Structure

```
.ruvyxa/docker/
  ├── Dockerfile
  ├── nginx.conf
  ├── server/               # Server bundle
  │   └── server.js
  └── public/               # Client bundles
      └── index.js
```

### Dockerfile

```dockerfile
FROM node:18-alpine
WORKDIR /app
COPY server/ server/
COPY public/ public/
EXPOSE 3000
CMD ["node", "server/server.js"]
```

### nginx.conf (optional)

```nginx
server {
    listen 80;
    location / {
        proxy_pass http://127.0.0.1:3000;
    }
    location /_next/static {
        # Immutable static assets
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}
```

---

## Adapter Selection

```rust
pub fn detect_adapter(root: &Path) -> Result<String>
pub fn run_adapter(adapter: &str, options: &AdaptOptions) -> Result<String>
```

`detect_adapter` reads `package.json` dependencies. Falls back to `@ruvyxa/adapter-node` if no
adapter is found. `run_adapter` shells out to the adapter's Node.js entry point.

---

## Why This Design

1. **Adapter as npm package, not built-in** — Users install only the adapter they need. No dead code
   from unused platforms. Adding a new platform is `npm install @ruvyxa/adapter-xxx` — no framework
   update.
2. **Common interface with platform-specific output** — Every adapter receives the same
   `AdaptOptions`. The transform logic is isolated per adapter. Changes to one platform never affect
   another.
3. **`detect_adapter` at build time** — No `ruvyxa.config.ts` adapter field needed. The presence of
   `@ruvyxa/adapter-vercel` in `package.json` is sufficient. Convention over configuration.
4. **Node adapter is the default** — Every Node.js deployment works with zero configuration.
   `ruvyxa build` with no adapter package installed still produces a runnable `server.js`.

# Runtime policy handoff

`BuildContext.buildInfo` is an optional read-only copy of `build.json`. First-party dynamic adapters
embed only `buildInfo.runtime` plus the route manifest's i18n policy. Older/custom adapters remain
source compatible because the field is additive. The Vercel adapter selects Build Output API edge
metadata with `edge: true`; its generated function and route registry are compiled for the browser
platform and contain no Node.js imports.
