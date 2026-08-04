# Configuration and environment

`ruvyxa.config.ts` is evaluated by the configuration renderer then validated. Use `config()` from
`ruvyxa/config` for typed authoring. Configuration names below come from `RuvyxaConfig` and its
nested source types.

## Primary options

| Key                                                                      | Type / default                                  | Effect                                     |
| ------------------------------------------------------------------------ | ----------------------------------------------- | ------------------------------------------ |
| `appDir`, `outDir`                                                       | strings                                         | App source and generated output locations. |
| `runtime`                                                                | `node \| bun \| edge \| static`, default `node` | Runtime/target policy.                     |
| `server.host`, `server.port`                                             | string, number                                  | Dev/start listening address.               |
| `build.minify`, `map`, `treeShake`, `manifest`, `warm`, `prerenderCache` | booleans; cache defaults true                   | Compiler/build artifact behavior.          |
| `build.split`                                                            | `single \| route \| manual`                     | Bundle splitting policy.                   |
| `build.workers`                                                          | number                                          | Build parallelism.                         |
| `render.strategy`, `render.revalidate`                                   | strategy, seconds                               | Default page rendering policy.             |
| `cache.routes`, `cache.css`, `cache.dir`                                 | booleans/string                                 | Route/CSS/cache-directory settings.        |

## Complete option map

| Group         | Keys                                                                                                                   | Operational decision                                                                                                                                                                                              |
| ------------- | ---------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Root          | `appDir`, `outDir`, `runtime`, `react`, `typescript.strict`                                                            | Keep defaults unless the source/output layout or target requires a change. `runtime` is `node`, `bun`, `edge`, or `static`; the CLI target can override it.                                                       |
| CSS and debug | `css.entries`, `debug.overlay`, `debug.traces`                                                                         | `entries` is for project-relative global styles not imported by a module. Debug flags change development diagnostics, not production access control.                                                              |
| Build         | `minify`, `map`, `treeShake`, `split`, `workers`, `jsx`, `target`, `manifest`, `warm`, `prerenderCache`                | `split` is `single`, `route`, or `manual`; `jsx` is `classic` or `automatic`; target is `es2018`, `es2019`, `es2020`, `es2022`, or `esnext`. Use source maps deliberately because they can expose source content. |
| Rendering     | `render.strategy`, `render.revalidate`                                                                                 | Strategy is `ssr`, `ssg`, `isr`, `csr`, or `ppr`. The default strategy is SSR and default revalidation is 60 seconds.                                                                                             |
| Image         | `optimize`, `quality`, `lossless`, `keepOriginal`, `variantWidths`, `workers`, `onDemand.enabled`, `onDemand.maxWidth` | Defaults are optimize true, quality 82, lossless false, keep-original true, listed standard widths, and workers 0 (available CPU count). Object-form on-demand mode defaults enabled with max width 3840.         |
| i18n          | `locales`, `defaultLocale`, `localeParam`, `detectLocale`, `cookie`                                                    | `locales` and `defaultLocale` are required when i18n is set. Default param is `lang`, detection true, cookie `RUVYXA_LOCALE`.                                                                                     |
| Site          | `site.url`, `site.sitemap`, `site.robots`                                                                              | Sitemap may set `exclude`, `additionalPaths`, `defaults`, and enriched `entries`; robots may set rules, sitemap URLs, and host.                                                                                   |
| Middleware    | `builtin.cors`, `builtin.timing`, `builtin.log`, `builtin.rate`, `builtin.headers`, `workers`, `timeoutMs`             | CORS has origins/methods/headers/credentials/maxAge. Built-in rate needs `max`, `window`, optional `key`. Plugin workers are 1–8; timeout is 30,000 ms by default and at most 300,000.                            |
| Integration   | `adapter`, `adapterOptions`, `plugins`                                                                                 | Use an adapter for build output and an array of `RuvyxaPlugin` values for extensions.                                                                                                                             |

## Production configuration example

Start from this narrow configuration, then add only the features your application has tested. The
values are all supported option names; replace the example origin before release.

```ts
import { config } from 'ruvyxa/config'
import { requireEnv, securityHeaders } from 'ruvyxa/plugins'

export default config({
  site: { url: 'https://app.example.com', sitemap: true, robots: true },
  build: { minify: true, map: false, treeShake: true, split: 'route', prerenderCache: true },
  security: { actionLimit: 1_048_576, apiLimit: 10_485_760, sameOrigin: true, fetchMeta: true },
  plugins: [
    requireEnv(['DATABASE_URL', 'RUVYXA_AUTH_SECRET']),
    securityHeaders({ contentSecurityPolicy: { 'default-src': ["'self'"] } }),
  ],
})
```

`requireEnv` validates names at the end of the production build, so configure its required values in
the same build environment. It does not read a secret into browser code. A CSP commonly needs extra
sources for analytics, images, fonts, or APIs; test every route after tightening it.

```ts
import { config } from 'ruvyxa/config'

export default config({
  build: { minify: true, map: false, treeShake: true, split: 'route', prerenderCache: true },
  render: { strategy: 'ssr', revalidate: 60 },
  image: {
    optimize: true,
    quality: 82,
    variantWidths: [640, 1200],
    onDemand: { enabled: true, maxWidth: 1920 },
  },
  i18n: { locales: ['en', 'th'], defaultLocale: 'en' },
})
```

## Security, middleware, site, and plugins

`security.actionLimit` defaults to 1,048,576 bytes; `security.apiLimit` defaults to 10,485,760
bytes; `security.pluginLimit` defaults to 33,554,432 and is capped at 268,435,456.
`security.actionRateLimit` defaults to 600 requests in 60 seconds. `trustedProxyIps` accepts exact
IPv4/IPv6 addresses or CIDR ranges; only configured non-loopback proxies may supply forwarded
client/protocol headers.

`middleware` contains built-ins (`cors`, `timing`, `log`, `rate`, `headers`) and TypeScript plugin
`workers` (1–8) and `timeoutMs` (default 30,000, maximum 300,000). `site` configures build-time
`sitemap.xml` and `robots.txt`; an exact app route or same-named `public/` file suppresses the core
generator. `plugins` is the array of `RuvyxaPlugin` objects.

## Environment variables

| Variable                                                                                                         | Evidence-backed purpose                       |
| ---------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| `RUVYXA_SITE_URL`                                                                                                | Fallback canonical origin for site discovery. |
| `RUVYXA_RUNTIME`                                                                                                 | CLI/runtime override used by dev/build paths. |
| `RUVYXA_ADAPTER`                                                                                                 | Build adapter selection override.             |
| `RUVYXA_BUILD_CACHE_DIR`                                                                                         | Shared build cache directory override.        |
| `RUVYXA_RENDER_CACHE_SIZE`                                                                                       | Render-cache capacity.                        |
| `RUVYXA_WORKER_POOL_SIZE`, `RUVYXA_WORKER_TIMEOUT_MS`, `RUVYXA_WORKER_MAX_CONCURRENCY`, `RUVYXA_MEMORY_LIMIT_MB` | Worker-pool operational controls.             |
| `RUVYXA_PUBLIC_*`                                                                                                | Browser-safe values injected for client use.  |

Internal variables beginning or ending in double underscores are runtime transport details, not
application configuration. Never set them manually. Values such as `RUVYXA_AUTH_SECRET` occur in the
auth scaffolder; use a private environment source and never expose one with the public prefix.

**Previous:** [UI, navigation, metadata, and assets](06-ui-navigation-metadata-and-assets.md) ·
**Next:** [Plugins and middleware](08-plugins-middleware.md)
