# Server and Client Components

Ruvyxa splits your React tree into two worlds: **server** and **client**. Understanding this
boundary is the key to building fast, maintainable apps.

```
┌─────────────────────────────────────────────────┐
│                  SERVER WORLD                    │
│  - Reads DB, files, env directly                │
│  - No JS shipped to browser                     │
│  - Runs on every request (or once at build)     │
│  - Default for all components                   │
│                                                  │
│  ┌────────────────────────────────────────┐     │
│  │          CLIENT WORLD                  │     │
│  │  - 'use client' directive             │     │
│  │  - useState, useEffect, onClick       │     │
│  │  - Browser APIs (localStorage, etc)   │     │
│  │  - JS shipped to browser              │     │
│  │  - Re-hydrated on the client          │     │
│  └────────────────────────────────────────┘     │
│           ▲                                    │
│           │ props must be JSON-serializable     │
│           └─── boundary ───────────────────────│
└─────────────────────────────────────────────────┘
```

---

## The Two Worlds

### Server Components (default)

Every component in Ruvyxa is a **Server Component** by default — unless it says `'use client'`.

Server components can:

- Read from a database directly
- Access file system
- Read environment variables (server-only ones)
- Use async/await for data fetching
- Import server-only libraries

Server components **cannot**:

- Use `useState`, `useEffect`, `useReducer`
- Call `onClick`, `onSubmit`, or any event handler
- Access `localStorage`, `window`, or any browser API
- Use hooks from `@ruvyxa/react` like `useRouter`

```tsx
// This is a Server Component (no 'use client')
// It can be async — cool, right?
export default async function ProfilePage() {
  const user = await db.query('SELECT * FROM users WHERE id = 1')
  //           ^^^ direct DB access, safe on server

  const posts = await fs.readFile('posts.json', 'utf-8')
  //           ^^^ file system access, safe on server

  return (
    <div>
      <h1>{user.name}</h1>
      <p>{posts.length} posts</p>
    </div>
  )
}
```

### Server Component Restrictions — What You CANNOT Do

```tsx
// ❌ No React hooks
const [count, setCount] = useState(0) // Runtime error

// ❌ No event handlers
<button onClick={() => {}} /> // Won't work without 'use client'

// ❌ No browser APIs
localStorage.getItem("key") // ReferenceError
window.innerWidth           // ReferenceError
document.getElementById("x") // ReferenceError

// ❌ No router hooks (RUV1008)
import { useRouter } from "@ruvyxa/react" // Error: Server-only hook

// ❌ No client-only libraries
import "client-only" // RUV1009: Client-only module in server graph

// ❌ No private env vars in reachable code
process.env.DATABASE_URL // Fine if in server only
import.meta.env.RUVYXA_PUBLIC_API_URL // Fine — public prefix
```

### Client Components

Add `'use client'` at the top to make a component run on the client.

```tsx
'use client'

import { useState } from 'react'

export default function Counter() {
  const [count, setCount] = useState(0)

  return (
    <div>
      <p>Count: {count}</p>
      <button onClick={() => setCount(count + 1)}>+1</button>
    </div>
  )
}
```

Client components are **islands** — they are hydrated on the client but their server-rendered HTML
is sent first.

```
Server-rendered HTML:
  <div>Count: 0</div>
  <button>+1</button>

  ↓ hydration

Client JS takes over:
  Interactivity, state, effects
```

### `'use client'` Detection Algorithm

Source: `crates/ruvyxa_graph/src/lib.rs`

```rust
fn detect_render_strategy(...) {
    let source = fs::read_to_string(file);
    let trimmed = source.trim_start();

    // The 'use client' directive must be:
    // 1. At the very top of the file (after whitespace trim)
    // 2. Before any imports or code
    // 3. Exactly "use client" with single or double quotes
    if trimmed.starts_with("\"use client\"") || trimmed.starts_with("'use client'") {
        return RenderStrategy::Csr
    }
}
```

**Rules:**

- The directive must be trimmed whitespace at the start of the file
- Either `"use client"` or `'use client'`
- Must appear before any import statements
- Comment lines before the directive are NOT allowed (unlike some tools)
- The directive applies to **the entire file** — every export in the file becomes a client component

### Client Component Bundle

When a file has `'use client'`:

1. Everything it imports (that isn't server-only) ships to the browser
2. The client bundle includes the component, its dependencies, and their dependencies
3. Shared chunks (React, vendor libs) are extracted into separate files

```
'use client' file:
  Counter.tsx
    ├── uses: useState (from React) → ships to browser
    ├── imports: Button.tsx → ships to browser
    │   └── imports: icons.ts → ships to browser
    └── imports: api.ts → ships to browser
        └── NO process.env.SECRET (RUV1008 if it did)
```

### Props Must Be JSON-Serializable

Props passed from a server component to a client component must be serializable with
`JSON.stringify`.

```tsx
// ✅ Works — plain objects, strings, numbers, arrays, booleans
<ClientComponent
  name="Alice"
  count={42}
  items={["a", "b", "c"]}
  config={{ theme: "dark" }}
  active={true}
  nullable={null}
/>

// ❌ Fails — non-serializable types
<ClientComponent
  onClick={() => {}}     // Functions can't cross boundary
  date={new Date()}       // Date is not serializable
  buffer={Buffer.from()}  // Buffers not serializable
  jsx={<span>hi</span>}  // JSX elements not serializable
  map={new Map()}         // Map not serializable
  set={new Set()}         // Set not serializable
  regex={/pattern/}       // RegExp not serializable
  bigint={BigInt(1)}       // BigInt not serializable
/>
```

**Serializable types:**

- `string`, `number`, `boolean`, `null`, `undefined`
- Plain objects (no prototype chain)
- Arrays
- `Date` → serialized as ISO string (pass `.toISOString()`)
- `Buffer` → serialize to base64 string
- JSX → extract into its own server component

---

## `PageProps` and `LayoutProps` Types

```ts
export interface PageProps<TParams extends RouteParams = RouteParams> {
  /** Route parameters — synchronous access (not a Promise) */
  params: TParams
  /** The original request path */
  requestPath: string
}

export interface LayoutProps<TParams extends RouteParams = RouteParams> {
  /** Child content (page or nested layout) */
  children: React.ReactNode
  /** Route parameters */
  params: TParams
}

// RouteParams = Record<string, string | string[] | undefined>
// e.g. { slug: "hello" }
// e.g. { path: ["docs", "guide", "routing"] }
export type RouteParams = Record<string, RouteParamValue>
export type RouteParamValue = string | string[] | undefined
```

Usage:

```tsx
// app/blog/[slug]/page.tsx
import type { PageProps } from 'ruvyxa'

export default function BlogPost({ params, requestPath }: PageProps<{ slug: string }>) {
  return (
    <h1>
      Post: {params.slug} at {requestPath}
    </h1>
  )
}
```

---

## Composition Pattern: Server Wraps Client

The best architecture is a **server component tree** with **client islands** embedded inside.

```tsx
// app/page.tsx — Server Component (default)
import { db } from './db'
import { Counter } from './Counter'
import { PostList } from './PostList'

export default async function HomePage() {
  // This runs on the server
  const posts = await db.query('SELECT * FROM posts LIMIT 10')

  return (
    <main>
      <h1>My App</h1>

      {/* Client island — interactive */}
      <Counter />

      {/* Another server component — no JS */}
      <PostList posts={posts} />

      {/* Multiple client islands are fine */}
      <Counter />
    </main>
  )
}
```

```tsx
// app/Counter.tsx — Client Component
'use client'

import { useState } from 'react'

export function Counter() {
  const [count, setCount] = useState(0)
  return <button onClick={() => setCount((c) => c + 1)}>{count}</button>
}
```

```
Resulting HTML (no JS for the page itself):
  <main>
    <h1>My App</h1>
    <button>0</button>   ← hydrated by tiny JS bundle
    <ul>...</ul>          ← pure server HTML
    <button>0</button>   ← hydrated by tiny JS bundle
  </main>
```

This pattern means the page content is rendered on the server (fast, SEO-friendly) while interactive
elements get JavaScript only where needed.

---

## The Client Boundary

When you add `'use client'` to a file, **everything it imports that contains components also ships
to the browser**.

```tsx
'use client'

import { expensiveLib } from 'expensive-lib' // ← shipped to browser!
import { HeavyComponent } from './HeavyComponent' // ← shipped to browser!
```

This is the **client boundary**. The entire import tree below a `'use client'` file is client code.

### Performance Cost of Each `'use client'` Boundary

| Factor           | Impact                                                                |
| ---------------- | --------------------------------------------------------------------- |
| Bundle size      | All dependencies of the client file are included in the client bundle |
| Hydration time   | More client components → more JS to parse and execute                 |
| Network transfer | Larger bundles → longer download times                                |
| Memory usage     | Each hydrated component retains its DOM node reference                |

**Optimization:**

- Push the `'use client'` boundary as deep as possible
- Extract interactive elements into tiny leaf components
- Use `hydrate="idle"` or `hydrate="visible"` to defer non-critical hydration

---

## Server-Only Code

### The `server-only` Package

Use the `server-only` package to guard against accidental client imports:

```bash
npm install server-only
```

```tsx
// app/lib/db.ts
import 'server-only'

export const db = createDatabaseConnection()
```

If some client component imports `db.ts`, the build fails with:

```
RUV1007: Server-only module imported into client graph
  This module is reachable from a hydrated page or client module
  but declares `server-only`.
  → Move server-only work behind a route handler/server module
    and pass serializable data to the client.
```

### Known Server-Only Packages

The boundary checker flags these specifiers as server-only:

```rust
fn is_server_only_specifier(specifier: &str) -> bool {
    matches!(
        specifier,
        "server-only" | "@ruvyxa/auth" | "@ruvyxa/database"
    )
}
```

Any module that imports these packages inherits the server-only restriction.

### The `client-only` Package

The inverse guard prevents server code from accidentally importing browser-only code:

```tsx
// app/lib/browser-api.ts
import 'client-only'

export function getLocalStorageItem(key: string): string | null {
  return localStorage.getItem(key)
}
```

If a server component imports `browser-api.ts`:

```
RUV1009: Client-only module imported into server graph
  This module is reachable from server runtime code but declares `client-only`.
  → Move browser-only code into a client component or client.tsx module.
```

### The `server/` Directory

Files inside `server/` directories are automatically server-only.

```
app/
  server/
    db.ts          ← auto-enforced server-only
    auth.ts        ← auto-enforced server-only
  utils/
    format.ts      ← safe to import anywhere
```

**Path detection:** The check works at the project root level. A file is considered under `server/`
if its path relative to the project root starts with the `server/` component:

```rust
fn relative_starts_with_server(relative: &Path) -> bool {
    let first = relative.components().next();
    matches!(first, Some(Component::Normal(name)) if name == "server")
}
```

`server/` directories at any depth in the project tree are enforced, including:

- `app/server/` — whole-tree server-only
- `server/` — project-root server-only
- `app/blog/server/` — nested server-only

If a client file imports from `server/`, Ruvyxa raises:

```
RUV1010: Server directory module reached by client graph
  Files under server/ are reserved for server-only code.
  → Move shared browser-safe code outside server/, or import it from a server route only.
```

---

## Private Environment Variable Detection

Ruvyxa detects `process.env.*` reads in client code using a regex scanner:

```rust
fn private_env_reads(source: &str) -> Vec<String> {
    // Pattern: process.env.<NAME> or process.env['NAME'] or process.env["NAME"]
    // Excludes RUVYXA_PUBLIC_* and NODE_ENV
}
```

### Allowed Env Vars in Client Code

| Variable                          | Allowed in client? | Notes                                       |
| --------------------------------- | ------------------ | ------------------------------------------- |
| `process.env.RUVYXA_PUBLIC_*`     | ✅ Yes             | Any variable prefixed with `RUVYXA_PUBLIC_` |
| `process.env.NODE_ENV`            | ✅ Yes             | Always safe (replaced at build time)        |
| `process.env.*` (any other)       | ❌ No              | Fires `RUV1008`                             |
| `import.meta.env.RUVYXA_PUBLIC_*` | ✅ Yes             | Vite-style public env vars                  |

### RUV1008 Error

```
RUV1008: Private environment variable used in client graph
  `process.env.DATABASE_URL` is reachable from browser code.
  Only `RUVYXA_PUBLIC_*` env vars may be exposed to client modules.
  → Move the env read into server-only code or rename it to
    `RUVYXA_PUBLIC_*` if it is safe to expose.
```

---

## Boundary Validation

Ruvyxa validates the server-client boundary at build time and during `dev`.

| Error Code | Message                                           | Cause                                               | Fix                                           |
| ---------- | ------------------------------------------------- | --------------------------------------------------- | --------------------------------------------- |
| `RUV1007`  | Server-only module imported into client graph     | Client file imports `server-only` or guarded module | Move import to server scope                   |
| `RUV1008`  | Private environment variable used in client graph | `process.env.SECRET` reachable from browser         | Rename to `RUVYXA_PUBLIC_*` or move to server |
| `RUV1009`  | Client-only module imported into server graph     | Server file imports `client-only` guarded module    | Move to client component                      |
| `RUV1010`  | Server directory module reached by client graph   | Client imports from `server/` directory             | Move outside `server/` or keep on server      |

### How Validation Works

The validator performs a **static graph walk** for each route:

1. Start with the route's page file
2. Collect all relative imports recursively (`.`, `..` relative specifiers only)
3. Include the layout chain's imports
4. For each module in the graph:
   - Check for `server-only` / `@ruvyxa/auth` / `@ruvyxa/database` imports → `RUV1007`
   - Check for `process.env.*` reads (non-public) → `RUV1008`
   - Check if file is under `server/` dir → `RUV1010`
5. For API routes and server modules:
   - Check for `client-only` imports → `RUV1009`

The graph walk is **BFS** (breadth-first) via `VecDeque`. Only relative imports are followed; npm
packages are checked by their specifier name.

### Example: Fixing RUV1008

```tsx
// ❌ This fails: process.env.DATABASE_URL in client-reachable code
'use client'

const dbUrl = process.env.DATABASE_URL // RUV1008

export function DbStatus() {
  return <p>DB: {dbUrl}</p>
}
```

Fix: pass as prop from server:

```tsx
// ✅ Works: env var read on server, passed as JSON-safe prop
// app/page.tsx (server)
import { DbStatus } from './DbStatus'

export default function Page() {
  const dbUrl = process.env.DATABASE_URL // safe on server
  return <DbStatus dbUrl={dbUrl} />
}

// app/DbStatus.tsx (client)
;('use client')

export function DbStatus({ dbUrl }: { dbUrl: string }) {
  return <p>DB: {dbUrl}</p> // dbUrl is just a string prop — safe
}
```

### Example: Fixing RUV1007

```tsx
// ❌ This fails: client imports server-only module
'use client'
import { db } from './server/db' // RUV1007

// ✅ Fix: import from a safe location or pass data as props
// app/page.tsx (server)
import { db } from './server/db'
import { ClientList } from './ClientList'

export default async function Page() {
  const users = await db.query('SELECT * FROM users')
  return <ClientList users={users} /> // data passes as serializable prop
}
```

### MDX/Markdown Handling

The validator has special handling for MDX files: it **blanks out fenced code blocks** and **inline
code spans** so that example code showing `import 'server-only'` or `process.env.SECRET` in
documentation doesn't trigger false positives.

````rust
fn markdown_without_code_examples(source: &str) -> String {
    // Blanks content inside ``` fences and `inline code`
    // Preserves import/export statements outside fences
    // Keeps newlines for position accuracy
}
````

---

## `hydrate` Prop

Control **when** a client component hydrates. Useful for deferring non-critical interactivity.

```tsx
'use client'

import { HeavyChart } from './HeavyChart'

export default function Page() {
  return (
    <div>
      <h1>Dashboard</h1>

      {/* Hydrate immediately (default) */}
      <HeavyChart hydrate="load" />

      {/* Hydrate after browser idle */}
      <HeavyChart hydrate="idle" />

      {/* Hydrate when scrolled into viewport */}
      <HeavyChart hydrate="visible" />

      {/* No hydration — pure server HTML, no JS */}
      <HeavyChart hydrate={false} />
    </div>
  )
}
```

### Hydrate Values

| Value              | When JS runs                 | Implementation                | Use case                     |
| ------------------ | ---------------------------- | ----------------------------- | ---------------------------- |
| `"load"` (default) | Immediately                  | Normal hydration after render | Critical interactivity       |
| `"idle"`           | After browser idle           | `requestIdleCallback`         | Non-urgent UI, analytics     |
| `"visible"`        | When element enters viewport | `IntersectionObserver`        | Below-the-fold content       |
| `false` / `"none"` | Never                        | No hydration bundle shipped   | Static content, no JS needed |

### HydrationMode Type

From the Rust side:

```rust
pub enum HydrationMode {
    Load,    // Default — hydrate as soon as module arrives
    Idle,    // hydrate via requestIdleCallback
    Visible, // hydrate via IntersectionObserver
    None,    // No client bundle for this route
}
```

Parsed from page source:

```rust
fn parse_hydration_mode(source: &str) -> HydrationMode {
    // Looks for: export const hydrate = <value>
    // "false" | "none" → HydrationMode::None
    // "idle"           → HydrationMode::Idle
    // "visible"        → HydrationMode::Visible
    // anything else    → HydrationMode::Load (default)
}
```

### Zero-JS Pages

Set `hydrate={false}` to ship **zero JavaScript** for a component. Combine with SSG for truly static
pages.

```tsx
// app/docs/page.tsx
import { TableOfContents } from './TableOfContents'

export default function DocsPage() {
  return (
    <div>
      <h1>Documentation</h1>
      <TableOfContents hydrate={false} /> {/* 0 JS */}
    </div>
  )
}
```

### Hydration Inheritance

When `hydrate` is set on a parent component or route:

- `hydrate={false}` at the route level applies to **all client components** in that route
- Individual components can override with `hydrate="load"`, `"idle"`, `"visible"`
- Route-level `hydrate` does NOT affect CSR pages (`'use client'` pages always hydrate)

```tsx
// app/layout.tsx — route-level hydrate setting
export const hydrate = "idle"; // all client components in this route defer hydration

// app/page.tsx — individual override
<HeavyChart hydrate="load" /> {/* This one hydrates immediately */}
```

---

## Route-Level Hydration Export

Export `hydrate` from a page to control hydration scheduling:

```tsx
// app/dashboard/page.tsx
export const hydrate = 'idle' // or "load", "visible", false, "none"
```

This affects the entire route's client bundle scheduling.

---

## Composition Rules

### Server imports Client (✅ Allowed)

```tsx
// server.tsx
import { ClientWidget } from './ClientWidget' // ✅
export default function Page() {
  return <ClientWidget />
}
```

### Client imports Server (❌ Not possible)

Everything imported by a `'use client'` file becomes client code. Server components cannot be
rendered inside client components.

Instead, pass server-rendered content as `children`:

```tsx
// ✅ Correct pattern
// app/page.tsx (server)
import { ClientShell } from './ClientShell'
import { ServerContent } from './ServerContent'

export default function Page() {
  return (
    <ClientShell>
      <ServerContent /> {/* server-rendered, passed as children */}
    </ClientShell>
  )
}
```

---

## When to Use Each

### Choose Server Component when:

- Rendering static or database-driven content
- You need to access server-only APIs (DB, env, filesystem)
- The component has no interactivity (no clicks, no state)
- You want to minimize JS bundle size
- You want the best possible SEO and initial load

### Choose Client Component when:

- You need `useState`, `useEffect`, `useReducer`
- You handle user events (onClick, onSubmit, onChange)
- You need browser APIs (localStorage, navigator, IntersectionObserver)
- You use hooks from `@ruvyxa/react` like `useRouter`, `usePathname`
- The component renders frequently changing data

---

## Best Practices

1. **Default to server.** Start without `'use client'`. Only add it when you need interactivity.

2. **Push the boundary down.** Make client components as leaf-like as possible — import them deep in
   the tree, not at the top.

3. **Extract client islands.** If a page has a small interactive element (like a like button),
   extract it into its own client component.

4. **Pass data, not logic.** Fetch data in the server component and pass it as props to client
   components.

5. **Use `'use client'` sparingly.** Every client component adds JS to the bundle.

6. **Guard server code.** Use `server-only` or the `server/` directory for database and auth code.

7. **Compose, don't convert.** Don't slap `'use client'` on a whole page just because one button
   needs interactivity. Extract the button.

8. **Use `hydrate` for deferred hydration.** Non-critical UI can use `hydrate="idle"` or
   `hydrate="visible"` to reduce initial JS.

9. **Keep props serializable.** When crossing the boundary, only pass JSON-safe values.

---

## Performance: Cost of Each `'use client'` Boundary

| Metric                  | Server Component      | Client Component                              |
| ----------------------- | --------------------- | --------------------------------------------- |
| Bundle size contributed | 0 bytes               | All imports added to client bundle            |
| Parse time              | 0                     | Bundle must be parsed in browser              |
| Hydration cost          | 0                     | Component tree must be reconciled             |
| Network transfer        | 0                     | Bundle downloaded over network                |
| Memory (server)         | Module kept in memory | Module also in browser memory                 |
| SEO                     | Full HTML content     | HTML present (SSR) but interactivity needs JS |

**Rule of thumb:** A single `'use client'` boundary at a leaf component is usually free. A
`'use client'` at a page level bundles the entire page for the client.

---

## Try It Yourself

Build this page structure:

```
app/
├── page.tsx              ← server component (default)
├── components/
│   ├── LikeButton.tsx    ← client component
│   └── CommentForm.tsx   ← client component
├── server/
│   └── db.ts             ← server-only
```

**Step 1:** Create `app/server/db.ts`:

```ts
import 'server-only'

export async function getPosts() {
  return [{ id: 1, title: 'Hello', likes: 10 }]
}
```

**Step 2:** Create `app/components/LikeButton.tsx`:

```tsx
'use client'

import { useState } from 'react'

export function LikeButton({ initialLikes }: { initialLikes: number }) {
  const [likes, setLikes] = useState(initialLikes)

  return <button onClick={() => setLikes((l) => l + 1)}>♥ {likes}</button>
}
```

**Step 3:** Create `app/page.tsx`:

```tsx
import { getPosts } from './server/db'
import { LikeButton } from './components/LikeButton'

export default async function HomePage() {
  const posts = await getPosts()

  return (
    <main>
      {posts.map((post) => (
        <div key={post.id}>
          <h2>{post.title}</h2>
          <LikeButton initialLikes={post.likes} />
        </div>
      ))}
    </main>
  )
}
```

Result: the post list is server-rendered HTML. Each ♥ button is a tiny client island. No JS is
shipped for the post list itself.

---

## Next Steps

- **[04-rendering-strategies.md](./04-rendering-strategies.md)** — SSR, SSG, ISR, PPR, CSR
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — Fetch data and cache it
- **[06-server-actions.md](./06-server-actions.md)** — Server actions for mutations
