# Plugins

Ruvyxa plugins are lifecycle extensions registered through the public plugin API. This page
separates the current implementation contract from retained historical examples so a build or
production review can trace each claim to source.

## Production contract

### Current built-in exports

The first-party package export is ruvyxa/plugins. The current source exports these 16 builders:

```ts
redirects
headers
observability
securityHeaders
cacheRules
pwa
sitemap
robots
feed
searchIndex
contentEngine
openApi
alias
bundleBudget
requireEnv
fonts
```

These are built-in plugin factories, not separately versioned packages. The checked-in first-party
package manifests use release version 1.0.26; realtime@1 is a native capability/protocol identifier,
not a plugin package version.

### Configuration and registration

```ts
import { config } from 'ruvyxa/config'
import { redirects, headers, observability, securityHeaders, cacheRules } from 'ruvyxa/plugins'

export default config({
  plugins: [
    redirects([{ source: '/old-page', destination: '/new-page', permanent: true }]),
    headers([{ source: '/api/*', headers: { 'cache-control': 'no-store' } }]),
    observability(),
    securityHeaders(),
    cacheRules([{ source: '/assets/*', browser: 'public, max-age=3600' }]),
  ],
})
```

The configuration entry point is config(). Route patterns use exact matches, *, or a trailing prefix
wildcard according to the factory. The implementation validates rule shapes, header values, paths,
and plugin-specific options at registration/build time.

### Lifecycle hooks

The public plugin bridge includes build hooks (onStart, onResolve, onLoad, onTransform, onComplete),
HTTP hooks (onRequest, onResponse, route), development file-change hooks, diagnostics reporting, and
native capability claims. The exact payload types are defined in packages/@ruvyxa/core/src/plugin.ts
and packages/@ruvyxa/core/src/types.ts; do not infer a payload shape from an example belonging to
another hook.

Build hooks run through the TypeScript plugin worker bridge. HTTP hooks are invoked by the
middleware bridge around request/response handling. A plugin hook has a default timeout of 30
seconds and a maximum configured timeout of 300 seconds; timeout is reported as RUV1700 and
malformed plugin protocol/response data as RUV1701.

### Source-backed behavior by builder

- redirects: emits 307 or 308 responses and rejects unsafe destination forms.
- headers: sets configured response headers for matching routes.
- observability: can add request IDs, trace context, Server-Timing, and structured timing logs;
  measured duration is workload data, not a benchmark guarantee.
- securityHeaders: applies configured response security policy; application-specific CSP choices
  remain the owner's responsibility.
- cacheRules: writes browser/CDN cache policy and Vary values without claiming cache coherence.
- pwa, sitemap, robots, feed, searchIndex, and contentEngine: emit build artifacts from validated
  project inputs.
- openApi: emits a validated OpenAPI document from explicitly supplied operations.
- alias: registers module-resolution aliases.
- bundleBudget: checks configured bundle-size limits; the source does not define a universal
  performance target or a dedicated RUV error code for every budget failure.
- requireEnv: validates required environment variables during build.
- fonts: fetches and self-hosts configured Google Fonts stylesheets when the build environment
  permits the request; verify generated assets and provider availability in CI.

### Production verification

```bash
npm run check
npm run analyze
npm run build
```

For a production review, record the selected plugins, configuration inputs, generated artifacts,
environment variables, target adapter, and workload measurements. The repository does not claim
universal latency, throughput, bundle-size, ROI, partner commitments, or deployment promotion.

## Source of truth

- packages/ruvyxa/src/plugins.ts
- packages/@ruvyxa/core/src/plugin.ts
- crates/ruvyxa_middleware/src
- packages/@ruvyxa/core/package.json

---

## Retained detailed draft

The original long-form plugin guide is retained for context and audit history only. It is
non-normative; revalidate every API, option, payload, metric, and provider statement against the
current source before using it in a production project.

### English plugin draft — historical draft (non-normative)

> **Archive warning:** The material below is retained for history only. It is not the current plugin
> contract; examples may be stale or unsupported and must not be copied as working code. The
> source-backed contract above is authoritative.

# Plugins

Ruvyxa plugins tap into framework lifecycle — module resolution, server startup, build completion.
Enforce security headers, generate sitemaps, add middleware, or build custom integrations. The
plugin system gives hooks for every phase.

---

## What You Will Learn

- Plugin architecture and socket registry
- All 16 built-in plugins with complete TypeScript types, options, and examples
- `definePlugin()` API: concise declarations and `register()` escape hatch
- Plugin hooks: `build.onResolve`, `build.onLoad`, `build.onTransform`, `build.onStart`,
  `build.onComplete`, `http.onRequest`, `http.onResponse`, `http.route`, `dev.onFileChange`,
  `diagnostics.report`, `native.claim`
- Plugin execution timing and ordering rules
- Response middleware limits (32 MiB default, 256 MiB max)
- Publishing a plugin to npm
- Custom plugin: SEO validator, virtual modules, analytics middleware
- Troubleshooting every plugin failure

---

## Plugin Architecture

```
┌──────────────────────────────────────────────────┐
│                Ruvyxa Framework                   │
│                                                   │
│  ┌──────────┐   ┌──────────┐   ┌──────────────┐  │
│  │  Build   │   │  Server   │   │   Config     │  │
│  │ Pipeline │   │  Runtime  │   │   Loader     │  │
│  └────┬─────┘   └────┬─────┘   └────┬──────────┘  │
│       │              │              │             │
│       ▼              ▼              ▼             │
│  ┌────────────────────────────────────────────┐   │
│  │           Socket Registry                   │   │
│  │                                              │   │
│  │  build.onResolve    build.onLoad            │   │
│  │  build.onTransform  build.onStart           │   │
│  │  build.onComplete   http.onRequest          │   │
│  │  http.onResponse    http.route              │   │
│  │  dev.onFileChange   diagnostics.report      │   │
│  │  native.claim                                │   │
│  └──────────┬──────────────────────────────────┘   │
│             │                                       │
│             ▼                                       │
│  ┌────────────────────────────────────────────┐   │
│  │           Plugin Chain                      │   │
│  │                                              │   │
│  │  [redirects] → [headers] → [security] → ... │   │
│  └────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────┘
```

### Socket Registry Timing

Each hook fires at a specific lifecycle point. Order within the same hook follows plugin declaration
order in `config()`.

| Hook                 | Phase               | Timing                                    |
| -------------------- | ------------------- | ----------------------------------------- |
| `build.onStart`      | Build start         | Before any module resolution              |
| `build.onResolve`    | Module resolution   | On each import specifier                  |
| `build.onLoad`       | Module loading      | After resolve, before transform           |
| `build.onTransform`  | Code transformation | After load, before compilation            |
| `build.onComplete`   | Build end           | After all modules compiled                |
| `http.onRequest`     | Request middleware  | Before route handler                      |
| `http.onResponse`    | Response middleware | After route handler                       |
| `http.route`         | Custom HTTP route   | At config registration, served at runtime |
| `dev.onFileChange`   | Dev file watch      | On file change detection                  |
| `diagnostics.report` | Config load         | At plugin registration                    |
| `native.claim`       | Native capability   | At plugin registration                    |

---

## Built-in Plugins (16 total)

All available from `ruvyxa/plugins`:

```typescript
import {
  redirects,
  headers,
  observability,
  securityHeaders,
  cacheRules,
  pwa,
  sitemap,
  robots,
  feed,
  searchIndex,
  contentEngine,
  openApi,
  alias,
  bundleBudget,
  requireEnv,
  fonts,
} from 'ruvyxa/plugins'
```

### Quick Reference Table

| Plugin            | Purpose                            | Hook(s)                              | Config time         | Runtime           |
| ----------------- | ---------------------------------- | ------------------------------------ | ------------------- | ----------------- |
| `redirects`       | Declarative URL redirects          | `http.onRequest`                     | Rules validated     | Before render     |
| `headers`         | Custom HTTP response headers       | `http.onResponse`                    | Rules validated     | After render      |
| `observability`   | Request IDs, trace context, timing | `http.onRequest` + `http.onResponse` | Options validated   | Every request     |
| `securityHeaders` | CSP, HSTS, permissions policy      | `http.onResponse`                    | Headers compiled    | Every response    |
| `cacheRules`      | Cache-Control per route            | `http.onResponse`                    | Rules validated     | Every response    |
| `pwa`             | Service worker + manifest          | `build.onComplete`                   | Options validated   | Post-build        |
| `sitemap`         | Generate sitemap.xml               | `build.onComplete`                   | Build manifest read | Post-build        |
| `robots`          | Generate robots.txt                | `build.onComplete`                   | Build manifest read | Post-build        |
| `feed`            | RSS/Atom feed                      | `build.onComplete`                   | Feed config         | Post-build        |
| `searchIndex`     | Search index JSON                  | `build.onComplete`                   | Collection config   | Post-build        |
| `contentEngine`   | Content query API                  | `http.route`                         | Routes registered   | On request        |
| `openApi`         | OpenAPI spec                       | `build.onComplete`                   | API routes scanned  | Post-build        |
| `alias`           | Module path aliases                | `build.onResolve`                    | Aliases registered  | Module resolution |
| `bundleBudget`    | Enforce bundle size limits         | `build.onComplete`                   | Budget defined      | Post-build        |
| `requireEnv`      | Validate required env vars         | `build.onStart`                      | Vars listed         | Pre-build         |
| `fonts`           | Optimize and bundle web fonts      | `build.onStart` + `build.onComplete` | Families configured | Pre/post build    |

---

### `redirects`

Declarative URL redirects served before rendering. Next.js-style `source`/`destination` patterns.

```typescript
// Type
interface RedirectRule {
  /** Exact path or prefix pattern ending in *. */
  source: string
  /** Destination path or absolute URL. Trailing * appends matched remainder. */
  destination: string
  /** HTTP 308 (permanent) instead of 307 (temporary). @default false */
  permanent?: boolean
}

function redirects(rules: RedirectRule[]): RuvyxaPlugin
```

```typescript
import { redirects } from 'ruvyxa/plugins'

export default config({
  plugins: [
    redirects([
      { source: '/old-page', destination: '/new-page', permanent: true }, // 308
      { source: '/blog/:slug', destination: '/posts/:slug', permanent: true }, // param
      { source: '/old-blog/*', destination: '/blog/*', permanent: false }, // wildcard 307
      { source: '/legacy/*', destination: 'https://new-site.com/legacy/*' }, // external
    ]),
  ],
})
```

**Under the hood**: Sources are registered as `http.onRequest` match patterns. Non-matching requests
skip the plugin entirely. The handler iterates rules, calls `matchSource()` for prefix/glob
matching, builds `Location` header via `redirectLocation()`, and returns 307/308 `Response`.

**Validation errors**:

- `source` must be `"*"` or start with `"/"`
- `destination` must not be an open redirect (absolute http(s) or same-origin path)
- `destination` must not use `//` prefix or backslashes

**Edge case**: When `destination` ends with `*` and `source` also ends with `*`, the wildcard
remainder is appended. If interpolation produces a different origin, the rule is silently skipped.

---

### `headers`

Declarative response headers per route.

```typescript
// Type
interface HeaderRule {
  /** Exact path or prefix pattern ending in *. Omit to match every route. */
  source?: string
  /** Header name-value pairs. */
  headers: Record<string, string>
}

function headers(rules: HeaderRule[]): RuvyxaPlugin
```

```typescript
import { headers } from 'ruvyxa/plugins'

export default config({
  plugins: [
    headers([
      { source: '/api/*', headers: { 'cache-control': 'no-store' } },
      { source: '/assets/*', headers: { 'cache-control': 'public, max-age=31536000, immutable' } },
      { headers: { 'x-framework': 'ruvyxa' } }, // all routes
    ]),
  ],
})
```

**Under the hood**: Registers `http.onResponse`. When `source` is provided, the handler checks
`matchSource()` against `request.url` pathname. Creates a new `Headers` instance, sets matched
rules, returns cloned `Response` with updated headers.

**Edge case**: All rules omitting `source` = scoped mode. A rule with a single `source` and others
without still works — rules without source match every path through `matchSource` returning non-null
for `matchSource(undefined, pathname)` fallback.

---

### `observability`

Request IDs, W3C trace context, timing headers, and structured logs.

```typescript
// Type
interface ObservabilityEntry {
  requestId: string
  traceparent: string
  method: string
  pathname: string
  status: number
  durationMs: number
}

interface ObservabilityOptions {
  /** Route patterns to observe. Omit = all. */
  routes?: string[]
  /** Request ID header name. @default "x-request-id" */
  requestIdHeader?: string
  /** Emit W3C traceparent. @default true */
  traceContext?: boolean
  /** Add Server-Timing metric. @default true */
  serverTiming?: boolean
  /** Log JSON records. @default true */
  log?: boolean
  /** Custom structured log sink. Defaults to console.info(JSON.stringify(...)) */
  logger?: (entry: ObservabilityEntry) => void
}

function observability(options?: ObservabilityOptions): RuvyxaPlugin
```

```typescript
import { observability } from 'ruvyxa/plugins'

export default config({
  plugins: [
    observability({
      requestIdHeader: 'x-request-id',
      traceContext: true,
      serverTiming: true,
      log: true,
      logger: (entry) => console.log(JSON.stringify(entry)),
    }),
  ],
})
```

**Under the hood**: Two hooks. `http.onRequest`: injects `x-request-id` (UUID if missing/invalid),
injects `traceparent` (W3C format `00-{32hex}-{16hex}-01`), stores timestamp as
`x-ruvyxa-observability-start` header. `http.onResponse`: reads the timestamp, calculates duration,
sets response headers, emits structured JSON log.

**Validation**:

- `requestIdHeader` must not be `traceparent` or `x-ruvyxa-observability-start`
- Incoming request IDs validated against `^[A-Za-z0-9._:-]{1,128}$`
- `logger` must be a function

**Edge case**: Telemetry failures are silently caught — logging never turns a valid response into an
HTTP error.

---

### `securityHeaders`

Route-scoped security policy headers.

```typescript
// Type
type ContentSecurityPolicy = Record<string, string | string[]>

interface SecurityHeadersOptions {
  /** Route patterns. Omit = all. */
  routes?: string[]
  /** CSP string or directive map. */
  contentSecurityPolicy?: string | ContentSecurityPolicy
  /** HSTS policy. @default "max-age=31536000; includeSubDomains" */
  strictTransportSecurity?: string
  permissionsPolicy?: string
  referrerPolicy?: string
  crossOriginOpenerPolicy?: string
  crossOriginEmbedderPolicy?: string
  crossOriginResourcePolicy?: string
  frameOptions?: string
  /** Additional response headers. */
  headers?: Record<string, string>
}

function securityHeaders(options?: SecurityHeadersOptions): RuvyxaPlugin
```

```typescript
import { securityHeaders } from 'ruvyxa/plugins'

export default config({
  plugins: [
    securityHeaders({
      contentSecurityPolicy: {
        'default-src': "'self'",
        'script-src': ["'self'", "'unsafe-inline'"],
        'style-src': ["'self'", "'unsafe-inline'"],
        'img-src': ["'self'", 'https:'],
      },
      strictTransportSecurity: 'max-age=63072000; includeSubDomains; preload',
      permissionsPolicy: 'camera=(), microphone=()',
      referrerPolicy: 'strict-origin-when-cross-origin',
      frameOptions: 'DENY',
      crossOriginOpenerPolicy: 'same-origin',
      crossOriginEmbedderPolicy: 'require-corp',
      crossOriginResourcePolicy: 'same-origin',
    }),
  ],
})
```

**Under the hood**: Compiles security headers into a `Headers` object at config time. On
`http.onResponse`, merges into response (preserving existing headers). CSP accepts either a raw
string or a directive map — directives are validated: names must match `/^[a-z][a-z0-9-]*$/`, source
values must not contain `;`, `\r`, `\n`.

**Defaults**: HSTS defaults to `max-age=31536000; includeSubDomains`. All other security headers are
omitted unless explicitly set.

**Edge case**: Passing an empty CSP map throws `TypeError`. Passing an empty string for CSP throws
`TypeError`.

---

### `cacheRules`

Browser/CDN cache policy per route.

```typescript
// Type
interface CacheRule {
  /** Route pattern. Omit = all. */
  source?: string
  /** Browser Cache-Control value. */
  browser?: string
  /** CDN-Cache-Control value. */
  cdn?: string
  /** Values appended to Vary header. */
  vary?: string[]
}

function cacheRules(rules: CacheRule[]): RuvyxaPlugin
```

```typescript
import { cacheRules } from 'ruvyxa/plugins'

export default config({
  plugins: [
    cacheRules([
      { source: '/', cdn: 'public, max-age=300' },
      { source: '/api/*', browser: 'no-store' },
      {
        source: '/assets/*',
        browser: 'public, max-age=31536000, immutable',
        cdn: 'public, max-age=31536000',
      },
      { browser: 'no-cache', vary: ['Accept-Encoding'] }, // all routes
    ]),
  ],
})
```

**Under the hood**: Registers `http.onResponse`. For each matching rule, sets `Cache-Control` and/or
`CDN-Cache-Control` on response. Merges `Vary` values (appends, does not replace). Returns cloned
response.

**Validation**: At least one of `browser`, `cdn`, or `vary` must be set per rule. Empty rules arrays
throw.

**Edge case**: If no rule matches, response headers are unmodified. Only the first matching rule's
headers are applied.

---

### `pwa`

Generate `manifest.json`, service worker, and offline page.

```typescript
// Type
interface PwaIcon {
  src: string
  sizes: string
  type?: string
  purpose?: 'any' | 'maskable' | 'monochrome'
}

interface PwaOptions {
  name: string
  shortName?: string
  description?: string
  themeColor?: string
  backgroundColor?: string
  icons: PwaIcon[]
  /** Service worker script path. Defaults to generated. */
  serviceWorker?: string
  /** Offline page fallback path. */
  offlinePage?: string
  /** Start URL. @default '/' */
  startUrl?: string
  /** Display mode. @default 'standalone' */
  display?: 'fullscreen' | 'standalone' | 'minimal-ui' | 'browser'
  /** Orientation. @default 'portrait' */
  orientation?: 'any' | 'natural' | 'portrait' | 'landscape'
  /** Scope for PWA. @default '/' */
  scope?: string
}

function pwa(options: PwaOptions): RuvyxaPlugin
```

```typescript
import { pwa } from 'ruvyxa/plugins'

export default config({
  plugins: [
    pwa({
      name: 'My App',
      shortName: 'MyApp',
      description: 'Amazing PWA',
      themeColor: '#0070f3',
      backgroundColor: '#ffffff',
      display: 'standalone',
      orientation: 'portrait',
      icons: [
        { src: '/icon-192x192.png', sizes: '192x192', type: 'image/png' },
        { src: '/icon-512x512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
      ],
      startUrl: '/',
      scope: '/',
    }),
  ],
})
```

**Under the hood**: At `build.onComplete`, writes `manifest.json` to build output and optionally
generates a minimal service worker script that caches app shell.

**Validation**: `name` and `icons` are required. Icon `src` must reference existing public assets.
Theme color must be valid hex/rgb.

---

### `sitemap`

Generate `sitemap.xml` from route manifest.

```typescript
// Type
interface SitemapOptions {
  /** Site base URL. Required — used as XML namespace. */
  siteUrl?: string
  /** Paths to exclude. */
  exclude?: string[]
  /** Additional static paths. */
  additionalPaths?: string[]
  /** Default metadata for all entries. */
  defaults?: {
    lastModified?: string | Date
    changeFrequency?: 'always' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly' | 'never'
    priority?: number
  }
  /** Also emit robots.txt. @default true */
  robots?: boolean
}

function sitemap(options?: SitemapOptions): RuvyxaPlugin
```

```typescript
import { sitemap } from 'ruvyxa/plugins'

export default config({
  plugins: [
    sitemap({
      siteUrl: 'https://example.com',
      exclude: ['/api/*', '/admin/*'],
      additionalPaths: ['/promo/summer-2026'],
      defaults: {
        changeFrequency: 'weekly',
        priority: 0.7,
      },
    }),
  ],
})
```

**Under the hood**: Reads build manifest routes at `build.onComplete`. Filters out API routes and
excluded paths. Generates XML with `<urlset>` namespace. Validates `siteUrl` is an absolute URL.

**Validation**: `siteUrl` must be a valid absolute URL (http or https). Exclude patterns support
glob-style `*` suffix.

**Edge case**: If no routes match (all excluded), the plugin skips generation with a warning. If
`siteUrl` is not set, falls back to `site.url` from config.

---

### `robots`

Generate `robots.txt`.

```typescript
// Type
interface RobotsPolicy {
  userAgent?: string | string[]
  allow?: string | string[]
  disallow?: string | string[]
  crawlDelay?: number
}

interface RobotsOptions {
  policies?: RobotsPolicy[]
  sitemap?: string | string[]
  host?: string
}

function robots(options?: RobotsOptions): RuvyxaPlugin
```

```typescript
import { robots } from 'ruvyxa/plugins'

export default config({
  plugins: [
    robots({
      policies: [
        { userAgent: '*', allow: '/', disallow: ['/api/', '/admin/'] },
        { userAgent: 'Googlebot', allow: '/' },
      ],
      sitemap: 'https://example.com/sitemap.xml',
      host: 'https://example.com',
    }),
  ],
})
```

**Under the hood**: At `build.onComplete`, writes `robots.txt`. Defaults to allowing all crawlers.
Multiple `User-Agent` groups are separated by blank lines per RFC 9309.

---

### `feed`

Generate RSS or Atom feed from content routes.

```typescript
// Type
interface FeedItem {
  id: string
  title: string
  content?: string
  summary?: string
  url: string
  date: Date | string
  author?: string
  categories?: string[]
  image?: string
}

interface FeedOptions {
  title: string
  description?: string
  /** Feed type. @default 'rss' */
  kind?: 'rss' | 'atom'
  items: FeedItem[]
  /** Site base URL. */
  siteUrl?: string
  /** Feed output path. @default '/feed.xml' */
  path?: string
  language?: string
}

function feed(options: FeedOptions): RuvyxaPlugin
```

```typescript
import { feed } from 'ruvyxa/plugins'

export default config({
  plugins: [
    feed({
      title: 'My Blog',
      description: 'Latest posts',
      kind: 'rss',
      siteUrl: 'https://example.com',
      path: '/feed.xml',
      items: [
        {
          id: 'post-1',
          title: 'Hello World',
          url: '/blog/hello-world',
          date: '2026-07-28',
          summary: 'First post',
          author: 'Author',
        },
      ],
    }),
  ],
})
```

**Under the hood**: At `build.onComplete`, generates XML or Atom feed. RSS uses
`<rss version="2.0">`. Atom uses `<feed xmlns="http://www.w3.org/2005/Atom">`.

---

### `searchIndex`

Build search index JSON for client-side search.

```typescript
// Type
interface SearchIndexCollection {
  name: string
  /** Route pattern to include. */
  route: string
  /** Fields to extract from route metadata. */
  fields: string[]
}

interface SearchIndexOptions {
  collections: SearchIndexCollection[]
  /** Output path. @default '/search-index.json' */
  outputPath?: string
}

function searchIndex(options: SearchIndexOptions): RuvyxaPlugin
```

```typescript
import { searchIndex } from 'ruvyxa/plugins'

export default config({
  plugins: [
    searchIndex({
      collections: [
        { name: 'posts', route: '/blog/*', fields: ['title', 'description', 'content'] },
        { name: 'docs', route: '/docs/*', fields: ['title', 'description'] },
      ],
      outputPath: '/search-index.json',
    }),
  ],
})
```

---

### `contentEngine`

Content query API for MDX and markdown pages.

```typescript
// Type
interface ContentEngineCollection {
  name: string
  route: string
}

interface ContentEngineOptions {
  collections: ContentEngineCollection[]
}

function contentEngine(options: ContentEngineOptions): RuvyxaPlugin
```

```typescript
import { contentEngine } from 'ruvyxa/plugins'

export default config({
  plugins: [
    contentEngine({
      collections: [
        { name: 'blog', route: '/blog/*' },
        { name: 'docs', route: '/docs/*' },
      ],
    }),
  ],
})
```

Registers API routes at build time for content queries. The generated API allows querying route
metadata, filtering, and sorting.

---

### `openApi`

Generate OpenAPI specification from API routes.

```typescript
// Type
interface OpenApiOptions {
  title: string
  version: string
  description?: string
  /** Paths filter. Omit = all. */
  paths?: string[]
  /** Output path. @default '/openapi.json' */
  outputPath?: string
}

function openApi(options: OpenApiOptions): RuvyxaPlugin
```

```typescript
import { openApi } from 'ruvyxa/plugins'

export default config({
  plugins: [
    openApi({
      title: 'My API',
      version: '1.0.0',
      description: 'API documentation',
    }),
  ],
})
```

**Under the hood**: Scans route manifest for API routes at `build.onComplete`. Generates OpenAPI 3.0
JSON specification.

---

### `alias`

Module path aliases — resolve imports to different paths.

```typescript
// Type
interface AliasEntry {
  /** Find pattern (string or regex). */
  find: string | RegExp
  /** Replacement path. */
  replacement: string
}

interface AliasOptions {
  entries: AliasEntry[]
}

function alias(options: AliasOptions): RuvyxaPlugin
```

```typescript
import { alias } from 'ruvyxa/plugins'

export default config({
  plugins: [
    alias({
      entries: [
        { find: '@', replacement: './src' },
        { find: '@components', replacement: './src/components' },
        { find: /^@utils\/(.*)/, replacement: './src/utils/$1' },
      ],
    }),
  ],
})
```

**Under the hood**: Registers `build.onResolve`. For each import, tests `find` pattern against the
specifier. On match, replaces with `replacement` and returns resolved path. Falls through to default
resolver if no alias matches.

**Edge cases**:

- Regex `find`: captured groups can be used in `replacement` via `$1`, `$2`, etc.
- String `find`: simple substring replacement
- Alias ordering: first match wins
- Return `null` to let next plugin or default resolver handle

---

### `bundleBudget`

Enforce bundle size limits after build.

```typescript
// Type
interface BundleBudgetOptions {
  /** Max bytes per individual bundle. @default 250000 */
  maxSize?: number
  /** Max total bytes across all bundles. @default 1000000 */
  maxTotalSize?: number
  /** Fail build instead of warning. @default false */
  strict?: boolean
  /** Exclude route patterns. */
  exclude?: string[]
}

function bundleBudget(options?: BundleBudgetOptions): RuvyxaPlugin
```

```typescript
import { bundleBudget } from 'ruvyxa/plugins'

export default config({
  plugins: [
    bundleBudget({
      maxSize: 250_000, // 250 KB per bundle
      maxTotalSize: 1_000_000, // 1 MB total
      strict: true,
    }),
  ],
})
```

**Under the hood**: At `build.onComplete`, reads chunk sizes from manifest. Sums all client bundle
sizes. Compares against configured limits. If a limit is exceeded, it throws a build error. The
current implementation does not assign a dedicated RUV diagnostic code to bundle-budget failures.

---

### `requireEnv`

Validate required environment variables before build.

```typescript
// Type
interface RequireEnvOptions {
  /** Required environment variable names. */
  variables: string[]
  /** Fail build if missing. @default true */
  strict?: boolean
  /** Also check RUVYXA_PUBLIC_ prefix variant. @default false */
  allowPublic?: boolean
}

function requireEnv(options: RequireEnvOptions): RuvyxaPlugin
```

```typescript
import { requireEnv } from 'ruvyxa/plugins'

export default config({
  plugins: [
    requireEnv({
      variables: ['DATABASE_URL', 'AUTH_SECRET', 'STRIPE_SECRET_KEY'],
      strict: true,
    }),
  ],
})
```

**Under the hood**: At `build.onStart`, checks `process.env` for each variable. If `allowPublic`,
also checks the `RUVYXA_PUBLIC_` prefixed variant. Missing variables produce a detailed error
listing all absent vars.

**Edge case**: With `strict: false`, missing vars produce warnings but build continues.

---

### `fonts`

Optimize and bundle web fonts.

```typescript
// Type
interface FontFamily {
  name: string
  weights?: number[]
  styles?: ('normal' | 'italic')[]
  /** Subset characters (e.g. 'latin', 'latin-ext'). */
  subsets?: string[]
  /** Swap display strategy. @default 'swap' */
  display?: 'auto' | 'block' | 'swap' | 'fallback' | 'optional'
}

interface FontsOptions {
  families: FontFamily[]
  /** Output directory for font files. @default 'assets/fonts' */
  outputDir?: string
}

function fonts(options: FontsOptions): RuvyxaPlugin
```

```typescript
import { fonts } from 'ruvyxa/plugins'

export default config({
  plugins: [
    fonts({
      families: [
        { name: 'Inter', weights: [400, 600, 700], subsets: ['latin'] },
        { name: 'JetBrains Mono', weights: [400], styles: ['normal', 'italic'] },
      ],
      outputDir: 'assets/fonts',
    }),
  ],
})
```

**Under the hood**: At `build.onStart`, downloads or resolves font files. At `build.onComplete`,
writes subsetted `.woff2` files to build output. Generates CSS `@font-face` declarations with
`font-display: swap`.

**Validation**: Font family names must match available fonts. Weights must be valid (100-900).
Invalid fonts produce a warning.

---

## New Hooks System (v0.5+)

Ruvyxa has 2 hook systems — Build Hooks (for build time) and HTTP Hooks (for runtime)

### build.onStart

Called when build starts — use for initialization and checking conditions:

```typescript
type BuildOnStart = (ctx: {
  root: string // Project root directory
  outDir: string // Output directory (.ruvyxa)
  config: Record<string, any> // Full config object
  env: Record<string, string> // Environment variables snapshot
}) => void | Promise<void>
```

**Example**: Check Node.js version

```ts
build: {
  onStart({ root, config }) {
    const nodeMajor = parseInt(process.versions.node, 10);
    if (nodeMajor < 20) {
      throw new Error('Node.js 20+ required');
    }
    console.log(`Building ${config.site?.name || 'app'} from ${root}`);
  },
}
```

### build.onResolve

Modify module resolution — edit import paths:

```typescript
type BuildOnResolve = (ctx: {
  source: string // Import source: './Button', 'react', etc.
  importer: string // File ที่ import
  resolve: (id: string) => string | null // Default resolver
}) => string | null | undefined // Return resolved path หรือ null
```

**Example**: Replace moment with dayjs

```ts
build: {
  onResolve({ source, resolve }) {
    if (source === 'moment') {
      return resolve('dayjs');
    }
    // Remember to return undefined if no modifications are needed
  },
}
```

### build.onTransform

Transform source code before bundling:

```typescript
type BuildOnTransform = (ctx: {
  code: string // Source code
  id: string // Module path
  resolve: (id: string) => string // Resolver
}) => { code: string; map?: string } | undefined | void
```

**Example**: Remove console.log in production

```ts
build: {
  onTransform({ code, id }) {
    if (process.env.NODE_ENV === 'production' && id.endsWith('.tsx')) {
      return {
        code: code.replace(/console\.\w+\([^)]*\)/g, '/* removed */'),
      };
    }
  },
}
```

### build.onComplete

Called when build completes — use for reporting, cleanup:

```typescript
type BuildOnComplete = (ctx: {
  duration: number // Build duration (ms)
  routes: number // Number of routes
  assets: { count: number; size: number } // Asset stats
  manifest: RouteManifest // Route manifest
  diagnostics: Diagnostic[] // Warnings/errors
}) => void | Promise<void>
```

**Example**: Notify Slack when build completes

```ts
build: {
  async onComplete({ duration, routes, diagnostics }) {
    const errors = diagnostics.filter(d => d.severity === 'error');
    if (errors.length > 0) {
      await fetch(process.env.SLACK_WEBHOOK!, {
        method: 'POST',
        body: JSON.stringify({
          text: `Build failed: ${errors.length} errors in ${duration}ms`,
        }),
      });
    }
  },
}
```

### http.onRequest

Modify request before reaching route handler:

```typescript
type HttpOnRequest = (ctx: {
  request: {
    method: string
    url: string
    headers: Record<string, string>
    body?: any
  }
  params: Record<string, string> // Route params
}) => {
  request?: Partial<PluginHttpRequest> // Modify request
  response?: PluginHttpResponse // Or respond immediately
} | void
```

**Example**: Rate limiting

```ts
http: {
  onRequest({ request }) {
    const ip = request.headers['x-forwarded-for'] || 'unknown';
    const key = `rate:${ip}`;
    // Check rate limit...
    if (isRateLimited(key)) {
      return {
        response: { status: 429, body: 'Too Many Requests' },
      };
    }
  },
}
```

### http.onResponse

Modify response before sending back to client:

```typescript
type HttpOnResponse = (ctx: { request: PluginHttpRequest; response: PluginHttpResponse }) => {
  response?: Partial<PluginHttpResponse>
} | void
```

**Example**: Add custom headers

```ts
http: {
  onResponse({ response }) {
    return {
      response: {
        headers: {
          ...response.headers,
          'X-Powered-By': 'Ruvyxa',
          'X-Response-Time': `${Date.now() - start}ms`,
        },
      },
    };
  },
}
```

### Hooks Legacy (v0.4)

Legacy hooks are still supported — `definePlugin`:

```typescript
type LegacyHookResolveId = (ctx: {
  source: string
  importer: string
  resolve: (id: string) => string | null
}) => string | null | undefined

type LegacyHookTransform = (ctx: {
  code: string
  id: string
  resolve: (id: string) => string
}) => { code: string; map?: string } | undefined

type LegacyHookBuildStart = (ctx: {
  root: string
  outDir: string
  config: Record<string, any>
}) => void

type LegacyHookBuildEnd = (ctx: { manifest: RouteManifest; diagnostics: Diagnostic[] }) => void

type LegacyHookServerStart = (ctx: { config: ServerConfig }) => void
type LegacyHookServerEnd = (ctx: {}) => void

type LegacyHookMiddleware = (ctx: {
  request: PluginHttpRequest
  response: PluginHttpResponse
  next: () => Promise<void>
}) => Promise<PluginHttpRequestResult | void>
```

---

## Creating Custom Plugins

### `definePlugin()` API

```typescript
import { definePlugin } from '@ruvyxa/core/plugin'
import type { RuvyxaPlugin, RuvyxaPluginDefinition } from '@ruvyxa/core/plugin'

function definePlugin(definition: RuvyxaPluginDefinition): RuvyxaPlugin
```

### Plugin Definition

```typescript
interface RuvyxaPluginDefinition {
  name: string
  /** Global response headers added to every response. */
  headers?: HeadersInit
  /** Head elements injected into every rendered document's <head>. */
  head?: PluginHeadEntry | readonly PluginHeadEntry[]
  /** Build hooks (resolve, load, transform, start, complete). */
  build?: PluginBuildDefinition
  /** HTTP hooks (onRequest, onResponse, routes). */
  http?: PluginHttpDefinition
  /** Dev hooks (onFileChange). */
  dev?: PluginDevDefinition
  /** Diagnostics reported at config validation time. */
  diagnostics?: PluginDiagnostic | readonly PluginDiagnostic[]
  /** Native capabilities (realtime). */
  native?: PluginNativeDefinition
  /** Advanced escape hatch — register any combination of hooks programmatically. */
  register?(api: PluginRegistrationApi): void | Promise<void>
}
```

### PluginRegistrationApi

```typescript
interface PluginRegistrationApi {
  readonly http: PluginHttpSocket
  readonly build: PluginBuildSocket
  readonly dev: PluginDevSocket
  readonly diagnostics: PluginDiagnosticsSocket
  readonly native: PluginNativeSocket
}
```

### Build Hooks

```typescript
interface PluginBuildSocket {
  /** Before build starts. */
  onStart(hook: PluginBuildStartHook): void
  /** Intercept module resolution. Return resolved path or null. */
  onResolve(hook: PluginBuildResolveHandler): void
  /** Load module content. Return source string or null. */
  onLoad(hook: PluginBuildLoadHandler): void
  /** Transform source code before compilation. Return modified code or null. */
  onTransform(hook: PluginBuildTransformHandler): void
  /** After build completes. */
  onComplete(hook: PluginBuildCompleteHook): void
}
```

#### `build.onStart`

```typescript
interface PluginBuildStartContext {
  readonly root: string
  readonly outDir: string
}

type PluginBuildStartHook = (context: PluginBuildStartContext) => void | Promise<void>
```

Called before any module resolution begins. Use for validation, file cleanup, or initializing state.

#### `build.onResolve`

```typescript
interface PluginBuildResolveContext extends PluginTransformContext {
  readonly id: string // Import specifier
  readonly importer?: string // Parent module path
}

type PluginBuildResolveHandler = (
  context: PluginBuildResolveContext,
) => string | null | void | Promise<string | null | void>
```

Return absolute file path to redirect resolution. Return `null` or `undefined` to fall through.

#### `build.onLoad`

```typescript
interface PluginBuildLoadContext extends PluginTransformContext {
  readonly id: string
}

type PluginBuildLoadHandler = (
  context: PluginBuildLoadContext,
) => string | TransformResult | null | void | Promise<string | TransformResult | null | void>
```

Return source code as string or `TransformResult` (`{ code: string, map?: unknown }`). Return null
to let default loader handle.

#### `build.onTransform`

```typescript
interface PluginBuildTransformContext extends PluginTransformContext {
  readonly code: string
  readonly id: string
}

type PluginBuildTransformHandler = (
  context: PluginBuildTransformContext,
) => string | TransformResult | null | void | Promise<string | TransformResult | null | void>
```

Transform source code. Return modified code or `TransformResult` with source map. Return null to
leave unchanged.

#### `build.onComplete`

```typescript
interface PluginBuildContext {
  root: string
  outDir: string
  manifest: Readonly<Record<string, unknown>>
}

type PluginBuildCompleteHook = (context: PluginBuildContext) => void | Promise<void>
```

Called after all modules compiled. Use for post-processing, generating files, or reporting.

### HTTP Hooks

```typescript
interface PluginHttpSocket {
  /** Intercept/modify requests before route handler. */
  onRequest(registration: PluginHttpRequestRegistration | PluginHttpRequestHandler): void
  /** Intercept/modify responses after route handler. */
  onResponse(registration: PluginHttpResponseRegistration | PluginHttpResponseHandler): void
  /** Register a custom HTTP route handled by the plugin. */
  route(registration: PluginHttpRouteRegistration): void
}
```

#### `http.onRequest`

```typescript
interface PluginHttpRequestContext extends PluginHttpContext {
  readonly request: Request
  /** Continue to next hook, optionally with replacement request. */
  next(request?: Request): void
}

type PluginHttpRequestHandler = (
  context: PluginHttpRequestContext,
) => Request | Response | void | Promise<Request | Response | void>

interface PluginHttpRequestRegistration {
  /** Route patterns to match. Omit = all. */
  match?: readonly PluginRoutePattern[]
  handler: PluginHttpRequestHandler
}
```

Return `Request` to replace the incoming request and continue normal processing. Return `Response`
to short-circuit (e.g., redirect). Return `void`/`undefined` to continue unchanged. Call
`next(request?)` to pass to the next hook.

#### `http.onResponse`

```typescript
interface PluginHttpResponseContext extends PluginHttpContext {
  readonly request: Request
  readonly response: Response
  next(response?: Response): void
}

type PluginHttpResponseHandler = (
  context: PluginHttpResponseContext,
) => Response | void | Promise<Response | void>

interface PluginHttpResponseRegistration {
  match?: readonly PluginRoutePattern[]
  handler: PluginHttpResponseHandler
}
```

Return `Response` to replace the outgoing response. Return `void`/`undefined` to leave unchanged.

#### `http.route`

```typescript
interface PluginHttpRouteContext extends PluginHttpContext {
  readonly request: Request
}

interface PluginHttpRouteRegistration {
  path: string // Exact application path
  method?: string | readonly string[] // Omit for any method
  handler(context: PluginHttpRouteContext): Response | Promise<Response>
}
```

Register a new HTTP route entirely handled by the plugin. Useful for custom API endpoints.

### Dev Hooks

```typescript
interface PluginDevSocket {
  onFileChange(registration: PluginDevFileChangeRegistration | PluginDevFileChangeHandler): void
}

interface PluginDevFileChangeContext {
  readonly root: string
  readonly paths: readonly string[]
}

type PluginDevFileChangeHandler = (context: PluginDevFileChangeContext) => void | Promise<void>

interface PluginDevFileChangeRegistration {
  match?: readonly string[] // Path patterns relative to app root
  handler: PluginDevFileChangeHandler
}
```

### Diagnostics

```typescript
interface PluginDiagnosticsSocket {
  report(diagnostic: PluginDiagnostic): void
}

interface PluginDiagnostic {
  level: 'info' | 'warning' | 'error'
  code: string
  message: string
}
```

### Native Capabilities

```typescript
interface PluginNativeSocket {
  claim(capability: 'realtime@1', options?: RealtimePluginOptions): void
}

interface RealtimePluginOptions {
  path?: string // @default "/__ruvyxa/realtime"
  heartbeatMs?: number // @default 25000
  capacity?: number // @default 256
}
```

### Head Entries

```typescript
interface PluginHeadEntry {
  tag: 'link' | 'meta' | 'noscript' | 'script' | 'style'
  attrs?: Record<string, string | number | boolean>
  /** Text content for script/style/noscript. Written verbatim. */
  children?: string
}
```

Declared once at config load and injected into every document's `<head>`. Only legal `<head>`
elements accepted. Attribute values are HTML-escaped.

---

## Plugin Example: Complete Custom Plugin

### Virtual Module + Analytics Middleware

```typescript
// plugins/analytics.ts
import { definePlugin } from '@ruvyxa/core/plugin'
import type { RuvyxaPlugin } from '@ruvyxa/core/plugin'

interface AnalyticsOptions {
  trackingId?: string
  endpoint?: string
}

export default function analyticsPlugin(options: AnalyticsOptions = {}): RuvyxaPlugin {
  const trackingId = options.trackingId ?? 'UA-000000-1'

  return definePlugin({
    name: 'analytics',

    // Inject analytics script into every page <head>
    head: {
      tag: 'script',
      attrs: {
        src: 'https://cdn.example.com/analytics.js',
        async: true,
        'data-tracking-id': trackingId,
      },
    },

    // Add global response header
    headers: { 'x-powered-by': 'ruvyxa-analytics' },

    // Build hooks
    build: {
      onStart() {
        console.log('[analytics] Build starting...')
      },
      onResolve({ id }) {
        if (id === 'virtual:analytics-config') {
          return '\0virtual:analytics-config'
        }
      },
      onLoad({ id }) {
        if (id === '\0virtual:analytics-config') {
          return { code: `export const trackingId = ${JSON.stringify(trackingId)}`, map: null }
        }
      },
      onComplete({ manifest }) {
        console.log(`[analytics] Build complete. ${manifest.routes?.length ?? 0} routes.`)
      },
    },

    // HTTP request middleware — log every request timing
    http: {
      onRequest({ next }) {
        const start = Date.now()
        const originalNext = next
        next = (req?: Request) => {
          const duration = Date.now() - start
          console.log(
            `[analytics] ${req?.method ?? 'GET'} ${new URL(req?.url ?? '').pathname} — ${duration}ms`,
          )
          return originalNext(req)
        }
      },
    },
  })
}
```

Usage in `ruvyxa.config.ts`:

```typescript
import analyticsPlugin from './plugins/analytics'

export default config({
  plugins: [
    analyticsPlugin({
      trackingId: 'UA-123456-1',
    }),
  ],
})
```

### SEO Validator Plugin

```typescript
// plugins/seo-validator.ts
import { definePlugin } from '@ruvyxa/core/plugin'
import type { RuvyxaPlugin, PluginBuildContext } from '@ruvyxa/core/plugin'

interface SeoValidatorOptions {
  requiredFields?: string[]
  strict?: boolean
}

export default function seoValidatorPlugin(options: SeoValidatorOptions = {}): RuvyxaPlugin {
  const required = options.requiredFields ?? ['title', 'description']
  const strict = options.strict ?? false

  return definePlugin({
    name: 'seo-validator',
    build: {
      onComplete({ manifest }: PluginBuildContext) {
        const routes = (manifest as any).routes ?? []
        const errors: string[] = []

        for (const route of routes) {
          if (route.type === 'api' || route.type === 'static') continue
          const meta = route.meta ?? {}
          for (const field of required) {
            if (!meta[field]) {
              errors.push(`${route.path}: missing meta.${field}`)
            }
          }
        }

        if (errors.length > 0) {
          const msg = `[seo-validator] ${errors.length} route(s) missing SEO metadata:\n  ${errors.join('\n  ')}`
          if (strict) throw new Error(msg)
          else console.warn(msg)
        }
      },
    },
  })
}
```

---

## Plugin Examples - 5 Examples

### 1. Request Logger

```ts
import { definePlugin } from 'ruvyxa/plugins'

interface LoggerOptions {
  format?: 'json' | 'text'
  logHeaders?: boolean
}

export default definePlugin<LoggerOptions>('request-logger', (options = {}) => ({
  name: 'request-logger',

  http: {
    onRequest(ctx) {
      ctx.request.headers['x-request-start'] = Date.now().toString()
    },

    onResponse(ctx) {
      const start = parseInt(ctx.request.headers['x-request-start'] || '0')
      const duration = Date.now() - start
      const { method, url } = ctx.request
      const { status } = ctx.response

      if (options.format === 'json') {
        console.log(JSON.stringify({ method, url, status, duration }))
      } else {
        console.log(`${method} ${url} → ${status} (${duration}ms)`)
      }

      if (options.logHeaders) {
        console.log('Request headers:', ctx.request.headers)
      }
    },
  },
}))
```

### 2. Env Validator

```ts
import { definePlugin } from 'ruvyxa/plugins'

interface EnvOptions {
  required: string[]
  prefix?: string
  strict?: boolean
}

export default definePlugin<EnvOptions>('env-validator', (options) => ({
  name: 'env-validator',

  build: {
    onStart({ config }) {
      const missing = options.required.filter((key) => !process.env[key])

      if (missing.length > 0) {
        if (options.strict !== false) {
          throw new Error(`Missing required env vars: ${missing.join(', ')}`)
        } else {
          console.warn(`Warning: Missing env vars: ${missing.join(', ')}`)
        }
      }

      if (options.prefix) {
        const vars = Object.keys(process.env).filter((k) => k.startsWith(options.prefix!))
        console.log(`Found ${vars.length} ${options.prefix}* variables`)
      }
    },
  },
}))
```

### 3. Cache Buster

```ts
import { definePlugin } from 'ruvyxa/plugins'

export default definePlugin('cache-buster', () => ({
  name: 'cache-buster',

  build: {
    onComplete({ duration, routes }) {
      // สร้าง cache buster file สำหรับ CDN
      const fs = require('fs')
      const hash = Date.now().toString(36)
      fs.writeFileSync(
        '.ruvyxa/assets/cache-version.json',
        JSON.stringify({
          version: hash,
          builtAt: new Date().toISOString(),
          routes,
          duration,
        }),
      )
      console.log(`Cache version: ${hash}`)
    },
  },

  http: {
    onResponse(ctx) {
      // Add cache buster query param
      if (ctx.response.headers['content-type']?.includes('text/html')) {
        // NOOP — cache buster สำหรับ assets
      }
    },
  },
}))
```

### 4. Response Time Header

```ts
import { definePlugin } from 'ruvyxa/plugins'

export default definePlugin('response-time', () => ({
  name: 'response-time',

  http: {
    onRequest(ctx) {
      ctx.request.headers['x-start-time'] = String(performance.now())
    },

    onResponse(ctx) {
      const start = parseFloat(ctx.request.headers['x-start-time'] || '0')
      const elapsed = performance.now() - start
      return {
        response: {
          headers: {
            ...ctx.response.headers,
            'X-Response-Time': `${Math.round(elapsed)}ms`,
          },
        },
      }
    },
  },
}))
```

### 5. S3 Image Upload

```ts
import { definePlugin } from 'ruvyxa/plugins'

interface S3Options {
  bucket: string
  region?: string
  pathPrefix?: string
}

export default definePlugin<S3Options>('s3-upload', (options) => ({
  name: 's3-upload',

  build: {
    async onComplete({ assets }) {
      const { S3Client, PutObjectCommand } = require('@aws-sdk/client-s3')
      const client = new S3Client({ region: options.region || 'ap-southeast-1' })

      for (const asset of assets.images) {
        const key = `${options.pathPrefix || 'assets'}/${asset.name}`
        await client.send(
          new PutObjectCommand({
            Bucket: options.bucket,
            Key: key,
            Body: require('fs').readFileSync(asset.path),
            ContentType: asset.mimeType,
          }),
        )
        console.log(`Uploaded: ${key}`)
      }
    },
  },
}))
```

---

## Plugin Ordering

Plugins run in declaration order. When multiple plugins hook the same event:

```typescript
plugins: [
  redirects([{ source: '/old', destination: '/new' }]), // 1st: http.onRequest
  securityHeaders({ contentSecurityPolicy: "default-src 'self'" }), // 2nd: http.onResponse
  headers([{ source: '/api/*', headers: { 'x-foo': 'bar' } }]), // 3rd: http.onResponse
]
```

**General rule**: Build-time plugins before server-time plugins. Redirects and security first, then
headers and cache rules, then build-output plugins (sitemap, robots, pwa).

### Ordering Within Same Hook

For `http.onRequest` and `http.onResponse`, handlers registered by earlier plugins run first. Each
handler can call `next()` to pass control. If a handler returns a `Response` without calling
`next()`, subsequent handlers are skipped.

For `build.onResolve`, the first plugin that returns a non-null string wins. Subsequent `onResolve`
handlers are not called for that specifier.

---

## Plugin Execution Limits

### Response Body Limit

TypeScript response middleware has a configurable buffer limit:

```typescript
// ruvyxa.config.ts
export default config({
  security: {
    pluginLimit: 33_554_432, // 32 MiB default, max 268_435_456 (256 MiB)
  },
})
```

If response middleware produces a buffered body exceeding this limit, the framework returns a 500
error. Binary streams and large file downloads should skip response middleware.

### Timeout

Plugin hooks have a configurable timeout via `middleware.timeoutMs`:

```typescript
export default config({
  middleware: {
    timeoutMs: 30_000, // 30 seconds default, max 300_000 (5 minutes)
  },
})
```

If exceeded: `RUV1700 TypeScript plugin hook timed out`. The worker is replaced. Timed-out hooks are
not retried.

### Worker Count

```typescript
export default config({
  middleware: {
    workers: 1, // 1-8, default 1
  },
})
```

Workers do not share module-level plugin state. Keep at 1 unless plugins are stateless and
throughput-bottlenecked.

---

## Publishing a Plugin

### Package Structure

```
ruvyxa-plugin-my-plugin/
├── src/
│   └── index.ts           # Plugin entry
├── package.json
├── tsconfig.json
├── README.md
└── LICENSE
```

### package.json

```json
{
  "name": "ruvyxa-plugin-seo-validator",
  "version": "1.0.0",
  "description": "Validate SEO metadata across Ruvyxa routes",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "files": ["dist"],
  "scripts": {
    "build": "tsc",
    "prepublishOnly": "npm run build"
  },
  "peerDependencies": {
    "ruvyxa": "^2.0.0"
  },
  "keywords": ["ruvyxa", "plugin", "seo"],
  "license": "MIT"
}
```

### Naming Convention

```
ruvyxa-plugin-<name>         # Unscoped
@scope/ruvyxa-plugin-<name>  # Scoped
```

### Workspace Protocol Restriction

Published plugins must NOT use `workspace:` protocol in `peerDependencies`. Only local monorepo
development uses `workspace:`. Replace with a version range before publishing.

### Build and Publish

```bash
# Build TypeScript
npm run build

# Test locally
# Add to ruvyxa.config.ts: import myPlugin from 'ruvyxa-plugin-seo-validator'

# Publish
npm publish

# Or scoped
npm publish --access public
```

---

## Troubleshooting

| Problem                     | Cause                                 | Fix                                                             |
| --------------------------- | ------------------------------------- | --------------------------------------------------------------- |
| Plugin not running          | Name mismatch                         | Check `name` matches in `definePlugin`                          |
| RUV1008: Private env        | Private server data in client context | Move the read to server code or use an approved public variable |
| RUV1700: Hook timeout       | Plugin hook exceeded its timeout      | Reduce work or adjust the validated middleware timeout          |
| Plugin hook error           | Unhandled exception                   | Fix plugin code; check the reported error                       |
| Plugin is not loaded        | Plugin is absent from config          | Install it and add it to `plugins`                              |
| Invalid plugin options      | The plugin rejects its input          | Check that plugin's documented options                          |
| RUV2102: Invalid plugin     | Not a valid Plugin object             | Return from `definePlugin` or `Plugin` type                     |
| Virtual module not found    | `resolveId` returns null              | Verify specifier match and importer                             |
| Plugin order wrong          | Hooks overwriting                     | Reorder plugins or use `next()`                                 |
| Head elements not rendered  | Invalid tag/attr                      | Check `PluginHeadEntry` constraints                             |
| `requireEnv` fails          | Missing env vars during dev           | Set variables in `.env.local`                                   |
| `bundleBudget` blocks build | Bundle too large                      | Optimize or increase budget                                     |
| `alias` not matching        | Regex/string mismatch                 | Test pattern in isolation                                       |
| `redirects` not firing      | Source pattern mismatch               | Check wildcard `*` syntax                                       |
| `pwa` icons missing         | Icon path not found                   | Ensure files exist in `public/`                                 |
| Response middleware 500     | Body exceeds `pluginLimit`            | Increase limit or skip for large responses                      |

---

## Error Codes (RUV1600-1699, RUV2000-2102)

| Code         | Title                               | Source           | Fix                                     |
| ------------ | ----------------------------------- | ---------------- | --------------------------------------- |
| RUV1007-1010 | Plugin boundary violation           | Graph/bundler    | Fix the reported server/client boundary |
| RUV1700      | Plugin hook timeout or host failure | Plugin runtime   | Inspect the hook error and timeout      |
| RUV1701      | Plugin bridge/protocol error        | Plugin runtime   | Inspect the plugin response             |
| RUV2102      | Invalid plugin definition           | `definePlugin()` | Return a valid plugin object            |
| RUV2103      | Font self-hosting warning           | `fonts()` plugin | Check the font URL/network              |

---

## Plugin Boundaries and a Minimal Safe Plugin

## Try It Yourself

1. เปิด `ruvyxa.config.ts` และเพิ่ม plugin redirects — redirect `/old` → `/new`
2. ทดลองเพิ่ม `securityHeaders` plugin — ดู headers ใน DevTools
3. เพิ่ม `fonts` plugin ด้วย Google Fonts Inter + Noto Sans Thai
4. สร้าง custom plugin ด้วย `ruvyxa plugin create`
5. ใช้ `build.onTransform` เพื่อแทนที่ text ใน production build
6. ใช้ `http.onRequest` เพื่อเพิ่ม rate limiting
7. ใช้ `head` field เพื่อเพิ่ม Google Analytics script
8. ลงทะเบียน plugin ใน config แล้วรัน dev — ดู logs
9. Test plugin ordering — สลับลำดับใน array
10. Publish plugin ของคุณไปยัง npm — `npm publish --access public`
11. ทดลอง `middleware.pluginLimit` — เพิ่มเป็น 64MiB
12. ใช้ definePlugin API สำหรับ plugin ใหม่ทั้งหมด

---

## Summary

- 16 built-in plugins — redirects, headers, observability, securityHeaders, cacheRules, pwa,
  sitemap, robots, feed, searchIndex, contentEngine, openApi, alias, bundleBudget, requireEnv, fonts
- TypeScript plugin system — 2 API sets: definePlugin (new) + hooks (legacy)
- Build hooks: onStart, onResolve, onTransform, onComplete
- HTTP hooks: onRequest, onResponse
- Socket registry — bi-directional IPC ระหว่าง Rust ↔ JS Worker
- Plugin ordering — array order for onRequest, reverse for onResponse
- Response limits — 32 MiB default, 256 MiB max
- Plugin naming: `ruvyxa-plugin-<name>` บน npm
- Head contribution — SEO, analytics, custom tags
- 5 Example plugin จริง — request logger, env validator, cache buster, response time, S3 upload
- Troubleshooting — 14 ปัญหาพร้อม error codes และวิธีแก้

---

The public plugin constructor is `definePlugin()` from `@ruvyxa/core/plugin` (also re-exported by
`ruvyxa/plugin`). A plugin needs a non-empty name and at least one behavior: a registration
callback, HTTP behavior, build hooks, development file-change behavior, diagnostics, a native
capability, or head entries. The constructor validates this before the plugin is registered.

```ts
import { definePlugin, withResponseHeader } from '@ruvyxa/core/plugin'

export default definePlugin({
  name: 'example:request-id',
  http: {
    match: '/api/*',
    onResponse({ response }) {
      return withResponseHeader(response, 'x-example-plugin', 'enabled')
    },
  },
})
```

Register the returned plugin value in `ruvyxa.config.ts`. The route pattern `*` matches all paths, a
trailing `*` is a prefix pattern, and any other pattern is exact. Keep request/response work
bounded: plugin runtime communication is a system boundary, so an expensive or broad hook affects
every matched request.

### Select a Capability, Not a Marketing Name

The first-party `ruvyxa/plugins` module currently exports: `redirects`, `headers`, `observability`,
`securityHeaders`, `cacheRules`, `pwa`, `sitemap`, `robots`, `feed`, `searchIndex`, `contentEngine`,
`openApi`, `alias`, `bundleBudget`, `requireEnv`, and `fonts`. Read the exported options type for
the capability being used; a plugin name in a tutorial is not a substitute for its current contract.

### Scaffold, Then Prove the Smallest Behavior

```bash
ruvyxa plugin create @acme/request-id --dir packages/request-id
ruvyxa analyze --format human
ruvyxa build
```

The CLI scaffolder creates a publishable package structure. It does not register the package in an
application config or publish it to npm. Add one behavior, test the matching request/build path, and
only then add wider hooks or a native capability.

---

## Next Steps

- **[11-configuration.md](./11-configuration.md)** — Plugin config in detail
- **[12-cli-commands.md](./12-cli-commands.md)** — `ruvyxa plugin create` command
- **[15-official-packages.md](./15-official-packages.md)** — Official packages with plugins
- **[16-error-handling.md](./16-error-handling.md)** — Plugin error codes
