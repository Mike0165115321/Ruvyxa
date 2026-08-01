# Getting Started with Ruvyxa

Welcome to Ruvyxa — a full-stack framework that puts developer experience first. Whether you are
building a marketing site, a SaaS dashboard, a blog, or an API backend, Ruvyxa gives you the tools
to ship fast without sacrificing control.

This guide walks you from zero to your first running app in about 10 minutes.

---

## What You Need

| Requirement | Minimum                  |
| ----------- | ------------------------ |
| Node.js     | 22.x or later            |
| npm         | 10+ (ships with Node)    |
| pnpm        | 9+ (optional)            |
| Yarn        | 4+ (optional)            |
| Bun         | 1.2+ (optional)          |
| OS          | Windows, macOS, or Linux |

Check your Node version:

```bash
node -v
# v22.0.0 or higher
```

Ruvyxa uses `node --run` mode internally. If you are on Windows, make sure you have PowerShell 7+ or
a modern terminal like Windows Terminal.

---

## Creating Your First Project

Open a terminal and run:

```bash
npm create ruvyxa@latest my-app
```

### All CLI Options

`npm create ruvyxa@latest` accepts the following flags:

| Flag         | Type                                             | Default     | Description                  |
| ------------ | ------------------------------------------------ | ----------- | ---------------------------- |
| `--template` | `'minimal' \| 'blog' \| 'crud' \| 'api-backend'` | `'minimal'` | Starter template to scaffold |

```bash
# Use blog template
npm create ruvyxa@latest my-blog -- --template blog
```

### Interactive Prompt

You will see an interactive prompt asking which template to use:

```
? Select a starter template (Use arrow keys)
❯ minimal     – Clean skeleton, bare-bones
  blog        – MDX blog with posts, tags, RSS
  crud        – Full CRUD with database and auth
  api-backend – Pure API with route.ts endpoints
```

Choose **minimal** for now. The CLI will scaffold the project and install dependencies.

```
✔  Successfully created project "my-app"!
   cd my-app
   npm run dev
```

### Creation Safety Checks

`create-ruvyxa` performs these validations before scaffolding:

1. **Directory name validation** — rejects names with `<>:"|?*`, reserved Windows names (`con`,
   `nul`, `com1`), names starting with `.` or `-`, names >128 chars
2. **Write permission check** — verifies parent directory is writable
3. **Target empty check** — fails if non-empty directory exists (detects existing Ruvyxa projects
   and suggests `cd` instead)
4. **Template integrity** — verifies required files exist: `app/page.tsx`, `app/layout.tsx`,
   `app/globals.css`, `package.json`, `ruvyxa.config.ts`, `AGENTS.md`, `CLAUDE.md`

---

## Project Structure

Here is what you get inside `my-app/` for each template:

### Minimal

```
my-app/
├── app/
│   ├── layout.tsx        # Root layout — wraps every page
│   ├── page.tsx          # Home page at /
│   └── globals.css       # Global stylesheet
├── public/
├── AGENTS.md
├── CLAUDE.md
├── ruvyxa.config.ts      # Framework configuration
├── tsconfig.json
├── package.json
└── node_modules/
```

### Blog

```
my-blog/
├── app/
│   ├── layout.tsx
│   ├── page.tsx          # Home page
│   ├── globals.css
│   ├── about/
│   │   └── page.tsx      # /about
│   └── blog/
│       ├── page.tsx      # /blog — post list
│       └── [slug]/
│           └── page.tsx  # /blog/:slug — individual post (SSG via staticParams)
├── public/
├── ruvyxa.config.ts
├── tsconfig.json
├── package.json
└── node_modules/
```

### CRUD

```
my-crud/
├── app/
│   ├── layout.tsx
│   ├── page.tsx          # Home page
│   ├── globals.css
│   ├── about/
│   │   └── page.tsx      # /about
│   └── tasks/
│       ├── page.tsx      # /tasks — server-rendered task list
│       ├── server.ts     # Server-only data access
│       └── action.ts     # Server actions (createTask, toggleTask, deleteTask)
├── public/
├── ruvyxa.config.ts
├── tsconfig.json
├── package.json
└── node_modules/
```

### API Backend

```
my-api/
├── app/
│   ├── layout.tsx
│   ├── page.tsx
│   ├── globals.css
│   └── api/
│       ├── health/
│       │   └── route.ts   # GET /api/health
│       └── items/
│           ├── route.ts   # GET, POST /api/items
│           ├── store.ts   # In-memory data store
│           └── [id]/
│               └── route.ts  # GET, PUT, DELETE /api/items/:id
├── public/
├── ruvyxa.config.ts
├── tsconfig.json
├── package.json
└── node_modules/
```

Every route lives inside `app/`. The file system **is** your router.

```
app/
  page.tsx       → /
  about/
    page.tsx     → /about
  blog/
    [slug]/
      page.tsx   → /blog/:slug
```

---

## Config File Reference

### `ruvyxa.config.ts`

```ts
import { config, type RuvyxaConfig } from 'ruvyxa/config'

const settings: RuvyxaConfig = {
  appDir: 'app',
  outDir: '.ruvyxa',
  server: {
    host: 'localhost',
    port: 3000,
  },
  build: {
    minify: true,
    map: false,
    treeShake: true,
    split: 'route',
    workers: 4,
  },
  cache: {
    routes: true,
    css: true,
  },
  debug: {
    overlay: true,
  },
  image: {
    optimize: true,
    quality: 82,
    lossless: false,
    workers: 0,
  },
}

export default config(settings)
```

### `RuvyxaConfig` Type — All Fields

```ts
export interface RuvyxaConfig {
  appDir?: string // @default 'app'
  outDir?: string // @default '.ruvyxa'
  runtime?: 'node' | 'bun' | 'edge' | 'static' // @default 'node'
  react?: boolean // @default true
  typescript?: {
    strict?: boolean // @default true
  }
  css?: {
    entries?: string[] // Additional global CSS files/directories
  }
  server?: {
    port?: number // @default 3000
    host?: string // @default 'localhost'
  }
  build?: {
    minify?: boolean // @default true
    map?: boolean // @default false
    treeShake?: boolean // @default true
    split?: 'single' | 'route' | 'manual' // @default 'route'
    workers?: number // @default available CPUs
    jsx?: 'classic' | 'automatic' // @default 'automatic'
    target?: 'es2018' | 'es2019' | 'es2020' | 'es2022' | 'esnext' // @default 'es2022'
    manifest?: boolean // @default true
    warm?: boolean // Precompile dev route modules
    prerenderCache?: boolean // @default true
  }
  render?: RenderConfig
  debug?: {
    overlay?: boolean // @default true
    traces?: boolean // @default false
  }
  image?: ImageConfig
  security?: {
    actionLimit?: number // @default 1048576 (1MB)
    apiLimit?: number // @default 10485760 (10MB)
    pluginLimit?: number // @default 33554432 (32MB) @maximum 268435456
    actionRateLimit?: {
      max?: number // @default 600
      window?: number // @default 60 (seconds)
    }
    sameOrigin?: boolean // @default false
    fetchMeta?: boolean // @default false
    trustedProxyIps?: string[]
    headers?: boolean // @default true
  }
  cache?: {
    routes?: boolean // @default true
    css?: boolean // @default true
    dir?: string // Shared compile-cache dir
  }
  site?: SiteConfig
  middleware?: MiddlewareConfig
  adapter?: Adapter
  adapterOptions?: Record<string, unknown>
  plugins?: RuvyxaPlugin[]
}
```

### `RenderConfig`

```ts
export interface RenderConfig {
  strategy?: 'ssr' | 'ssg' | 'isr' | 'csr' | 'ppr' // @default 'ssr'
  revalidate?: number // @default 60
}
```

### `ImageConfig`

```ts
export interface ImageConfig {
  optimize?: boolean // @default true — Convert PNG/JPEG to WebP
  quality?: number // @default 82 — 1-100
  lossless?: boolean // @default false
  keepOriginal?: boolean // @default true
  variantWidths?: number[] // @default [640, 750, 828, 1080, 1200, 1920, 2048, 3840]
  workers?: number // @default 0 (auto)
}
```

### `RUV1601` / `RUV1602` Config Validation

Config validation fires these errors:

| Code      | Condition                     | Message                                                                            |
| --------- | ----------------------------- | ---------------------------------------------------------------------------------- |
| `RUV1601` | Field must be > 0             | `config field 'build.workers' must be greater than zero`                           |
| `RUV1601` | Field must be non-empty       | `config field 'appDir' must not be empty`                                          |
| `RUV1601` | Path must be project-relative | `config field 'cache.dir' must be a project-relative path inside the project root` |
| `RUV1601` | Invalid enum value            | `build.jsxRuntime must be 'classic' or 'automatic', got 'other'`                   |
| `RUV1601` | Invalid target                | `build.esTarget must be es2018, es2019, es2020, es2022, or esnext, got 'other'`    |
| `RUV1601` | Invalid split                 | `build.splitStrategy must be 'single', 'route', or 'manual', got 'other'`          |
| `RUV1602` | Field exceeds maximum         | `config field 'security.pluginLimit' must not exceed 268435456 bytes`              |
| `RUV1602` | Invalid IP or CIDR range      | `config field 'security.trustedProxyIps' contains invalid IP or CIDR range 'xyz'`  |
| `RUV1602` | Workers range                 | `config field 'middleware.workers' must be between 1 and 8`                        |
| `RUV1602` | Timeout range                 | `config field 'middleware.timeoutMs' must be between 1 and 300000`                 |

---

## TypeScript Configuration

### `tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "react-jsx",
    "strict": true,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "paths": {
      "~/*": ["./*"]
    }
  },
  "include": ["app", "ruvyxa.config.ts"]
}
```

### `ruvyxa-env.d.ts`

Create `app/ruvyxa-env.d.ts` for type-safe CSS modules and env vars:

```ts
declare module '*.css' {
  const content: Record<string, string>
  export default content
}

interface ImportMetaEnv {
  RUVYXA_PUBLIC_APP_NAME: string
  RUVYXA_PUBLIC_API_URL: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
```

This enables:

- Type-safe CSS module imports: `import styles from './styles.module.css'`
- Type-safe public env vars: `import.meta.env.RUVYXA_PUBLIC_APP_NAME`
- Autocompletion in editors

---

## `.gitignore`

All Ruvyxa projects include this `.gitignore`:

```gitignore
node_modules/
.ruvyxa/
dist/
.env
.env.*
!.env.example
*.log
.DS_Store

# Deployment artifacts (generated by ruvyxa build)
.vercel/
.netlify/
```

---

## `package.json` Scripts

Every Ruvyxa project comes with these scripts:

```json
{
  "scripts": {
    "dev": "ruvyxa dev",
    "build": "ruvyxa build",
    "start": "ruvyxa start",
    "typecheck": "tsc --noEmit",
    "check": "ruvyxa check"
  }
}
```

| Script        | CLI Command          | Exact invocation         | What it does                                      |
| ------------- | -------------------- | ------------------------ | ------------------------------------------------- |
| `dev`         | `ruvyxa dev`         | `npx ruvyxa dev`         | Start dev server with HMR                         |
| `build`       | `ruvyxa build`       | `npx ruvyxa build`       | Production build to `.ruvyxa/`                    |
| `start`       | `ruvyxa start`       | `npx ruvyxa start`       | Serve production build from `.ruvyxa/`            |
| `typecheck`   | `tsc --noEmit`       | `npx tsc --noEmit`       | Type-check without emitting files                 |
| `check`       | `ruvyxa check`       | `npx ruvyxa check`       | Validate routes, config, imports, parity          |
| `preview`     | `ruvyxa preview`     | `npx ruvyxa preview`     | Serve production build locally (alias of `start`) |
| `routes`      | `ruvyxa routes`      | `npx ruvyxa routes`      | Print the discovered route table                  |
| `analyze`     | `ruvyxa analyze`     | `npx ruvyxa analyze`     | Validate routes, imports, and boundaries          |
| `doctor`      | `ruvyxa doctor`      | `npx ruvyxa doctor`      | Diagnose project issues                           |
| `clean`       | `ruvyxa clean`       | `npx ruvyxa clean`       | Remove configured generated build output          |
| `trace`       | `ruvyxa trace`       | `npx ruvyxa trace`       | Inspect one route-manifest entry                  |
| `bench`       | `ruvyxa bench`       | `npx ruvyxa bench`       | Benchmark discovery, analysis, and build          |
| `test:parity` | `ruvyxa test:parity` | `npx ruvyxa test:parity` | Compare dev/prod routes and smoke-render pages    |
| `plugin`      | `ruvyxa plugin`      | `npx ruvyxa plugin`      | Manage and scaffold plugins                       |

### Dependencies

```json
{
  "dependencies": {
    "@ruvyxa/react": "^1.0.25",
    "ruvyxa": "^1.0.25",
    "react": "^19.2.8",
    "react-dom": "^19.2.8"
  },
  "devDependencies": {
    "@types/react": "^19.2.17",
    "@types/react-dom": "^19.2.3",
    "typescript": "^7.0.2"
  },
  "engines": {
    "node": ">=22.12.0"
  }
}
```

---

## All CLI flags

### ruvyxa dev / start / preview

| Flag        | Type        | Default                 | Description            |
| ----------- | ----------- | ----------------------- | ---------------------- |
| `--root`    | `path`      | `.`                     | Project root directory |
| `--host`    | `string`    | From config (localhost) | Host to bind           |
| `--port`    | `number`    | From config (3000)      | Port to listen on      |
| `--runtime` | `node\|bun` | Auto                    | JavaScript runtime     |

### ruvyxa build

| Flag        | Type                  | Default      | Description             |
| ----------- | --------------------- | ------------ | ----------------------- |
| `--root`    | `path`                | `.`          | Project root            |
| `--target`  | `production\|preview` | `production` | Build target            |
| `--adapter` | `string`              | —            | Adapter name or package |
| `--runtime` | `node\|bun`           | Auto         | JavaScript runtime      |

Known adapter names: `node`, `bun`, `static`, `vercel`, `netlify`, `cloudflare`, `railway`,
`render`, `firebase`, `aws` or package name like `@scope/ruvyxa-adapter-deno`.

### ruvyxa check

| Flag        | Type        | Default | Description        |
| ----------- | ----------- | ------- | ------------------ |
| `--root`    | `path`      | `.`     | Project root       |
| `--runtime` | `node\|bun` | Auto    | JavaScript runtime |

### ruvyxa analyze

| Flag        | Type                       | Default | Description          |
| ----------- | -------------------------- | ------- | -------------------- |
| `--root`    | `path`                     | `.`     | Project root         |
| `--runtime` | `node\|bun`                | Auto    | JavaScript runtime   |
| `--format`  | `auto\|human\|json\|sarif` | `auto`  | Report format        |
| `--output`  | `path`                     | —       | Write output to file |

### ruvyxa doctor

| Flag        | Type                  | Default | Description        |
| ----------- | --------------------- | ------- | ------------------ |
| `--root`    | `path`                | `.`     | Project root       |
| `--target`  | `production\|preview` | —       | Production target  |
| `--adapter` | `string`              | —       | Inspect adapter    |
| `--runtime` | `node\|bun`           | Auto    | JavaScript runtime |
| `--json`    | —                     | —       | Report as JSON     |

### ruvyxa trace

| Flag     | Type     | Default  | Description           |
| -------- | -------- | -------- | --------------------- |
| `--root` | `path`   | `.`      | Project root          |
| `--path` | `string` | Required | Route path to inspect |

### ruvyxa bench

| Flag     | Type   | Default | Description  |
| -------- | ------ | ------- | ------------ |
| `--root` | `path` | `.`     | Project root |

---

## Running the Dev Server

```bash
cd my-app
npm run dev
```

### `ruvyxa dev` Options

| Flag                    | Type             | Description                                      |
| ----------------------- | ---------------- | ------------------------------------------------ |
| `--root <path>`         | path             | Project root; defaults to the current directory. |
| `--host <host>`         | string           | Override the configured bind host.               |
| `--port <port>`         | unsigned integer | Override the configured bind port.               |
| `--runtime <node\|bun>` | enum             | Override `RUVYXA_RUNTIME` and `config.runtime`.  |

### Dev Server Startup Sequence

The dev server goes through this exact sequence:

```
1. Config loading
   └─ Read ruvyxa.config.ts (or .js, .mjs, .cjs)
   └─ Validate config (fires RUV1601/RUV1602 on errors)
   └─ Apply defaults for missing fields

2. Port binding
   └─ Try configured port (default: 3000)
   └─ If busy → scan next port (3001, 3002, ... up to 65535)
   └─ If no port found → RUV1201: "No available server port was found"

3. Route discovery
   └─ Walk app/ directory recursively
   └─ Filter _ and @ prefixed folders
   └─ Collect page.tsx, page.jsx, page.mdx, page.md, route.ts, route.js
   └─ Detect render strategy per route
   └─ Validate layouts (RUV1004: missing default export)
   └─ Detect ambiguous routes (RUV1003)
   └─ Validate server/client boundary (RUV1007, RUV1008, RUV1009, RUV1010)

4. HMR initialization
   └─ Watch project files for changes
   └─ Serve the framework HMR endpoint
   └─ Invalidate affected route and style state

5. Server ready
   └─ Print summary: routes found, conflicts, HMR status
```

Example output:

```
⚡ Ruvyxa dev server running

  ➜  Local:   http://localhost:3000
  ➜  Network: http://192.168.x.x:3000

  ✓ 2 routes scanned
  ✓ 0 conflicts
  ✓ HMR ready
```

### HMR Protocol Overview

| File type                       | Change action                                                           |
| ------------------------------- | ----------------------------------------------------------------------- |
| `.tsx`, `.jsx` (page/component) | Hot module replacement — component re-renders in place, state preserved |
| `.ts`, `.js` (module)           | Full HMR — module re-evaluated, dependents update                       |
| `.css`, `.scss`, `.module.css`  | Hot style replacement — no page reload                                  |
| `.md`, `.mdx` (content pages)   | Full page re-render                                                     |
| `ruvyxa.config.ts`              | Full server restart                                                     |
| `app/` route file added/removed | Route re-scan + page reload                                             |
| `public/` assets                | No HMR needed — assets served directly                                  |

---

## Dev Server Startup Sequence (Diagram)

```
ruvyxa dev
    │
    ▼
┌─────────────┐
│ Config Load │← ruvyxa.config.ts + env
└──────┬──────┘
       ▼
┌─────────────┐
│ Port Scan   │← fallback if port is in use
└──────┬──────┘
       ▼
┌─────────────┐
│ Route Scan  │← WalkDir app/ + validate
└──────┬──────┘
       ▼
┌─────────────┐
│ Server Init │← Router + workers + watcher
└──────┬──────┘
       ▼
┌─────────────┐
│ Ready       │← HMR listening
└─────────────┘
```

---

## HMR — Hot Module Replacement

When you edit a file, Ruvyxa sends a WebSocket message to the browser:

### HMR event types

| Event type     | Trigger                    | Action                            |
| -------------- | -------------------------- | --------------------------------- |
| `route-change` | Route file created/deleted | Route table reload + page refresh |
| `page-update`  | Edit page component        | Hot-replace component, no refresh |
| `style-update` | Edit CSS/SCSS              | Inject stylesheet, no refresh     |
| `full-reload`  | Config or layout changed   | Full page reload                  |

HMR works via:

1. File watcher (notify crate) detects changes
2. HMR tracker summarizes the event type
3. WebSocket broadcasts event to the browser
4. Browser runtime handles the hot update

---

## Under the Hood: Radix Trie Routing

Ruvyxa uses a **Radix Tree** (compressed trie) to match URLs to routes:

1. Route paths are converted to trie nodes
2. Static segments match exactly
3. Dynamic segment `[param]` matches any single-level value
4. Catch-all `[...param]` matches all remaining segments
5. Optional catch-all `[[...param]]` matches all remaining segments or nothing at all
6. Priority: static > dynamic > catch-all > optional

Radix Router implementation is found in `crates/ruvyxa_dev_server/src/router.rs`.

---

## Your First Edit

Open `app/page.tsx`:

```tsx
export default function HomePage() {
  return (
    <main>
      <h1>Hello, Ruvyxa!</h1>
      <p>This is my first app.</p>
    </main>
  )
}
```

Save it. The browser updates immediately. That is HMR in action.

---

## Adding a Second Page

Create a new folder and file:

```
app/
  about/
    page.tsx
```

Write into `app/about/page.tsx`:

```tsx
export default function AboutPage() {
  return (
    <main>
      <h1>About Us</h1>
      <p>We make web frameworks.</p>
    </main>
  )
}
```

Visit **http://localhost:3000/about**. The new page works instantly — no restart needed.

---

## The 4 Starter Templates

### minimal

Clean skeleton with a single page and root layout. Best starting point for any project.

```
my-app/
├── app/
│   ├── layout.tsx
│   ├── page.tsx
│   └── globals.css
├── public/
├── ruvyxa.config.ts
├── tsconfig.json
└── package.json
```

### blog

MDX-powered blog with:

- `app/blog/[slug]/page.tsx` — individual posts with `staticParams` SSG
- Tags and category pages
- RSS feed generation
- Syntax-highlighted code blocks

```
my-blog/
├── app/
│   ├── layout.tsx
│   ├── page.tsx
│   ├── about/page.tsx
│   └── blog/
│       ├── page.tsx
│       └── [slug]/page.tsx
├── public/
└── ...
```

### crud

Full-stack CRUD app with:

- `app/tasks/server.ts` — server-only data access
- `app/tasks/action.ts` — server actions for mutations (createTask, toggleTask, deleteTask)
- Form validation with error display

```
my-crud/
├── app/
│   ├── layout.tsx
│   ├── page.tsx
│   ├── about/page.tsx
│   └── tasks/
│       ├── page.tsx
│       ├── server.ts
│       └── action.ts
├── public/
└── ...
```

### api-backend

Pure API backend:

- `route.ts` endpoints only
- No client components
- Adapter pre-configured for Node/Bun
- Example endpoints: CRUD, webhooks, streaming

```
my-api/
├── app/
│   ├── layout.tsx
│   ├── page.tsx
│   └── api/
│       ├── health/route.ts
│       └── items/
│           ├── route.ts
│           ├── store.ts
│           └── [id]/route.ts
├── public/
└── ...
```

---

## Full API: `createRuvyxaApp`

```ts
export interface CreateRuvyxaOptions {
  template?: 'minimal' | 'blog' | 'crud' | 'api-backend' // @default 'minimal'
}

export async function createRuvyxaApp(
  targetDir: string,
  options?: CreateRuvyxaOptions,
): Promise<void>
```

### Errors Thrown

| Condition              | Error message                                                                     |
| ---------------------- | --------------------------------------------------------------------------------- |
| No target dir          | `Project directory name is required.`                                             |
| Empty name             | `Project directory name must not be empty.`                                       |
| Whitespace padding     | `Project directory name must not start or end with whitespace.`                   |
| Invalid template       | `Unknown starter template "foo". Choose one of: minimal, blog, crud, api-backend` |
| Invalid chars          | `Directory names cannot contain: < > : " \| ? *`                                  |
| Windows reserved name  | `This name is reserved or unsafe on Windows.`                                     |
| Name too long (>128)   | `Maximum is 128 characters.`                                                      |
| Starts with `.` or `-` | `Project name "..." should not start with "." or "-".`                            |
| Non-empty target dir   | `Directory "..." already exists and is not empty.`                                |
| Permission denied      | `Cannot write to "...". Permission denied.`                                       |
| Missing template       | `Template directory was not found.`                                               |
| Template incomplete    | `Template is incomplete. Missing required files:`                                 |
| Copy failure           | `Failed to create project at "...".`                                              |

---

## Your First 10 Minutes

Here is a mini-plan to get familiar:

1. **`npm create ruvyxa@latest playtime`** — scaffold a minimal project
2. **`cd playtime && npm run dev`** — start the server
3. Edit `app/page.tsx` — change the heading
4. Create `app/hello/page.tsx` — write a simple component
5. Create `app/blog/[slug]/page.tsx` — try dynamic routing:

```tsx
export default function BlogPost({ params }: { params: { slug: string } }) {
  return <h1>Post: {params.slug}</h1>
}
```

6. Visit `/blog/hello-world` — see the slug appear
7. Run `npm run routes` — see the route table
8. Run `npm run doctor` — check for any issues

---

## Error Codes

| Code      | Meaning                            | Cause                                                | Solution                                           |
| --------- | ---------------------------------- | ---------------------------------------------------- | -------------------------------------------------- |
| `RUV1001` | App directory not found            | Missing app/ folder                                  | Create app/ or configure appDir                    |
| `RUV1002` | Invalid dynamic route segment      | Wrong dynamic segment syntax or catch-all not at end | Use `[name]` not `:name`; put catch-all at the end |
| `RUV1003` | Conflicting route paths            | Two files match the same URL shape                   | Use `npm run routes` to find duplicates            |
| `RUV1004` | Page missing default export        | page.tsx missing `export default`                    | Add `export default function Page() {}`            |
| `RUV1007` | Server-only module in client graph | Client component imports `server-only` module        | Move import to a server component                  |
| `RUV1008` | Private env var in client graph    | Client component uses `process.env.PRIVATE`          | Use `process.env.RUVYXA_PUBLIC_*` instead          |
| `RUV1009` | Client-only module in server graph | Server component imports `client-only` module        | Relocate browser-only code                         |
| `RUV1010` | Server directory reached by client | Client imports from server/ directory                | Move shared code outside of server/                |
| `RUV1100` | React SSR failed                   | Server-side render error                             | Check stack trace in console                       |
| `RUV1102` | SSR renderer not found             | Build output is missing server handler               | Re-run `npm run build`                             |
| `RUV1200` | API route execution failed         | `route.ts` runtime error                             | Review error message                               |
| `RUV1201` | No available server port           | No port available to bind                            | Specify `--port` or stop processes using the port  |
| `RUV1202` | API renderer was not found         | Runtime renderer is not ready                        | Install/verify runtime dependencies                |
| `RUV1205` | Prerender path conflict            | Static path conflicts with build output              | Change outDir                                      |
| `RUV1300` | Client hydration bundling failed   | Build client bundle error                            | Check compiler output                              |
| `RUV1303` | Client route not found             | Requested client bundle for missing route            | Check route path                                   |
| `RUV1304` | Client bundle for non-page route   | Requested client bundle for API route                | Apply only to page routes                          |
| `RUV1400` | Tailwind CSS compilation failed    | Tailwind CLI error                                   | Verify Tailwind config                             |
| `RUV1401` | Tailwind CLI not found             | Missing Tailwind dependency                          | `npm install tailwindcss`                          |
| `RUV1402` | Sass compilation failed            | .scss file syntax error                              | Check SCSS files                                   |
| `RUV1403` | CSS entry not found                | CSS file specified in config doesn't exist           | Verify paths                                       |
| `RUV1404` | CSS entry outside project root     | CSS entry path is outside project                    | Use relative paths inside project                  |
| `RUV1500` | SSG/ISR render failed              | Static generation error                              | Check error detail                                 |
| `RUV1501` | Route action not found             | Missing action.ts in route                           | Create action.ts                                   |
| `RUV1550` | PPR render failed                  | PPR streaming error                                  | Check error detail                                 |
| `RUV1600` | Config validation error            | ruvyxa.config.ts format is invalid                   | Run `ruvyxa doctor`                                |
| `RUV1601` | Config value out of range          | Field value out of acceptable range                  | Adjust value range                                 |
| `RUV1602` | Config value exceeds maximum       | Field value exceeds limits                           | Reduce value                                       |
| `RUV1700` | TypeScript plugin error            | Plugin runtime error                                 | Inspect plugin code                                |
| `RUV1701` | TypeScript plugin protocol error   | Plugin returned invalid payload                      | Verify plugin implementation                       |
| `RUV1702` | Worker pool script not found       | Missing runtime script                               | Re-run `npm run build`                             |
| `RUV1704` | Worker pool error                  | Worker crashed                                       | Review worker logs                                 |
| `RUV2200` | Adapter build failed               | Adapter runtime error                                | Inspect adapter                                    |
| `RUV2202` | Strategy not supported             | Adapter doesn't support render strategy              | Change strategy or adapter                         |
| `RUV2203` | Adapter package missing            | Adapter package not found                            | `npm install @ruvyxa/adapter-*`                    |
| `RUV9999` | Internal error                     | Compiler internal error                              | Report a bug                                       |

---

## Troubleshooting

### CLI Toolbox

| Problem                | Try                                                     |
| ---------------------- | ------------------------------------------------------- |
| Dev server won't start | `npm run doctor` — checks Node version, port, config    |
| Routes not appearing   | `npm run routes` — prints matched routes                |
| Strange build errors   | `npm run clean && npm run build` — fresh state          |
| "Address in use" error | Change port: `ruvyxa dev --port 4000`                   |
| Dependency issues      | Delete `node_modules` + lockfile, reinstall             |
| TypeScript errors      | `npm run typecheck` — pinpoints type issues             |
| Bundle too large       | `npm run analyze` — find what's bloating the bundle     |
| Adapter issues         | `npm run doctor` — verifies adapter configuration       |
| HMR not working        | Check file is inside `app/`, not excluded by `_` prefix |
| Config changes ignored | `ruvyxa dev` auto-restarts on config change             |

### Port Already in Use

```
RUV1201: No available server port was found
```

The dev server tries the configured port, then scans upward. If every port is busy:

```bash
ruvyxa dev --port 4000    # Use specific port
ruvyxa dev --port 0        # Use any available port (OS-assigned)
```

### Config Not Loading

If `ruvyxa.config.ts` has syntax errors, the dev server shows:

```
RUV1600: Config parse error
  └─ SyntaxError: Unexpected token ...
```

Check:

- Valid TypeScript/JavaScript syntax
- All string values are quoted
- Object keys don't have trailing commas
- `export default config(settings)` is present

### RUV##### Error Codes Reference

| Code      | Message                            | Cause                                                 |
| --------- | ---------------------------------- | ----------------------------------------------------- |
| `RUV1001` | App directory not found            | `app/` folder missing or misconfigured `appDir`       |
| `RUV1002` | Invalid dynamic route segment      | Segment name has invalid chars or catch-all not final |
| `RUV1003` | Conflicting route paths            | Two files resolve to same URL                         |
| `RUV1004` | Page missing default export        | `page.tsx` without `export default function`          |
| `RUV1007` | Server-only module in client graph | Client imports `server-only` or `@ruvyxa/auth`        |
| `RUV1008` | Private env var in client graph    | `process.env.SECRET` reachable from browser code      |
| `RUV1009` | Client-only module in server graph | Server imports `client-only`                          |
| `RUV1010` | Server directory in client graph   | Client imports from `server/` directory               |
| `RUV1100` | React SSR failed                   | Error during server-side rendering                    |
| `RUV1102` | SSR renderer not found             | Missing renderer entry                                |
| `RUV1201` | No available server port           | All ports in range are busy                           |
| `RUV1202` | API renderer not found             | Missing API route handler                             |
| `RUV1205` | Prerender path collision           | SSG route writes to build output directory            |
| `RUV1300` | Compile error                      | Module bundling or compilation failed                 |
| `RUV1310` | Unsupported content extension      | File extension not recognized                         |
| `RUV1311` | MDX parse error                    | Invalid MDX syntax                                    |
| `RUV1312` | Frontmatter error                  | Invalid YAML frontmatter in MD/MDX file               |
| `RUV1402` | Sass compilation failed            | Invalid SCSS/Sass syntax                              |
| `RUV1500` | Render error                       | SSG/ISR/action rendering failed                       |
| `RUV1550` | PPR render failed                  | Partial prerender error                               |
| `RUV1600` | Config error                       | General configuration error                           |
| `RUV1601` | Config field validation            | Field value outside allowed range                     |
| `RUV1602` | Config field exceeded limit        | Field too large or invalid                            |
| `RUV1700` | Plugin execution failed            | TypeScript plugin crashed or timed out                |
| `RUV1701` | Plugin protocol error              | Plugin returned invalid data                          |
| `RUV2000` | Middleware configuration error     | Invalid middleware config                             |
| `RUV2001` | Middleware execution failed        | Middleware hook threw                                 |
| `RUV2200` | Build error                        | General build failure                                 |
| `RUV2202` | Unsupported strategy for adapter   | Route strategy not supported by selected adapter      |
| `RUV2203` | Missing package                    | Required dependency not found                         |
| `RUV9999` | Internal error                     | Unclassified error (dev only)                         |

---

## Glossary

| Term                 | Meaning                                                              |
| -------------------- | -------------------------------------------------------------------- |
| **Route**            | A URL path handled by a file in `app/`                               |
| **Layout**           | A wrapper component (`layout.tsx`) that persists across child pages  |
| **HMR**              | Hot Module Replacement — updates browser without full reload         |
| **SSR**              | Server-Side Rendering — HTML generated per request                   |
| **SSG**              | Static Site Generation — HTML generated at build time                |
| **ISR**              | Incremental Static Regeneration — SSG with cache expiry              |
| **CSR**              | Client-Side Rendering — minimal HTML, JS does the rest               |
| **PPR**              | Partial Prerendering — static shell + dynamic slots                  |
| **API Route**        | A `route.ts` file that returns JSON (or any response)                |
| **Server Action**    | A function in `action.ts` that runs on the server                    |
| **Adapter**          | A deployment plugin for Vercel, Netlify, Cloudflare, Node, etc.      |
| **`.ruvyxa/`**       | Generated output directory (cached, build artifacts)                 |
| **`RUV####`**        | Error code format, e.g. `RUV1003` — search docs for details          |
| **Layout Chain**     | Ordered list of layout files wrapping a route (root → parent → page) |
| **Route Group**      | `(name)` folder that groups routes without affecting URL             |
| **Client Island**    | A `'use client'` component embedded in a server component tree       |
| **Bundle Splitting** | Strategy for dividing code into loaded-on-demand chunks              |

---

## Security Implications

### Environment Variables

- `RUVYXA_PUBLIC_*` variables are safe to use in client code
- Any other `process.env.*` reference in client code fires `RUV1008`
- Server-only env vars must stay in server components, API routes, or actions

### Payload Limits

| Limit                | Default            | Config key             |
| -------------------- | ------------------ | ---------------------- |
| Server action body   | 1 MB               | `security.actionLimit` |
| API route body       | 10 MB              | `security.apiLimit`    |
| Plugin response body | 32 MB (max 256 MB) | `security.pluginLimit` |

### Rate Limiting

```ts
// ruvyxa.config.ts
security: {
  actionRateLimit: {
    max: 600,     // requests per window
    window: 60,   // window in seconds
  }
}
```

---

## What a Fresh Scaffold Actually Provides

`create-ruvyxa` deliberately starts with a small, inspectable application. The current scaffolder
accepts four templates — `minimal` (the default), `blog`, `crud`, and `api-backend` — and copies a
validated template rather than synthesizing files one by one. Use one of these commands when
starting a new app:

```bash
npm create ruvyxa@latest my-app
npm create ruvyxa@latest my-blog -- --template blog
npm create ruvyxa@latest my-api -- --template api-backend
```

The generated project always contains at least `app/page.tsx`, `app/layout.tsx`, `app/globals.css`,
`ruvyxa.config.ts`, `package.json`, `AGENTS.md`, and `CLAUDE.md`. The project name in `package.json`
is derived from the directory name, so create the project in an empty directory you intend to own.
The generator rejects non-empty directories and unsafe Windows names before copying the template.

### The First Files Have Separate Jobs

| File               | Responsibility                                             | Safe first change                                 |
| ------------------ | ---------------------------------------------------------- | ------------------------------------------------- |
| `app/page.tsx`     | The page for `/`. A page module needs a default export.    | Replace the starter's `<main>` content.           |
| `app/layout.tsx`   | Wraps child pages and imports the baseline stylesheet.     | Set document language or shared shell.            |
| `app/globals.css`  | Global CSS imported by the root layout.                    | Add reset, font, or design-token rules.           |
| `ruvyxa.config.ts` | Project-level server, build, cache, and security settings. | Change `server.port` or add a documented setting. |
| `package.json`     | Dependency versions and runnable scripts.                  | Use its scripts as the canonical local workflow.  |

The minimal template uses `app` as the application directory and `.ruvyxa` as generated output. Keep
`.ruvyxa` out of source control: it is created by builds and can be regenerated with `ruvyxa clean`
followed by `ruvyxa build`.

### Learn by Checking One Fact at a Time

The minimal starter defines `dev`, `build`, `start`, `typecheck`, and `check` scripts. Commands such
as `routes`, `analyze`, `doctor`, `clean`, `trace`, and `bench` are CLI commands, not starter
scripts. Invoke them directly unless your own project adds a script:

```bash
npm run dev
ruvyxa routes
ruvyxa doctor
ruvyxa analyze --format human
```

This distinction makes examples portable: `npm run <name>` only works when that exact script exists
in the current project's `package.json`.

### A Useful First-hour Loop

After installing dependencies, use this short loop rather than changing configuration at random:

```bash
npm run dev
# in another terminal, after adding or renaming a page
ruvyxa routes
# before committing a meaningful change
npm run check
```

`dev` owns file watching and HMR. `routes` only discovers and prints routes; it does not start a
server. `check` runs TypeScript checking when a `tsconfig.json` is present and then runs the
framework parity flow. If the generated output seems stale, use `ruvyxa clean` before rebuilding; do
not delete dependencies or source files as part of that operation.

---

## Next Steps

- **[02-routing.md](./02-routing.md)** — Understand the file-based router in depth
- **[03-server-client-components.md](./03-server-client-components.md)** — Server vs client
  components
- **[04-rendering-strategies.md](./04-rendering-strategies.md)** — SSR, SSG, ISR, PPR, CSR explained
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — Fetch data and cache it
- **[06-server-actions.md](./06-server-actions.md)** — Server actions for mutations
- **[07-api-routes.md](./07-api-routes.md)** — Building REST or GraphQL endpoints
- **[08-styling.md](./08-styling.md)** — CSS, SCSS, CSS Modules

---

_You now have a running Ruvyxa app. Go build something great._
