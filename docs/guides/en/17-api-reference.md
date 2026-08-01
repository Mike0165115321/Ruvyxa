# 17 — API Reference

**Ruvyxa 1.0**

Reference for the public exports documented from `@ruvyxa/react`, `@ruvyxa/core`, and related
packages. Confirm any API not listed here against the package export map and generated type files.

---

## What You Will Learn

- `@ruvyxa/react` — Image, Picture, Seo, Link, Answer, ErrorBoundary
- `@ruvyxa/react` hooks — useRouteContext, useRouter, useRuvyxaLoader, and more
- `@ruvyxa/react` functions — notFound, compilePattern, route matchers
- `@ruvyxa/core/server` — loader, action, cache, redirect, response helpers
- `@ruvyxa/core/config` — config helper and all re-exported types
- `@ruvyxa/core` utilities — constants, globs, cache control, build helpers
- `@ruvyxa/core/plugin-harness` — test harness for plugins
- Plugin type system — all socket, definition, and registration types
- Full RuvyxaConfig, SiteConfig, Adapter, BuildContext interfaces

---

## 1. @ruvyxa/react

### Image

```typescript
function Image(props: ImageProps): ReactElement
```

Optimized image with automatic WebP conversion and responsive srcset.

**ImageProps** — union of two forms:

```typescript
type ImageProps =
  | (ImageBaseProps & { fill?: false; width: number; height: number })
  | (ImageBaseProps & { fill: true; width?: number; height?: number })

interface ImageBaseProps extends Omit<ImgHTMLAttributes<...>, 'alt' | 'src' | 'width' | 'height'> {
  src: string
  alt: string
  unoptimized?: boolean
  priority?: boolean
  fill?: boolean
  loader?: ImageLoader
  quality?: number
  loading?: 'eager' | 'lazy'
  fetchPriority?: 'auto' | 'high' | 'low'
}

type ImageLoader = (props: ImageLoaderProps) => string

interface ImageLoaderProps {
  src: string
  width?: number
  quality?: number
}
```

| Prop            | Type        | Default  | Description                                                 |
| --------------- | ----------- | -------- | ----------------------------------------------------------- |
| `src`           | string      | —        | Public image URL. Local PNG/JPEG rewritten to WebP at build |
| `alt`           | string      | —        | Required accessible label; empty string for decorative      |
| `unoptimized`   | boolean     | `false`  | Keep local source URLs unchanged                            |
| `priority`      | boolean     | `false`  | Eager-load with `fetchPriority="high"`                      |
| `fill`          | boolean     | `false`  | Fill positioned parent (`position: absolute; inset: 0`)     |
| `loader`        | ImageLoader | —        | Custom CDN URL generator                                    |
| `quality`       | number      | —        | Passed to custom loader                                     |
| `loading`       | string      | `'lazy'` | Override native loading                                     |
| `fetchPriority` | string      | `'auto'` | Override native fetch priority                              |

```tsx
// Width/height — intrinsic dimensions prevent CLS
<Image src="/hero.png" alt="Hero" width={1200} height={600} priority />

// Fill mode — parent must be `position: relative | absolute | fixed`
<div style={{ position: 'relative', width: '100%', height: '400px' }}>
  <Image src="/bg.png" alt="Background" fill />
</div>

// Custom CDN loader
<Image
  src="/photo.jpg"
  alt="Photo"
  width={800}
  height={600}
  loader={({ src, width }) => `https://cdn.example.com/${src}?w=${width}`}
/>
```

### Picture

```typescript
function Picture(props: PictureProps): ReactElement
```

Art-direction wrapper. Adds `<source>` children before a fallback `<Image>`.

```typescript
type PictureProps = ImageProps & {
  sources: readonly PictureSource[]
}

interface PictureSource extends Omit<SourceHTMLAttributes<HTMLSourceElement>, 'srcSet'> {
  srcSet: string
  unoptimized?: boolean
}
```

```tsx
<Picture
  src="/photo.jpg"
  alt="Art-directed photo"
  width={1200}
  height={600}
  sources={[
    { media: '(min-width: 1024px)', srcSet: '/photo-desktop.jpg' },
    { media: '(min-width: 640px)', srcSet: '/photo-tablet.jpg' },
  ]}
/>
```

### Seo

```typescript
function Seo(props: SeoProps): ReactElement
```

Render `<title>`, `<meta>`, `<link>`, and JSON-LD for SEO and social previews. React 19 hoists these
into `<head>`.

```typescript
interface SeoProps {
  title: string
  description?: string
  canonical?: string
  image?: string
  imageAlt?: string
  siteName?: string
  type?: 'website' | 'article' | 'profile'
  locale?: string
  noindex?: boolean
  card?: 'summary' | 'summary_large_image'
  twitterCard?: 'summary' | 'summary_large_image'
  article?: SeoArticle
  breadcrumbs?: readonly SeoBreadcrumb[]
  jsonLd?: Record<string, unknown> | Array<Record<string, unknown>>
}

interface SeoArticle {
  type?: 'Article' | 'BlogPosting' | 'NewsArticle'
  publishedAt?: string
  updatedAt?: string
  authors?: readonly SeoAuthor[]
  section?: string
  tags?: readonly string[]
}

interface SeoAuthor {
  name: string
  url?: string
  type?: 'Person' | 'Organization'
}

interface SeoBreadcrumb {
  name: string
  url: string
}
```

```tsx
<Seo
  title="My Page"
  description="A great page"
  canonical="https://example.com/page"
  image="https://example.com/og.png"
  siteName="My Site"
  type="website"
  locale="en_US"
  noindex={false}
  card="summary_large_image"
  breadcrumbs={[
    { name: 'Home', url: '/' },
    { name: 'Blog', url: '/blog' },
  ]}
  jsonLd={{ '@type': 'WebPage', name: 'My Page' }}
/>
```

### Link

```typescript
function Link(props: LinkProps): ReactElement
```

Client-side navigation without document load. Renders a real `<a href>` — crawlable,
middle-clickable, works before hydration.

```typescript
interface LinkProps extends Omit<AnchorHTMLAttributes<HTMLAnchorElement>, 'href'> {
  href: string
  replace?: boolean
  scroll?: boolean
  prefetch?: LinkPrefetch
  children?: ReactNode
  ref?: Ref<HTMLAnchorElement>
}

type LinkPrefetch = boolean | 'hover' | 'viewport' | 'none'
```

| Prop       | Type         | Default   | Description                           |
| ---------- | ------------ | --------- | ------------------------------------- |
| `href`     | string       | —         | Destination URL, relative or absolute |
| `replace`  | boolean      | `false`   | Replace history entry instead of push |
| `scroll`   | boolean      | `true`    | Scroll to top after navigation        |
| `prefetch` | LinkPrefetch | `'hover'` | When to warm the target bundle        |

```tsx
<Link href="/">Home</Link>
<Link href="/blog/hello" prefetch="viewport">Preload on scroll into view</Link>
<Link href="/settings" prefetch="none">No prefetch</Link>
<Link href="/replace-me" replace>Replace current entry</Link>
```

### Answer

```typescript
function Answer(props: AnswerProps): ReactElement
```

Question/Answer block with Schema.org microdata and cited sources.

```typescript
type AnswerProps = AnswerBaseProps &
  | { answer: ReactNode; children?: never }
  | { answer?: never; children: ReactNode }

interface AnswerBaseProps {
  question: string
  id?: string
  sources?: readonly AnswerSource[]
  sourcesLabel?: ReactNode
  className?: string
}

interface AnswerSource {
  name: string
  url: string
}
```

```tsx
<Answer
  question="What is Ruvyxa?"
  answer="A React framework with a native Rust heart."
  sources={[
    { name: "Ruvyxa Docs", url: "https://ruvyxa.dev" },
  ]}
/>

// FAQ pattern
<Answer question="Is it fast?">
  <p>Yes — Rust-powered bundler and dev server.</p>
</Answer>
```

### RuvyxaErrorBoundary

```typescript
class RuvyxaErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState>
```

React error boundary for production-grade error recovery.

```typescript
interface ErrorBoundaryProps {
  children: ReactNode
  fallback: (props: ErrorFallbackProps) => ReactNode
  onError?: (error: Error, info: ErrorInfo) => void
}

interface ErrorFallbackProps {
  error: Error
  resetError: () => void
}
```

```tsx
<RuvyxaErrorBoundary
  fallback={({ error, resetError }) => (
    <div role="alert">
      <p>Error: {error.message}</p>
      <button onClick={resetError}>Retry</button>
    </div>
  )}
  onError={(error, info) => reportError(error, info.componentStack)}
>
  <App />
</RuvyxaErrorBoundary>
```

### Hooks

#### useRouteContext

```typescript
function useRouteContext(): RouteContextValue

interface RouteContextValue {
  pathname: string
  params: RouteParams
  route: string
}
```

```tsx
const { pathname, params, route } = useRouteContext()
```

#### usePathname

```typescript
function usePathname(): string
```

```tsx
const pathname = usePathname() // e.g. "/blog/hello"
```

#### useParams

```typescript
function useParams(): RouteParams
// RouteParams = Record<string, string | string[] | undefined>
```

```tsx
const params = useParams() // { slug: "hello" }
```

#### useSearchParams

```typescript
function useSearchParams(): URLSearchParams
```

Returns empty set during SSR — hydration mismatch avoided.

```tsx
const sp = useSearchParams()
const q = sp.get('q')
```

#### useSelectedRoute

```typescript
function useSelectedRoute(): string
```

```tsx
const pattern = useSelectedRoute() // "/blog/[slug]"
```

#### useRouter

```typescript
function useRouter(): RuvyxaRouter

interface RuvyxaRouter {
  push(href: string, options?: NavigateOptions): Promise<void>
  replace(href: string, options?: NavigateOptions): Promise<void>
  back(): void
  forward(): void
  refresh(): void
  prefetch(href: string): void
  readonly pending: boolean
}

interface NavigateOptions {
  replace?: boolean
  scroll?: boolean
}
```

```tsx
const router = useRouter()
router.push('/dashboard', { scroll: true })
router.prefetch('/settings') // warm bundle ahead of navigation
```

#### useRuvyxaLoader

```typescript
function useRuvyxaLoader<T>(
  loader: () => Promise<T>,
  options?: UseLoaderOptions,
): UseLoaderResult<T>

interface UseLoaderOptions {
  enabled?: boolean
  deps?: unknown[]
}

interface UseLoaderResult<T> {
  data: T | undefined
  loading: boolean
  error: Error | undefined
  refetch: () => void
}
```

```tsx
const { data, loading, error, refetch } = useRuvyxaLoader(
  () => fetch('/api/users').then((r) => r.json()),
  { deps: [] },
)
```

### Functions

```typescript
function hydrate(options?: HydrationOptions): void

interface HydrationOptions {
  root?: Element | Document
  onError?: HydrationErrorHandler
}

function reportHydrationError(
  error: unknown,
  context?: {
    componentStack?: string
    digest?: string
  },
): void
```

```typescript
hydrate({ onError: (err, ctx) => myLogger.error(err, ctx) })
```

```typescript
function notFound(): never
```

Throws a tagged error caught by the nearest `not-found.tsx`.

```typescript
const post = await getPost(params.slug)
if (!post) notFound()
```

```typescript
function isNotFoundError(value: unknown): value is NotFoundError
```

Type guard for errors thrown by `notFound()`.

```typescript
function compilePattern(routePath: string): CompiledPattern
```

Build regex and param list from a route pattern.

```typescript
function routeSpecificity(routePath: string): number[]
```

Per-segment specificity: static `0` < dynamic `1` < catch-all `2` < optional catch-all `3`.

```typescript
function compareSpecificity(left: number[], right: number[]): number
```

Order two specificity vectors — shorter sorts first.

```typescript
function normalizeMatchPath(pathname: string): string
```

Collapse request path (remove duplicate slashes, trailing slash).

```typescript
function createRouteMatcher<Route extends RouteManifestEntry>(
  routes: readonly Route[],
): (pathname: string) => RouteMatch<Route> | null
```

Compile route table and return a matcher over it.

### Constants

```typescript
const DEFAULT_DEVICE_WIDTHS: readonly [640, 750, 828, 1080, 1200, 1920, 2048, 3840]

const NOT_FOUND_PROPERTY: '__ruvyxaNotFound'

const RouteContext: Context<RouteContextValue | null>
```

### SEO Types

```typescript
export type { SeoAuthor, SeoArticle, SeoBreadcrumb, SeoProps } from './seo.js'
export type { Meta, MetaAlternate, MetaContext, MetaExport, MetaFactory } from './meta.js'
```

---

## 2. @ruvyxa/core/server

### loader

```typescript
function loader<TResult>(handler: LoaderHandler<TResult>): Loader<TResult>

type LoaderHandler<TResult> = (ctx: LoaderContext) => TResult | Promise<TResult>

interface LoaderContext {
  params: Record<string, string>
  request: Request
  cache: typeof cache
}

interface Loader<TResult> {
  (ctx?: Partial<LoaderContext>): Promise<TResult>
  ruvyxa: { kind: 'loader' }
}
```

```typescript
export const getProduct = loader(async ({ params, cache }) => {
  return cache(`product:${params.id}`)
    .ttl('5m')
    .get(async () => {
      return db.product.findUnique({ where: { id: params.id } })
    })
})
```

### action

```typescript
const action: ActionBuilder

interface ActionBuilder<TInput = unknown> {
  input<TNextInput>(schema: Schema<TNextInput>): ActionBuilder<TNextInput>
  realtime(channels?: string | readonly string[]): ActionBuilder<TInput>
  handler<TResult>(
    handler: (ctx: ActionContext<TInput>) => TResult | Promise<TResult>,
  ): ServerAction<TInput, TResult>
}

interface ActionContext<TInput> {
  input: TInput
  request: Request
  user?: unknown
  invalidate(key: string): void
}

interface ServerAction<TInput, TResult> {
  (input: TInput, ctx?: Partial<ActionContext<TInput>>): Promise<TResult>
  ruvyxa: { kind: 'action'; realtime?: ActionRealtimeOptions }
}

interface Schema<TInput> {
  parse(value: unknown): TInput
}
```

```typescript
'use server'
export const submitContact = action
  .input({
    parse: (v) => {
      const { name, email } = v as any
      if (!name || !email) throw new Error('Validation failed')
      return { name, email }
    },
  })
  .handler(async ({ input, invalidate }) => {
    await db.contact.create({ name: input.name, email: input.email })
    invalidate('contacts')
    return { success: true }
  })
```

### cache

```typescript
function cache(key: string): CacheBuilder

interface CacheBuilder {
  ttl(value: string): CacheBuilder
  swr(value: string): CacheBuilder
  get<T>(producer: () => T | Promise<T>): Promise<T>
}
```

TTL format: `"30s"`, `"5m"`, `"1h"`, `"1d"`. Default: 60s. LRU eviction at 1024 entries.

```typescript
const data = await cache('users:list')
  .ttl('5m')
  .swr('1m')
  .get(async () => db.users.findMany())
```

### invalidateCache

```typescript
function invalidateCache(keyOrPrefix?: string): void
```

Clear exact key, prefix (`keyOrPrefix:`), or entire cache (when omitted).

### cacheStats

```typescript
function cacheStats(): { size: number; maxEntries: number }
```

### redirect

```typescript
function redirect(location: string, status?: number): Response
```

Returns a 3xx Response. Defaults to 302.

### notFound

```typescript
function notFound(message?: string): Response
```

Returns a 404 Response.

### json

```typescript
function json(data: unknown, init?: ResponseInit): Response
```

### Types

```typescript
export type {
  LoaderContext,
  ActionContext,
  LoaderHandler,
  Loader,
  Schema,
  ActionBuilder,
  ServerAction,
  CacheBuilder,
  CacheEntry,
}
```

---

## 3. @ruvyxa/core/config

```typescript
function config<TConfig extends RuvyxaConfig>(config: TConfig): TConfig
```

Identity helper for type inference in `ruvyxa.config.ts`.

```typescript
import { config } from 'ruvyxa/config'

export default config({
  server: { port: 3000 },
})
```

**Re-exported types** (19 total):

```typescript
export type {
  BuiltinMiddlewareConfig,
  CachedStaticParams,
  CorsConfig,
  GetStaticParams,
  ImageConfig,
  MiddlewareConfig,
  PageProps,
  RateLimitConfig,
  RenderConfig,
  RenderStrategy,
  RouteParamValue,
  RouteParams,
  RuvyxaConfig,
  SiteConfig,
  StaticParamsContext,
  StaticParamSegment,
  StaticParamsCacheDuration,
  StaticParamsResult,
  StaticParamsValues,
  TransformResult,
}
```

---

## 4. @ruvyxa/core Utilities

### Constants

```typescript
const STATIC_ASSET_EXTENSIONS: readonly [
  'apng',
  'avif',
  'bmp',
  'css',
  'eot',
  'gif',
  'ico',
  'jpeg',
  'jpg',
  'js',
  'map',
  'mjs',
  'mov',
  'mp3',
  'mp4',
  'ogg',
  'otf',
  'png',
  'svg',
  'ttf',
  'wav',
  'webm',
  'webp',
  'woff',
  'woff2',
] // 25 entries

const CLIENT_BUNDLE_PREFIX = '/__ruvyxa/client/'

const IMMUTABLE_CACHE_CONTROL = 'public, max-age=31536000, immutable'

const PUBLIC_ASSET_CACHE_CONTROL = 'public, max-age=3600, must-revalidate'

const DEFAULT_SECURITY_HEADERS: {
  'X-Content-Type-Options': 'nosniff'
  'Referrer-Policy': 'strict-origin-when-cross-origin'
  'Permissions-Policy': 'camera=(), microphone=(), geolocation=()'
  'Cross-Origin-Opener-Policy': 'same-origin'
  'Cross-Origin-Resource-Policy': 'same-origin'
  'X-Frame-Options': 'DENY'
  'X-Permitted-Cross-Domain-Policies': 'none'
}
```

### Functions

```typescript
function staticAssetPattern(): string
// PCRE regex: ^/(?!__ruvyxa/).+\.(?:apng|avif|...)$

function staticAssetGlobs(): string[]
// ['/*.apng', '/*.avif', ...]

function publicAssetGlobs(): string[]
// Like staticAssetGlobs but excludes css/js/mjs/map

function headersFileContents(): string
// _headers file content with security defaults + asset caching

function clientBuildOutput(ctx: BuildContext): {
  clientDir: string
  chunkManifest: string
}

function projectRelativeOutDir(ctx: BuildContext): string
// POSIX-normalized, root-relative

function validateBuildContext(ctx: BuildContext, adapterName: string): asserts ctx is BuildContext

function standaloneServerSource(options?: StandaloneServerOptions): string
// Returns Node HTTP server source code

function withResponseHeader(response: Response, name: string, value: string): Response
```

---

## 5. @ruvyxa/core/plugin-harness

```typescript
function createPluginHarness(
  plugins: RuvyxaPlugin | readonly RuvyxaPlugin[],
  options?: { root?: string },
): Promise<PluginHarness>
```

Unit-test a plugin without booting a server.

### PluginHarness Members

| Member              | Type                                     | Description                       |
| ------------------- | ---------------------------------------- | --------------------------------- |
| `head`              | `readonly PluginHeadEntry[]`             | Declared head elements            |
| `routes`            | `readonly PluginHttpRouteRegistration[]` | Registered routes                 |
| `diagnostics`       | `readonly HarnessDiagnostic[]`           | Diagnostics reported              |
| `nativeClaims`      | `readonly HarnessNativeClaim[]`          | Native capabilities claimed       |
| `request()`         | function                                 | Run request hooks                 |
| `respond()`         | function                                 | Run response hooks                |
| `route()`           | function                                 | Invoke matching route handler     |
| `fileChange()`      | function                                 | Deliver file-change notifications |
| `build.start()`     | function                                 | Build start hook                  |
| `build.resolve()`   | function                                 | Build resolve hook                |
| `build.load()`      | function                                 | Build load hook                   |
| `build.transform()` | function                                 | Build transform hook              |
| `build.complete()`  | function                                 | Build complete hook               |

### Types

```typescript
interface HarnessNativeClaim {
  plugin: string
  capability: PluginNativeCapability
  options: RealtimePluginOptions
}

interface HarnessDiagnostic extends PluginDiagnostic {
  plugin: string
}

interface HarnessFileChange {
  paths: readonly string[]
}

interface HarnessRequestOptions {
  path?: string
  method?: string
  headers?: HeadersInit
  body?: BodyInit | null
}

interface HarnessBuildOptions {
  root?: string
  outDir?: string
  manifest?: Record<string, unknown>
  environment?: PluginEnvironment
}
```

---

## 6. Full Type Reference

### Plugin Sockets

| Interface                 | Methods                                                                 |
| ------------------------- | ----------------------------------------------------------------------- |
| `PluginHttpSocket`        | `onRequest()`, `onResponse()`, `route()`                                |
| `PluginBuildSocket`       | `onStart()`, `onResolve()`, `onLoad()`, `onTransform()`, `onComplete()` |
| `PluginDevSocket`         | `onFileChange()`                                                        |
| `PluginDiagnosticsSocket` | `report()`                                                              |
| `PluginNativeSocket`      | `claim()`                                                               |

### Plugin Registration API

```typescript
interface PluginRegistrationApi {
  readonly http: PluginHttpSocket
  readonly build: PluginBuildSocket
  readonly dev: PluginDevSocket
  readonly diagnostics: PluginDiagnosticsSocket
  readonly native: PluginNativeSocket
}
```

### Plugin Definition & Plugin

```typescript
interface RuvyxaPluginDefinition {
  name: string
  headers?: HeadersInit
  head?: PluginHeadEntry | readonly PluginHeadEntry[]
  http?: PluginHttpDefinition
  build?: PluginBuildDefinition
  dev?: PluginDevDefinition
  diagnostics?: PluginDiagnostic | readonly PluginDiagnostic[]
  native?: PluginNativeDefinition
  register?(api: PluginRegistrationApi): void | Promise<void>
}

interface RuvyxaPlugin {
  readonly name: string
  readonly head?: readonly PluginHeadEntry[]
  register(api: PluginRegistrationApi): void | Promise<void>
}
```

### Plugin Head Entry

```typescript
interface PluginHeadEntry {
  tag: 'link' | 'meta' | 'noscript' | 'script' | 'style'
  attrs?: Record<string, string | number | boolean>
  children?: string
}
```

### Handler Signatures

```typescript
type PluginHttpRequestHandler = (
  context: PluginHttpRequestContext,
) => Request | Response | void | Promise<Request | Response | void>

type PluginHttpResponseHandler = (
  context: PluginHttpResponseContext,
) => Response | void | Promise<Response | void>

type PluginBuildResolveHandler = (
  context: PluginBuildResolveContext,
) => string | null | void | Promise<string | null | void>

type PluginBuildLoadHandler = (
  context: PluginBuildLoadContext,
) => string | TransformResult | null | void | Promise<...>

type PluginBuildTransformHandler = (
  context: PluginBuildTransformContext,
) => string | TransformResult | null | void | Promise<...>

type PluginBuildCompleteHook = (context: PluginBuildContext) => void | Promise<void>

type PluginDevFileChangeHandler = (
  context: PluginDevFileChangeContext,
) => void | Promise<void>

type PluginEnvironment = 'client' | 'server' | 'edge' | 'worker' | 'shared'
```

### Adapter Types

```typescript
interface Adapter {
  name: string
  target: 'node' | 'edge' | 'serverless' | 'static'
  supports?: Array<RenderStrategy | 'api'>
  build(ctx: BuildContext): AdapterOutput | Promise<AdapterOutput>
}

interface AdapterOutput {
  name: string
  target: Adapter['target']
  entry: string
  assetsDir: string
  clientDir?: string
  chunkManifest?: string
  platform?:
    | 'node'
    | 'vercel'
    | 'cloudflare'
    | 'netlify'
    | 'bun'
    | 'static'
    | 'railway'
    | 'render'
    | 'firebase'
    | 'aws'
  runtime?: 'node' | 'bun'
  configFiles?: string[]
  functionsDir?: string
  artifacts?: AdapterArtifact[]
}

interface AdapterArtifact {
  kind: 'file' | 'static-site' | 'function'
  path: string
  contents?: string
  handlerSource?: string
  scope?: 'build' | 'project'
  skipIfExists?: boolean
  optional?: boolean
  excludeStrategies?: string[]
}

interface BuildContext {
  root: string
  outDir: string
  chunkManifest?: string
}
```

### RuvyxaConfig (Complete)

```typescript
interface RuvyxaConfig {
  appDir?: string
  outDir?: string
  runtime?: 'node' | 'bun' | 'edge' | 'static'
  react?: boolean
  typescript?: { strict?: boolean }
  css?: { entries?: string[] }
  server?: { port?: number; host?: string }
  build?: {
    minify?: boolean
    map?: boolean
    treeShake?: boolean
    split?: 'single' | 'route' | 'manual'
    workers?: number
    jsx?: 'classic' | 'automatic'
    target?: 'es2018' | 'es2019' | 'es2020' | 'es2022' | 'esnext'
    manifest?: boolean
    warm?: boolean
    prerenderCache?: boolean
  }
  render?: RenderConfig
  debug?: { overlay?: boolean; traces?: boolean }
  image?: ImageConfig
  security?: {
    actionLimit?: number
    apiLimit?: number
    pluginLimit?: number
    actionRateLimit?: { max?: number; window?: number }
    sameOrigin?: boolean
    fetchMeta?: boolean
    trustedProxyIps?: string[]
    headers?: boolean
  }
  cache?: { routes?: boolean; css?: boolean; dir?: string }
  site?: SiteConfig
  middleware?: MiddlewareConfig
  adapter?: Adapter
  adapterOptions?: Record<string, unknown>
  plugins?: RuvyxaPlugin[]
}
```

### SiteConfig

```typescript
interface SiteConfig {
  url?: string
  sitemap?: boolean | SiteSitemapConfig
  robots?: boolean | SiteRobotsConfig
}

interface SiteSitemapConfig {
  exclude?: string[]
  additionalPaths?: string[]
  defaults?: SiteSitemapEntryDefaults
  entries?: SiteSitemapEntry[]
}

type SiteSitemapChangeFrequency =
  'always' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly' | 'never'

interface SiteRobotsConfig {
  rules?: SiteRobotsRule | SiteRobotsRule[]
  sitemap?: string | string[]
  host?: string
}
```

### Middleware, Render, Image Config

```typescript
interface MiddlewareConfig {
  builtin?: BuiltinMiddlewareConfig
  workers?: number
  timeoutMs?: number
}

interface BuiltinMiddlewareConfig {
  cors?: CorsConfig
  timing?: boolean
  log?: boolean
  rate?: RateLimitConfig
  headers?: Record<string, string>
}

interface CorsConfig {
  origins?: string[]
  methods?: string[]
  headers?: string[]
  credentials?: boolean
  maxAge?: number
}

interface RateLimitConfig {
  max: number
  window: number
  key?: string
}

interface RenderConfig {
  strategy?: RenderStrategy
  revalidate?: number
}

type RenderStrategy = 'ssr' | 'ssg' | 'isr' | 'csr' | 'ppr'

interface ImageConfig {
  optimize?: boolean
  quality?: number
  lossless?: boolean
  keepOriginal?: boolean
  variantWidths?: number[]
  workers?: number
}
```

### Page & Static Params Types

```typescript
interface PageProps<TParams extends RouteParams = RouteParams> {
  params: TParams
  requestPath: string
}

type RouteParamValue = string | string[] | undefined
type RouteParams = Record<string, RouteParamValue>

type StaticParamsCacheDuration = number | `${number}${'s' | 'm' | 'h' | 'd'}`

type StaticParamsValues<TParams extends RouteParams = RouteParams> = ReadonlyArray<
  TParams | string | number
>

interface CachedStaticParams<TParams extends RouteParams = RouteParams> {
  params: StaticParamsValues<TParams>
  cache: StaticParamsCacheDuration
}

type StaticParamsResult<TParams extends RouteParams = RouteParams> =
  StaticParamsValues<TParams> | CachedStaticParams<TParams>

type GetStaticParams<TParams extends RouteParams = RouteParams> = (
  ctx: StaticParamsContext,
) => StaticParamsResult<TParams> | Promise<StaticParamsResult<TParams>>

interface StaticParamsContext {
  routes: Array<{ path: string; id: string }>
  route: { path: string; segments: StaticParamSegment[] }
}

interface StaticParamSegment {
  name: string
  catchAll: boolean
  optional: boolean
}

interface TransformResult {
  code: string
  map?: unknown
}
```

## How to Use This Reference Safely

The public export barrels are the implementation-backed source for this chapter: `@ruvyxa/react`
exports UI/runtime helpers such as `Image`, `Picture`, `Link`, `Seo`, `Answer`, route-context hooks,
`useRuvyxaLoader`, hydration helpers, and error/not-found helpers. `@ruvyxa/core` exports `config`,
plugin helpers, the server helpers `action`, `cache`, `loader`, `json`, `notFound`, `redirect`, and
`invalidateCache`, plus the public configuration and route types.

Prefer importing from these public package paths rather than reaching into `src/` or runtime files:

```ts
import { config } from 'ruvyxa/config'
import { action, cache, loader } from '@ruvyxa/core/server'
import { Image, Link, useParams } from '@ruvyxa/react'
```

Types and runtime behavior can evolve together. When adding a new example or upgrading the package,
check the corresponding exported symbol and run the TypeScript/project check; do not treat a type
snippet in this page as proof of a private implementation contract.
