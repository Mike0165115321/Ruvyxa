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
