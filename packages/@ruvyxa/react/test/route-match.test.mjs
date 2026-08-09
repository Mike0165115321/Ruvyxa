import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { describe, it } from 'node:test'
import { fileURLToPath } from 'node:url'

import { canonicalRoutePath, createRouteMatcher } from '@ruvyxa/core/route-match'

// The browser router and the serverless handler now share one matcher module,
// so they agree by construction. What still has to be proven is that the shared
// module answers what every other host answers — including the Rust router,
// which replays the same fixture in
// `crates/ruvyxa_dev_server/src/router.rs::matches_the_shared_cross_language_conformance_table`.
const serverModuleUrl = new URL('../../../ruvyxa/runtime/serverless-handler.mjs', import.meta.url)
const { resolveRouteForTesting } = await import(serverModuleUrl)

// Sanity that the path resolved (helps when the monorepo layout changes).
fileURLToPath(serverModuleUrl)

const fixtureUrl = new URL(
  '../../../../tests/fixtures/route-match-conformance.json',
  import.meta.url,
)
const fixture = JSON.parse(readFileSync(fixtureUrl, 'utf8'))
const ROUTES = fixture.routes

describe('shared route-match conformance table', () => {
  const match = createRouteMatcher(ROUTES)

  for (const { input, output } of fixture.canonical) {
    it(`canonicalizes ${input} to ${output === null ? 'a refusal' : output}`, () => {
      assert.equal(canonicalRoutePath(input), output)
    })
  }

  for (const testCase of fixture.match) {
    const { path, route, params } = testCase
    it(`resolves ${path} to ${route ?? 'no route'}`, () => {
      const client = match(path)
      const server = resolveRouteForTesting(ROUTES, path)

      if (route === null) {
        assert.equal(client, null)
        assert.equal(server, null)
        return
      }

      assert.ok(client, `client failed to match ${path}`)
      assert.equal(client.route.path, route)
      assert.deepEqual(client.params, params)

      // The handler compiles its own table from the shared primitives, so this
      // also covers its dispatch path, not just the matcher it borrows.
      assert.ok(server, `server failed to match ${path}`)
      assert.equal(server.path, route)
      assert.deepEqual(server.params, params)
    })
  }
})

describe('createRouteMatcher route selection', () => {
  const match = createRouteMatcher(ROUTES)

  it('preserves the public match result shape', () => {
    assert.deepEqual(match('/about'), { route: ROUTES[1], params: {} })
  })

  it('normalizes trailing and doubled slashes to the same match', () => {
    const canonical = match('/blog/hello')
    assert.deepEqual(match('/blog/hello/'), canonical)
    assert.deepEqual(match('/blog//hello'), canonical)
  })

  it('omits an optional catch-all key when it captured nothing', () => {
    const result = match('/shop')
    assert.equal(result?.route.path, '/shop/[[...category]]')
    assert.deepEqual(result?.params, {})
  })

  it('refuses boundary-changing encoded segments on both hosts', () => {
    for (const pathname of ['/blog/%ZZ', '/blog/%2F', '/blog/%5C', '/blog/%2e%2e']) {
      assert.equal(match(pathname), null, pathname)
      assert.equal(resolveRouteForTesting(ROUTES, pathname), null, pathname)
    }
  })

  it('preserves the first manifest entry for duplicate static patterns', () => {
    const first = { path: '/duplicate', src: '/first.js' }
    const second = { path: '/duplicate', src: '/second.js' }
    assert.equal(createRouteMatcher([first, second])('/duplicate')?.route, first)
  })

  it('falls back from the static index to parameterized routes', () => {
    const parameterized = { path: '/catalog/[item]' }
    const exact = { path: '/catalog/featured' }
    const matchCatalog = createRouteMatcher([parameterized, exact])

    assert.equal(matchCatalog('/catalog/featured')?.route, exact)
    assert.deepEqual(matchCatalog('/catalog/widget'), {
      route: parameterized,
      params: { item: 'widget' },
    })
  })
})
