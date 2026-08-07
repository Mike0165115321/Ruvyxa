/**
 * Typed routes, checked by the compiler that has to agree with them.
 *
 * These assertions cannot be made in a runtime test: `RouteHref` erases
 * completely, so the only observable behaviour is which programs `tsc` accepts.
 * Each fixture uses `@ts-expect-error` for the cases that must fail, which
 * makes "zero diagnostics" the single assertion for both directions — a
 * rejection that stopped being rejected turns the unused directive into an
 * error of its own.
 */

import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import test from 'node:test'

const here = dirname(fileURLToPath(import.meta.url))
// `typescript`'s `exports` map does not expose `./bin/tsc`, so the launcher is
// located from the package entry point rather than resolved as a subpath.
const tsc = join(dirname(createRequire(import.meta.url).resolve('typescript')), '..', 'bin', 'tsc')

function typecheck(fixture) {
  try {
    execFileSync(process.execPath, [tsc, '-p', join(here, 'fixtures', fixture)], {
      encoding: 'utf8',
      stdio: 'pipe',
    })
    return ''
  } catch (error) {
    return `${error.stdout ?? ''}${error.stderr ?? ''}`.trim()
  }
}

test('a generated registry narrows RouteHref to the real routes', () => {
  assert.equal(
    typecheck('typed-routes'),
    '',
    'every accepted href in narrowed.ts must compile, and every @ts-expect-error must fire',
  )
})

test('RouteHref stays `string` with no generated registry', () => {
  assert.equal(
    typecheck('untyped-routes'),
    '',
    'projects that never opt in must type-check exactly as they did before',
  )
})
