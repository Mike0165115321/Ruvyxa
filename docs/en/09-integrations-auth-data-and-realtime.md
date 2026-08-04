# Integrations: authentication, data, realtime, adapters, and testing

## Authentication

`@ruvyxa/auth` exports `createAuth`, provider helpers `google` and `github`, memory stores, types,
and `AuthError`. Its package exports `@ruvyxa/auth/client` and `@ruvyxa/auth/plugin` separately.
Supported provider contracts include credentials, OAuth, magic link, and WebAuthn. The memory stores
are process-local; choose a durable shared store before horizontally scaling authentication.

```ts
import { createAuth, memoryAuthStore, memoryRateLimitStore } from '@ruvyxa/auth'

const auth = createAuth({
  secret: process.env.RUVYXA_AUTH_SECRET!,
  origin: 'https://example.test',
  store: memoryAuthStore({ development: true }),
  rateLimitStore: memoryRateLimitStore({ development: true }),
  providers: {},
})
```

The exact `AuthOptions` contract is exported by the package; do not pass this example's placeholder
as a real secret. Register the plugin returned by the auth runtime, then use the separate browser
entry point only in client code:

```ts
// ruvyxa.config.ts
export default config({ plugins: [auth.plugin] })

// a client module
import { createAuthClient } from '@ruvyxa/auth/client'
const authClient = createAuthClient()
```

The default auth path is `/__ruvyxa/auth`. The client exposes `login`, `logout`, `session`, and
`oauth`; `createAuth` also exposes `handle`, `login`, `getSession`, and `logout` for server-side
integration. The memory stores require `{ development: true }` and deliberately fail the production
build with `RUV3105`; provide durable `AuthStore` and `AuthRateLimitStore` implementations instead.
`createAuthPlugin(bridge)` is available when a custom bridge is needed.

## Database

`@ruvyxa/database` is a typed normalized-operation layer, not an ORM migration system.
`createDatabase<TSchema>(adapter)` creates model delegates for `findMany`, `findFirst`,
`findUnique`, `create`, `createMany`, `update`, `updateMany`, `delete`, `deleteMany`, and `count`.
It supplies `prismaAdapter`, `dynamoAdapter`, and `defineDatabaseAdapter`; adapter errors use
`RUV3001`–`RUV3003`.

```ts
import { createDatabase, defineDatabaseAdapter } from '@ruvyxa/database'
const adapter = defineDatabaseAdapter({
  name: 'example',
  execute: async (operation) => {
    throw new Error(`implement ${operation.kind}`)
  },
})
const db = createDatabase<{ todo: { id: string; title: string } }>(adapter)
```

The framework does not ship a database server, migration engine, or backup service. Those remain
application/infrastructure responsibilities.

## Realtime and adapters

`@ruvyxa/realtime/plugin` exports `realtime()`, which claims native realtime capability. It rejects
builds that are not long-lived Node/Bun output and explicitly rejects aws, cloudflare, firebase,
netlify, static, and vercel adapters with `RUV3201`. `@ruvyxa/realtime/client` exports
`createRealtimeClient`; it caps active channels at 16 and reconnects with bounded exponential
backoff.

First-party adapter packages exist for Node, Bun, static, Vercel, Netlify, Cloudflare, Railway,
Render, Firebase, and AWS. Build selection is `ruvyxa build --adapter <name>` or config `adapter`;
see [Deploy, run, and operate](15-deploy-run-and-operate.md). `@ruvyxa/testing` exports
`mockLoader`, `mockAction`, and `mockCache` for unit tests.

**Previous:** [Plugins and middleware](08-plugins-middleware.md) · **Next:**
[CLI reference](10-cli.md)
