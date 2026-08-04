# Architecture

## Boundary map

```mermaid
flowchart TB
  CLI[ruvyxa_cli] --> GRAPH[ruvyxa_graph]
  CLI --> BUNDLER[ruvyxa_bundler]
  CLI --> SERVER[ruvyxa_dev_server]
  SERVER --> MW[ruvyxa_middleware]
  CLI --> DIAG[ruvyxa_diagnostics]
  BUNDLER --> RT[packages/ruvyxa runtime]
  APP[Application + plugins] --> CLI
  APP --> REACT[@ruvyxa/react]
  APP --> CORE[@ruvyxa/core]
```

`ruvyxa_cli` owns commands, config loading, build output, prerendering, artifact caching, adapter
selection, and package-facing execution. `ruvyxa_graph` discovers and validates file-system routes
and rendering intent. `ruvyxa_bundler` compiles TypeScript/JSX, resolves/links modules, splits
chunks, minifies, writes source maps, handles styles, caches incrementally, and checks server/client
boundaries. `ruvyxa_dev_server` supplies Axum serving, routing, HMR, worker pools, render
cache/pipeline, static assets, i18n, image handling, and plugin bridge/head integration.

`ruvyxa_middleware` owns built-in middleware configuration/stack and plugin host behavior.
`ruvyxa_diagnostics` holds shared diagnostic reporting. JavaScript runtime files in
`packages/ruvyxa/runtime/` execute rendering/compiler/worker/adapters at the boundary where Rust
invokes TypeScript/React work.

## Request lifecycle

```mermaid
sequenceDiagram
  participant C as Client
  participant S as Dev/prod server
  participant M as Middleware/plugins
  participant R as Router/render pipeline
  participant W as Worker pool
  C->>S: Request
  S->>M: request hooks / built-ins
  M->>R: route or Response
  R->>W: API or React render work
  W-->>R: Response/HTML
  R->>M: response hooks
  M-->>C: Response
```

Request and response hooks can replace values or continue. Plugin response middleware buffers
TypeScript responses under `security.pluginLimit`, so large streaming responses require careful
sizing and testing. Worker settings are process controls, not a dependency-injection container; no
codebase evidence exposes a general public DI API, queue system, scheduler, or framework-managed
event bus.

## Build lifecycle

Build validates config and graph, compiles route/client code, runs build plugin hooks, prerenders
eligible SSG/ISR/PPR routes, emits site discovery files, records a manifest, and commits staging
output into place. The artifact cache fingerprints relevant inputs and can reuse final prerendered
HTML when `build.prerenderCache` is enabled (the default). Static adapters require generated
prerendered pages.

**Previous:** [CLI reference](10-cli.md) · **Next:**
[Development and testing](12-development-testing.md)
