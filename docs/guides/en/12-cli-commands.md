# CLI Commands

The `ruvyxa` package installs the Ruvyxa CLI. Its command surface is defined in
`crates/ruvyxa_cli/src/main.rs`; the command help shown by the installed binary is the authoritative
source for flags. Command signatures below name the underlying CLI; in an application, use the
matching `npm run` script shown in each example.

```bash
npm run
```

> Flags are command-specific. `--root` and `--runtime` are not global flags, and the CLI does not
> define `--verbose`, `--no-color`, `--open`, or build-analysis flags.

## Command Overview

| Command         | Purpose                                                                      |
| --------------- | ---------------------------------------------------------------------------- |
| `dev`           | Start the development server with route watching and HMR.                    |
| `build`         | Produce a production build and optionally run a deployment adapter.          |
| `check`         | Run TypeScript checking when `tsconfig.json` exists, then run `test:parity`. |
| `start`         | Serve an existing production build.                                          |
| `preview`       | Serve an existing production build for local preview.                        |
| `routes`        | Print the discovered route table.                                            |
| `analyze`       | Validate routes, imports, and server/client boundaries.                      |
| `add`           | Scaffold framework-native forms, data tables, or authentication flows.       |
| `doctor`        | Inspect project setup, dependencies, runtime, and optionally an adapter.     |
| `clean`         | Remove the configured generated build output.                                |
| `trace`         | Print one discovered route-manifest entry.                                   |
| `bench`         | Benchmark route discovery, analysis, and production build work.              |
| `test:parity`   | Compare dev and production route behavior and smoke-render page routes.      |
| `plugin create` | Scaffold a publishable plugin package.                                       |

`test:parity` also accepts the `parity` alias.

## Shared Command Options

Commands that take a project root expose `--root <PATH>` and default it to the current directory.
The following commands also accept `--runtime <node|bun>`: `dev`, `build`, `check`, `start`,
`preview`, `routes`, `analyze`, `doctor`, `clean`, and `test:parity`. For the commands that accept
it, this override has priority over `RUVYXA_RUNTIME` and `config.runtime`.

Every command supports `-h` / `--help`.

## `ruvyxa dev`

```bash
ruvyxa dev [--root <PATH>] [--host <HOST>] [--port <PORT>] [--runtime <node|bun>]
```

Use it while developing an application. Host and port are optional; the final defaults come from the
project configuration and server configuration.

```bash
npm run dev
npm run dev -- --port 4000
npm run dev -- --root ../other-app --runtime bun
```

## `ruvyxa build`

```bash
ruvyxa build [--root <PATH>] [--target <node|bun|edge|static>] \
  [--adapter <NAME_OR_NPM_PACKAGE>] [--runtime <node|bun>]
```

`--target` overrides the build target. `--adapter` selects a deployment adapter without editing
`ruvyxa.config.ts`; the built-in names are `node`, `bun`, `static`, `vercel`, `netlify`,
`cloudflare`, `railway`, `render`, `firebase`, and `aws`. A syntactically valid npm package name is
also accepted and resolved by the adapter runner.

```bash
npm run build
npm run build -- --target static
npm run build -- --adapter vercel
```

The output directory is configured by `outDir` (the starter uses `.ruvyxa`). For the detailed build
stages and generated artifacts, see [CLI Architecture](../../architecture/cli.md) and
[Deployment](./13-deployment.md).

## `ruvyxa check`

```bash
ruvyxa check [--root <PATH>] [--runtime <node|bun>]
```

This is the project readiness command. It runs `tsc --noEmit` when the project contains a
`tsconfig.json`, then runs the same parity flow as `ruvyxa test:parity`.

```bash
npm run check
npm run check -- --runtime bun
```

## `ruvyxa start` and `ruvyxa preview`

```bash
ruvyxa start [--root <PATH>] [--host <HOST>] [--port <PORT>] [--runtime <node|bun>]
ruvyxa preview [--root <PATH>] [--host <HOST>] [--port <PORT>] [--runtime <node|bun>]
```

Both commands serve an existing production build; run `ruvyxa build` first. `preview` is a separate
command for the local-preview workflow, not a command that builds automatically.

```bash
npm run build
npm run start
npm run preview -- --port 4173
```

## `ruvyxa routes`

```bash
ruvyxa routes [--root <PATH>] [--runtime <node|bun>]
```

Print the routes discovered from the configured application directory. Use it to inspect route paths
before building or to investigate a route conflict.

```bash
npm run routes
```

## `ruvyxa analyze`

```bash
ruvyxa analyze [--root <PATH>] [--runtime <node|bun>] \
  [--format <auto|human|json|sarif>] [--output <PATH>]
```

The command validates routes, imports, and server/client boundaries. `auto` preserves the terminal
or piped-output behavior; `--output` writes the selected report to a file.

```bash
npm run analyze
npm run analyze -- --format sarif --output reports/ruvyxa.sarif
```

## `ruvyxa doctor`

```bash
ruvyxa doctor [--root <PATH>] [--target <node|bun|edge|static>] \
  [--adapter <NAME_OR_NPM_PACKAGE>] [--runtime <node|bun>] [--json]
```

Use `--adapter` to inspect an adapter without writing its artifacts. `--json` emits the complete
compatibility report as JSON.

```bash
npm run doctor
npm run doctor -- --adapter cloudflare --json
```

## `ruvyxa clean`

```bash
ruvyxa clean [--root <PATH>] [--runtime <node|bun>]
```

Removes the configured generated build directory. It is intentionally scoped to Ruvyxa output, not
to dependencies or arbitrary project files.

```bash
npm run clean
```

## `ruvyxa trace`

```bash
ruvyxa trace <ROUTE> [--root <PATH>]
```

`<ROUTE>` is required. The command discovers the current manifest and prints the matching entry.

```bash
npm run trace -- /
npm run trace -- /blog/[slug]
```

## `ruvyxa bench`

```bash
ruvyxa bench [--root <PATH>] [--samples <COUNT>] [--json]
```

The default sample count is `3`. Use JSON when a CI job or another tool needs to consume the
benchmark result.

```bash
npm run bench -- --samples 5
npm run bench -- --json
```

## `ruvyxa test:parity`

```bash
ruvyxa test:parity [--root <PATH>] [--runtime <node|bun>]
```

This compares dev and production route manifests and smoke-renders page routes. It is the parity
check used by `ruvyxa check`.

```bash
npm run test:parity
npm run test:parity
```

## `ruvyxa plugin create`

```bash
ruvyxa plugin create <NAME> [--root <PATH>] [--dir <PATH>]
```

`<NAME>` is required. `--dir` is relative to `--root`; without it, the CLI creates the package in a
directory named after the plugin.

```bash
npm run plugin -- create my-plugin
npm run plugin -- create @acme/analytics --dir packages/analytics
```

See [Plugins](./14-plugins.md) for the TypeScript plugin contract.

## Starter Scripts

Every application starter defines these scripts:

```json
{
  "dev": "ruvyxa dev",
  "build": "ruvyxa build",
  "start": "ruvyxa start",
  "preview": "ruvyxa preview",
  "typecheck": "tsc --noEmit",
  "check": "ruvyxa check",
  "routes": "ruvyxa routes",
  "routes:json": "ruvyxa routes --json",
  "analyze": "ruvyxa analyze",
  "analyze:html": "ruvyxa analyze --html",
  "add": "ruvyxa add",
  "doctor": "ruvyxa doctor",
  "clean": "ruvyxa clean",
  "trace": "ruvyxa trace",
  "bench": "ruvyxa bench",
  "test:parity": "ruvyxa test:parity",
  "plugin": "ruvyxa plugin"
}
```

Use `npm run <script>` from the application directory. Put command arguments after `--` so npm
forwards them to Ruvyxa: `npm run analyze -- --format json`.

## Practical Recipes

The recipes below use the starter package scripts. Replace `../my-app` with the path to your project
when needed.

### Start a Project on a Different Port

Use this when another local service already owns your usual port:

```bash
npm run dev -- --port 4000
```

To make the server reachable from another device on your network, choose a host explicitly:

```bash
npm run dev -- --host 0.0.0.0 --port 4000
```

Keep the terminal open while developing; HMR and route watching run in that process.

### Work on Another Checkout

`--root` makes it unnecessary to change directories before using the CLI:

```bash
npm run dev -- --root ../my-app
npm run routes -- --root ../my-app
npm run analyze -- --root ../my-app --format human
```

The root is resolved per command, so pass it to every command in a script rather than assuming a
previous invocation changes the shell's current directory.

### Use Bun for a Command

When a project is configured for Node but you need to test it with Bun, use the per-command runtime
override:

```bash
npm run dev -- --runtime bun
npm run build -- --runtime bun
npm run check -- --runtime bun
```

The override applies only to that invocation. It does not edit `ruvyxa.config.ts` or the
environment.

### Build for a Local Node Server

The default production workflow builds first and then serves the generated output:

```bash
npm run build
npm run start
```

To be explicit about the output target, run:

```bash
npm run build -- --target node
npm run start -- --port 3001
```

`start` and `preview` read an existing build; neither command performs a build implicitly.

### Preview a Static Build

Use the static target only when the application and selected adapter support the routes being built:

```bash
npm run build -- --target static --adapter static
npm run preview -- --port 4173
```

If a route strategy is not supported by the platform adapter, the build reports that
incompatibility. See [Deployment](./13-deployment.md) for adapter-node output and hosting steps.

### Build for a Hosting Adapter Without Editing Config

`--adapter` is useful in CI or when evaluating a deployment target temporarily:

```bash
npm run build -- --adapter vercel
npm run build -- --adapter cloudflare
npm run build -- --adapter @acme/ruvyxa-adapter-node
```

The last form is a package name; it must be resolvable by the adapter runner in the project
environment.

### Run the Pre-commit Readiness Check

The shortest repeatable gate for an application is:

```bash
npm run check
```

It type-checks with `tsc --noEmit` when the project contains `tsconfig.json`, then runs the route
and render parity flow. To run just the parity portion:

```bash
npm run test:parity
# equivalent alias
npm run test:parity
```

### Inspect Routes Before Debugging a URL

First list the discovered URL table, then inspect the route that matters:

```bash
npm run routes
npm run trace -- /about
npm run trace -- /blog/[slug]
```

`trace` takes the route pattern, not a source filename. For a dynamic page, use `/blog/[slug]`, not
a specific slug such as `/blog/hello`.

### Produce a Machine-readable Analysis Report

Use JSON when another program will consume the result, or SARIF when a code-scanning tool accepts
it:

```bash
npm run analyze -- --format json --output reports/ruvyxa-analysis.json
npm run analyze -- --format sarif --output reports/ruvyxa.sarif
```

Create the destination directory first if it does not exist. Use `--format human` for a readable
terminal report.

### Inspect Toolchain and Adapter Compatibility

Run `doctor` before a first deployment or after changing runtime/adapter settings:

```bash
npm run doctor
npm run doctor -- --target edge
npm run doctor -- --adapter cloudflare --json
```

`--adapter` inspects the adapter contract without materializing its build artifacts.

### Reset Only Generated Output

When a local build needs to be regenerated, clean the configured Ruvyxa output and build again:

```bash
npm run clean
npm run build
```

This command is limited to the configured output directory. It does not delete `node_modules`,
source files, or arbitrary cache directories.

### Measure a Change Repeatedly

Use a fixed number of samples when comparing two local changes:

```bash
npm run bench -- --samples 5
npm run bench -- --samples 5 --json
```

Treat a benchmark as a local signal: use the same machine, dependency state, and sample count when
comparing runs.

### Scaffold a Plugin Outside the App Directory

For a monorepo, choose the package directory explicitly:

```bash
npm run plugin -- create analytics --root . --dir packages/analytics
```

For a standalone project, the default is enough:

```bash
npm run plugin -- create my-plugin
```

Then follow [Plugins](./14-plugins.md) to implement and register the generated package.

### CI Example

The CLI commands can be used directly in a CI job after dependencies are installed:

```bash
npm run check
npm run analyze -- --format sarif --output reports/ruvyxa.sarif
npm run build -- --adapter node
```

Do not add unsupported flags such as `--verbose`, `--no-cache`, or `--sourcemap`; they are not part
of the current CLI contract.

## Guided Workflows

The following walkthroughs show how the commands fit together. They intentionally show commands that
exist today rather than guessed output: projects differ in their routes, configuration, and
installed adapters.

### First Development Session

Start by checking what the project exposes, then keep the development server running while making
changes:

```bash
# from the application root
npm run routes
npm run dev
```

If the app is in a sibling directory, the same workflow is:

```bash
npm run routes -- --root ../my-app
npm run dev -- --root ../my-app
```

Use `routes` before `dev` when you are unsure whether a page was discovered. Do not expect it to
start a server; it only reports the route table. Conversely, leave `dev` running while editing,
because it owns the watcher and HMR lifecycle.

### Diagnose a Route That Does Not Behave as Expected

For a route issue, move from broad discovery to the route-specific manifest entry, then run the
static checks:

```bash
npm run routes
npm run trace -- /blog/[slug]
npm run analyze -- --format human
npm run check
```

Replace `/blog/[slug]` with the pattern printed by `routes`. `trace` needs that route pattern; it
does not accept a component filename or a concrete URL parameter. `analyze` checks project structure
and boundaries, while `check` additionally runs TypeScript checking when applicable and the parity
flow. None of these commands edits source files.

### Investigate an Adapter Before Building

When moving an app to a platform target, inspect the selected target and adapter first:

```bash
npm run doctor -- --target edge
npm run doctor -- --target edge --adapter cloudflare --json
npm run build -- --target edge --adapter cloudflare
```

The first two commands are diagnostics. The final command is the one that builds and invokes the
selected adapter. If the project is intended for a Node server instead, use `--target node` and
`--adapter node`; adapter selection should agree with the deployment environment.

### Produce Files for a Code-quality Job

`analyze` can write an artifact without forcing a particular CI vendor. Make the report directory as
part of the job, then select a format that the next tool understands:

```bash
mkdir -p reports
npm run analyze -- --format json --output reports/ruvyxa-analysis.json
npm run analyze -- --format sarif --output reports/ruvyxa.sarif
```

In PowerShell, create the directory with `New-Item -ItemType Directory -Force reports` instead of
`mkdir -p reports`. The JSON and SARIF files are different formats; choose one unless the job
actually needs both. Keep `npm run check` as a separate gate for the type-check and parity flow.

### Rebuild After Changing Build Configuration

Use this sequence after a configuration or adapter change when you want fresh generated output:

```bash
npm run doctor -- --adapter vercel
npm run clean
npm run build -- --adapter vercel
npm run preview
```

`clean` removes only Ruvyxa's configured output directory. It is not a dependency reset, and it does
not replace package-manager commands such as `npm install`. `preview` comes last because it serves
what `build` has already generated.

### Compare a Performance Change Fairly

Benchmark before and after a focused change using the same sample count and output form:

```bash
npm run bench -- --samples 10 --json
# make one focused change, then run the same command again
npm run bench -- --samples 10 --json
```

Save the two JSON results in your CI system or compare them locally. Avoid treating two runs with
different runtimes, dependency trees, or sample counts as a direct regression comparison.

### Get Help Without Guessing a Flag

The package scripts and command-specific CLI help are the safest way to confirm current syntax:

```bash
npm run
npm run build -- --help
npm run analyze -- --help
npm run plugin -- create --help
```

Use `npm run` to list the available starter scripts, then command-specific help before putting a
command in automation. In particular, `--format` belongs to `analyze`, `--json` belongs to `doctor`
and `bench`, and `--dir` belongs to `plugin create`.

## Next Steps

- [Configuration](./11-configuration.md) — configure the CLI's project inputs.
- [Deployment](./13-deployment.md) — select and configure an adapter.
- [Plugins](./14-plugins.md) — write or scaffold a plugin.

## Under the Hood: CLI Diagnostics

CLI commands surface diagnostics from the route graph and bundler. For example, `ruvyxa analyze` can
report the source-confirmed boundary codes `RUV1007` (`server-only` reachable from a client graph),
`RUV1008` (private environment variable in a client graph), `RUV1009` (`client-only` reached from a
server graph), and `RUV1010` (a `server/` module reached from a client graph). The exact code
depends on the violated rule; the list above is the current boundary-code set used by the analyzer.
The `ruvyxa doctor` command evaluates the selected deployment adapter and its reported capabilities.

### Full CLI Capabilities

- `ruvyxa dev`: Boots the Axum server with HMR and persistent JS workers.
- `ruvyxa build`: Triggers Oxc compilation, Tree-shaking, and CSS fnv1a_64 hashing.
- `ruvyxa test:parity`: Evaluates behavioral drift between development and production routes.

# Current DX additions

The current CLI also exposes the following additive developer workflows:

```bash
npm run routes:json
npm run analyze:html
npm run analyze:html -- --output .ruvyxa/reports/bundles.html
npm run add -- form
npm run add -- data-table
npm run add -- auth
```

`analyze:html` writes a self-contained interactive report to `.ruvyxa/analyze.html` unless
`--output <file>` selects a different location; JSON and SARIF remain the preferred CI formats.
`add` checks every destination before writing so a conflict never leaves a half-created scaffold. It
will not overwrite existing files unless `--force` is explicit. The auth scaffold always prints the
`@ruvyxa/auth` dependency and next-step reminder, so install it before using the generated
authentication files.

While `ruvyxa dev` is running, `/__ruvyxa/devtools` shows routes, LRU render-cache state, bundle
metrics, Server Action timings, and uptime. It is not registered by the production server.
