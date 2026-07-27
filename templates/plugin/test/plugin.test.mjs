import assert from 'node:assert/strict'
import test from 'node:test'

import plugin from '../dist/index.js'

test('exports a Ruvyxa plugin and registers its HTTP socket', async () => {
  let responseHook
  await plugin.register({
    http: {
      onRequest() {},
      onResponse(value) {
        responseHook = value
      },
      route() {},
    },
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
  })

  assert.equal(plugin.name, 'request-logger')
  assert.equal(plugin.name, '__PLUGIN_NAME__')
  assert.deepEqual(responseHook.match, ['/*'])
  const response = await responseHook.handler({ response: new Response('ok') })
  assert.equal(response.headers.get('x-__PLUGIN_NAME__'), 'active')
})
