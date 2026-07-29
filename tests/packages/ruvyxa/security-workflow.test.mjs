import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { describe, it } from 'node:test'

const workflow = readFileSync(
  new URL('../../../.github/workflows/security.yml', import.meta.url),
  'utf8',
)

describe('dependency security workflow', () => {
  it('audits both Rust and JavaScript lockfiles', () => {
    assert.match(workflow, /rustsec\/audit-check@v2\.0\.0/)
    assert.match(workflow, /pnpm audit --audit-level low/)
    assert.match(workflow, /Cargo\.lock/)
    assert.match(workflow, /pnpm-lock\.yaml/)
  })

  it('runs on a schedule and on lockfile changes with read-only repository access', () => {
    assert.match(workflow, /schedule:/)
    assert.match(workflow, /workflow_dispatch:/)
    assert.match(workflow, /permissions:\s+contents: read/)
    assert.match(workflow, /persist-credentials: false/)
  })
})
