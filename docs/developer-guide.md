# Ruvyxa Developer Guide

**Audience**: framework contributors changing Rust crates, npm packages, adapters, templates, or
runtime.

**Application authors**: start at [User Guide](guides/en/01-getting-started.md).

---

## 1. Setup

```bash
# Requirements
node --version  # ≥ 22.12
pnpm --version  # ≥ 11
rustc --version # ≥ 1.96 (edition 2024)

# Install
.\setup.bat        # Windows
./setup.sh         # macOS/Linux

# Verify
cargo run -p ruvyxa_cli -- doctor --root examples/demo
cargo run -p ruvyxa_cli -- routes --root examples/demo
```

Setup installs locked deps, builds workspace packages, compiles CLI.

**Never commit**: `target/`, `node_modules/`, `.ruvyxa/`, `dist/`, `.npm-pack/`, `.npm-smoke/`.

---

## 2. Repo Map

```
packages/ruvyxa/bin/ruvyxa.js → platform-specific CLI binary
  └─ crates/
       ├── ruvyxa_cli          Commands, config loading, build orchestration
       ├── ruvyxa_graph        Route discovery, render detection, validation
       ├── ruvyxa_bundler      TS/JSX/MDX compilation, Oxc transforms, resolution, linking, minification
       ├── ruvyxa_dev_server   Axum server, HMR, router, cache, Node/Bun worker pool, styles
       ├── ruvyxa_middleware   Tower middleware + plugin bridge
       └── ruvyxa_diagnostics  Structured RUV#### diagnostics

packages/
  ├── ruvyxa                   Re-exports @ruvyxa/core + runtime scripts
  ├── @ruvyxa/core             Config types, server APIs, adapter contracts
  ├── @ruvyxa/react            Image, SEO, hydration, loaders, error boundaries
  ├── @ruvyxa/auth             Sessions, OAuth, magic-link, WebAuthn
  ├── @ruvyxa/database         Typed CRUD with Prisma/DynamoDB/custom adapters
  ├── @ruvyxa/realtime         Action-driven WebSocket transport
  ├── @ruvyxa/adapter-*        10 platform adapters (vercel, netlify, cloudflare, node, bun, static, …)
  ├── @ruvyxa/cli-*            5 platform binaries (darwin-arm64, linux-arm64, linux-x64, win32-arm64, win32-x64)
  └── create-ruvyxa            Scaffold CLI + template packaging
```

---

## 3. Working Loop

```bash
# Narrowest check first, expand only when shared behavior changes
cargo test -p ruvyxa_graph --locked
cargo test -p ruvyxa_cli --locked
pnpm --filter ruvyxa start
pnpm --filter ruvyxa check

# E2E signal via demo
cargo run -p ruvyxa_cli -- analyze --root examples/demo
cargo run -p ruvyxa_cli -- check --root examples/demo
cargo run -p ruvyxa_cli -- test:parity --root examples/demo
```

**Full suite before handoff**:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --locked -- -D warnings
pnpm -r build
pnpm -r check
pnpm -r test
pnpm format:check
pnpm release:validate
pnpm pack:smoke
```

---

## 4. Change Guide

| Change                           | Primary Surface                         | Minimum Proof                     |
| -------------------------------- | --------------------------------------- | --------------------------------- |
| CLI command, config parsing      | `crates/ruvyxa_cli/src/main.rs`         | Rust test + demo `check`          |
| Route matching, validation       | `crates/ruvyxa_graph/src/lib.rs`        | Graph test + `routes`/`analyze`   |
| Compilation, linking, transforms | `crates/ruvyxa_bundler`                 | Bundler tests + demo build        |
| CSS, style HMR                   | `crates/ruvyxa_dev_server/src/style.rs` | Crate tests + demo build          |
| API/action/HMR/server            | `crates/ruvyxa_dev_server`              | Crate tests + parity              |
| Core config/server API           | `packages/@ruvyxa/core/src`             | Package test/check                |
| npm launcher/runtime             | `packages/ruvyxa`                       | Package test + `pnpm pack:smoke`  |
| Template/starter                 | `templates/minimal`, `create-ruvyxa`    | Create test + pack smoke          |
| Cross-cutting app behavior       | `examples/demo`                         | `analyze`, `check`, `test:parity` |

### Config Field Lifecycle

1. Add type + docs in `packages/@ruvyxa/core`
2. Add matching Rust field (`camelCase`)
3. Validate in Rust (unsafe/impossible values)
4. Wire to dev + production paths
5. Add tests for accepted/rejected values
6. Update user guide if app-visible

Unknown config keys **fail** — never silently ignored.

### Rendering Detection Order (preserve)

1. `'use client'` directive → CSR
2. `export const ppr = true` → PPR
3. `export const revalidate = <n>` → ISR
4. `getStaticParams` / `staticParams` → SSG
5. Static route (no dynamic markers) → SSG
6. No match → SSR (default)

### Server/Client Boundary (preserve)

| Rule                             | Code    | Severity   |
| -------------------------------- | ------- | ---------- |
| `"server-only"` in client bundle | RUV1007 | Error      |
| Private `process.env` in client  | RUV1008 | Error      |
| `"client-only"` in SSR bundle    | RUV1009 | Warning    |
| `server/` dir in client graph    | RUV1010 | Error      |
| Only `RUVYXA_PUBLIC_*` in client | —       | Convention |

---

## 5. Packaging Rules

- Tarballs must NOT contain tests or `workspace:` dependencies
- Must include every runtime script, template, platform binary, launcher
- npm strips `.gitignore` from tarballs → `create-ruvyxa` renames to `gitignore` → scaffold restores
  as `.gitignore`
- `ruvyxa/bin/ruvyxa.js` must be `100755` in Git and tarball (Vercel requirement)
- Verify with: `git ls-files --stage packages/ruvyxa/bin/ruvyxa.js` + `pnpm pack:smoke`

---

## 6. Release Order

`pnpm release:bump <version>` syncs all workspace packages + crates + starter deps to one version.

Publish order (from `.github/workflows/release.yml`):

1. Native CLI packages (`@ruvyxa/cli-*`)
2. Shared JS packages (`@ruvyxa/core`, `@ruvyxa/react`, …)
3. All adapters
4. `ruvyxa`
5. `create-ruvyxa`

Adding an adapter = update the workflow in the same change.

---

## 7. Diagnostics (RUV####)

New diagnostic needs:

1. Code in appropriate range
2. Concise title
3. Explanation of violated contract
4. File location when known
5. Concrete suggested fix
6. Tests for the diagnostic
7. English guide update if user-facing

---

## 8. Templates

Source starters: `templates/minimal/`, `templates/blog/`, `templates/crud/`,
`templates/api-backend/`

`packages/create-ruvyxa/scripts/prepare-template.mjs` copies all four into package before packing.

Keep starter scripts consistent: `dev`, `build`, `start`, `check`.

---

## 9. Demo as Integration Fixture

`examples/demo` exercises static, dynamic, catch-all, API, action, MDX, env, style, and
rendering-strategy paths.

```bash
pnpm --dir examples/demo doctor      # Setup check
pnpm --dir examples/demo routes      # Route table
pnpm --dir examples/demo analyze     # Route/import/boundary problems
pnpm --dir examples/demo check       # Typecheck + build + parity
pnpm --dir examples/demo trace /blog # Single route inspection
```

Use `analyze` first for route/import/boundary problems, `check` for full readiness.

---

## 10. Known Boundaries

- Document only what source code and tests support
- Rendering strategy = source scanning with documented precedence (not a runtime toggle)
- Config paths restricted to project root (prevents traversal)
- Adapter `build()` runs within staging before atomic commit
- `check` is application-readiness signal, not E2E/load/security audit
- If Windows locks `target/debug/ruvyxa.exe` → stop dev server, don't delete `target/`

---

## 11. Documentation Evidence Workflow

When changing a user-visible behavior, update the documentation that owns the contract, not an
unrelated overview. Use the implementation boundary to choose the document:

| Changed behavior                                | Inspect first                                                   | Update first                                         |
| ----------------------------------------------- | --------------------------------------------------------------- | ---------------------------------------------------- |
| Route convention, manifest, or render detection | `crates/ruvyxa_graph/src/lib.rs` and graph tests                | routing/rendering guides and `architecture/graph.md` |
| CLI command or flag                             | `crates/ruvyxa_cli/src/main.rs` and live `--help`               | CLI guide and `architecture/cli.md`                  |
| Config field                                    | `packages/@ruvyxa/core/src/types.ts` plus CLI config validation | configuration guide                                  |
| Bundling/resolution/boundary rule               | `crates/ruvyxa_bundler/src` and parser tests                    | bundler architecture and affected guide              |
| Server behavior/HMR/security                    | `crates/ruvyxa_dev_server/src`                                  | dev-server/security/protocol docs                    |
| Public package export                           | package `src/index.ts` and package tests                        | API reference and package guide                      |

For a documentation-only change, verify Markdown formatting and links. For a factual change tied to
runtime behavior, also run the narrowest owning check. Never create an example around an inferred
flag, an imagined generated file, or a planned plugin capability; label a proposal as proposed until
the code and tests exist.
