English Guide

This is the current Ruvyxa plugin system. A plugin is a TypeScript/JavaScript package that exports
one `definePlugin({ name, register })` value and is registered once in `ruvyxa.config.ts`.

## The mental model

`register()` receives sockets. Pick only the sockets you need:

| Socket        | Purpose                                                   |
| ------------- | --------------------------------------------------------- |
| `http`        | Requests, responses, and plugin-owned endpoints           |
| `build`       | Validation, resolve/load, transforms, and build artifacts |
| `dev`         | Development file-change events                            |
| `diagnostics` | Startup information, warnings, and errors                 |
| `native`      | Framework-owned, versioned capabilities                   |

The plugin is trusted server/build code, not a sandbox. You can use normal JavaScript, TypeScript,
npm packages, filesystem APIs, and server environment variables. Keep private code and secrets out
of client modules.

## Step 0: prerequisites

Install Node.js and npm or pnpm. Have a Ruvyxa application that can already run `ruvyxa dev`.

## Step 1: create a plugin

```bash
npx ruvyxa plugin create request-logger
cd request-logger
npm install
npm test
```

To place it in a monorepo directory:

```bash
npx ruvyxa plugin create request-logger --dir packages/request-logger
```

The generated package contains `src/index.ts`, `test/plugin.test.mjs`, `package.json`,
`tsconfig.json`, `README.md`, and `.gitignore`. There is no plugin category selector and no plugin
`--template` flag.

## Step 2: write the first plugin

```ts
import { definePlugin } from 'ruvyxa/plugin'

export default definePlugin({
  name: 'request-logger',
  register({ http }) {
    http.onResponse({
      handler({ response }) {
        const headers = new Headers(response.headers)
        headers.set('x-request-logger', 'active')
        return new Response(response.body, {
          status: response.status,
          statusText: response.statusText,
          headers,
        })
      },
    })
  },
})
```

`definePlugin()` validates the plugin name and returns the plugin object. The `register({ http })`
parameter exposes the HTTP socket used by this plugin.

## Step 3: install and register it

For a local package:

```bash
pnpm add ../packages/request-logger
```

For a published package:

```bash
npm install ruvyxa-plugin-request-logger
```

Register the exported value:

```ts
import { config } from 'ruvyxa/config'
import requestLogger from 'ruvyxa-plugin-request-logger'

export default config({ plugins: [requestLogger] })
```

The array order is registration order. Plugin names must be unique.

## Step 4: run and verify it

```bash
npx ruvyxa dev
curl -I http://localhost:3000/
```

Look for `x-request-logger: active` in the response. This confirms that the plugin loaded,
registered, and ran through the real host.

## Step 5: add HTTP behavior

Protect a route:

```ts
http.onRequest({
  match: ['/admin/*'],
  handler({ request }) {
    if (request.headers.get('authorization') !== `Bearer ${process.env.ADMIN_TOKEN}`) {
      return new Response('Unauthorized', { status: 401 })
    }
  },
})
```

Return nothing to continue, a `Request` to replace the request, or a `Response` to stop the chain.
Use `next()` or `next(replacement)` when you need explicit chain control.

Create an endpoint:

```ts
http.route({
  method: 'GET',
  path: '/plugin/status',
  handler({ plugin }) {
    return Response.json({ plugin, ready: true })
  },
})
```

The `method + path` pair must be unique. Match patterns may be omitted, use `*`, or use one trailing
wildcard such as `/api/*`; matching uses the pathname without the query string.

## Step 6: add build, dev, diagnostics, or native hooks

Transform client source:

```ts
build.onTransform(({ code, id, environment }) => {
  if (environment !== 'client' || !id.endsWith('.tsx')) return
  return code.replaceAll('__BUILD_CHANNEL__', JSON.stringify(process.env.CHANNEL ?? 'local'))
})
```

Resolve and load a virtual module:

```ts
import path from 'node:path'

build.onResolve(({ id, root }) =>
  id === 'virtual:feature-flags'
    ? path.join(root, '.ruvyxa-virtual', 'feature-flags.ts')
    : undefined,
)

build.onLoad(({ id }) =>
  id.endsWith('feature-flags.ts')
    ? { code: `export const flags = ${JSON.stringify({ checkoutV2: true })}` }
    : undefined,
)
```

`onResolve` returns an absolute path. `onLoad` can provide source even when no file exists.

Receive development changes and report diagnostics:

```ts
dev.onFileChange({
  match: ['content/*'],
  handler({ paths }) {
    console.log('changed', paths)
  },
})

diagnostics.report({ level: 'warning', code: 'ANL001', message: 'Analytics is disabled' })
```

Development paths are project-relative with `/`. Diagnostic levels are `info`, `warning`, and
`error`; an error stops startup. Do not put secrets in messages.

Use `native` only for a capability implemented by Ruvyxa, for example `realtime@1`:

```ts
native.claim('realtime@1', { path: '/__ruvyxa/realtime', capacity: 256 })
```

Plugins cannot load new Rust capabilities from npm, and each native capability has one owner.

## Step 7: accept typed options

Export a factory that returns `RuvyxaPlugin`:

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
    register({ http }) {
      http.onRequest({
        match,
        handler({ request }) {
          if (!request.headers.has(header))
            return new Response(`Missing ${header}`, { status: 400 })
        },
      })
    },
  })
}
```

The app can then use `plugins: [audit({ header: 'x-trace-id' })]`.

## Step 8: test it

Run the generated unit test and add socket-spy tests for every registration:

```bash
npm test
npm pack --dry-run
```

Then test the real host and app:

```bash
npx ruvyxa check --root ../my-app
npx ruvyxa test:parity --root ../my-app
```

For build hooks, virtual modules, routes, native claims, and response bodies, include at least one
real fixture-app integration test. Assert the exported plugin name and registered hooks in contract
tests.

## Step 9: publish checklist

Before `npm publish`:

1. Put `ruvyxa` or `@ruvyxa/core` in `peerDependencies`.
2. Publish compiled `dist` files and declarations.
3. Exclude tests, `node_modules`, `.ruvyxa`, and `workspace:` dependencies from the tarball.
4. Use ESM exports; no Ruvyxa-specific package metadata is required.
5. Document installation, registration, options, security, and deployment limits.
6. Run `npm test` and `npm pack --dry-run` before publishing.

## Security and troubleshooting

Plugins have server/build privileges. Do not use plugin memory as durable state, do not expose
private environment variables to client code, and avoid response hooks for very large downloads
because responses are bounded. Timeouts are not automatically retried because side effects may
already have occurred.

| Symptom                    | Check                                                                  |
| -------------------------- | ---------------------------------------------------------------------- |
| Plugin validation fails    | Provide a non-empty `name` and a `register(api)` function              |
| Plugin not loaded          | Check the config `plugins` array and default export                    |
| Duplicate name             | Give every plugin a unique `name`                                      |
| Route conflict             | Check the `method + path` pair                                         |
| Unresolved import          | Return an absolute path from `onResolve` and source/file from `onLoad` |
| Hook does not run          | Check `match`, `environment`, and registration order                   |
| File change does not match | Use a project-relative pattern such as `content/*`                     |
