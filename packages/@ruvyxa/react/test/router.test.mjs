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

/**
 * Minimal `document`/`CSS` stand-ins covering exactly what `prefetch` touches:
 * an href-keyed `modulepreload` lookup and an appendable head. A real DOM is
 * not needed to observe how many hints the router emits.
 */
function stubPreloadDocument() {
  const links = []
  return {
    links,
    document: {
      head: {
        append(node) {
          links.push(node)
        },
      },
      createElement() {
        return { rel: '', href: '' }
      },
      querySelector(selector) {
        const match = /^link\[rel="modulepreload"\]\[href="(.*)"\]$/.exec(selector)
        if (!match) return null
        const href = match[1].replaceAll('\\', '')
        return links.find((link) => link.rel === 'modulepreload' && link.href === href) ?? null
      },
    },
  }
}

describe('client router prefetch hints', () => {
  it('emits one modulepreload per module when routes share a chunk', async () => {
    const keys = [
      'window',
      'document',
      'CSS',
      '__RUVYXA_ROUTES__',
      '__RUVYXA_ROUTE_MANIFEST__',
      '__RUVYXA_ROUTER_INSTANCE__',
    ]
    const previous = new Map(
      keys.map((key) => [key, Object.getOwnPropertyDescriptor(globalThis, key)]),
    )

    try {
      const preload = stubPreloadDocument()
      globalThis.window = {
        location: {
          href: 'https://example.test/',
          origin: 'https://example.test',
          pathname: '/',
          search: '',
          assign() {},
          replace() {},
        },
        history: { pushState() {}, replaceState() {}, back() {}, forward() {} },
        addEventListener() {},
        scrollTo() {},
      }
      globalThis.document = preload.document
      globalThis.CSS = { escape: (value) => value }
      globalThis.__RUVYXA_ROUTES__ = {}
      globalThis.__RUVYXA_ROUTE_MANIFEST__ = {
        routes: [
          { path: '/a', src: '/chunks/a.js', sharedChunks: [{ src: '/chunks/vendor.js' }] },
          { path: '/b', src: '/chunks/b.js', sharedChunks: [{ src: '/chunks/vendor.js' }] },
        ],
      }
      delete globalThis.__RUVYXA_ROUTER_INSTANCE__

      const router = getRouterInstance()
      // `prefetch` resolves the manifest first, so each call finishes its work
      // in a microtask rather than inline.
      router.prefetch('/a')
      await Promise.resolve()
      router.prefetch('/b')
      await Promise.resolve()
      // A repeat of an already hinted route must stay a no-op.
      router.prefetch('/a')
      await Promise.resolve()

      const hinted = preload.links.map((link) => link.href)
      assert.deepEqual([...hinted].sort(), ['/chunks/a.js', '/chunks/b.js', '/chunks/vendor.js'])
    } finally {
      for (const [key, descriptor] of previous) {
        if (descriptor) Object.defineProperty(globalThis, key, descriptor)
        else delete globalThis[key]
      }
    }
  })
})

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
