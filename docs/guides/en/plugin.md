# Ruvyxa plugin authoring guide

A Ruvyxa plugin is a TypeScript/JavaScript module that exports one `definePlugin(...)` value. It
runs as trusted server/build code and extends an application through typed HTTP, build, development,
diagnostics, and framework-native capability hooks.

## 1. Choose the right authoring style

Start with concise declarations. Each concise declaration creates one registration of that kind. Use
`register(api)` only when a plugin needs repeated hooks of the same kind, precise ordering, or
conditional/loop-based registration.

| Need                                               | Concise declaration | Advanced socket           |
| -------------------------------------------------- | ------------------- | ------------------------- |
| Add response headers                               | `headers`           | `http.onResponse(...)`    |
| Add elements to every document's `<head>`          | `head`              | `head`                    |
| One request/response hook or a small route list    | `http`              | `http`                    |
| One lifecycle/resolve/load/transform/complete hook | `build`             | `build`                   |
| One file-change handler                            | `dev.onFileChange`  | `dev.onFileChange(...)`   |
| One or more static startup messages                | `diagnostics`       | `diagnostics.report(...)` |
| Realtime with fixed options                        | `native.realtime`   | `native.claim(...)`       |
| Several registrations of the same socket           | —                   | `register(api)`           |

When both styles are present, Ruvyxa registers concise declarations first in this order: HTTP,
build, dev, diagnostics, native; it calls `register(api)` last.

Before writing one, check `ruvyxa/plugins` — it ships `redirects`, `headers`, `securityHeaders`,
`cacheRules`, `sitemap`, `robots`, `feed`, `searchIndex`, `contentEngine`, `openApi`, `pwa`,
`observability`, `alias`, `bundleBudget`, `requireEnv`, and `fonts`.

`fonts` is the one to reach for on a Lighthouse performance score: a `<link>` to
`fonts.googleapis.com` blocks first paint on a third-party origin, and the plugin downloads the
stylesheet and its `.woff2` files at build time, rewrites the URLs to local paths, and declares the
self-hosted stylesheet in `<head>`.

```ts
import { fonts } from 'ruvyxa/plugins'

export default config({
  plugins: [
    fonts({
      google: ['https://fonts.googleapis.com/css2?family=Inter:wght@400;700&display=swap'],
    }),
  ],
})
```

Remove the original `<link rel="stylesheet" href="https://fonts.googleapis.com/...">` from your
layout when you adopt it; leaving it in keeps the blocking request the plugin exists to remove.

## 2. Create the package

```bash
npx ruvyxa plugin create request-logger
cd request-logger
npm install
npm test
```

For a monorepo location, supply `--dir`:

```bash
npx ruvyxa plugin create request-logger --dir packages/request-logger
```

The scaffold contains the source entry (`src/index.ts`), a contract test, `package.json`,
`tsconfig.json`, `README.md`, and `.gitignore`.

## 3. Write and register the smallest plugin

`src/index.ts`:

```ts
import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: 'request-logger',
  headers: { 'x-request-logger': 'active' },
})
```

Install a local plugin into the application:

```bash
cd ../my-app
pnpm add ../packages/request-logger
```

Then register its default export in `ruvyxa.config.ts`:

```ts
import { config } from 'ruvyxa/config'
import requestLogger from 'ruvyxa-plugin-request-logger'

export default config({ plugins: [requestLogger] })
```

Plugin names must be non-empty and unique. The `plugins` array is the registration order. Start the
application and verify the header through the real host:

```bash
npx ruvyxa dev
curl -I http://localhost:3000/
```

## 4. Concise API reference

The following is a valid single plugin using every concise section:

```ts
import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: 'site-tools',
  headers: { 'x-site-tools': 'enabled' },
  http: {
    match: ['/admin/*'],
    onRequest({ request }) {
      if (!request.headers.has('authorization'))
        return new Response('Unauthorized', { status: 401 })
    },
    onResponse({ response }) {
      return response
    },
    routes: [
      {
        method: 'GET',
        path: '/plugin/status',
        handler({ plugin }) {
          return Response.json({ plugin, ready: true })
        },
      },
    ],
  },
  build: {
    onStart({ root, outDir }) {
      console.log('building', root, outDir)
    },
    onComplete({ manifest }) {
      console.log('finished', manifest)
    },
  },
  dev: {
    onFileChange({ paths }) {
      console.log('changed', paths)
    },
  },
  head: [
    { tag: 'link', attrs: { rel: 'preconnect', href: 'https://cdn.example' } },
    { tag: 'script', attrs: { defer: true }, children: 'window.siteTools = 1' },
  ],
  diagnostics: [{ level: 'info', code: 'SITE001', message: 'Site tools enabled' }],
  native: { realtime: true },
})
```

### `head`

`head` declares elements the server writes into every rendered document's `<head>`. Declaring them
once — rather than rewriting response bodies per request — is what makes analytics, preconnect, and
verification-tag plugins cheap:

```ts
export default definePlugin({
  name: 'analytics',
  head: { tag: 'script', attrs: { src: 'https://cdn.example/a.js', defer: true } },
})
```

Only `link`, `meta`, `noscript`, `script`, and `style` are accepted — anything else in `<head>` ends
the head early and the browser moves the rest of the document into `<body>`. Attribute values are
HTML-escaped; `children` is written verbatim (a script or stylesheet cannot be escaped) and is
allowed only on the raw-text elements. Entries appear in plugin configuration order.

`head` cannot vary per route: a plugin does not know which route is rendering. Export
[`meta`](routing.md#page-metadata) from the route for that.

### `headers`

`headers` accepts any `HeadersInit` value. It adds or replaces those fields on the response. It is
implemented as a response hook; if `http.match` is present, it uses that same match scope.

```ts
headers: new Headers([['x-plugin', 'active']]),
http: { match: ['/api/*'], onResponse() {} },
```

The `http` section must declare `onRequest`, `onResponse`, or `routes`; an empty `http: {}` is
invalid.

### `http`: request hooks

`onRequest` receives `{ plugin, root, request, next }`. Return nothing to retain the request, return
a replacement `Request`, or return a `Response` to end request processing.

```ts
http: {
  match: ['/admin/*'],
  onRequest({ request }) {
    if (request.headers.get('authorization') !== `Bearer ${process.env.ADMIN_TOKEN}`) {
      return new Response('Unauthorized', { status: 401 })
    }
  },
},
```

Use `next()` or `next(replacementRequest)` only when explicit continuation is needed. Match patterns
are exact paths, `*`, or a prefix ending in `*` (for example `/api/*`); matching uses the decoded
pathname without its query string. Omit `match` to apply the hook to every application path.

### `http`: response hooks and routes

`onResponse` receives `{ plugin, root, request, response, next }`. Return nothing to preserve the
response, or return a replacement `Response`.

```ts
http: {
  onResponse({ response }) {
    const headers = new Headers(response.headers)
    headers.set('x-request-checked', 'yes')
    return new Response(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers,
    })
  },
  routes: [
    {
      method: ['GET', 'HEAD'],
      path: '/plugin/health',
      handler({ plugin, request }) {
        return Response.json({ plugin, method: request.method })
      },
    },
  ],
},
```

A route has an exact `path`, an optional method or method array (omitted means every method), and a
handler that returns `Response` or `Promise<Response>`. Each method/path pair must be unique across
plugins.

### `build`

All build hooks are optional, but `build` itself cannot be empty. Each concise hook registers once.

```ts
import path from 'node:path'

export default definePlugin({
  name: 'virtual-flags',
  build: {
    onStart({ root, outDir }) {
      console.log({ root, outDir })
    },
    onResolve({ id, root }) {
      return id === 'virtual:flags' ? path.join(root, '.virtual', 'flags.ts') : undefined
    },
    onLoad({ id }) {
      return id.endsWith('flags.ts') ? { code: 'export const checkoutV2 = true' } : undefined
    },
    onTransform({ code, id, environment }) {
      if (environment !== 'client' || !id.endsWith('.tsx')) return
      return { code: code.replaceAll('__CHANNEL__', JSON.stringify('stable')) }
    },
    onComplete({ outDir, manifest }) {
      console.log('output', outDir, manifest)
    },
  },
})
```

`onResolve` receives `id`, optional `importer`, `root`, and `environment`; return an absolute path,
`null`, or nothing. `onLoad` and `onTransform` may return source text, `{ code, map }`, `null`, or
nothing. The transform environment is one of `client`, `server`, `edge`, `worker`, or `shared`.

### `dev`, `diagnostics`, and `native`

```ts
export default definePlugin({
  name: 'content-tools',
  dev: {
    onFileChange: {
      match: ['content/*'],
      handler({ root, paths }) {
        console.log(root, paths)
      },
    },
  },
  diagnostics: [
    { level: 'info', code: 'CONTENT001', message: 'Content tools enabled' },
    { level: 'warning', code: 'CONTENT002', message: 'Remote sync is disabled' },
  ],
  native: {
    realtime: { path: '/events', heartbeatMs: 25_000, capacity: 256 },
  },
})
```

`dev.onFileChange` can also be a handler function directly. A registration can restrict it with
project-relative `match` patterns. `diagnostics` is one diagnostic or an array; levels are `info`,
`warning`, and `error`. `native.realtime: true` uses defaults; its options are `path`,
`heartbeatMs`, and `capacity`. Native capability ownership is exclusive: only one plugin can claim
`realtime@1`.

## 5. Advanced `register(api)` escape hatch

Use this form for repeated registrations, computed configuration, or deliberate ordering.

```ts
import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: 'security',
  register({ http, build, diagnostics }) {
    http.onRequest(requireAuthentication)
    http.onRequest(rateLimit)
    http.onResponse({ handler: addAuditHeader })
    http.route({ method: 'GET', path: '/plugin/metrics', handler: metrics })

    build.onTransform(instrumentClientCode)
    build.onTransform(removeDebugCalls)

    diagnostics.report({ level: 'info', code: 'SEC001', message: 'Security enabled' })
  },
})
```

All socket forms are available here:

```ts
register({ http, build, dev, diagnostics, native }) {
  http.onRequest(handlerOrRegistration)
  http.onResponse(handlerOrRegistration)
  http.route({ path, method, handler })
  build.onStart(hook)
  build.onResolve(hook)
  build.onLoad(hook)
  build.onTransform(hook)
  build.onComplete(hook)
  dev.onFileChange(handlerOrRegistration)
  diagnostics.report({ level, code, message })
  native.claim('realtime@1', options)
}
```

This hybrid is valid; its direct header hook registers before `register` adds the audit hook:

```ts
definePlugin({
  name: 'hybrid',
  headers: { 'x-powered-by': 'ruvyxa' },
  register({ http }) {
    http.onResponse({ handler: addAuditHeader })
  },
})
```

## 6. Make a configurable plugin

Export a factory when consumers need options. The factory must return `RuvyxaPlugin`.

```ts
import { definePlugin, type RuvyxaPlugin } from 'ruvyxa/plugin'

export interface AuditOptions {
  match?: readonly string[]
  header?: string
}

export function audit(options: AuditOptions = {}): RuvyxaPlugin {
  const match = options.match ?? ['/api/*']
  const header = options.header ?? 'x-audit-id'

  return definePlugin({
    name: 'audit',
    http: {
      match,
      onRequest({ request }) {
        if (!request.headers.has(header)) return new Response(`Missing ${header}`, { status: 400 })
      },
    },
  })
}
```

Consumers install `plugins: [audit({ header: 'x-trace-id' })]`.

## 7. Test and publish

Test a plugin as a unit with `createPluginHarness`. It runs `register(api)` against recording
sockets and exposes the same entry points the server uses, so no application has to boot:

```ts
import assert from 'node:assert/strict'
import { createPluginHarness } from 'ruvyxa/plugin-harness'

import siteTools from './index.js'

const harness = await createPluginHarness(siteTools)

// Response hooks, scoped by the plugin's own `match` patterns.
const response = await harness.respond(new Response('ok'), '/admin/users')
assert.equal(response.headers.get('x-site-tools'), 'enabled')

// A request hook that short-circuits reports the response it returned.
const blocked = await harness.request('/admin/users')
assert.equal(blocked.response?.status, 401)

// Registered routes, build hooks, dev hooks, diagnostics, and head entries.
assert.deepEqual(await (await harness.route('/plugin/status')).json(), {
  plugin: 'site-tools',
  ready: true,
})
await harness.build.start()
assert.equal(await harness.build.transform('const a = 1', '/a.ts'), null)
await harness.fileChange(['content/post.md'])
assert.equal(harness.diagnostics[0].code, 'SITE001')
assert.equal(harness.head.length, 2)
```

Pass an array to register several plugins in configuration order, which is how conflicts between
them surface. Then test a fixture application for anything that depends on real routing.

```bash
npm test
npm pack --dry-run
npx ruvyxa check --root ../my-app
npx ruvyxa test:parity --root ../my-app
```

Before publishing, ensure `ruvyxa` or `@ruvyxa/core` is a peer dependency when used, package ESM
output and declarations, and inspect the tarball. Do not publish tests, `node_modules`, `.ruvyxa`,
or dependencies that retain the `workspace:` protocol.

## 8. Validation and troubleshooting

| Symptom                         | Direct check                                                                       |
| ------------------------------- | ---------------------------------------------------------------------------------- |
| Plugin definition fails         | Supply a non-empty `name` and at least one declaration or `register(api)`          |
| Plugin is not loaded            | Check the config `plugins` array and its default import/export                     |
| Name collision                  | Give every plugin a distinct `name`                                                |
| Route collision                 | Make every method/path pair unique                                                 |
| Hook does not apply             | Check `match`, environment, and plugin order                                       |
| Virtual import does not resolve | Return an absolute path from `onResolve`, then source or a real file from `onLoad` |
| File handler does not run       | Use a project-relative match pattern such as `content/*`                           |
| Native claim fails              | Use a supported capability and ensure it has a single owner                        |

Do not expose private environment values through transformed client source or diagnostic messages.
