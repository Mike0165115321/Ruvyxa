import assert from 'node:assert/strict'
import { describe, it } from 'node:test'
import { fileURLToPath } from 'node:url'

import { canonicalRoutePath, createRouteMatcher } from '../dist/route-match.js'
// The server matcher lives in the ruvyxa runtime. Both must resolve any URL to
// the same route and params, or a soft navigation would render a different
// page than a reload of the same address.
const serverModuleUrl = new URL('../../../ruvyxa/runtime/serverless-handler.mjs', import.meta.url)
const { resolveRouteForTesting } = await import(serverModuleUrl)

// Sanity that the path resolved (helps when the monorepo layout changes).
fileURLToPath(serverModuleUrl)

const ROUTES = [
  { path: '/' },
  { path: '/about' },
  { path: '/blog/new' },
  { path: '/blog/[slug]' },
  { path: '/docs/[...slug]' },
  { path: '/shop/[[...category]]' },
  { path: '/users/[id]/posts/[postId]' },
  { path: '/ทดสอบ' },
]

const CASES = [
  '/',
  '/about',
  '/blog/new', // static must win over /blog/[slug]
  '/blog/hello',
  '/blog/hello/', // trailing slash normalizes to the same match
  '/blog//hello', // doubled slash normalizes too
  '/docs/a/b/c', // catch-all captures the rest, split into segments
  '/docs/a%20b', // percent-decoded per segment
  '/shop', // optional catch-all matches the bare parent
  '/shop/electronics/phones',
  '/users/7/posts/42',
  '/%E0%B8%97%E0%B8%94%E0%B8%AA%E0%B8%AD%E0%B8%9A', // encoded Unicode static route
  '/blog/%2520', // decode once: the parameter value is the literal string "%20"
  '/nope/nope', // no route
]

describe('createRouteMatcher parity with the server matcher', () => {
  const match = createRouteMatcher(ROUTES)

  for (const pathname of CASES) {
    it(`resolves ${pathname} identically on client and server`, () => {
      const client = match(pathname)
      const server = resolveRouteForTesting(ROUTES, pathname)

      if (server === null) {
        assert.equal(client, null)
        return
      }

      assert.ok(client, `client failed to match ${pathname}`)
      assert.equal(client.route.path, server.path)
      assert.deepEqual(client.params, server.params)
      assert.equal(canonicalRoutePath(pathname), server.pathname)
    })
  }
})

describe('createRouteMatcher route selection', () => {
  const match = createRouteMatcher(ROUTES)

  it('preserves the public match result shape', () => {
    assert.deepEqual(match('/about'), { route: ROUTES[1], params: {} })
  })

  it('prefers a static segment over a dynamic one at the same position', () => {
    assert.equal(match('/blog/new')?.route.path, '/blog/new')
    assert.equal(match('/blog/other')?.route.path, '/blog/[slug]')
  })

  it('captures a required catch-all as a decoded segment array', () => {
    assert.deepEqual(match('/docs/a/b')?.params, { slug: ['a', 'b'] })
  })

  it('matches encoded Unicode static routes and decodes parameters exactly once', () => {
    assert.equal(match('/%E0%B8%97%E0%B8%94%E0%B8%AA%E0%B8%AD%E0%B8%9A')?.route.path, '/ทดสอบ')
    assert.equal(match('/blog/%2520')?.params.slug, '%20')
    assert.equal(canonicalRoutePath('/blog/%2520'), '/blog/%20')
  })

  it('omits an optional catch-all key when it captured nothing', () => {
    const result = match('/shop')
    assert.equal(result?.route.path, '/shop/[[...category]]')
    assert.deepEqual(result?.params, {})
  })

  it('returns null for an unmatched path', () => {
    assert.equal(match('/does/not/exist/here'), null)
  })

  it('rejects malformed and boundary-changing encoded segments as non-matches', () => {
    for (const pathname of ['/blog/%ZZ', '/blog/%2F', '/blog/%5C', '/blog/%2e%2e']) {
      assert.equal(match(pathname), null, pathname)
      assert.equal(resolveRouteForTesting(ROUTES, pathname), null, pathname)
    }
  })
})
