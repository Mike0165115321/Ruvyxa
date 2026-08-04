# Ruvyxa System Overview

**Philosophy**: Rust before render (route discovery, bundling, minification, serving). JS runtime
(Node/Bun) during render (SSR, SSG, API, config). This gives Rust speed + JS ecosystem.

```
┌──────────────────────────────────────────────────────────────────┐
│                        ruvyxa_cli                               │
│   (14 CLI commands · config loading · build orchestration)       │
├──────────┬───────────┬──────────────┬──────────┬────────────────┤
│ruvyxa_   │ruvyxa_    │ruvyxa_dev_   │ruvyxa_   │ruvyxa_         │
│graph     │bundler    │server        │middleware│diagnostics      │
│(route    │(TS/JSX    │(Axum + HMR + │(Tower    │(RUV#### codes)  │
│disc+val) │comp+link) │router+cache) │+host)    │                 │
└────┬─────┴─────┬─────┴──────┬───────┴────┬─────┴────────┬───────┘
     │           │            │            │              │
     └───────────┴────────────┴────────────┴──────────────┘
                               │
                    ┌──────────▼──────────┐
                    │  Node / Bun Workers  │
                    │  (SSR, SSG, API,     │
                    │   Action, Config)    │
                    └─────────────────────┘
```

---

## Crate Dependency Graph

```
ruvyxa_diagnostics          (serde + thiserror — nothing else)
    ↑
    ├── ruvyxa_graph        (route discovery, validation, manifest)
    ├── ruvyxa_bundler      (Oxc compiler, resolver, linker, minifier, CSS modules)
    ├── ruvyxa_middleware   (Tower middleware, plugin bridge)
    └── ruvyxa_dev_server   (Axum serving, HMR, cache, worker pool, router)
         │
         └── ruvyxa_cli     (depends ALL crates — binary entry via clap)
```

---

## Key Design Decisions

| Decision                               | Why                                                                                                                   |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| **Rust core, Node/Bun render**         | Rust owns discovery/build orchestration; persistent workers handle JS rendering without per-request process creation. |
| **Oxc for TS/JSX**                     | Oxc provides the repository's parser/compiler/minifier pipeline. Performance must be measured for the target project. |
| **Persistent worker pool**             | Server workers are bounded to the available parallelism range (2–8 by default). NDJSON over stdin/stdout.             |
| **Radix trie router**                  | O(path_depth) vs O(n) linear scan. Recompiled on manifest change.                                                     |
| **Blake3 content hashing**             | Immutable caching (max-age=31536000).                                                                                 |
| **Staging + atomic commit**            | Build writes to staging and commits by rename, restoring the previous output if the commit fails.                     |
| **fnv1a_64 deterministic CSS scoping** | Reproducible builds: `fnv1a_64(project_relative_path + class_name)`.                                                  |
| **`deny_unknown_fields` config**       | Typos fail fast, not silently ignored.                                                                                |

---

## Rendering Strategy Decision Tree

```
Your page is...
│
├── Browser-only (game, editor, canvas)?
│   └── → CSR  (add 'use client')
│
├── Mostly static, a few slow parts?
│   └── → PPR  (export const ppr = true + <Suspense>)
│
├── Changes every few minutes?
│   └── → ISR  (export const revalidate = 60)
│
├── Dynamic paths known at build time?
│   └── → SSG  (export getStaticParams)
│
├── Same for everyone, rarely changes?
│   └── → SSG  (auto-detected — do nothing)
│
└── Fresh data per request?
    └── → SSR  (default — do nothing)
```

**Detection priority** (first match wins):

1. `'use client'` → CSR
2. `export const ppr = true` → PPR
3. `export const revalidate = <n>` → ISR
4. `getStaticParams` / `staticParams` → SSG
5. Static candidates → SSG
6. Default → SSR

---

## NPM Package Architecture

```
ruvyxa (CLI + re-exports)
├── @ruvyxa/core          — config types, server APIs, adapter contracts
├── @ruvyxa/react         — Image, SEO, hydration, loaders, error boundary
├── @ruvyxa/auth          — sessions, OAuth, magic-link, WebAuthn
├── @ruvyxa/database      — typed CRUD with adapter pattern
├── @ruvyxa/realtime      — WebSocket action transport
├── @ruvyxa/adapter-*     — 10 platform adapters
├── @ruvyxa/cli-*         — 5 platform binaries
└── create-ruvyxa         — project scaffold
```

`ruvyxa` re-exports `@ruvyxa/core` subpaths:

- `ruvyxa/config` → `@ruvyxa/core/config`
- `ruvyxa/server` → `@ruvyxa/core/server`
- `ruvyxa/plugin` → `@ruvyxa/core/plugin`
- `ruvyxa/plugins` → built-in plugins (redirects, headers, sitemap, PWA, etc.)

---

## Source File → URL Mapping

| Pattern                         | URL                    | Type                    |
| ------------------------------- | ---------------------- | ----------------------- |
| `app/page.tsx`                  | `/`                    | Page                    |
| `app/about/page.tsx`            | `/about`               | Page                    |
| `app/blog/[slug]/page.tsx`      | `/blog/:slug`          | Dynamic                 |
| `app/docs/[...rest]/page.tsx`   | `/docs/*`              | Catch-all               |
| `app/shop/[[...cats]]/page.tsx` | `/shop` or `/shop/a/b` | Optional catch-all      |
| `app/api/route.ts`              | `/api`                 | API                     |
| `app/layout.tsx`                | —                      | Layout (wraps children) |
| `app/(group)/page.tsx`          | `/`                    | Route group             |
| `app/@modal/page.tsx`           | —                      | Parallel slot (ignored) |
| `app/_private/page.tsx`         | —                      | Private dir (ignored)   |
| `app/action.ts`                 | —                      | Server action           |
| `app/server.ts`                 | —                      | Server module           |
| `app/client.tsx`                | —                      | Client module           |
| `app/page.md` / `.mdx`          | `/`                    | Content page            |

---

## Project Structure (created by `create-ruvyxa`)

```
my-app/
├── app/
│   ├── globals.css       # Global styles
│   ├── layout.tsx        # Root layout (HTML shell)
│   └── page.tsx          # Home page
├── public/               # Static assets served from /
├── ruvyxa.config.ts      # Framework config
├── tsconfig.json
└── package.json
```

---

## Key CLI Commands

| Command              | Description                                                   |
| -------------------- | ------------------------------------------------------------- |
| `ruvyxa dev`         | Development server with HMR                                   |
| `ruvyxa build`       | Production build → `.ruvyxa/`                                 |
| `ruvyxa check`       | App-level production-readiness checks                         |
| `ruvyxa start`       | Serve production build                                        |
| `ruvyxa preview`     | Preview an existing production build locally                  |
| `ruvyxa routes`      | Print route table (`--json` emits its manifest)               |
| `ruvyxa analyze`     | Validate routes/imports/boundaries; can emit interactive HTML |
| `ruvyxa add`         | Scaffold a form, data table, or authentication flow           |
| `ruvyxa doctor`      | Check environment and project setup                           |
| `ruvyxa clean`       | Remove generated Ruvyxa build output                          |
| `ruvyxa trace`       | Inspect one route manifest entry by path                      |
| `ruvyxa bench`       | Benchmark route discovery, analysis, and production build     |
| `ruvyxa test:parity` | Dev/prod route comparison + smoke renders                     |
| `ruvyxa plugin`      | Create a publishable plugin package                           |

---

## Next: Architecture Deep Dives

## Implementation Entry Points and Reading Order

This documentation is easiest to verify by following the runtime path rather than reading crates
alphabetically. The table below maps each user-visible concern to its primary source boundary.

| Concern                           | Primary implementation                                                           | What it owns                                                           | Read next                                         |
| --------------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------- |
| Command parsing and orchestration | `crates/ruvyxa_cli/src/main.rs`                                                  | CLI surface, argument precedence, dispatch                             | [CLI & Build Pipeline](cli.md)                    |
| Configuration translation         | `crates/ruvyxa_cli/src/config.rs`, `packages/ruvyxa/runtime/config-renderer.mjs` | Config files, validation, runtime config hand-off                      | [CLI & Build Pipeline](cli.md)                    |
| Route discovery and validation    | `crates/ruvyxa_graph/src/lib.rs`                                                 | File conventions, manifests, rendering detection, boundary diagnostics | [Route Discovery](graph.md)                       |
| Client compilation and linking    | `crates/ruvyxa_bundler/src`                                                      | AST scanning, resolution, boundary checks, output                      | [Bundler](bundler.md)                             |
| HTTP serving and rendering        | `crates/ruvyxa_dev_server/src/lib.rs`                                            | Axum routes, request dispatch, HMR, render cache, security application | [Dev Server](dev-server.md)                       |
| Middleware and plugin bridge      | `crates/ruvyxa_middleware/src` and `packages/ruvyxa/runtime/plugin-runtime.mjs`  | Middleware stacking and JavaScript-plugin communication                | [Middleware](middleware.md)                       |
| Public TypeScript contract        | `packages/@ruvyxa/core/src`, `packages/@ruvyxa/react/src`                        | Config, server helpers, React components/hooks                         | [API Reference](../guides/en/17-api-reference.md) |

### Boundary Walkthrough: One Request

For a page request, the dev server obtains or refreshes the route manifest from `ruvyxa_graph`,
matches the request, and sends rendering work through its worker/runtime path. The bundler and graph
share source-scanning facts for imports and environment reads so a `check` result and a build are
less likely to disagree about a client/server boundary. Plugins and middleware wrap the HTTP path;
they are not a substitute for route discovery or rendering strategy selection.

When investigating a framework issue, start with the user-visible symptom and follow this order:

```text
CLI command -> config -> route manifest -> module/boundary validation -> dev server or build output
```

This order prevents a common debugging mistake: changing the server or adapter when the route was
never discovered, or changing a page when the failure is a module boundary violation.

- [Route Discovery & Validation](graph.md) — how `app/` becomes a route manifest
- [Compilation Pipeline](bundler.md) — resolver → compiler → linker → minifier
- [Dev Server](dev-server.md) — Axum serving, HMR protocol, render cache
- [CLI & Build Pipeline](cli.md) — command structure, config loading, staging
- [Middleware](middleware.md) — Tower stack, plugin bridge
- [Worker Pool](worker-pool.md) — Node/Bun workers, protocol, recovery
- [Diagnostics](diagnostics.md) — RUV#### error catalog
- [Protocols](protocols.md) — NDJSON, WebSocket HMR, Fetch
- [Security](security.md) — env isolation, rate limiting, boundaries
- [Deployment Adapters](deployment-adapters.md) — adapter system overview
- [Concurrency](concurrency.md) — parallelism model, locks
- [Site Discovery](site-discovery.md) — sitemap/robots generation
