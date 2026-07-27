import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { getRouterInstance } from '../dist/router.js'

function deferred() {
  let resolve
  const promise = new Promise((complete) => {
    resolve = complete
  })
  return { promise, resolve }
}

function routeModuleSource(route, gate) {
  const source = `
    await globalThis[${JSON.stringify(gate)}]
    globalThis.__RUVYXA_ROUTES__[${JSON.stringify(route)}] = (context) => context
  `
  return `data:text/javascript,${encodeURIComponent(source)}`
}

describe('client router navigation state', () => {
  it('keeps pending true when a stale route load finishes before the current navigation', async () => {
    const keys = [
      'window',
      '__RUVYXA_ROUTES__',
      '__RUVYXA_ROOT__',
      '__RUVYXA_ROUTE_PARAMS__',
      '__RUVYXA_REQUEST_PATH__',
      '__RUVYXA_ROUTE_MANIFEST__',
      '__RUVYXA_ROUTER_INSTANCE__',
      '__RUVYXA_TEST_ROUTE_A__',
      '__RUVYXA_TEST_ROUTE_B__',
    ]
    const previous = new Map(
      keys.map((key) => [key, Object.getOwnPropertyDescriptor(globalThis, key)]),
    )
    const routeA = deferred()
    const routeB = deferred()

    try {
      globalThis.window = {
        location: {
          href: 'https://example.test/',
          origin: 'https://example.test',
          pathname: '/',
          search: '',
          assign() {},
          replace() {},
        },
        history: {
          pushState() {},
          replaceState() {},
          back() {},
          forward() {},
        },
        addEventListener() {},
        scrollTo() {},
      }
      globalThis.__RUVYXA_ROUTES__ = {}
      globalThis.__RUVYXA_ROOT__ = { render() {} }
      globalThis.__RUVYXA_REQUEST_PATH__ = '/'
      globalThis.__RUVYXA_ROUTE_MANIFEST__ = {
        routes: [
          {
            path: '/slow-a',
            src: routeModuleSource('/slow-a', '__RUVYXA_TEST_ROUTE_A__'),
          },
          {
            path: '/slow-b',
            src: routeModuleSource('/slow-b', '__RUVYXA_TEST_ROUTE_B__'),
          },
        ],
      }
      globalThis.__RUVYXA_TEST_ROUTE_A__ = routeA.promise
      globalThis.__RUVYXA_TEST_ROUTE_B__ = routeB.promise
      delete globalThis.__RUVYXA_ROUTER_INSTANCE__

      const router = getRouterInstance()
      const firstNavigation = router.navigate('/slow-a')
      await Promise.resolve()
      await Promise.resolve()
      assert.equal(router.getPending(), true)

      const secondNavigation = router.navigate('/slow-b')
      await Promise.resolve()
      await Promise.resolve()
      assert.equal(router.getPending(), true)

      routeA.resolve()
      await firstNavigation
      assert.equal(router.getPending(), true)

      routeB.resolve()
      await secondNavigation
      assert.equal(router.getPending(), false)
      assert.equal(router.getSnapshot().pathname, '/slow-b')
    } finally {
      routeA.resolve()
      routeB.resolve()
      for (const [key, descriptor] of previous) {
        if (descriptor) Object.defineProperty(globalThis, key, descriptor)
        else delete globalThis[key]
      }
    }
  })
})
