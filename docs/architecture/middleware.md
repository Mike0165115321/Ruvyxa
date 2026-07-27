# Middleware Architecture

Ruvyxa has two HTTP middleware locations with one deterministic request pipeline:

1. Built-in policies use native Rust/Tower layers for CORS, rate limiting, timing, logging,
   compression, and security headers.
2. Application and package plugins use the `http` socket with standard Fetch primitives.

```ts
import { config } from 'ruvyxa/config'
import { definePlugin } from 'ruvyxa/plugin'

const auth = definePlugin({
  name: 'auth',
  register({ http }) {
    http.onRequest({
      match: ['/api/*'],
      handler({ request }) {
        if (!request.headers.has('authorization')) {
          return new Response('Unauthorized', { status: 401 })
        }
      },
    })
    http.onResponse({
      match: ['/api/*'],
      handler({ response, plugin }) {
        const headers = new Headers(response.headers)
        headers.set('x-plugin', plugin)
        return new Response(response.body, { status: response.status, headers })
      },
    })
  },
})

export default config({
  middleware: { builtin: { timing: true, log: true } },
  plugins: [auth],
})
```

Returning nothing continues. A returned `Request` replaces the request; a returned `Response`
short-circuits request hooks and application routing. Response hooks may replace the response.
`next()` and `next(replacement)` provide the same continuation behavior when it is clearer inside a
branch.

## Runtime boundary

The Rust server owns the socket, Axum routing, body limits, and final response. A bounded
`PluginHost` worker pool executes callbacks in Node or Bun over NDJSON:

```text
Rust -> { hook: "http.request", request: ... }
Node -> { ok: true, result: { kind: "request" | "response", ... } }

Rust -> { hook: "http.response", request: ..., response: ... }
Node -> { ok: true, result: { response: ... } }
```

Headers use ordered pairs and bodies use base64. Rust validates every result and enforces
`security.pluginLimit` before response buffering. Plugin-owned exact routes are registered through
`http.route()` and participate in the request phase; duplicate method/path ownership is rejected
when the registry starts.

## Ordering and failures

Plugin request handlers run in config and source order. The first explicit response stops the
request chain. Response handlers run sequentially on the current response. Connection middleware
runs before plugin request processing; native security headers are filled after plugin response
processing so explicit application/plugin values are preserved.

Exceptions and unsupported return values fail the call with a named diagnostic. A timeout does not
retry a possibly side-effecting handler; the poisoned worker is replaced. Console output remains
visible on stderr while stdout stays reserved for the protocol.

There is no separate middleware-plugin object model. HTTP behavior is one socket on the same plugin
that may also register build, dev, diagnostic, or native behavior.
