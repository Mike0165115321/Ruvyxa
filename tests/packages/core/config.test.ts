import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { config, type RuvyxaConfig } from '../../../packages/@ruvyxa/core/src/config.ts'
import {
  definePlugin,
  withResponseHeader,
  type PluginHttpRequestHandler,
  type PluginHttpRequestRegistration,
  type PluginRegistrationApi,
} from '../../../packages/@ruvyxa/core/src/plugin.ts'

function registrationApi(
  onRequest: (value: PluginHttpRequestRegistration | PluginHttpRequestHandler) => void,
): PluginRegistrationApi {
  return {
    http: { onRequest, onResponse() {}, route() {} },
    build: {
      onStart() {},
      onResolve() {},
      onLoad() {},
      onTransform() {},
      onComplete() {},
    },
    dev: { onFileChange() {} },
    diagnostics: { report() {} },
    native: { claim() {} },
  }
}

describe('config and plugin APIs', () => {
  it('accepts grouped plugin sockets in application config', async () => {
    const authPlugin = definePlugin({
      name: 'auth',
      register({ http, build }) {
        http.onRequest({
          match: ['/api/*'],
          handler({ request }) {
            return request.headers.has('authorization')
              ? undefined
              : new Response('Unauthorized', { status: 401 })
          },
        })
        build.onTransform(({ code, id, environment }) =>
          environment === 'client' && id.endsWith('.tsx')
            ? { code: `${code}\n// transformed` }
            : undefined,
        )
        build.onComplete(({ root, outDir, manifest }) => {
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
          rate: { max: 100, window: 60, key: 'ip' },
          headers: { 'X-Powered-By': 'Ruvyxa' },
        },
      },
      plugins: [authPlugin],
      adapterOptions: { region: 'iad1' },
      build: { treeShake: false, manifest: true },
    }

    const defined = config(settings)
    assert.equal(defined.middleware?.builtin?.timing, true)
    assert.equal(defined.plugins?.[0]?.apiVersion, 2)
    assert.equal(defined.plugins?.[0]?.name, 'auth')

    let registered: PluginHttpRequestRegistration | PluginHttpRequestHandler | undefined
    await authPlugin.register(registrationApi((value) => (registered = value)))
    assert.deepEqual((registered as PluginHttpRequestRegistration).match, ['/api/*'])
  })

  it('rejects malformed definitions and stamps the sole API version', () => {
    assert.throws(() => definePlugin({ name: ' ', register() {} }), /must have a non-empty name/)
    assert.throws(() => definePlugin({ name: 'broken' } as never), /must provide register\(api\)/)
    assert.equal(definePlugin({ name: 'valid', register() {} }).apiVersion, 2)
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
})
