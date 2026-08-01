# Error Handling

Ruvyxa uses `RUV####` diagnostics for many framework failures. A diagnostic may include a code,
explanation, suggested fix, and source context; the exact fields and code depend on the subsystem.
This guide documents source-confirmed codes and does not claim that every possible runtime error has
a stable public code or a fixed file location.

---

## What You Will Learn

- Error code format and how to read diagnostic output
- Source-confirmed error catalog entries organized by range
- Error boundaries (`error.tsx`) — component signature, hierarchy, `reset()`
- `not-found.tsx` — nearest-boundary resolution, programmatic `notFound()`
- `loading.tsx` — Suspense fallback integration
- Server action and API route error patterns
- Dev server error overlay — format, configuration, dismissal
- Troubleshooting every known error condition

---

## Error Code Format

Every Ruvyxa error follows this structure:

```
RUV####: Error Title

  Detail: A human-readable explanation of what went wrong.
  File: app/page.tsx:25
  Context: Additional info (variable values, route params, etc.)

  Fix: What to do to resolve the error.

  For more information, see:
  https://ruvyxa.dev/docs/errors/RUV####
```

### Diagnostic Structure (Rust)

```rust
struct Diagnostic {
    code: &'static str,      // e.g. "RUV1007"
    title: String,            // e.g. "Private import"
    explanation: String,      // Human-readable why
    span: Option<SourceSpan>, // File + line + column
    import_chain: Vec<PathBuf>, // Import trace for boundary violations
    suggested_fix: Option<String>,
    affected_routes: Vec<String>,
}

struct SourceSpan {
    file: PathBuf,
    line: Option<u32>,
    column: Option<u32>,
}
```

---

## Documented Error Catalog (source-confirmed entries)

### RUV1001–1099: Boundary Violations

Fire when crossing server/client boundary illegally. Detected by the bundler during module graph
analysis.

| Code    | Title                                              | Cause                                                | Fix                                                    |
| ------- | -------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------ |
| RUV1001 | App directory was not found                        | Missing `app/` directory at project root             | Create `app/` directory                                |
| RUV1002 | Invalid dynamic route segment                      | Route segment uses disallowed characters or syntax   | Use correct `[param]`, `[...param]`, or `[[...param]]` |
| RUV1003 | Conflicting route paths                            | Two files match same URL pattern                     | Remove or rename conflicting file                      |
| RUV1004 | Page is missing a default export                   | Page file lacks `export default`                     | Add `export default function Page() { ... }`           |
| RUV1007 | Server-only module imported into client bundle     | `import "server-only"` reachable from client bundle  | Move server code behind API route or loader            |
| RUV1008 | Private environment variable used in client bundle | `process.env.SECRET` in client-reachable code        | Prefix with `RUVYXA_PUBLIC_` or move to server         |
| RUV1009 | Client-only module imported into SSR graph         | `'client-only'` module reachable from server runtime | Use dynamic import with `{ ssr: false }`               |
| RUV1010 | Server directory module reached by client graph    | File inside `server/` directory imported from client | Restructure imports                                    |

#### RUV1001 — App directory was not found

```
RUV1001: App directory was not found

  The project root does not contain an `app/` directory.

  Fix: Create the `app/` directory at the project root.
```

**Source**: `crates/ruvyxa_graph/src/lib.rs:172`

#### RUV1002 — Invalid dynamic route segment

```
RUV1002: Invalid dynamic route segment

  File: app/products/[id].tsx

  Route segment uses characters that are not allowed for
  dynamic segments. Catch-all segments must be the final
  URL segment.

  Fix: Use `[param]` for single, `[...param]` for catch-all,
       `[[...param]]` for optional catch-all.
```

**Source**: `crates/ruvyxa_graph/src/lib.rs:1138`

#### RUV1003 — Conflicting route paths

```
RUV1003: Conflicting route paths

  Route: /products/[id]
  Files:
    app/products/[id]/page.tsx
    app/products/[slug]/page.tsx

  Both files match the same URL pattern like /products/123.

  Fix: Rename one of the conflicting dynamic segments so
       they have different parameter names.
```

**Source**: `crates/ruvyxa_graph/src/lib.rs:1501`

#### RUV1004 — Page is missing a default export

```
RUV1004: Page is missing a default export

  File: app/about/page.tsx

  Fix: Add `export default function Page() { ... }`
```

Any form of default export satisfies the check, including a re-export —
`export { Page as default }`, `export { default } from './page-impl'`, and
`export * as default from './page-impl'` all count. A `export type { X as default }` does not: a
type export erases at compile time, so nothing is left to render.

**Source**: `crates/ruvyxa_graph/src/lib.rs:300`

#### RUV1007 — Server-only module imported into client bundle

```
RUV1007: Server-only module imported into client bundle

  Module: server-only
  File: app/components/UserCard.tsx:3
  Import chain:
    app/components/UserCard.tsx
    app/lib/auth.ts

  Fix: Remove the `import "server-only"` statement or
       move the file out of the client component tree.
```

**Source**: `crates/ruvyxa_bundler/src/boundary.rs:73` **Detection**: At build time when bundler
walks module graph and encounters `server-only` import in a client reachable module.

#### RUV1008 — Private environment variable used in client bundle

```
RUV1008: Private environment variable used in client bundle

  Variable: DATABASE_URL
  File: app/components/UserCard.tsx:12
  Value: postgres://user:pass@localhost:5432/db

  ⚠ This variable is NOT prefixed with RUVYXA_PUBLIC_.
    It will be inlined in the client bundle and exposed to users.

  Fix:
    1. If this value is safe for clients, rename it to
       RUVYXA_PUBLIC_DATABASE_URL in your .env file.
    2. If this value must remain secret, move the usage
       to a server component, API route, or server action.
```

**Source**: `crates/ruvyxa_bundler/src/boundary.rs:104` **Detection**: Non-fatal warning emitted
during build when bundler finds `process.env.<NON_PUBLIC_VAR>` in client-reachable code. The
variable is still inlined but the diagnostic warns. Fatal only if the env var access is in a client
component that would ship it to browsers.

**Edge case**: Variables accessed only inside `if (typeof window === 'undefined')` guards are
considered server-only and do not trigger RUV1008.

#### RUV1009 — Client-only module imported into SSR graph

```
RUV1009: Client-only module imported into SSR graph

  File: app/components/Map.tsx:1
  Import: leaflet

  This module uses browser APIs and cannot be rendered on the server.

  Fix: Use dynamic import with `{ ssr: false }` or wrap
       in a client component boundary.
```

**Source**: `crates/ruvyxa_bundler/src/boundary.rs:132`

#### RUV1010 — Server directory module reached by client graph

```
RUV1010: Server directory module reached by client graph

  File: app/server/db.ts
  Imported in: app/components/List.tsx

  Files inside server/ directories must only be imported
  from server components.

  Fix: Move the shared logic to a file outside server/,
       or restructure to avoid importing it from client code.
```

**Source**: `crates/ruvyxa_bundler/src/boundary.rs:89`

---

### RUV1100–1199: SSR Renderer Errors

| Code    | Title                      | Cause                                     | Fix                                               |
| ------- | -------------------------- | ----------------------------------------- | ------------------------------------------------- |
| RUV1100 | React SSR failed           | React rendering threw on server           | Check page component, imports, React dependencies |
| RUV1101 | SSR renderer args missing  | SSR renderer called without required args | Framework bug — report                            |
| RUV1102 | SSR renderer was not found | Route has layout but no SSR renderer      | Ensure page file exports a default component      |

#### RUV1100 — React SSR failed

```
RUV1100: React SSR failed

  Route: /dashboard
  Error: The page component threw during server-side rendering.

  Fix: Check the page component, its imports, and whether
       React dependencies are installed.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:782`

#### RUV1101 — SSR renderer args missing

```
RUV1101: SSR renderer requires projectRoot, appDir, and pageFile arguments

  Fix: This is likely a framework bug — report it.
```

**Source**: `packages/ruvyxa/runtime/ssr-renderer.mjs:21`

#### RUV1102 — SSR renderer was not found

```
RUV1102: SSR renderer was not found

  Route: /dashboard

  The route has a layout but no matching SSR renderer.

  Fix: Ensure the page file exports a default component.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:1205`

---

### RUV1200–1299: API / Port Errors

| Code    | Title                              | Cause                              | Fix                                          |
| ------- | ---------------------------------- | ---------------------------------- | -------------------------------------------- |
| RUV1200 | API route execution failed         | API route handler threw            | Check route handler code, add error handling |
| RUV1201 | No available server port was found | All ports in range are in use      | Free a port or change `server.port` range    |
| RUV1202 | API renderer was not found         | API route has no matching renderer | Ensure route file exports handler            |

#### RUV1200 — API route execution failed

```
RUV1200: API route execution failed

  Route: /api/users
  Error: Handler threw an exception during execution.

  Fix: Check the API route handler code and add
       proper error handling.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:846`

#### RUV1201 — No available server port was found

```
RUV1201: No available server port was found

  The dev server tried all ports in the configured range
  but none were available.

  Fix: Free a port on your system or configure a
       different port range in ruvyxa.config.ts.
```

**Source**: `crates/ruvyxa_dev_server/src/port_binding.rs:97`

#### RUV1202 — API renderer was not found

```
RUV1202: API renderer was not found

  Route: /api/hello

  The route file does not export a compatible API handler.

  Fix: Ensure the route file exports a request handler
       (GET, POST, etc.).
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:1344`

---

### RUV1300–1399: Build / Compilation Errors

| Code    | Title                                | Cause                                   | Fix                      |
| ------- | ------------------------------------ | --------------------------------------- | ------------------------ |
| RUV1300 | Client hydration bundling failed     | Client bundle for hydration failed      | Check compilation errors |
| RUV1303 | Client route was not found           | Client bundle for CSR route missing     | Rebuild the application  |
| RUV1304 | Client bundle requested for non-page | Bundle requested for API/non-page route | Framework bug — report   |
| RUV1311 | MDX compilation error                | MDX file has invalid syntax             | Fix MDX syntax           |
| RUV1312 | Frontmatter YAML error               | MD/MDX frontmatter is invalid           | Fix YAML frontmatter     |

#### RUV1300 — Client hydration bundling failed

```
RUV1300: Client hydration bundling failed

  Error: The client bundle for hydration could not be built.

  Fix: Check the compilation output for syntax errors
       in your components.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:931`

#### RUV1303 — Client route was not found

```
RUV1303: Client route was not found

  Route: /dashboard (type: csr)

  The client bundle for this CSR route was not found in the
  build output.

  Fix: Rebuild the application.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:886`

#### RUV1304 — Client bundle requested for non-page route

```
RUV1304: Client bundle requested for a non-page route

  Route: /api/hello

  API routes do not have client bundles.

  Fix: This is likely a framework bug — report it.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:894`

#### RUV1311 — MDX compilation error

```
RUV1311: MDX compilation error

  File: app/blog/post.mdx:12

  The MDX file could not be compiled due to a syntax error.

  Fix: Check the MDX syntax around the indicated line.
```

**Source**: `packages/ruvyxa/runtime/compiler.mjs:1209`

#### RUV1312 — Frontmatter YAML error

```
RUV1312: Frontmatter YAML error

  File: app/blog/post.mdx

  The YAML frontmatter in this file is invalid or missing
  a closing delimiter.

  Fix: Ensure frontmatter is valid YAML with proper
       `---` delimiters.
```

**Source**: `packages/ruvyxa/runtime/compiler.mjs:1325`

---

### RUV1400–1499: Style Compilation Errors

| Code    | Title                                       | Cause                               | Fix                                  |
| ------- | ------------------------------------------- | ----------------------------------- | ------------------------------------ |
| RUV1400 | Tailwind CSS compilation failed             | Tailwind CLI error                  | Check tailwind config, content paths |
| RUV1401 | Tailwind CSS CLI was not found              | Missing Tailwind in node_modules    | Install tailwindcss                  |
| RUV1402 | Sass compilation failed                     | SCSS syntax error                   | Fix SCSS syntax                      |
| RUV1403 | Configured CSS entry was not found          | CSS file from `css.entries` missing | Check file path                      |
| RUV1404 | CSS entry must stay inside the project root | CSS path escapes project root       | Move CSS file into project           |

#### RUV1400 — Tailwind CSS compilation failed

```
RUV1400: Tailwind CSS compilation failed

  Error: Tailwind CSS CLI exited with code 1

  Fix: Check tailwind.config.ts for errors, ensure all
       configured content paths exist.
```

**Source**: `crates/ruvyxa_dev_server/src/style.rs:494`

#### RUV1401 — Tailwind CSS CLI not found

```
RUV1401: Tailwind CSS CLI was not found

  Ruvyxa uses the Tailwind CSS CLI directly for production
  builds, but it was not found in node_modules.

  Fix: Install Tailwind CSS:
       npm install -D tailwindcss @tailwindcss/postcss
```

**Source**: `crates/ruvyxa_dev_server/src/style.rs:471`

#### RUV1402 — Sass compilation failed

```
RUV1402: Sass compilation failed

  File: app/styles/custom.scss:24
  Error: Expected "{" after selector

  Fix: Check the SCSS syntax around line 24.
```

**Source**: `crates/ruvyxa_dev_server/src/style.rs:245`

#### RUV1403 — Configured CSS entry was not found

```
RUV1403: Configured CSS entry was not found at: ...

  CSS entry: ./src/styles/main.css

  The file specified in css.entries could not be found.

  Fix: Check that the CSS file exists at the specified path.
```

**Source**: `crates/ruvyxa_dev_server/src/style.rs:52, 209, 268`

#### RUV1404 — CSS entry outside project root

```
RUV1404: CSS entry must stay inside the project root

  Path: ../shared/styles.css

  CSS entries must be inside the project directory tree.

  Fix: Move the CSS file into the project or use a symlink.
```

**Source**: `crates/ruvyxa_dev_server/src/style.rs:188`

---

### RUV1500–1599: Render / Static Generation Errors

| Code    | Title                                                | Cause                                         | Fix                                 |
| ------- | ---------------------------------------------------- | --------------------------------------------- | ----------------------------------- |
| RUV1500 | SSG / action render failed                           | SSG page or server action threw               | Check logs, fix component           |
| RUV1501 | Route action file was not found                      | Action file for route is missing              | Create action file at expected path |
| RUV1510 | Static params must be array or object with params    | `getStaticParams` returned invalid shape      | Fix return type                     |
| RUV1511 | Static params shorthand needs single dynamic segment | String shorthand used for multi-segment route | Use object form                     |
| RUV1512 | Static params entry must be object or scalar         | Return value element has wrong type           | Return array of objects             |
| RUV1513 | Static params cache duration invalid                 | Cache duration format is wrong                | Use seconds or duration string      |
| RUV1550 | PPR render failed                                    | Partial pre-render error                      | Check component, reduce complexity  |

#### RUV1500 — SSG / action render failed

```
RUV1500: SSG render failed

  Worker: render-worker-2
  Status: exit code 1

  A render worker process crashed while handling a request.

  Fix: Check server logs for the crash reason. Common causes:
       - Out of memory
       - Unhandled exception in route handler
       - Native module incompatibility
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:320`

#### RUV1501 — Route action file was not found

```
RUV1501: Route action file was not found

  Route: /contact
  Expected: app/contact/action.ts

  The action file for this route does not exist.

  Fix: Create the action file at the expected path.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:972`

#### RUV1510 — Static params resolution failed

```
RUV1510: Static params must be an array or an object with a params array

  Route: /blog/[slug]
  getStaticParams returned: [{ slug: null }]

  Static params values must be strings or numbers.

  Fix: Filter out null/undefined values before returning.
```

**Source**: `packages/ruvyxa/runtime/worker-pool.mjs:614`

#### RUV1511 — Static params shorthand invalid

```
RUV1511: Static params shorthand at index requires exactly one dynamic route segment

  Route: /products/[category]/[id]
  getStaticParams returned: ["electronics"]

  String shorthand is only valid for routes with exactly
  one dynamic segment.

  Fix: Use object form: [{ category: "electronics", id: "123" }]
```

**Source**: `packages/ruvyxa/runtime/worker-pool.mjs:621`

#### RUV1512 — Static params shape invalid

```
RUV1512: Static params entry at index must be an object or scalar

  Route: /posts/[slug]
  getStaticParams returned: "not-an-array"

  getStaticParams must return an array of parameter objects.

  Fix: Return an array, e.g., [{ slug: "hello" }, { slug: "world" }]
```

**Source**: `packages/ruvyxa/runtime/worker-pool.mjs:629`

#### RUV1513 — Static params cache duration invalid

```
RUV1513: Static params cache must use seconds or a duration like 10m

  Route: /blog/[slug]
  cache: "forever"

  Cache duration must be a number (seconds) or a string like
  "10m", "1h", "1d".

  Fix: Use "365d" or 31536000 for one year.
```

**Source**: `packages/ruvyxa/runtime/worker-pool.mjs:644`

#### RUV1550 — PPR render failed

```
RUV1550: PPR render failed

  Route: /dashboard

  Partial pre-rendering encountered an error during the
  static shell generation.

  Fix: Check the component for dynamic data access during
       the static shell phase.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:697`

---

### RUV1600–RUV1603: Config Errors Documented by Current Source

| Code    | Title                                         | Cause                                                 | Fix                                      |
| ------- | --------------------------------------------- | ----------------------------------------------------- | ---------------------------------------- |
| RUV1600 | Config load failure                           | Config file threw or returned error                   | Check config syntax, run `ruvyxa doctor` |
| RUV1601 | Config field invalid                          | Config field has invalid value                        | Fix the field value                      |
| RUV1602 | Config shape, unknown field, or limit invalid | Unsupported shape/field or value above a source limit | Correct the config                       |
| RUV1603 | Adapter must provide build function           | `config.adapter` missing `build` method               | Ensure adapter exports `build(context)`  |

#### RUV1600 — Config load failure

```
RUV1600: Config load failure

  The configuration file threw an error during loading
  or validation.

  Fix: Check ruvyxa.config.ts for syntax errors and
       run `ruvyxa doctor` for diagnostics.
```

**Source**: `crates/ruvyxa_cli/src/runtime_config.rs`, `packages/ruvyxa/runtime/config-renderer.mjs`

#### RUV1601 — Config field invalid

```
RUV1601: Config field `security.actionLimit` must be greater than zero

  Field: security.actionLimit
  Value: 0

  Fix: Provide a positive value for the field.
```

**Source**: `crates/ruvyxa_cli/src/config.rs`, `packages/ruvyxa/runtime/config-renderer.mjs`

#### RUV1602 — Config shape, unknown field, or limit invalid

```
RUV1602: Config field `security.pluginLimit` must not exceed 268435456 bytes

  Field: security.pluginLimit
  Value: 99999999

  Fix: Reduce the value to within the allowed limit.
```

**Source**: `crates/ruvyxa_cli/src/config.rs`, `packages/ruvyxa/runtime/config-renderer.mjs`

#### RUV1603 — Adapter must provide build function

```
RUV1603: config.adapter must provide a build(context) function

  The adapter configuration does not include a build method.

  Fix: Ensure the adapter exports `build(context)` that
       returns an output object.
```

**Source**: `packages/ruvyxa/runtime/config-renderer.mjs:582`

---

### RUV1700–1799: Plugin Host & Worker Pool Errors

| Code    | Title                               | Cause                                           | Fix                                 |
| ------- | ----------------------------------- | ----------------------------------------------- | ----------------------------------- |
| RUV1700 | Plugin hook timed out / host exited | Plugin exceeded timeout or crashed              | Increase timeout or fix plugin code |
| RUV1701 | Plugin protocol error               | Invalid JSON, bad hook return, or unsafe path   | Usually framework bug — report      |
| RUV1702 | Worker pool script was not found    | Plugin runtime script missing from installation | Reinstall ruvyxa                    |
| RUV1704 | Worker pool stream error            | Worker pool stream communication error          | Check logs, framework bug — report  |

#### RUV1700 — Plugin hook timed out / host exited

```
RUV1700: TypeScript plugin hook `http.onRequest` timed out after 30000 ms

  Plugin: my-plugin
  Hook: http.onRequest

  The plugin exceeded middleware.timeoutMs.

  Fix: Reduce plugin work or increase middleware.timeoutMs.
```

```
RUV1700: TypeScript plugin host exited before responding (status: 1)

  The plugin host process crashed.

  Fix: Check plugin code for unhandled exceptions.
```

**Source**: `crates/ruvyxa_middleware/src/plugin_host.rs:480`,
`crates/ruvyxa_middleware/src/plugin_host.rs:524`

#### RUV1701 — Plugin protocol error

```
RUV1701: TypeScript plugin host returned invalid JSON

  The plugin host sent malformed JSON over the IPC channel.

  Fix: This is likely a framework or plugin bug — report it.
```

```
RUV1701: TypeScript request middleware returned an invalid result

  Plugin: my-plugin
  Hook: http.onRequest

  The `onRequest` handler must return undefined, a Request, or a Response.

  Fix: Check the return value of the onRequest handler.
```

```
RUV1701: Plugin returned an unsafe request path

  Plugin: my-plugin
  Path: //evil.com/steal

  The plugin tried to redirect to an unsafe destination.

  Fix: Ensure plugin redirect destinations are validated
       absolute paths or http(s) URLs.
```

**Source**: `crates/ruvyxa_middleware/src/plugin_host.rs:240`,
`crates/ruvyxa_dev_server/src/plugin_bridge.rs:240`

#### RUV1702 — Worker pool script was not found

```
RUV1702: Worker pool script was not found

  Script: plugin-runtime.mjs

  The TypeScript plugin host runtime script is missing from
  the ruvyxa start installation.

  Fix: Reinstall ruvyxa: npm install ruvyxa
```

**Source**: `crates/ruvyxa_dev_server/src/worker_pool.rs:877`

#### RUV1704 — Worker pool stream error

```
RUV1704: Worker pool stream error

  The worker pool encountered a stream communication error
  while handling a request.

  Fix: Check server logs for details. This may be a
       framework bug — report it.
```

**Source**: `crates/ruvyxa_dev_server/src/worker_pool.rs:307`

---

### RUV1800–1899: Compiler Errors

| Code    | Title                        | Cause                                   | Fix                                 |
| ------- | ---------------------------- | --------------------------------------- | ----------------------------------- |
| RUV1801 | Module resolution failed     | Import specifier could not be resolved  | Fix import path or install package  |
| RUV1802 | Oxc transform failed         | JavaScript/TypeScript transform error   | Fix syntax error in source file     |
| RUV1803 | Circular dependency detected | Two or more modules import each other   | Break the cycle with dynamic import |
| RUV1804 | Invalid JSX runtime          | `build.jsxRuntime` is not a valid value | Use `"classic"` or `"automatic"`    |

#### RUV1801 — Module resolution failed

```
RUV1801: cannot resolve 'missing-module' from app/page.tsx

  Specifier: missing-module
  Importer: app/page.tsx

  Fix: Check that the import path is correct and the module
       exists. Install missing packages.
```

**Source**: `packages/ruvyxa/runtime/compiler.mjs:393`

#### RUV1802 — Oxc transform failed

```
RUV1802: Oxc transform failed for app/page.tsx: syntax error

  Fix: Check the file for JavaScript/TypeScript syntax errors.
```

**Source**: `packages/ruvyxa/runtime/compiler.mjs:1499`

#### RUV1803 — Circular dependency detected

```
RUV1803: circular dependency detected: app/utils/a.ts -> app/utils/b.ts -> app/utils/a.ts

  Two or more modules form a circular import chain.

  Fix: Break the cycle by extracting shared logic into
       a separate module or using dynamic imports.
```

**Source**: `packages/ruvyxa/runtime/compiler.mjs:484`

#### RUV1804 — Invalid JSX runtime

```
RUV1804: JSX runtime must be `classic` or `automatic`, got `modern`

  Fix: Set build.jsxRuntime to "classic" or "automatic"
       in ruvyxa.config.ts.
```

**Source**: `packages/ruvyxa/runtime/compiler.mjs:1508`

---

### RUV2000–2102: Adapter / Config / Plugin Definition Errors

| Code    | Title                     | Cause                        | Fix                        |
| ------- | ------------------------- | ---------------------------- | -------------------------- |
| RUV2000 | BuildContext validation   | Adapter BuildContext invalid | Fix adapter configuration  |
| RUV2001 | Adapter option error      | Invalid adapter options      | Fix adapter options        |
| RUV2102 | Invalid plugin definition | `definePlugin()` type error  | Return valid plugin object |
| RUV2200 | Adapter build hook failed | Adapter `build()` threw      | Check adapter logs         |

#### RUV2000 — BuildContext validation failed

```
RUV2000: BuildContext.root is required and must be a non-empty string

  Adapter: vercelAdapter

  Fix: Ensure the adapter receives a valid BuildContext.
```

**Source**: `packages/@ruvyxa/core/src/utils.ts:147`

#### RUV2001 — Adapter option errors

```
[RUV2001] vercelAdapter: "regions" must be a non-empty array of region codes, such as ["sin1"]
[RUV2001] netlifyAdapter: "functionsDir" must not be an empty string
[RUV2001] cloudflareAdapter: "workerEntry" must be a string
[RUV2001] staticAdapter: "outputDir" overlaps protected build output
[RUV2001] firebaseAdapter: "functionName" must be a valid JavaScript identifier
[RUV2001] renderAdapter: "serviceName" must contain lowercase letters, digits, or hyphens
[RUV2001] bunAdapter: "entry" must not be an empty string
[RUV2001] nodeAdapter: "entry" must be a string
[RUV2001] awsAdapter: package version must be valid semantic version metadata
```

Each adapter validates its options at build time. Errors follow
`[RUV2001] <adapterName>: <description>` format.

#### RUV2102 — Invalid plugin definition

```
RUV2102: Ruvyxa plugin must be an object.
RUV2102: Ruvyxa plugin must have a non-empty name.
RUV2102: Ruvyxa plugin "my-plugin" register must be a function.
RUV2102: Ruvyxa plugin "my-plugin" must declare behavior or provide register(api).
RUV2102: Ruvyxa plugin "my-plugin" http must be an object.
RUV2102: Ruvyxa plugin "my-plugin" http.onRequest must be a function.
RUV2102: Ruvyxa plugin "my-plugin" build.onResolve must be a function.
RUV2102: Ruvyxa plugin "my-plugin" head.attrs has an invalid attribute name.
RUV2102: Ruvyxa plugin "my-plugin" head.children is only supported on script, style, noscript.
```

**Source**: `packages/@ruvyxa/core/src/plugin.ts:62-300`

All validation errors from `definePlugin()` use `RUV2102` prefix. The error message pinpoints the
exact field that failed validation.

#### RUV2200 — Adapter build hook failed

```
RUV2200: Adapter build hook failed

  Adapter: vercelAdapter

  The adapter's build() function threw an error.

  Fix: Check the adapter logs for details. This may indicate
       a misconfiguration or platform issue.
```

**Source**: `crates/ruvyxa_cli/src/runtime_config.rs`, `packages/ruvyxa/runtime/adapter-runner.mjs`

---

### RUV3000–3201: Official Package Errors

#### @ruvyxa/database errors

| Code    | Title                    | Cause                           | Fix                    |
| ------- | ------------------------ | ------------------------------- | ---------------------- |
| RUV3001 | Database operation error | Invalid args, model name unsafe | Check query parameters |
| RUV3002 | Adapter error            | Adapter-specific failure        | Check adapter logs     |
| RUV3003 | Connection failed        | Database unreachable            | Check DATABASE_URL     |

#### @ruvyxa/auth errors

| Code    | Title                  | Cause                        | Fix                      |
| ------- | ---------------------- | ---------------------------- | ------------------------ |
| RUV3100 | Auth service error     | Magic link delivery failed   | Check email provider     |
| RUV3101 | Auth request invalid   | Cross-origin, body too large | Fix request, reduce body |
| RUV3102 | Too many attempts      | Rate limit bucket exhausted  | Wait for `Retry-After`   |
| RUV3103 | OAuth state invalid    | State mismatch or expired    | Re-authenticate          |
| RUV3104 | OAuth provider error   | Token/profile request failed | Check provider           |
| RUV3105 | Production store error | Non-durable store            | Use persistent store     |

#### @ruvyxa/realtime errors

| Code    | Title          | Cause              | Fix                  |
| ------- | -------------- | ------------------ | -------------------- |
| RUV3201 | Realtime error | Protocol violation | Check message format |

---

## Error Boundaries

### `error.tsx` — Component Signature

Create `error.tsx` in any route segment to catch rendering errors and show a fallback UI.

```typescript
// app/error.tsx
'use client'

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string }
  reset: () => void
}) {
  return (
    <div className="error-container">
      <h1>Something went wrong</h1>
      <p>{error.message}</p>
      <p>Error ID: {error.digest}</p>
      <button onClick={() => reset()}>Try again</button>
    </div>
  )
}
```

**Props**:

- `error`: The caught error object. Has optional `digest` property that maps to server-side error
  logs.
- `reset`: Function to re-render the route segment. Does NOT reload the page — re-executes the route
  component.

### Error Boundary Hierarchy

```
         ┌──────────────────┐
         │  Root layout     │
         │  error.tsx       │  ← Catches all unhandled errors
         └────────┬─────────┘
                  │
         ┌────────▼─────────┐
         │  Blog layout     │
         │  error.tsx       │  ← Catches blog errors only
         └────────┬─────────┘
                  │
         ┌────────▼─────────┐
         │  Blog [slug]     │
         │  error.tsx       │  ← Catches specific post errors
         └──────────────────┘
```

**Resolution rules**:

1. Error bubbles up to the nearest `error.tsx` in the route tree
2. If no `error.tsx` in the route segment, bubbles to parent layout
3. If root layout has no `error.tsx`, falls back to framework default error page
4. `error.tsx` must be a Client Component (`'use client'`)
5. `reset()` re-renders only the error boundary's children, not siblings

### Nested Error Boundaries

```tsx
// Each route segment can have its own error boundary
app/
├── error.tsx                    // Catches root + unmatched routes
├── blog/
│   ├── error.tsx                // Catches all /blog/* errors
│   └── [slug]/
│       └── error.tsx            // Catches specific post errors
```

When error occurs in `/blog/my-post`:

1. Checks `app/blog/[slug]/error.tsx` — if exists, renders it
2. If not, checks `app/blog/error.tsx`
3. If not, checks `app/error.tsx`
4. If none, shows the built-in error page (white screen with error info in dev)

---

## 404 Pages: `not-found.tsx`

### Component Signature

```tsx
// app/not-found.tsx
export default function NotFound() {
  return (
    <main>
      <h1>404 — Page Not Found</h1>
      <p>The page you are looking for does not exist.</p>
      <a href="/">Go home</a>
    </main>
  )
}
```

`not-found.tsx` does NOT need `'use client'`. It can be a Server Component or Client Component.

### Scoped 404 Pages

```tsx
// app/blog/not-found.tsx — only affects /blog/* routes
export default function BlogNotFound() {
  return <p>That blog post does not exist.</p>
}
```

### Programmatic `notFound()` Trigger

```typescript
import { notFound } from '@ruvyxa/react'

export default async function ProductPage({ params }: { params: { id: string } }) {
  const product = await getProduct(params.id)
  if (!product) {
    notFound()  // throws, caught by nearest not-found.tsx
    // This line never executes
  }
  return <div>{/* render product */}</div>
}
```

### not-found Hierarchy

```
Request to /blog/nonexistent

  app/blog/[slug]/page.tsx         ← route handler
        │
        ├── calls notFound()
        │
        ▼
  app/blog/not-found.tsx           ← closest not-found (route segment)
        │
        (if missing)
        ▼
  app/not-found.tsx                ← root fallback
        │
        (if missing)
        ▼
  Built-in 404 page                ← framework default
```

**Resolution rules**:

1. `notFound()` triggers the closest `not-found.tsx` in the route hierarchy
2. Skips layouts with unmatched path segments
3. If the route has no `not-found.tsx`, uses parent layout's (if parent's path matches)
4. Root `app/not-found.tsx` is the final fallback
5. Unlike `error.tsx`, `not-found.tsx` can be a Server Component

### When `notFound()` Fires

- Explicit call in page/layout/action: `notFound()`
- Route file does not exist (request to non-existent path)
- Dynamic route param fails to match configured `getStaticParams` in production
- Server action called on non-existent resource

---

## Loading States: `loading.tsx`

### Component Signature

```tsx
// app/loading.tsx
export default function Loading() {
  return <p>Loading...</p>
}
```

### Suspense Fallback Integration

```tsx
// app/loading.tsx
export default function Loading() {
  return (
    <div className="skeleton-list">
      {[1, 2, 3].map((i) => (
        <div key={i} className="skeleton-item" />
      ))}
    </div>
  )
}
```

`loading.tsx` wraps the page component in `<Suspense fallback={<Loading />}>`. It shows during:

- Server component Data fetching (async component)
- Dynamic import loading
- Stream completion for SSR

### Scoped Loading

```tsx
// app/blog/loading.tsx — only for /blog/* pages
export default function BlogLoading() {
  return <div>Loading blog...</div>
}
```

### Loading Hierarchy

Same as error boundaries: nearest `loading.tsx` in the route tree wraps the segment. If no
`loading.tsx`, no Suspense boundary is added.

---

## Server Action Errors

### Validation Pattern

```typescript
// app/actions/register/action.ts
'use server'

import { action } from 'ruvyxa/server'

export const registerUser = action(async (formData: FormData) => {
  const email = formData.get('email') as string
  const password = formData.get('password') as string

  // Validation errors
  const errors: Record<string, string> = {}
  if (!email || !email.includes('@')) errors.email = 'Invalid email'
  if (!password || password.length < 8) errors.password = 'Too short (min 8)'

  if (Object.keys(errors).length > 0) {
    return { success: false, errors }
  }

  // Business logic error
  try {
    await createUser({ email, password })
    return { success: true }
  } catch (err) {
    return {
      success: false,
      errors: { email: 'This email is already registered' },
    }
  }
})
```

### Client Error Handling

```tsx
'use client'

import { useActionState } from 'react'

export function RegisterForm() {
  const [state, action, pending] = useActionState(registerUser, null)

  return (
    <form action={action}>
      <input name="email" type="email" />
      {state?.errors?.email && <p className="error">{state.errors.email}</p>}
      <input name="password" type="password" />
      {state?.errors?.password && <p className="error">{state.errors.password}</p>}
      <button type="submit" disabled={pending}>
        Register
      </button>
    </form>
  )
}
```

### Action Thrown Errors

If a server action throws (not returns) an error:

```typescript
throw new Error('Unauthorized')
// → RUV1500: SSG / action render failed
// → Client receives 500 with error digest in production
// → In dev, error overlay shows the stack trace
```

Thrown errors in actions should be avoided — return structured error objects instead.

---

## API Route Error Responses

### Standard Error Response Format

```typescript
// app/api/users/route.ts
export async function GET(request: Request) {
  try {
    const users = await fetchUsers()
    return Response.json(users)
  } catch (err) {
    return Response.json(
      {
        error: 'Failed to fetch users',
        code: 'RUV1200',
        details: {},
        requestId: 'req_abc123',
      },
      { status: 500 },
    )
  }
}
```

### Error Response Schema

```json
{
  "error": "Human-readable message",
  "code": "RUV1200",
  "details": {},
  "requestId": "req_abc123"
}
```

| Field       | Type   | Description                                    |
| ----------- | ------ | ---------------------------------------------- |
| `error`     | string | Human-readable error description               |
| `code`      | string | RUV#### error code                             |
| `details`   | object | Optional structured error data                 |
| `requestId` | string | If observability plugin active, the request ID |

### HTTP Status Code Guidelines

| Situation        | Status | Code    |
| ---------------- | ------ | ------- |
| Validation error | 400    | —       |
| Unauthorized     | 401    | —       |
| Not found        | 404    | —       |
| Internal error   | 500    | RUV1200 |
| Plugin timeout   | 500    | RUV1700 |
| Adapter error    | 502    | RUV2200 |

---

## Error Prevention Patterns

### Guard Against RUV1007 (Boundary Violations)

```typescript
// ❌ Bad — imports server-only package in client component
'use client'
import { db } from '../lib/db' // RUV1007

// ✅ Good — server action handles DB, client calls it
;('use client')
import { fetchUsers } from '../actions/users'

// ✅ Good — use /client subpath for auth
;('use client')
import { createAuthClient } from '@ruvyxa/auth/client'
```

### Guard Against RUV1008 (Env Leaks)

```typescript
// ❌ Bad — private env in client-reachable code
const url = process.env.DATABASE_URL // RUV1008

// ✅ Good — public env (prefixed)
const apiUrl = process.env.RUVYXA_PUBLIC_API_URL

// ✅ Good — private env accessed only in server actions
;('use server')
export const getData = action(async () => {
  const url = process.env.DATABASE_URL // safe here
})
```

### Guard Against RUV1510-1513 (Static Params)

```typescript
// ✅ Good — always return valid params
export const getStaticParams: GetStaticParams = async () => {
  const posts = await fetchPosts()
  return posts.filter((p) => p.slug).map((p) => ({ slug: p.slug }))
}

// ❌ Bad — returns null slugs
export const getStaticParams = async () => {
  return [{ slug: null }] // RUV1510
}

// ❌ Bad — string shorthand for multi-segment route
// Route: /[category]/[id]
export const getStaticParams = async () => {
  return ['electronics'] // RUV1511 — use [{ category: 'electronics', id: '123' }]
}
```

### Guard Against Config Errors

```typescript
// ✅ Good — use the config() helper for type checking
import { config } from 'ruvyxa/config'
export default config({
  server: { port: 3000 },
})

// ❌ Bad — typos are not caught
export default {
  server: { port: '3000' }, // RUV1602 — shape/type mismatch
  build: { split: 'none' }, // RUV1601 — unknown value
}
```

---

## Dev Server Error Overlay

### Overlay Format

During development (`ruvyxa dev`), errors show as an in-browser overlay:

```
┌──────────────────────────────────────────┐
│  ⚠  RUV1008: Private env variable leak  │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │  process.env.DATABASE_URL          │  │
│  │  at UserCard.tsx:12                │  │
│  │                                    │  │
│  │  import { db } from "../lib/db";   │  │
│  │  const url = process.env.DATABASE… │  │
│  │                    ^^^^^^^^^^^^^   │  │
│  │                                    │  │
│  │  Fix:                               │  │
│  │    → Rename to RUVYXA_PUBLIC_...   │  │
│  │    → Move to server component       │  │
│  └────────────────────────────────────┘  │
│                                          │
│  [Dismiss] [View full log]               │
└──────────────────────────────────────────┘
```

### Overlay Elements

1. **Header**: Error code + title with icon (`⚠` for warning, `✕` for error)
2. **Code snippet**: Relevant source code with error location highlighted
3. **Fix suggestion**: Quick resolution steps
4. **Actions**: Dismiss (close overlay), View full log (opens terminal)

### Configuring the Overlay

```typescript
// ruvyxa.config.ts
export default config({
  debug: {
    overlay: true, // show overlay in dev (default)
    // overlay: false, // disable overlay, log to console instead
    traces: true, // show stack traces (default: false)
  },
})
```

### Overlay Behavior

- Shows on: compilation errors, boundary violations, runtime errors during render
- Does NOT show on: 404s, successful compilations with warnings, API route errors
- Multiple errors: stacked overlay, dismiss one at a time
- React strict mode: double-render warnings shown as console warnings, not overlay

### Disabling in Production

```typescript
export default config({
  debug: {
    overlay: false, // always disable in production
  },
})
```

In production, errors return HTTP 500 or show `error.tsx` fallback. The overlay is never loaded.

---

## Common Error Patterns by Development Phase

### During `ruvyxa dev`

| Symptom                    | Likely Error                                     | Action                                   |
| -------------------------- | ------------------------------------------------ | ---------------------------------------- |
| Error overlay on page load | RUV1300 (hydration bundling), RUV1007 (boundary) | Check file indicated in overlay          |
| Page renders blank         | RUV1100 (SSR error in component)                 | Add `error.tsx`, check browser console   |
| HMR not updating           | Refresh browser, check network                   | —                                        |
| Slow page loads            | RUV1100 (SSR render slow)                        | Optimize component, reduce data fetching |
| 404 for existing route     | Route not exported                               | Check route naming, file location        |
| 500 on form submit         | RUV1500 (action error)                           | Check action code, add error handling    |
| Plugin not running         | RUV1602 (plugin/config shape invalid)            | Check plugin configuration               |

### During `ruvyxa build`

| Symptom                                  | Likely Error                           | Action                                 |
| ---------------------------------------- | -------------------------------------- | -------------------------------------- |
| Build fails immediately                  | RUV1600 (config load failure)          | Run `ruvyxa doctor`                    |
| Build fails during compilation           | RUV1802 (Oxc transform), RUV1311 (MDX) | Fix indicated syntax error             |
| Build fails at module resolution         | RUV1801 (module not resolved)          | Install package or fix import path     |
| Build succeeds but output missing routes | RUV1002 (invalid route segment)        | Check filenames for invalid characters |
| Build succeeds but circular deps found   | RUV1803 (circular dependency)          | Break the dependency cycle             |
| Build OOM                                | Not a numbered error                   | Reduce `build.workers`, increase RAM   |

### During `ruvyxa check`

| Symptom                      | Likely Error          | Action                       |
| ---------------------------- | --------------------- | ---------------------------- |
| Boundary violations reported | RUV1007-1010          | Restructure imports          |
| Route conflicts reported     | RUV1003 (conflicting) | Rename conflicting files     |
| Config validation errors     | RUV1600-1603          | Fix config file              |
| SSG params missing           | RUV1510-1513          | Add/export `getStaticParams` |

### During Deployment

| Symptom                  | Likely Error                  | Action                       |
| ------------------------ | ----------------------------- | ---------------------------- |
| Build fails in CI        | RUV2200 (adapter hook failed) | Check adapter logs           |
| 502 on all routes        | RUV2200 (adapter build error) | Check adapter configuration  |
| Static site has no pages | RUV2200 (adapter mismatch)    | Use correct adapter          |
| Functions timeout        | RUV1700 (plugin timeout)      | Increase timeout or optimize |
| Cold starts are slow     | Not a numbered error          | Use `build.warm: true`       |

---

## Quick Reference

### Error Code Ranges

```
Range       Category          Where to look
──────────  ────────────────  ──────────────────────
RUV1001     Route discovery   03-server-client-components.md
RUV1007     Boundary          bundler boundary check
RUV1010     Boundary          bundler boundary check
RUV1101     SSR               ssr-renderer.mjs
RUV1205     Prerender         prerender.rs
RUV1400     Style             dev_server style path
RUV1550     PPR               dev_server render path
RUV1600-1603 Config            config renderer and CLI validation
RUV1700-1701 Plugin host       middleware plugin bridge
RUV2200-2203 Adapter           adapter runner
RUV2200     Adapter build     runtime_config.rs, adapter-runner.mjs
RUV3001     Database          15-official-packages.md
RUV3100     Auth              15-official-packages.md
RUV3201     Realtime          15-official-packages.md
```

### File Locations

| Error source            | File                                                                         |
| ----------------------- | ---------------------------------------------------------------------------- |
| Bundler boundary checks | `crates/ruvyxa_bundler/src/boundary.rs`                                      |
| Graph route validation  | `crates/ruvyxa_graph/src/lib.rs`                                             |
| Dev server rendering    | `crates/ruvyxa_dev_server/src/render_pipeline.rs`                            |
| Worker pool             | `crates/ruvyxa_dev_server/src/worker_pool.rs`                                |
| Style compilation       | `crates/ruvyxa_dev_server/src/style.rs`                                      |
| Plugin host             | `crates/ruvyxa_middleware/src/plugin_host.rs`                                |
| Config validation       | `crates/ruvyxa_cli/src/config.rs`, `crates/ruvyxa_cli/src/runtime_config.rs` |
| Plugin bridge           | `crates/ruvyxa_dev_server/src/plugin_bridge.rs`                              |
| Plugin validation       | `packages/@ruvyxa/core/src/plugin.ts`                                        |
| Compiler (JS)           | `packages/ruvyxa/runtime/compiler.mjs`                                       |
| Config renderer (JS)    | `packages/ruvyxa/runtime/config-renderer.mjs`                                |
| Worker pool (JS)        | `packages/ruvyxa/runtime/worker-pool.mjs`                                    |
| SSR renderer (JS)       | `packages/ruvyxa/runtime/ssr-renderer.mjs`                                   |
| API renderer (JS)       | `packages/ruvyxa/runtime/api-renderer.mjs`                                   |
| Auth errors             | `packages/@ruvyxa/auth/src/index.ts`                                         |
| Database errors         | `packages/@ruvyxa/database/src/index.ts`                                     |
| Realtime errors         | `packages/@ruvyxa/realtime/src/plugin.ts`                                    |
| Adapter errors          | `packages/@ruvyxa/adapter-*/src/index.ts`                                    |

---

## Troubleshooting

| Symptom                        | Likely cause                    | Fix                                      |
| ------------------------------ | ------------------------------- | ---------------------------------------- |
| Blank white page               | Uncaught client error           | Check browser console, add `error.tsx`   |
| RUV1003 after adding file      | Two files match same URL        | Remove duplicate route file              |
| RUV1008 on build               | Private env in client component | Rename to `RUVYXA_PUBLIC_*`              |
| RUV1600 on project creation    | Typo in config                  | Run `ruvyxa doctor`                      |
| RUV1801 for local import       | Incorrect import path           | Use relative path or alias               |
| RUV1100 after deploy           | SSR render failure              | Check page component, dependencies       |
| RUV1500 on worker pool         | SSG / action render failed      | Check server logs, restart               |
| RUV1501 on render              | Route action file missing       | Create action file at expected path      |
| RUV1700 with plugins           | Plugin timeout or crash         | Increase timeout or fix plugin code      |
| RUV2200 on build               | Adapter build hook failed       | Check adapter logs, configuration        |
| 404 on existing route          | Route not exported              | Add `export default function Page()`     |
| 500 on server action           | Validation failed               | Check action error return                |
| Error overlay not showing      | `debug.overlay: false`          | Enable in config                         |
| Error overlay always showing   | Persistent compile error        | Fix the error in your code               |
| `notFound()` does nothing      | No `not-found.tsx`              | Create the file or check hierarchy       |
| `reset()` does not clear error | State persists                  | Use `key` prop to force remount          |
| WebSocket (HMR) disconnects    | Network proxy/firewall          | Check WebSocket path, restart dev server |

---

## Read a Diagnostic as a Boundary Signal

Ruvyxa diagnostics are emitted by the subsystem that observed the problem. Start from the code and
message, then move to the owning boundary rather than treating numeric ranges as a promise that
every number exists. The current high-value groups include:

| Area                    | Examples                                              | First place to inspect                                                |
| ----------------------- | ----------------------------------------------------- | --------------------------------------------------------------------- |
| Route discovery         | `RUV1001`, `RUV1002`, `RUV1004`                       | `appDir`, route entry filename, dynamic-segment shape, default export |
| Client/server boundary  | `RUV1007`, `RUV1008`, `RUV1009`, `RUV1010`            | The diagnostic file and its reachable relative imports                |
| Content and styles      | `RUV1310`–`RUV1312`, `RUV1402`, `RUV1403`             | Markdown/MDX frontmatter, Sass source, or stylesheet import path      |
| Configuration           | `RUV1601`, `RUV1602`                                  | The named config field and its documented range/path constraint       |
| Plugin/adapter contract | `RUV2102`, `RUV2200`, `RUV2202`, `RUV2203`, `RUV2210` | Plugin definition or selected target/adapter package                  |

Use the smallest command that exposes the same boundary:

```bash
ruvyxa routes
ruvyxa trace /the-route-pattern
ruvyxa analyze --format human
ruvyxa doctor --json
```

`routes` answers discovery, `trace` answers one manifest entry, `analyze` answers route/import
validation, and `doctor` answers environment/configuration/adapter compatibility. None of these
commands automatically repairs source or config; preserve the diagnostic's file/path context while
making the smallest correction.

### A Safe Escalation Sequence

1. Reproduce with the narrow command that owns the failure.
2. Read the exact file and import/config edge named by the result.
3. Change one cause, not several unrelated settings.
4. Re-run the focused command.
5. Run `npm run check` when the failure crosses route, render, or type boundaries.

Avoid logging secrets, full environment dumps, or private request payloads to "get more detail". The
framework's boundary diagnostics are intentionally designed to identify a source location and a safe
direction without requiring sensitive data.

---

## Error Code Ranges

```
RUV1001-1099  →  Boundary / Graph      — server/client boundary, env leak, route discovery
RUV1100-1199  →  SSR / Render           — React SSR, renderer discovery
RUV1200-1299  →  API / Server Runtime   — API route, port binding, renderer
RUV1300-1399  →  Bundle / Compilation   — hydration bundling, client route, MDX
RUV1400-1499  →  Style                  — Tailwind, Sass, CSS entries
RUV1500-1599  →  Worker / Static Params — render worker, actions, static params, PPR
RUV1600-1699  →  Config / Adapter       — config loading, validation, range, build()
RUV1700-1799  →  Plugin Bridge          — plugin hook timeout, protocol, worker pool
RUV1800-1899  →  JS Runtime             — module resolution, Oxc transform, circular deps
RUV2000-2200  →  Adapter / Plugin Def   — BuildContext, options, definePlugin, build hook
RUV3000-3201  →  Official Packages      — database, auth, realtime
```

---

## Using `notFound()` and `redirect()`

```ts
// app/blog/[slug]/page.tsx
import { notFound, redirect } from 'ruvyxa/server';

export default async function BlogPost({ params }: { params: { slug: string } }) {
  const post = await db.post.findUnique({ where: { slug: params.slug } });

  if (!post) {
    notFound();              // → แสดง not-found.tsx (404)
  }

  if (post.redirectTo) {
    redirect(post.redirectTo, 301);  // → redirect (301 permanent)
  }

  if (!post.published) {
    if (!isAdmin) {
      redirect('/blog/drafts', 302);  // → redirect (302 temporary)
    }
  }

  return <article>{post.title}</article>;
}
```

**redirect status codes**:

- `redirect(path)` — 307 (temporary)
- `redirect(path, 301)` — 301 (permanent)
- `redirect(path, 302)` — 302 (found)
- `redirect(path, 308)` — 308 (permanent, preserve method)

---

## ดู Error Codes ทั้งหมด

```bash
# CLI tools สำหรับ debug
ruvyxa doctor            # ตรวจสอบทุกอย่าง — config, routes, boundary, env
ruvyxa doctor --json     # รับ compatibility report เป็น JSON
ruvyxa check             # TypeScript check (เมื่อมี tsconfig) และ parity test
ruvyxa analyze           # ตรวจ routes, imports และ server/client boundary
ruvyxa trace /           # ดู route manifest entry ของ path ที่ระบุ
```

**Output `ruvyxa doctor`**:

```
━━━ Ruvyxa Doctor ━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Version:   0.1.0
  Node:      22.4.0
  Platform:  win32

  Config:
    ✓ ruvyxa.config.ts found
    ✓ All fields valid
    ✓ Site URL set

  Routes:
    ✓ 12 routes registered
    ✓ No ambiguous routes
    ✓ All pages have metadata

  Boundary:
    ✓ 0 violations (server → client)
    ✓ All server directives correct
    ✓ Private env vars not exposed

  Plugins:
    ✓ 3 plugins registered
    ✓ All plugin configs valid
    ✓ Adapter: vercel

  Ready for production ✓
```

---

## ตาราง Error Codes — ทุก code

| Code        | Title (Thai)                        | ช่วง              | Severity |
| ----------- | ----------------------------------- | ----------------- | -------- |
| **RUV1001** | ไม่พบไดเรกทอรี app                  | Boundary / Graph  | Error    |
| **RUV1002** | Segment route dynamic ไม่ถูกต้อง    | Boundary / Graph  | Error    |
| **RUV1003** | เส้นทาง route ขัดแย้งกัน            | Boundary / Graph  | Error    |
| **RUV1004** | Page ไม่มี default export           | Boundary / Graph  | Error    |
| **RUV1007** | โมดูล server-only ใน client graph   | Boundary / Graph  | Error    |
| **RUV1008** | ตัวแปร env ส่วนบุคคลรั่วไหล         | Boundary / Graph  | Error    |
| **RUV1009** | โมดูล client-only ใน server graph   | Boundary / Graph  | Error    |
| **RUV1010** | ไฟล์ใน server/ ถึง client graph     | Boundary / Graph  | Error    |
| **RUV1100** | React SSR ล้มเหลว                   | SSR / Render      | Error    |
| **RUV1101** | SSR renderer ขาดพารามิเตอร์         | SSR / Render      | Error    |
| **RUV1102** | ไม่พบ SSR renderer                  | SSR / Render      | Error    |
| **RUV1200** | การเรียก API route ล้มเหลว          | API / Server      | Error    |
| **RUV1201** | ไม่พบพอร์ตเซิร์ฟเวอร์ที่ว่าง        | API / Server      | Error    |
| **RUV1202** | ไม่พบ API renderer                  | API / Server      | Error    |
| **RUV1300** | Client hydration bundling ล้มเหลว   | Bundle / Compile  | Error    |
| **RUV1303** | ไม่พบ client route                  | Bundle / Compile  | Error    |
| **RUV1304** | Client bundle สำหรับ non-page route | Bundle / Compile  | Error    |
| **RUV1311** | MDX compilation error               | Bundle / Compile  | Error    |
| **RUV1312** | Frontmatter YAML error              | Bundle / Compile  | Error    |
| **RUV1400** | Tailwind CSS compilation ล้มเหลว    | Style             | Error    |
| **RUV1401** | ไม่พบ Tailwind CSS CLI              | Style             | Error    |
| **RUV1402** | Sass compilation ล้มเหลว            | Style             | Error    |
| **RUV1403** | ไม่พบ CSS entry ที่กำหนด            | Style             | Error    |
| **RUV1404** | CSS entry ต้องอยู่ใน project root   | Style             | Error    |
| **RUV1500** | SSG/action render ล้มเหลว           | Worker / Params   | Error    |
| **RUV1501** | ไม่พบไฟล์ route action              | Worker / Params   | Error    |
| **RUV1510** | Static params รูปแบบผิด             | Worker / Params   | Error    |
| **RUV1511** | String shorthand ไม่ถูกต้อง         | Worker / Params   | Error    |
| **RUV1512** | Static params entry ผิด             | Worker / Params   | Error    |
| **RUV1513** | Static params cache duration ผิด    | Worker / Params   | Error    |
| **RUV1550** | PPR render ล้มเหลว                  | Worker / Params   | Error    |
| **RUV1600** | การโหลด config ล้มเหลว              | Config / Adapter  | Error    |
| **RUV1601** | ค่าฟิลด์ config ไม่ถูกต้อง          | Config / Adapter  | Error    |
| **RUV1602** | ค่าฟิลด์ config เกินค่าสูงสุด       | Config / Adapter  | Error    |
| **RUV1603** | Adapter ต้องมี build()              | Config / Adapter  | Error    |
| **RUV1700** | Plugin hook timeout / host หยุด     | Plugin Bridge     | Error    |
| **RUV1701** | Plugin protocol error               | Plugin Bridge     | Error    |
| **RUV1702** | ไม่พบ Worker pool script            | Plugin Bridge     | Error    |
| **RUV1704** | Worker pool stream error            | Plugin Bridge     | Error    |
| **RUV1801** | ไม่สามารถ resolve โมดูล             | JS Runtime        | Error    |
| **RUV1802** | Oxc transform ล้มเหลว               | JS Runtime        | Error    |
| **RUV1803** | Circular dependency                 | JS Runtime        | Error    |
| **RUV1804** | JSX runtime ไม่ถูกต้อง              | JS Runtime        | Error    |
| **RUV2000** | BuildContext validation ล้มเหลว     | Adapter / Plugin  | Error    |
| **RUV2001** | ค่า options ของ adapter ไม่ถูกต้อง  | Adapter / Plugin  | Error    |
| **RUV2102** | Plugin definition ไม่ถูกต้อง        | Adapter / Plugin  | Error    |
| **RUV2200** | Adapter build hook ล้มเหลว          | Adapter / Plugin  | Error    |
| **RUV3001** | Database operation error            | Official Packages | Error    |
| **RUV3002** | Database adapter error              | Official Packages | Error    |
| **RUV3003** | Database connection failed          | Official Packages | Error    |
| **RUV3100** | Auth service error                  | Official Packages | Error    |
| **RUV3101** | Auth request invalid                | Official Packages | Error    |
| **RUV3102** | Too many authentication attempts    | Official Packages | Error    |
| **RUV3103** | OAuth state invalid                 | Official Packages | Error    |
| **RUV3104** | OAuth provider error                | Official Packages | Error    |
| **RUV3105** | Production store error              | Official Packages | Error    |
| **RUV3201** | Realtime error                      | Official Packages | Error    |

---

## Troubleshooting — Quick Reference

| Error Code(s)       | ปัญหาที่พบบ่อย      | วิธีแก้ด่วน                                         |
| ------------------- | ------------------- | --------------------------------------------------- |
| RUV1007-1010        | Boundary violations | ตรวจ `'use client'`, imports, env vars              |
| RUV1001-1004        | Route discovery     | ตรวจ `app/` directory, route names, default exports |
| RUV1100-1102        | SSR / Render        | ตรวจ component, `error.tsx`, default export         |
| RUV1200-1202        | API / Port          | ตรวจ route handler, port config, try/catch          |
| RUV1300, 1303-1304  | Bundle / Hydration  | `ruvyxa clean && ruvyxa build`, ดู error message    |
| RUV1311-1312        | MDX / Frontmatter   | ตรวจ syntax MDX และ YAML                            |
| RUV1400-1404        | Style / CSS         | ตรวจ Tailwind config, SCSS syntax, css.entries      |
| RUV1500-1501        | Render / Action     | ดู server logs, ตรวจ action files                   |
| RUV1510-1513        | Static params       | ตรวจ `getStaticParams` return shape                 |
| RUV1550             | PPR                 | ตรวจ component ใน static shell phase                |
| RUV1600-1603        | Config / Adapter    | ตรวจ config fields, adapter implements build()      |
| RUV1700, 1702, 1704 | Plugin bridge       | รีสตาร์ท dev server, `npm install ruvyxa`           |
| RUV1701             | Plugin protocol     | อัปเดต Ruvyxa, ตรวจ plugin compatibility            |
| RUV1801-1804        | JS Runtime          | ตรวจ import paths, syntax, circular deps            |
| RUV2000-2001        | Adapter config      | ตรวจ BuildContext และ adapter options               |
| RUV2102             | Plugin definition   | ตรวจ `definePlugin()` return value                  |
| RUV2200             | Build hook          | ตรวจ adapter compatibility                          |
| RUV3001-3003        | Database            | ตรวจ DATABASE_URL, adapter logs                     |
| RUV3100-3105        | Auth                | ตรวจ provider config, OAuth state                   |
| RUV3201             | Realtime            | ตรวจ message format                                 |

---

## Try It Yourself

1. สร้าง `app/error.tsx` พร้อม UI สวยงาม — แสดง error message, digest, reset button
2. สร้าง `app/not-found.tsx` พร้อมลิงก์กลับหน้าแรก และ search
3. สร้าง `app/loading.tsx` — spinner + skeleton
4. ทดลองสร้าง RUV1007 โดย import server module ใน client component — ดู error
5. ใช้ `notFound()` ใน dynamic route — ดู 404 page
6. จัดการ error ใน server action ด้วย try/catch — return error object
7. ใช้ `RuvyxaError` class ใน custom error
8. เปิด `debug.overlay: false` ถ้าไม่ต้องการ error overlay
9. รัน `ruvyxa doctor` — ดู error ทั้งหมดในแอป
10. ทดสอบ API route error handling — ส่ง POST ผิด format
11. ตรวจสอบ bundle budget — เพิ่ม `bundleBudget` plugin
12. ดู error overlay ใน dev mode — ทำ intentional error

---

## Summary

- Error codes: RUV1001-3201 — แบ่งเป็น 11 ช่วง: boundary/graph, SSR/render, API/server,
  bundle/compile, style, worker/params, config/adapter, plugin bridge, JS runtime, adapter/plugin
  def, official packages
- รวม 50+ error codes แต่ละตัวมี: code, title, คำอธิบาย, error text, วิธีแก้
- Error boundary: `error.tsx` (catch errors), `not-found.tsx` (404), `loading.tsx` (loading state)
- Error overlay ใน dev mode — แสดง file, line, why, fix, dismiss, reload, open in editor
- Server actions และ API routes: จัดการด้วย try/catch + `RuvyxaError`
- `notFound()` และ `redirect()` (301, 302, 307, 308) สำหรับควบคุม flow
- `ruvyxa doctor` ตรวจทุกอย่าง — config, routes, boundary, env
- 2 ตาราง: error code ทั้งหมด + cross-reference

---

## อ่าน Diagnostic เป็น Boundary Signal

diagnostics ของ Ruvyxa ถูกสร้างจาก subsystem ที่เห็นปัญหา ให้เริ่มจาก code/message แล้วไปที่
boundary ที่เป็นเจ้าของปัญหา แทนการมอง numeric ranges ว่าทุกเลขต้องมีอยู่จริง กลุ่มสำคัญปัจจุบันคือ:

| Area                    | ตัวอย่าง                                              | จุดที่ควรตรวจแรก                                                       |
| ----------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------- |
| Route discovery         | `RUV1001`, `RUV1002`, `RUV1004`                       | `appDir`, route entry filename, รูปแบบ dynamic segment, default export |
| Client/server boundary  | `RUV1007`, `RUV1008`, `RUV1009`, `RUV1010`            | ไฟล์ที่ diagnostic ระบุและ reachable relative imports                  |
| Content และ styles      | `RUV1310`–`RUV1312`, `RUV1402`, `RUV1403`             | Markdown/MDX frontmatter, Sass source หรือ stylesheet import path      |
| Configuration           | `RUV1601`, `RUV1602`                                  | config field ที่ระบุและ range/path constraint ในเอกสาร                 |
| Plugin/adapter contract | `RUV2102`, `RUV2200`, `RUV2202`, `RUV2203`, `RUV2210` | plugin definition หรือ target/adapter package ที่เลือก                 |

ใช้คำสั่งที่เล็กที่สุดที่เปิดเผย boundary เดียวกัน:

```bash
ruvyxa routes
ruvyxa trace /the-route-pattern
ruvyxa analyze --format human
ruvyxa doctor --json
```

`routes` ตอบเรื่อง discovery, `trace` ตอบ manifest entry เดียว, `analyze` ตอบ route/import
validation และ `doctor` ตอบ environment/configuration/adapter compatibility คำสั่งเหล่านี้ไม่ได้แก้
source/config ให้อัตโนมัติ จึงควรเก็บ file/path context ของ diagnostic แล้วแก้ที่สาเหตุที่เล็กที่สุด

### ลำดับ Escalation ที่ปลอดภัย

1. reproduce ด้วยคำสั่งที่เป็นเจ้าของ failure
2. อ่านไฟล์และ import/config edge ที่ result ระบุ
3. เปลี่ยนสาเหตุเดียว ไม่เปลี่ยน settings ที่ไม่เกี่ยวหลายอย่างพร้อมกัน
4. รัน focused command ซ้ำ
5. รัน `npm run check` เมื่อ failure ข้าม route, render หรือ type boundary

อย่า log secrets, environment dump ทั้งหมด หรือ private request payload เพื่อ "ดูรายละเอียดเพิ่ม"
boundary diagnostics ตั้งใจให้ชี้ source location และ safe direction โดยไม่ต้องใช้ข้อมูล sensitive

## Next Steps

- **[02-routing.md](./02-routing.md)** — Resolve route conflicts (RUV1002-1004)
- **[03-server-client-components.md](./03-server-client-components.md)** — Understand boundary
  (RUV1007, RUV1009, RUV1010)
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — Static params (RUV1510-1513)
- **[10-environment-variables.md](./10-environment-variables.md)** — Fix RUV1008 leaks
- **[11-configuration.md](./11-configuration.md)** — Fix config errors (RUV1600-RUV1603)
- **[13-deployment.md](./13-deployment.md)** — Fix deploy errors (RUV2000, RUV2001, RUV2200)
- **[14-plugins.md](./14-plugins.md)** — Fix plugin errors (RUV1700, RUV1701, RUV2102)
- **[15-official-packages.md](./15-official-packages.md)** — Fix auth/database/realtime errors
