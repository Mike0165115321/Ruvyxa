#!/usr/bin/env node
// Ruvyxa compiles every project twice with oxc: `ruvyxa_bundler` (Rust) emits
// the client bundle, and `packages/ruvyxa/runtime/compiler.mjs` (Node, via
// `oxc-transform`) emits the server bundle. Both halves read the same source
// files, so a version split lets one page's server render and client hydration
// disagree — which surfaces as a hydration mismatch far from its cause instead
// of as a build error.
//
// Nothing in cargo or npm relates these two packages, so this check is the only
// thing holding them together. It fails the build the moment they drift.
import { readFileSync } from 'node:fs'

const CARGO_LOCK = 'Cargo.lock'
const CARGO_TOML = 'Cargo.toml'
const RUNTIME_PKG = 'packages/ruvyxa/package.json'

const failures = []

const lockedRustVersion = readCargoLockVersion(CARGO_LOCK, 'oxc')
const pinnedRustRequirement = readWorkspaceOxcRequirement(CARGO_TOML)
const runtimePkg = JSON.parse(readFileSync(RUNTIME_PKG, 'utf8'))
const nodeRequirement = runtimePkg.dependencies?.['oxc-transform']

if (!lockedRustVersion) {
  failures.push(`${CARGO_LOCK} has no resolved \`oxc\` package`)
}
if (!pinnedRustRequirement) {
  failures.push(`${CARGO_TOML} has no \`oxc\` workspace dependency`)
}
if (!nodeRequirement) {
  failures.push(`${RUNTIME_PKG} has no \`oxc-transform\` dependency`)
}

// An exact pin on both sides is what makes the comparison below meaningful. A
// range would let a routine `cargo update` or `pnpm install` reintroduce the
// split without touching a tracked file, so the check has to reject ranges
// rather than resolve them.
if (pinnedRustRequirement && !pinnedRustRequirement.startsWith('=')) {
  failures.push(
    `${CARGO_TOML} must pin oxc exactly (found "${pinnedRustRequirement}", expected "=${lockedRustVersion ?? 'x.y.z'}")`,
  )
}
if (nodeRequirement && !/^\d+\.\d+\.\d+$/.test(nodeRequirement)) {
  failures.push(
    `${RUNTIME_PKG} must pin oxc-transform exactly (found "${nodeRequirement}", expected a bare "x.y.z")`,
  )
}

const pinnedRustVersion = pinnedRustRequirement?.replace(/^=/, '')
if (pinnedRustVersion && lockedRustVersion && pinnedRustVersion !== lockedRustVersion) {
  failures.push(
    `${CARGO_TOML} pins oxc ${pinnedRustVersion} but ${CARGO_LOCK} resolved ${lockedRustVersion}. Run: cargo update -p oxc --precise ${pinnedRustVersion}`,
  )
}

if (lockedRustVersion && nodeRequirement && lockedRustVersion !== nodeRequirement) {
  failures.push(
    `oxc version split: Rust bundler uses ${lockedRustVersion}, Node runtime uses oxc-transform ${nodeRequirement}.\n` +
      `  The two compile the same sources for one page. Align them, then update both lockfiles:\n` +
      `    cargo update -p oxc --precise <version>\n` +
      `    pnpm install`,
  )
}

if (failures.length > 0) {
  console.error(failures.map((failure) => `- ${failure}`).join('\n'))
  process.exit(1)
}

console.log(`oxc is in lockstep at ${lockedRustVersion} (Rust bundler and Node runtime).`)

/**
 * Read a resolved version out of Cargo.lock.
 *
 * Matched per `[[package]]` block rather than by scanning for the name alone:
 * `oxc` shares a prefix with `oxc_parser`, `oxc_span`, and two dozen siblings,
 * so a looser match would silently report the wrong crate's version.
 */
function readCargoLockVersion(file, packageName) {
  const lock = readFileSync(file, 'utf8')
  for (const block of lock.split('[[package]]')) {
    const name = block.match(/^\s*name\s*=\s*"([^"]+)"/m)?.[1]
    if (name !== packageName) continue
    return block.match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1] ?? null
  }
  return null
}

/** Read the `version = "..."` field of the workspace `oxc` dependency. */
function readWorkspaceOxcRequirement(file) {
  const manifest = readFileSync(file, 'utf8')
  const line = manifest.match(/^oxc\s*=\s*(.+)$/m)?.[1]
  if (!line) return null
  return line.match(/version\s*=\s*"([^"]+)"/)?.[1] ?? null
}
