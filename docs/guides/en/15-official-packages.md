# Official Packages

This guide documents the public APIs currently exported by the first-party packages in this
repository. Examples are intentionally limited to APIs present in `src/` and the package export
maps.

## Package boundaries

| Package            | Server entry       | Browser entry             | Main responsibility                         |
| ------------------ | ------------------ | ------------------------- | ------------------------------------------- |
| `@ruvyxa/auth`     | `@ruvyxa/auth`     | `@ruvyxa/auth/client`     | Sessions and provider-driven authentication |
| `@ruvyxa/database` | `@ruvyxa/database` | None                      | Database adapter contract and typed facade  |
| `@ruvyxa/realtime` | `@ruvyxa/realtime` | `@ruvyxa/realtime/client` | Action-driven WebSocket capability          |

The server entries must not be imported into a client bundle. The browser-safe `/client` entries
contain only the corresponding browser helpers.

## `@ruvyxa/auth`

The auth runtime requires an application secret, canonical origin, durable stores in production, and
an explicit provider map. Built-in OAuth helpers are Google and GitHub; magic-link and WebAuthn are
provider interfaces that the application supplies.

```ts
import { config } from 'ruvyxa/config'
import { createAuth, google, memoryAuthStore, memoryRateLimitStore } from '@ruvyxa/auth'

const auth = createAuth({
  secret: process.env.AUTH_SECRET!, // at least 32 characters
  origin: 'http://localhost:3000',
  store: memoryAuthStore({ development: true }),
  rateLimitStore: memoryRateLimitStore({ development: true }),
  providers: {
    credentials: {
      type: 'credentials',
      async authorize(input) {
        if (input.email === 'demo@example.com' && input.password === 'demo') {
          return { id: 'demo', email: 'demo@example.com' }
        }
        return null
      },
    },
    google: google({
      clientId: process.env.GOOGLE_CLIENT_ID!,
      clientSecret: process.env.GOOGLE_CLIENT_SECRET!,
    }),
  },
})

export default config({ plugins: [auth.plugin] })
```

`memoryAuthStore` and `memoryRateLimitStore` require `{ development: true }` and are for tests or
development. A production build validates that both stores are durable. The options supported by
`createAuth` include `basePath`, `session.ttlSeconds`, `session.rememberTtlSeconds`,
`session.cookieName`, `session.secure`, `session.sameSite`, `rateLimit`, `clientIp`, and `onError`.

### Browser client

```ts
import { createAuthClient } from '@ruvyxa/auth/client'

const authClient = createAuthClient({ basePath: '/__ruvyxa/auth' })
await authClient.login('credentials', { email: 'demo@example.com', password: 'demo' })
const session = await authClient.session()
authClient.oauth('google', '/account')
await authClient.logout()
```

The client methods are `login`, `logout`, `session`, and `oauth`; there is no `useRealtime` or
`signIn` API in this package.

## `@ruvyxa/database`

The database package exposes one facade over one adapter. The public built-in adapters are
`prismaAdapter` and `dynamoAdapter`, both imported from the package root.

```ts
import { createDatabase, prismaAdapter } from '@ruvyxa/database'
import { PrismaClient } from '@prisma/client'

const prisma = new PrismaClient()
export const db = createDatabase(prismaAdapter(prisma))

const users = await db.users.findMany({
  where: { active: true },
  orderBy: { createdAt: 'desc' },
  take: 20,
})
const user = await db.users.create({ data: { email: 'alice@example.com', active: true } })
await db.users.update({ where: { id: 'user_123' }, data: { active: false } })
```

There is no `@ruvyxa/database/prisma` or `@ruvyxa/database/dynamodb` export in the current package
map. The Dynamo adapter accepts an explicit `transport` and model-to-table `tables` map; the
transport can wrap an AWS SDK client or another implementation of the `execute(operation)` contract.
Custom implementations can be validated with `defineDatabaseAdapter`.

The database plugin is separate from the adapter facade and validates private required environment
variables during build:

```ts
import { databasePlugin } from '@ruvyxa/database/plugin'

databasePlugin({ requiredEnv: ['DATABASE_URL'] })
```

## `@ruvyxa/realtime`

Realtime is an action-driven native capability. Register the root plugin and mark actions with the
core `action.realtime()` builder. The plugin validates that the selected build target can provide a
long-lived Node/Bun WebSocket runtime; it does not expose a server `createRealtime()` or `publish()`
API.

```ts
import { config } from 'ruvyxa/config'
import { realtime } from '@ruvyxa/realtime'

export default config({ plugins: [realtime()] })
```

```ts
import { action } from '@ruvyxa/core/server'

export const sendMessage = action
  .input({
    parse(value: unknown) {
      if (!value || typeof value !== 'object') throw new Error('Invalid message')
      return value as { text: string }
    },
  })
  .realtime('chat:general')
  .handler(async ({ input }) => ({ ok: true, text: input.text }))
```

### Browser client

```ts
import { createRealtimeClient } from '@ruvyxa/realtime/client'

const client = createRealtimeClient({ url: '/__ruvyxa/realtime' })
const unsubscribe = client.subscribe('chat:general', (event) => {
  if (event.type === 'action') console.log(event.action, event.invalidated)
})
// Later: unsubscribe(); client.close()
```

The client supports `subscribe`, `subscribeRoute`, and `close`, with bounded reconnect behavior.
Platform adapters that cannot provide the required long-lived runtime reject the realtime plugin at
build completion.

## Versioning

This source tree uses one repository release version for the first-party packages. The checked-in
package manifests currently report `1.0.26` for `ruvyxa`, `@ruvyxa/auth`, `@ruvyxa/database`,
`@ruvyxa/realtime`, and the first-party adapter packages. Built-in plugins are not separately
versioned or split into independent plugin versions; they follow the `ruvyxa` package version.

`realtime@1` is a native capability/protocol identifier, not a package version.

## Evidence and limits

Provider behavior, platform limits, credentials, and deployment health are external concerns. Verify
them against the provider documentation and the generated adapter artifacts for the chosen target;
this guide does not claim a benchmark, ROI, partner commitment, or automatic production deployment.

---

## Production contract and retained detail

The section above is the current, source-backed contract for this release. The original long-form
draft is retained below to preserve instructional context and audit history. It is non-normative: do
not copy its API snippets or capability claims unless they are revalidated against the current
source and package export map. This boundary is intentional so the document can retain its original
depth without presenting unsupported historical design as production behavior.

### English package draft — historical draft (non-normative)

> **Archive warning:** The material below is retained for history only. It is not the current
> package API; examples may be stale or unsupported and must not be copied as working code. The
> source-backed contract above is authoritative.

# Official Packages

Ruvyxa ships three official packages solving common full-stack problems: authentication, databases,
real-time communication. Each follows the same design philosophy — server-first, typed APIs,
zero-config defaults, and plugin integration.

---

## What You Will Learn

- `@ruvyxa/auth` — sessions, OAuth (Google, GitHub, Discord), PKCE, magic-link, WebAuthn
- `@ruvyxa/database` — typed CRUD with Prisma, DynamoDB, custom adapters
- `@ruvyxa/realtime` — WebSocket transport for server actions
- Import rules: server-only vs browser-safe exports (RUV1007 enforcement)
- Plugin integration for each package
- Complete TypeScript type definitions
- Error codes: RUV3100-3105 (auth), RUV3001-3003 (database), RUV3201 (realtime)

---

## Import Rules

```
┌─────────────────────────────────────┐
│          Server Bundle              │
│                                     │
│  @ruvyxa/auth          ✅          │
│  @ruvyxa/auth/client   ✅          │
│  @ruvyxa/database      ✅          │
│  @ruvyxa/database/client  ❌ (does not exist) │
│  @ruvyxa/realtime      ✅          │
│  @ruvyxa/realtime/client ✅         │
└─────────────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│          Client Bundle              │
│                                     │
│  @ruvyxa/auth          ❌ RUV1007  │
│  @ruvyxa/auth/client   ✅          │
│  @ruvyxa/database      ❌ RUV1007  │
│  @ruvyxa/realtime      ❌ RUV1007  │
│  @ruvyxa/realtime/client ✅         │
└─────────────────────────────────────┘
```

```typescript
// ✅ Server only — import from root
import { createAuth } from '@ruvyxa/auth'
import { createDatabase } from '@ruvyxa/database'
import { createRealtime } from '@ruvyxa/realtime'

// ✅ Browser safe — import from /client subpath
import { createAuthClient } from '@ruvyxa/auth/client'
import { useRealtime } from '@ruvyxa/realtime/client'

// ❌ RUV1007 — root package in client bundle
// import { createAuth } from "@ruvyxa/auth" // in a 'use client' file

// ❌ Errors
// import { createDatabase } from "@ruvyxa/auth/client"   // does not exist
// import { createAuth } from "@ruvyxa/database/client"    // does not exist
```

---

## @ruvyxa/auth

Authentication with sessions, OAuth providers, magic-link, and WebAuthn passkeys. Server-only API
with a thin browser client.

### Installation

```bash
npm install @ruvyxa/auth
```

### Type Definitions

```typescript
// === Core Types ===

interface AuthOptions {
  /** Required — used for session encryption. */
  secret: string
  /** Session configuration. */
  session?: SessionConfig
  /** Authentication providers. */
  providers?: AuthProviders
  /** Session store. Defaults to cookie-based JWT. */
  store?: SessionStore
  /** Rate-limit store. Defaults to in-memory. */
  rateLimitStore?: RateLimitStore
  /** Base path for auth routes. @default '/api/auth' */
  basePath?: string
  /** Site origin for CSRF protection. */
  origin?: string
  /** Session cookie configuration. */
  cookies?: CookieConfig
  /** Error handler. */
  onError?: (error: unknown, request: Request) => void | Promise<void>
  /** Allow insecure HTTP for local dev. @default false */
  allowInsecure?: boolean
  /** Maximum authentication body bytes. @default 32768 */
  maxBodyBytes?: number
}

interface SessionConfig {
  /** Session strategy. @default 'jwt' */
  strategy?: 'jwt' | 'database'
  /** Session max age in seconds. @default 604800 (7 days) */
  maxAge?: number
  /** Update session expiry on each request. @default true */
  updateAge?: boolean
}

interface CookieConfig {
  /** Cookie name. @default '__session' */
  name?: string
  /** Cookie domain. */
  domain?: string
  /** Cookie path. @default '/' */
  path?: string
  /** SameSite policy. @default 'lax' */
  sameSite?: 'strict' | 'lax' | 'none'
  /** Secure flag. @default true in production */
  secure?: boolean
  /** HTTP only. @default true */
  httpOnly?: boolean
}

// === Session ===

interface AuthSession {
  user: AuthUser
  expiresAt: number // Unix timestamp
}

interface AuthUser {
  id: string
  email: string
  name?: string
  image?: string
  emailVerified?: boolean
}

// === Auth Runtime ===

interface AuthRuntime {
  /** Get current session from request. */
  getSession(request?: Request): Promise<AuthSession | null>
  /** Create a session for a user. */
  createSession(user: AuthUser): Promise<{ session: AuthSession; headers: HeadersInit }>
  /** Destroy session. */
  destroySession(request: Request): Promise<HeadersInit>
  /** Send magic link email. */
  sendMagicLink(options: { email: string }): Promise<void>
  /** Get the plugin for ruvyxa.config.ts. */
  plugin: RuvyxaPlugin
  /** Direct request handler. */
  handle(request: Request): Promise<Response | undefined>
}

// === Session Store Interface ===

interface SessionStore {
  /** Whether this store persists across restarts. */
  durable: boolean
  get(key: string): Promise<AuthSession | null>
  set(key: string, session: AuthSession, ttl: number): Promise<void>
  delete(key: string): Promise<void>
}

// === Rate Limit Store ===

interface RateLimitStore {
  durable: boolean
  increment(key: string, window: number): Promise<{ count: number; ttl: number }>
  reset(key: string): Promise<void>
}

// === Providers ===

interface AuthProviders {
  google?: OAuthProvider
  github?: OAuthProvider
  discord?: OAuthProvider
  /** Generic OAuth 2.0 provider. */
  oauth?: CustomOAuthProvider
  /** PKCE-based OAuth. */
  pkce?: PkceProvider
  magicLink?: MagicLinkConfig
  webAuthn?: WebAuthnConfig
  /** Custom async authentication function. */
  credentials?: CredentialsProvider
}

interface OAuthProvider {
  clientId: string
  clientSecret: string
  /** Additional scopes. */
  scope?: string[]
  /** Custom authorize URL. */
  authorizeUrl?: string
  /** Custom token URL. */
  tokenUrl?: string
  /** Custom profile URL. */
  profileUrl?: string
  /** Map provider profile to AuthUser. */
  profile?(profile: Record<string, unknown>): AuthUser
}

interface CustomOAuthProvider {
  clientId: string
  clientSecret: string
  authorizeUrl: string
  tokenUrl: string
  profileUrl: string
  scope?: string[]
  profile(profile: Record<string, unknown>): AuthUser
}

interface PkceProvider {
  clientId: string
  clientSecret: string
  authorizeUrl: string
  tokenUrl: string
  profileUrl: string
  codeChallengeMethod?: 'S256' | 'plain'
  profile(profile: Record<string, unknown>): AuthUser
}

interface MagicLinkConfig {
  secret: string
  /** Link expiry in seconds. @default 900 */
  expiresIn?: number
  /** Custom email sender. Default sends via fetch to a configured endpoint. */
  send?(email: string, url: string): Promise<void>
}

interface WebAuthnConfig {
  rpName: string
  rpId: string
  /** Origin override. Defaults to request origin. */
  origin?: string
}

interface CredentialsProvider {
  authorize(credentials: Record<string, unknown>): Promise<AuthUser | null>
}

// === Auth Client (browser) ===

interface AuthClient {
  /** Sign in with OAuth provider. */
  signIn(provider: string, options?: SignInOptions): Promise<void>
  /** Sign out current session. */
  signOut(): Promise<void>
  /** Get current session. */
  getSession(): Promise<AuthSession | null>
  /** WebAuthn passkey methods. */
  webAuthn: {
    register(options?: CredentialCreationOptions): Promise<void>
    authenticate(options?: CredentialRequestOptions): Promise<void>
  }
  /** Magic link sign in. */
  signIn(provider: 'magic-link', options: { email: string }): Promise<void>
  /** Listen to session changes. */
  onChange(callback: (session: AuthSession | null) => void): () => void
  /** Exchange an auth code from a third-party provider. */
  exchangeCode(provider: string, code: string, state: string): Promise<AuthSession>
}

interface SignInOptions {
  /** Redirect URL after sign in. @default '/' */
  returnTo?: string
  /** PKCE code challenge. */
  codeChallenge?: string
}

function createAuth(options: AuthOptions): AuthRuntime
function createAuthClient(): AuthClient
```

### Server API — Full Reference

```typescript
// app/lib/auth.ts (server only)
import { createAuth } from '@ruvyxa/auth'

export const auth = createAuth({
  secret: process.env.AUTH_SECRET, // required
  session: {
    strategy: 'jwt', // 'jwt' | 'database'
    maxAge: 7 * 86400, // 7 days
    updateAge: true,
  },
  providers: {
    google: {
      clientId: process.env.GOOGLE_CLIENT_ID,
      clientSecret: process.env.GOOGLE_CLIENT_SECRET,
      scope: ['openid', 'profile', 'email'],
    },
    github: {
      clientId: process.env.GITHUB_CLIENT_ID,
      clientSecret: process.env.GITHUB_CLIENT_SECRET,
      scope: ['read:user', 'user:email'],
    },
    discord: {
      clientId: process.env.DISCORD_CLIENT_ID,
      clientSecret: process.env.DISCORD_CLIENT_SECRET,
      scope: ['identify', 'email'],
    },
    magicLink: {
      secret: process.env.MAGIC_LINK_SECRET,
      expiresIn: 600, // 10 minutes
    },
    webAuthn: {
      rpName: 'My App',
      rpId: 'example.com',
    },
  },
  cookies: {
    name: '__session',
    sameSite: 'lax',
    secure: true,
    httpOnly: true,
  },
  basePath: '/api/auth',
  allowInsecure: process.env.NODE_ENV === 'development',
})
```

### Session Handling

```typescript
// Server component
import { auth } from '../../lib/auth'

export default async function DashboardPage() {
  const session = await auth.getSession()
  if (!session) return <p>Please sign in</p>
  return <p>Welcome, {session.user.name}!</p>
}

// API route
export async function GET(request: Request) {
  const session = await auth.getSession(request)
  return Response.json({ user: session?.user ?? null })
}

// Middleware
export async function middleware(request: Request) {
  const session = await auth.getSession(request)
  if (!session) {
    return Response.redirect(new URL('/api/auth/login/google', request.url))
  }
}
```

### OAuth Flow — Detailed

```
User clicks "Sign in with Google"
        │
        ▼
GET /api/auth/login/google  (or via client: authClient.signIn('google'))
        │
        ▼
Server generates PKCE challenge + state
        │
        ▼
302 Redirect to Google OAuth consent screen
  ?client_id=...
  &redirect_uri=/api/auth/callback/google
  &response_type=code
  &scope=openid+profile+email
  &state=<random>
  &code_challenge=<sha256>
        │
        ▼
User approves → Google redirects to /api/auth/callback/google?code=<x>&state=<y>
        │
        ▼
Server validates state matches stored value
        │
        ▼
POST to Google token endpoint with code + code_verifier
        │
        ▼
Receive access_token + id_token
        │
        ▼
GET Google profile endpoint with access_token
        │
        ▼
Map profile to AuthUser, create session
        │
        ▼
Set session cookie, redirect to returnTo URL
```

### Client API

```typescript
// app/components/AuthButtons.tsx
'use client'

import { createAuthClient } from '@ruvyxa/auth/client'

const authClient = createAuthClient()

export function SignInButtons() {
  return (
    <div>
      <button onClick={() => authClient.signIn('google', { returnTo: '/dashboard' })}>
        Sign in with Google
      </button>
      <button onClick={() => authClient.signIn('github')}>
        Sign in with GitHub
      </button>
    </div>
  )
}

export function SignOutButton() {
  return <button onClick={() => authClient.signOut()}>Sign out</button>
}

export function SessionDisplay() {
  const [session, setSession] = useState<AuthSession | null>(null)

  useEffect(() => {
    authClient.getSession().then(setSession)
    const unsub = authClient.onChange(setSession)
    return () => unsub()
  }, [])

  return <p>{session ? `Hello ${session.user.name}` : 'Not signed in'}</p>
}
```

### Magic Link — Full API

```typescript
// Server — send
await auth.sendMagicLink({ email: 'user@example.com' })

// Client — request
;('use client')
import { createAuthClient } from '@ruvyxa/auth/client'
const authClient = createAuthClient()
await authClient.signIn('magic-link', { email: 'user@example.com' })
// Email sent with link: /api/auth/magic-link/callback?token=<jwt>&email=user@example.com

// Magic link lifecycle:
// 1. POST /api/auth/magic-link → generates JWT token, sends email
// 2. GET /api/auth/magic-link/callback → shows confirmation page (does NOT consume token)
// 3. POST /api/auth/magic-link/callback → consumes token, creates session

// Security: GET requests do NOT consume the magic link token
// (email security scanners prefetch links, so a consuming GET would
//  burn the token before the user clicks)
```

### WebAuthn (Passkeys)

```typescript
'use client'
import { createAuthClient } from '@ruvyxa/auth/client'

const authClient = createAuthClient()

// Register a passkey
async function registerPasskey() {
  try {
    await authClient.webAuthn.register()
    console.log('Passkey registered')
  } catch (err) {
    if (err instanceof DOMException && err.name === 'NotAllowedError') {
      console.log('User cancelled passkey registration')
    } else if (err instanceof DOMException && err.name === 'NotSupportedError') {
      console.log('Browser does not support WebAuthn (RUV-AUTH-004)')
    }
  }
}

// Sign in with passkey
async function signInWithPasskey() {
  try {
    await authClient.webAuthn.authenticate()
    console.log('Signed in with passkey')
  } catch (err) {
    console.error('Passkey authentication failed:', err)
  }
}
```

### Auth Plugin

```typescript
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'

export default config({
  plugins: [
    {
      name: '@ruvyxa/auth/plugin',
      options: {
        authRoutes: {
          signIn: '/auth/signin',
          callback: '/auth/callback',
          signOut: '/auth/signout',
        },
      },
    },
  ],
})
```

The plugin automatically creates API routes for OAuth callbacks, sign-out, and session management.
Route paths default to `/api/auth/*` and can be customized via `basePath` in `createAuth()`.

### Session Stores

```typescript
// Built-in: JWT (cookie-based, no server-side store needed)
// strategy: 'jwt' — session encrypted in cookie, no DB required

// Built-in: Database store (requires @ruvyxa/database or Prisma)
// strategy: 'database' — session stored in DB, cookie only has session ID

// Custom store
import type { SessionStore } from '@ruvyxa/auth'

const redisStore: SessionStore = {
  durable: true, // survives restarts
  async get(key) {
    const data = await redis.get(`session:${key}`)
    return data ? JSON.parse(data) : null
  },
  async set(key, session, ttl) {
    await redis.set(`session:${key}`, JSON.stringify(session), { EX: ttl })
  },
  async delete(key) {
    await redis.del(`session:${key}`)
  },
}

export const auth = createAuth({
  secret: process.env.AUTH_SECRET,
  store: redisStore,
})
```

### Auth Request Dispatch — Under the Hood

```
POST /api/auth/session       → getSession()     → returns current session
POST /api/auth/logout        → logout()          → destroys session, clears cookie
POST /api/auth/login/<name>  → login()           → validates credentials, creates session
GET  /api/auth/login/<name>  → startOAuth()      → redirects to OAuth provider (Google, GitHub, Discord)
GET  /api/auth/callback/<n>  → finishOAuth()     → exchanges code, creates session
POST /api/auth/magic-link    → startMagicLink()  → generates JWT token, sends email
GET  /api/auth/magic-link/callback → magicLinkConfirmPage() → shows confirmation (no token consume)
POST /api/auth/magic-link/callback → finishMagicLink() → consumes token, creates session
```

All paths prefixed with `basePath` (default `/api/auth`). Requests to unknown paths return
`undefined`, letting other route handlers or plugins process them.

**Dispatch logic** (`packages/@ruvyxa/auth/src/index.ts:64-100`):

1. `GET /session` → return JSON of `getSession(request)`
2. `POST /logout` → assertSameOrigin, destroy session
3. `POST /login/*` → assertSameOrigin, read JSON body, call `login()`
4. `GET /login/google|github|discord` → start OAuth flow (redirect)
5. `GET /callback/google|github|discord` → finish OAuth (code exchange)
6. `POST /magic-link` → assertSameOrigin, generate + send magic link
7. `GET /magic-link/callback` → show confirmation page (GET never consumes token)
8. `POST /magic-link/callback` → assertSameOrigin, consume token, create session

`assertSameOrigin` checks `Origin` header matches configured `origin` or falls back to
`X-Forwarded-Proto` / `Host` headers. This prevents CSRF attacks on auth endpoints.

### Session Cookie — JWT Format

```typescript
// JWT session structure (strategy: 'jwt')
interface JwtSessionPayload {
  user: AuthUser
  iat: number // issued at (Unix seconds)
  exp: number // expires at (Unix seconds)
}

// Cookie: __session=<base64url-encoded-jwt>
//
// JWT is:
//   1. JSON serialize { user, iat, exp }
//   2. Encrypt with AUTH_SECRET (AES-256-GCM)
//   3. Base64url encode
//   4. Set as cookie
//
// Cookie attributes:
//   HttpOnly — cannot be read by JavaScript
//   Secure — only over HTTPS (configurable)
//   SameSite=Lax — protects against CSRF
//   Path=/ — sent with every request
```

**JWT strategy**: No server-side storage needed. Session is encrypted in the cookie. Every request
decrypts the cookie to read session. To revoke sessions, use database strategy or add a deny list.

**Database strategy**: Cookie contains only a session ID (`sid=<random>`). Full session data stored
in `SessionStore`. To revoke, delete from store. Requires database adapter.

### PKCE OAuth Flow

```typescript
// PKCE (Proof Key for Code Exchange) — more secure than basic OAuth
// Used by default for all OAuth providers. The flow:

// Step 1: Client generates code_verifier + code_challenge (SHA-256)
const verifier = generateRandomString(64) // /[a-zA-Z0-9._~-]{43,128}/
const challenge = await sha256(verifier) // base64url(sha256(verifier))

// Step 2: Authorization URL includes code_challenge
// GET https://accounts.google.com/o/oauth2/v2/auth
//   ?client_id=...
//   &code_challenge=<base64url(sha256(verifier))>
//   &code_challenge_method=S256

// Step 3: Callback receives code, server sends code + verifier to token endpoint
// POST https://oauth2.googleapis.com/token
//   ?client_id=...
//   &code=<received_code>
//   &code_verifier=<verifier>  // stored server-side during step 2
```

PKCE prevents authorization code interception attacks. Even if the code is intercepted, the attacker
cannot exchange it without the `code_verifier`.

### Credentials Provider

```typescript
export const auth = createAuth({
  secret: process.env.AUTH_SECRET,
  providers: {
    credentials: {
      async authorize(credentials) {
        const { email, password } = credentials as { email: string; password: string }
        const user = await db.user.findUnique({ where: { email } })
        if (!user) return null
        const valid = await bcrypt.compare(password, user.passwordHash)
        if (!valid) return null
        return { id: user.id, email: user.email, name: user.name }
      },
    },
  },
})

// Client
await authClient.signIn('credentials', { email, password })
```

### Auth Edge Cases

| Scenario                       | Behavior                                               |
| ------------------------------ | ------------------------------------------------------ |
| Session cookie missing         | `getSession()` returns null                            |
| Session cookie expired         | JWT validation fails, cookie cleared, returns null     |
| Session cookie tampered        | AES-GCM decryption fails, cookie cleared, returns null |
| Cross-origin auth request      | Blocked with RUV3101 (403)                             |
| Body > 32 KiB                  | Blocked with RUV3101 (413)                             |
| Invalid JSON body              | Blocked with RUV3101 (400)                             |
| OAuth state expired            | State stored for 600s (10 min). Expired = RUV3103      |
| Magic link expired             | Token stored for 900s (15 min). Expired = RUV3103      |
| GET /magic-link/callback       | Shows confirmation page, token NOT consumed            |
| Unknown provider               | RUV3101 with 404                                       |
| Provider not configured        | RUV3101 with 401                                       |
| OAuth token endpoint down      | RUV3104 with 502                                       |
| WebAuthn unsupported           | DOMException `NotSupportedError` → RUV-AUTH-004        |
| Production + non-durable store | RUV3105 thrown at startup                              |

### Error Codes (RUV3100-3105)

| Code    | Title                  | Source (package/@ruvyxa/auth)              | Cause                                                | Fix                                  |
| ------- | ---------------------- | ------------------------------------------ | ---------------------------------------------------- | ------------------------------------ |
| RUV3100 | Auth service error     | `src/index.ts:315,708`                     | Magic link delivery failed, invalid user             | Check email service, provider config |
| RUV3101 | Auth request invalid   | `src/index.ts:299,524,531,562,740,853,857` | Cross-origin, body too large, bad JSON, bad provider | Fix request, reduce body, valid JSON |
| RUV3102 | Too many attempts      | `src/index.ts:450`                         | Rate limit exceeded (per-identity or per-client)     | Wait for `Retry-After`               |
| RUV3103 | OAuth state invalid    | `src/index.ts:241,243,246,249,366,368`     | State mismatch, expired, missing code                | Re-authenticate                      |
| RUV3104 | OAuth provider error   | `src/index.ts:263,266,281,870`             | Token/profile fetch failed                           | Check provider credentials           |
| RUV3105 | Production store error | `src/index.ts:42`                          | Non-durable store in production                      | Use persistent store                 |

**RUV3100** — `Auth service error` (line 315, 708): Magic link delivery failed (check email `send`
function or SMTP), provider returned invalid user (check `profile` mapper return value).

**RUV3101** — `Auth request invalid` (line 299, 524, 531, 562, 740, 853, 857): Cross-origin
authentication blocked (set `origin` in config or use `allowInsecure` for dev), body exceeds 32 KiB
(trim request), body must be valid JSON, provider name invalid or not configured.

**RUV3102** — `Too many authentication attempts` (line 450): a rate-limit bucket was exhausted. The
response carries `Retry-After` in seconds.

Two independent buckets are consumed per attempt:

| Bucket       | Key                       | Budget              | Stops                             |
| ------------ | ------------------------- | ------------------- | --------------------------------- |
| Per-identity | scope + identity + client | `rateLimit.max`     | Hammering one account             |
| Per-client   | client only               | `rateLimit.max` × 5 | One source sweeping many accounts |

The per-identity key contains the email, so on its own it allowed one source to try `max` passwords
against an unlimited number of accounts — the shape of credential stuffing and account enumeration.
The client-only bucket caps that total. Its larger budget keeps shared egress (offices, mobile
carriers, CGNAT) working, since a legitimate client rarely signs in for many distinct identities.

Both buckets use the same client key, which is the resolved client IP when
[`clientIp`](#authoptions) is configured and a truncated user-agent otherwise. **Configure
`clientIp` in production** — the user-agent fallback is client-controlled and therefore rotatable.

**RUV3103** — `OAuth state invalid` (line 241-249, 366-368): OAuth callback missing `code` or
`state` parameters, state does not match initiating browser (possible CSRF attempt), state expired
(default 600s TTL), magic link token missing or expired (default 900s TTL).

**RUV3104** — `OAuth provider error` (line 263, 266, 281, 870): Token endpoint returned non-200
(check provider credentials), no access token in response (provider API changed), profile endpoint
returned non-200 (rate limiting or scope issue).

**RUV3105** — `Production store error` (line 42): `createAuth` called during production build with
non-durable stores (`store.durable === false` or `rateLimitStore.durable === false`). Use Redis,
database, or other persistent stores for production.

---

## @ruvyxa/database

Typed database access with adapter system. First-class support for Prisma and DynamoDB.

### Installation

```bash
npm install @ruvyxa/database
# Plus adapter:
npm install @prisma/client  # for Prisma
npm install @aws-sdk/client-dynamodb @aws-sdk/lib-dynamodb  # for DynamoDB
```

### Type Definitions

```typescript
// === Core Types ===

interface DatabaseConfig {
  adapter: DatabaseAdapter
  /** Default query timeout in ms. @default 30000 */
  timeout?: number
  /** Maximum batch size for createMany. @default 100 */
  maxBatchSize?: number
}

interface DatabaseAdapter {
  /** Adapter name for error reporting. */
  name: string
  findMany(model: string, args?: QueryArgs): Promise<Record<string, unknown>[]>
  findUnique(
    model: string,
    args: { where: Record<string, unknown> },
  ): Promise<Record<string, unknown> | null>
  create(model: string, data: Record<string, unknown>): Promise<Record<string, unknown>>
  update(
    model: string,
    where: Record<string, unknown>,
    data: Record<string, unknown>,
  ): Promise<Record<string, unknown>>
  delete(model: string, where: Record<string, unknown>): Promise<void>
  /** Optional: batch create operation. */
  createMany?(model: string, data: Record<string, unknown>[]): Promise<Record<string, unknown>[]>
  /** Optional: raw query execution. */
  execute?(query: string, params?: unknown[]): Promise<unknown>
  /** Optional: transaction support. */
  transaction?<T>(fn: (tx: DatabaseAdapter) => Promise<T>): Promise<T>
}

// Typed database instance
type Database = {
  [model: string]: {
    findMany(args?: QueryArgs): Promise<Record<string, unknown>[]>
    findUnique(args: { where: Record<string, unknown> }): Promise<Record<string, unknown> | null>
    create(data: Record<string, unknown>): Promise<Record<string, unknown>>
    update(
      where: Record<string, unknown>,
      data: Record<string, unknown>,
    ): Promise<Record<string, unknown>>
    delete(where: Record<string, unknown>): Promise<void>
    createMany?(data: Record<string, unknown>[]): Promise<Record<string, unknown>[]>
  }
}

interface QueryArgs {
  where?: Record<string, unknown>
  select?: Record<string, boolean | Record<string, unknown>>
  include?: Record<string, boolean | Record<string, unknown>>
  orderBy?: Record<string, 'asc' | 'desc'>
  skip?: number
  take?: number
}

function createDatabase(config: DatabaseConfig): Database
```

### Setup

```typescript
// app/lib/db.ts (server only)
import { createDatabase } from '@ruvyxa/database'
import { PrismaAdapter } from '@ruvyxa/database/prisma'
import { PrismaClient } from '@prisma/client'

const prisma = new PrismaClient()

export const db = createDatabase({
  adapter: PrismaAdapter(prisma),
  timeout: 30_000,
})
```

### Usage — Full Query Reference

```typescript
// Read
const users = await db.user.findMany({
  where: { active: true },
  select: { id: true, name: true, email: true },
  orderBy: { name: 'asc' },
  skip: 0,
  take: 10,
})

const user = await db.user.findUnique({
  where: { id: 'user_123' },
  include: { posts: true },
})

// Create
const newUser = await db.user.create({
  name: 'Alice',
  email: 'alice@example.com',
})

const users = await db.user.createMany([
  { name: 'Bob', email: 'bob@example.com' },
  { name: 'Charlie', email: 'charlie@example.com' },
])

// Update
await db.user.update({ id: 'user_123' }, { name: 'Alice Updated' })

// Delete
await db.user.delete({ id: 'user_123' })
```

### Adapter Pattern — Custom Adapter

```typescript
import { createAdapter } from '@ruvyxa/database'

const myAdapter = createAdapter({
  name: 'my-custom-db',

  async findMany(model, args) {
    const query = buildQuery(model, args)
    return myDbClient.query(query)
  },

  async findUnique(model, { where }) {
    const result = await myDbClient.get({ pk: `${model}:${where.id}` })
    return result ?? null
  },

  async create(model, data) {
    return myDbClient.put({ pk: `${model}:${data.id}`, ...data })
  },

  async update(model, where, data) {
    return myDbClient.update({ pk: `${model}:${where.id}`, ...data })
  },

  async delete(model, where) {
    await myDbClient.delete({ pk: `${model}:${where.id}` })
  },

  async transaction(fn) {
    return myDbClient.transaction(async (tx) => {
      return fn({
        ...myAdapter,
        findMany: (model, args) => tx.query(...),
        // ... wrap each method with tx
      })
    })
  },
})

export const db = createDatabase({ adapter: myAdapter })
```

### Prisma Adapter

```typescript
import { PrismaAdapter } from '@ruvyxa/database/prisma'

const adapter = PrismaAdapter(new PrismaClient(), {
  /** Log queries. @default false */
  logQueries?: boolean
  /** Connection pool size. @default 10 */
  poolSize?: number
})
```

### DynamoDB Adapter

```typescript
import { DynamoDBAdapter } from '@ruvyxa/database/dynamodb'

const adapter = DynamoDBAdapter({
  tableName: 'my-app',
  region: 'us-east-1',
  /** Optional: custom DynamoDB client. */
  client?: DynamoDBClient
})
```

### Database Plugin

```typescript
// ruvyxa.config.ts
export default config({
  plugins: [
    {
      name: '@ruvyxa/database/plugin',
      options: {
        migrations: {
          autoMigrate: false, // run migrations manually in production
          dir: './prisma', // migration directory
        },
        connection: {
          poolSize: 10,
          timeout: 30_000,
        },
      },
    },
  ],
})
```

### Transaction Support

```typescript
// Prisma adapter supports transactions natively
const [user, post] = await db.$transaction(async (tx) => {
  const user = await tx.user.create({ name: 'Alice', email: 'alice@example.com' })
  const post = await tx.post.create({ title: 'Hello', authorId: user.id })
  return [user, post]
})

// Custom adapter — implement transaction() in DatabaseAdapter
// Transactions are not supported by all adapters (DynamoDB single-document).
// Check adapter documentation before relying on transactions.
```

**Transaction semantics**:

- All operations inside a transaction callback must use the `tx` adapter, not `db`
- If any operation fails, all changes in the transaction are rolled back
- Nested transactions are not supported — use sequential transactions
- Timeout applies to the entire transaction, not individual operations

### Batch Operations

```typescript
// createMany — batch insert
const users = await db.user.createMany([
  { name: 'Bob', email: 'bob@example.com' },
  { name: 'Charlie', email: 'charlie@example.com' },
  { name: 'Diana', email: 'diana@example.com' },
])
// Returns all created records
// Max batch size controlled by maxBatchSize config (default 100)
// Larger batches are split into multiple adapter calls

// Bulk updates — not supported natively, use loop or raw query
// For Prisma: use updateMany via adapter's execute()
await db.$execute(`UPDATE "User" SET "active" = false WHERE "lastLogin" < $1`, [cutoffDate])
```

### Model Name Safety

Model names are validated against unsafe characters. Names matching JavaScript `Object.prototype`
properties (e.g., `constructor`, `__proto__`, `toString`) are rejected with RUV3001. Model names
containing dots, spaces, or special characters are also rejected.

### Database Edge Cases

| Scenario                                     | Behavior                                             |
| -------------------------------------------- | ---------------------------------------------------- |
| Model name `constructor`                     | RUV3001 — unsafe model name                          |
| `findUnique` without `where`                 | RUV3001 — requires non-empty where                   |
| `update` without `where`                     | RUV3001 — requires non-empty where clause            |
| `delete` without `where`                     | RUV3001 — requires non-empty where clause            |
| `create` with empty object                   | RUV3001 — requires non-empty data                    |
| `createMany` with empty array                | RUV3001 — requires non-empty data array              |
| Query timeout                                | RUV1400 — operation exceeds timeout                  |
| Adapter returns null for `findMany`          | Treated as empty array `[]`                          |
| Adapter returns null for `findUnique`        | Treated as `null` (not found)                        |
| Transaction with non-transactional adapter   | Falls back to sequential operations without rollback |
| Database URL with wrong protocol             | RUV3003 — connection refused                         |
| Missing adapter method                       | RUV3001 — unsupported operation                      |
| `createMany` on adapter without `createMany` | Falls back to sequential `create` calls              |

### Error Codes (RUV3001-3003)

| Code    | Title                    | Source (package/@ruvyxa/database)                             | Cause                           | Fix                |
| ------- | ------------------------ | ------------------------------------------------------------- | ------------------------------- | ------------------ |
| RUV3001 | Database operation error | `src/index.ts:48,87,90,99,106,109,112`, `src/adapters.ts:139` | Invalid args, model name unsafe | Check query params |
| RUV3002 | Adapter error            | `src/adapters.ts:45,107,139`                                  | Adapter-specific failure        | Check adapter logs |
| RUV3003 | Connection failed        | `src/index.ts:32`                                             | Database unreachable            | Check DATABASE_URL |

**RUV3001** — `Database operation error` (index.ts:48,87,90,99,106,109,112, adapters.ts:139): Model
name contains unsafe characters (matches Object.prototype property), operation type unsupported by
adapter, `where` clause empty for update/delete, `data` empty for create, `createMany` called with
empty array, `findMany`/`findUnique` expects options object.

**RUV3002** — `Adapter error` (adapters.ts:45,107,139): Prisma or DynamoDB adapter threw an error.
Check the underlying adapter logs and database status.

**RUV3003** — `Connection failed` (index.ts:32): Database URL invalid or unreachable at startup.
Verify `DATABASE_URL` in environment, check network access, firewall rules, and database service
status.

---

## @ruvyxa/realtime

WebSocket transport for server actions. Instead of HTTP request-response, data flows over persistent
connection.

### Installation

```bash
npm install @ruvyxa/realtime
```

### Type Definitions

```typescript
// === Server ===

interface RealtimeOptions {
  /** Channel name prefix. */
  prefix?: string
  /** Maximum payload bytes. @default 262144 (256 KiB) */
  maxPayloadBytes?: number
  /** Publish queue capacity. @default 256 */
  capacity?: number
  /** Heartbeat interval in ms. @default 25000 */
  heartbeatMs?: number
}

interface Realtime {
  /** Publish event to a channel. */
  publish(channel: string, event: RealtimeEvent): void
  /** Subscribe to channel (server-side). */
  subscribe(channel: string, handler: (event: RealtimeEvent) => void): () => void
  /** Get subscriber count for channel. */
  subscriberCount(channel: string): number
  /** Close all connections. */
  close(): void
}

interface RealtimeEvent {
  type: string
  payload?: unknown
  [key: string]: unknown
}

function createRealtime(options?: RealtimeOptions): Realtime

// === Client ===

interface RealtimeClient {
  /** Subscribe to channel. */
  subscribe(channel: string, handler: (event: RealtimeEvent) => void): () => void
  /** Unsubscribe from channel. */
  unsubscribe(channel: string): void
  /** Publish event to channel. */
  publish(channel: string, event: RealtimeEvent): void
  /** Connection state. */
  readonly state: 'connecting' | 'connected' | 'disconnected'
  /** Close connection. */
  close(): void
}

function realtimeClient(url: string): RealtimeClient

// React hook
function useRealtime(channel: string, handler: (event: RealtimeEvent) => void): void
```

### Server API

```typescript
// app/lib/realtime.ts (server only)
import { createRealtime } from '@ruvyxa/realtime'

export const realtime = createRealtime({
  prefix: 'app',
  maxPayloadBytes: 256 * 1024,
  capacity: 256,
  heartbeatMs: 25000,
})
```

### Action Integration

```typescript
// app/actions/chat/action.ts
'use server'

import { action } from 'ruvyxa/server'
import { realtime } from '../../lib/realtime'

export const sendMessage = action(async (message: string) => {
  // Broadcast to channel
  realtime.publish('chat:room1', {
    type: 'message',
    payload: { text: message, timestamp: Date.now() },
  })
})

// Mark action as real-time — clients get live updates
sendMessage.realtime('chat:room1')
```

### Channel Format

```typescript
// Convention: <scope>:<id>
// Server channels:
realtime.publish('chat:room1', event)
realtime.publish('game:lobby', event)
realtime.publish('stock:AAPL', event)
realtime.publish('notifications:user-123', event)

// Route-based channels (automatic via action.realtime()):
sendMessage.realtime('chat:general')
// Server broadcasts to all clients subscribed to 'chat:general'

// Channel types:
// 'route:/path' — channel scoped to a route
// 'route-hash:hex' — hashed route identifier
```

### Client Subscriptions

```typescript
// app/components/Chat.tsx
'use client'

import { useRealtime } from '@ruvyxa/realtime/client'
import { useState } from 'react'

export function Chat() {
  const [messages, setMessages] = useState<string[]>([])

  useRealtime('chat:room1', (event) => {
    if (event.type === 'message') {
      setMessages((prev) => [...prev, event.payload.text])
    }
  })

  return (
    <div>
      {messages.map((msg, i) => <p key={i}>{msg}</p>)}
    </div>
  )
}
```

### Direct Client API

```typescript
'use client'

import { realtimeClient } from '@ruvyxa/realtime/client'
import { useEffect } from 'react'

export function StockTicker() {
  useEffect(() => {
    const wsUrl = `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/__ruvyxa/realtime`
    const client = realtimeClient(wsUrl)

    const unsub = client.subscribe('stock:AAPL', (event) => {
      console.log('AAPL price:', event.payload.price)
    })

    client.subscribe('stock:GOOGL', (event) => {
      console.log('GOOGL price:', event.payload.price)
    })

    return () => {
      client.close()
    }
  }, [])

  return <div>Stock Ticker Connected</div>
}
```

### WebSocket Protocol

```
Connection: ws://host/__ruvyxa/realtime

Actions with .realtime(channel) pattern:
  1. Client sends action via form POST
  2. Server action handler processes request
  3. Server broadcasts RealtimeEvent to channel via WebSocket
  4. All subscribed clients receive the event

Channel subscription:
  Client → Server: {"type": "subscribe", "channel": "chat:room1"}
  Server → Client: {"type": "subscribed", "channel": "chat:room1"}

Event broadcast:
  Server → Client: {"type": "message", "channel": "chat:room1", "payload": {...}}

Heartbeat:
  Server → Client: {"type": "ping"}
  Client → Server: {"type": "pong"}

Error:
  Server → Client: {"type": "error", "code": "RUV3201", "message": "..."}
```

### Realtime Plugin

```typescript
// ruvyxa.config.ts
export default config({
  plugins: [
    {
      name: '@ruvyxa/realtime/plugin',
      options: {
        ws: {
          path: '/_ws', // WebSocket endpoint
          maxClients: 1000, // per server
          heartbeatMs: 25000,
        },
      },
    },
  ],
})
```

### WebSocket Connection Lifecycle

```
Client                          Server
  │                               │
  │  ws://host/__ruvyxa/realtime  │
  │─────────────────────────────>│
  │                               │  Accept connection
  │                               │  Assign client ID
  │                               │
  │  {"type":"connected",         │
  │   "clientId":"c_abc123"}      │
  │<─────────────────────────────│
  │                               │
  │  {"type":"subscribe",         │
  │   "channel":"chat:room1"}     │
  │─────────────────────────────>│
  │                               │  Add to channel subscribers
  │  {"type":"subscribed",        │
  │   "channel":"chat:room1"}     │
  │<─────────────────────────────│
  │                               │
  │  (someone publishes event)    │
  │  {"type":"message",           │
  │   "channel":"chat:room1",     │
  │   "payload":{...}}            │
  │<─────────────────────────────│
  │                               │
  │  {"type":"ping"}              │
  │<─────────────────────────────│  Every heartbeatMs (25s default)
  │  {"type":"pong"}              │
  │─────────────────────────────>│
  │                               │
  │  {"type":"unsubscribe",       │
  │   "channel":"chat:room1"}     │
  │─────────────────────────────>│
  │                               │  Remove from channel
  │  {"type":"unsubscribed",      │
  │   "channel":"chat:room1"}     │
  │<─────────────────────────────│
  │                               │
  │  (connection close)           │
  │  ──── CLOSE ────────────────>│  Remove from all channels
```

### Multi-Process Broadcasting

When running multiple server processes (horizontal scaling), the in-memory publish queue does NOT
broadcast across processes. Each process has its own subscriber list. Events published on process A
are NOT received by clients connected to process B.

**Solution**: Use an external message broker:

```typescript
// Not built-in — implement your own cross-process bridge
// Example using Redis pub/sub:
import { createRealtime } from '@ruvyxa/realtime'

const realtime = createRealtime({ capacity: 256 })

// Subscribe to Redis channel and forward to Ruvyxa realtime
const redis = new Redis()
redis.subscribe('ruvyxa:events', (channel, message) => {
  const event = JSON.parse(message)
  realtime.publish(event.channel, event)
})

// Forward published events to Redis
const originalPublish = realtime.publish.bind(realtime)
realtime.publish = (channel, event) => {
  redis.publish('ruvyxa:events', JSON.stringify({ channel, ...event }))
  originalPublish(channel, event)
}
```

### Realtime Edge Cases

| Scenario                                | Behavior                                             |
| --------------------------------------- | ---------------------------------------------------- |
| Subscribe to same channel twice         | Duplicate subscription ignored, single handler fires |
| Unsubscribe from non-subscribed channel | No-op                                                |
| Publish without subscribers             | Event silently dropped (no-op)                       |
| Payload > maxPayloadBytes               | RUV3201 — message rejected                           |
| Channel name empty or too long          | RUV3201 — invalid channel                            |
| Client disconnects uncleanly            | Cleaned up on next heartbeat cycle                   |
| Heartbeat timeout (no pong)             | Client disconnected, removed from all channels       |
| Capacity exceeded (publish queue full)  | Oldest event dropped, newest accepted                |
| `sendMessage.realtime()` called twice   | RUV1500 — duplicate metadata                         |
| Route-based channel (`route:/path`)     | Auto-subscribes clients on that route                |
| Connection from wrong origin            | Depends on server CORS config                        |

### Error Codes (RUV3201)

| Code    | Title          | Source                | Cause                               | Fix                                |
| ------- | -------------- | --------------------- | ----------------------------------- | ---------------------------------- |
| RUV3201 | Realtime error | `src/plugin.ts:43-46` | Invalid message, protocol violation | Check message format, channel name |

**RUV3201** — `Realtime error` (plugin.ts:43): Invalid message format (not valid JSON), protocol
violation (missing `type` field), channel name too long (>256 chars), payload exceeds
`maxPayloadBytes` (default 256 KiB), subscribe/publish to invalid channel name.

---

## Combining Packages — Full Example

### app/lib/auth.ts

```typescript
import { createAuth } from '@ruvyxa/auth'

export const auth = createAuth({
  secret: process.env.AUTH_SECRET,
  session: { strategy: 'jwt', maxAge: 7 * 86400 },
  providers: {
    google: {
      clientId: process.env.GOOGLE_CLIENT_ID!,
      clientSecret: process.env.GOOGLE_CLIENT_SECRET!,
    },
    magicLink: {
      secret: process.env.MAGIC_LINK_SECRET!,
    },
  },
})
```

### app/lib/db.ts

```typescript
import { createDatabase } from '@ruvyxa/database'
import { PrismaAdapter } from '@ruvyxa/database/prisma'
import { PrismaClient } from '@prisma/client'

const prisma = new PrismaClient()
export const db = createDatabase({ adapter: PrismaAdapter(prisma) })
```

### app/lib/realtime.ts

```typescript
import { createRealtime } from '@ruvyxa/realtime'
export const realtime = createRealtime()
```

### app/actions/chat/action.ts

```typescript
'use server'

import { action } from 'ruvyxa/server'
import { auth } from '../../lib/auth'
import { db } from '../../lib/db'
import { realtime } from '../../lib/realtime'

export const sendMessage = action(async (formData: FormData) => {
  const session = await auth.getSession()
  if (!session) throw new Error('Unauthorized')

  const message = formData.get('message') as string
  if (!message || message.length > 1000) {
    throw new Error('Invalid message')
  }

  // Save to database
  const saved = await db.message.create({
    text: message,
    userId: session.user.id,
    roomId: 'general',
  })

  // Broadcast to all connected clients
  realtime.publish('chat:general', {
    type: 'message',
    payload: { id: saved.id, text: message, user: session.user.name },
  })
})

sendMessage.realtime('chat:general')
```

### app/components/Chat.tsx

```typescript
'use client'

import { createAuthClient } from '@ruvyxa/auth/client'
import { useRealtime } from '@ruvyxa/realtime/client'
import { useState } from 'react'

const authClient = createAuthClient()

export function ChatRoom() {
  const [messages, setMessages] = useState<any[]>([])
  const [input, setInput] = useState('')

  useRealtime('chat:general', (event) => {
    if (event.type === 'message') {
      setMessages((prev) => [...prev, event.payload])
    }
  })

  return (
    <div>
      <div>
        {messages.map((m) => (
          <p key={m.id}><strong>{m.user}:</strong> {m.text}</p>
        ))}
      </div>
      <form action={sendMessage}>
        <input name="message" value={input} onChange={(e) => setInput(e.target.value)} />
        <button type="submit">Send</button>
      </form>
    </div>
  )
}
```

### ruvyxa.config.ts

```typescript
import { config } from 'ruvyxa/config'

export default config({
  plugins: [
    { name: '@ruvyxa/auth/plugin' },
    { name: '@ruvyxa/database/plugin', options: { migrations: { autoMigrate: false } } },
    { name: '@ruvyxa/realtime/plugin', options: { ws: { path: '/_ws', maxClients: 1000 } } },
  ],
})
```

### Complete Data Flow

```
User types message in Chat.tsx
        │
        ▼
Client: form action → POST to server action
        │
        ▼
Server: sendMessage action
  1. auth.getSession() → validates session cookie
  2. db.message.create() → persists to database
  3. realtime.publish('chat:general') → broadcasts event
        │
        ├───► Response returns to sending client
        │
        └───► WebSocket broadcast to all other subscribed clients
                │
                ▼
              Client: useRealtime('chat:general') handler fires
              → setMessages(prev => [...prev, event.payload])
```

**Key architectural points**:

- Auth is validated per-action, not per-WebSocket connection
- Database operations are synchronous within the action — the client waits for DB write
- Realtime broadcast happens after DB write, so all clients see confirmed data
- The sending client gets the response via HTTP, not WebSocket (if the form action re-renders, the
  new data includes the sent message)
- Other connected clients receive the WebSocket event and update their local state

---

## Package Version Compatibility

| Package                  | Compatible Ruvyxa | Notes                                    |
| ------------------------ | ----------------- | ---------------------------------------- |
| `@ruvyxa/auth`           | ^2.0.0            | Requires `ruvyxa` peer dependency        |
| `@ruvyxa/database`       | ^2.0.0            | Adapter packages versioned independently |
| `@ruvyxa/realtime`       | ^2.0.0            | Plugin and client packages               |
| `@ruvyxa/adapter-vercel` | ^2.0.0            | Deploy adapter                           |
| `@ruvyxa/adapter-node`   | ^2.0.0            | Standalone server                        |

Always use the latest compatible version. Breaking changes are documented in changelogs. Run
`ruvyxa check` after upgrading packages to verify compatibility.

---

## Troubleshooting

| Problem                                    | Cause                                 | Fix                                                                         |
| ------------------------------------------ | ------------------------------------- | --------------------------------------------------------------------------- |
| `Cannot find module '@ruvyxa/auth'`        | Package not installed                 | `npm install @ruvyxa/auth`                                                  |
| RUV1007 on import                          | Root package imported in client       | Use `/client` subpath                                                       |
| Auth session always null                   | Missing `AUTH_SECRET`                 | Set `AUTH_SECRET` in `.env.local`                                           |
| Auth session always null                   | Wrong cookie name                     | Check `cookies.name` matches between `createAuth` and browser               |
| OAuth callback 404                         | Auth plugin not configured            | Add `@ruvyxa/auth/plugin` to plugins                                        |
| OAuth redirects to wrong domain            | `origin` config mismatch              | Set `origin: 'https://example.com'` in `createAuth()`                       |
| OAuth state invalid                        | Cookies not sent cross-origin         | Check `sameSite` config, ensure callback is same-site                       |
| OAuth state invalid                        | Rate-limit store cleared              | OAuth state stored in rate-limit store; use durable store                   |
| OAuth "redirect_uri mismatch"              | Provider config mismatch              | Check callback URL in provider dashboard                                    |
| OAuth token exchange 400                   | Wrong client credentials              | Check `clientId` and `clientSecret` in env vars                             |
| Magic link not sent                        | Email `send` function error           | Check `send()` implementation or SMTP config                                |
| Magic link token invalid                   | Link expired (>900s)                  | Request new magic link                                                      |
| WebAuthn fails                             | HTTPS required                        | Use HTTPS in production (WebAuthn requires secure context)                  |
| WebAuthn `NotSupportedError`               | Browser/platform doesn't support      | Check `PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable()` |
| Database connection refused                | Wrong URL or network                  | Check `DATABASE_URL`, firewall, VPN, service status                         |
| Database RUV3001 on create                 | Model name unsafe                     | Avoid `constructor`, `__proto__`, `toString` as model names                 |
| Database RUV3001 on update                 | Missing `where` clause                | Always provide a where clause for update/delete                             |
| Prisma migrate fails                       | Schema changed                        | Run `npx prisma migrate dev` to sync                                        |
| Prisma `@ruvyxa/database/prisma` not found | Wrong import path                     | Import from `@ruvyxa/database/prisma`, not `@ruvyxa/database`               |
| DynamoDB adapter not found                 | Missing dependency                    | `npm install @aws-sdk/client-dynamodb @aws-sdk/lib-dynamodb`                |
| WebSocket won't connect                    | Plugin not registered                 | Add `@ruvyxa/realtime/plugin` to config plugins                             |
| WebSocket disconnects                      | Heartbeat timeout                     | Check network stability, increase `heartbeatMs` (max 120000)                |
| `realtime.publish()` no-op                 | Channel has no subscribers            | Subscribe before publishing                                                 |
| Max clients reached                        | Too many concurrent connections       | Increase `maxClients` or scale horizontally                                 |
| Payload too large                          | Event > 256 KiB                       | Reduce payload or increase `maxPayloadBytes`                                |
| RUV1500: Duplicate realtime metadata       | `sendMessage.realtime()` called twice | Remove duplicate `.realtime()` call on action                               |
| `getSession()` returns null in API routes  | Request object not passed             | Pass `request` to `auth.getSession(request)`                                |
| Cross-origin auth blocked                  | `Origin` header mismatch              | Set `origin` in `createAuth()` or use `allowInsecure` for dev               |

---

## Choose a First-party Package by Its Boundary

The repository ships three first-party integration packages with distinct responsibilities:

| Package            | Owns                                                                         | Important boundary                                               |
| ------------------ | ---------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `@ruvyxa/auth`     | Authentication runtime, provider/session flows, and auth plugin integration. | Keep credentials and session handling server-side.               |
| `@ruvyxa/database` | Typed facade over an application-supplied database adapter.                  | It does not select, install, or pool a database driver for you.  |
| `@ruvyxa/realtime` | Realtime plugin/client integration.                                          | Treat delivery as transient event delivery, not durable storage. |

`@ruvyxa/database` starts with an explicit adapter, then exposes model delegates such as `findMany`,
`findUnique`, `create`, `update`, and `delete`. It validates unsafe model names and requires a
non-empty `where` clause for unique/update/delete operations. The application owns the adapter's
connection lifecycle and database schema.

```ts
import { createDatabase } from '@ruvyxa/database'

const db = createDatabase<AppSchema>(adapter)
const product = await db.Product.findUnique({ where: { id: productId } })
```

`@ruvyxa/auth` provides `createAuth()` and built-in OAuth provider helpers such as `google()` and
`github()`. Its runtime performs security-sensitive work such as same-origin request checks and
session-cookie handling, but it still needs application configuration and a secure deployment
origin. Never import auth/database server work into a client-reachable module: the framework's
boundary validator treats both packages as server-only specifiers.

### Integrate One Capability at a Time

1. Install and configure one package with its documented options.
2. Keep its secrets in a server-only module/environment.
3. Run `ruvyxa analyze --format human` to catch a boundary leak.
4. Exercise the route/action that consumes the integration.
5. Run `npm run check` before broadening the integration.

This sequence avoids presenting a package as a complete authentication, database, or event-delivery
architecture. It supplies a framework integration boundary; data modeling, authorization policy,
driver configuration, and operational monitoring remain application decisions.

---

## Next Steps

- **[03-server-client-components.md](./03-server-client-components.md)** — Server/client boundary
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — Fetch data from database
- **[06-server-actions.md](./06-server-actions.md)** — Actions with auth
- **[07-api-routes.md](./07-api-routes.md)** — API routes with auth and database
- **[10-environment-variables.md](./10-environment-variables.md)** — Env vars for API keys
- **[14-plugins.md](./14-plugins.md)** — Plugin system used by official packages
- **[16-error-handling.md](./16-error-handling.md)** — Error codes reference

# Unit testing Server primitives

`@ruvyxa/testing` provides dependency-free Vitest/Jest/Node-test utilities for loaders, actions, and
cache policies without starting a real server:

```ts
import { mockAction, mockCache, mockLoader } from '@ruvyxa/testing'

const loadUser = mockLoader(async ({ params }) => ({ id: params.id }))
const saveUser = mockAction(async ({ input, invalidate }) => {
  invalidate('users')
  return input
})
const cache = mockCache({ users: [{ id: '1' }] })
```

Each mock exposes recorded calls and `reset()`. Use a real disposable database and generated route
artifacts for integration tests; these mocks intentionally isolate application logic.
