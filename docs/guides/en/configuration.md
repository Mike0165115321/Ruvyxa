# Configuration Reference

> 🔴 **Reference** · ⏱️ dip in as needed
>
> **You'll find:** every `ruvyxa.config.ts` option — server, build, rendering, security, cache,
> styles — with defaults and types.

**Beginners: you can skip this chapter for now.** A new project works with the generated
`ruvyxa.config.ts` untouched — every option below has a sensible default. Come back here when you
need to change a port, add an adapter, tune caching, or tighten security, and use your editor's
autocomplete (the `config()` helper is fully typed) to explore.

Use `config()` so TypeScript validates the public configuration shape:

```ts
import { config } from 'ruvyxa/config'

export default config({
  appDir: 'app',
  outDir: '.ruvyxa',
  css: { entries: ['styles/theme.css'] },
  server: { host: 'localhost', port: 3000 },
  build: {
    minify: true,
    map: false,
    treeShake: true,
    split: 'route',
    workers: 4,
    jsx: 'automatic',
    target: 'es2022',
    manifest: false,
    warm: true,
    prerenderCache: true,
  },
  plugins: [],
  middleware: {
    builtin: { log: true, rate: true, cors: false, headers: {} },
  },
  render: { strategy: 'ssr', revalidate: 60 },
  cache: { routes: true, css: true, dir: '.ruvyxa/cache/bundler' },
  debug: { overlay: true, traces: false },
  image: { optimize: true, quality: 82, lossless: false, keepOriginal: true, workers: 0 },
  security: {
    actionLimit: 1024 * 1024,
    apiLimit: 10 * 1024 * 1024,
    pluginLimit: 32 * 1024 * 1024,
    actionRateLimit: { max: 600, window: 60 },
    sameOrigin: true,
    fetchMeta: true,
    trustedProxyIps: ['10.0.0.2'],
    headers: true,
  },
})
```

Unknown configuration keys intentionally fail rather than being ignored — this prevents typos from
silently changing deployment behaviour.

---

## Reference by Section

### `appDir`

| Property       | Value                                                                            |
| -------------- | -------------------------------------------------------------------------------- |
| **Type**       | `string`                                                                         |
| **Default**    | `"app"`                                                                          |
| **Constraint** | Must be a project-relative path. Absolute paths and `..` traversal are rejected. |

### `outDir`

| Property       | Value             |
| -------------- | ----------------- |
| **Type**       | `string`          |
| **Default**    | `".ruvyxa"`       |
| **Constraint** | Same as `appDir`. |

### `css`

| Field     | Type       | Default | Description                                     |
| --------- | ---------- | ------- | ----------------------------------------------- |
| `entries` | `string[]` | `[]`    | Global CSS files/dirs not imported by app code. |

### `server`

| Field  | Type     | Default       |
| ------ | -------- | ------------- |
| `host` | `string` | `"localhost"` |
| `port` | `number` | `3000`        |

### `build`

| Field            | Type      | Default          | Options                                                                                                                                                                                        |
| ---------------- | --------- | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `minify`         | `boolean` | `true`           | Oxc-powered JavaScript minification                                                                                                                                                            |
| `map`            | `boolean` | `false`          | Emit source maps                                                                                                                                                                               |
| `treeShake`      | `boolean` | `true`           | Linker-aware tree shaking                                                                                                                                                                      |
| `split`          | `string`  | `"route"`        | `"single"`, `"route"` (`"manual"` is an alias for `"single"`)                                                                                                                                  |
| `workers`        | `number`  | CPU count (auto) | Bounded concurrency for route preparation/final emission plus prerendering. Example `workers: 4` is an explicit override; prerendering remains capped to avoid excessive JavaScript processes. |
| `jsx`            | `string`  | `"automatic"`    | JSX runtime mode; use `"classic"` only for code that provides a React global/import                                                                                                            |
| `target`         | `string`  | `"es2022"`       | `"es2018"`, `"es2019"`, `"es2020"`, `"es2022"`, `"esnext"`                                                                                                                                     |
| `manifest`       | `boolean` | `false`          | Emit chunk manifest                                                                                                                                                                            |
| `warm`           | `boolean` | `true`           | Pre-bundle dependencies in dev server (no effect during production build)                                                                                                                      |
| `prerenderCache` | `boolean` | `true`           | Reuse final SSG/ISR/PPR HTML only when config, environment, assets, styles, and every source fingerprint match; disable for intentionally non-deterministic pages.                             |

### `plugins`

Register values created by `definePlugin({ name, register })` from `ruvyxa/plugin`. The same plugin
may use any combination of grouped `http`, `build`, `dev`, `diagnostics`, and `native` sockets.
Plugins execute in declaration/registration order. See the [plugin guide](plugin.md).

### `middleware`

#### Builtin Middleware

```ts
middleware: {
  workers: 1,              // TypeScript middleware processes (1-8)
  timeoutMs: 30_000,       // per-hook timeout (1-300,000 ms)
  builtin: {
    timing: true,           // server-timing response headers
    log: true,              // request logging
    rate: {                 // rate limiting
      max: 100,
      window: 60,
      key: 'ip',
    },
    cors: {                 // CORS
      origins: ['https://myapp.com'],
      methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
      credentials: true,
      maxAge: 86400,
    },
    headers: {              // custom response headers
      'X-Powered-By': 'Ruvyxa',
    },
  },
}
```

The `http` socket accepts `onRequest`, `onResponse`, and exact `route` registrations using Fetch
objects. The `build` socket provides start, resolve, load, transform, and complete handlers. All
handlers run in declaration/registration order through the plugin runtime.

`workers` defaults to one because module state is process-local. `timeoutMs` defaults to 30 seconds;
a timed-out or protocol-corrupt worker is replaced without retrying that hook, while a worker that
exits before responding is restarted and retried once.

### `render`

| Field        | Type     | Default | Description                           |
| ------------ | -------- | ------- | ------------------------------------- |
| `strategy`   | `string` | `"ssr"` | Default rendering strategy            |
| `revalidate` | `number` | —       | Default revalidate interval (seconds) |

### `cache`

| Field    | Type      | Default                   | Description           |
| -------- | --------- | ------------------------- | --------------------- |
| `routes` | `boolean` | `true`                    | Cache route manifest  |
| `css`    | `boolean` | `true`                    | Cache collected CSS   |
| `dir`    | `string`  | `".ruvyxa/cache/bundler"` | Build cache directory |

### `site`

Identity for the crawler discovery files the build emits into the output assets.

| Field     | Type                           | Default | Description                                   |
| --------- | ------------------------------ | ------- | --------------------------------------------- |
| `url`     | `string`                       | —       | Actual absolute production origin             |
| `sitemap` | `boolean \| SiteSitemapConfig` | `true`  | Emit `sitemap.xml` (needs a resolvable `url`) |
| `robots`  | `boolean \| SiteRobotsConfig`  | `true`  | Emit `robots.txt` or declare crawler rules    |

`ruvyxa build` writes `robots.txt` and `sitemap.xml` from the route manifest and concrete paths
produced by prerendering. API routes and unresolved patterns such as `/blog/[slug]` are not pages.
Sitemaps are UTF-8, use absolute escaped URLs, and are automatically split into an index plus
numbered files before either the 50,000 URL or 50 MB protocol limit is crossed.

A file of the same name in `public/` suppresses core generation. An exact `app/sitemap.xml/route.ts`
or `app/robots.txt/route.ts` route also suppresses it, allowing fully programmatic output. Explicit
first-party plugins run after core generation and may intentionally replace the staged asset they
own.

When `url` is absent, resolution tries `RUVYXA_SITE_URL`, Vercel's production project URL, and then
Netlify's production `URL`. A Vercel preview URL is never selected as canonical. A bare hostname is
given an `https` scheme; credentials, paths, queries, and fragments are rejected. Without a
production origin, the build warns and writes only `robots.txt`.

```ts
const siteUrl = process.env.RUVYXA_SITE_URL
if (!siteUrl) throw new Error('RUVYXA_SITE_URL must be set to the deployed application origin')

const absoluteUrl = (pathname: string) => new URL(pathname, siteUrl).href

export default config({
  site: {
    url: siteUrl,
    sitemap: {
      exclude: ['/admin/*', '/drafts/*'],
      defaults: {
        changeFrequency: 'weekly',
        priority: 0.7,
      },
      entries: [
        {
          url: '/',
          lastModified: new Date('2026-07-29'),
          changeFrequency: 'daily',
          priority: 1,
          images: [absoluteUrl('/ruvyxa.png')],
        },
        { url: '/about', changeFrequency: 'monthly', priority: 0.6 },
      ],
    },
    robots: {
      rules: [
        { userAgent: '*', allow: '/', disallow: ['/admin/', '/api/'] },
        { userAgent: 'GPTBot', disallow: '/' },
      ],
      host: siteUrl,
    },
  },
})
```

`exclude` accepts exact paths or a trailing `*` prefix. `additionalPaths` must contain concrete
root-relative paths. `defaults` applies `lastModified`, `changeFrequency`, and `priority` to every
discovered URL; a matching object in `entries` overrides those fields and can add language
alternates, image URLs, and video metadata. Entry URLs may be root-relative or absolute on the
configured origin. Dates accept `Date`, `YYYY-MM-DD`, or an RFC 3339 timestamp. Use the real content
modification time rather than changing it on every build. Namespaces are emitted only when needed,
and invalid dates, priorities, cross-origin entries, media URLs, and video fields fail the build.

Every root-relative entry is joined to the resolved production origin. Never commit a guessed
domain: inject the real deployment origin through `RUVYXA_SITE_URL` (as above), or omit `url` and
let a supported host provide its production URL. Absolute media and alternate URLs must point to
routes or assets that actually exist on the deployed site.

Robots rules accept a string or string array for `userAgent`, `allow`, and `disallow`, plus
`crawlDelay`; `sitemap` can override the default with one or more absolute URLs. Route-level
`meta.noindex` is intentionally independent, so also exclude a page when it must not appear in the
generated sitemap.

### `debug`

| Field     | Type      | Default | Description          |
| --------- | --------- | ------- | -------------------- |
| `overlay` | `boolean` | `true`  | Error overlay in dev |
| `traces`  | `boolean` | `false` | Debug trace output   |

### `image`

| Field          | Type      | Default | Description                                 |
| -------------- | --------- | ------- | ------------------------------------------- |
| `optimize`     | `boolean` | `true`  | Convert PNG/JPEG to WebP                    |
| `quality`      | `number`  | `82`    | WebP quality (1–100)                        |
| `lossless`     | `boolean` | `false` | Lossless WebP mode                          |
| `keepOriginal` | `boolean` | `true`  | Publish the source PNG/JPEG beside the WebP |
| `workers`      | `number`  | `0`     | Thread count (0 = CPU count)                |

`keepOriginal` exists because `public/` is a URL contract: a file you put there is served at the
matching path. `ruvyxa dev` and `ruvyxa start` resolve `/logo.png` to `logo.webp` when only the WebP
was published, but a CDN (Vercel, Netlify, Cloudflare, S3) has no such fallback, so a plain
`<img src="/logo.png">` would 404 in production only. Keeping the original makes both URLs valid on
every host. Use `<Image>` from `@ruvyxa/react`, which points at the WebP, to actually serve the
smaller file; turn `keepOriginal` off only when every reference goes through `<Image>`.

### `security`

| Field             | Type              | Default                    | Description                                                                |
| ----------------- | ----------------- | -------------------------- | -------------------------------------------------------------------------- |
| `actionLimit`     | `number`          | `1048576` (1 MiB)          | Body size limit for actions                                                |
| `apiLimit`        | `number`          | `10485760` (10 MiB)        | Body size limit for API routes                                             |
| `pluginLimit`     | `number`          | `33554432` (32 MiB)        | Max buffered response for plugin response middleware                       |
| `actionRateLimit` | `{ max, window }` | `{ max: 600, window: 60 }` | Rate limit per client-action per window                                    |
| `sameOrigin`      | `boolean`         | `true`                     | Same-origin validation for actions                                         |
| `fetchMeta`       | `boolean`         | `true`                     | Fetch Metadata protection                                                  |
| `trustedProxyIps` | `string[]`        | `[]`                       | Exact non-loopback proxies trusted for forwarded identity/protocol headers |
| `headers`         | `boolean`         | `true`                     | Fill missing response headers with Ruvyxa security defaults                |

Security limits must be positive when set.

Loopback proxies are trusted without configuration. When a reverse proxy runs elsewhere, list its
exact IP in `trustedProxyIps`; private network ranges are not trusted implicitly.

### `adapter`

```ts
import { config } from 'ruvyxa/config'
import { vercelAdapter } from '@ruvyxa/adapter-vercel'

export default config({
  adapter: vercelAdapter(),
})
```

An adapter's `build()` function is evaluated while configuration is loaded and again after the
production build to materialize its declared artifacts inside `.ruvyxa/`. The result is written as
`adapterArtifacts` in `.ruvyxa/build.json`. Node and Bun adapters create launchers. Cloudflare,
Netlify, and Vercel adapters are hybrid: they emit a static publish directory for pre-rendered pages
and client assets alongside a serverless function that serves SSR and API routes.

The function artifact contains `route-modules.mjs`, a compiled static registry bundle used by the
platform handler. Adapter handlers do not execute copied `.ts`/`.tsx` source files.

Each adapter declares the route kinds and render strategies it can deploy. Routes outside that set
are rejected with `RUV2202`, naming each unsupported route, before the adapter's `build()` runs:

| Adapter                      | Target                    | Deployable routes  |
| ---------------------------- | ------------------------- | ------------------ |
| `@ruvyxa/adapter-node`       | Node launcher             | all                |
| `@ruvyxa/adapter-bun`        | Bun launcher              | all                |
| `@ruvyxa/adapter-vercel`     | Vercel static + function  | all                |
| `@ruvyxa/adapter-netlify`    | Netlify static + function | all                |
| `@ruvyxa/adapter-cloudflare` | Worker + asset binding    | SSR, SSG, CSR, API |
| `@ruvyxa/adapter-static`     | Static files              | SSG, CSR           |

Cloudflare excludes ISR and PPR because a Worker's asset binding is read-only, so there is nowhere
to write a revalidated page. The static adapter has no server at all.

### `runtime`

```ts
export default config({
  runtime: 'bun', // 'node' or 'bun'; omitted means Node, then Bun if Node is unavailable
})
```

`runtime` selects the JavaScript runtime that executes Ruvyxa configuration, SSR, static rendering,
API routes, actions, and plugins. It does not change the Rust HTTP server. When omitted, Ruvyxa
prefers Node and automatically falls back to Bun if Node is unavailable.

Set `RUVYXA_RUNTIME=bun` in the app command when Bun must be used from the first configuration load,
for example `RUVYXA_RUNTIME=bun bunx ruvyxa dev`. This bootstrap override takes precedence over
`runtime` and is useful in CI.

For backward compatibility, `runtime: 'edge'` and `runtime: 'static'` remain build-target aliases
and execute JavaScript with Node. New deployment builds should use `ruvyxa build --target edge` or
`ruvyxa build --target static` instead.
