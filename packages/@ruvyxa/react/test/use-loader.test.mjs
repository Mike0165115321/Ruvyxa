import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { describe, it } from 'node:test'
import { fileURLToPath } from 'node:url'
import { format } from 'prettier'

const sourceFile = fileURLToPath(new URL('../src/use-loader.ts', import.meta.url))
const source = await readFile(sourceFile, 'utf8')

function assertLoaderLifecycleContract(candidate) {
  const code = candidate.replace(/\s+/g, ' ')

  assert.match(code, /const loaderRef = useRef\(loader\)/)
  assert.match(code, /loaderRef\s*\.\s*current\(\)/)
  assert.match(code, /\}, \[enabled\]\)/)
  assert.doesNotMatch(code, /\}, \[enabled, loader\]\)/)
}

function assertLoaderFailureContract(candidate) {
  const code = candidate.replace(/\s+/g, ' ')

  assert.match(
    code,
    /if \(!enabled\) \{ .*requestIdRef\.current\+\+.*setLoading\(false\).*return.*\}/,
  )
  assert.match(code, /Promise\.resolve\(\) \.then\(\(\) => loaderRef\.current\(\)\)/)
}

/**
 * React refuses a dependency array whose size changes between renders, so a
 * caller-supplied `deps` must never be spread into one. The refetch trigger has
 * to stay a fixed-shape array while still comparing every entry the way React
 * would.
 */
function assertFixedEffectDependencyContract(candidate) {
  const code = candidate.replace(/\s+/g, ' ')

  assert.doesNotMatch(code, /\[execute, \.\.\.deps\]/)
  assert.match(code, /\}, \[execute, depsVersion\]\)/)
  assert.match(code, /depsRef\.current\.length !== deps\.length/)
  assert.match(code, /!Object\.is\(value, deps\[index\]\)/)
  assert.match(code, /depsVersionRef\.current \+= 1/)
}

/**
 * Replay of the hook's render-phase dependency comparison. Guards the rule the
 * source contract above can only describe: equal entries must not bump the
 * version, and a length change must bump it instead of throwing.
 */
function replayDepsVersion(renders) {
  let current = renders[0] ?? []
  let version = 0
  const versions = []
  for (const deps of renders) {
    if (
      current.length !== deps.length ||
      current.some((value, index) => !Object.is(value, deps[index]))
    ) {
      current = deps
      version += 1
    }
    versions.push(version)
  }
  return versions
}

describe('useRuvyxaLoader dependency tracking', () => {
  it('keeps a fixed-shape effect dependency array', async () => {
    assertFixedEffectDependencyContract(source)
    assertFixedEffectDependencyContract(await format(source, { filepath: sourceFile }))
  })

  it('refetches on changed entries and on a changed deps length', () => {
    // Same values re-rendered: no refetch.
    assert.deepEqual(replayDepsVersion([['a'], ['a'], ['a']]), [0, 0, 0])
    // Changed value: one refetch.
    assert.deepEqual(replayDepsVersion([['a'], ['b'], ['b']]), [0, 1, 1])
    // Growing and shrinking lists are the case a spread array cannot express.
    assert.deepEqual(replayDepsVersion([['a'], ['a', 'b'], ['a', 'b'], ['a']]), [0, 1, 1, 2])
    // `NaN` follows `Object.is`, matching React's own comparison.
    assert.deepEqual(replayDepsVersion([[Number.NaN], [Number.NaN]]), [0, 0])
  })
})

describe('useRuvyxaLoader request lifecycle', () => {
  it('keeps inline loaders out of automatic refetch dependencies after formatting', async () => {
    assertLoaderLifecycleContract(source)
    assertLoaderLifecycleContract(await format(source, { filepath: sourceFile }))
  })

  it('invalidates disabled requests and normalizes synchronous loader failures', async () => {
    assertLoaderFailureContract(source)
    assertLoaderFailureContract(await format(source, { filepath: sourceFile }))
  })
})
