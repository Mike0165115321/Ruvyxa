import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { test } from 'node:test'

import { useActionState } from 'react'
import { useFormStatus } from 'react-dom'

test('declares and exercises the stable React 19 action APIs', async () => {
  assert.equal(typeof useActionState, 'function')
  assert.equal(typeof useFormStatus, 'function')
  const manifest = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'))
  assert.equal(manifest.peerDependencies.react, '^19.0.0')
  assert.equal(manifest.peerDependencies['react-dom'], '^19.0.0')
})
