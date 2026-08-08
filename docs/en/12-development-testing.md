# Development and testing

> **Tutorial goal:** set up a contributor loop and choose the smallest test that proves a change.
> **Start from:** the boundary map in [Architecture](11-architecture.md). **Checkpoint:** run the
> narrowest relevant check before choosing a broader repository gate.

## Framework contributor setup

This is a Rust workspace plus pnpm workspace. Install the declared Node version and pnpm, then use a
Rust toolchain compatible with the locked workspace.

```bash
pnpm install
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --locked -- -D warnings
pnpm -r build
pnpm -r check
pnpm -r test
pnpm format:check
```

For the broad fixture, use the exact commands established by the repository guide:

```bash
cargo run -p ruvyxa_cli -- check --root examples/demo
cargo run -p ruvyxa_cli -- test:parity --root examples/demo
```

## Test layers

Rust tests live with the relevant crate and cover CLI/graph/bundler/server behavior. Package tests
run through Node's built-in test runner; package manifests point at `tests/packages/**` or
package-local tests. `@ruvyxa/react` has tests for the client router. `@ruvyxa/testing` lets a unit
test create a loader/action/cache double and inspect its calls and invalidations.

```ts
import test from 'node:test'
import assert from 'node:assert/strict'
import { mockAction } from '@ruvyxa/testing'

test('records invalidation', async () => {
  const save = mockAction(({ input, invalidate }) => {
    invalidate('todos')
    return input
  })
  await save({ title: 'Write docs' })
  assert.deepEqual(save.invalidations, ['todos'])
})
```

The current repository has CI workflows at `.github/workflows/ci.yml` and
`.github/workflows/release.yml`. Do not claim an individual job's exact command without reading the
workflow at the revision you are changing; workflows can evolve independently of package scripts.

## Definition of done

For a public framework change, update the Rust/TypeScript contract, tests, templates where
applicable, and both language editions in `docs/`. Run the narrowest relevant test during iteration,
then the broader checks above before handoff. Do not commit generated `.ruvyxa/`, `dist/`,
`target/`, `node_modules/`, or package smoke directories.

**Previous:** [Architecture](11-architecture.md) · **Next:** [Security](13-security.md)
