# CLI Commands

Ruvyxa ships with 13 CLI commands that cover the full development lifecycle -- from scaffolding to
dev servers to production builds to diagnostics. Every command shares the same `ruvyxa` binary,
built in Rust and published via npm.

---

## What You Will Learn

- Every CLI command with full syntax and all options
- Real example output for each command (exact format)
- Global flags available on all commands
- Exit codes: 0 = success, 1 = error, 2 = config error
- All subcommand argument structs (Rust source)
- Common use cases and recipes
- Troubleshooting every known issue

---

## Global Options

These flags work with any command:

| Flag               | Short | Type        | Default     | Description                                             |
| ------------------ | ----- | ----------- | ----------- | ------------------------------------------------------- |
| `--root <path>`    | `-r`  | `PathBuf`   | `.`         | Project root directory                                  |
| `--runtime <name>` |       | `node\|bun` | auto-detect | JS runtime; overrides RUVYXA_RUNTIME and config.runtime |
| `--help`           | `-h`  | flag        | --          | Print command help                                      |
| `--version`        | `-v`  | flag        | --          | Print Ruvyxa version                                    |
| `--no-color`       |       | flag        | --          | Disable colored output                                  |

### CLI Entry Point

The CLI uses `clap` v4 with derive macros. Entry point in `crates/ruvyxa_cli/src/main.rs`:

```rust
#[derive(Debug, Parser)]
#[command(styles = cli_styles(), ...)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
```

---

## Command Tree

```
ruvyxa
├── dev            Start dev server with HMR
├── build          Production build
├── start          Serve production build
├── preview        Build + start
├── check          Validate project
├── routes         Print route table
├── analyze        Bundle analysis (human | json | sarif)
├── doctor         Environment diagnostics
├── clean          Remove build artifacts
├── trace          Inspect a specific route
├── bench          Benchmark rendering
├── test:parity    Dev/prod parity check (alias: parity)
└── plugin         Plugin scaffolding
    └── create     Scaffold a new plugin package
```

### Rust Command Enum

```rust
enum Command {
    Dev(ServerArgs),
    Build(BuildArgs),
    Check(ProjectArgs),
    Start(ServerArgs),
    Preview(ServerArgs),
    Routes(ProjectArgs),
    Analyze(AnalyzeArgs),
    Doctor(DoctorArgs),
    Clean(ProjectArgs),
    Trace(TraceArgs),
    Bench(BenchArgs),
    TestParity(ProjectArgs),  // alias: parity
    Plugin(PluginArgs),
}
```

---

## ruvyxa dev

Start the development server with Hot Module Replacement.

```bash
ruvyxa dev
ruvyxa dev --root ./my-app
ruvyxa dev --host 127.0.0.1 --port 4000
ruvyxa dev -r ./my-app --port 8080
ruvyxa dev --runtime bun
```

### Options

| Option             | Short | Type        | Default   | Description         |
| ------------------ | ----- | ----------- | --------- | ------------------- |
| `--root <path>`    | `-r`  | `PathBuf`   | `.`       | Project root        |
| `--host <host>`    |       | `String`    | `0.0.0.0` | Bind host           |
| `--port <port>`    | `-p`  | `u16`       | `3000`    | Bind port           |
| `--runtime <name>` |       | `node\|bun` | auto      | JS runtime override |

### Rust Struct

```rust
struct ServerArgs {
    root: PathBuf,
    host: Option<String>,
    port: Option<u16>,
    runtime: Option<CliRuntime>,
}
```

### Example Output

```
+============================================+
|  Ruvyxa dev server running                 |
|                                            |
|  -> Local:   http://localhost:3000         |
|  -> Network: http://192.168.1.42:3000      |
|                                            |
|  v 15 routes scanned                       |
|  v 0 conflicts                             |
|  v HMR ready                               |
+============================================+

  SSR     /                          app/page.tsx
  SSR     /about                     app/about/page.tsx
  ISR     /blog/[slug]               app/blog/[slug]/page.tsx
  API     /api/hello                 app/api/hello/route.ts
  STATIC  /favicon.ico               app/favicon.ico

  v Ready in 48ms
```

HMR is active. Edit any file -- the browser updates instantly.

### Common Uses

```bash
# Different port if 3000 is busy
ruvyxa dev --port 4000

# Only listen locally (safer for shared networks)
ruvyxa dev --host 127.0.0.1

# Work on a project in a different directory
ruvyxa dev --root ../my-other-project

# Use Bun runtime
ruvyxa dev --runtime bun
```

---

## ruvyxa build

Create a production build in `outDir`.

```bash
ruvyxa build
ruvyxa build --root ./my-app
ruvyxa build --target es2022
ruvyxa build --adapter vercel
ruvyxa build --runtime bun
```

### Options

| Option                | Short | Type                      | Default      | Description                                                                                                      |
| --------------------- | ----- | ------------------------- | ------------ | ---------------------------------------------------------------------------------------------------------------- |
| `--root <path>`       | `-r`  | `PathBuf`                 | `.`          | Project root                                                                                                     |
| `--target <target>`   |       | `node\|bun\|edge\|static` | config value | Override build target                                                                                            |
| `--adapter <adapter>` |       | `string`                  | --           | Override deploy adapter (node, vercel, netlify, cloudflare, railway, render, firebase, aws, or npm package name) |
| `--runtime <name>`    |       | `node\|bun`               | auto         | JS runtime override                                                                                              |

### Rust Struct

```rust
struct BuildArgs {
    root: PathBuf,
    target: Option<BuildTarget>,
    adapter: Option<String>,
    runtime: Option<CliRuntime>,
}
```

### Example Output

```
+============================================+
|  Ruvyxa build                              |
|                                            |
|  v Resolved 47 modules                     |
|  v Compiled 12 routes                      |
|  v Optimized 8 images                      |
|  v Minified 3 bundles                      |
|  v Manifest written                        |
|                                            |
|  Output: .ruvyxa/                          |
|  Size:   1.2 MB (632 KB gzip)              |
|  Time:   2.3s                              |
+============================================+
```

### Build Output Structure

```
.ruvyxa/
+-- client/              # Browser bundles
|   +-- _entry.js        # Entry point
|   +-- _shared.js       # Shared dependencies
|   +-- index.js         # Route: /
|   +-- about.js         # Route: /about
+-- server/              # Server bundles
|   +-- index.js
|   +-- about.js
+-- assets/              # Static assets
|   +-- images/          # Optimized images
|   +-- fonts/
|   +-- styles.css
+-- prerender/           # SSG output
|   +-- index.html
|   +-- about.html
+-- manifest.json        # Build manifest
+-- build.json           # Adapter input
```

### Build Pipeline (Rust)

```
1. load_project_config(root) -- evaluate ruvyxa.config.ts
2. discover_routes() -- scan appDir for route files
3. compile_styles() -- process CSS entries
4. optimize_public_images() -- PNG/JPEG -> WebP via rayon
5. scan_raw_image_usage() -- warn on raw <img> tags
6. build_router() -- compile all routes (server + client)
7. write_manifest() -- emit manifest.json
8. run_adapter_runner() -- if adapter configured, emit deploy artifacts
```

---

## ruvyxa start

Serve a production build. Run this after `ruvyxa build`.

```bash
ruvyxa start
ruvyxa start --root ./my-app
ruvyxa start --port 8080
```

### Options

Same as `dev` -- uses the same `ServerArgs` struct.

### Example Output

```
+============================================+
|  Ruvyxa production server                  |
|                                            |
|  -> Local:   http://localhost:3000         |
|  -> Network: http://192.168.1.42:3000      |
|                                            |
|  v Serving 12 routes                       |
|  v Cache layer active                      |
|  v 3 workers ready                         |
|                                            |
|  Mode: production                          |
+============================================+
```

---

## ruvyxa preview

Build then start in one step -- useful for CI checks and quick production testing.

```bash
ruvyxa preview
ruvyxa preview --root ./my-app --port 5000
```

### Options

Combines `BuildArgs` and `ServerArgs`.

---

## ruvyxa check

Type-check your project and validate routes, config, and imports. Runs `tsc --noEmit` internally
plus parity checks.

```bash
ruvyxa check
ruvyxa check --root ./my-app
ruvyxa check --runtime bun
```

### Options

| Option             | Short | Type        | Default | Description         |
| ------------------ | ----- | ----------- | ------- | ------------------- |
| `--root <path>`    | `-r`  | `PathBuf`   | `.`     | Project root        |
| `--runtime <name>` |       | `node\|bun` | auto    | JS runtime override |

### Rust Struct

```rust
struct ProjectArgs {
    root: PathBuf,
    runtime: Option<CliRuntime>,
}
```

### Example Output (pass)

```
+============================================+
|  Ruvyxa check                              |
|                                            |
|  v TypeScript: 0 errors                    |
|  v 12 routes valid                         |
|  v 0 route conflicts                       |
|  v Config validated                        |
|  v Server/client boundary clean            |
|  v 0 unused files                          |
|                                            |
|  All checks passed.                        |
+============================================+
```

### Example Output (fail)

```
x TypeScript: 2 errors
  app/page.tsx:5:12 -- Type 'string' is not assignable to type 'number'

x 1 route conflict
  app/blog/[slug]/page.tsx and app/blog/latest/page.tsx
  Both match /blog/latest

x Config: unknown field "unkownField"
  ruvyxa.config.ts:3

  Run with --verbose for details.
```

### Checks Performed

1. TypeScript compilation (`tsc --noEmit`)
2. Route validation (no conflicts, valid params)
3. Config validation (RUV1600-1602)
4. Server/client boundary check (RUV1007, RUV1008, RUV1009, RUV1010)
5. Unused file detection
6. Dependency resolution

---

## ruvyxa routes

Print the route table -- every URL path your app handles.

```bash
ruvyxa routes
ruvyxa routes --root ./my-app
```

### Options

Same `ProjectArgs` struct as `check`.

### Example Output

```
+============================================+
|  Route Table                               |
|                                            |
|  Method  URL                     File      |
|  ------  -----------------------  -------- |
|  SSR     /                       app/page.tsx
|  SSR     /about                  app/about/page.tsx
|  SSR     /blog                   app/blog/page.tsx
|  SSR     /blog/[slug]            app/blog/[slug]/page.tsx
|  SSG     /blog/hello-world       app/blog/[slug]/page.tsx
|  API     /api/hello              app/api/hello/route.ts
|  POST    /api/users              app/api/users/route.ts
|  ACTION  /actions/newsletter     app/actions/newsletter/action.ts
|  STATIC  /favicon.ico            app/favicon.ico
|  MDX     /docs/guide             app/docs/guide/page.mdx
|                                            |
|  10 routes total, 0 conflicts              |
+============================================+
```

The Method column shows rendering strategy or HTTP method. This is the fastest way to understand
your app's URL surface.

### Route Method Types

| Method   | Meaning                  | Source                                                          |
| -------- | ------------------------ | --------------------------------------------------------------- |
| `SSR`    | Server-side render       | `page.tsx`, `page.md`, `page.mdx` without export const strategy |
| `SSG`    | Static generation        | Page with `strategy = 'ssg'`                                    |
| `ISR`    | Incremental static regen | Page with `strategy = 'isr'`                                    |
| `CSR`    | Client-side render       | Page with `strategy = 'csr'`                                    |
| `API`    | API route (GET)          | `route.ts` with `export GET`                                    |
| `POST`   | API route (POST)         | `route.ts` with `export POST`                                   |
| `ACTION` | Server action            | `action.ts` files                                               |
| `STATIC` | Static file              | `favicon.ico`, `robots.txt`, etc.                               |
| `MDX`    | Markdown/MDX page        | `page.md`, `page.mdx`                                           |

---

## ruvyxa analyze

Generate a detailed analysis of your build -- bundle sizes, module dependencies, image sizes.

```bash
ruvyxa analyze
ruvyxa analyze --format json --output report.json
ruvyxa analyze --format tree
ruvyxa analyze --format sarif
```

### Options

| Option              | Short | Type                       | Default | Description                                               |
| ------------------- | ----- | -------------------------- | ------- | --------------------------------------------------------- |
| `--root <path>`     | `-r`  | `PathBuf`                  | `.`     | Project root                                              |
| `--runtime <name>`  |       | `node\|bun`                | auto    | JS runtime override                                       |
| `--format <format>` |       | `auto\|human\|json\|sarif` | `auto`  | Output format (auto = table for terminal, json for piped) |
| `--output <path>`   | `-o`  | `PathBuf`                  | stdout  | Write to file instead of stdout                           |

### Rust Struct

```rust
struct AnalyzeArgs {
    root: PathBuf,
    runtime: Option<CliRuntime>,
    format: AnalyzeFormat,  // Auto, Human, Json, Sarif
    output: Option<PathBuf>,
}

enum AnalyzeFormat { Auto, Human, Json, Sarif }
```

### Example Output (human/table)

```
+============================================+
|  Bundle Analysis                           |
|                                            |
|  Bundle          Size    Modules           |
|  --------------- ------ ------------------ |
|  _entry.js       12 KB   3                |
|  _shared.js      48 KB   14               |
|  index.js        124 KB  22               |
|  about.js        89 KB   18               |
|  blog/[slug].js  156 KB  31               |
|                                            |
|  Images          Original  Optimized       |
|  --------------- --------  --------------- |
|  hero.jpg        1.2 MB   84 KB           |
|  cat.png         800 KB   320 KB          |
|                                            |
|  Total: 1.2 MB (632 KB gzip)              |
+============================================+
```

### Example Output (tree)

```
.ruvyxa/                  1.2 MB
+-- client/              892 KB
|   +-- _entry.js         12 KB
|   +-- _shared.js        48 KB
|   +-- index.js         124 KB
|   |   +-- react-dom    420 KB (shared)
|   |   +-- lodash        24 KB (shared)
|   +-- about.js          89 KB
+-- server/              312 KB
+-- assets/
    +-- images/          210 KB
```

---

## ruvyxa doctor

Diagnose your project environment -- Node version, port availability, config validity, dependency
health.

```bash
ruvyxa doctor
ruvyxa doctor --target production
ruvyxa doctor --adapter vercel
ruvyxa doctor --json
ruvyxa doctor --runtime bun
```

### Options

| Option                | Short | Type                      | Default | Description                               |
| --------------------- | ----- | ------------------------- | ------- | ----------------------------------------- |
| `--root <path>`       | `-r`  | `PathBuf`                 | `.`     | Project root                              |
| `--target <target>`   |       | `node\|bun\|edge\|static` | --      | Check compatibility with specific runtime |
| `--adapter <adapter>` |       | `string`                  | --      | Check adapter compatibility               |
| `--runtime <name>`    |       | `node\|bun`               | auto    | JS runtime override                       |
| `--json`              |       | `bool`                    | `false` | Output as JSON                            |

### Rust Struct

```rust
struct DoctorArgs {
    root: PathBuf,
    target: Option<BuildTarget>,
    adapter: Option<String>,
    runtime: Option<CliRuntime>,
    json: bool,
}
```

### Example Output

```
+============================================+
|  Ruvyxa doctor                             |
|                                            |
|  v Node.js 22.4.1                          |
|  v Config valid                            |
|  v Dependencies up to date                 |
|  v Port 3000 available                     |
|  v OutDir .ruvyxa/ writable                |
|  v 12 routes valid                         |
|  v TypeScript config found                 |
|  v .env file present                       |
|  v Adapter: vercel compatible              |
|                                            |
|  w Recommendation:                         |
|    Set build.manifest: true in            |
|    ruvyxa.config.ts for Vercel            |
+============================================+
```

### JSON Output

```json
{
  "nodeVersion": "22.4.1",
  "configValid": true,
  "portAvailable": true,
  "outDirWritable": true,
  "routesValid": 12,
  "envFilePresent": true,
  "adapterCompatible": true,
  "recommendations": ["Set build.manifest: true"],
  "node": "22.4.1",
  "platform": "win32"
}
```

### Checks Performed

1. Node.js version
2. Config validity
3. Dependency freshness
4. Port availability
5. Output directory writability
6. Route validity (via discovery)
7. TypeScript config presence
8. .env file presence
9. Adapter compatibility (if --adapter specified)
10. .env.example presence

---

## ruvyxa clean

Delete the output directory (`.ruvyxa/` by default) and all caches.

```bash
ruvyxa clean
ruvyxa clean --root ./my-app
```

### Options

Same `ProjectArgs` struct.

### Example Output

```
+============================================+
|  Ruvyxa clean                              |
|                                            |
|  v Removed .ruvyxa/                        |
|  v Removed .cache/                         |
|                                            |
|  Cleaned 2 directories, 347 files          |
+============================================+
```

Use this when you suspect stale cache is causing issues, then rebuild.

---

## ruvyxa trace

Inspect a specific route -- its rendering strategy, params, layout chain, and data dependencies.

```bash
ruvyxa trace /blog/hello-world
ruvyxa trace /blog/[slug] --root ./my-app
```

### Options

| Option          | Short | Type      | Default  | Description         |
| --------------- | ----- | --------- | -------- | ------------------- |
| `--root <path>` | `-r`  | `PathBuf` | `.`      | Project root        |
| positional      |       | `String`  | required | Route path to trace |

### Rust Struct

```rust
struct TraceArgs {
    route: String,
    root: PathBuf,
}
```

### Example Output

```
+============================================+
|  Route trace: /blog/hello-world            |
|                                            |
|  Route                                     |
|    Path:       /blog/[slug]                |
|    File:       app/blog/[slug]/page.tsx    |
|    Strategy:   ISR (revalidate: 60)        |
|    Params:     { slug: "hello-world" }     |
|                                            |
|  Layout chain                              |
|    root/layout.tsx                         |
|    +-- blog/layout.tsx                    |
|                                            |
|  Data                                      |
|    loader:    app/blog/[slug]/page.tsx:12  |
|    cache:     force-cache                  |
|    deps:      [                            |
|      "app/blog/posts.json"                 |
|    ]                                       |
|                                            |
|  Preview                                   |
|    http://localhost:3000/blog/             |
|    hello-world                             |
+============================================+
```

---

## ruvyxa bench

Benchmark your application rendering performance.

```bash
ruvyxa bench
ruvyxa bench --samples 100
ruvyxa bench --samples 5 --json
ruvyxa bench --json --output bench.json
```

### Options

| Option          | Short | Type      | Default | Description                 |
| --------------- | ----- | --------- | ------- | --------------------------- |
| `--root <path>` | `-r`  | `PathBuf` | `.`     | Project root                |
| `--samples <n>` | `-s`  | `usize`   | `3`     | Number of samples per route |
| `--json`        |       | `bool`    | `false` | JSON output                 |

### Rust Struct

```rust
struct BenchArgs {
    root: PathBuf,
    samples: usize,       // default: 3
    json: bool,
}
```

### Example Output

```
+============================================+
|  Bench results (3 samples)                 |
|                                            |
|  Route           Avg      P95              |
|  --------------- -------  ---------------- |
|  /                12 ms    18 ms           |
|  /about            8 ms    14 ms           |
|  /blog/[slug]     24 ms    42 ms           |
|  /api/hello        3 ms     5 ms           |
|                                            |
|  Overall avg: 12 ms                        |
|  Slowest route: /blog/[slug] 24 ms        |
+============================================+
```

---

## ruvyxa test:parity

Compare dev server output against production build output for every route. Ensures no differences
between dev and prod rendering.

```bash
ruvyxa test:parity
ruvyxa test:parity --root ./my-app
ruvyxa parity      # alias
```

### Options

Same `ProjectArgs` struct.

### Example Output (pass)

```
+============================================+
|  Dev/Prod parity check                     |
|                                            |
|  Route           Status                    |
|  --------------- ------------------------- |
|  /               v match                   |
|  /about          v match                   |
|  /blog/[slug]    v match                   |
|  /api/hello      v match                   |
|                                            |
|  4/4 routes passed                        |
+============================================+
```

### Example Output (fail)

```
|  /counter        x mismatch               |
|    dev:   <button>Count: 5</button>       |
|    prod:  <button>Count: 0</button>       |
|    Cause: client state hydration           |
```

---

## ruvyxa plugin create

Scaffold a new plugin package.

```bash
ruvyxa plugin create my-plugin
ruvyxa plugin create my-plugin --dir ./plugins
ruvyxa plugin create @scope/ruvyxa-plugin-analytics
```

### Options

| Option          | Short | Type      | Default         | Description                     |
| --------------- | ----- | --------- | --------------- | ------------------------------- |
| `--root <path>` | `-r`  | `PathBuf` | `.`             | Project root                    |
| `--dir <path>`  |       | `PathBuf` | `{root}/<name>` | Parent directory for the plugin |

### Rust Struct

```rust
struct PluginCreateArgs {
    name: String,
    root: PathBuf,
    dir: Option<PathBuf>,
}
```

### Example Output

```
+============================================+
|  Creating plugin: my-plugin                |
|                                            |
|  v Created plugins/my-plugin/              |
|  v Created package.json                    |
|  v Created src/index.ts                    |
|  v Created tsconfig.json                   |
|  v Created README.md                       |
|  v Created test/plugin.test.mjs            |
|  v Created .gitignore                      |
|                                            |
|  Next steps:                              |
|    cd plugins/my-plugin                   |
|    npm install                            |
|    npm run build                          |
+============================================+
```

### Template Files

The plugin scaffold creates these files:

```rust
const PLUGIN_TEMPLATE_FILES: &[(&str, &str)] = &[
    ("src/index.ts", ...),
    ("test/plugin.test.mjs", ...),
    ("package.json", ...),
    ("tsconfig.json", ...),
    ("README.md", ...),
    (".gitignore", ...),
];
```

### Name Validation

Plugin names are normalized: non-alphanumeric characters replaced with hyphens, lowercase. The `dir`
must not contain `..` components and must not be empty.

---

## Exit Codes

| Code | Meaning       | When                                       |
| ---- | ------------- | ------------------------------------------ |
| `0`  | Success       | Command completed without errors           |
| `1`  | General error | Build failure, config error, runtime error |
| `2`  | Config error  | Config validation failure (RUV1600-1602)   |

Exit codes are returned by Rust's `anyhow::Error` propagation through `main()`.

---

## Quick Reference Card

| What you want        | Command                          |
| -------------------- | -------------------------------- |
| Start coding         | `ruvyxa dev`                     |
| Build for production | `ruvyxa build`                   |
| Serve production     | `ruvyxa start`                   |
| Build + serve        | `ruvyxa preview`                 |
| Validate everything  | `ruvyxa check`                   |
| See all URLs         | `ruvyxa routes`                  |
| Debug a route        | `ruvyxa trace /my-route`         |
| Find problems        | `ruvyxa doctor`                  |
| Analyze bundle       | `ruvyxa analyze`                 |
| Fresh start          | `ruvyxa clean && ruvyxa dev`     |
| Performance test     | `ruvyxa bench`                   |
| Dev vs prod check    | `ruvyxa test:parity`             |
| Create a plugin      | `ruvyxa plugin create my-plugin` |

---

## Troubleshooting

| Problem                        | Solution                                                          |
| ------------------------------ | ----------------------------------------------------------------- |
| `ruvyxa: command not found`    | Install: `npm install -g ruvyxa` or use `npx ruvyxa`              |
| Dev server won't start         | Run `ruvyxa doctor` to check Node, port, config                   |
| "Address in use"               | Port 3000 taken -- use `ruvyxa dev --port 4000`                   |
| Build fails with cryptic error | `ruvyxa clean && ruvyxa build` for fresh build                    |
| Routes not showing             | Ensure files are in correct `app/` directory                      |
| `check` reports false errors   | Update `tsconfig.json` paths, run `ruvyxa clean`                  |
| Config load fails              | `ruvyxa doctor` to validate config; check ruvyxa.config.ts syntax |
| RUV1600 on startup             | Config renderer failed; check Node/Bun installation               |
| Adapter not found              | Use known name or full npm package name                           |
| --runtime flag ignored         | Ensure runtime is installed on PATH                               |

---

## Environment Variables Affecting CLI

| Variable                                                                      | Effect                                                                      |
| ----------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `RUVYXA_RUNTIME`                                                              | JS runtime (node/bun); overrides config, overridden by --runtime flag       |
| `RUVYXA_ADAPTER`                                                              | Overrides platform adapter auto-detection                                   |
| `VERCEL`, `NETLIFY`, `CF_PAGES`, `RAILWAY_PROJECT_ID`, `RENDER`, `AWS_APP_ID` | Auto-detect deployment platform (see `PLATFORM_ADAPTER_ENV` in Rust source) |

---

## Next Steps

- [01-getting-started.md](./01-getting-started.md) -- First project with CLI
- [11-configuration.md](./11-configuration.md) -- Config options consumed by CLI
- [13-deployment.md](./13-deployment.md) -- Build and deploy
- [14-plugins.md](./14-plugins.md) -- Plugin create command
