# CLI reference

The verified command surface is `dev`, `build`, `check`, `start`, `preview`, `routes`, `analyze`,
`add`, `doctor`, `clean`, `trace`, `bench`, `test:parity`, and `plugin create`. Run
`ruvyxa <command> --help` for the installed version's complete flags.

| Command                                                                                             | Purpose                                                                              |
| --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `ruvyxa dev [--root .] [--host H] [--port P] [--runtime node\|bun]`                                 | Route watching and hot reload.                                                       |
| `ruvyxa build [--root .] [--target node\|bun\|edge\|static] [--adapter NAME] [--runtime node\|bun]` | Production output.                                                                   |
| `ruvyxa check` / `analyze`                                                                          | App readiness; route/import/boundary analysis.                                       |
| `ruvyxa start` / `preview`                                                                          | Serve or locally preview an existing build.                                          |
| `ruvyxa routes [--json]` / `trace`                                                                  | Route table/one manifest entry.                                                      |
| `ruvyxa doctor`, `clean`, `bench`, `test:parity`                                                    | Diagnose setup, remove output, benchmark, compare dev/prod routes and smoke renders. |
| `ruvyxa add`, `ruvyxa plugin create`                                                                | Scaffold supported application flows or a publishable plugin.                        |

## Recommended local loop

```bash
pnpm dev
pnpm routes
pnpm check
pnpm build
pnpm test:parity
```

Use `--root examples/demo` when running from this monorepo root against the broad fixture. `clean`
removes generated Ruvyxa build output; do not run it against a path that contains manually
maintained artifacts. `analyze --html` has a matching project script and produces an HTML analysis
view.

## Repository scripts

The root `package.json` defines `build`, `check`, `test`, `prepare`, `check:cargo-lock`,
`check:oxc-lockstep`, `format`, `format:check`, `format:staged`, `release:validate`, `release:bump`,
`pack:smoke`, `test:full-flow`, and `publish:dry-run`. Published TypeScript packages consistently
define `build`, `check`, `test`, `format`, and `prepack`; consult the relevant package manifest for
its test glob.

**Previous:** [Integrations](09-integrations-auth-data-and-realtime.md) · **Next:**
[Architecture](11-architecture.md)
