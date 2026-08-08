# Plugins and middleware

> **Tutorial goal:** add cross-cutting behavior once, then apply it to the routes that need it.
> **Start from:** a configured application in [Configuration](07-configuration.md). **Checkpoint:**
> verify one matching and one non-matching route after enabling a plugin or middleware rule.

Plugins are values returned by `definePlugin()` from `ruvyxa/plugin` (also re-exported by `ruvyxa`).
Add them to `plugins` in `ruvyxa.config.ts`. A plugin needs a non-empty name and either declarative
behavior or `register(api)`; invalid definitions fail with `RUV2102`.

## Declarative plugin

```ts
// plugins/request-id.ts
import { definePlugin } from 'ruvyxa/plugin'

export const requestId = definePlugin({
  name: 'example:request-id',
  http: {
    match: ['/api/*'],
    onResponse({ response }) {
      const headers = new Headers(response.headers)
      headers.set('x-example', 'enabled')
      return new Response(response.body, { status: response.status, headers })
    },
  },
})
```

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'
import { requestId } from './plugins/request-id'
export default config({ plugins: [requestId] })
```

`http.match` uses exact paths or trailing-`*` prefixes. A request hook may return `Request`,
`Response`, or nothing; a response hook may return `Response` or nothing. `http.routes` declares
exact plugin-owned routes and accepts one method, multiple methods, or every method if omitted. The
advanced `register` API exposes `http`, `build`, `dev`, `diagnostics`, `native`, and `head` sockets.

## Build and dev lifecycle

Build hooks are `onStart`, `onResolve`, `onLoad`, `onTransform`, and `onComplete`.
Resolve/load/transform hooks receive an environment of `client`, `server`, `edge`, `worker`, or
`shared`; transformations return code, `{ code, map }`, null, or nothing. Dev exposes an
`onFileChange` registration. Plugins can report diagnostics and contribute document-head entries. Do
not rely on module-level middleware state across workers: config explicitly states workers do not
share it.

## First-party plugins

`ruvyxa/plugins` implements: `redirects`, `headers`, `observability`, `securityHeaders`,
`cacheRules`, `sitemap`, `robots`, `alias`, and additional file-backed helpers in that public entry
point. Use its validation rather than reconstructing the behavior. For example, redirects permit
`*`, exact paths, or trailing-prefix patterns and only accept absolute HTTP(S) URLs or safe absolute
paths as destinations.

```ts
import { redirects, securityHeaders } from 'ruvyxa/plugins'
export default config({
  plugins: [
    redirects([{ source: '/old/*', destination: '/new/*', permanent: true }]),
    securityHeaders({ contentSecurityPolicy: { 'default-src': ["'self'"] } }),
  ],
})
```

`permanent: true` makes `redirects` send 308; otherwise it sends 307. `securityHeaders` supplies
HSTS by default but cannot choose a safe CSP for your application—set one deliberately and test
third-party resources.

## First-party plugin catalog

| Plugin                                | Output or runtime behavior                                                                          |
| ------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `redirects`, `headers`, `cacheRules`  | Route-scoped redirects, response headers, and browser/CDN cache directives.                         |
| `observability`, `securityHeaders`    | Request ID/timing/structured logs and response security policy.                                     |
| `pwa`                                 | Manifest, service worker, registration script, optional precache/offline fallback, and HTML wiring. |
| `sitemap`, `robots`, `feed`           | Build-time `sitemap.xml`, `robots.txt`, and RSS output from explicit metadata.                      |
| `searchIndex`, `contentEngine`        | Build-time search index and content-derived answer/search artifacts.                                |
| `openApi`                             | OpenAPI 3.1 JSON served in development and written into production output.                          |
| `alias`, `bundleBudget`, `requireEnv` | Build-time import aliasing, client JavaScript size limits, and required environment validation.     |
| `fonts`                               | Build-time self-hosting for supplied Google Fonts stylesheet URLs.                                  |

Use explicit data with build-time plugins: they do not discover your business content or API
semantics automatically. For example, this is a complete PWA declaration with the required `name`:

```ts
import { pwa, openApi } from 'ruvyxa/plugins'

export default config({
  plugins: [
    pwa({
      name: 'Example app',
      icons: [{ src: '/icon-192.png', sizes: '192x192', type: 'image/png' }],
    }),
    openApi({
      info: { title: 'Example API', version: '1.0.0' },
      operations: [
        { method: 'GET', path: '/api/health', responses: { '200': { description: 'Healthy' } } },
      ],
    }),
  ],
})
```

The PWA plugin defaults to `/manifest.webmanifest`, `/sw.js`, and `/pwa-register.js`; all three
paths must differ. `openApi` defaults to `/openapi.json`, requires a non-empty title/version, and
rejects duplicate method/path and `operationId` entries. Run a production build and inspect the
generated output whenever adding a build plugin.

**Previous:** [Configuration and environment](07-configuration.md) · **Next:**
[Integrations](09-integrations-auth-data-and-realtime.md)
