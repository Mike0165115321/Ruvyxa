import { describe, it } from 'node:test'
import assert from 'node:assert/strict'

import {
  config,
  definePlugin,
  plugin,
  withResponseHeader,
  type RuvyxaConfig,
} from '../../../packages/@ruvyxa/core/src/config.ts'

describe('config API', () => {
  it('accepts builtin middleware and plugins', () => {
    const authPlugin = definePlugin({
      name: 'auth',
      setup({ addMiddleware, transform, onBuildComplete }) {
        addMiddleware({
          routes: ['/api/*'],
          onRequest(request) {
            return request.headers.has('authorization')
              ? undefined
              : new Response('Unauthorized', { status: 401 })
          },
        })
        transform((code, id, context) =>
          context.environment === 'client' && id.endsWith('.tsx')
            ? { code: `${code}\n// transformed` }
            : undefined,
        )
        onBuildComplete(({ root, outDir, manifest }) => {
          assert.ok(root)
          assert.ok(outDir)
          assert.ok(manifest)
        })
      },
    })
    const settings: RuvyxaConfig = {
      middleware: {
        workers: 2,
        timeoutMs: 15_000,
        builtin: {
          timing: true,
          log: true,
          cors: {
            origins: ['http://localhost:5173'],
            methods: ['GET', 'POST'],
            headers: ['Content-Type'],
            credentials: true,
            maxAge: 86400,
          },
          rate: {
            max: 100,
            window: 60,
            key: 'ip',
          },
          headers: {
            'X-Powered-By': 'Ruvyxa',
          },
        },
      },
      plugins: [authPlugin],
      adapterOptions: {
        region: 'iad1',
      },
      build: {
        treeShake: false,
        manifest: true,
      },
    }

    const defined = config(settings)

    assert.equal(defined.middleware?.builtin?.timing, true)
    assert.equal(defined.middleware?.workers, 2)
    assert.equal(defined.middleware?.timeoutMs, 15_000)
    assert.equal(defined.plugins?.[0]?.name, 'auth')
    assert.equal(defined.adapterOptions?.region, 'iad1')
    assert.equal(defined.build?.treeShake, false)
    assert.equal(defined.build?.manifest, true)
  })

  it('rejects malformed plugin definitions at the application boundary', () => {
    assert.throws(() => definePlugin({ name: ' ', setup() {} }), /must have a non-empty name/)
    assert.throws(() => definePlugin({ name: 'broken' } as never), /must provide setup\(context\)/)
  })

  it('creates a middleware plugin without setup boilerplate', () => {
    const auth = plugin('auth', {
      routes: ['/api/*'],
      onRequest: (request) =>
        request.headers.has('authorization')
          ? undefined
          : new Response('Unauthorized', { status: 401 }),
    })
    let registered: unknown

    auth.setup({
      addMiddleware(value) {
        registered = value
      },
      resolveId() {},
      transform() {},
      onBuildComplete() {},
      enableRealtime() {},
    })

    assert.equal(auth.name, 'auth')
    assert.deepEqual((registered as { routes?: string[] }).routes, ['/api/*'])
    assert.equal(typeof (registered as { onRequest?: unknown }).onRequest, 'function')

    const logger = plugin('logger', (request) => request)
    logger.setup({
      addMiddleware(value) {
        registered = value
      },
      resolveId() {},
      transform() {},
      onBuildComplete() {},
      enableRealtime() {},
    })
    assert.equal(typeof registered, 'function')
  })

  it('copies a response when changing one header', async () => {
    const original = new Response('Hello', {
      status: 201,
      statusText: 'Created',
      headers: { 'x-existing': 'kept' },
    })
    const updated = withResponseHeader(original, 'x-plugin', 'active')

    assert.notEqual(updated, original)
    assert.equal(updated.status, 201)
    assert.equal(updated.statusText, 'Created')
    assert.equal(updated.headers.get('x-existing'), 'kept')
    assert.equal(updated.headers.get('x-plugin'), 'active')
    assert.equal(await updated.text(), 'Hello')
  })

  it('turns declarative headers into response middleware', async () => {
    const responseHeaders = plugin('response-headers', {
      routes: ['/api/*'],
      headers: { 'x-plugin': 'active' },
    })
    let registered: unknown

    responseHeaders.setup({
      addMiddleware(value) {
        registered = value
      },
      resolveId() {},
      transform() {},
      onBuildComplete() {},
      enableRealtime() {},
    })

    const middleware = registered as {
      routes?: string[]
      onResponse?: (request: Request, response: Response) => Promise<Response | void>
    }
    const response = await middleware.onResponse?.(
      new Request('https://example.test/api/items'),
      new Response('OK', { status: 202, headers: { 'x-existing': 'kept' } }),
    )

    assert.deepEqual(middleware.routes, ['/api/*'])
    assert.equal(response?.status, 202)
    assert.equal(response?.headers.get('x-existing'), 'kept')
    assert.equal(response?.headers.get('x-plugin'), 'active')
    assert.equal(await response?.text(), 'OK')
  })
})
