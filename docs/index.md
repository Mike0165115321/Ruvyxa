# Ruvyxa Documentation

[GitHub](https://github.com/thirawat27/ruvyxa) · [npm](https://www.npmjs.com/package/ruvyxa)

---

Ruvyxa is a React full-stack framework with file-system routing, a Rust-native toolchain (bundler,
server, graph), and first-class support for SSR, SSG, ISR, PPR, and CSR.

```
npm create ruvyxa@latest my-app
cd my-app
npm install
npm run dev        # → http://localhost:3000
```

---

## Quick Start by Background

| If you are…               | Start here                                                                                                           |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| **New to frameworks**     | [Getting Started](guides/en/01-getting-started.md) — zero to running app in 10 min                                   |
| **React developer**       | [Routing](guides/en/02-routing.md) then [Rendering](guides/en/04-rendering-strategies.md)                            |
| **From Next.js**          | [Routing mental model](guides/en/02-routing.md) + [Server/Client boundary](guides/en/03-server-client-components.md) |
| **Framework contributor** | [Developer Guide](developer-guide.md)                                                                                |
| **Architecture explorer** | [System Overview](architecture/overview.md)                                                                          |

---

## User Guide (17 chapters)

| #   | Chapter                                                                | Level | Topics                                                                                              |
| --- | ---------------------------------------------------------------------- | ----- | --------------------------------------------------------------------------------------------------- |
| 01  | [Getting Started](guides/en/01-getting-started.md)                     | 🟢    | Install, scaffold, project structure, first page, dev server, HMR, troubleshooting                  |
| 02  | [Routing](guides/en/02-routing.md)                                     | 🟢    | File-system routes, dynamic segments, catch-all, layouts, metadata, route groups, client navigation |
| 03  | [Server & Client Components](guides/en/03-server-client-components.md) | 🟡    | `'use client'` directive, server-only code, boundary validation, composition patterns               |
| 04  | [Rendering Strategies](guides/en/04-rendering-strategies.md)           | 🟢    | SSR, SSG, ISR, PPR, CSR, hydration scheduling, zero-JS pages, prerender output                      |
| 05  | [Data Loading & Cache](guides/en/05-data-loading-cache.md)             | 🟡    | Loader, cache API, TTL/SWR, invalidation, `useRuvyxaLoader` hook                                    |
| 06  | [Server Actions](guides/en/06-server-actions.md)                       | 🟡    | Mutations, input validation, form integration, security, cache invalidation                         |
| 07  | [API Routes](guides/en/07-api-routes.md)                               | 🟡    | HTTP handlers, streaming, body limits, security, error responses                                    |
| 08  | [Styling](guides/en/08-styling.md)                                     | 🟢    | Global CSS, SCSS/Sass, CSS Modules, HMR, `css.entries` config                                       |
| 09  | [Markdown, MDX & Images](guides/en/09-markdown-mdx-images.md)          | 🟢    | Content pages, frontmatter, MDX components, Image optimization, SEO                                 |
| 10  | [Environment Variables](guides/en/10-environment-variables.md)         | 🟢    | `.env` files, `RUVYXA_PUBLIC_*` prefix, server-only secrets, TypeScript declarations                |
| 11  | [Configuration](guides/en/11-configuration.md)                         | 🔴    | Complete `ruvyxa.config.ts` reference, every field, default, validation rule                        |
| 12  | [CLI Commands](guides/en/12-cli-commands.md)                           | 🟢    | All 13 commands, options, example output, exit codes                                                |
| 13  | [Deployment](guides/en/13-deployment.md)                               | 🟢    | Build output, adapters, all platforms, production checklist, CI/CD                                  |
| 14  | [Plugins](guides/en/14-plugins.md)                                     | 🔴    | 16 built-in plugins, custom plugin API, hooks, lifecycle                                            |
| 15  | [Official Packages](guides/en/15-official-packages.md)                 | 🟡    | Auth, Database, Realtime — server API, client API, plugin setup                                     |
| 16  | [Error Handling](guides/en/16-error-handling.md)                       | 🔴    | Current RUV#### diagnostics, error boundaries, and recovery guidance                                |
| 17  | [API Reference](guides/en/17-api-reference.md)                         | 🔴    | Public exports from `@ruvyxa/react` and `@ruvyxa/core`                                              |

---

## User Guide (ภาษาไทย — 17 บท)

| #   | บท                                                                     | ระดับ |
| --- | ---------------------------------------------------------------------- | ----- |
| 01  | [เริ่มต้นใช้งาน](guides/th/01-getting-started.md)                      | 🟢    |
| 02  | [Routing](guides/th/02-routing.md)                                     | 🟢    |
| 03  | [Server & Client Components](guides/th/03-server-client-components.md) | 🟡    |
| 04  | [กลยุทธ์การ Render](guides/th/04-rendering-strategies.md)              | 🟢    |
| 05  | [การโหลดข้อมูลและ Cache](guides/th/05-data-loading-cache.md)           | 🟡    |
| 06  | [Server Actions](guides/th/06-server-actions.md)                       | 🟡    |
| 07  | [API Routes](guides/th/07-api-routes.md)                               | 🟡    |
| 08  | [การจัดแต่งสไตล์](guides/th/08-styling.md)                             | 🟢    |
| 09  | [Markdown, MDX และรูปภาพ](guides/th/09-markdown-mdx-images.md)         | 🟢    |
| 10  | [ตัวแปร Environment](guides/th/10-environment-variables.md)            | 🟢    |
| 11  | [อ้างอิงการกำหนดค่า](guides/th/11-configuration.md)                    | 🔴    |
| 12  | [คำสั่ง CLI](guides/th/12-cli-commands.md)                             | 🟢    |
| 13  | [การ Deploy](guides/th/13-deployment.md)                               | 🟢    |
| 14  | [ปลั๊กอิน](guides/th/14-plugins.md)                                    | 🔴    |
| 15  | [แพ็กเกจทางการ](guides/th/15-official-packages.md)                     | 🟡    |
| 16  | [การจัดการ Error](guides/th/16-error-handling.md)                      | 🔴    |
| 17  | [เอกสารอ้างอิง API](guides/th/17-api-reference.md)                     | 🔴    |

---

## Architecture Reference

| Document                                                   | Crate                | Coverage                                                                                          |
| ---------------------------------------------------------- | -------------------- | ------------------------------------------------------------------------------------------------- |
| [System Overview](architecture/overview.md)                | —                    | Design philosophy, crate dependency graph, key decisions                                          |
| [Route Discovery](architecture/graph.md)                   | `ruvyxa_graph`       | File conventions, segment types, validation, manifest, rendering strategy detection               |
| [Compilation Pipeline](architecture/bundler.md)            | `ruvyxa_bundler`     | Oxc resolver/compiler, IIFE linker, minifier, CSS modules, boundary checks, chunking, source maps |
| [Dev Server](architecture/dev-server.md)                   | `ruvyxa_dev_server`  | Axum HTTP server, radix trie router, HMR WebSocket, render cache, style pipeline                  |
| [CLI & Build Pipeline](architecture/cli.md)                | `ruvyxa_cli`         | 13 commands, two-phase config, staging build, atomic commit, image optimization                   |
| [Middleware](architecture/middleware.md)                   | `ruvyxa_middleware`  | Tower middleware stack, compression, CORS, security headers, rate limiting, plugin host           |
| [Worker Pool](architecture/worker-pool.md)                 | —                    | Node/Bun persistent workers, NDJSON protocol, LRU bundle cache, request coalescing                |
| [Diagnostics](architecture/diagnostics.md)                 | `ruvyxa_diagnostics` | All RUV#### codes with title, explanation, fix                                                    |
| [Wire Protocols](architecture/protocols.md)                | —                    | NDJSON frames, HMR WebSocket messages, Fetch API wrappers                                         |
| [Security Model](architecture/security.md)                 | —                    | Env isolation, same-origin, rate limiting, body limits, trusted proxies                           |
| [Deployment Adapters](architecture/deployment-adapters.md) | —                    | Adapter contract, all 10 platform adapters, build.json schema                                     |
| [Concurrency](architecture/concurrency.md)                 | —                    | Rayon parallelism, DashMap, mutex use, performance characteristics                                |
| [Site Discovery](architecture/site-discovery.md)           | —                    | Sitemap/robots generation, URL ownership, metadata, validation                                    |

---

## Package Reference

| Package             | Role                      | Key exports                                                                                  |
| ------------------- | ------------------------- | -------------------------------------------------------------------------------------------- |
| `ruvyxa`            | CLI + runtime bridge      | Re-exports `@ruvyxa/core` subpaths + runtime scripts + plugins                               |
| `@ruvyxa/core`      | Config types, server APIs | `config()`, `loader`, `cache`, `action`, `definePlugin`                                      |
| `@ruvyxa/react`     | React integration         | `Link`, `Image`, `Seo`, `useRouter`, `useRuvyxaLoader`, `notFound`                           |
| `@ruvyxa/auth`      | Authentication            | `createAuth`, `createAuthClient`, session stores, OAuth providers                            |
| `@ruvyxa/database`  | Database facade           | `createDatabase`, Prisma/DynamoDB/custom adapters                                            |
| `@ruvyxa/realtime`  | WebSocket transport       | `realtimeClient`, `useRealtime`, action channel subscriptions                                |
| `@ruvyxa/adapter-*` | Platform deploy           | 10 adapters (vercel, netlify, cloudflare, node, bun, static, aws, firebase, railway, render) |
| `@ruvyxa/cli-*`     | Native binaries           | 5 platform binaries (darwin-arm64, linux-arm64, linux-x64, win32-arm64, win32-x64)           |
| `create-ruvyxa`     | Project scaffold          | `create-ruvyxa` CLI, 4 starter templates                                                     |

---

## Quick Links

- **[Getting Started](guides/en/01-getting-started.md)** — start here
- **[Routing](guides/en/02-routing.md)** — understand the file-system router
- **[CLI Commands](guides/en/12-cli-commands.md)** — full command reference
- **[Configuration](guides/en/11-configuration.md)** — complete config reference
- **[Error Codes](architecture/diagnostics.md)** — RUV#### code catalog
- **[Developer Guide](developer-guide.md)** — contributing to the framework

---

## How to Navigate by Task

Use the guides for application decisions and the architecture pages for implementation reasoning.
Each detailed guide now includes an implementation-bound section that distinguishes the supported
contract from examples that are application-specific.

| If you need to…                  | Start here                                                     | Then verify with                               |
| -------------------------------- | -------------------------------------------------------------- | ---------------------------------------------- |
| Add or troubleshoot a page       | [Routing](guides/en/02-routing.md)                             | `ruvyxa routes`, then `ruvyxa trace <pattern>` |
| Choose SSR/SSG/ISR/PPR/CSR       | [Rendering Strategies](guides/en/04-rendering-strategies.md)   | Route manifest / `ruvyxa trace`                |
| Keep a secret out of the browser | [Environment Variables](guides/en/10-environment-variables.md) | `ruvyxa analyze --format human`                |
| Configure a deployment           | [Deployment](guides/en/13-deployment.md)                       | `ruvyxa doctor --adapter <name>`               |
| Create a plugin                  | [Plugins](guides/en/14-plugins.md)                             | `ruvyxa plugin create <name>`                  |
| Understand framework internals   | [Architecture Overview](architecture/overview.md)              | Owning crate/package source listed there       |

The minimal starter has scripts only for `dev`, `build`, `start`, `typecheck`, and `check`. Invoke
other CLI capabilities directly unless the project deliberately adds a corresponding package script.
