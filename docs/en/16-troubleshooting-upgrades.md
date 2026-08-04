# Troubleshooting and upgrade compatibility

Run the narrowest diagnostic first, from the application root:

```bash
pnpm routes
pnpm check
pnpm analyze
pnpm doctor
pnpm trace --help
pnpm test:parity
```

## Symptoms and evidence-backed fixes

| Symptom                                        | Likely condition                                                                                 | Check and remedy                                                                                |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| A route is absent                              | File does not follow discovered special-file/dynamic-segment rules.                              | Run `routes`; compare its directory/name with [Project structure](03-project-structure.md).     |
| Client build reports private import/env access | Boundary validation found a server-only import or non-public environment value in a client path. | Move the work server-side; expose only deliberately safe `RUVYXA_PUBLIC_*` values.              |
| Static build fails                             | Static adapter has no generated prerender pages, or the route needs a runtime-only behavior.     | Use a compatible target or supply static params/route strategy; inspect build output.           |
| `RUV2102`                                      | Plugin definition is missing a name/behavior or has invalid hook shape.                          | Ensure `definePlugin` has a non-empty `name` and a valid declaration/register callback.         |
| `RUV3001`–`RUV3003`                            | Database adapter input, mapping, or operation cannot be satisfied.                               | Inspect `DatabaseAdapterError` message and adapter model/table mapping.                         |
| `RUV3201`                                      | Native realtime was built for an unsupported target/adapter.                                     | Deploy long-lived Node/Bun output, or remove realtime.                                          |
| Actions/API reject a body                      | Body exceeds configured action/API limit or input parser throws.                                 | Review `security.actionLimit`/`apiLimit`; validate and return a safe application error.         |
| Cache seems stale                              | The entry is inside TTL/SWR or another process has its own memory cache.                         | Use `invalidateCache`, inspect strategy, and use shared infrastructure for multi-instance data. |

## Common questions

**Why does a route 404 after calling `notFound`?** `@ruvyxa/react` throws a tagged signal and the
nearest route boundary renders `not-found.tsx`. `ruvyxa/server` instead returns a 404 response.
Import the version appropriate to page rendering or an HTTP handler.

**Why is an environment value missing in the browser?** Only `RUVYXA_PUBLIC_*` is intentionally
available client-side. Move secrets or server-only computation out of client code rather than
changing the prefix.

**Can I upgrade without a migration guide?** This repository includes `CHANGELOG.md`, but this
documentation does not infer a version-by-version migration path from it. Before upgrading, compare
exports/config types and run `pnpm check`, `pnpm build`, and `pnpm test:parity` against your app.
Treat deprecated `Seo.twitterCard` as a concrete migration: use `Seo.card`.

**Previous:** [Deploy, run, and operate in production](15-deploy-run-and-operate.md) · **Next:**
[Public API reference](17-public-api-reference.md)
