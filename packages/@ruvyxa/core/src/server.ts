export interface LoaderContext {
  params: Record<string, string>
  request: Request
  cache: typeof cache
}

export interface ActionContext<TInput> {
  input: TInput
  request: Request
  user?: unknown
  invalidate(key: string): void
}

export type LoaderHandler<TResult> = (ctx: LoaderContext) => TResult | Promise<TResult>

export interface Loader<TResult> {
  (ctx?: Partial<LoaderContext>): Promise<TResult>
  ruvyxa: {
    kind: 'loader'
  }
}

export function loader<TResult>(handler: LoaderHandler<TResult>): Loader<TResult> {
  const callable = async (ctx: Partial<LoaderContext> = {}) => {
    return handler({
      params: ctx.params ?? {},
      request: ctx.request ?? new Request('http://localhost/'),
      cache,
    })
  }

  return Object.assign(callable, {
    ruvyxa: {
      kind: 'loader' as const,
    },
  })
}

export interface Schema<TInput> {
  parse(value: unknown): TInput
}

export interface ActionBuilder<TInput = unknown> {
  input<TNextInput>(schema: Schema<TNextInput>): ActionBuilder<TNextInput>
  /** Publish an action event after a successful invocation. Omit channels to use the route channel. */
  realtime(channels?: string | readonly string[]): ActionBuilder<TInput>
  handler<TResult>(
    handler: (ctx: ActionContext<TInput>) => TResult | Promise<TResult>,
  ): ServerAction<TInput, TResult>
}

export interface ServerAction<TInput, TResult> {
  (input: TInput, ctx?: Partial<ActionContext<TInput>>): Promise<TResult>
  ruvyxa: {
    kind: 'action'
    realtime?: ActionRealtimeOptions
  }
}

export interface ActionRealtimeOptions {
  /** Explicit subscription channels. An empty list resolves to `route:<request pathname>`. */
  channels: readonly string[]
}

export const action: ActionBuilder = createActionBuilder()

function createActionBuilder<TInput>(
  schema?: Schema<TInput>,
  realtimeOptions?: ActionRealtimeOptions,
): ActionBuilder<TInput> {
  return {
    input<TNextInput>(nextSchema: Schema<TNextInput>) {
      return createActionBuilder(nextSchema, realtimeOptions)
    },
    realtime(channels: string | readonly string[] = []) {
      const values = typeof channels === 'string' ? [channels] : [...channels]
      if (values.length > 16) {
        throw new TypeError('action.realtime() accepts at most 16 channels')
      }
      for (const [index, channel] of values.entries()) {
        if (typeof channel !== 'string' || !/^[A-Za-z0-9:._/-]{1,128}$/.test(channel.trim())) {
          throw new TypeError(
            `action.realtime() channels[${index}] must use 1-128 letters, digits, colon, dot, underscore, slash, or dash`,
          )
        }
      }
      return createActionBuilder(schema, {
        channels: Object.freeze([...new Set(values.map((channel) => channel.trim()))]),
      })
    },
    handler<TResult>(handler: (ctx: ActionContext<TInput>) => TResult | Promise<TResult>) {
      const callable = async (rawInput: TInput, ctx: Partial<ActionContext<TInput>> = {}) => {
        const input = schema ? schema.parse(rawInput) : rawInput
        return handler({
          input,
          request: ctx.request ?? new Request('http://localhost/'),
          user: ctx.user,
          invalidate: ctx.invalidate ?? (() => {}),
        })
      }

      return Object.assign(callable, {
        ruvyxa: {
          kind: 'action' as const,
          ...(realtimeOptions ? { realtime: realtimeOptions } : {}),
        },
      })
    },
  }
}

// --- Production-grade Cache ---
// LRU-bounded, stale-while-revalidate, error-isolated cache store.
// Prevents unbounded memory growth in long-running production servers.

export interface CacheBuilder {
  /** Set time-to-live (e.g. "30s", "5m", "1h", "1d"). Default: 60s. */
  ttl(value: string): CacheBuilder
  /** Set stale-while-revalidate window (serves stale data while refreshing in background). */
  swr(value: string): CacheBuilder
  /** Retrieve or compute a value. Producer errors are isolated and don't crash the server. */
  get<T>(producer: () => T | Promise<T>): Promise<T>
}

export interface CacheEntry {
  value: unknown
  expiresAt: number
  staleUntil: number
  refreshing: boolean
}

/** Maximum cache entries before LRU eviction kicks in. */
const CACHE_MAX_ENTRIES = 1024

/**
 * Production in-memory TTL cache with LRU eviction and stale-while-revalidate.
 *
 * Features:
 * - Bounded to CACHE_MAX_ENTRIES to prevent memory leaks
 * - Stale-while-revalidate: serves expired data while refreshing in background
 * - Error isolation: producer failures return stale data when available
 * - Periodic cleanup of fully expired entries
 */
class CacheStore {
  #entries = new Map<string, CacheEntry>()
  #accessOrder: string[] = []
  #pendingWrites = new Map<string, Set<symbol>>()
  #maxEntries: number

  constructor(maxEntries = CACHE_MAX_ENTRIES) {
    this.#maxEntries = maxEntries
  }

  get(key: string): CacheEntry | undefined {
    const entry = this.#entries.get(key)
    if (entry) {
      // Move to end of access order (most recently used)
      this.#touchAccessOrder(key)
    }
    return entry
  }

  peek(key: string): CacheEntry | undefined {
    return this.#entries.get(key)
  }

  set(key: string, entry: CacheEntry): void {
    // Updating an existing key does not increase the cache size. Evicting before
    // that check would discard an unrelated LRU entry on every refresh at capacity.
    while (!this.#entries.has(key) && this.#entries.size >= this.#maxEntries) {
      this.#evictOldest()
    }

    this.#entries.set(key, entry)
    this.#touchAccessOrder(key)
  }

  delete(key: string): boolean {
    this.#accessOrder = this.#accessOrder.filter((k) => k !== key)
    this.#pendingWrites.delete(key)
    return this.#entries.delete(key)
  }

  clear(): void {
    this.#entries.clear()
    this.#accessOrder = []
    this.#pendingWrites.clear()
  }

  invalidate(keyOrPrefix?: string): void {
    if (keyOrPrefix === undefined) {
      this.clear()
      return
    }

    const keys = new Set([...this.#entries.keys(), ...this.#pendingWrites.keys()])
    for (const key of keys) {
      if (key === keyOrPrefix || key.startsWith(keyOrPrefix + ':')) {
        this.delete(key)
      }
    }
  }

  beginWrite(key: string): symbol {
    const token = Symbol(key)
    const writes = this.#pendingWrites.get(key) ?? new Set<symbol>()
    writes.add(token)
    this.#pendingWrites.set(key, writes)
    return token
  }

  commitWrite(key: string, token: symbol, entry: CacheEntry, expectedEntry?: CacheEntry): boolean {
    if (!this.#pendingWrites.get(key)?.has(token)) return false
    if (expectedEntry && this.#entries.get(key) !== expectedEntry) return false
    this.set(key, entry)
    return true
  }

  finishWrite(key: string, token: symbol): void {
    const writes = this.#pendingWrites.get(key)
    if (!writes) return
    writes.delete(token)
    if (writes.size === 0) this.#pendingWrites.delete(key)
  }

  /** Remove all entries that have fully expired (past staleUntil). */
  prune(): number {
    const now = Date.now()
    let pruned = 0
    for (const [key, entry] of this.#entries) {
      if (entry.staleUntil < now) {
        this.delete(key)
        pruned++
      }
    }
    if (pruned > 0) {
      this.#accessOrder = this.#accessOrder.filter((k) => this.#entries.has(k))
    }
    return pruned
  }

  get size(): number {
    return this.#entries.size
  }

  #touchAccessOrder(key: string): void {
    const idx = this.#accessOrder.indexOf(key)
    if (idx !== -1) {
      this.#accessOrder.splice(idx, 1)
    }
    this.#accessOrder.push(key)
  }

  #evictOldest(): void {
    const oldest = this.#accessOrder.shift()
    if (oldest !== undefined) {
      this.delete(oldest)
    }
  }
}

const cacheStore = new CacheStore()

// Periodic cleanup every 60s to reclaim memory from fully expired entries
let cleanupTimer: ReturnType<typeof setInterval> | undefined
if (typeof setInterval !== 'undefined') {
  cleanupTimer = setInterval(() => cacheStore.prune(), 60_000)
  // Don't hold the process open
  if (cleanupTimer && typeof cleanupTimer === 'object' && 'unref' in cleanupTimer) {
    ;(cleanupTimer as { unref(): void }).unref()
  }
}

function parseTtl(value: string): number {
  const match = value.match(/^(\d+)\s*(ms|s|m|h|d)$/)
  if (!match) {
    throw invalidCacheDuration(value)
  }
  const amount = Number(match[1])
  if (!Number.isSafeInteger(amount) || amount <= 0) {
    throw invalidCacheDuration(value)
  }

  const multiplier = (() => {
    switch (match[2]) {
      case 'ms':
        return 1
      case 's':
        return 1000
      case 'm':
        return 60_000
      case 'h':
        return 3_600_000
      case 'd':
        return 86_400_000
      default: {
        throw new Error(`Unsupported cache duration unit: ${match[2]}`)
      }
    }
  })()
  const duration = amount * multiplier
  if (!Number.isSafeInteger(duration)) {
    throw invalidCacheDuration(value)
  }
  return duration
}

function invalidCacheDuration(value: string): Error {
  return new Error(
    `Invalid cache duration "${value}". Use a positive value within JavaScript's safe integer range, such as "30s", "5m", "1h", or "1d".`,
  )
}

/**
 * Create a cache builder for the given key.
 *
 * Usage:
 * ```ts
 * const data = await cache("users:list").ttl("5m").swr("1m").get(async () => {
 *   return db.users.findMany()
 * })
 * ```
 */
export function cache(key: string): CacheBuilder {
  let ttlMs = 60_000 // default 60 seconds
  let swrMs = 0 // default: no stale-while-revalidate

  return {
    ttl(value: string) {
      ttlMs = parseTtl(value)
      return this
    },
    swr(value: string) {
      swrMs = parseTtl(value)
      return this
    },
    async get<T>(producer: () => T | Promise<T>): Promise<T> {
      const now = Date.now()
      const cached = cacheStore.get(key)

      // Fresh hit: return immediately
      if (cached && cached.expiresAt > now) {
        return cached.value as T
      }

      // Stale hit with SWR: return stale value and refresh in background
      if (cached && cached.staleUntil > now) {
        if (!cached.refreshing) {
          cached.refreshing = true
          const writeToken = cacheStore.beginWrite(key)
          // Fire-and-forget background refresh. All concurrent stale readers
          // receive the stale value; only the first reader starts the refresh.
          Promise.resolve()
            .then(() => producer())
            .then((value) => {
              const populatedAt = Date.now()
              const committed = cacheStore.commitWrite(
                key,
                writeToken,
                {
                  value,
                  expiresAt: populatedAt + ttlMs,
                  staleUntil: populatedAt + ttlMs + swrMs,
                  refreshing: false,
                },
                cached,
              )
              // A rejected commit leaves the old entry in place. Without
              // clearing its flag the entry claims a refresh is still running
              // and no later reader ever starts another one, so it serves
              // stale until it falls out of the window entirely.
              if (!committed && cacheStore.peek(key) === cached) cached.refreshing = false
            })
            .catch(() => {
              // Producer failed during background refresh — keep serving stale
              if (cacheStore.peek(key) === cached) cached.refreshing = false
            })
            .finally(() => cacheStore.finishWrite(key, writeToken))
        }
        return cached.value as T
      }

      // Miss or fully expired: produce fresh value with error isolation
      const writeToken = cacheStore.beginWrite(key)
      try {
        const value = await producer()
        const populatedAt = Date.now()
        cacheStore.commitWrite(key, writeToken, {
          value,
          expiresAt: populatedAt + ttlMs,
          staleUntil: populatedAt + ttlMs + swrMs,
          refreshing: false,
        })
        return value
      } catch (error) {
        // If we have stale data, return it rather than propagating the error
        if (cached && cacheStore.peek(key) === cached) {
          return cached.value as T
        }
        throw error
      } finally {
        cacheStore.finishWrite(key, writeToken)
      }
    },
  }
}

/**
 * Invalidate a specific cache key, all keys matching a prefix, or the entire cache.
 *
 * @param keyOrPrefix - If omitted, clears the entire cache. If provided, clears the
 *   exact key and any keys that start with `keyOrPrefix:`.
 */
export function invalidateCache(keyOrPrefix?: string): void {
  cacheStore.invalidate(keyOrPrefix)
}

/**
 * Get current cache statistics for observability.
 */
export function cacheStats(): { size: number; maxEntries: number } {
  return { size: cacheStore.size, maxEntries: CACHE_MAX_ENTRIES }
}

export function redirect(location: string, status = 302): Response {
  if (status < 300 || status > 399) {
    throw new Error(`redirect() status must be 3xx, got ${status}`)
  }
  return new Response(null, {
    status,
    headers: {
      Location: location,
    },
  })
}

export function notFound(message = 'Not found'): Response {
  return new Response(message, { status: 404 })
}

export function json(data: unknown, init?: ResponseInit): Response {
  return Response.json(data, init)
}

/**
 * Ambient access to the request being served.
 *
 * A page component is called by the renderer, not by the router, so it has no
 * parameter through which a `Request` could reach it. `cookies()`, `headers()`,
 * and `draftMode()` close that gap the way Next.js does: the host installs a
 * per-request store before rendering, and these read it.
 *
 * ## Why the store lives on `globalThis`
 *
 * The host that installs the store (`packages/ruvyxa/runtime/*.mjs`) and the
 * page that reads it are compiled separately and may each end up with their own
 * copy of this module — the SSR bundle aliases `ruvyxa/server` to the workspace
 * source, while a dependency importing it resolves `dist`. A module-level
 * variable would be per-copy, so a page would read a store the host never set.
 * A well-known key on `globalThis` is the one thing both copies agree on. The
 * same reasoning already governs `__RUVYXA_ROUTE_CONTEXT__`.
 *
 * ## Why the store is not created here
 *
 * Isolating concurrent renders needs `AsyncLocalStorage`, and importing
 * `node:async_hooks` from this module would put a Node built-in in every edge
 * and browser bundle that touches `@ruvyxa/core/server`. The host owns that
 * import and installs an implementation; this module only reads one.
 */

/** One request's data, as the host provides it. */
export interface RequestContext {
  /** Request headers in wire order, so repeated names survive. */
  headers: readonly (readonly [string, string])[]
  /** Request method, uppercased. */
  method: string
  /** Path and query of the request target. */
  url: string
  /** Whether draft mode is enabled for this request. */
  draft: boolean
  /**
   * `Set-Cookie` values a server action or API route has queued.
   *
   * Absent during page rendering: the response headers are already being
   * written by the time a page renders, so a cookie set there would be
   * silently dropped. `cookies().set()` reports that rather than pretending.
   */
  setCookies?: string[]
}

/** The seam a host installs on `globalThis`. */
export interface RequestContextHost {
  /** The context for the request being served on this call stack, if any. */
  current(): RequestContext | null
}

const CONTEXT_KEY = '__RUVYXA_REQUEST_CONTEXT__'

function host(): RequestContextHost | null {
  return (globalThis as Record<string, unknown>)[CONTEXT_KEY] as RequestContextHost | null
}

/**
 * Install the per-request store. Called by Ruvyxa's runtime hosts, not by
 * applications.
 */
export function installRequestContextHost(implementation: RequestContextHost): void {
  ;(globalThis as Record<string, unknown>)[CONTEXT_KEY] = implementation
}

/**
 * The active request, or an error naming the accessor that needed one.
 *
 * Deliberately not named `require`: a local function by that name is rewritten
 * to a module load by bundlers targeting CommonJS, which turned every
 * `cookies()` call into an import of a package called `cookies()`.
 */
function activeRequest(api: string): RequestContext {
  const context = host()?.current()
  if (!context) {
    throw new Error(
      `${api} was called outside a request.\n\n` +
        'It is available while a page, API route, or server action is being served. ' +
        'Calling it at module scope runs at import time, when there is no request to read — ' +
        'move the call inside the component or handler.',
    )
  }
  return context
}

/** Read-only view of one request's cookies. */
export interface RequestCookies {
  get(name: string): string | undefined
  has(name: string): boolean
  /** Every cookie on the request, in the order the header listed them. */
  getAll(): { name: string; value: string }[]
}

/**
 * Cookies sent with the request being served.
 *
 * Reading a cookie makes a page's output depend on who is asking, so a route
 * that calls this is served per request and is never stored in a shared render
 * cache. See `route_reads_request_state` in `crates/ruvyxa_graph/src/lib.rs`.
 *
 * @example
 * ```tsx
 * export default function Page() {
 *   const theme = cookies().get('theme') ?? 'light'
 *   return <main data-theme={theme} />
 * }
 * ```
 */
export function cookies(): RequestCookies {
  const context = activeRequest('cookies()')
  const parsed = parseCookieHeader(headerValue(context, 'cookie'))
  return {
    get: (name) => parsed.find((entry) => entry.name === name)?.value,
    has: (name) => parsed.some((entry) => entry.name === name),
    getAll: () => parsed.map((entry) => ({ ...entry })),
  }
}

/**
 * Headers sent with the request being served.
 *
 * Returns a standard read-only `Headers`, so `get`, `has`, `getSetCookie`, and
 * iteration all behave as they do on a `Request`.
 */
export function headers(): Headers {
  const context = activeRequest('headers()')
  const collected = new Headers()
  for (const [name, value] of context.headers) collected.append(name, value)
  return collected
}

/** Draft mode state for the request being served. */
export interface DraftMode {
  /** Whether this request is in draft mode. */
  readonly isEnabled: boolean
}

/**
 * Whether the request is previewing unpublished content.
 *
 * Enabled by the `__ruvyxa_draft` cookie, which an API route sets after
 * checking whatever secret the CMS shares with the application. A request in
 * draft mode is never served from a static or incrementally regenerated cache,
 * for the same reason a request that reads cookies is not.
 */
export function draftMode(): DraftMode {
  const context = activeRequest('draftMode()')
  return { isEnabled: context.draft }
}

/** Cookie name that turns draft mode on. Shared with the Rust request path. */
export const DRAFT_MODE_COOKIE = '__ruvyxa_draft'

function headerValue(context: RequestContext, name: string): string {
  const lowered = name.toLowerCase()
  const values = context.headers
    .filter(([header]) => header.toLowerCase() === lowered)
    .map(([, value]) => value)
  return values.join('; ')
}

/**
 * Split a `Cookie` header into name/value pairs.
 *
 * Deliberately tolerant: a malformed pair is skipped rather than throwing,
 * because the header is attacker-controlled and a page must not fail to render
 * because a browser extension wrote something odd. A value is returned exactly
 * as sent apart from surrounding whitespace and one layer of double quotes —
 * percent-decoding is the application's choice, since not every cookie is
 * percent-encoded and decoding one that is not can throw.
 */
export function parseCookieHeader(header: string): { name: string; value: string }[] {
  const entries: { name: string; value: string }[] = []
  for (const part of header.split(';')) {
    const separator = part.indexOf('=')
    if (separator <= 0) continue
    const name = part.slice(0, separator).trim()
    if (!name) continue
    let value = part.slice(separator + 1).trim()
    if (value.length >= 2 && value.startsWith('"') && value.endsWith('"')) {
      value = value.slice(1, -1)
    }
    entries.push({ name, value })
  }
  return entries
}
