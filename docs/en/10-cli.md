# CLI and application scripts

The root [README](../../README.md) is the authoritative project overview. In a generated Ruvyxa
application, use the npm scripts below. They are the stable, copy-pasteable interface provided by
every starter; in particular, use `routes:json` and `analyze:html` rather than teaching readers to
reconstruct the flags behind those scripts.

| Application command                                                                                                                   | Runs                                  | Purpose                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------- |
| `npm run dev`                                                                                                                         | `ruvyxa dev`                          | Route watching and hot reload.                                                         |
| `npm run build`                                                                                                                       | `ruvyxa build`                        | Production output.                                                                     |
| `npm run check`                                                                                                                       | `ruvyxa check`                        | Application readiness checks.                                                          |
| `npm run start` / `npm run preview`                                                                                                   | `ruvyxa start` / `preview`            | Serve or locally preview an existing build.                                            |
| `npm run routes`                                                                                                                      | `ruvyxa routes`                       | Human-readable route table.                                                            |
| `npm run routes:json`                                                                                                                 | Starter-defined route JSON command    | Machine-readable route output.                                                         |
| `npm run analyze`                                                                                                                     | `ruvyxa analyze`                      | Validate routes, imports, and server/client boundaries.                                |
| `npm run analyze:html`                                                                                                                | Starter-defined HTML analysis command | Interactive local analysis page.                                                       |
| `npm run add -- form`                                                                                                                 | `ruvyxa add form`                     | Scaffold a supported application flow.                                                 |
| `npm run doctor`, `npm run clean`, `npm run trace -- /`, `npm run bench`, `npm run test:parity`, `npm run plugin -- create my-plugin` | Matching `ruvyxa` command             | Diagnose, clean output, inspect a route, benchmark, verify parity, or create a plugin. |

## Recommended application loop

Run this from the root of a generated application, not from this framework monorepo:

```bash
npm run dev
npm run routes
npm run check
npm run build
npm run test:parity
```

Use `npm run routes:json` only when another tool needs structured route data; open the report from
`npm run analyze:html` when investigating bundle, route, import, or boundary findings. `clean`
removes generated Ruvyxa build output, so do not run it against a path containing manually
maintained artifacts.

## Running the framework CLI from this monorepo

This repository root deliberately has workspace scripts such as `pnpm build`, `pnpm check`, and
`pnpm test`, but it does **not** define application scripts such as `npm run dev` or
`npm run routes`. To exercise the broad fixture from the repository root, invoke the CLI through
Cargo and name the fixture explicitly:

```bash
cargo run -p ruvyxa_cli -- dev --root examples/demo
cargo run -p ruvyxa_cli -- routes --root examples/demo
cargo run -p ruvyxa_cli -- check --root examples/demo
```

Run `cargo run -p ruvyxa_cli -- <command> --help` when maintaining the framework itself. The checked
CLI exposes `dev`, `build`, `check`, `start`, `preview`, `routes`, `analyze`, `add`, `doctor`,
`clean`, `trace`, `bench`, `test:parity`, and `plugin create`.

## Repository scripts

The root `package.json` defines `build`, `check`, `test`, `prepare`, `check:cargo-lock`,
`check:oxc-lockstep`, `format`, `format:check`, `format:staged`, `release:validate`, `release:bump`,
`pack:smoke`, `test:full-flow`, and `publish:dry-run`. Published TypeScript packages consistently
define `build`, `check`, `test`, `format`, and `prepack`; consult the relevant package manifest for
its test glob.

**Previous:** [Integrations](09-integrations-auth-data-and-realtime.md) · **Next:**
[Architecture](11-architecture.md)
