# Changelog

## v1.0.27 (2026-08-04)

### Fixed

- Markdown and MDX pages no longer gain graph edges from imports shown inside fenced code examples.
  Every other reader masked those examples before scanning; the import-edge walk did not, so a
  documented `import './config'` pulled a real module into the page's client graph and could raise
  RUV1007, RUV1008, or RUV1010 against code the page never runs. Source masking now happens where
  the file is read, so no reader can skip it.
- Fixed a window in the dev-server render cache where an entry expiring at the same moment as a
  write of the same key could leave that key out of the recency list. The eviction path recovered by
  clearing the entire cache, so the symptom was an unexplained loss of every cached render.

### Performance

- Route discovery and validation now read and scan each module once per run. A page was read three
  times and scanned four; a component shared by many routes was re-read for each of them, because
  rendering-strategy detection built a throwaway edge cache per route. Diagnostics and detected
  strategies are unchanged.
- Rendering-strategy detection no longer reads and masks a page's entire reachable dependency graph
  before the rules that answer from the page's own exports. Pages that declare `"use client"`,
  `ppr`, `revalidate`, or `getStaticParams` now skip that walk entirely. A page matching one of
  those rules also keeps its declared strategy when a dependency cannot be read, instead of falling
  back to SSR.
- The auth runtime compiles its OAuth route pattern once per `createAuth()` instead of once per
  request that reaches the auth handler.

### Documentation

- Removed the pinned framework version from the English and Thai documentation homes. It named the
  release the docs were written for and went stale on every bump.
- Corrected `ARCHITECTURE.md` against the code it describes. Eleven documented Rust APIs did not
  exist under any name — `RuvyxaCompiler`, `check_boundary`, `produce_iife`,
  `produce_server_module`, `WorkerPool` with crossbeam channels, `compile_all`, `rewrite_env_vars`,
  `validate_route_path`, `BundleProfile`, and a `ModuleRegistry`/`SharedCache`/`DiagnosticCollector`
  lock hierarchy for types the workspace never defined. Each is now the real signature, plus
  corrected `BundleOptions`, `BundleInput`, and `CompiledModule` structs, the real emit layout, and
  the actual lock ordering.
- Repointed the stale documentation paths in `CONTRIBUTING.md`, and dropped the `docs/` links from
  the `create-ruvyxa` README: that directory is not in the published tarball, so they resolved in
  git and 404'd on npm.
- Added `@ruvyxa/testing` to the README package table, dropped a hardcoded export count, and
  replaced the "Complete Error Catalog" claim — the linked page is a symptom table covering a
  fraction of the 60+ codes, not a catalog.

### Internal

- `pnpm release:validate` now verifies every relative Markdown link and heading anchor in the
  repository. The 1.0.26 documentation restructure left 25 dead links behind, including two on the
  `create-ruvyxa` npm package page, and nothing in the toolchain could see them. All are repointed
  at their successors under `docs/en/`.
- Removed the named-export list from the bundler's module facts. It was collected on every scan and
  read only by its own tests.
- `@ruvyxa/testing` now declares its `@ruvyxa/core` peer range as `workspace:^`, matching every
  other package, so releases cannot leave it pinned to an older minor.

## v1.0.26 (2026-08-03)

### Developer experience

- Added a self-contained interactive `ruvyxa analyze --html` report, `routes --json`, and a
  development-only `/__ruvyxa/devtools` dashboard for routes, LRU cache state, bundle metrics,
  Server Action timing, and uptime.
- Added atomic `ruvyxa add form|data-table|auth` scaffolds and the dependency-free `@ruvyxa/testing`
  package with loader, action, and cache mocks.

### Breaking changes

- Renamed the scaffold command from `ruvyxa add` to `ruvyxa adds`. Generated applications now use
  `npm run adds -- form` (or `data-table` / `auth`).

### Runtime and routing

- Added validated file-system i18n routing with locale detection, prerender expansion, automatic
  document language and hreflang output, and native/serverless parity.
- Added opt-in browser-native View Transitions, React 19 stable action API coverage, and bounded
  same-origin on-demand image optimization with Cloudflare image-transform integration.
- Carried validated built-in middleware policy into standalone, serverless, Cloudflare Workers, and
  Vercel Edge artifacts through Fetch-native CORS, rate limiting, timing, logging, and headers.
  Vercel now supports an explicit `edge: true` mode without Node.js polyfills.

### Documentation

- Added production-shaped Prisma and Drizzle ORM starters in English and Thai and documented the new
  CLI, runtime, adapter, image, routing, and testing contracts.

### Bundler Correctness

- **Fixed dependency scanning around regular expressions and template literals.** The shared source
  scanner now distinguishes a regular-expression literal from division, skips quoted content and
  comments correctly, and scans `${…}` interpolations as code. Imports, `require()` calls,
  re-exports, default-export validation, and client-boundary checks therefore remain visible after
  patterns such as `/["']/` and inside real template expressions.
- **Fixed interpolation scans reading into surrounding template text.** Scanner helpers are now
  bounded to the interpolation range, so text following `${import}` or `${require}` cannot be
  interpreted as a module specifier.
- **Fixed warm builds resolving aliases differently from cold builds.** The incremental graph cache
  now persists each source-specifier-to-path alias with its dependency edges. Cache entries created
  before that field are resolved fresh rather than being reused with an empty alias map, preventing
  unresolved alias specifiers in warm client bundles.

### Realtime

- **Coalesced subscription-driven reconnects.** A burst of channel subscriptions now settles into
  one queued refresh using the final channel set, instead of repeatedly opening and discarding
  sockets as each subscription is registered.

### Build Architecture

- **Consolidated source facts in the bundler AST.** The compiler, linker, boundary validation, and
  route graph now share the parsed import/export/default-export/environment-read facts. Compiled
  modules keep that parse result for the duration of a build, avoiding repeated scans while keeping
  route validation and bundling aligned.
- **Split the CLI implementation by responsibility.** Command dispatch remains in `main.rs`; build,
  caching, client bundles, prerendering, configuration, plugin bridging, diagnostics, and UI now
  live in dedicated CLI modules. This is an internal refactor and does not add or remove CLI
  commands.

### Documentation

- Updated the bundler, graph, and CLI architecture references to describe the shared scanner, cache
  and resolver behavior, and the current CLI module layout.

## v1.0.25 (2026-07-30)

### Route Metadata

- **Added `export const meta`.** A page or layout can declare document metadata; the framework
  merges every `meta` on the route root-layout-first and renders the result into `<head>`. Fields:
  `title`, `titleTemplate`, `description`, `canonical`, `robots`/`noindex`, `lang`, `alternates`,
  `image`, `imageAlt`, `siteName`, `type`, `locale`, and `card`. `meta` may be an object or a
  synchronous function of `{ path, params }`.
- A level's own `title` is never formatted by its own `titleTemplate`, so a layout template formats
  its pages without reformatting the layout's own title.
- `lang` is applied to the `<html>` element of the document each server render produces, covering
  SSR, SSG, PPR, prerender, and serverless. Client-side navigation does not change it.
- Metadata is composed as a sibling of the route's layouts, so a suspended layout cannot hold the
  document title back past the flushed shell, and no wrapper element is created per render.
- Added `Meta`, `MetaFactory`, `MetaExport`, `MetaContext`, and `MetaAlternate` types to
  `@ruvyxa/react`.

### Crawler Discovery Files

- **`ruvyxa build` now generates `robots.txt` and `sitemap.xml`** from the route manifest and the
  URLs the build prerendered, instead of leaving both to opt-in plugins. A file of the same name in
  `public/` always wins.
- Added the `site` configuration block: `url`, `sitemap`, and `robots`. When `url` is absent the
  build resolves a production-only origin from `RUVYXA_SITE_URL`, Vercel, or Netlify. Structured
  options now support sitemap exclusions/additional paths and Next-style robots rule groups.
- Sitemap output now validates and escapes absolute URLs and automatically shards at the protocol's
  50,000 URL or 50 MB limits. Exact application routes can own `/sitemap.xml` or `/robots.txt`
  without being shadowed in production, and both Rust and standalone servers return the correct
  UTF-8 XML/plain-text content types.
- Added Next-style rich sitemap entries through `site.sitemap.defaults` and `site.sitemap.entries`:
  modification dates, change frequencies, priorities, language alternates, images, and videos. Core
  and first-party plugin output use readable multi-line XML, conditional namespaces, strict
  URL/date/value validation, and the same sharding limits.
- **Fixed `/robots.txt` and `/sitemap.xml` being answered with an HTML page.** Those exact paths now
  return 404 when no file backs them, rather than letting a bare dynamic route such as `/[lang]`
  capture them. `dev`, `start`, and the serverless handler apply the same rule.

### Plugins

- **Added the `head` declaration.** A plugin contributes `link`, `meta`, `noscript`, `script`, and
  `style` elements to every rendered document's `<head>`, declared once at config load and injected
  by the server with no per-request round trip into the plugin host. Attribute values are escaped
  and the element list is closed, so a declaration cannot end the head early.
- **Added `createPluginHarness()`**, exported from `ruvyxa/plugin-harness`. It runs `register(api)`
  against recording sockets and exposes the request, response, route, build, dev, diagnostics, and
  head entry points the server uses, so a plugin can be tested without booting an application.
- **Added the `fonts()` built-in plugin.** It downloads Google Fonts stylesheets and their `.woff2`
  files at build time, rewrites the URLs to local paths, and declares the self-hosted stylesheet in
  `<head>`, removing a render-blocking third-party origin from the critical path. A network failure
  reports a diagnostic instead of failing the build.
- `definePlugin` validation errors now carry the `RUV2102` diagnostic code instead of raising bare
  `TypeError` messages.

### Security

- **Fixed every server action being rejected with `403 Cross-origin action request blocked` behind a
  TLS-terminating proxy.** The same-origin check compared the request's scheme against a hardcoded
  `http` whenever no trusted proxy reported one, so an `https` origin never matched — and the
  comparison was inverted relative to its intent, admitting a plain-`http` origin while blocking the
  secure one. The host comparison, which is the check that actually stops CSRF, now stands on its
  own; the scheme is compared only when a trusted peer states it through `X-Forwarded-Proto`.
  Deployments whose proxy is neither loopback nor listed in `security.trustedProxyIps` — the
  ordinary Docker Compose, Kubernetes, and managed-platform-edge shapes — work without
  configuration. Setting `trustedProxyIps` remains recommended: it is what enables forwarded
  client-IP detection and restores the strict scheme comparison.
- **`security.trustedProxyIps` accepts CIDR ranges.** Entries are matched as prefixes (`10.0.0.0/8`,
  `2001:db8::/32`), a bare address means a host route, and an IPv4 range also matches the
  IPv4-mapped form (`::ffff:10.0.0.9`) a dual-stack listener reports. Previously only exact
  addresses worked, which made trusting a proxy pool impractical. An unparseable entry now fails
  startup with `RUV1602` instead of being silently discarded, so a typo can no longer leave a proxy
  untrusted and every client sharing one rate-limit bucket.
- **Fixed the action rate limiter being usable to lock out every other client.** It tracked a map of
  live keys capped at 10,000 entries and denied any key it could not admit, so filling the map —
  trivial by rotating source addresses within an IPv6 `/64` — denied service to every first-time
  client until the window elapsed. Counters now live in a fixed 8,192-slot array with per-process
  hash seeding: memory no longer depends on how many clients have been seen, admission is never
  refused for lack of room, and a slot collision can only limit a client early, never grant it extra
  budget.
- **`@ruvyxa/auth` now rate-limits per client in addition to per identity.** The existing bucket
  keys on the email, so one source could try `rateLimit.max` passwords against an unlimited number
  of accounts — the shape of credential stuffing and account enumeration. A second bucket keyed on
  the client alone, with five times the budget, caps that total. The larger budget keeps shared
  egress (offices, mobile carriers, CGNAT) working. **This can return `RUV3102` where a request
  previously succeeded**, for traffic that authenticates many distinct identities from one client
  key. Configure `clientIp` in production — the user-agent fallback is client-controlled and
  therefore rotatable.
- **A plugin hook that reached the worker is no longer retried automatically.** Any delivery failure
  was treated as "worker gone" and retried, so a `request` or `response` hook whose worker died
  after receiving the request could run its side effects twice. Write and flush failures — where the
  worker provably never saw the request — are still retried; a failure while reading the response is
  retried only for hooks with no observable effect.

### Correctness

- **Fixed a page whose default export is re-exported being reported as missing one.** Route
  validation tested for the literal text `export default`, so `export { Page as default }`,
  `export { default } from './impl'`, and `export * as default from './impl'` all failed `RUV1004`,
  while the same text inside a string or comment passed. Detection now shares the bundler's scanner
  (`ruvyxa_bundler::ast::has_default_export`), which skips strings and comments and recognises every
  valid form. `export type { X as default }` is correctly rejected, since a type export erases.
- **`ctx.path` in a client bundle is the actual pathname again.** It fell back to the route pattern,
  so a page rendered at `/blog/hello` saw `/blog/[slug]` whenever the request path global was
  absent. The pattern is now published separately as `__RUVYXA_ROUTE_PATTERN__` and `ctx.path` falls
  back to `location.pathname`. The router seeds its initial snapshot from the same global, so the
  first `useRoute()` reports the pattern the server rendered rather than a re-derived guess.
- `router.refresh()` on a route whose bundle is not registered now throws a message naming the route
  and what to do, instead of failing inside the renderer with no context.

- **Fixed the Node compiler mis-linking any module containing a regular expression with a quote.**
  The source scanner had no regex-literal handling, so a pattern such as `/("[^"]*")/` opened a
  phantom string that ran to the next quote anywhere later in the file; every `import` and `export`
  in between was read as string content and survived into the bundle, producing
  `SyntaxError: Unexpected token 'export'` at runtime.

### Performance

- **Removed a file read and a full scan of the page source from every rendered request.** Each
  render re-read `page.tsx` from disk and scanned it for a default export purely to produce a
  friendlier error. Route validation already covers that case at build time, and a genuinely missing
  export is now recognised from the loader's own message, so the check no longer costs an I/O round
  trip and a scan per request.
- **Cached HTML is no longer copied on the way out.** Render-cache entries are stored as `Arc<str>`
  and served by handing back the stored allocation, so a cache hit no longer duplicates the whole
  document per request. Compiled content modules share their allocation the same way.
- **`public/` asset links are resolved once per invalidation instead of once per render.** Every
  SSG/ISR/CSR/PPR render walked the public directory to rebuild the same `<link>` list; the result
  is now memoized alongside the other runtime caches and recomputed when they are invalidated.
- **Bounded the module graphs a build worker retains.** Production prerendering imports each path
  under a fresh module URL so page state cannot leak between paths, and Node's ESM registry never
  releases a URL — so every isolated import permanently added one more module graph, and no
  in-worker cache eviction could reclaim it. A build worker is now retired after
  `RUVYXA_PRERENDER_RECYCLE_AFTER` isolated renders (default 32, `0` disables), and only while idle
  so sibling renders are never dropped. The dev server never requests isolated imports and is
  unaffected.
- **Bounded per-worker concurrency.** A worker now admits at most `RUVYXA_WORKER_MAX_CONCURRENCY`
  requests at once (default: core count clamped to 2–8) and queues the rest. Renders are CPU-bound
  and each holds a React tree, a compiled bundle, and a response buffer, so admitting a whole burst
  exhausted the heap or thrashed the CPU into timeouts that presented as hangs. `invalidate` and
  `ping` bypass the queue, since delaying an invalidation would serve stale bundles exactly when the
  worker is busiest. `ping` now also reports `queuedRequests` and `maxConcurrentRequests`.
- A worker shutdown now writes its reason to stderr, so a pool that disappears during a build is
  diagnosable instead of silent.

- **Added a build diagnostic for images that bypass the image pipeline.** A raw `<img>` pointing at
  a public PNG/JPEG the optimizer already converted is reported with its file, line, and the bytes
  the page ships versus the generated WebP. The optimization was previously performed and silently
  unused.
- Route bundles for the browser no longer carry the `<html lang>` rewrite helper, which only a
  server entry can use.

### API Naming

- `card` replaces `twitterCard` on `<Seo>` and in route metadata. `twitterCard` still works and is
  marked deprecated; the emitted `<meta name="twitter:card">` attribute is unchanged, since that is
  the name the crawler still reads.
- Site URL resolution reads one framework-owned `RUVYXA_SITE_URL` variable rather than a list of
  host-specific environment variable names.
- `ServerConfig.trusted_proxy_ips: Vec<IpAddr>` is now `trusted_proxies: TrustedProxies`, since the
  field has to hold prefixes rather than addresses. The `security.trustedProxyIps` configuration key
  and its accepted values are unchanged; only the internal Rust field, which is not published to
  crates.io, is affected. The workspace crates are now marked `publish = false` to keep that so.

### Documentation

- Added page-metadata sections to the English and Thai routing guides, the `site` block to both
  configuration guides, and `head` plus `createPluginHarness` coverage to both plugin guides,
  including a first-party plugin list that calls out `fonts()`.
- Documented the `RUV2102` plugin-definition diagnostic.
- Documented the same-origin algorithm, the sliding-window rate limiter, CIDR support in
  `trustedProxyIps`, the two `@ruvyxa/auth` rate-limit buckets, and the worker environment variables
  in both the English and Thai guides, and corrected `RUV3102`, which was documented as a WebAuthn
  failure rather than a rate-limit rejection.
- Corrected `RUVYXA_WORKER_TIMEOUT` to `RUVYXA_WORKER_TIMEOUT_MS` and the build default from 2 to 5
  minutes in the Thai API-routes guide, and rewrote the worker-pool architecture reference, which
  described an in-process Rust thread pool that no longer exists, to document the Node/Bun pool that
  does.

### Benchmarks

- Refreshed the minimal-starter benchmark on Windows with Node.js 22.23.1, npm 10.9.8, and pnpm
  11.17.0. Across three cold-cache runs, Ruvyxa 1.0.25 recorded a 1.698 s median production build,
  1.103 s dev readiness, 0.917 s production readiness, and 37,381 requests/second. The comparison
  used Next.js 16.2.12 and Astro 7.1.4 under the same harness; exact conditions and limitations are
  recorded in the README.
- The benchmark uses local packed 1.0.25 artifacts for Ruvyxa and compares minimal starter output;
  it is not a universal framework performance ranking.

## v1.0.24 (2026-07-27)

### Breaking: Unified Plugin API

- Replaced the previous `definePlugin({ name, setup })` API with `definePlugin({ name, register })`
  from the new `ruvyxa/plugin` export. Existing plugins must migrate their configuration and
  imports.
- Replaced the flat setup callbacks with grouped sockets: `http` (`onRequest`, `onResponse`, and
  `route`), `build` (`onStart`, `onResolve`, `onLoad`, `onTransform`, and `onComplete`), `dev`
  (`onFileChange`), `diagnostics`, and `native`. One plugin can register across any of these
  sockets.
- Replaced middleware `routes` with `match` and request/response callback arguments with typed
  context objects. Request hooks can continue with `next(request?)`; response hooks can continue
  with `next(response?)`.
- Migrated the built-in plugins plus `@ruvyxa/auth`, `@ruvyxa/database`, and `@ruvyxa/realtime` to
  the same contract. Each official package now exposes its plugin integration through `./plugin`.
- Replaced the old scaffolding command with `ruvyxa plugin create <name>`. The generated package is
  a TypeScript npm package with source, tests, typed framework dependencies, and a minimal headers
  example; it does not require plugin-specific package metadata.

### Plugin Runtime, Build, and Development

- Reworked the Node/Bun plugin runtime and Rust host bridge around one NDJSON protocol with
  deterministic registration, hook-failure reporting, diagnostics, and response-size limits.
- Added validation at plugin definition and configuration boundaries: a plugin requires a non-empty
  name and `register(api)` function, and invalid configured plugin objects fail during startup.
- Added plugin-aware source resolution for aliases, virtual modules, loading, transforms, lifecycle
  hooks, and dependency invalidation. Exact dependency aliases now carry
  source-specifier-to-resolved path bindings through compilation and dynamic-import chunking.
- Kept one TypeScript plugin worker/registry alive for the complete production build, so lifecycle
  and bundler hooks share initialization instead of restarting the runtime for each phase.
- Normalized development file-change notifications to project-relative paths and wired plugin hooks
  through native, standalone, and development execution paths without relaxing server/client
  boundary checks.

### Correctness and Security

- **Fixed stale client navigation pending state.** Concurrent route loads now use navigation IDs, so
  a completed older navigation cannot clear the pending state of a newer one.
- **Fixed binary Vercel responses.** The adapter now preserves response bytes by creating its body
  from `arrayBuffer()` data rather than decoding it as text.
- **Fixed package-manager detection on Windows.** `create-ruvyxa` supplies the process environment
  and uses shell execution for `.cmd` shims, allowing npm/pnpm commands to be detected correctly.
- Hardened plugin-controlled request rewrites: targets must be absolute application paths, percent
  decoded segments cannot introduce `/`, `\\`, `.`, `..`, controls, malformed encoding, or invalid
  UTF-8, and external URI targets are rejected.
- Hardened plugin scaffolding input validation by rejecting absolute and drive-prefixed `--dir`
  values and plugin names containing consecutive hyphens.

### Tooling, Documentation, and Release Reliability

- Added English and Thai plugin-authoring guides with the new API, lifecycle flow, socket selection,
  route matching, local package workflow, and HTTP/build examples. Added English and Thai error
  handling guides and updated CLI, configuration, architecture, official-package, demo, and guide
  navigation references.
- Migrated the demo plugins and configuration to the new API and expanded compiler, plugin, core,
  official-package, Vercel-adapter, router, and scaffolding test coverage for the new behavior.
- Updated release validation and package smoke coverage: version bumping synchronizes
  plugin-template peer and development dependencies, metadata validation rejects obsolete
  plugin-specific metadata, and tarball smoke tests scaffold, compile, and test a generated plugin
  package.
- Bumped Rust crates, npm packages, platform CLI packages, and starter templates to 1.0.24 while
  keeping workspace dependency ranges aligned.

### Benchmarks

- Refreshed the documented minimal-starter benchmark on Windows 11 Home, Ryzen 7 8845HS, Node.js
  22.23.1, npm 10.9.8, and pnpm 11.17.0. Across three cold-cache runs, Ruvyxa 1.0.24 recorded a
  1.848 s median production build, 1.020 s dev readiness, 0.828 s production readiness, and 44,316
  requests/second; exact Next.js and Astro conditions and limits are recorded in the README.
- Clarified that the benchmark compares minimal starter output, uses local packed artifacts for the
  unpublished Ruvyxa candidate, and is not a universal framework ranking.

## v1.0.23 (2026-07-26)

### Incremental Builds and Hydration Control

- Connected the persistent module graph to production client resolution. Warm builds reuse
  content-verified dependency edges, save graph state only after successful client emission, retain
  untouched entries when route artifacts hit, and invalidate the namespace when evaluated config or
  plugin dependencies change. Build telemetry now reports graph hits and tracked modules.
- Added route-level deferred hydration with `export const hydrate = 'idle'` and `'visible'` while
  preserving `true`/default eager hydration and `false` zero-JS output. Deferred pages share one
  content-hashed loader and do not module-preload the route bundle before its trigger. This is a
  route-level scheduling feature, not component resumability.

### Deployment Compatibility and Security

- Added a read-only adapter inspection protocol and expanded `ruvyxa doctor` with `--target`,
  `--adapter`, and `--json`. Doctor now reports adapter target/runtime/platform/capabilities and
  lists routes the selected deployment target cannot host before a build writes artifacts.
- Unified seven non-breaking security headers across native, standalone, and serverless responses;
  explicit application values retain precedence. Static and Cloudflare `_headers` output receives
  the same defaults. CSP and HSTS remain opt-in because framework-wide values would break valid
  inline bootstrap code or require deployment-specific HTTPS assumptions.
- Added `ruvyxa analyze --format sarif` with optional `--output`. SARIF 2.1.0 is serialized directly
  from existing `RUV####` diagnostics, preserving file locations, fixes, affected routes, and the
  command's non-zero exit status when violations exist.

### Production Build Performance

- **Fixed a responsive-image regression that increased the minimal production build from roughly 2
  seconds to 22 seconds.** The 2,000×2,000 starter image produces one full-size WebP and six
  responsive variants. Variant work was performed sequentially inside a source-level Rayon task, so
  a project with one large image used only one encoder path. Variant resize and WebP encoding now
  run in parallel while preserving deterministic manifest order, content-addressed cache keys, and
  output filenames.
- Extended the concurrency pass beyond responsive variants: full-size image encoding now overlaps
  variant work, asset/style/server preparation overlaps client bundling, and independent dynamic
  `getStaticParams` requests use the existing bounded worker pool instead of waiting route by route.
  Results and errors are reduced in deterministic order, and style files that can share an output
  path remain serialized after directory copies complete.
- Replaced static contiguous route-bundle chunks with a bounded dynamic work queue. Outer route
  workers claim the next available route while nested module resolution and compilation retain their
  separate Rayon pool, preventing an expensive route tail from leaving peer workers idle without
  recursively scheduling both levels in one pool.
- A clean `RUNS=3` comparison through `scripts/bench-frameworks.mjs` measured a **1.5 s** median
  Ruvyxa build, down from **2.1 s** after the first responsive-image fix and **22.1 s** before it.
  The same run measured Next.js 16.2.11 at 6.2 s and Astro 7.1.3 at 2.3 s. Ruvyxa still emits the
  complete responsive image set; the improvement does not disable optimization or remove variants.
- The v1.0.18 CLI built the same fixture in 1.2 s. The remaining difference is the cost of the
  responsive image outputs introduced after that release, rather than the 20-second serialization
  regression.
- A second clean audit run using locally packed, unpublished 1.0.23 packages measured a 1.8 s Ruvyxa
  build, 1.2 s dev readiness, 1.1 s production readiness, and 30,431 requests/second. The comparison
  used Next.js 16.2.12 and Astro 7.1.3; exact conditions are recorded in the README.
- Re-ran the clean benchmark after rebuilding the current release binary: Ruvyxa measured 1.609 s
  build, 1.123 s dev readiness, 1.056 s production readiness, and 41,991 requests/second. The same
  run measured Next.js 16.2.12 at 6.991 s / 3.903 s / 1.183 s / 3,653 requests/second and Astro
  7.1.3 at 2.363 s / 4.624 s / 1.867 s / 3,398 requests/second.

### Build and Scaffolding Correctness

- **Fixed incomplete builds leaving `.build-staging-*` directories behind.** Build staging now has
  an RAII owner from creation until commit, so every validation, bundle, prerender, plugin, and I/O
  error path removes partial output. A forced prerender failure verifies that no staging directory
  remains.
- **Fixed source checkouts scaffolding from an ignored, stale generated template.** `create-ruvyxa`
  now prefers tracked root templates when run from the monorepo and uses the packed template only
  after installation. Both preparation and copy boundaries exclude `.ruvyxa`, `dist`, and
  `node_modules`, preventing build output from leaking into newly created apps.
- Removed unused private linker and CLI parameters after tracing every caller; public and trait
  compatibility parameters remain intact.

### Dependency Compatibility

- Updated direct Rust `base64` usage from 0.22.1 to the latest stable 0.23.0 API. Axum and Oxc
  continue to bring 0.22 transitively until their own stable releases move forward.
- Updated Sass from 1.101.3 to 1.102.0. Registry checks found every other direct Rust and npm
  dependency already at its latest stable release; the Notify 9 line remains prerelease-only.
- Updated the pinned workspace package manager from pnpm 11.15.1 to 11.17.0 and verified the
  existing lockfile with the new version.
- Updated CI/release actions to `actions/checkout` v7, `actions/setup-node` v7, and
  `pnpm/action-setup` v6 so the automation dependency surface is current as well.

### Image Configuration Correctness

- **Fixed: documented `image.variantWidths` configuration was rejected as an unknown field.** The
  runtime config renderer now validates and forwards finite numeric arrays to the native CLI. Custom
  breakpoints work again, and `variantWidths: []` disables responsive variant generation as
  documented.
- Added config serialization coverage for `keepOriginal`, `variantWidths`, quality, lossless mode,
  and worker selection alongside the existing native image optimizer tests.

### Release Reliability

- The release workflow publishes every workspace package instead of relying on a recursive publish
  shape that could omit newly added packages. This prevents the main `ruvyxa` package from
  referencing an adapter version that was never uploaded to npm.
- Release jobs now verify every expected npm package and version after publication, turning a
  partial release into an explicit workflow failure instead of discovering it later through an
  application install error.

### Documentation and Verification

- Updated the README benchmark table, concurrency architecture, and methodology with the post-fix
  clean results, exact framework versions, cold-cache behavior, hardware, and the distinction
  between median startup measurements and the final throughput run.
- Verified with the complete `ruvyxa_cli` test suite, runtime compiler/config tests, TypeScript
  checks for `ruvyxa` and `@ruvyxa/core`, Rust formatting, Prettier, and the three-framework clean
  benchmark.
- Made `scripts/bench-frameworks.mjs` process cleanup portable and exception-safe. POSIX runs now
  own a detached process group, Windows uses tree/port cleanup only on Windows, and every readiness
  or load-test path terminates its server in `finally`, preventing stale listeners from corrupting
  later samples.

## v1.0.22 (2026-07-25)

### Four Additional Deployment Adapters

- Added `@ruvyxa/adapter-railway`, which emits a self-contained Railway service deployment with the
  standalone Node runtime and explicit deployment metadata.
- Added `@ruvyxa/adapter-render`, including Render Web Service and Blueprint-compatible output for
  deploying the generated standalone server.
- Added `@ruvyxa/adapter-firebase`, which packages static assets for Firebase Hosting and dynamic
  routes for Cloud Functions v2 while preserving Ruvyxa's route and rendering contracts.
- Added `@ruvyxa/adapter-aws`, which emits AWS Amplify Hosting static and compute artifacts for
  hybrid Ruvyxa applications.
- The CLI recognizes Railway, Render, Firebase, and AWS alongside the existing Node, Bun, static,
  Vercel, Netlify, and Cloudflare targets. The main `ruvyxa` package includes the new adapters in
  its deployment surface so configured and auto-selected builds use one adapter contract.

### Deployment Contract Alignment

- Extended shared adapter output types and standalone-server helpers for the four new platforms,
  keeping generated assets, client bundles, function handlers, and runtime metadata aligned with the
  existing deployment targets.
- Expanded adapter-runner validation and package smoke coverage so generated deployment artifacts
  are materialized inside the atomic build staging directory and required runtime files are present
  in package output.
- Updated realtime deployment guards and package guidance for targets whose server runtime can host
  the native self-hosted WebSocket transport.

### Documentation and Packaging

- Added dedicated Railway, Render, Firebase, and AWS package documentation plus an architecture
  reference for the full adapter matrix.
- Expanded the English and Thai deployment, CLI, plugin, Netlify, realtime, and static-adapter
  troubleshooting guides.
- Bumped all Rust crates, npm packages, platform CLI packages, and starter templates to 1.0.22 and
  synchronized workspace dependency ranges and the lockfile.

## v1.0.21 (2026-07-24)

### Packaging Fix

- **Fixed: `ruvyxa build` failed with `ERR_MODULE_NOT_FOUND` for `entry-templates.mjs`.** The
  `runtime/entry-templates.mjs` module — which `worker-pool.mjs` imports to compose route element
  trees — was missing from the `"files"` array in `packages/ruvyxa/package.json`. Published tarballs
  and local installs therefore never included it, causing the Node worker pool health check to crash
  immediately on `ruvyxa build` and `ruvyxa dev`. The file is now listed alongside the other runtime
  modules.

## v1.0.20 (2026-07-24)

### Client-Side Navigation

Ruvyxa route bundles already knew how to re-render into an existing React root; what was missing was
the half that decides _when_ to do so. That half now ships in `@ruvyxa/react`.

- **`<Link>` navigates without a document load.** It renders a real `<a href>`, so it stays
  crawlable, middle-clickable, and functional before hydration or with JavaScript off; the soft
  navigation is a progressive enhancement on top. Modifier-clicks, non-primary buttons, `target`,
  and `download` all fall through to the browser. Prefetch is configurable (`hover` by default,
  `viewport`, or off) and warms the target bundle with `modulepreload` — without executing it, so a
  prefetch can never register a tree built from the wrong parameters.
- **New hooks: `useRouter`, `usePathname`, `useParams`, `useSearchParams`, `useSelectedRoute`.**
  `useRouter` exposes `push`/`replace`/`back`/`forward`/`refresh`/`prefetch` and a `pending` flag
  for a route whose bundle is still loading. The routing context is created on `globalThis` so a
  generated entry can provide it without importing `@ruvyxa/react` — an app may render plain React
  pages and never install the package.
- **The browser matcher is a verified port of the server's.** `createRouteMatcher` in
  `@ruvyxa/react` shares one case table with the serverless handler's matcher
  (`tests/packages/react/route-match.test.mjs`), so a link click and a reload of the same URL always
  resolve to the same route and params, including static-over-dynamic precedence, catch-all
  decoding, and trailing-slash normalization.
- **The build publishes a lean `route-manifest.json`** the router fetches on first navigation —
  `{ path, src, sharedChunks }` per page route only. It deliberately is not `manifest.json`, which
  is a build report carrying absolute source paths that must never reach a browser. The dev server
  synthesizes the same shape at `/__ruvyxa/client/route-manifest.json`, so soft navigation works in
  development too. A missing manifest or an unmatched URL falls back to a full document load.

### Shared Route Composition

- **One source now composes every route's element tree.** The page-in-layouts-in-routing-context
  tree was re-implemented in five places (the Rust bundler, the dev server's SSR/SSG/client
  bundlers, the one-shot renderer, and the serverless registry). Composition now lives in
  `runtime/entry-templates.mjs` with a Rust mirror in `bundler/output.rs`, asserted equivalent by
  `tests/packages/ruvyxa/entry-templates.test.mjs`. A change to how routes are wrapped is a
  single-file change again, which is what makes the routing context reach the browser identically on
  every render path.

### Responsive Images

- **`<Image sizes=…>` now emits a real `srcset`.** For each public PNG/JPEG the build writes a
  downscaled `name-<w>w.webp` at every breakpoint narrower than the source, and `<Image>` builds its
  `srcset` from the same width list (`DEFAULT_DEVICE_WIDTHS`, matched to the optimizer's
  `DEFAULT_VARIANT_WIDTHS` and asserted equal in `tests/packages/react/image-variants.test.mjs`).
  The set is capped at the intrinsic width, so the browser never requests a variant the build did
  not produce. Configure the breakpoints with `images.variantWidths`; an empty array disables
  variants. A custom `loader`, `unoptimized`, or a remote/SVG source opts out untouched.

### Security: Open Redirect in the `redirects()` Plugin

- **Fixed: a wildcard redirect rule could send visitors to another origin.** The matched remainder
  of the request path was concatenated straight into the `Location` header, so with a rule such as
  `redirects([{ source: '/go/*', destination: '/*' }])` a request to `/go//evil.example` — or
  `/go/\evil.example`, which browsers fold the same way — produced `Location: //evil.example` and a
  cross-origin navigation. The remainder is request-controlled; the origin now is not.
  - A rule's reachable origin is fixed by its configured destination: an absolute destination pins
    its own origin, a path destination pins the requesting origin. Only path, query, and fragment
    may come from the request, and a rule whose interpolated destination would leave that origin is
    skipped instead of sent.
  - Destinations a browser reads as another origin (`*`, `//host`, `/\host`, non-http(s) schemes)
    are now rejected when the plugin is constructed, rather than at the first request that exploits
    them.
  - This is the same escape `safeReturnTo` blocks for `returnTo` in `@ruvyxa/auth`; the redirect
    plugin had been missed.

### Stability and Consistency

- **`--adapter bun` now emits a self-contained deployment.** It previously produced only a launcher
  that shelled out to `bunx ruvyxa start`, so a Bun host still needed the CLI and its native binary
  installed at runtime — unlike every other self-hosted target. Bun and Node now share one server
  source (`standaloneServerSource()` in `@ruvyxa/core`), so request ordering, static fallbacks, and
  cache headers cannot drift between the two runtimes. Run it with
  `bun .ruvyxa/deploy/bun/server/index.mjs`; the launcher is still emitted for the CLI workflow.
- **Fixed: a failed background refresh could freeze a cache entry as stale.** In `cache().swr()`, a
  refresh whose commit was rejected (the entry had been replaced or invalidated meanwhile) left the
  old entry flagged as refreshing, and no later reader ever started another refresh. The flag is now
  cleared when the commit does not land.
- **Fixed: the development auth stores grew without bound.** `memoryAuthStore` and
  `memoryRateLimitStore` only reclaimed a key when someone read it again, and rate-limit keys are
  derived from client IPs — one key per address, never read twice. Writes now sweep expired entries
  and evict oldest-first under a 10,000-entry ceiling.
- Every published package declares the same Node floor (`>=22.12.0`). Some advertised `>=22.0.0`
  while the framework they ship with requires `>=22.12.0`, so npm enforced a version that could not
  actually run the code. `pnpm release:validate` now fails on any package that disagrees.
- Published packages include `src`, so the shipped declaration maps resolve. `declarationMap` and
  `sourceMap` were on while `files` listed only `dist`, which pointed every "go to definition" and
  every stack frame at a file that was never in the tarball. `release:validate` enforces the
  pairing.

### Quality Gate

- `noUnusedLocals` and `noUnusedParameters` are enabled in `tsconfig.base.json`, giving the
  TypeScript packages the dead-code gate the Rust crates already get from
  `cargo clippy -- -D warnings`. The workspace passes today, so this only keeps it that way.

### Deployed Apps Now Behave Like `ruvyxa dev`

Five deploy-only failures, all of the same shape: a rule that only the Rust server enforced, so it
disappeared the moment a CDN or a platform bundler stood in front of the app.

- **Fixed: the Netlify function crashed on every request** with
  `ENOENT: no such file or directory, open '/var/task/manifest.json'`. Netlify re-bundles the
  function with esbuild and keeps only what the module graph reaches, so the sibling `manifest.json`
  that the handler read through `import.meta.dirname` never reached the deployed bundle. The route
  manifest now also ships as `manifest.mjs` and every adapter imports it statically — Netlify,
  Vercel, Cloudflare, and the standalone Node server. `included_files` cannot express this on the
  zero-config Frameworks API path, so removing the runtime read was the only host-independent fix.
- **Fixed: `public/` images 404'd on every static host.** Image optimization replaced
  `public/logo.png` with `logo.webp` in the build output, and only `ruvyxa dev`/`ruvyxa start`
  resolved the old URL to the new file. A CDN has no such fallback, so a plain
  `<img src="/logo.png">` broke in production only. The source file is now published beside its
  WebP; opt out with `image: { keepOriginal: false }` when every reference goes through `<Image>`.
- **Fixed: a missing asset returned `200` with an HTML body.** With no file behind it, `/logo.png`
  and `/favicon.ico` fell through to routing and were captured by a bare dynamic route such as
  `/[lang]`, so browsers received a page where they expected image bytes — and every favicon request
  paid for a serverless invocation in the function region. Asset-shaped paths now answer `404` in
  `dev`, `start`, and every adapter; routes that declare the extension themselves (`/sitemap.xml`)
  are unaffected.
- **Fixed: ISR and PPR pages never revalidated on Vercel or Netlify.** Their build-time HTML was
  published as a static file, and both hosts serve a matching static file before invoking the
  function (`handle: filesystem`, `preferStatic`), so the page was pinned to its deploy-time
  snapshot forever. Those pages are now withheld from the publish directory and kept inside the
  function bundle as the first cache entry.
- **Fixed: public assets were served with `max-age=0, must-revalidate` on Vercel**, so every
  navigation re-fetched each image and font. They now carry `public, max-age=3600, must-revalidate`,
  matching the header the Rust server already sent for the same files. Hashed client bundles keep
  their immutable header.
- `.ruvyxa-images.json` (build telemetry: source paths and byte counts) is no longer copied into the
  publish directory.
- Added `vercelAdapter({ regions: ['sin1'] })` to pin the serverless function near your users.
  Static pages are served from the edge, but SSR, API routes, and ISR revalidation run in the
  function region — `iad1` by default, a cross-continent round trip from Asia.

### The Same Audit, Applied to the Remaining Targets

- **Fixed (standalone Node server): `/logo.png` was answered by a page render.** The generated
  server routed before consulting the publish directory for everything except `/__ruvyxa/`, so a
  dynamic route captured the filename and the real file was unreachable. Asset-shaped paths are now
  resolved first, matching the Rust server's order.
- **Fixed (standalone Node server): public assets carried no `Cache-Control` at all**, and a
  PNG/JPEG URL did not fall back to the published WebP the way `ruvyxa start` does — so
  `image: { keepOriginal: false }` worked locally and 404'd in the shipped directory.
- **Fixed (Cloudflare): the Worker's `compatibility_date` was the build date.** Two builds of the
  same commit produced different Workers, and a build machine ahead of the deploy machine's
  `workerd` emitted a date `wrangler` rejects. It is now a fixed, tested default; override with
  `cloudflareAdapter({ compatibilityDate })`.
- **Fixed (Cloudflare): the Worker dropped the execution context**, so `waitUntil` was unavailable
  to anything the shared handler schedules in the background.
- **Fixed (static adapter): no `_headers` file was emitted at all**, so hosts that read one (Netlify
  drops, Cloudflare Pages) served even the content-hashed bundles with a revalidate-every-request
  default.
- Public-asset cache headers were extended to Netlify (`netlify.toml` and `.netlify/v1/config.json`)
  and Cloudflare (`_headers`), which both default to `max-age=0, must-revalidate` for
  publish-directory files.

Compatibility note: `deploy/bun/start.mjs` remains a launcher for projects that intentionally use
the installed CLI workflow. For a self-contained Bun deployment, use the server emitted at
`deploy/bun/server/index.mjs`; it shares the standalone server source with the Node adapter and does
not require the Ruvyxa CLI or native binary at runtime.

## v1.0.19 (2026-07-23)

### Deploy Anywhere: Static Linux Binaries

- Linux CLI binaries (`@ruvyxa/cli-linux-x64`, `@ruvyxa/cli-linux-arm64`) are now fully static musl
  builds. Releases before 1.0.19 were dynamically linked against the build machine's glibc and
  failed on hosts with an older glibc — most visibly Vercel's build image with
  ``/lib64/libc.so.6: version `GLIBC_2.39' not found``. The release pipeline now rejects any
  dynamically linked Linux artifact.

### Zero-Config Deploys Without Root Config Files

- `ruvyxa build` auto-detects the hosting platform from its build environment (`VERCEL`, `NETLIFY`,
  `CF_PAGES`) and runs the matching adapter when no adapter is configured. `RUVYXA_ADAPTER`
  overrides detection.
- All six official adapters are bundled with the `ruvyxa` package: `--adapter <name>` and platform
  detection work with zero installs. A project-installed adapter package still wins, and `--adapter`
  now also accepts any third-party adapter package name (`@scope/…` or `ruvyxa-adapter-…`), reported
  with the tried candidates in `RUV2203` on failure.
- The Netlify adapter now emits Netlify's Frameworks API directory (`.netlify/v1/`: the SSR/API
  function plus immutable cache headers) as a gitignored build artifact — no `netlify.toml` is
  written to the project root by default. Opt back in with
  `netlifyAdapter({ projectConfig: true })`.
- The Cloudflare adapter no longer writes a root `wrangler.jsonc` by default; the deploy directory
  is self-sufficient (`wrangler deploy -c .ruvyxa/deploy/cloudflare/wrangler.jsonc`). Opt back in
  with `cloudflareAdapter({ projectConfig: true })`.
- Fixed: opt-in `netlify.toml` and `wrangler.jsonc` previously embedded the absolute build-machine
  `outDir` (including a transient staging path and Windows backslashes), which broke Netlify deploys
  with a 404 on every route when the file was committed. Generated configs now embed
  project-relative POSIX paths only (`projectRelativeOutDir` in `@ruvyxa/core`).

### Standalone Node Server

- The Node adapter now emits a self-contained server at `.ruvyxa/deploy/node/server/index.mjs`
  (plain `node:http` around the shared serverless handler, static assets from `deploy/node/public`,
  `PORT`/`HOST` env, SSR/API/ISR/PPR/SSG/CSR). It runs on any Node host — Docker, PM2, systemd, any
  PaaS — with no ruvyxa CLI or native binary at runtime.

### Correctness

- Immutable cache headers for hashed client bundles now target the real URL prefix
  `/__ruvyxa/client/*` on Vercel, Netlify, and Cloudflare; the previous `/client/*` rules never
  matched, so hashed bundles were re-downloaded on every visit.
- `static-site` adapter artifacts can be marked `optional`, tolerating API-only builds with no
  prerendered pages instead of failing with `RUV2202`.
- Identical function bundles emitted at several destinations (deploy directory + platform discovery
  directory) are compiled once and copied, keeping build time flat.

## v1.0.18 (2026-07-22)

### Markdown Content Route Validation

- Boundary validation and rendering-strategy detection no longer treat fenced code blocks and inline
  code spans in `page.md`/`page.mdx` content routes as executable code. A guide that shows
  `process.env.SECRET` or `import 'server-only'` inside an example previously failed the build with
  false `RUV1007`/`RUV1008`/`RUV1009` diagnostics, and an example containing `fetch(` could silently
  demote a static page from SSG to SSR. MDX ESM outside fences is still validated.

### Bundler: Windows Path Normalization

- The package-`exports` resolver branch now strips Windows verbatim path prefixes (`\\?\`) the same
  way as every other resolver branch. Mixed prefixes previously broke shared-route chunk planning on
  Windows with `build.split: 'route'` in npm-layout projects, failing production builds with
  `prepared shared route module is unavailable: …\react\index.js`.

Both defects were found by building the Ruvyxa documentation site with the framework itself.

## v1.0.17 (2026-07-22)

### Official Data, Auth, and Realtime Packages

- Added `@ruvyxa/database`, a typed CRUD and transaction facade with Prisma-compatible, DynamoDB,
  and custom adapter contracts plus production environment validation.
- Added `@ruvyxa/auth`, with opaque durable sessions, credentials, OAuth PKCE, magic links,
  delegated WebAuthn, atomic replay/rate-limit contracts, and browser/server entrypoint separation.
- Added `@ruvyxa/realtime`, with action opt-in metadata, a bounded native Axum WebSocket transport,
  same-origin and channel filtering, reconnect/resync support, and explicit unsupported-target
  failures.
- The bundler and graph validator now treat root `@ruvyxa/auth` and `@ruvyxa/database` imports as
  server-only (`RUV1007`); browser code uses the `/client` entrypoints.

### Hardening

- Realtime transport paths are validated against reserved framework routes (`/__ruvyxa/hmr`,
  `/__ruvyxa/client`, `/__ruvyxa/action`, `/__ruvyxa/trace`) on both the TypeScript plugin runtime
  and the Rust dev server, failing configuration with a clear `RUV1701` diagnostic instead of a
  router panic at startup.
- The WebAuthn `options` endpoint now consumes the shared auth rate limit and reports failures
  through the same fail-closed error path as every other credential endpoint.
- The realtime browser client's `subscribe` no longer depends on `this` binding, so destructured
  usage (`const { subscribeRoute } = client`) works correctly.

### Plugin Infrastructure

- One `definePlugin({ name, setup })` registry now provides `resolveId`, `transform`, request and
  response middleware, and `onBuildComplete` hooks through a persistent Node/Bun plugin host, with
  NDJSON protocol isolation and per-plugin validation of names, hooks, and middleware route
  patterns.
- Middleware `routes` unions are reported to the native server, which skips the plugin round-trip
  entirely for requests no middleware can match.
- Added a configurable middleware worker pool (`middleware.workers`, 1–8) with round-robin dispatch,
  per-hook timeouts, crash restart with single retry, and replacement without retry on timeout or
  protocol errors.

### Content Engine and React Primitives

- Added the `contentEngine()` plugin: scans native `app/**/page.md(x)` routes once and derives
  `/content.json`, `/search-index.json`, `/rss.xml`, `/sitemap.xml`, and an experimental `/llms.txt`
  from frontmatter and body, live in development and byte-equivalent in production.
- Added the `Answer` component to `@ruvyxa/react` for schema.org Question/Answer microdata rendered
  from author-written content.
- SEO metadata API now supports `article` and `breadcrumbs` structured data, and `image`/`type`
  replace the previous `ogImage`/`ogType` property names.
- The render pipeline supports `header_pairs` so responses can carry multiple headers with the same
  name (for example several `set-cookie` values); header insertion appends instead of overwriting.

### Runtime and Tooling

- Persistent worker pool request handling was extended with a dedicated test suite and shared
  fixture workspace for API, compiler, and worker-pool tests.
- Serverless runtime adapters were expanded, including Cloudflare adapter updates and deployment
  documentation for every adapter target.
- `ruvyxa doctor` no longer reports a Deno version check; toolchain reporting focuses on the
  supported Node and Bun runtimes.
- Automated npm package smoke testing (`pnpm pack:smoke`) validates packed tarballs, template
  scaffolds, and Content Engine build artifacts.

### Performance: Static Serve Hot Path

- Production SSG responses are now served from the in-memory render cache after a single disk read,
  instead of re-opening the prerendered HTML file on every request. Measured on the minimal starter:
  ~1,300 → ~31,700 requests/second (p50 <1 ms) at 25 connections.
- The route manifest and radix router are shared via `Arc` instead of deep-cloned per request.

### Zero-JS Content Pages

- `export const hydrate = false` opts any server-rendered page (SSR, SSG, ISR, PPR) out of client
  hydration: the served and prerendered HTML contains no script tags and the production build emits
  no client bundle for that route. `'use client'` (CSR) pages ignore the export — the directive
  wins. Interactivity does not run on opted-out pages.

### Documentation and Benchmarks

- Added a measured benchmark suite against the Next.js and Astro minimal starters with a
  reproducible harness at `scripts/bench-frameworks.mjs`; results and methodology are published in
  the README.
- Added user guide chapter 15, "Official Packages: Database, Auth & Realtime" (English and Thai).
- Rewrote the Routing and Server & Client Components guides and expanded Getting Started with a
  first-10-minutes path and troubleshooting tables (English and Thai).

### Bundler: Resolution Cache

- Cache parsed `package.json` `exports` fields per package, fingerprinted by (mtime, len).
  Bare-specifier resolution (`react`, `react/jsx-runtime`, etc.) no longer re-reads and re-parses
  the same `node_modules` package.json for every importing module — the file is read once per build
  and served from cache thereafter, invalidated automatically if the file changes.

### Dev Server: Modular Architecture

- Split `ruvyxa_dev_server` into focused modules: `action_security.rs` (origin/fetch-metadata
  validation and per-key rate limiting), `cli_output.rs` (structured terminal formatting),
  `env_file.rs` (environment variable file parsing), `html_document.rs` (HTML document manipulation
  and template rendering), `plugin_bridge.rs` (plugin communication and lifecycle management),
  `port_binding.rs` (port availability detection and binding), and `static_assets.rs` (asset serving
  and caching strategies).
- Reduced `lib.rs` from ~1675 lines to ~108 lines of focused public exports, improving separation of
  concerns and maintainability.
- Extracted the rendering pipeline into `render_pipeline.rs` (SSR/SSG/ISR/CSR/PPR strategy dispatch,
  worker-pool render paths, ISR revalidation, and the render-process fallback), leaving `lib.rs`
  with server core only (config, serve loop, watcher, HTTP handlers).
- Response plugin middleware no longer fails oversized responses: a response whose sized body
  exceeds `plugin_response_body_limit_bytes` is now passed through unmodified (with a warning log)
  instead of returning a 500. Response plugins are skipped only for that response.
- Extended the oversized pass-through to unsized (streaming) response bodies: chunks are buffered up
  to the limit, and on overflow the already-read chunks are replayed in front of the untouched
  remainder so the response is served byte-identically instead of failing. Genuine body read errors
  still return a 500.

### Built-in Plugins and Middleware Fast Path

- Added `ruvyxa/plugins` package with first-party plugins: `redirects` (declarative 307/308
  redirects with wildcard remainders), `headers` (route-scoped response headers), `sitemap` and
  `robots` (build-time `sitemap.xml`/`robots.txt` generation from the route manifest), `alias`
  (exact import specifier resolution), `bundleBudget` (fail build when client JavaScript exceeds
  per-chunk or total KiB budgets), and `requireEnv` (fail build when required environment variables
  are missing or empty).
- Added native middleware fast path: the plugin registry reports middleware route patterns per
  direction, and the Rust server skips the plugin stdio round-trip for requests no middleware can
  match. Registries without request middleware no longer pay any per-request plugin cost.
- Added automatic plugin host recovery: when the persistent TypeScript plugin process crashes, the
  server restarts it once and retries the in-flight hook instead of failing subsequent requests.
- Added opt-in `middleware.workers` setting (1-8, default 1) for plugin runtime worker pool with
  round-robin dispatch and per-process crash recovery.
- Added the `ruvyxa/plugins` runtime alias for workspace and packed installs compatibility.
- Updated demo app to integrate `sitemap`, `bundleBudget`, and two-worker middleware pool as
  integration coverage.

### Runtime Rendering Consolidation

- Removed standalone `action-renderer.mjs`, `client-renderer.mjs`, and `ssg-renderer.mjs` modules.
- Consolidated all rendering operations (SSR, SSG/ISR/PPR, API, actions, client) into the persistent
  `worker-pool.mjs` process.
- Added `ssr-renderer.mjs` and `api-renderer.mjs` as standalone fallbacks when the worker pool is
  unavailable.
- Updated package manifests, smoke tests, and documentation to reflect the consolidated runtime
  architecture.

### Edge Runtime Bundle Target

- Added Edge bundle target variant for Cloudflare Workers and Vercel Edge Functions.
- Updated bundler to treat Edge bundles like SSR with server-side rendering but restricted Node.js
  APIs.
- Extended resolver to use `edge-light` condition for Edge target exports resolution.
- Added `serverless-handler.mjs` runtime for invoking Edge render functions.
- Updated adapter implementations (Vercel, Netlify, Cloudflare) to support full server rendering
  including SSR, API, and ISR routes on edge platforms.
- Added Edge runtime rendering tests across all three serverless adapters.

### Plugin Scaffolding Enhancements

- Added a custom plugin package directory option with path traversal protection.
- Changed default plugin output from `plugins/<name>` to root-level `<name>` directory.
- Generated plugin packages now include npm, pnpm, and Bun setup instructions in README templates.
- Added `scope` and `skipped` optional fields to adapter artifact reports for fine-grained build
  tracking.

### Platform and CI Improvements

- Normalized Windows path handling across bundler, CLI, dev server (HMR tracker, style modules), and
  diagnostics using `normalized_canonical_path()` utility.
- Expanded Bun runtime parity tests to Windows in CI workflow.
- Replaced environment variable runtime selection with explicit `--runtime` CLI flag for
  cross-platform consistency.
- Fixed Windows reserved port range handling (WSAEACCES 10013 errors from Hyper-V/WinNAT port
  exclusions) during dev server listener binding.
- Cleaned up unused `base64` dependency from `ruvyxa_middleware`.
- Improved npm package existence check reliability with Windows shell compatibility.

### Documentation

- Updated v1.0.16 release notes with comprehensive coverage of build output enhancements, server
  actions improvements, runtime detection, Bun support, progressive phase reporting, and CI/CD
  upgrades.
- Enhanced Thai CLI commands guide with detailed pipeline descriptions, `.ruvyxa/` output structure,
  `build.json` timing metadata, and command examples.
- Updated English and Thai plugin guides with built-in plugin documentation and middleware worker
  pool configuration.
- Updated deployment guides with Edge runtime serverless adapter capability matrix.

## v1.0.16 (2026-07-20)

### Plugin System Overhaul

- Replaced the split legacy plugin model with one TypeScript-native `definePlugin({ name, setup })`
  registry loaded from `ruvyxa.config.ts`.
- Added the typed setup API for `addMiddleware`, `resolveId`, `transform`, and `onBuildComplete`,
  with shared plugin state and deterministic registration order across server and build phases.
- Added `plugin(name, middleware)` as the compact authoring path for request/response middleware;
  `definePlugin({ name, setup })` remains available for plugins that also register build hooks.
- Added Fetch-native request and response middleware using standard `Request` and `Response` values;
  `undefined` continues, a returned `Request` replaces the request, and a returned `Response`
  short-circuits or replaces the response.
- Added route-scoped middleware matching with exact, wildcard, and prefix patterns, plus plugin
  context metadata containing the plugin name and project root.
- Added the persistent `runtime/plugin-runtime.mjs` Node/Bun registry process. It validates plugin
  setup, serializes hook results through NDJSON, redirects diagnostic logging to stderr, and keeps
  module-level state alive across calls.
- Added lossless request/response transport for binary bodies, query strings, duplicate headers, and
  repeated `Set-Cookie` values using ordered header pairs and base64 bodies.
- Added bounded response buffering through `security.pluginLimit` and Rust-side validation before
  converting plugin output into Axum responses.
- Added post-commit build completion execution so plugins can write deployment metadata and other
  artifacts after the production output is available.
- Replaced the public Rust bundler plugin trait with the internal `BuildHookPipeline` boundary and
  aligned resolver, compiler, source-map, and cache integration with the TypeScript host.
- Added the Rust `PluginHost` middleware bridge with process lifecycle management, descriptor
  validation, serialized hook errors, stderr forwarding, and graceful child cleanup.
- Removed Wasmtime, the raw Wasm ABI, Wasm plugin configuration, custom middleware layers, legacy
  plugin metadata (`enforce`, `parallel`, and hook flags), and the old `plugin-runner.mjs` worker.
- Simplified plugin scaffolding into a publishable npm package workflow with `src/index.ts`, package
  metadata, TypeScript build settings, and usage documentation.
- Updated package exports, runtime file manifests, keyword metadata, templates, configuration
  validation, README files, architecture references, English guides, and Thai guides for the new
  plugin lifecycle.
- Added focused coverage for plugin validation, persistent transform state, Fetch middleware, binary
  response preservation, repeated cookies, build completion, imported-plugin cache invalidation, CLI
  scaffolding, and Rust host protocol decoding.
- Removed orphaned Wasmtime dependencies from the workspace lockfile and verified packed npm output
  includes `runtime/plugin-runtime.mjs` without legacy runtime files.

### Built-in Plugins and Middleware Fast Path

- Added the `ruvyxa/plugins` package entry with first-party plugins built on the public hook API:
  `redirects` (declarative 307/308 redirects with wildcard remainders), `headers` (route-scoped
  response headers), `sitemap` and `robots` (build-time `sitemap.xml`/`robots.txt` generation from
  the route manifest into the served asset directory), and `alias` (exact import specifier
  resolution ahead of the native resolver).
- Added a native middleware fast path: the plugin registry now reports the union of middleware route
  patterns per direction, and the Rust server skips the plugin stdio round-trip — including request
  body base64 encoding and response buffering — for requests no middleware can match. Registries
  without request middleware no longer pay any per-request plugin cost, and older runtimes that do
  not report routes keep the previous match-all behavior.
- Added automatic plugin host recovery: when the persistent TypeScript plugin process crashes or its
  pipes break, the server restarts it once and retries the in-flight hook instead of failing every
  subsequent request. Hook-level errors are never retried.
- Added `bundleBudget` (fail the build when emitted client JavaScript exceeds per-chunk or total KiB
  budgets) and `requireEnv` (fail the build when required environment variables are missing or
  empty) to `ruvyxa/plugins`, and taught `sitemap` to read the committed route manifest when the
  build summary omits the route list.
- Added the opt-in `middleware.workers` setting (1-8, default 1): the server starts a pool of
  identical plugin runtime processes dispatched round-robin for middleware-heavy workloads, each
  with independent crash recovery. Module-level plugin state is per-process, so the default stays at
  one worker.
- Added the `ruvyxa/plugins` runtime alias so `ruvyxa.config.ts` can import built-in plugins inside
  the workspace and from packed installs, and wired the demo app to `sitemap`, `bundleBudget`, and a
  two-worker middleware pool as integration coverage.

### Large-Build and Content Compiler Follow-up

- Split route bundling into reusable prepare/emit stages so cold route-split builds resolve,
  compile, validate, and plan dynamic imports once, then perform only the final shared-aware
  link/minify/output pass.
- Added lightweight content-validated route-plan caching while preserving final artifact reuse;
  dynamic-import dependencies now participate in artifact invalidation instead of allowing stale
  lazy chunks after a source edit.
- Parallelized route preparation and final client emission while retaining deterministic
  manifest/output order and the existing `build.workers` bound.
- Replaced per-route dependency re-reading during warm artifact validation with one build-scoped,
  content-based fingerprint snapshot, preventing shared layouts and packages from being hashed
  repeatedly across large route sets.
- Replaced line-based MDX ESM extraction with markdown-rs MDX boundaries backed by Oxc syntax
  feedback, including multiline imports and exports.
- Combined MDX with GFM tables, task lists, strikethrough, autolink literals, references, and
  footnotes; added semantic table headings/alignment, reference resolution, stable duplicate heading
  slugs, JSX member/spread support, comments, and Markdown element component overrides.
- Upgraded frontmatter from a scalar line parser to locked `serde_yaml_ng` parsing for nested maps,
  arrays, quoted values, and block scalars, with actionable `RUV1312` failures for malformed or
  non-mapping documents.
- Aligned the packaged Node content compiler with the native contract using locked `yaml` and
  `remark-gfm` dependencies; Node SSR/SSG now preserves nested frontmatter, renders the documented
  GFM surface, and derives stable heading exports and rendered IDs from the same MDX AST.
- Added focused cache/concurrency regressions plus native MDX unit, full-bundler integration, and
  Node runtime parity coverage.
- Reused the first Oxc transform during Node module linking and added a bounded content-keyed
  transform cache, removing repeated work both within a graph and across identical route inputs.
- Memoized plugin-free native dependency closures, reused a production-build source snapshot, and
  cached successful native Markdown/MDX compilation results with bounded storage.
- Loaded prerender client assets once per build and shared immutable CSS across jobs instead of
  parsing the manifest and cloning the complete stylesheet for every route.
- Emitted the cold shared-route registry from prepared modules for plugin-free builds and persisted
  a fingerprint-validated warm artifact; shared source edits invalidate both the registry and
  affected route artifacts, while plugin builds retain their existing hook pass.
- Reduced the isolated 16-route demo benchmark from 13.61s to 4.02s cold and from 1.94s to a 1.62s
  warm median, with cold prerender down 89.2% and warm client bundling down 93.3%.

### Build Output and Release Profile

- Added progressive build phase reporting that displays real-time progress with per-phase durations
  for route discovery, validation, asset preparation, client bundling, and prerendering, so
  developers see timing as each stage completes rather than waiting for a single final summary.
- Added release profile optimizations (`thin` LTO, single codegen unit, symbol stripping) to
  `Cargo.toml` for smaller binaries, faster downloads, and improved runtime performance.
- Refactored build summary output into incremental metrics with a route size table and consolidated
  timing information for easier post-build inspection.
- Enhanced plugin scaffolding output with a visual file tree and numbered next steps for faster
  developer onboarding.

### Server Actions and Streaming

- Passed request headers through the server action rendering pipeline so Actions receive the
  originating `HeaderMap` via the worker pool and action renderer.
- Collected response headers from action handlers (`append`-style, multi-value) and propagated them
  back through the render pipeline to the HTTP response.
- Optimized the render cache recency tracking from O(n) linear queue scans to O(1) operations via a
  hash-indexed doubly linked list, replacing `VecDeque` with explicit `RecencyLinks` and
  `RecencyList`.
- Switched the API response stream from unbounded MPSC channels to bounded channels with capacity
  `MAX_PENDING_RESPONSE_FRAMES`, applying backpressure at the channel layer instead of manual queue
  overflow detection.

### Runtime Detection and Bun Support

- Added Bun as a selectable JavaScript runtime alongside Node, with `RUVYXA_RUNTIME` environment
  variable support for runtime override.
- Implemented `JavaScriptRuntime::detect()` to automatically select Node or Bun based on
  availability: Node is preferred, Bun is selected only when Node is unavailable and Bun can be
  executed, and Node is kept as the diagnostic target when neither runtime is installed.
- Extended `ServerConfig` and `ProjectConfig` with a `runtime` field (`"node"` or `"bun"`), and
  updated the worker pool, config renderer, and dev server to initialize with the selected runtime.
- Added the `@ruvyxa/adapter-bun` package for Bun-based deployment and launcher integration.
- Documented runtime configuration, automatic detection, and Bun parity guidance in English guides,
  Thai guides, README, and architecture references.

### Documentation

- Added comprehensive Thai CLI commands guide with structured sections, common options reference,
  detailed pipeline descriptions, `.ruvyxa/` output structure, `build.json` timing metadata, and
  command examples.
- Added system architecture reference guide spanning Rust/Node.js architecture, crate dependency
  maps, compilation pipeline stages, route graph algorithms, bundler resolution order, CSS Module
  handling, middleware plugin lifecycle, dev server hot reload, wire protocol specifications, error
  codes, and data flow diagrams.
- Added detailed architecture module guides for the bundler, CLI, concurrency, dev server,
  diagnostics, graph, middleware, protocols, security, and worker pool, with reference
  implementations and code examples.
- Removed archived architecture documentation (`build-performance-and-mdx.md`,
  `bundler-modernization.md`, `production-readiness.md`) after their content was integrated into the
  new architecture guides.

### CI/CD, Tooling, and Cleanup

- Rebranded CI/CD workflow and job names with consistent framework references and consolidated
  security scanning into primary workflows, removing the redundant standalone `security.yml`.
- Upgraded `pnpm/action-setup` from v4 to v5 and consolidated pnpm version management to the
  repository `packageManager` field as the single source of truth.
- Extended version-bump automation to iterate over all starter templates (`minimal`, `blog`, `crud`,
  `api-backend`) and validate framework dependencies across every template.
- Improved npm package existence check reliability with Windows shell compatibility and explicit
  error handling for `npm view` failures.
- Removed unused `anyhow` and `walkdir` dependencies from `ruvyxa_bundler`, `tower-http` from
  `ruvyxa_dev_server`, and `base64` from `ruvyxa_middleware` to reduce build footprint and
  transitive dependency counts.
- Bumped all npm workspace packages and Rust crates to `1.0.16` and regenerated `Cargo.lock` with
  synchronized dependency versions.

## v1.0.15 (2026-07-18)

### Full-System Reliability Hardening

- Hardened the API worker protocol so streamed responses require an explicit `api-end` terminal
  frame; premature EOF, worker crashes, and stream errors now reach the HTTP consumer instead of
  being reported as successful truncated responses.
- Preserved binary request and response bodies, query strings, duplicate request headers, and
  repeated `Set-Cookie` response headers across the Rust/Node worker boundary.
- Centralized request-path canonicalization to decode valid URL segments consistently while
  rejecting malformed escapes, encoded separators, traversal segments, and unsafe prerender paths.
- Fixed runtime-directory resolution for installations whose paths contain spaces or other
  URL-encoded characters by using filesystem-safe URL conversion throughout the Node runtime.
- Made automatic JSX the consistent default across the Rust bundler, CLI, dev server, and Node
  renderers; classic JSX remains available as an explicit opt-in.
- Validated JSX runtime configuration at startup and linked the generated `react/jsx-runtime` helper
  imports correctly in SSR, SSG, client, and worker bundles.
- Extended package `exports` resolution with target-aware conditions, wildcard subpaths, array
  fallbacks, explicit blocked entries, package-root containment checks, and safer filesystem
  fallback behavior.
- Preserved the server/client boundary and private environment-variable checks while improving
  resolver and compiler cache invalidation behavior.
- Corrected CORS ordinary `OPTIONS` handling, preservation of all `Vary` values, trusted-proxy
  forwarding rules, loader request lifecycle handling, cache-duration validation, and related
  middleware/runtime regressions.
- Updated CLI/configuration documentation and the full-flow smoke script to match the maintained
  `examples/demo` fixture and current JSX defaults.

### Client Bundling Reliability

- Fixed the Node runtime compiler's client-module initialization order. It now performs a stable
  dependency-first traversal instead of reversing module discovery order, which was not a valid
  topological order when separate graph branches shared React or another dependency.
- Prevented client components that import React hooks from failing at `/__ruvyxa/client` with
  `Cannot access '__m…' before initialization` during development or hydration bundle evaluation.
- Added a runtime compiler regression that reproduces the cross-branch shared-dependency graph and
  evaluates the generated bundle to prove every acyclic local dependency initializes before its
  importers.
- Kept the Node runtime behavior aligned with the Rust bundler's existing dependency-first linker
  without changing compiler APIs, entry exports, module identifiers, or source-map behavior.

### Release Metadata and Documentation

- Bumped all npm workspace packages and Rust crates to `1.0.15` and regenerated `Cargo.lock`.
- Updated the minimal starter to require both `ruvyxa` and `@ruvyxa/react` `^1.0.15`.
- Updated the version-bump workflow so future releases keep both starter framework dependencies in
  sync; the ignored `create-ruvyxa` package copy continues to be regenerated from the source
  template during prepack.
- Documented the client initialization root cause, applied repair, and regression evidence in the
  July reliability audit.

### Stability and Compatibility Follow-up

- Fixed Node worker environment parsing so values with trailing units or other extra characters,
  such as `1234ms` and `64mb`, are rejected and safely fall back instead of being partially parsed.
- Preserved conditional `package.json` `exports` key declaration order to match Node resolution
  semantics without changing JSON ordering behavior elsewhere in the workspace.
- Assigned the unique `RUV1804` diagnostic code to invalid JSX runtime configuration, keeping
  `RUV1803` reserved for circular dependency diagnostics.
- Added regression coverage for malformed worker configuration, invalid JSX runtime diagnostics,
  conditional exports declaration order, early API-stream termination, encoded URL boundaries, and
  cross-runtime JSX helper linking.
- Revalidated the release surface: 325 Rust tests, workspace clippy with warnings denied, npm
  build/check/test, demo parity for all 16 routes, package metadata validation, and packed-package
  consumer type checks all pass on Windows x64.
- No tracked critical files were deleted or missing, and no dependency was removed without direct
  evidence of being orphaned; generated build, cache, and package-smoke outputs remain excluded from
  the tracked release surface.

## v1.0.14 (2026-07-16)

### Reliability and Configuration Safety

- Normalized `RUVYXA_WORKER_TIMEOUT_MS` and `RUVYXA_MEMORY_LIMIT_MB` in the persistent Node worker:
  invalid or zero values now safely retain the 30-second watchdog and 512 MiB cache-pressure
  threshold instead of silently disabling protection.
- Aligned the Rust worker-response and API stream-idle timeout with the normalized
  `RUVYXA_WORKER_TIMEOUT_MS` value passed to Node. Interactive requests now consistently use the
  documented 30-second fallback, while build workers retain their 300-second fallback unless
  explicitly overridden. Values above Node's 2,147,483,647 ms timer limit now fall back safely
  instead of being coerced by Node to a 1 ms timeout.
- Bounded environment-derived `RUVYXA_RENDER_CACHE_SIZE` at 16,384 entries before render-cache
  allocation, while preserving `0` as an explicit cache-disable setting and preserving existing
  development and production defaults.
- Added regression coverage for worker environment fallback and render-cache capacity normalization.
- Streamed API response bodies from Node workers into Axum with binary-safe 64 KiB Base64 frames, a
  bounded 16-frame per-response queue, idle timeouts, stdout backpressure, and stream error
  propagation instead of materializing each response as one text value.
- Kept the API worker protocol backward-compatible: new Rust callers accept legacy single-message
  responses, while new Node workers retain that response shape unless streaming is requested.
- Added Rust and Node regressions for binary reconstruction, large multi-frame responses, queue
  overflow, stalled streams, worker errors, request capability serialization, and legacy fallback.
- Corrected the README cache description from FIFO to its implemented LRU policy and documented the
  supported worker/cache environment settings in English and Thai CLI guides.
- Refreshed the July reliability audit with current v1.0.14 bundler context, applied repairs, and
  the completed streaming API-response IPC repair.

### Bundler and Build Pipeline

- Added shared module bundling and a shared-route registry so modules common to multiple routes can
  be compiled once and reused instead of duplicated in every client bundle
- Added `bundle_shared_route_modules()` and shared-route output types for producing executable
  shared module registries
- Added linker support for shared modules, dynamic imports, dependency-first linking, and exclusion
  of already-emitted shared modules from individual route bundles
- Added `collect_module_manifest` and improved static-module tracking for more accurate chunk and
  module manifests
- Integrated shared-module output with the CLI build pipeline, render cache, and development server
- Added async build phases for route discovery, validation, preparation, client bundling, and
  prerendering
- Added per-phase timing metrics and total build duration reporting in build output metadata
- Added a prerender worker pool that chooses parallelism from route count and available CPU capacity
- Migrated static prerendering and SSG rendering to the async worker-pool workflow
- Replaced the duplicated hand-written TypeScript stripping and JSX lowering paths in both the Rust
  bundler and `runtime/compiler.mjs` with Oxc 0.139.0 transformers
- Preserved the existing resolver, graph cache, plugin ordering, linker, module metadata, client
  boundary validation, and public compile APIs while moving syntax transformation behind narrow Oxc
  adapters
- Added Oxc semantic analysis before Rust-side transformation so TypeScript enums, namespaces,
  `satisfies`, typed destructuring, JSX fragments, spread props, and namespaced JSX tags continue to
  compile through one parser-backed pipeline
- Kept classic React JSX output as the compatibility default and retained the automatic JSX runtime
  option without changing caller-facing compiler configuration
- Retained the Rust bundler's historical decorator behavior with a compatibility pre-pass, avoiding
  unresolved `@oxc-project/runtime` helper imports until helper-aware graph integration is
  introduced
- Removed Node's experimental `stripTypeScriptTypes` dependency and the custom runtime
  `JsxTransformer`; all Node renderers now reach the same Oxc-backed compiler entry points
- Pinned the Rust and npm transformer implementations to Oxc `0.139.0` and included native bindings
  for supported Windows, macOS, Linux, and WASI targets in the package lock
- Raised the framework, workspace, demo, and starter app Node requirement from `22.0.0` to `22.12.0`
  to match the native Oxc transformer runtime contract
- Improved resolver, compiler, and graph-cache reuse across multi-route builds
- Rebranded native bundler references to **Ruvyxa Bundler** across diagnostics, documentation, and
  package metadata

### Runtime and Developer Experience

- Improved worker-pool lifecycle and prerender reliability for production builds
- Added consistent millisecond-duration reporting for build and render phases
- Improved runtime worker-pool coordination for asynchronous route rendering
- Added clearer file I/O errors that include the missing source path, making dependency and package
  setup failures easier to diagnose
- Simplified the path-aware resolver read helper so strict workspace Clippy passes without the
  redundant enclosing `Ok(...)` and `?`, while preserving the original I/O error kind and path
- Updated compiler and worker-pool regression coverage for the new asynchronous execution model
- Expanded compiler parity coverage across Rust parser fixtures and the published Node runtime. Rust
  fixtures cover annotations, enums, decorators, fragments, spreads, and nested expressions; Node
  runtime tests cover enum and namespace lowering, TSX, CSS-in-JS objects, dynamic imports, cache
  invalidation, source maps, and paths containing spaces
- Added cross-platform project setup scripts:
  - `setup.bat` with the complete Windows setup workflow
  - `setup.sh` with the complete macOS/Linux setup workflow
- Setup now installs locked workspace dependencies, builds all npm workspace packages, and compiles
  the Ruvyxa CLI before development, without depending on a shared `setup.mjs` launcher

### Release and Documentation

- Bumped workspace packages and Rust crates to `1.0.14`
- Updated English and Thai CLI documentation for shared bundling, async builds, and prerender
  parallelism
- Updated English and Thai configuration documentation for the new build behavior
- Updated bundler architecture, developer, package, and production-readiness documentation
- Documented the Oxc ownership boundary, decorator compatibility strategy, source-map follow-up, and
  native Node version requirement
- Added and updated compiler, parser compatibility, shared bundling, and worker-pool regression
  coverage

## v1.0.13 (2026-07-14)

### Runtime Path Compatibility

- Fixed runtime alias resolution when Ruvyxa is installed in a path containing spaces or other
  URL-encoded characters by using `fileURLToPath()` across standalone renderers, the worker pool,
  and the runtime compiler
- Added regression coverage that loads the runtime compiler from a temporary path containing spaces

### Server Reliability and Forwarded-Header Security

- Fixed server startup so action endpoints receive Axum TCP connection metadata instead of failing
  `ConnectInfo` extraction at runtime
- Restricted forwarded client and protocol headers to loopback or explicitly configured
  `security.trustedProxyIps`, preventing private-network clients from bypassing action rate limits

### Release Metadata and Templates

- Bumped all npm packages and Rust crates to `1.0.13`
- Updated both minimal starter template copies to require `ruvyxa` and `@ruvyxa/react` `^1.0.13`

## v1.0.12 (2026-07-13)

### Oxc Integration and Modernized Minification

- Integrated the Oxc 0.139.0 ecosystem (parser, semantic minifier, mangler, code generator) for
  production JavaScript minification, replacing the selective token compressor
- Oxc owns final parsing, semantic compression, name mangling, and minified code generation while
  Ruvyxa retains framework-specific resolution, linking, boundary checks, and output composition
- `build.treeShaking` keeps its public meaning: enabled uses Oxc full compression; disabled uses
  `CompressOptions::safest()` to preserve unused bindings
- Removed the old selective token compressor from production code paths; `minify_parallel` now
  delegates to a single whole-program Oxc pass since semantic mangling cannot be performed per
  linker segment
- Removed unused `compile_graph`, `CompilerError`, `compile_graph_resilient`, and
  `parse_error_location` utilities, simplifying the compiler public API
- Added `base64-simd`, `compact_str`, and `num-bigint` for performance-critical operations in the
  bundler pipeline
- Published `docs/architecture/bundler-modernization.md` documenting the oxc integration boundary,
  adoption map, and next safe stages

### Linker and Resolver Improvements

- Added CommonJS `module.exports` / `__exports` / `process.env` shims for compatibility with
  CommonJS bundles expecting Node.js globals; `process.env` stubs default to production
- Implemented tsconfig/jsconfig content fingerprinting and cached path resolution to avoid repeated
  I/O and parsing across multi-file builds; invalidates cached configuration on file modification
- Added support for `import Default, * as ns` import clause combinations
- Introduced `is_identifier()` utility for robust identifier validation in import clause parsing
- Converted the linker's `rewrite_module_into`, `try_rewrite_import`, and `rewrite_import_clause` to
  return `Result` types for consistent error propagation through `link_inner` and `link_parallel`
- Skipped dynamic chunk generation when `emit_chunk_manifest` is disabled to avoid unnecessary
  processing

### Packaging

- Bumped all npm packages and Rust crates from 1.0.11 to 1.0.12
- Applied consistent compact array syntax to `files`, `keywords`, `os`, and `cpu` fields across all
  platform CLI packages, adapter packages, core, react, and create-ruvyxa package manifests

### Security: Wasm Plugin Response Buffering Limits

- Added `security.pluginLimit` configuration option defaulting to 32 MiB (max 256 MiB) to control
  response-phase Wasm plugin body buffering, preventing unbounded memory growth
- Introduced `MAX_PLUGIN_RESPONSE_BODY_LIMIT_BYTES` constant and RUV1602 diagnostic for invalid
  limits; zero and beyond-maximum values are rejected at config load
- Propagated `plugin_response_body_limit_bytes` through `ServerConfig` into both dev and production
  server paths, applying the limit at the Axum body extraction layer
- Updated user guide with plugin buffering limits, memory considerations, and configuration examples
- Added validation tests for zero, within-range, at-maximum, and over-maximum limit values

### Developer Experience: Pre-commit Hook

- Added `.githooks/pre-commit` hook that runs `format-staged.mjs` before every commit, verifying
  Prettier formatting for staged JS/TS/JSON/MD files and `cargo fmt --check` for staged Rust files
- Created `scripts/format-staged.mjs` to detect changed files, run the appropriate formatter, and
  block commits that would fail CI formatting checks
- Added `scripts/setup-git-hooks.mjs` and a `prepare` lifecycle script so hooks activate
  automatically on `pnpm install`
- Added `format:staged` package script for manual on-demand staged-file formatting
- Updated `CONTRIBUTING.md` to document the pre-commit hook behaviour

### Documentation: User Guide Restructuring

- Replaced the single `docs/user-guide.md` (517 lines) with an organized `docs/guides/` directory
  containing 12 focused chapters per language
- Added complete **English** guides: getting started, routing, server/client components, API routes,
  data loading and cache, server actions, rendering strategies, markdown/MDX/images, environment
  variables, configuration reference, CLI commands, and deployment
- Added complete **Thai** (ภาษาไทย) translations alongside every English chapter under
  `docs/guides/th/`
- Created `docs/guides/index.md` with a bilingual table of contents, language selector, and quick
  navigation section for application authors
- Updated `README.md` Documentation section with a linked table pointing to all four doc resources
  (User Guide, Developer Guide, Bundler Modernization, Production Readiness) and moved it higher for
  visibility
- Updated `developer-guide.md` links to point to the new guide index
- Updated documentation to reflect current system defaults: added `preview` and `bench` CLI
  commands, `parity` alias, `pluginLimit` security option, `plugins` and `middleware` config fields,
  explicit Rust 1.96+ requirement, and correct `middleware.builtin.log` / `middleware.builtin.rate`
  field names

## v1.0.11 (2026-07-12)

### macOS x64 Native Binary Removal

- Removed `@ruvyxa/cli-darwin-x64` package directory and configuration
- Removed `darwin-x64` from `supportedPlatforms` mapping in `scripts/native-platform.mjs`
- Removed `@ruvyxa/cli-darwin-x64` optional dependency from main package
- Updated error message in `bin/ruvyxa.js` to reflect remaining 5 supported platforms
- Added test case verifying `darwin-x64` is not published or resolved
- Intel macOS support discontinued in favor of ARM64 architecture

### Production Minification and CSS Optimization

- Replaced the third-party minification bypass with token-aware compression for the complete client
  bundle, including `node_modules`
- Preserved regular expressions, strings, template literals, legal comments, and JavaScript
  automatic-semicolon-insertion boundaries during compression
- Folded CommonJS `process.env.NODE_ENV` guards while resolving production client dependencies so
  React and similar packages include production implementations without development branches
- Updated module labeling in linker to use full paths consistently
- Added CSS minification support with `minify_css()` in dev server for production builds while
  preserving readable CSS in watch mode
- CSS minifier strips comments and collapses whitespace, preserving string/`url()` content

### Rate Limit Bypass Prevention and Worker Reliability

- Extracted peer socket address in action endpoint to capture direct client IP
- Implemented trusted proxy detection to prevent `X-Forwarded-For` spoofing attacks
- Only trust forwarded headers when direct peer is loopback or private address
- Added idempotent request detection to safely retry only SSR, SSG, and client requests
- Quarantined failed workers to prevent processing conflicting retry requests
- Added stderr drain task to prevent Node worker process pipe buffer overflow
- Implemented sliding-window rate limiter middleware with per-client IP tracking
- Improved worker pool fallback messaging to clarify idempotent request retry logic

### Documentation Consolidation

- Reorganized docs structure into two main guides: `docs/user-guide.md` for app developers and
  `docs/developer-guide.md` for framework contributors
- Deleted specialized docs (getting-started, routing, content-and-images, data, actions, deployment,
  debugging, performance, parity, production-readiness, publishing, architecture/project-structure)
- Updated README.md documentation links to point to the two new consolidated guides
- Added demo app README with health check example
- Updated CONTRIBUTING.md to reference new documentation structure
- Simplified documentation maintenance by centralizing content into purpose-specific guides

### Smoke Test and Script Improvements

- Isolated scaffolded app workspace context in smoke tests by creating empty `pnpm-workspace.yaml`
- Overrode smoke test dependencies with local tarballs for comprehensive validation
- Added pnpm overrides for transitive dependency resolution during smoke tests
- Added tarball resolution for `@ruvyxa/core`, `@ruvyxa/react`, and platform-specific CLI packages
- Improved smoke test isolation by using system temp directory instead of hardcoded path
- Removed redundant `ruvyxa` type declaration from minimal template `tsconfig.json`
- Simplified type resolution by relying on `ruvyxa` package's included types

### Infrastructure

- Removed `.githooks/pre-commit` hook for Cargo.lock validation (now handled through CI/CD)
- Suppressed clippy `too_many_arguments` warning on `print_build_report` function

### Windows arm64 Support

- Added `@ruvyxa/cli-win32-arm64` platform package with native CLI binary for Windows arm64
- Extended supported platform mapping in `scripts/native-platform.mjs` to include `win32-arm64`
- Updated `nativeBinaryPackageName()` — all supported platforms are now resolved through a shared
  data module instead of a hardcoded switch
- Added Windows arm64 to the CI build matrix (`.github/workflows/ci.yml`,
  `.github/workflows/release.yml`)
- Updated binary resolution in `bin/ruvyxa.js` to display `win32-arm64` in the supported-platforms
  message and route to the new optional package
- Added `@ruvyxa/cli-win32-arm64` as a dependency in `ruvyxa/package.json`
- Added native platform test suite (`native-platform.test.mjs`) verifying the mapping, package
  metadata, and unsupported-platform fallback

### Security Configuration

- Added `security.apiLimit` configuration for maximum API route request payload size (default: 10 MB
  / 10,485,760 bytes)
- Added `security.actionRateLimit` with `max` (default: 600) and `window` (default: 60s) for
  configurable per-client/action rate limiting
- Raised default `actionLimit` from 64 KB to 1 MB (1,048,576 bytes)
- Raised default action rate limiter from 60 req/min to 600 req/min
- Added `RUV1601` config validation for zero-valued security limits (`actionLimit`, `apiLimit`,
  `actionRateLimit.max`, `actionRateLimit.window`)
- Added strict unknown-field rejection for `config.security.actionRateLimit`
- Extended TypeScript types in `@ruvyxa/core` with `apiLimit` and `actionRateLimit` fields
- Forwarded new security config fields through runtime config renderer (`config-renderer.mjs`) and
  into production `build.json` output
- Updated security section in all documentation to reflect new keys and defaults

### Server and Worker Pool Lifecycle

- **Graceful server shutdown** — intercepts SIGTERM / Ctrl+C, notifies workers, and terminates with
  a 5-second grace period before force-closing remaining connections
- **Worker pool shutdown** — added `NodeWorkerPool::shutdown()` that closes stdin on every worker,
  clears pending requests, and force-terminates workers that do not exit within 2 seconds
- Worker stdin access now uses a `Mutex<Option<mpsc::Sender>>` so senders are safely drained during
  shutdown; operations after shutdown return a clear `"Worker process is shutting down"` error
- Worker `_child` made accessible via `Mutex<Option<Child>>` to support `kill` + `wait` on shutdown
- HMR client script simplified — now always issues `location.reload()` for every update, eliminating
  the fragile targeted CSS/component refresh code path
- Security headers no longer inject `Connection: keep-alive` / `Keep-Alive: timeout=30, max=1000`
  into every response; WebSocket `Connection: Upgrade` headers are preserved

### Config Validation and CLI

- Added `validate_positive_limit()` helper raising `RUV1601` for zero-valued numeric limits
- Added Rust tests for zero-limit rejection on `apiLimit` and `actionRateLimit`
- Updated existing security config tests to verify new `apiLimit` / `actionRateLimit` fields
- `config()` shorthand key table in getting-started docs updated with `apiLimit` and
  `actionRateLimit`

### Compiler and Runtime

- Runtime compiler (`compiler.mjs`) now rewrites named `export class` declarations before wrapping
  modules, making class exports available after module wrapping
- Added compiler test for named class export rewriting with runtime verification

### create-ruvyxa

- Scaffolded projects now receive their own `package.json#name` derived from the target directory
  name (sanitized to a portable npm package name)
- Added `toPackageName()` and `writeProjectPackageName()` helpers in `create-ruvyxa/src/index.ts`
- Added test coverage for package-name derivation and output verification

### CI and Infrastructure

- Added Ubuntu 24.04 ARM64 to the CI and release build matrix
- All npm packages, Rust crates, lockfiles, and template dependencies synchronized

### Documentation

- Documented `security.apiLimit` and `security.actionRateLimit` config keys across all guides
- Updated security defaults (1 MB action limit, 10 MB API limit, 600 req/min rate limit) in actions,
  deployment, production-readiness, and publishing docs
- Added `@ruvyxa/cli-win32-arm64` to native binary platform tables in production-readiness,
  publishing, deployment, and project-structure documentation
- Updated CI/CD documentation to reflect Windows arm64 and Ubuntu ARM64 build runners
- Updated build metadata example in deployment docs with new security fields
- All concise config key tables reflect the current configuration contract
- Version and dependency references updated across the documentation set

## v1.0.10 (2026-07-11)

### Content, Images, and SEO

- Added first-class `page.md` and `page.mdx` routes with frontmatter, heading metadata, GFM
  Markdown, MDX ESM imports, JSX components, expressions, SSG, and HMR support
- Shared content compilation across Ruvyxa Bundler and Node runtime compiler, including
  content-aware dependency scanning that ignores imports inside fenced code examples
- Added `frontmatter`, `meta`, `headings`, and `contentFormat` exports to generated content modules
- Rebuilt image optimization around a single-output `.webp` pipeline that replaces local PNG/JPEG
  asset extensions instead of generating AVIF/WebP sidecars beside the original files
- Optimized public assets in one parallel pass with persistent content caching, direct cache reuse,
  collision detection, and unchanged fallback copies for invalid or non-image files
- Simplified development and production image serving so `.webp` assets resolve directly, while
  legacy local PNG/JPEG requests can still map to the optimized `.webp` output where applicable
- Added compact image manifest output with source/output paths, dimensions, byte sizes, source
  bytes, output bytes, optimized image counts, and cache hit tracking
- Updated typed image configuration to `image.optimize`, `image.quality`, `image.lossless`, and
  `image.workers`
- Upgraded `@ruvyxa/react` images with local-only `.webp` rewriting, `fill`, author-managed
  `srcSet`, browser-native `Picture` art direction, loading controls, and per-image CDN loaders
  without adding runtime image transformation

### Hashing and Build

- Upgraded asset hashing from BLAKE3-64 to BLAKE3-256: `content_hash()` now returns the full
  64-character hex output instead of a truncated 16-character value; `ASSET_HASH_ALGORITHM` constant
  changed from `"blake3-64"` to `"blake3-256"`
- Updated `build.json` hash algorithm output and documentation to reflect 256-bit hashing
- Client bundle file names now use full BLAKE3-256 content hashes for stronger cache uniqueness

### CLI and Config

- Replaced `defineConfig()` with `config()` and adopted concise configuration keys across the public
  contract; `appDir` and `outDir` remain unchanged
- Added `debug.traces` configuration option for debug trace control in the dev server
- Added `deny_unknown_fields` to `ProjectConfig` and `DebugConfigOptions` for strict config
  validation against unknown keys
- Added strict top-level config validation for `runtime`, `react`, `typescript`, `render`, `image`,
  `security`, `cache`, `middleware`, `adapter`, `adapterOptions`, and `plugins`
- Implemented `normalize_source_path()` to gracefully handle non-existent paths in HMR tracking
- Fixed Windows watcher paths prefixed with `.` so generated `.ruvyxa` cache writes are ignored
  instead of triggering repeated reloads; condensed dev startup and HMR logs into readable summaries
- Added concise dev document-request logs with method, route, response status, and sub-millisecond
  timing while excluding HMR and static asset traffic
- Updated worker pool and config renderer with improved runtime implementations
- Added tests for asset hash algorithm, dev config overlay/trace flags, unknown field rejection, and
  HMR tracker path normalization

### Branding and Error Page

- Centralized the framework logo at `assets/branding/ruvyxa.png` as the canonical source
- Added `assets/branding/README.md` documenting synchronization of runtime copies across starters
  and the error page
- Refined the plain error page into a centered 404/500 recovery layout with logo, status code,
  title, and escaped diagnostics on a dark outer background with white card and purple accent

### Infrastructure

- Added `.githooks/pre-commit` hook validating `Cargo.lock` synchronization before commits
- Added `scripts/check-cargo-lock.mjs` script and `check:cargo-lock` npm script for manual
  validation
- Upgraded Rust workspace from edition 2021 to 2024 and resolver from "2" to "3"
- Applied `cargo fmt` with Rust 2024 formatting rules across all crates
- Upgraded Rust dependencies: cranelift 0.132.2→0.133.1, tower-http 0.6.11→0.7.0, pulley
  45.0.2→46.0.1, mach2 0.4.3→0.6.0, wasm-compose/encoder/parser to 0.251.0
- Upgraded bytes 1.11.1→1.12.0, cc 1.2.64→1.2.65, log 0.4.32→0.4.33, quote 1.0.45→1.0.46
- Upgraded Node.js package versions across all workspace packages and regenerated lockfiles

### Diagnostic Codes

- Added `RUV1101` SSR renderer args missing diagnostic
- Added `RUV1550` PPR (Partial Prerendering) render failed diagnostic
- Added `RUV1801` Module resolution error diagnostic
- Added Partial Prerendering (PPR) error code section to diagnostics guide
- Refined error code table formatting and alignment for readability

### Testing

- Added `worker-pool.test.mjs` test suite for worker pool behavior
- Expanded compiler tests with content compilation, fenced-import handling, and image configuration
  coverage
- Added tests for React metadata, route discovery, dev/prod parity, error-page escaping and layout
- Added regression coverage for the new single-output `.webp` optimizer, cache reuse, collision
  rejection, invalid image fallback, disabled optimization, and dev server `.webp` source resolution
- All existing test suites updated and passing

## v1.0.9 (2026-07-10)

### Client Bundling and Boundaries

- Bundled browser React and React DOM dependencies, including CommonJS package dependencies, so
  client hydration no longer leaves unresolved bare `react` module specifiers
- Preserved valid third-party JavaScript, including regular-expression literals, when the native
  text minifier cannot safely parse the dependency source
- Made server/client boundary diagnostics syntax-aware so ordinary content containing `server-only`
  is not treated as a module marker
- Ignored type-only imports during runtime dependency resolution

### Build Reliability

- Capped default and configured static pre-render concurrency at two workers to prevent memory
  exhaustion on content-heavy sites
- Added Windows rename retries for transient file locks while committing build output
- Fixed file-watcher cache invalidation on threads without a Tokio runtime

### Starter and Documentation

- Added the CSS module declaration required by the minimal TypeScript starter
- Synchronized all npm packages, Rust crates, lockfiles, and template dependencies to 1.0.9
- Added regression coverage for client dependency bundling, boundary markers, Windows-safe build
  commits, pre-render limits, watcher invalidation, and starter generation

## v1.0.8 (2026-07-10)

### Performance and Build

- Parallelized build-time prerendering for CSR, SSG, ISR, and PPR routes while preserving manifest
  order
- Reused the configured build parallelism for prerender work to reduce production build time
- Kept client bundling parallelism capped to available work to avoid oversubscription
- Reduced the demo production build benchmark from about 2.3s to about 1.1s

### Styling

- Collected CSS through the application dependency graph, including styles imported from outside
  `app/` and nested local CSS `@import` dependencies
- Added project-relative `css.entries` for unimported global style files and directories
- Preserved runtime CSS-in-JS style objects and `<style>` elements, with external style HMR and
  production-copy coverage
- Added actionable diagnostics for unresolved styles, unsafe entries, and preprocessors without a
  transform plugin

## v1.0.7 (2026-07-10)

### Performance and Bundling

- Reused one persistent Node worker for JavaScript config plugin hooks during each build
- Forwarded plugin transform Source Map v3 data into generated client bundle maps
- Added route-scoped shared chunk metadata and `modulepreload` hints to runtime and pre-rendered
  HTML
- Ensured pre-rendered SSG, ISR, PPR, and CSR output loads hashed hydration assets from the client
  manifest
- Added fixture-driven advanced TypeScript/JSX parser coverage and fixed multiline enums,
  `implements`, and namespaced JSX tags
- Invalidated native compile artifacts when imported config/plugin dependencies change
- Added shared build-cache directories via `cache.dir` or `RUVYXA_BUILD_CACHE_DIR`
- Pre-bundled dev route dependencies in background across every persistent Node worker
- Added consistent client directory and chunk-manifest references to every deployment adapter

## v1.0.6 (2026-07-09)

### Highlights

- SSG, ISR, and PPR pre-rendering support added to the rendering pipeline
- New runtime SSG renderer for server-side page pre-rendering at build time
- CSR minimal shell HTML generation for client-side rendered pages
- Revalidation metadata tracking for ISR routes
- Dev server and build output updated with prerendered routes manifest
- Demo examples demonstrating SSG, ISR, PPR, and CSR rendering strategies
- Codebase-wide formatting standardization with Prettier configuration
- `render_api` refactored to use structured request object for improved maintainability
- Documentation overhaul across all guides (rendering strategies, cache, security, middleware)
- pnpm requirement upgraded from 10+ to 11+

### Rust Crates

- **ruvyxa_cli**:
  - SSG/ISR/PPR pre-rendering at build time with dynamic route support
  - `getStaticParams` resolution for dynamic routes during build
  - Build output includes prerendered routes manifest and prerender stats
  - Code formatting improvements
- **ruvyxa_dev_server**:
  - Prerender directory support in dev server and production configs
  - `render_api` refactored to accept `RenderApiRequest` struct instead of multiple params
  - Reduced parameter passing complexity and improved type safety
  - Worker pool and router enhancements
- **ruvyxa_graph**:
  - Route manifest generation updates for prerendering
  - Enhanced route discovery

### npm Packages

- All packages updated with version bumps
- **@ruvyxa/core**: Added `RenderStrategy` enum and rendering configuration to types
- **ruvyxa/runtime**:
  - New `ssg-renderer.mjs` for server-side page rendering
  - `worker-pool.mjs` modernized with improved concurrent request handling
  - All runtime modules formatted to new Prettier standards
- All adapter packages updated with `tsconfig.check.json` and formatting
- All CLI binary packages updated

### Examples

- **demo**:
  - New SSG blog with `[slug]` dynamic routes (`app/ssg-blog/`)
  - New ISR page with revalidation (`app/isr-page/`)
  - New PPR page with partial pre-rendering (`app/ppr-page/`)
  - New CSR page with client-side rendering (`app/static-page/`)
  - Static page example
  - Updated layout, routing, and configurations

### Documentation

- Updated README with rendering strategies, pnpm 11+ requirement, expanded crate descriptions
- Updated CONTRIBUTING with correct Rust verification flags and adapter guidelines
- Enhanced `docs/architecture/project-structure.md` with crate capabilities and features
- Updated `docs/routing.md`, `docs/data.md`, `docs/actions.md` with rendering strategy details
- Revamped `docs/debugging.md`, `docs/deployment.md`, `docs/performance.md`
- Expanded `docs/production-readiness.md` with cache configuration and security
- Improved `docs/publishing.md` and `docs/parity.md`
- `docs/testing.md` updated with API renderer test documentation

### Infrastructure

- Added `.prettierrc` and `.prettierignore` for consistent code formatting
- pnpm requirement changed from `^10.32.1` to `^11.7.0`
- Package metadata validation uses dynamic license from root `package.json`
- All `package.json` files updated with version and dependency sync
- TypeScript config check files added to adapter packages
- GitHub Actions workflows updated for formatting consistency

### Testing

- New `api-renderer.test.mjs` test suite for API rendering
- Updated `compiler.test.mjs`, `client-renderer.test.mjs`, `action-renderer.test.mjs`
- Updated adapter tests for all 6 deployment targets
- Updated core config and server tests
- `test-full-flow.ps1` updated with expanded coverage

## v1.0.5 (2026-07-09)

### Highlights

- Full Ruvyxa Bundler pipeline with AST parsing, plugin system, chunking, and tree-shaking
- New `demo` example app replacing `basic-app`
- Comprehensive end-to-end test script (`test-full-flow.ps1`)
- Project structure and engineering backlog documentation
- README logo switched to local asset for reliability

### Rust Crates

- **ruvyxa_bundler**: Major feature expansion
  - AST module (`ast.rs`) for structured parsing of imports, exports, JSX, decorators, TypeScript
  - Plugin system (`plugin.rs`) for custom transformations in the bundler pipeline
  - Chunking module (`chunking.rs`) for dynamic import split points and output chunk generation
  - Context module (`context.rs`) for shared bundler execution state across parallel workers
  - Types module (`types.rs`) with core bundler type definitions
  - Tree-shaking as separate step before minification (`treeShake` build option)
  - Cache hit tracking via `cache_hit` field on `CompiledModule`
  - Plugin-runner module for runtime plugin execution
  - Resolver enhancements: CommonJS `require()`, dynamic `import()`, improved caching
  - Source map improvements
  - Parallel cache reuse across bundle jobs
- **ruvyxa_cli**: Integrated new bundler components, expanded CLI commands
- **ruvyxa_dev_server**: Render cache improvements, HMR tracker updates, worker pool enhancements
- **ruvyxa_middleware**: WASM plugin system improvements
- **ruvyxa_graph**: Graph module updates
- **ruvyxa_diagnostics**: Diagnostic enhancements

### npm Packages

- All packages bumped to v1.0.5
- **@ruvyxa/core**: Added `utils.ts`, `PluginContext` and `TransformResult` exports, config updates
- **@ruvyxa/react**: Package updates
- **ruvyxa/runtime**: Added `plugin-runner.mjs`, `config-renderer.mjs` enhancements, `compiler.mjs`
  updates
- **adapters**: All 6 adapter packages updated with platform info and README improvements
- **CLI platform binaries**: All 5 platform packages updated
- **create-ruvyxa**: Updates

### Examples

- Replaced `basic-app` with comprehensive `demo` example
  - Multiple route patterns (static, dynamic `[slug]`, catchall `[...slug]`)
  - Todos with server actions and in-memory DB
  - Blog routes, environment variables page
  - Full TypeScript + Tailwind CSS setup
  - AGENTS.md and CLAUDE.md for AI-assisted development

### Documentation

- Added `docs/architecture/project-structure.md`
- Added `docs/roadmap/engineering-backlog.md`
- Updated debugging, deployment, parity, performance, production-readiness docs
- Bundler comparison documentation (`bundler-comparison.md`)
- README refreshed with new logo, features, and bundler comparison link

### Testing

- New `scripts/test-full-flow.ps1` end-to-end test script
- Expanded compiler test coverage
- Integration tests for all adapter packages
- AST parsing tests across import forms

### Infrastructure

- Removed obsolete `basic-app` example
- Cleaned up old design spec documents
- CLAUDE.md and AGENTS.md updated

---

## v1.0.4 (2026-07-09)

### Highlights

- `ruvyxa check` command for pre-deploy verification
- Type checking, build validation, dev/prod parity, and page smoke rendering
- Plugin contract documentation and type exports
- Simplified template structure (removed `.env.example`, consolidated CSS)

### Rust Crates

- **ruvyxa_cli**: Added `CheckArgs` and `ruvyxa check` command
  - Runs type checking, build validation, dev/prod parity, smoke rendering
- **ruvyxa_dev_server**: Exported `render_request` for programmatic rendering in `check` command
- **ruvyxa_graph**: Route discovery and manifest generation updates
- All crates bumped to v1.0.4

### npm Packages

- All packages bumped to v1.0.4
- **@ruvyxa/core**:
  - Exported `PluginContext` and `TransformResult` types
  - Added comprehensive Plugin Contract section to README
  - New `types.ts` for shared type definitions
  - `config.ts` improvements
- **ruvyxa**: Type exports synced, removed unused CSS module declarations
- **create-ruvyxa**: Package manager detection utility
- All adapters and CLI binaries updated

### Examples & Templates

- **basic-app**: Removed `.env.example`, simplified
- **minimal template**:
  - Removed todos example and about page
  - Consolidated `global.css` → `globals.css`
  - Removed legacy API examples
  - Updated TypeScript configuration

### Documentation

- Plugin Contract documentation with example implementation
- `check` positioned as primary verification command before `build`
- SKILL.md and app-guide.md updated to recommend `pnpm check`
- `test:parity` command description clarified
- Updated routing, data, debugging, deployment, getting-started docs
- Removed outdated `Plan.md`

### Testing

- `core/config.test.ts` test suite for configuration validation
- Expanded compiler test coverage
- create-ruvyxa test suite enhancements
- Verification guidance: use `check` for routine changes, `parity`/`analyze` for drill-down

---

## v1.0.3 (2026-07-08)

### Highlights

- Incremental caching and HMR tracking for bundler and dev server
- Centralized test directory structure (`tests/`)
- First-class adapter packages with dedicated tests
- React utilities: error boundary, hydration, `useLoader` hook

### Rust Crates

- **ruvyxa_bundler**:
  - Incremental graph cache with `incremental.rs` (file fingerprinting via blake3)
  - `CachedModuleEntry` and `GraphManifest` for persistent dependency storage
  - Fast-reject optimization (mtime/size check before hashing)
  - Cache hit tracking improvements
  - Compiler enhancements: TypeScript annotation stripping fixes, JSX child detection
  - All crates bumped to v1.0.3
- **ruvyxa_cli**: Path validation in `ProjectConfig`, command additions
- **ruvyxa_dev_server**:
  - HMR tracker module (`hmr_tracker.rs`)
  - Module invalidation tracking and dependency relationship mapping
  - Render cache expiry and validation improvements
  - Worker pool task scheduling and error handling enhancements
- **ruvyxa_middleware**: Updates
- **ruvyxa_graph**: Updates
- **ruvyxa_diagnostics**: Updates

### npm Packages

- All packages bumped to v1.0.3
- **@ruvyxa/react**:
  - Error boundary component
  - Hydration utilities for React client initialization
  - `useLoader` hook for data loading patterns
- **@ruvyxa/core**: Server implementation improvements
- **ruvyxa/runtime**:
  - Full runtime compiler (`compiler.mjs`)
  - SSR renderer, action renderer, client renderer, API renderer improvements
  - Worker pool enhancements
- **create-ruvyxa**: Package manager detection utility
- All adapters and CLI binaries updated

### Testing

- Reorganized tests from package-local to centralized `tests/` directory
- Dedicated test files for each adapter: Bun, Cloudflare, Netlify, Node, Static, Vercel
- `tests/packages/core/server.test.ts`
- `tests/packages/ruvyxa/compiler.test.mjs`
- `tests/packages/ruvyxa/action-renderer.test.mjs`
- `tests/packages/ruvyxa/client-renderer.test.mjs`
- vitest configuration
- Adapter test coverage for all deployment targets

### Documentation

- `docs/testing.md` with testing layout guidance
- Updated debugging, performance, production-readiness, publishing docs
- README logo enlarged
- CLI platform binary READMEs

### Infrastructure

- Dashmap, memmap2, parking_lot dependencies for concurrent caching
- Clap bumped to 4.6
- Dependency updates

---

## v1.0.2 (2026-06-18)

### Highlights

- First release of `ruvyxa_bundler` — Ruvyxa Bundler
- `ruvyxa_middleware` crate with WASM plugin support
- Compression, caching, and worker pool in dev server
- Upgraded toolchain: Node.js 22, Rust 1.96, pnpm 10

### Rust Crates

- **ruvyxa_bundler** (new crate):
  - Ruvyxa Bundler TypeScript/JSX compiler pipeline
  - Boundary checker for server/client module isolation
  - Caching layer with blake3 hashing for incremental builds
  - AST transformation and code generation
  - Linker for module resolution and bundle generation
  - Minifier for production bundle optimization
  - Source map generation
  - Module path resolution and dependency tracking
  - Output formatter with bundle metadata
- **ruvyxa_middleware** (new crate):
  - Builtin middleware implementations
  - Config-driven middleware stack
  - WASM plugin system for custom middleware
- **ruvyxa_dev_server**:
  - HTTP compression (gzip + brotli) via tower-http
  - Render cache system with blake3 hashing
  - Radix router for efficient route matching
  - Node.js worker pool for concurrent request handling
- **ruvyxa_cli**: Integrated bundler, middleware, expanded CLI
- All crates bumped to v1.0.2

### npm Packages

- All packages bumped to v1.0.2
- **@ruvyxa/core**: Server refactoring, config improvements, type safety
- **@ruvyxa/react**: Package initialized with exports
- **ruvyxa/runtime**:
  - `config-renderer.mjs` for runtime configuration
  - `worker-pool.mjs` for Node.js worker management
- **CLI platform binaries**: All 5 platform packages with native binaries
- **create-ruvyxa**: Enhanced CLI with platform detection
- **@ruvyxa/adapter-***: All 6 adapters with README and teamplate support

### CI/CD

- GitHub Actions CI and release workflows
- Cargo caching (Swatinem/rust-cache@v2)
- Smart publishing via `publish-if-new.mjs`
- Version resolution and tag management
- Cross-platform npm pack smoke testing
- Exponential backoff retry for cargo fetch and pnpm install
- `bump-version.mjs` script
- `--locked` flag for reproducible builds

### Toolchain

- Node.js upgraded from 20 to 22
- Rust toolchain upgraded from 1.80+ to 1.96+
- pnpm upgraded to v10
- TypeScript 6.0
- pnpm action v5

### Templates

- **minimal template**: Complete Ruvyxa starter with routing, API, todos, Tailwind CSS
- AGENTS.md and CLAUDE.md for AI-assisted development
- `config-renderer.mjs` integration
- TypeScript configuration with path aliases

### Documentation

- Comprehensive README with hero section, features, CLI commands
- Full CONTRIBUTING.md with development setup, conventions, workflows
- Getting started, routing, data, actions, debugging, deployment, performance docs
- Production readiness and publishing guides
- Parity testing documentation
- Security documentation in SECURITY.md
- Skill framework documentation (SKILL.md, app-guide.md)

---

## v1.0.1 (2026-06-17)

### Highlights

- Cross-platform npm pack smoke testing
- HMR WebSocket payload optimization
- `create-ruvyxa` CLI UX improvements
- CI/CD reliability and version management

### Rust Crates

- All crates bumped to v1.0.1 (ruvyxa_cli, ruvyxa_dev_server, ruvyxa_diagnostics, ruvyxa_graph)
- **ruvyxa_dev_server**: Simplified HMR WebSocket handling using pre-serialized messages from
  channel

### npm Packages

- All packages bumped to v1.0.1
- **create-ruvyxa**:
  - Try-catch error handling with graceful error display
  - Formatted next steps after app creation (cd, pnpm install, pnpm dev)
  - Target directory validation (exists + empty check)
  - Clear error messages for non-empty directories
- **ruvyxa**: Release packaging scripts
- **@ruvyxa/adapter-***: All adapters updated
- **CLI platform binaries**: All platform packages updated

### CI/CD

- `resolve-version` job for version extraction and git tag validation
- Auto tag creation on `workflow_dispatch`
- Git tag existence check via `git ls-remote`
- Release summary in GitHub step summary
- `release:bump` script for syncing workspace versions
- Cross-platform npm pack smoke detection (dynamic tarball resolution)
- macOS 13 build target removal
- HMR error handling simplification

### Documentation

- `docs/publishing.md` with npm publishing guidelines
- Updated deployment docs
- Version reference updates across docs
- README version badges

### Infrastructure

- `scripts/validate-package-metadata.mjs`
- `scripts/pack-smoke.mjs` with dynamic tarball detection
- Platform-specific native binary preparation scripts

---

## v1.0.0 (2026-06-17)

### Highlights

- Initial production release of Ruvyxa
- Native Rust CLI with full-stack React framework
- Built-in development server and production server
- Route discovery and manifest generation
- Diagnostic system with error codes

### Rust Crates

- **ruvyxa_cli**:
  - CLI entry point with commands: `dev`, `build`, `start`, `preview`, `routes`, `analyze`,
    `doctor`, `clean`, `trace`, `bench`, `test:parity`
  - Project configuration and build pipeline
  - PID file management
- **ruvyxa_dev_server**:
  - Development server with HMR and WebSocket support
  - Production server with static file serving
  - Node.js runtime integration
- **ruvyxa_diagnostics**:
  - Diagnostic type system: warnings, errors, hints, tips
  - Structured diagnostic output
- **ruvyxa_graph**:
  - Route discovery from file system
  - Route manifest generation
  - Layout and page tree construction

### npm Packages

- **ruvyxa**: Main CLI wrapper package with native binary resolution
- **@ruvyxa/core**: Core framework with server, config, types, request/response handling
- **@ruvyxa/react**: React integration with SSR support
- **create-ruvyxa**: Project scaffolding CLI
- **@ruvyxa/adapter-bun**: Bun deployment adapter
- **@ruvyxa/adapter-cloudflare**: Cloudflare Workers deployment adapter
- **@ruvyxa/adapter-netlify**: Netlify deployment adapter
- **@ruvyxa/adapter-node**: Node.js deployment adapter
- **@ruvyxa/adapter-static**: Static site generation adapter
- **@ruvyxa/adapter-vercel**: Vercel deployment adapter
- **@ruvyxa/cli-darwin-arm64**: macOS ARM64 native binary
- **@ruvyxa/cli-linux-arm64**: Linux ARM64 native binary
- **@ruvyxa/cli-linux-x64**: Linux x64 native binary
- **@ruvyxa/cli-win32-x64**: Windows x64 native binary

### Runtime

- `ssr-renderer.mjs` — Server-side rendering
- `client-renderer.mjs` — Client hydration and rendering
- `action-renderer.mjs` — Server action handling
- `api-renderer.mjs` — API route handling
- `config-renderer.mjs` — Runtime configuration
- `worker-pool.mjs` — Worker pool management

### Examples

- **basic-app**: Starter application with:
  - Layout and page routing
  - About page
  - Blog with dynamic `[slug]` routes
  - Todos with server actions
  - Tailwind CSS styling
  - TypeScript configuration

### Templates

- **minimal template**: Minimal Ruvyxa starter
  - Single page with layout
  - Basic route structure
  - TypeScript + Tailwind CSS

### Documentation

- README.md with feature overview, getting started, examples
- CLI command documentation
- Architecture overview

### Infrastructure

- Rust workspace with 5 crates
- pnpm monorepo with 18 packages
- GitHub repository setup
- Prebuilt native CLI binaries for 5 platforms (Windows x64/ARM64, macOS ARM64, Linux x64/ARM64)
- npm publishing configuration
- TypeScript base configuration

---

## Pre-release History (unversioned)

The following commits occurred before the v1.0.0 tag and represent the initial project bootstrap:

| Date       | Description                                                                   |
| ---------- | ----------------------------------------------------------------------------- |
| 2026-06-17 | Initial project scaffold (`first commit`)                                     |
|            | Application structure with Tailwind CSS, todos page, navigation               |
|            | Security headers, performance benchmarks, deployment docs                     |
|            | Repository references updated, npm publishing documentation                   |
|            | Prebuilt native CLI binaries for multiple platforms                           |
|            | Adapter packages initialized (Bun, Cloudflare, Netlify, Node, Static, Vercel) |
|            | Foundational documentation and contributing guide                             |

---

## Release Tags

| Tag       | Date       | Type       |
| --------- | ---------- | ---------- |
| `v1.0.0`  | 2026-06-17 | Production |
| `v1.0.1`  | 2026-06-17 | Patch      |
| `v1.0.2`  | 2026-06-18 | Minor      |
| `v1.0.3`  | 2026-07-08 | Minor      |
| `v1.0.4`  | 2026-07-09 | Minor      |
| `v1.0.5`  | 2026-07-09 | Minor      |
| `v1.0.6`  | 2026-07-09 | Patch      |
| `v1.0.7`  | 2026-07-10 | Minor      |
| `v1.0.8`  | 2026-07-10 | Minor      |
| `v1.0.9`  | 2026-07-10 | Patch      |
| `v1.0.10` | 2026-07-11 | Minor      |
| `v1.0.11` | 2026-07-12 | Minor      |
| `v1.0.12` | 2026-07-13 | Minor      |
| `v1.0.13` | 2026-07-14 | Patch      |
| `v1.0.14` | 2026-07-16 | Minor      |
| `v1.0.15` | 2026-07-18 | Minor      |
| `v1.0.16` | 2026-07-20 | Minor      |
| `v1.0.17` | 2026-07-22 | Minor      |
| `v1.0.18` | 2026-07-22 | Patch      |
| `v1.0.19` | 2026-07-23 | Patch      |
| `v1.0.20` | 2026-07-24 | Minor      |
| `v1.0.21` | 2026-07-24 | Patch      |
