/**
 * Route matching — the single JavaScript implementation.
 *
 * Every JavaScript host that has to answer "which route is this URL?" reads
 * this module: the browser router in `@ruvyxa/react`, the serverless handler
 * in `packages/ruvyxa/runtime/serverless-handler.mjs` (via the generated
 * `runtime/route-match.mjs` copy), and the standalone Node server. A URL that
 * resolves differently in any of them renders a different page than a reload
 * of the same address, so they are not allowed to hold separate ports of these
 * rules.
 *
 * The Rust router in `crates/ruvyxa_dev_server/src/router.rs` cannot share this
 * code — it is a different language — so it is held to the same behaviour by a
 * shared case table instead: `tests/fixtures/route-match-conformance.json` is
 * replayed by both the Rust and the Node test suites.
 *
 * This module must stay dependency-free and free of Node and DOM APIs. It is
 * copied verbatim into serverless function bundles, where nothing else is
 * resolvable, and it runs in the browser.
 */

/** Route parameters extracted from a matched URL. */
export type RouteParams = Record<string, string | string[] | undefined>

/** A route entry as published in `.ruvyxa/client/manifest.json`. */
export interface RouteManifestEntry {
  /** Route pattern, e.g. `/blog/[slug]`. */
  path: string
  /** Client bundle URL for this route. */
  src?: string
  /** Shared chunks this route's bundle depends on. */
  sharedChunks?: Array<{ src: string }>
  /** Render strategy, when the manifest records one. */
  strategy?: string
}

/** A successful match of a URL against a route. */
export interface RouteMatch<Route extends RouteManifestEntry = RouteManifestEntry> {
  route: Route
  params: RouteParams
}

/** The compiled form of one route pattern. */
export interface CompiledPattern {
  regex: RegExp
  paramNames: string[]
  catchAll: { name: string; optional: boolean } | null
}

interface CompiledRoute<Route extends RouteManifestEntry> {
  route: Route
  pattern: CompiledPattern
  specificity: number[]
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/**
 * Build the matching regex and parameter list for a route pattern.
 *
 * An optional catch-all also has to match the parent path itself (`/shop` for
 * `/shop/[[...slug]]`), which is why its slash is inside the group.
 */
export function compilePattern(routePath: string): CompiledPattern {
  if (routePath === '/') {
    return { regex: /^\/$/, paramNames: [], catchAll: null }
  }

  const segments = routePath.split('/').filter(Boolean)
  const paramNames: string[] = []
  let catchAll: CompiledPattern['catchAll'] = null
  let pattern = '^'

  for (const segment of segments) {
    const optionalCatchAll = /^\[\[\.\.\.(\w+)\]\]$/.exec(segment)
    if (optionalCatchAll) {
      const name = optionalCatchAll[1]!
      paramNames.push(name)
      catchAll = { name, optional: true }
      pattern += '(?:/(.*))?'
      continue
    }

    const requiredCatchAll = /^\[\.\.\.(\w+)\]$/.exec(segment)
    if (requiredCatchAll) {
      const name = requiredCatchAll[1]!
      paramNames.push(name)
      catchAll = { name, optional: false }
      pattern += '/(.+)'
      continue
    }

    const dynamic = /^\[(\w+)\]$/.exec(segment)
    if (dynamic) {
      paramNames.push(dynamic[1]!)
      pattern += '/([^/]+)'
      continue
    }

    pattern += `/${escapeRegex(segment)}`
  }

  pattern += '/?$'
  return { regex: new RegExp(pattern), paramNames, catchAll }
}

/**
 * Per-segment specificity: static (0) < dynamic (1) < catch-all (2) <
 * optional catch-all (3). Lower sorts first, so `/blog/new` wins over
 * `/blog/[slug]`.
 */
export function routeSpecificity(routePath: string): number[] {
  if (routePath === '/') return [0]
  return routePath
    .split('/')
    .filter(Boolean)
    .map((segment) => {
      if (/^\[\[\.\.\.\w+\]\]$/.test(segment)) return 3
      if (/^\[\.\.\.\w+\]$/.test(segment)) return 2
      if (/^\[\w+\]$/.test(segment)) return 1
      return 0
    })
}

/** Order two specificity vectors; a shorter vector sorts before a longer one. */
export function compareSpecificity(left: number[], right: number[]): number {
  const length = Math.max(left.length, right.length)
  for (let index = 0; index < length; index++) {
    const leftScore = left[index] ?? -1
    const rightScore = right[index] ?? -1
    if (leftScore !== rightScore) return leftScore - rightScore
  }
  return 0
}

/**
 * Collapse a request path to the segment form the router matches against.
 *
 * `/docs/a/`, `/docs//a`, and `/docs/a` must resolve to the same route with
 * the same parameters; without this the greedy catch-all group captures the
 * trailing slash and produces a stray empty parameter segment.
 */
export function normalizeMatchPath(pathname: string): string {
  const segments = pathname.split('/').filter(Boolean)
  return segments.length === 0 ? '/' : `/${segments.join('/')}`
}

function hasControlCharacter(value: string): boolean {
  for (const character of value) {
    const code = character.codePointAt(0)!
    if (code <= 0x1f || (code >= 0x7f && code <= 0x9f)) return true
  }
  return false
}

/**
 * Decode each path segment exactly once without allowing an encoded value to
 * create a route boundary or traversal component. This mirrors
 * `canonical_request_path` at the development-server request boundary.
 *
 * Returns `null` when the path is malformed or an encoded segment would change
 * the segment structure — callers treat that as "no route", never as a match.
 */
export function canonicalRoutePath(pathname: string): string | null {
  if (!pathname.startsWith('/')) return null
  const decoded: string[] = []
  try {
    for (const segment of pathname.split('/').filter(Boolean)) {
      const value = decodeURIComponent(segment)
      if (
        value === '' ||
        value === '.' ||
        value === '..' ||
        value.includes('/') ||
        value.includes('\\') ||
        hasControlCharacter(value)
      ) {
        return null
      }
      decoded.push(value)
    }
  } catch {
    return null
  }
  return normalizeMatchPath(`/${decoded.join('/')}`)
}

/**
 * Bind one compiled pattern's captures to named parameters.
 *
 * Split out so a host that already has a match in hand — the serverless
 * handler dispatches on its own compiled table — produces exactly the same
 * parameter object as `createRouteMatcher`.
 */
export function bindPatternParams(pattern: CompiledPattern, matched: RegExpExecArray): RouteParams {
  const params: RouteParams = {}
  for (let index = 0; index < pattern.paramNames.length; index++) {
    const name = pattern.paramNames[index]!
    const value = matched[index + 1]

    if (pattern.catchAll && name === pattern.catchAll.name) {
      // An optional catch-all that captured nothing stays absent rather than
      // becoming `[]`: the documented contract is "undefined at the parent
      // route", and every host omits the key there.
      if (value) {
        params[name] = value.split('/')
      }
    } else {
      params[name] = value || undefined
    }
  }
  return params
}

/**
 * Compile a route table once and return a matcher over it.
 *
 * Manifest order is alphabetical, where `[` sorts before letters — matching in
 * that order would shadow `/blog/new` behind `/blog/[slug]`. Sorting by
 * specificity restores the static-first behaviour of the dev server.
 */
export function createRouteMatcher<Route extends RouteManifestEntry>(
  routes: readonly Route[],
): (pathname: string) => RouteMatch<Route> | null {
  const compiled: Array<CompiledRoute<Route>> = routes
    .map((route) => ({
      route,
      pattern: compilePattern(route.path),
      specificity: routeSpecificity(route.path),
    }))
    .sort((left, right) => compareSpecificity(left.specificity, right.specificity))

  return function match(pathname: string): RouteMatch<Route> | null {
    const normalized = canonicalRoutePath(pathname)
    if (normalized === null) return null

    for (const entry of compiled) {
      const matched = entry.pattern.regex.exec(normalized)
      if (!matched) continue
      return { route: entry.route, params: bindPatternParams(entry.pattern, matched) }
    }

    return null
  }
}
