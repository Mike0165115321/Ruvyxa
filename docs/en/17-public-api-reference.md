# Public API reference

This reference lists stable exported surfaces found in package entry points. It intentionally
separates implementation details in Rust/runtime files from the APIs applications import.

## `ruvyxa`, `ruvyxa/server`, and `ruvyxa/config`

| Export                               | Signature / purpose                                                                    |
| ------------------------------------ | -------------------------------------------------------------------------------------- |
| `config`                             | `<T extends RuvyxaConfig>(config: T) => T`; typed config identity helper.              |
| `loader`                             | `(handler: LoaderHandler<T>) => Loader<T>`; handler gets `params`, `request`, `cache`. |
| `action`                             | Builder: `.input(schema)`, `.realtime(channels?)`, `.handler(fn)`.                     |
| `cache`                              | `(key) => CacheBuilder`; `.ttl(string)`, `.swr(string)`, `.get(producer)`.             |
| `invalidateCache`, `cacheStats`      | Remove exact/prefix/all cache entries; report `{ size, maxEntries }`.                  |
| `json`, `redirect`, `notFound`       | Response helpers; redirect only permits 3xx statuses.                                  |
| `definePlugin`, `withResponseHeader` | Plugin definition and response-header helper.                                          |
| `standaloneServerSource`             | Source generator for the standalone server artifact.                                   |

Types include `RuvyxaConfig`, `PageProps`, `GetStaticParams`, `RenderStrategy`, `Adapter`,
`MiddlewareConfig`, `ImageConfig`, `I18nConfig`, `SiteConfig`, and plugin contracts. Use imports
from `ruvyxa` for public primitives and `ruvyxa/config` or `ruvyxa/plugin` for explicit intent.

## `@ruvyxa/react`

| Export family    | Main names                                                                                                |
| ---------------- | --------------------------------------------------------------------------------------------------------- |
| Navigation       | `Link`, `useRouter`, `usePathname`, `useParams`, `useSearchParams`, `useSelectedRoute`, `useRouteContext` |
| Rendering errors | `RuvyxaErrorBoundary`, `notFound`, `isNotFoundError`, `RouteErrorProps`                                   |
| Metadata/content | `Seo`, `Meta`, `MetaFactory`, `Answer`                                                                    |
| Browser/runtime  | `hydrate`, `reportHydrationError`, `useRuvyxaLoader`                                                      |
| Assets           | `Image`                                                                                                   |

`useRuvyxaLoader<T>(loader, { enabled?, deps? })` returns `{ data, loading, error, refetch }`.
`hydrate({ root?, onError? })` dispatches the hydration event and installs optional reporting.
`notFound()` from this package always throws and therefore returns `never`.

## Other public packages

| Package             | Exported integration                                                                          |
| ------------------- | --------------------------------------------------------------------------------------------- |
| `@ruvyxa/auth`      | `createAuth`, providers, stores, client/plugin entry points, auth types/errors.               |
| `@ruvyxa/database`  | `createDatabase`, operation/types, `prismaAdapter`, `dynamoAdapter`, `defineDatabaseAdapter`. |
| `@ruvyxa/realtime`  | Plugin entry point; client exposes `createRealtimeClient`.                                    |
| `@ruvyxa/testing`   | `mockLoader`, `mockAction`, `mockCache`.                                                      |
| `@ruvyxa/adapter-*` | Typed build adapter packages.                                                                 |

For option details and defaults, use [Configuration](07-configuration.md) and the exported
TypeScript declarations in the installed package. Public API names shown here are source-verified;
runtime names beginning with `RUVYXA_` and double underscores are not application API.

**Previous:** [Troubleshooting and upgrade compatibility](16-troubleshooting-upgrades.md) ·
**Next:** [Documentation scope and sources](18-documentation-scope-and-sources.md)
