/**
 * Keep files that exist byte-identically in more than one tracked tree in sync.
 *
 * `templates/minimal/` is copied into every new project by `create-ruvyxa`, and
 * `examples/demo/` is the broad integration fixture. A few files are deliberately
 * the same in both: the demo exercises exactly what a scaffolded app ships with,
 * so a component that behaves one way in the fixture and another way in the
 * template would make the fixture stop testing the thing users receive.
 *
 * Nothing enforced that. The pairs below were kept aligned by editing both copies
 * in the same commit, by hand, five commits in a row — and a real defect (one
 * projectile scoring a hit on two targets in the same frame) lived in both copies
 * because the duplication carried it. `pnpm release:validate` runs this so the
 * next divergence fails a check instead of shipping two versions of one file.
 *
 * Adding a pair here is a deliberate statement that the two paths must not
 * diverge. Files that are *supposed* to differ between the template and the demo
 * — config, layout, page content, package manifests — must stay out of this list.
 *
 * Usage:
 *   node scripts/check-template-mirrors.mjs           copy each source over its mirror
 *   node scripts/check-template-mirrors.mjs --check   fail if any mirror has drifted
 */

import { copyFileSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)))
const checkOnly = process.argv.includes('--check')

/**
 * `source` is the authority. `templates/minimal` wins over `examples/demo`
 * because the template is the artifact users actually receive; the fixture
 * follows it.
 */
const MIRRORS = [
  {
    source: 'templates/minimal/app/components/ruvyxa-runner.tsx',
    mirror: 'examples/demo/app/components/ruvyxa-runner.tsx',
  },
]

function read(relativePath) {
  try {
    return readFileSync(path.join(repoRoot, relativePath))
  } catch {
    return null
  }
}

let drifted = 0

for (const { source, mirror } of MIRRORS) {
  const sourceBytes = read(source)
  if (sourceBytes === null) {
    console.error(`Mirror source is missing: ${source}`)
    process.exitCode = 1
    continue
  }

  const mirrorBytes = read(mirror)
  if (mirrorBytes !== null && mirrorBytes.equals(sourceBytes)) continue

  if (!checkOnly) {
    copyFileSync(path.join(repoRoot, source), path.join(repoRoot, mirror))
    console.log(`synced ${mirror} from ${source}`)
    continue
  }

  drifted += 1
  console.error(
    `${mirror} ${mirrorBytes === null ? 'is missing' : 'has drifted from'} ${source}.\n\n` +
      'These two paths are required to hold identical bytes so the demo fixture\n' +
      'exercises the same code a scaffolded project receives.\n\n' +
      'Copy the authoritative file over the mirror:\n\n' +
      '  node scripts/check-template-mirrors.mjs\n',
  )
}

if (checkOnly) {
  if (drifted > 0) process.exit(1)
  console.log(`template mirrors are in sync (${MIRRORS.length} pair(s))`)
}
