# Error Handling

Ruvyxa speaks a common language when something goes wrong: `RUV####` codes. Every error — missing
config field, crashed worker, server-client boundary violation, plugin timeout — has unique code,
clear explanation, suggested fix, and exact file location.

---

## What You Will Learn

- Error code format and how to read diagnostic output
- Complete error catalog RUV1000-3201 organized by range
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

## Complete Error Catalog

### RUV1000–1099: Boundary Violations

Fire when crossing server/client boundary illegally. Detected by the bundler during module graph
analysis.

| Code    | Title                         | Cause                                          | Fix                                               |
| ------- | ----------------------------- | ---------------------------------------------- | ------------------------------------------------- |
| RUV1000 | Server-only module in client  | `import "server-only"` in client bundle        | Remove import or restructure code                 |
| RUV1001 | Server-only package in client | Server-side package import in `'use client'`   | Use `/client` subpath or move to server component |
| RUV1002 | Client boundary violation     | Server component usage in client context       | Add `'use client'` or restructure                 |
| RUV1003 | Ambiguous route               | Two files match same URL pattern               | Remove or rename conflicting file                 |
| RUV1004 | Duplicate route               | Two files have identical URL                   | Delete one                                        |
| RUV1005 | Invalid route parameter       | Route param does not match constraints         | Check `GetStaticParams` or path structure         |
| RUV1006 | Hook called outside component | `useState`/`useEffect` outside React component | Only call hooks inside components                 |
| RUV1007 | Import boundary violation     | Server-only import in client bundle            | Use `/client` subpath                             |
| RUV1008 | Private env variable leaked   | `process.env.SECRET` in client bundle          | Prefix with `RUVYXA_PUBLIC_` or move to server    |
| RUV1009 | Client-only module in SSR     | `'use client'` only module imported in SSR     | Add server-compatible fallback                    |
| RUV1010 | `server/` directory in client | File inside `server/` reachable from client    | Restructure imports                               |

#### RUV1000 — Server-only module in client

```
RUV1000: Server-only module in client bundle

  Module: server-only
  File: app/components/UserCard.tsx:3
  Import chain:
    app/components/UserCard.tsx
    app/lib/auth.ts

  Fix: Remove the `import "server-only"` statement or
       move the file out of the client component tree.
```

**Source**: `crates/ruvyxa_bundler/src/boundary.rs:66` **Detection**: At build time when bundler
walks module graph and encounters `server-only` import in a client reachable module.

#### RUV1001 — Server-only package in client

```
RUV1001: Private import

  File: app/components/Profile.tsx:1
  Import: @ruvyxa/database

  Fix: Use @ruvyxa/database from a server component or
       server action only.
```

**Source**: `crates/ruvyxa_diagnostics/src/lib.rs`

#### RUV1003 — Ambiguous route

```
RUV1003: Ambiguous route

  Route: /products/[id]
  Files:
    app/products/[id]/page.tsx
    app/products/[slug]/page.tsx

  Both files match the same URL pattern like /products/123.

  Fix: Rename one of the conflicting dynamic segments so
       they have different parameter names.
```

#### RUV1007 — Import boundary violation

```
RUV1007: Private import

  Package: @ruvyxa/database
  File: app/components/UserList.tsx:1
  Import chain:
    app/components/UserList.tsx (client)
    app/lib/db.ts

  Server-only packages cannot be imported in client bundles.

  Fix: Move the database access to a server action or
       use @ruvyxa/auth/client instead of @ruvyxa/auth.
```

**Source**: `crates/ruvyxa_bundler/src/boundary.rs:73` **Detection**: When bundler traces `import`
from a client entry and reaches a package with `"sideEffects": false` or a known server-only
package.

#### RUV1008 — Private env variable leaked

```
RUV1008: Private environment variable leaked to client bundle

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

#### RUV1009 — Client-only module in SSR

```
RUV1009: Client-only module imported into SSR graph

  File: app/components/Map.tsx:1
  Import: leaflet

  This module uses browser APIs and cannot be rendered on the server.

  Fix: Use dynamic import with `{ ssr: false }` or wrap
       in a client component boundary.
```

**Source**: `crates/ruvyxa_bundler/src/boundary.rs:132`

#### RUV1010 — server/ directory in client

```
RUV1010: File inside server/ directory reachable by client graph

  File: app/server/db.ts
  Imported in: app/components/List.tsx

  Files inside server/ directories must only be imported
  from server components.

  Fix: Move the shared logic to a file outside server/,
       or restructure to avoid importing it from client code.
```

**Source**: `crates/ruvyxa_bundler/src/boundary.rs:89`

---

### RUV1100–1199: Route Errors

| Code    | Title                                   | Cause                                           | Fix                                       |
| ------- | --------------------------------------- | ----------------------------------------------- | ----------------------------------------- |
| RUV1100 | Route not found                         | No file matches requested URL                   | Create route file or check URL            |
| RUV1101 | Route parse failure                     | File system scanning error                      | Check for invalid characters in filenames |
| RUV1102 | Route conflict / SSR renderer not found | Layout and page conflict, or renderer missing   | Restructure route hierarchy               |
| RUV1103 | Invalid layout export                   | Layout does not export default component        | Add `export default function Layout(...)` |
| RUV1104 | Invalid page export                     | Page missing default export                     | Add `export default function Page(...)`   |
| RUV1105 | Missing params export                   | Dynamic route without `GetStaticParams` for SSG | Export `GetStaticParams` function         |
| RUV1106 | Params type mismatch                    | Route params do not match TypeScript types      | Check `params` prop type                  |

#### RUV1100 — Route not found

```
RUV1100: Route not found

  URL: /products/123
  Method: GET

  None of the scanned routes matched this URL.

  Scanned routes:
    SSR  /
    SSR  /about
    SSR  /blog/[slug]
    API  /api/hello

  Fix: Create app/products/[id]/page.tsx
       or check that the URL is correct.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:773`

#### RUV1101 — Route parse failure

```
RUV1101: Route parse failure

  File: app/products/[id].tsx

  File name contains invalid characters for route parsing.

  Fix: Rename the file to use valid route naming conventions.
       Dynamic segments: [param]
       Catch-all: [...param]
       Optional catch-all: [[...param]]
```

**Source**: File system scanner

#### RUV1102 — Route conflict / SSR renderer not found

```
RUV1102: SSR renderer was not found

  Route: /dashboard

  The route has a layout but no matching SSR renderer.

  Fix: Ensure the page file exports a default component.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:1205`

#### RUV1103 — Invalid layout export

```
RUV1103: Invalid layout export

  File: app/layout.tsx

  Layout must export a default React component.

  Fix: Add `export default function RootLayout({ children }) { ... }`
```

#### RUV1104 — Invalid page export

```
RUV1104: Page is missing a default export

  File: app/about/page.tsx

  Fix: Add `export default function Page() { ... }`
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:752`

#### RUV1105 — Missing params export

```
RUV1105: Missing params export

  Route: /blog/[slug] (type: ssg)

  SSG routes with dynamic segments must export getStaticParams.

  Fix: Add `export const getStaticParams = async () => [...]`
```

#### RUV1106 — Params type mismatch

```
RUV1106: Params type mismatch

  Route: /products/[id]
  Expected: { id: string }
  Received: { slug: string }

  Fix: Check that the params object returned by getStaticParams
       matches the route's dynamic segment names.
```

---

### RUV1200–1299: Config Errors

| Code    | Title                                   | Cause                                            | Fix                                      |
| ------- | --------------------------------------- | ------------------------------------------------ | ---------------------------------------- |
| RUV1200 | Unknown config field                    | Unrecognized field in `ruvyxa.config.ts`         | Remove or rename field                   |
| RUV1201 | Config path not found                   | `appDir` or `outDir` does not exist              | Create directory or fix path             |
| RUV1202 | Invalid port                            | Port outside 1024–65535                          | Use valid port range                     |
| RUV1203 | Out of range                            | Value outside allowed range                      | Adjust value                             |
| RUV1204 | Invalid integer                         | Expected number got string                       | Fix type                                 |
| RUV1205 | Below minimum / Prerender path conflict | Value too low, or prerender path in build output | Increase value, or change prerender path |
| RUV1206 | Missing required field                  | Plugin missing `name`                            | Add `name` to plugin config              |
| RUV1207 | Invalid value range                     | Negative where positive required                 | Use positive number                      |
| RUV1208 | Invalid IP address                      | `trustedProxyIps` contains invalid IP            | Fix IP format                            |
| RUV1209 | Config file not found                   | `ruvyxa.config.ts` missing                       | Create config file                       |
| RUV1210 | Config type error                       | Wrong type for config field                      | Fix value type                           |

#### RUV1200 — Unknown config field

```
RUV1200: Unknown config field

  Field: build.foo
  File: ruvyxa.config.ts:10

  "foo" is not a recognized field in "build".

  Fix: Remove "foo" or check the configuration documentation
       for the correct field name.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:837`

#### RUV1201 — Config path not found

```
RUV1201: Config path not found

  Field: appDir
  Value: ./src/app

  The directory "./src/app" does not exist relative to
  the project root.

  Fix: Create the directory or update the path in config.
```

**Source**: `crates/ruvyxa_dev_server/src/port_binding.rs:97`

#### RUV1202 — Invalid port

```
RUV1202: Invalid port

  Field: server.port
  Value: 99999

  Port must be between 1024 and 65535.

  Fix: Use a port in the valid range, e.g. 3000.
```

#### RUV1203 — Out of range

```
RUV1203: Out of range

  Field: image.quality
  Value: 150

  quality must be between 1 and 100.

  Fix: Set quality to a value between 1 and 100.
```

#### RUV1205 — Prerender path cannot be inside build output

```
RUV1205: Prerender path `/ruvyxa/index.html` for route `/`
         cannot be written inside the build output.

  The prerender output path falls inside the .ruvyxa/ build output
  directory.

  Fix: Choose a different output path for this route.
```

**Source**: `crates/ruvyxa_cli/src/main.rs:2378`

#### RUV1208 — Invalid IP address

```
RUV1208: Invalid IP address

  Field: security.trustedProxyIps
  Value: not-an-ip

  "not-an-ip" is not a valid IP address.

  Fix: Provide valid IPv4 or IPv6 addresses.
```

**Source**: `crates/ruvyxa_cli/src/main.rs:629`

#### RUV1209 — Config file not found

```
RUV1209: Config file not found

  Ruvyxa requires a configuration file.

  Fix: Create ruvyxa.config.ts in your project root.
       Minimal example:
         import { config } from 'ruvyxa/config'
         export default config({})
```

---

### RUV1300–1399: Build Errors

| Code    | Title                                       | Cause                                   | Fix                                    |
| ------- | ------------------------------------------- | --------------------------------------- | -------------------------------------- |
| RUV1300 | Compilation error                           | TypeScript/JSX compilation failure      | Fix syntax error                       |
| RUV1301 | Module resolution failure                   | Cannot resolve import                   | Install package or fix import path     |
| RUV1302 | Bundle too large                            | Exceeds `bundleBudget` limits           | Reduce size or increase budget         |
| RUV1303 | Minification error / Client route not found | Minifier error, or client route missing | Fix syntax, rebuild                    |
| RUV1304 | Source map error / Client bundle non-page   | Source map generation failed            | Usually benign                         |
| RUV1305 | Worker crash during build                   | Build worker fatal error                | Check logs, reduce parallelism         |
| RUV1306 | Image optimization failure                  | Image processing error                  | Check file validity                    |
| RUV1307 | Out of memory                               | Build exceeds memory                    | Reduce `build.workers` or increase RAM |
| RUV1312 | Frontmatter YAML error                      | MD/MDX frontmatter syntax               | Fix YAML frontmatter                   |

#### RUV1300 — Compilation error

```
RUV1300: Compile error

  File: app/page.tsx:15
  Error: Unexpected token '}'

  TypeScript compilation failed for this file.

  Fix: Check for syntax errors around line 15.
```

**Source**: `crates/ruvyxa_dev_server/src/lib.rs:1835`

#### RUV1301 — Module resolution failure

```
RUV1301: Module resolution failure

  Specifier: '@/components/Header'
  Importer: app/page.tsx

  Fix: Check that the import path is correct and the module
       exists. If using path aliases, verify tsconfig.json paths.
```

#### RUV1302 — Bundle too large

```
RUV1302: Bundle too large

  Bundle: client/dashboard.js
  Size: 350 KB
  Limit: 250 KB

  This client bundle exceeds the bundleBudget limit.

  Fix: Split the bundle with dynamic imports, or increase
       bundleBudget.maxSize in config.
```

#### RUV1303 — Client route not found

```
RUV1303: Client route was not found

  Route: /dashboard (type: csr)

  The client bundle for this CSR route was not found in the
  build output.

  Fix: Rebuild the application.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:886`

#### RUV1304 — Client bundle for non-page route

```
RUV1304: Client bundle requested for a non-page route

  Route: /api/hello

  API routes do not have client bundles.

  Fix: This is likely a framework bug — report it.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:894`

#### RUV1305 — Worker crash during build

```
RUV1305: Worker crash during build

  Worker: build-worker-3
  Signal: SIGSEGV

  A build worker process exited unexpectedly.

  Fix: Check worker logs for crash details. Reduce
       build.workers to lower parallelism if memory-related.
```

#### RUV1306 — Image optimization failure

```
RUV1306: Image optimization failure

  File: public/images/logo.png
  Error: Input file is corrupted

  Fix: Replace the corrupted image file with a valid one.
```

#### RUV1307 — Out of memory

```
RUV1307: Out of memory

  Current: 6.2 GB used
  Limit: 4 GB

  The build process exceeded the available memory.

  Fix: Reduce build.workers in config, increase system
       memory, or enable swap.
```

---

### RUV1400–1499: Server Errors

| Code    | Title                                                  | Cause                         | Fix                     |
| ------- | ------------------------------------------------------ | ----------------------------- | ----------------------- |
| RUV1400 | Server runtime error / Tailwind CSS compilation failed | Unhandled exception           | Add try/catch           |
| RUV1401 | Tailwind CSS CLI not found                             | Missing Tailwind binary       | Install tailwindcss     |
| RUV1402 | Sass compilation failed                                | SCSS processing error         | Fix SCSS syntax         |
| RUV1403 | Stylesheet import not resolved                         | CSS @import failed            | Check import path       |
| RUV1404 | CSS entry outside project root                         | CSS path escapes root         | Move CSS file           |
| RUV1405 | HMR connection error                                   | WebSocket for HMR failed      | Refresh, check network  |
| RUV1406 | Rate limit exceeded                                    | Too many requests from one IP | Wait or increase limits |

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

#### RUV1403 — Stylesheet import not resolved

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

#### RUV1405 — HMR connection error

```
RUV1405: HMR connection error

  WebSocket connection to ws://localhost:3000/_ruvyxa/hmr failed.

  Fix: Refresh the browser. If persistent, check that the
       dev server is running and no firewall is blocking
       WebSocket connections.
```

#### RUV1406 — Rate limit exceeded

```
RUV1406: Rate limit exceeded

  IP: 192.168.1.100
  Route: /api/contact
  Limit: 600 requests per 60 seconds

  Fix: Wait before retrying. If this is an expected traffic
       pattern, increase security.actionRateLimit.max or
       security.actionRateLimit.window.
```

---

### RUV1500–1599: Worker Errors

| Code    | Title                                        | Cause                                               | Fix                                  |
| ------- | -------------------------------------------- | --------------------------------------------------- | ------------------------------------ |
| RUV1500 | Worker crash / Action error                  | Worker exited unexpectedly, or action runtime error | Check logs, reduce workload          |
| RUV1501 | Worker timeout / Route action file not found | Worker too slow, or action file missing             | Increase timeout, create action file |
| RUV1502 | Worker protocol error                        | NDJSON message malformed                            | Usually framework bug — report       |
| RUV1503 | Worker initialization failed                 | Worker could not start                              | Check Node version, dependencies     |
| RUV1504 | Worker communication error                   | IPC failure between main process and worker         | Check system resources, restart      |
| RUV1510 | Static params resolution failed              | `getStaticParams` returned invalid data             | Fix return shape                     |
| RUV1511 | Static params shorthand invalid              | String shorthand for multi-segment route            | Use object form                      |
| RUV1512 | Static params shape invalid                  | Return value is not an array                        | Return array                         |
| RUV1513 | Static params duration invalid               | Cache duration format wrong                         | Use `"10m"` or `number`              |
| RUV1550 | PPR render failed                            | Partial pre-render error                            | Check component, reduce complexity   |
| RUV1700 | Worker timeout (middleware)                  | Plugin middleware exceeded timeout                  | Increase timeoutMs                   |

#### RUV1500 — Worker crash

```
RUV1500: Worker crash

  Worker: render-worker-2
  Status: exit code 1

  A render worker process crashed while handling a request.

  Fix: Check server logs for the crash reason. Common causes:
       - Out of memory
       - Unhandled exception in route handler
       - Native module incompatibility
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:316`

#### RUV1500 — Action returned duplicate realtime event metadata

```
RUV1500: Action returned duplicate realtime event metadata

  Action: app/actions/chat/action.ts

  The action handler returned multiple realtime event metadata
  entries, but only one is allowed per action.

  Fix: Remove duplicate sendMessage.realtime() calls.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:1027`

#### RUV1500 — Action realtime event metadata exceeds 24 KiB

```
RUV1500: Action realtime event metadata exceeds 24 KiB

  Action: app/actions/notifications/action.ts

  The realtime event payload attached to this action is
  too large.

  Fix: Reduce the size of the event metadata to under 24 KiB.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:1053`

#### RUV1501 — Route action file was not found

```
RUV1501: Route action file was not found

  Route: /contact
  Expected: app/contact/action.ts

  The action file for this route does not exist.

  Fix: Create the action file at the expected path.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:972`

#### RUV1501 — Worker timeout

```
RUV1501: Worker timeout

  Worker: render-worker-1
  Duration: 31.2s
  Limit: 30s

  The worker did not respond within the configured limit.

  Fix: Reduce the workload in the route handler or increase
       the timeout.
```

#### RUV1510 — Static params resolution failed

```
RUV1510: Static params resolution failed

  Route: /blog/[slug]
  getStaticParams returned: [{ slug: null }]

  Static params values must be strings or numbers, not null.

  Fix: Filter out null/undefined values before returning.
```

#### RUV1511 — Static params shorthand invalid

```
RUV1511: Static params shorthand invalid

  Route: /products/[category]/[id]
  getStaticParams returned: ["electronics"]

  String shorthand is only valid for routes with exactly
  one dynamic segment.

  Fix: Use object form: [{ category: "electronics", id: "123" }]
```

#### RUV1512 — Static params shape invalid

```
RUV1512: Static params shape invalid

  Route: /posts/[slug]
  getStaticParams returned: "not-an-array"

  getStaticParams must return an array of parameter objects.

  Fix: Return an array, e.g., [{ slug: "hello" }, { slug: "world" }]
```

#### RUV1513 — Static params duration invalid

```
RUV1513: Static params duration invalid

  Route: /blog/[slug]
  cache: "forever"

  Cache duration must be a number (seconds) or a string like
  "10m", "1h", "1d".

  Fix: Use "forever" is not valid. Use "365d" or 31536000.
```

#### RUV1550 — PPR render failed

```
RUV1550: PPR render failed

  Route: /dashboard

  Partial pre-rendering encountered an error during the
  static shell generation.

  Fix: Check the component for dynamic data access during
       the static shell phase.
```

**Source**: `crates/ruvyxa_dev_server/src/render_pipeline.rs:693`

---

### RUV1600–1699: Plugin Errors

| Code    | Title                      | Cause                                           | Fix                           |
| ------- | -------------------------- | ----------------------------------------------- | ----------------------------- |
| RUV1600 | Plugin boundary violation  | Plugin injected server data into client context | Fix plugin boundary           |
| RUV1601 | Plugin hook timeout        | Plugin exceeded `pluginLimit`                   | Reduce work or increase limit |
| RUV1602 | Plugin hook error          | Unhandled exception in plugin hook              | Fix plugin code               |
| RUV1603 | Unknown plugin             | Plugin name not recognized                      | Install or register plugin    |
| RUV1604 | Plugin configuration error | Invalid plugin options                          | Fix plugin options            |

#### RUV1600 — Plugin boundary violation

```
RUV1600: Plugin boundary violation

  Plugin: my-analytics
  Hook: resolveId
  Detail: Plugin attempted to access process.env.SECRET_KEY
          from a client-side context.

  Fix: Check that the plugin does not inject server-only
       values into client-facing hooks.
```

**Source**: Config validation, server startup. Plugins cannot inject server-only data into client
bundles. The same `RUVYXA_PUBLIC_` rules apply inside plugins.

#### RUV1601 — Plugin hook timeout

```
RUV1601: Plugin hook timeout

  Plugin: my-analytics
  Hook: buildEnd
  Duration: 6.2s (limit: 5.0s)

  Fix: Reduce work in the hook or increase security.pluginLimit
       in ruvyxa.config.ts.
```

**Source**: `crates/ruvyxa_middleware/src/plugin_host.rs:480`

#### RUV1603 — Unknown plugin

```
RUV1603: Unknown plugin

  Plugin: my-custom-plugin

  "my-custom-plugin" is not a registered plugin name. Ensure
  the plugin is installed and imported in ruvyxa.config.ts.

  Fix: npm install my-custom-plugin
       Then add "import myCustomPlugin from 'my-custom-plugin'"
```

#### RUV1604 — Plugin configuration error

```
RUV1604: Plugin configuration error

  Plugin: requireEnv
  Detail: "variables" must be a non-empty array of strings

  Fix: Pass an array of environment variable names:
       options: { variables: ["DATABASE_URL"] }
```

---

### RUV1700–1799: Deploy / Adapter Errors

| Code    | Title                                                     | Cause                           | Fix                             |
| ------- | --------------------------------------------------------- | ------------------------------- | ------------------------------- |
| RUV1700 | Adapter not found                                         | Specified adapter not installed | Install adapter package         |
| RUV1701 | Adapter build failed                                      | Adapter transform error         | Check compatibility             |
| RUV1702 | Manifest generation failed / Worker pool script not found | Build JSON write failure        | Check disk/permissions, rebuild |
| RUV1703 | Deploy config missing                                     | Platform config file not found  | Create config file              |
| RUV1704 | Adapter incompatible                                      | Strategy not supported          | Change strategy or adapter      |
| RUV1705 | Node adapter entry not found                              | Missing server.js               | Rebuild                         |
| RUV1706 | Static adapter SSR route                                  | SSR in static build             | Use SSG or switch adapter       |

#### RUV1700 — Adapter not found

```
RUV1700: Adapter not found

  Adapter: "vercel"

  The "vercel" adapter is not installed.

  Fix: npm install -D @ruvyxa/adapter-vercel
```

**Source**: `crates/ruvyxa_cli/src/main.rs:3147`, `crates/ruvyxa_middleware/src/plugin_host.rs:533`

#### RUV1700 — TypeScript plugin hook timed out

```
RUV1700: TypeScript plugin hook timed out after 30000 ms

  Plugin: my-plugin
  Hook: http.onRequest

  The plugin exceeded middleware.timeoutMs.

  Fix: Reduce plugin work or increase middleware.timeoutMs.
```

**Source**: `crates/ruvyxa_middleware/src/plugin_host.rs:480`

#### RUV1700 — TypeScript plugin host exited before responding

```
RUV1700: TypeScript plugin host exited before responding (status: 1)

  The plugin host process crashed.

  Fix: Check plugin code for unhandled exceptions.
```

**Source**: `crates/ruvyxa_middleware/src/plugin_host.rs:524`

#### RUV1701 — TypeScript plugin protocol error

```
RUV1701: TypeScript plugin host returned invalid JSON

  The plugin host sent malformed JSON over the IPC channel.

  Fix: This is likely a framework or plugin bug — report it.
```

**Source**: `crates/ruvyxa_middleware/src/plugin_host.rs:544`

#### RUV1701 — TypeScript request middleware returned invalid result

```
RUV1701: TypeScript request middleware returned an invalid result

  Plugin: my-plugin
  Hook: http.onRequest

  The `onRequest` handler must return undefined, a Request, or a Response.

  Fix: Check the return value of the onRequest handler.
```

**Source**: `crates/ruvyxa_middleware/src/plugin_host.rs:304`

#### RUV1701 — Plugin returned an unsafe request path

```
RUV1701: Plugin returned an unsafe request path

  Plugin: my-plugin
  Path: //evil.com/steal

  The plugin tried to redirect to an unsafe destination.

  Fix: Ensure plugin redirect destinations are validated
       absolute paths or http(s) URLs.
```

**Source**: `crates/ruvyxa_dev_server/src/plugin_bridge.rs:240`

#### RUV1702 — Worker pool script was not found

```
RUV1702: Worker pool script was not found

  Script: plugin-runtime.mjs

  The TypeScript plugin host runtime script is missing from
  the ruvyxa package installation.

  Fix: Reinstall ruvyxa: npm install ruvyxa
```

**Source**: `crates/ruvyxa_dev_server/src/worker_pool.rs:877`

#### RUV1704 — Adapter strategy incompatibility

```
RUV1704: Adapter incompatible

  Route: /dashboard (type: isr)
  Adapter: cloudflare

  The Cloudflare adapter does not support ISR because it
  requires persistent storage.

  Fix: Use SSG or SSR for this route, or switch to a
       different adapter (e.g., Vercel or Node).
```

**Source**: `crates/ruvyxa_dev_server/src/worker_pool.rs:307`

---

### RUV2000–2102: CLI / Config / Plugin Definition Errors

| Code    | Title                     | Cause                       | Fix                        |
| ------- | ------------------------- | --------------------------- | -------------------------- |
| RUV2000 | Adapter config error      | BuildContext validation     | Fix adapter configuration  |
| RUV2001 | Adapter option error      | Invalid adapter options     | Fix adapter options        |
| RUV2102 | Invalid plugin definition | `definePlugin()` type error | Return valid plugin object |

#### RUV2000 — Adapter config error

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

---

### RUV2200–2210: Build / Prerender Errors

| Code    | Title                  | Cause                          | Fix                                   |
| ------- | ---------------------- | ------------------------------ | ------------------------------------- |
| RUV2200 | Build error            | Generic build failure          | Check preceding diagnostics           |
| RUV2202 | No prerendered pages   | Static adapter on SSR-only app | Add SSG routes or skip static adapter |
| RUV2210 | Strategy not supported | Adapter rejects strategy       | Use different strategy                |

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
| RUV3102 | WebAuthn error         | Platform authenticator issue | Check browser            |
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
// → RUV1400: Server runtime error
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
        code: 'RUV1400',
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
  "code": "RUV1400",
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
| Not found        | 404    | RUV1100 |
| Rate limited     | 429    | RUV1406 |
| Internal error   | 500    | RUV1400 |
| Plugin timeout   | 500    | RUV1601 |
| Adapter error    | 502    | RUV1701 |

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

### Guard Against RUV1200-1210 (Config Errors)

```typescript
// ✅ Good — use the config() helper for type checking
import { config } from 'ruvyxa/config'
export default config({
  server: { port: 3000 },
})

// ❌ Bad — typos are not caught
export default {
  server: { port: '3000' }, // RUV1204 — string, not number
  build: { split: 'none' }, // RUV1200 — unknown value
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

| Symptom                    | Likely Error                              | Action                                   |
| -------------------------- | ----------------------------------------- | ---------------------------------------- |
| Error overlay on page load | RUV1300 (compilation), RUV1007 (boundary) | Check file indicated in overlay          |
| Page renders blank         | RUV1400 (runtime error in component)      | Add `error.tsx`, check browser console   |
| HMR not updating           | RUV1405 (WebSocket lost)                  | Refresh browser, check network           |
| Slow page loads            | RUV1501 (SSR timeout)                     | Optimize component, reduce data fetching |
| 404 for existing route     | RUV1100 (route not found)                 | Check route naming, file location        |
| 500 on form submit         | RUV1400 (action error)                    | Check action code, add error handling    |
| Plugin not running         | RUV1603 (unknown plugin)                  | Check plugin is imported and listed      |

### During `ruvyxa build`

| Symptom                                  | Likely Error                               | Action                                 |
| ---------------------------------------- | ------------------------------------------ | -------------------------------------- |
| Build fails immediately                  | RUV1200 (config), RUV1209 (missing config) | Run `ruvyxa doctor`                    |
| Build fails during compilation           | RUV1300 (syntax error)                     | Fix indicated syntax error             |
| Build fails at module resolution         | RUV1301 (missing import)                   | Install package or fix import path     |
| Build succeeds but output missing routes | RUV1101 (route parse)                      | Check filenames for invalid characters |
| Build succeeds but bundle too large      | RUV1302 (budget exceeded)                  | Optimize or adjust bundleBudget        |
| Build OOM                                | RUV1307 (memory)                           | Reduce `build.workers`, increase RAM   |
| Image optimization fails                 | RUV1306 (corrupt image)                    | Replace corrupt image file             |

### During `ruvyxa check`

| Symptom                      | Likely Error          | Action                       |
| ---------------------------- | --------------------- | ---------------------------- |
| Boundary violations reported | RUV1000-1009          | Restructure imports          |
| Route conflicts reported     | RUV1003 (ambiguous)   | Rename conflicting files     |
| Config validation errors     | RUV1200-1210          | Fix config file              |
| SSG params missing           | RUV1105, RUV1510-1513 | Add/export `getStaticParams` |

### During Deployment

| Symptom                  | Likely Error                   | Action                       |
| ------------------------ | ------------------------------ | ---------------------------- |
| Build fails in CI        | RUV1700 (adapter missing)      | Install adapter package      |
| 502 on all routes        | RUV1704 (incompatible adapter) | Change adapter or strategy   |
| Static site has no pages | RUV1706 (SSR in static)        | Use SSG or Node adapter      |
| Functions timeout        | RUV1700 (exceeded maxDuration) | Increase timeout or optimize |
| Cold starts are slow     | RUV1503 (init failure)         | Use `build.warm: true`       |

---

## Quick Reference

### Error Code Ranges

```
Range       Category          Where to look
──────────  ────────────────  ──────────────────────
RUV1000     Boundary          03-server-client-components.md
RUV1010     Boundary          bundler boundary check
RUV1100     Route             02-routing.md
RUV1200     Config            11-configuration.md
RUV1300     Build             12-cli-commands.md
RUV1400     Server runtime    architecture/worker-pool.md
RUV1500     Worker            architecture/worker-pool.md
RUV1510     Static params     05-data-loading-cache.md
RUV1550     PPR               05-data-loading-cache.md
RUV1600     Plugin            14-plugins.md
RUV1700     Deploy            13-deployment.md
RUV2000     Adapter           13-deployment.md
RUV2102     Plugin def        14-plugins.md
RUV2200     Build             12-cli-commands.md
RUV3001     Database          15-official-packages.md
RUV3100     Auth              15-official-packages.md
RUV3201     Realtime          15-official-packages.md
```

### File Locations

| Error source            | File                                              |
| ----------------------- | ------------------------------------------------- |
| Bundler boundary checks | `crates/ruvyxa_bundler/src/boundary.rs`           |
| Build diagnostics       | `crates/ruvyxa_diagnostics/src/lib.rs`            |
| Dev server rendering    | `crates/ruvyxa_dev_server/src/render_pipeline.rs` |
| Worker pool             | `crates/ruvyxa_dev_server/src/worker_pool.rs`     |
| Style compilation       | `crates/ruvyxa_dev_server/src/style.rs`           |
| Plugin host             | `crates/ruvyxa_middleware/src/plugin_host.rs`     |
| Config validation       | `crates/ruvyxa_cli/src/main.rs`                   |
| Plugin bridge           | `crates/ruvyxa_dev_server/src/plugin_bridge.rs`   |
| Plugin validation       | `packages/@ruvyxa/core/src/plugin.ts`             |
| Auth errors             | `packages/@ruvyxa/auth/src/index.ts`              |
| Database errors         | `packages/@ruvyxa/database/src/index.ts`          |
| Realtime errors         | `packages/@ruvyxa/realtime/src/plugin.ts`         |
| Adapter errors          | `packages/@ruvyxa/adapter-*/src/index.ts`         |

---

## Troubleshooting

| Symptom                        | Likely cause                    | Fix                                      |
| ------------------------------ | ------------------------------- | ---------------------------------------- |
| Blank white page               | Uncaught client error           | Check browser console, add `error.tsx`   |
| RUV1003 after adding file      | Two files match same URL        | Remove duplicate route file              |
| RUV1008 on build               | Private env in client component | Rename to `RUVYXA_PUBLIC_*`              |
| RUV1200 on project creation    | Typo in config                  | Run `ruvyxa doctor`                      |
| RUV1301 for local import       | Incorrect import path           | Use relative path or alias               |
| RUV1400 after deploy           | Missing env var                 | Set env on platform                      |
| RUV1500 on worker pool         | Worker crashed                  | Check server logs, restart               |
| RUV1501 on render              | Route too slow                  | Optimize or increase timeout             |
| RUV1601 with plugins           | Plugin too slow                 | Increase `pluginLimit`                   |
| RUV1700 on build               | Adapter package missing         | `npm install @ruvyxa/adapter-<name>`     |
| 404 on existing route          | Route not exported              | Add `export default function Page()`     |
| 500 on server action           | Validation failed               | Check action error return                |
| Error overlay not showing      | `debug.overlay: false`          | Enable in config                         |
| Error overlay always showing   | Persistent compile error        | Fix the error in your code               |
| `notFound()` does nothing      | No `not-found.tsx`              | Create the file or check hierarchy       |
| `reset()` does not clear error | State persists                  | Use `key` prop to force remount          |
| WebSocket (HMR) disconnects    | Network proxy/firewall          | Check WebSocket path, restart dev server |

---

## Next Steps

- **[02-routing.md](./02-routing.md)** — Resolve route conflicts (RUV1100-1106)
- **[03-server-client-components.md](./03-server-client-components.md)** — Understand boundary
  (RUV1000-1008)
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — Static params (RUV1510-1513)
- **[10-environment-variables.md](./10-environment-variables.md)** — Fix RUV1008 leaks
- **[11-configuration.md](./11-configuration.md)** — Fix config errors (RUV1200-1210)
- **[13-deployment.md](./13-deployment.md)** — Fix deploy errors (RUV1700-1706)
- **[14-plugins.md](./14-plugins.md)** — Fix plugin errors (RUV1600-1604)
- **[15-official-packages.md](./15-official-packages.md)** — Fix auth/database/realtime errors
