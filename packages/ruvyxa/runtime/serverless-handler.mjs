/**
 * Standalone serverless request handler for Ruvyxa.
 *
 * Provides a self-contained Request → Response function that does not depend
 * on the Rust host process or the NDJSON worker-pool protocol. Adapters
 * generate a thin platform wrapper that imports this handler.
 *
 * At build time, adapter-runner.mjs bundles route modules into the output
 * directory. This handler imports those pre-compiled modules and dispatches
 * requests using the build manifest.
 *
 * Supported rendering strategies:
 *   - SSR: full server render on every request
 *   - ISR: serve pre-rendered HTML, revalidate in background after TTL
 *   - PPR: serve pre-rendered shell, stream dynamic slots
 *   - CSR: serve static shell HTML
 *   - API: invoke method-specific handlers (GET/POST/PUT/DELETE/PATCH etc.)
 *
 * ISR/PPR behavior depends on platform capabilities passed via options.
 *
 * The only import this file is allowed to carry is the sibling `route-match.mjs`,
 * which `adapter-runner.mjs` copies into the function bundle next to this file.
 * Everything else must stay inlined: a deployed function directory resolves no
 * bare specifiers.
 */

import {
  bindPatternParams,
  canonicalRoutePath,
  compareSpecificity,
  compilePattern,
  routeSpecificity,
} from './route-match.mjs'

/**
 * @typedef {Object} RouteEntry
 * @property {string} id
 * @property {string} path
 * @property {'page'|'api'} kind
 * @property {string} file
 * @property {string[]} layoutChain
 * @property {{strategy: string, revalidate?: number, hasDynamicSlots?: boolean}} render
 */

/**
 * @typedef {Object} HandlerOptions
 * @property {RouteEntry[]} routes - Build manifest routes
 * @property {string} buildDir - Absolute path to the build output directory
 * @property {string} [basePath] - Optional base path prefix
 * @property {(routeId: string) => Promise<{render: (ctx: object) => Promise<string>}>} importPage
 *   Import a pre-compiled page module. Adapters supply this to abstract away
 *   platform-specific module resolution.
 * @property {(routeId: string) => Promise<Record<string, Function>>} importApi
 *   Import a pre-compiled API route module.
 * @property {(path: string, revalidate?: number) => string|{html: string, stale: boolean}|null} [readPrerendered]
 *   Synchronous read of a pre-rendered HTML file. ISR-capable adapters return
 *   freshness explicitly; a legacy string result is treated as stale.
 * @property {(path: string, html: string, revalidate: number) => void} [writePrerendered]
 *   Write pre-rendered HTML to ISR cache with a TTL.
 * @property {string[]} [supportedStrategies]
 *   Strategies the platform supports. Defaults to ['ssr','ssg','csr','isr','ppr','api'].
 *   Unsupported strategies produce a 501 response.
 * @property {boolean} [securityHeaders=true]
 *   Apply Ruvyxa's non-breaking security headers unless the response already
 *   defines a value for that header.
 * @property {{builtin?: object}} [middleware]
 *   Validated built-in middleware policy emitted by the Ruvyxa build. The
 *   Fetch-native implementation mirrors the Axum/Tower CORS, rate-limit,
 *   timing, logging, and custom-header behavior without Node.js polyfills.
 * @property {{locales: string[], defaultLocale: string, localeParam: string, detectLocale: boolean, cookie: string}} [i18n]
 * @property {(request: Request, input: {src: string, width: number, quality: number}) => Promise<Response>} [optimizeImage]
 */

/** Security defaults shared with the native and standalone runtimes. */
export const DEFAULT_SECURITY_HEADERS = Object.freeze({
  'x-content-type-options': 'nosniff',
  'referrer-policy': 'strict-origin-when-cross-origin',
  'permissions-policy': 'camera=(), microphone=(), geolocation=()',
  'cross-origin-opener-policy': 'same-origin',
  'cross-origin-resource-policy': 'same-origin',
  'x-frame-options': 'DENY',
  'x-permitted-cross-domain-policies': 'none',
})

/**
 * Create a serverless request handler.
 *
 * @param {HandlerOptions} options
 * @returns {(request: Request, runtimeContext?: {waitUntil?: (promise: Promise<unknown>) => void}) => Promise<Response>}
 */
export function createHandler(options) {
  const {
    routes,
    basePath = '',
    importPage,
    importApi,
    readPrerendered,
    writePrerendered,
    supportedStrategies = ['ssr', 'ssg', 'csr', 'isr', 'ppr', 'api'],
    securityHeaders = true,
    middleware,
    i18n,
    optimizeImage,
  } = options
  const pendingRevalidations = new Map()
  const fetchMiddleware = createFetchMiddleware(middleware)

  // Pre-compile route patterns for matching. Sort by specificity so a
  // static segment always wins over a dynamic one at the same position —
  // manifest order is alphabetical, where "[" sorts before letters and
  // would otherwise shadow /blog/new behind /blog/[slug], diverging from
  // the dev server's static-first router.
  const compiledRoutes = routes
    .map((route) => ({
      ...route,
      pattern: compilePattern(route.path),
      specificity: routeSpecificity(route.path),
    }))
    .sort((left, right) => compareSpecificity(left.specificity, right.specificity))

  return async function handle(request, runtimeContext = {}) {
    const response = await fetchMiddleware(request, () => dispatch(request, runtimeContext))
    return securityHeaders ? withDefaultSecurityHeaders(response) : response
  }

  async function dispatch(request, runtimeContext = {}) {
    const url = new URL(request.url)
    const rawPathname = url.pathname
    let canonicalPathname
    try {
      canonicalPathname = canonicalRequestPath(rawPathname)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      console.error(`[ruvyxa] Malformed request path ${rawPathname}:`, message)
      return new Response('Bad Request', {
        status: 400,
        headers: { 'content-type': 'text/plain; charset=utf-8' },
      })
    }
    const pathname = stripBasePath(canonicalPathname, basePath)
    // A request outside the configured base path is not ours to serve.
    // Slicing unconditionally would turn `/other/thing` into `r/thing` and let
    // it match an unrelated route.
    if (pathname === null) {
      return new Response('Not Found', { status: 404 })
    }

    // The request boundary above already decoded and normalized the path using
    // the same segment rules as the Rust development server.
    if (pathname === '/__ruvyxa/image') {
      return handleDynamicImage(request, runtimeContext.optimizeImage ?? optimizeImage)
    }

    const match = matchRoute(compiledRoutes, pathname)
    if (!match) {
      const redirect = localeRedirect(request, pathname, basePath, compiledRoutes, i18n)
      if (redirect) return Response.redirect(new URL(redirect, request.url), 307)
      return new Response('Not Found', { status: 404 })
    }

    const { route, params } = match

    // A missing static file must not be answered by a page render. The Rust
    // server resolves public files before routing, so `/logo.png` never
    // reaches the router there; in a deploy the CDN checks the filesystem
    // first and then hands the miss to this function, where a bare dynamic
    // segment such as `/[lang]` happily captures `logo.png` and returns a 200
    // HTML document. Browsers then show a broken image, and every favicon or
    // asset miss costs a function invocation. Explicitly declared routes
    // (`/sitemap.xml`, `/api/data.json`) still match — only dynamic segments
    // are refused.
    if (isStaticAssetPath(pathname) && hasDynamicSegment(route.path)) {
      return new Response('Not Found', {
        status: 404,
        headers: { 'content-type': 'text/plain; charset=utf-8' },
      })
    }

    // Check platform support for the route's strategy
    const strategy = route.kind === 'api' ? 'api' : route.render.strategy
    if (!supportedStrategies.includes(strategy)) {
      return new Response(
        `RUV2210 Platform does not support rendering strategy "${strategy}" for route ${route.path}. ` +
          `Supported: ${supportedStrategies.join(', ')}.`,
        { status: 501, headers: { 'content-type': 'text/plain; charset=utf-8' } },
      )
    }

    try {
      if (route.kind === 'api') {
        return await handleApi(route, request, params)
      }
      return await handlePage(route, request, pathname, params, runtimeContext)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      console.error(`[ruvyxa] Error handling ${pathname}:`, message)
      // Log the detail server-side only: serverless is production, and the
      // dev server likewise never exposes internal error text to clients.
      return new Response('Internal Server Error', {
        status: 500,
        headers: { 'content-type': 'text/plain; charset=utf-8' },
      })
    }
  }

  async function handleApi(route, request, params) {
    const mod = await importApi(route.id)
    const method = request.method.toUpperCase()
    const handler = mod[method]

    if (typeof handler !== 'function') {
      return new Response(`Method ${method} is not allowed`, {
        status: 405,
        headers: { 'content-type': 'text/plain; charset=utf-8' },
      })
    }

    const result = await handler({ request, params })
    return normalizeResponse(result)
  }

  async function handlePage(route, request, pathname, params, runtimeContext) {
    const strategy = route.render.strategy

    // CSR: return pre-rendered shell (no server render needed)
    if (strategy === 'csr') {
      const cached = normalizeCacheEntry(readPrerendered?.(pathname))
      if (cached) {
        return new Response(cached.html, {
          status: 200,
          headers: { 'content-type': 'text/html; charset=utf-8' },
        })
      }
      // Fallback: render the shell
      return await renderPage(route, pathname, params)
    }

    // SSG: serve pre-rendered HTML directly
    if (strategy === 'ssg') {
      const cached = normalizeCacheEntry(readPrerendered?.(pathname))
      if (cached) {
        return new Response(cached.html, {
          status: 200,
          headers: { 'content-type': 'text/html; charset=utf-8' },
        })
      }
      // Fallback to SSR if pre-rendered not available
      return await renderPage(route, pathname, params)
    }

    // ISR: serve cached HTML, revalidate in background if stale
    if (strategy === 'isr') {
      const revalidate = route.render.revalidate ?? 60
      const cached = normalizeCacheEntry(readPrerendered?.(pathname, revalidate))
      if (cached) {
        if (cached.stale) {
          const revalidation = scheduleRevalidation(route, pathname, params)
          if (revalidation) {
            if (typeof runtimeContext.waitUntil === 'function') {
              runtimeContext.waitUntil(revalidation)
            } else {
              // A serverless runtime may freeze untracked work as soon as the
              // response is returned. Waiting is slower, but never loses the
              // refresh when the platform exposes no lifetime hook.
              await revalidation
            }
          }
        }
        return new Response(cached.html, {
          status: 200,
          headers: {
            'content-type': 'text/html; charset=utf-8',
            'x-ruvyxa-isr': 'HIT',
            'cache-control': `s-maxage=${route.render.revalidate ?? 60}, stale-while-revalidate`,
          },
        })
      }
      // Cache miss: render on demand
      const rendered = await renderPage(route, pathname, params)
      // Cache the result for future requests
      if (writePrerendered && rendered.status === 200) {
        const body = await rendered.clone().text()
        writePrerendered(pathname, body, route.render.revalidate ?? 60)
      }
      return rendered
    }

    // PPR: serve pre-rendered shell, then dynamic content
    if (strategy === 'ppr') {
      // For serverless without streaming support, fall back to full SSR
      // Platform wrappers can override this with streaming if available
      return await renderPage(route, pathname, params)
    }

    // SSR (default): full server render
    return await renderPage(route, pathname, params)
  }

  async function renderPage(route, pathname, params) {
    const mod = await importPage(route.id)
    const rendered = await mod.render({ path: pathname, params: params ?? {} })
    const html = localizeHtmlDocument(rendered, route.path, pathname, params ?? {}, i18n)
    return new Response(html, {
      status: 200,
      headers: { 'content-type': 'text/html; charset=utf-8' },
    })
  }

  function scheduleRevalidation(route, pathname, params) {
    if (!writePrerendered) return null
    const pending = pendingRevalidations.get(pathname)
    if (pending) return pending
    const revalidation = Promise.resolve().then(async () => {
      try {
        const mod = await importPage(route.id)
        const rendered = await mod.render({ path: pathname, params: params ?? {} })
        const html = localizeHtmlDocument(rendered, route.path, pathname, params ?? {}, i18n)
        writePrerendered(pathname, html, route.render.revalidate ?? 60)
      } catch (error) {
        console.error(`[ruvyxa] ISR revalidation failed for ${pathname}:`, error)
      } finally {
        pendingRevalidations.delete(pathname)
      }
    })
    pendingRevalidations.set(pathname, revalidation)
    return revalidation
  }
}

async function handleDynamicImage(request, optimizer) {
  if (typeof optimizer !== 'function') return new Response('Not Found', { status: 404 })
  if (!['GET', 'HEAD'].includes(request.method)) {
    return new Response('Method Not Allowed', { status: 405, headers: { allow: 'GET, HEAD' } })
  }
  const url = new URL(request.url)
  const src = url.searchParams.get('src')
  const width = Number(url.searchParams.get('w'))
  const quality = Number(url.searchParams.get('q') ?? 82)
  if (
    typeof src !== 'string' ||
    !src.startsWith('/') ||
    src.startsWith('//') ||
    src.includes('\\') ||
    !Number.isInteger(width) ||
    width < 16 ||
    width > 8192 ||
    !Number.isInteger(quality) ||
    quality < 1 ||
    quality > 100
  ) {
    return new Response('Invalid image request', {
      status: 400,
      headers: { 'content-type': 'text/plain; charset=utf-8' },
    })
  }
  return optimizer(request, { src, width, quality })
}

function localeRedirect(request, pathname, basePath, routes, config) {
  if (
    !config ||
    config.detectLocale === false ||
    !['GET', 'HEAD'].includes(request.method) ||
    pathname.startsWith('/__ruvyxa') ||
    pathname === '/api' ||
    pathname.startsWith('/api/') ||
    isStaticAssetPath(pathname) ||
    pathLocale(pathname, config)
  )
    return null

  const preferred = preferredLocale(request.headers, config)
  for (const locale of [preferred, config.defaultLocale]) {
    const candidate = pathname === '/' ? `/${locale}` : `/${locale}${pathname}`
    const matched = matchRoute(routes, candidate)
    if (matched?.route.kind === 'page') {
      return `${basePath === '/' ? '' : basePath}${candidate}`
    }
  }
  return null
}

function preferredLocale(headers, config) {
  const locales = Array.isArray(config.locales) ? config.locales : []
  const canonical = (value) =>
    locales.find((locale) => locale.toLowerCase() === value.toLowerCase())
  const cookie = headers.get('cookie') ?? ''
  for (const part of cookie.split(';')) {
    const separator = part.indexOf('=')
    if (separator < 0 || part.slice(0, separator).trim() !== config.cookie) continue
    const locale = canonical(part.slice(separator + 1).trim())
    if (locale) return locale
  }

  const languages = (headers.get('accept-language') ?? '')
    .split(',')
    .map((entry) => {
      const [language, ...parameters] = entry.trim().split(';')
      const quality = parameters.map((part) => part.trim()).find((part) => part.startsWith('q='))
      return { language, quality: quality ? Number(quality.slice(2)) : 1 }
    })
    .filter(({ language, quality }) => language && language !== '*' && quality > 0)
    .sort((left, right) => right.quality - left.quality)
  for (const { language } of languages) {
    const exact = canonical(language)
    if (exact) return exact
    const primary = language.split('-')[0].toLowerCase()
    const matched = locales.find((locale) => locale.split('-')[0].toLowerCase() === primary)
    if (matched) return matched
  }
  return config.defaultLocale
}

function pathLocale(pathname, config) {
  const first = pathname.replace(/^\//, '').split('/')[0]
  return config.locales?.find((locale) => locale.toLowerCase() === first.toLowerCase()) ?? null
}

function localizeHtmlDocument(html, routePath, pathname, params, config) {
  if (!config || typeof html !== 'string') return html
  const marker = `[${config.localeParam}]`
  if (routePath.split('/')[1] !== marker) return html
  const locale = pathLocale(`/${String(params[config.localeParam] ?? '')}`, config)
  if (!locale) return html
  const rest = pathname.replace(/^\//, '').split('/').slice(1).join('/')
  const localizedPath = (alternate) => (rest ? `/${alternate}/${rest}` : `/${alternate}`)
  const links = [
    ...config.locales.map(
      (alternate) =>
        `<link rel="alternate" hreflang="${escapeHtmlAttribute(alternate)}" href="${escapeHtmlAttribute(localizedPath(alternate))}">`,
    ),
    `<link rel="alternate" hreflang="x-default" href="${escapeHtmlAttribute(localizedPath(config.defaultLocale))}">`,
  ].join('')
  let document = html.replace(/<html(?:\s[^>]*)?>/i, (tag) => {
    if (/\slang\s*=/i.test(tag))
      return tag.replace(/\slang\s*=\s*(["']).*?\1/i, ` lang="${locale}"`)
    return tag.replace(/>$/, ` lang="${locale}">`)
  })
  if (!document.includes('hreflang=')) {
    document = /<\/head>/i.test(document)
      ? document.replace(/<\/head>/i, `${links}</head>`)
      : document.replace(/<body(?:\s[^>]*)?>/i, `${links}$&`)
  }
  return document
}

function escapeHtmlAttribute(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('"', '&quot;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
}

const MAX_TRACKED_RATE_LIMIT_KEYS = 10_000

/** Compile validated built-in middleware into a Fetch-native wrapper. */
function createFetchMiddleware(config) {
  const builtin = config?.builtin
  if (!builtin || typeof builtin !== 'object') {
    return async (_request, next) => next()
  }

  const cors = builtin.cors && typeof builtin.cors === 'object' ? builtin.cors : null
  const rate = builtin.rate && typeof builtin.rate === 'object' ? builtin.rate : null
  const customHeaders = validHeaderEntries(builtin.headers)
  const buckets = new Map()
  let nextRequestId = 1

  return async function applyFetchMiddleware(request, next) {
    const started = nowMilliseconds()
    const requestId =
      normalizedRequestId(request.headers.get('x-request-id')) ??
      `ruvyxa-${(nextRequestId++).toString(16)}`

    let response
    const preflight = corsPreflightResponse(request, cors)
    if (preflight) {
      response = preflight
    } else {
      const limited = rateLimitResponse(request, rate, buckets)
      response = limited ?? (await next())
      response = withCorsHeaders(response, request, cors)
    }

    const headers = new Headers(response.headers)
    for (const [name, value] of customHeaders) headers.set(name, value)
    const elapsed = Math.max(0, nowMilliseconds() - started)
    if (builtin.timing === true) headers.set('x-response-time', `${Math.floor(elapsed)}ms`)
    if (builtin.log === true) headers.set('x-request-id', requestId)

    const result = new Response(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers,
    })
    if (builtin.log === true) {
      console.info(
        `[ruvyxa] request_id=${requestId} method=${request.method} path=${new URL(request.url).pathname} ` +
          `status=${result.status} duration_ms=${Math.floor(elapsed)}`,
      )
    }
    return result
  }
}

function validHeaderEntries(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return []
  const entries = []
  for (const [name, headerValue] of Object.entries(value)) {
    try {
      const headers = new Headers([[name, String(headerValue)]])
      entries.push([name, headers.get(name)])
    } catch {
      // Project config validation rejects these. Direct createHandler callers
      // still fail closed instead of crashing every request at runtime.
    }
  }
  return entries
}

function corsPreflightResponse(request, cors) {
  if (!cors || request.method !== 'OPTIONS') return null
  const requestedMethod = request.headers.get('access-control-request-method')
  if (!requestedMethod || !isAllowedCorsOrigin(request.headers.get('origin'), cors)) return null
  return withCorsHeaders(new Response(null, { status: 204 }), request, cors, true)
}

function withCorsHeaders(response, request, cors, preflight = false) {
  if (!cors) return response
  const headers = new Headers(response.headers)
  const origin = request.headers.get('origin')
  if (isAllowedCorsOrigin(origin, cors)) {
    headers.set('access-control-allow-origin', origin)
    appendVaryOrigin(headers)
    const methods = Array.isArray(cors.methods) ? cors.methods : []
    const allowedHeaders = Array.isArray(cors.headers) ? cors.headers : []
    if (methods.length > 0) headers.set('access-control-allow-methods', methods.join(', '))
    if (allowedHeaders.length > 0) {
      headers.set('access-control-allow-headers', allowedHeaders.join(', '))
    }
    if (cors.credentials === true) headers.set('access-control-allow-credentials', 'true')
    headers.set('access-control-max-age', String(cors.maxAge ?? 86400))
  } else {
    appendVaryOrigin(headers)
  }
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  })
}

function isAllowedCorsOrigin(origin, cors) {
  return (
    typeof origin === 'string' &&
    Array.isArray(cors.origins) &&
    !(cors.credentials === true && cors.origins.includes('*')) &&
    (cors.origins.includes('*') || cors.origins.includes(origin))
  )
}

function appendVaryOrigin(headers) {
  const values = (headers.get('vary') ?? '')
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean)
  if (!values.some((value) => value.toLowerCase() === 'origin')) values.push('Origin')
  headers.set('vary', values.join(', '))
}

function rateLimitResponse(request, rate, buckets) {
  if (!rate) return null
  const max = Number(rate.max)
  const windowSeconds = Number(rate.window)
  if (!Number.isInteger(max) || max < 1 || !Number.isFinite(windowSeconds) || windowSeconds <= 0) {
    return new Response('Rate limit configuration error', { status: 500 })
  }

  const now = Date.now()
  const windowMs = windowSeconds * 1000
  const key = rateLimitKey(request, rate.key)
  let bucket = buckets.get(key)
  if (bucket && now - bucket.startedAt >= windowMs) {
    buckets.delete(key)
    bucket = undefined
  }
  if (!bucket) {
    if (buckets.size >= MAX_TRACKED_RATE_LIMIT_KEYS) {
      for (const [trackedKey, tracked] of buckets) {
        if (now - tracked.startedAt >= windowMs) buckets.delete(trackedKey)
      }
      if (buckets.size >= MAX_TRACKED_RATE_LIMIT_KEYS) {
        return new Response('Rate limit exceeded', {
          status: 429,
          headers: { 'content-type': 'text/plain; charset=utf-8', 'retry-after': '1' },
        })
      }
    }
    bucket = { remaining: max, startedAt: now }
    buckets.set(key, bucket)
  }
  if (bucket.remaining > 0) {
    bucket.remaining -= 1
    return null
  }
  const retryAfter = Math.max(1, Math.ceil((windowMs - (now - bucket.startedAt)) / 1000))
  return new Response('Rate limit exceeded', {
    status: 429,
    headers: { 'content-type': 'text/plain; charset=utf-8', 'retry-after': String(retryAfter) },
  })
}

function rateLimitKey(request, configuredKey) {
  if (typeof configuredKey === 'string' && configuredKey.startsWith('header:')) {
    return request.headers.get(configuredKey.slice('header:'.length)) ?? 'unknown'
  }
  // Edge runtimes do not expose a transport SocketAddr. These headers are set
  // by the supported platforms at their trusted ingress; explicit `header:`
  // remains available for custom deployments.
  return (
    request.headers.get('cf-connecting-ip') ??
    request.headers.get('x-vercel-forwarded-for') ??
    request.headers.get('x-real-ip') ??
    'unknown'
  )
}

function normalizedRequestId(value) {
  return typeof value === 'string' && value.length > 0 && value.length <= 128 ? value : null
}

function nowMilliseconds() {
  return globalThis.performance?.now?.() ?? Date.now()
}

function withDefaultSecurityHeaders(response) {
  const headers = new Headers(response.headers)
  for (const [name, value] of Object.entries(DEFAULT_SECURITY_HEADERS)) {
    if (!headers.has(name)) headers.set(name, value)
  }
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  })
}

function normalizeCacheEntry(value) {
  if (typeof value === 'string') return { html: value, stale: true }
  if (!value || typeof value !== 'object' || typeof value.html !== 'string') return null
  return { html: value.html, stale: value.stale === true }
}

// ─── Prerender Cache Paths ──────────────────────────────────────────────────

/**
 * Map a request path to the relative location of its pre-rendered HTML.
 *
 * Mirrors the build writer, which stores `<prerenderDir>/<path>/index.html`
 * from its canonical route path. Request handlers canonicalize before calling
 * this mapper; direct callers must provide the path representation they store.
 *
 * Returns `null` when the path cannot be mapped to a contained location.
 * Adapters join the result onto their cache directory and touch the file
 * system, so this is the single place that decides what is in bounds — the
 * platform URL parser is not a substitute, because adapters may be handed a
 * path from a source that never went through it.
 *
 * @param {string} pathname Request path, beginning with `/`.
 * @returns {string|null} A `.../index.html` relative path, or null if unsafe.
 */
/**
 * Reject a path segment that could escape, or misname, the cache directory.
 *
 * Written as explicit character tests rather than a regular expression: this
 * guard decides what reaches the file system, and it must stay obvious that
 * separators, control characters, and Windows stream/drive separators are all
 * covered.
 */
function isUnsafeSegment(segment) {
  if (segment === '.' || segment === '..') return true
  for (const char of segment) {
    if (char === '/' || char === '\\' || char === ':') return true
    const code = char.codePointAt(0)
    if (code < 0x20 || code === 0x7f) return true
  }
  return false
}

export function prerenderRelativePath(pathname) {
  if (typeof pathname !== 'string' || !pathname.startsWith('/')) return null

  const segments = []
  for (const segment of pathname.split('/')) {
    if (segment === '') continue
    if (isUnsafeSegment(segment)) return null
    segments.push(segment)
  }

  return segments.length === 0 ? 'index.html' : `${segments.join('/')}/index.html`
}

// ─── Static Asset Paths ─────────────────────────────────────────────────────

/**
 * Extensions that only ever name a build or public asset. Kept to images,
 * fonts, media, and emitted web assets: these are never a plausible value for
 * a dynamic route parameter, so refusing them cannot swallow a real page.
 * Mirrors `is_static_asset_request` in `crates/ruvyxa_dev_server/src/static_assets.rs`.
 */
const STATIC_ASSET_EXTENSIONS = new Set([
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
])

/**
 * Well-known crawler files that are never a page.
 *
 * `.txt` and `.xml` are deliberately absent from `STATIC_ASSET_EXTENSIONS` — a
 * route may legitimately end in either — but these exact paths are fixed by
 * convention. Letting `/[lang]` answer `/robots.txt` returns 200 with an HTML
 * body, which is what Lighthouse's `robots-txt` audit fails on. Mirrors
 * `is_crawler_discovery_path()` in
 * `crates/ruvyxa_dev_server/src/static_assets.rs`.
 */
const CRAWLER_DISCOVERY_PATHS = new Set(['/robots.txt', '/sitemap.xml', '/sitemap_index.xml'])

/** True when the last path segment names a static asset file. */
export function isStaticAssetPath(pathname) {
  if (typeof pathname !== 'string') return false
  if (CRAWLER_DISCOVERY_PATHS.has(pathname.replace(/\/+$/, ''))) return true
  const lastSlash = pathname.lastIndexOf('/')
  const segment = lastSlash === -1 ? pathname : pathname.slice(lastSlash + 1)
  const dot = segment.lastIndexOf('.')
  if (dot <= 0 || dot === segment.length - 1) return false
  return STATIC_ASSET_EXTENSIONS.has(segment.slice(dot + 1).toLowerCase())
}

/** True when the route pattern contains a dynamic, catch-all, or optional segment. */
function hasDynamicSegment(routePath) {
  return typeof routePath === 'string' && routePath.includes('[')
}

// ─── Route Matching ─────────────────────────────────────────────────────────

/**
 * Remove `basePath` from a request path.
 *
 * Returns the remaining path, or `null` when the request falls outside the
 * base path and must not be served by this handler.
 */
function stripBasePath(pathname, basePath) {
  if (!basePath) return pathname

  const prefix = basePath.endsWith('/') ? basePath.slice(0, -1) : basePath
  if (!prefix) return pathname
  if (pathname === prefix) return '/'
  // Require a segment boundary so `/appointments` is not treated as `/app`
  // plus `ointments`.
  if (!pathname.startsWith(`${prefix}/`)) return null
  return pathname.slice(prefix.length) || '/'
}

/**
 * Decode a request path exactly once while preserving segment boundaries.
 *
 * Thin wrapper over the shared `canonicalRoutePath`, which answers "is this
 * path acceptable, and what are its segments?" for every JavaScript host. The
 * handler needs the rejection as an exception so `dispatch` can turn it into a
 * 400 with a message, while the client matcher wants a null — the decision
 * itself must stay in one place, so only the reporting differs here.
 */
function canonicalRequestPath(rawPathname) {
  if (typeof rawPathname !== 'string' || !rawPathname.startsWith('/')) {
    throw new URIError('Request path must start with "/"')
  }
  const canonical = canonicalRoutePath(rawPathname)
  if (canonical === null) {
    throw new URIError('Request path contains an unsafe encoded segment')
  }
  return canonical
}

function matchRoute(compiledRoutes, pathname) {
  for (const route of compiledRoutes) {
    const match = route.pattern.regex.exec(pathname)
    if (!match) continue
    return { route, params: bindPatternParams(route.pattern, match) }
  }
  return null
}

/**
 * Match a request path against a route table, exposed for cross-implementation
 * testing.
 *
 * The handler, `@ruvyxa/react`'s router, and the standalone server all compile
 * their tables with the same shared `compilePattern`/`routeSpecificity`, so a
 * link click and a page reload resolve the same URL to the same route and
 * params by construction rather than by review. This entry point exists so the
 * conformance suite can drive the handler's own dispatch path — including its
 * base-path and error reporting behaviour — against the shared case table in
 * `tests/fixtures/route-match-conformance.json`, alongside the Rust router.
 * It is not part of the handler's runtime path.
 */
export function resolveRouteForTesting(routes, pathname) {
  const compiled = routes
    .map((route) => ({
      ...route,
      pattern: compilePattern(route.path),
      specificity: routeSpecificity(route.path),
    }))
    .sort((left, right) => compareSpecificity(left.specificity, right.specificity))
  try {
    const canonicalPathname = canonicalRequestPath(pathname)
    const matched = matchRoute(compiled, canonicalPathname)
    return matched
      ? { path: matched.route.path, params: matched.params, pathname: canonicalPathname }
      : null
  } catch {
    return null
  }
}

// ─── Response Normalization ─────────────────────────────────────────────────

function normalizeResponse(result) {
  if (result instanceof Response) return result
  return Response.json(result)
}
