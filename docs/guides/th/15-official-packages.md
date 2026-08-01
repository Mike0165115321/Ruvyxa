# Official Packages

เอกสารนี้อธิบาย API ที่ export อยู่จริงใน first-party packages ของ repository ปัจจุบัน ตัวอย่างจะ
ใช้เฉพาะ API ที่พบใน `src/` และ export map ของ package เท่านั้น

## ขอบเขตของแต่ละ package

| Package            | ฝั่ง Server        | ฝั่ง Browser              | หน้าที่                              |
| ------------------ | ------------------ | ------------------------- | ------------------------------------ |
| `@ruvyxa/auth`     | `@ruvyxa/auth`     | `@ruvyxa/auth/client`     | Session และ authentication providers |
| `@ruvyxa/database` | `@ruvyxa/database` | ไม่มี                     | Adapter contract และ typed facade    |
| `@ruvyxa/realtime` | `@ruvyxa/realtime` | `@ruvyxa/realtime/client` | WebSocket ที่ขับเคลื่อนด้วย action   |

ห้าม import server entry เข้า client bundle ส่วน `/client` มีเฉพาะ browser helper ของ package นั้น

## `@ruvyxa/auth`

Auth runtime ต้องมี secret, origin, store, rate-limit store และ provider map อย่างชัดเจน ใน
repository นี้มี OAuth helper สำหรับ Google และ GitHub ส่วน magic-link กับ WebAuthn เป็น provider
interface ที่ application ต้อง implement เอง

```ts
import { config } from 'ruvyxa/config'
import { createAuth, google, memoryAuthStore, memoryRateLimitStore } from '@ruvyxa/auth'

const auth = createAuth({
  secret: process.env.AUTH_SECRET!, // อย่างน้อย 32 ตัวอักษร
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

`memoryAuthStore` และ `memoryRateLimitStore` ต้องรับ `{ development: true }` และใช้สำหรับ test หรือ
development เท่านั้น production build จะตรวจว่า store เป็น durable store ส่วน option อื่นที่รองรับ
ได้แก่ `basePath`, `session.ttlSeconds`, `session.rememberTtlSeconds`, `session.cookieName`,
`session.secure`, `session.sameSite`, `rateLimit`, `clientIp` และ `onError`

### Browser client

```ts
import { createAuthClient } from '@ruvyxa/auth/client'

const authClient = createAuthClient({ basePath: '/__ruvyxa/auth' })
await authClient.login('credentials', { email: 'demo@example.com', password: 'demo' })
const session = await authClient.session()
authClient.oauth('google', '/account')
await authClient.logout()
```

Method ที่มีจริงคือ `login`, `logout`, `session` และ `oauth`; ไม่มี `signIn`, `signOut` หรือ hook
ชื่อ `useRealtime` ใน package นี้

## `@ruvyxa/database`

Database facade รับ adapter หนึ่งตัว โดย built-in adapters ที่ export คือ `prismaAdapter` และ
`dynamoAdapter` จาก package root

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

ปัจจุบันไม่มี export `@ruvyxa/database/prisma` หรือ `@ruvyxa/database/dynamodb` ตัว Dynamo adapter
รับ `transport` และ map `tables` อย่างชัดเจน โดย transport อาจห่อ AWS SDK หรือ implementation
อื่นที่ ทำตาม `execute(operation)` contract ส่วน custom adapter ตรวจสอบได้ด้วย
`defineDatabaseAdapter`

Database plugin ใช้ตรวจ private environment variables ระหว่าง build:

```ts
import { databasePlugin } from '@ruvyxa/database/plugin'

databasePlugin({ requiredEnv: ['DATABASE_URL'] })
```

## `@ruvyxa/realtime`

Realtime เป็น native capability ที่ขับเคลื่อนด้วย action ให้ลงทะเบียน root plugin และใช้
`action.realtime()` จาก core plugin จะตรวจว่า target ที่เลือกมี Node/Bun WebSocket runtime แบบ
long-lived ได้หรือไม่ ไม่ได้ export server API ชื่อ `createRealtime()` หรือ `publish()`

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
// ภายหลัง: unsubscribe(); client.close()
```

Client รองรับ `subscribe`, `subscribeRoute` และ `close` พร้อม reconnect ที่มีขอบเขต หาก adapter ไม่
สามารถให้ runtime แบบ long-lived ได้ plugin จะ reject ตอน build completion

## Versioning

source tree นี้ใช้ release version เดียวกันสำหรับ first-party packages manifest ที่ commit อยู่
ปัจจุบันระบุ `1.0.25` สำหรับ `ruvyxa`, `@ruvyxa/auth`, `@ruvyxa/database`, `@ruvyxa/realtime` และ
first-party adapter packages ทั้งหมด Built-in plugins ไม่ได้แยก package และไม่มี version แยก จึง
ติดตาม version ของ package `ruvyxa`

`realtime@1` เป็น native capability/protocol identifier ไม่ใช่ package version

## ขอบเขตของข้ออ้าง

พฤติกรรมของ provider, ข้อจำกัดของ platform, credential และ deployment health เป็นข้อมูลภายนอก ต้อง
ตรวจสอบกับเอกสาร provider และ artifact ที่ adapter สร้างขึ้น เอกสารนี้ไม่ได้อ้าง benchmark, ROI,
พันธมิตร หรือการ deploy production อัตโนมัติ

---

## Production contract and retained detail

The section above is the current, source-backed contract for this release. The original long-form
draft is retained below to preserve instructional context and audit history. It is non-normative: do
not copy its API snippets or capability claims unless they are revalidated against the current
source and package export map. This boundary is intentional so the document can retain its original
depth without presenting unsupported historical design as production behavior.

### Thai package draft — historical draft (non-normative)

> **คำเตือน archive:** เนื้อหาด้านล่างเก็บไว้เพื่อประวัติเท่านั้น ไม่ใช่ package API ปัจจุบัน
> ตัวอย่างอาจเก่าหรือไม่รองรับ และห้ามนำไปใช้เป็น code จริง production contract
> ด้านบนเป็นแหล่งอ้างอิงหลัก

# Official Packages: Auth, Database, Realtime

Ruvyxa มี 3 official packages ที่ทำงานร่วมกับเฟรมเวิร์กได้อย่างลื่นไหล:

- `@ruvyxa/auth` — การยืนยันตัวตน (session, OAuth, PKCE, magic-link, WebAuthn)
- `@ruvyxa/database` — ฐานข้อมูล (typed facade รองรับ Prisma, DynamoDB, custom)
- `@ruvyxa/realtime` — เวลาจริง via WebSocket (channels, broadcast, presence)

ทั้ง 3 packages ลงทะเบียนเป็น plugin ใน `ruvyxa.config.ts` — Ruvyxa จัดการ endpoints, environment
variables, และ runtime dependencies ให้อัตโนมัติ

---

## ข้อสำคัญ: Import Rules

```
Package               Server Import              Client Import
─────────────────────────────────────────────────────────────────
@ruvyxa/auth          @ruvyxa/auth               @ruvyxa/auth/client
@ruvyxa/database      @ruvyxa/database           ❌ ไม่มี (server-only)
@ruvyxa/realtime      @ruvyxa/realtime           @ruvyxa/realtime/client
```

**กฎสำคัญ**:

- `@ruvyxa/database` ไม่มี client path — database ต้องอยู่บน server เท่านั้น
- ถ้า import server path ใน client component → **RUV1007** Client Boundary Violation
- `@ruvyxa/realtime` มีทั้ง server และ client — server สำหรับ broadcast, client สำหรับ subscribe

### RUV1007 — ตัวอย่างที่ทำให้เกิด error

```tsx
// ❌ app/page.tsx (client component)
'use client'
import { auth } from '@ruvyxa/auth' // RUV1007 — ต้องใช้ /client
import { database } from '@ruvyxa/database' // RUV1007 — server-only

// ✅ แก้ไข
import { useSession, signIn } from '@ruvyxa/auth/client'
// database ใช้ผ่าน server action เท่านั้น
```

---

## @ruvyxa/auth

ระบบ authentication ที่ครอบคลุม — session-based, OAuth, magic link, WebAuthn (passkeys)

### Type Definitions

```typescript
// ===== Core Types =====

interface AuthConfig {
  providers: Provider[]
  session: SessionConfig
  pages?: AuthPages // Custom page paths
  callbacks?: AuthCallbacks // Custom callbacks
  events?: AuthEvents // Event hooks
  logger?: AuthLogger
}

type Provider =
  | 'google'
  | 'github'
  | 'facebook'
  | 'line'
  | 'credentials'
  | 'magic-link'
  | 'webauthn'
  | OAuthProviderConfig // Custom OAuth provider

interface SessionConfig {
  strategy: 'jwt' | 'database' // JWT = stateless, database = stateful
  maxAge: number // Session age (seconds)
  updateAge?: number // อัปเดต session ทุกกี่วินาที (default: 86400)
}

interface AuthPages {
  signIn?: string // '/auth/login'
  error?: string // '/auth/error'
  verifyRequest?: string // '/auth/verify-request'
  newUser?: string // '/auth/new-user'
}

// ===== Session Types =====

interface Session {
  user: {
    id: string
    email: string
    name: string | null
    image: string | null
    emailVerified: boolean
  }
  expiresAt: Date
  accessToken?: string // สำหรับ JWT strategy
}

// ===== OAuth Provider Config =====

interface OAuthProviderConfig {
  id: string // Unique provider ID
  name: string // Display name
  type: 'oauth' | 'oidc'
  clientId: string
  clientSecret: string
  authorization: {
    url: string
    params?: Record<string, string>
  }
  token: {
    url: string
    params?: Record<string, string>
  }
  userinfo: {
    url: string
    request?: (token: string) => Promise<any>
  }
  profile: (profile: any) => UserProfile
  checks?: ('pkce' | 'state' | 'nonce')[]
}

// ===== createAuth =====

function createAuth(config: AuthConfig): {
  auth: () => Promise<Session | null>
  signIn: (provider: string, options?: SignInOptions) => Promise<void>
  signOut: (options?: SignOutOptions) => Promise<void>
  handlers: AuthHandlers // Next.js-like handlers
  providers: ProviderRegistry
}

// ===== createAuthClient =====

function createAuthClient(): {
  useSession: () => {
    data: Session | null
    status: 'loading' | 'authenticated' | 'unauthenticated'
  }
  signIn: (provider: string, options?: SignInOptions) => Promise<void>
  signOut: (options?: SignOutOptions) => Promise<void>
  useWebAuthn: () => WebAuthnHooks
}
```

### ติดตั้ง

```bash
npm install @ruvyxa/auth
```

### Plugin Registration

`@ruvyxa/auth` ต้องลงทะเบียนเป็น plugin ใน `ruvyxa.config.ts`:

```ts
// ruvyxa.config.ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  plugins: [
    {
      name: '@ruvyxa/auth',
      options: {
        providers: [
          'google',
          'github',
          'magic-link',
          'webauthn',
          {
            id: 'credentials',
            name: 'Email & Password',
            type: 'credentials',
            credentials: {
              email: { label: 'Email', type: 'email' },
              password: { label: 'Password', type: 'password' },
            },
            authorize: async (credentials) => {
              const user = await validateUser(credentials.email, credentials.password)
              return user || null
            },
          },
        ],
        session: {
          strategy: 'jwt', // 'jwt' | 'database'
          maxAge: 7 * 24 * 60 * 60, // 7 วัน
        },
        pages: {
          signIn: '/auth/login',
          error: '/auth/error',
        },
      },
    },
  ],
})
```

### Environment Variables

```bash
# .env.local — development
# .env.production — production

# Core
AUTH_SECRET=my-super-secret-key-min-32-chars

# Google OAuth
AUTH_GOOGLE_ID=xxx.apps.googleusercontent.com
AUTH_GOOGLE_SECRET=GOCSPX-xxxx

# GitHub OAuth
AUTH_GITHUB_ID=Ov23li...
AUTH_GITHUB_SECRET=xxxx

# LINE OAuth
AUTH_LINE_ID=xxx
AUTH_LINE_SECRET=xxx

# Facebook OAuth
AUTH_FACEBOOK_ID=xxx
AUTH_FACEBOOK_SECRET=xxx

# Magic Link (SMTP)
AUTH_SMTP_HOST=smtp.gmail.com
AUTH_SMTP_PORT=587
AUTH_SMTP_USER=my-app@gmail.com
AUTH_SMTP_PASSWORD=xxxx
AUTH_EMAIL_FROM=noreply@my-app.com

# WebAuthn
AUTH_WEBAUTHN_RP_NAME=My App
AUTH_WEBAUTHN_RP_ID=example.com
AUTH_WEBAUTHN_ORIGIN=https://example.com

# Database session
DATABASE_URL=postgres://...   # จำเป็นถ้าใช้ session.strategy='database'
```

### OAuth Provider — ทุกค่าที่เป็นไปได้

| Provider ID | ชื่อ         | ต้องใช้ env var                              | PKCE Support |
| ----------- | ------------ | -------------------------------------------- | ------------ |
| `google`    | Google       | `AUTH_GOOGLE_ID`, `AUTH_GOOGLE_SECRET`       | ✓            |
| `github`    | GitHub       | `AUTH_GITHUB_ID`, `AUTH_GITHUB_SECRET`       | ✗            |
| `facebook`  | Facebook     | `AUTH_FACEBOOK_ID`, `AUTH_FACEBOOK_SECRET`   | ✓            |
| `line`      | LINE         | `AUTH_LINE_ID`, `AUTH_LINE_SECRET`           | ✓            |
| `apple`     | Apple        | `AUTH_APPLE_ID`, `AUTH_APPLE_SECRET`         | ✓            |
| `discord`   | Discord      | `AUTH_DISCORD_ID`, `AUTH_DISCORD_SECRET`     | ✓            |
| `twitter`   | Twitter/X    | `AUTH_TWITTER_ID`, `AUTH_TWITTER_SECRET`     | ✗            |
| `spotify`   | Spotify      | `AUTH_SPOTIFY_ID`, `AUTH_SPOTIFY_SECRET`     | ✓            |
| `notion`    | Notion       | `AUTH_NOTION_ID`, `AUTH_NOTION_SECRET`       | ✓            |
| `slack`     | Slack        | `AUTH_SLACK_ID`, `AUTH_SLACK_SECRET`         | ✓            |
| `microsoft` | Microsoft    | `AUTH_MICROSOFT_ID`, `AUTH_MICROSOFT_SECRET` | ✓            |
| `gitlab`    | GitLab       | `AUTH_GITLAB_ID`, `AUTH_GITLAB_SECRET`       | ✓            |
| `auth0`     | Auth0        | `AUTH_AUTH0_ID`, `AUTH_AUTH0_SECRET`         | ✓            |
| `okta`      | Okta         | `AUTH_OKTA_ID`, `AUTH_OKTA_SECRET`           | ✓            |
| `keycloak`  | Keycloak     | `AUTH_KEYCLOAK_ID`, `AUTH_KEYCLOAK_SECRET`   | ✓            |
| `custom`    | Custom OAuth | —                                            | ✓            |

### Server API

```ts
// app/lib/auth.ts
'use server'

import { createAuth } from '@ruvyxa/auth'

// สร้าง auth instance — ใช้ config จาก plugin
const { auth, signIn, signOut, handlers } = createAuth()

// === ตรวจสอบ session ===
export const getSession = auth(async () => {
  const session = await auth()
  return session // Session | null
})

// === Sign in ด้วย credentials ===
export const login = auth(async (formData: FormData) => {
  await signIn('credentials', {
    email: formData.get('email') as string,
    password: formData.get('password') as string,
    redirectTo: '/dashboard',
  })
})

// === Sign in ด้วย OAuth ===
export const loginWithGoogle = auth(async () => {
  await signIn('google', { redirectTo: '/dashboard' })
})

export const loginWithGithub = auth(async () => {
  await signIn('github', { redirectTo: '/dashboard' })
})

// === Sign out ===
export const logout = auth(async () => {
  await signOut({ redirectTo: '/' })
})

// === API route handlers (alternative to plugin endpoints) ===
export const GET = handlers.GET
export const POST = handlers.POST
```

#### Session Object — ละเอียด

```typescript
interface Session {
  user: {
    id: string;                  // Unique user ID
    email: string;               // อีเมล (verified หรือไม่)
    name: string | null;         // ชี่อผู้ใช้
    image: string | null;        // URL รูปโปรไฟล์
    emailVerified: boolean;      // อีเมลยืนยันแล้ว?
  };
  expiresAt: Date;               // หมดอายุเมื่อไร
  accessToken?: string;          // JWT token (ถ้า strategy='jwt')
  provider?: string;             // Provider ที่ใช้ login ('google', 'github', ฯลฯ)
}

// ตัวอย่าง session จริง
{
  "user": {
    "id": "user_2kR7q8XyZw3Ab",
    "email": "user@example.com",
    "name": "สมชาย ใจดี",
    "image": "https://lh3.googleusercontent.com/a/...",
    "emailVerified": true
  },
  "expiresAt": "2026-08-05T10:30:00Z",
  "accessToken": "eyJhbGciOiJIUzI1NiIs...",
  "provider": "google"
}
```

### Client API (`@ruvyxa/auth/client`)

```typescript
// @ruvyxa/auth/client exports:
export { useSession, signIn, signOut, useWebAuthn, createAuthClient }
```

```tsx
'use client'

import { useSession, signIn, signOut } from '@ruvyxa/auth/client'

export default function AuthButtons() {
  const { data: session, status } = useSession()

  // status: 'loading' | 'authenticated' | 'unauthenticated'

  if (status === 'loading') return <p>กำลังโหลด...</p>

  if (session) {
    return (
      <div>
        <img
          src={session.user.image || '/default-avatar.png'}
          alt={session.user.name || ''}
          width={32}
          height={32}
        />
        <p>สวัสดี, {session.user.name}</p>
        <p>อีเมล: {session.user.email}</p>
        <button onClick={() => signOut()}>ออกจากระบบ</button>
      </div>
    )
  }

  return (
    <div className="flex gap-2">
      <button onClick={() => signIn('google')}>เข้าสู่ระบบด้วย Google</button>
      <button onClick={() => signIn('github')}>เข้าสู่ระบบด้วย GitHub</button>
      <button onClick={() => signIn('line')}>เข้าสู่ระบบด้วย LINE</button>
    </div>
  )
}
```

#### signIn Options

```typescript
interface SignInOptions {
  redirectTo?: string // Redirect after sign in
  redirect?: boolean // Auto-redirect? (default: true)
  callbackUrl?: string // Callback URL (แทน redirectTo)
  [key: string]: any // Provider-specific options:
  // - email (magic-link)
  // - password (credentials)
}
```

#### signOut Options

```typescript
interface SignOutOptions {
  redirectTo?: string // Redirect after sign out
  redirect?: boolean // Auto-redirect? (default: true)
  callbackUrl?: string
}
```

### PKCE Flow (Proof Key for Code Exchange)

PKCE ใช้กับ OAuth สำหรับ mobile/native apps หรือ server-side flow ที่ต้องการ ความปลอดภัยสูงขึ้น:

```typescript
// เซิร์ฟเวอร์
import { generatePKCEChallenge, verifyPKCE } from '@ruvyxa/auth'

async function startPKCE() {
  // สร้าง challenge pair
  const { challenge, verifier } = await generatePKCEChallenge()

  // เก็บ verifier ไว้ใน session/storage
  await saveVerifier(sessionId, verifier)

  // ส่ง challenge ไป client
  return {
    challenge,
    authorizationUrl: `https://accounts.google.com/o/oauth2/v2/auth?code_challenge=${challenge}&code_challenge_method=S256`,
  }
}

async function completePKCE(code: string, sessionId: string) {
  const verifier = await getVerifier(sessionId)
  if (!verifier) throw new Error('PKCE session expired')

  // ตรวจสอบ code กับ verifier
  const valid = await verifyPKCE(code, verifier)
  if (!valid) throw new Error('PKCE verification failed')

  // code verified → สร้าง session
  const session = await createSession(user)
  return session
}
```

**PKCE ทำงานยังไง:**

```
Client                          Server
  │                               │
  │── GET /auth/start ──────────► │
  │                               ├── generate code_verifier (random)
  │                               ├── generate code_challenge = SHA256(verifier)
  │◄── { challenge } ──────────── │
  │                               │
  │── redirect to OAuth provider ─┤
  │   ?code_challenge=...         │
  │                               │
  │── callback with code ────────►│
  │   + code_verifier             │
  │                               ├── verify: SHA256(verifier) == challenge?
  │                               ├── YES → exchange code for token
  │◄── session ────────────────── │
```

### Magic Link

ส่งลิงก์ magic ทางอีเมล — ผู้ใช้คลิก → เข้าสู่ระบบทันที:

```typescript
import { createAuth, sendMagicLink } from '@ruvyxa/auth'

// Server: ส่ง magic link
export const requestMagicLink = auth(async (email: string) => {
  // ตรวจสอบอีเมล
  if (!email || !email.includes('@')) {
    throw new Error('กรุณากรอกอีเมลที่ถูกต้อง')
  }

  // ส่ง magic link
  await sendMagicLink({
    email,
    url: `${process.env.RUVYXA_PUBLIC_SITE_URL}/auth/callback`,
    expiresIn: 60 * 60, // ลิงก์หมดอายุใน 1 ชม.
  })

  return { success: true }
})
```

**Magic Link Token**:

- Default expiry: 1 hour
- Single-use token (ใช้แล้วหมดอายุทันที)
- เก็บ hash ของ token ใน database (ไม่เก็บ plaintext)

### WebAuthn (Passkeys)

WebAuthn หรือ passkeys — เข้าสู่ระบบด้วย biometrics (fingerprint, face ID) หรือ PIN:

```tsx
'use client'

import { useWebAuthn } from '@ruvyxa/auth/client'

export default function WebAuthnSection() {
  const {
    register, // ลงทะเบียน passkey
    authenticate, // เข้าสู่ระบบด้วย passkey
    isSupported, // browser รองรับ?
    isLoading, // กำลังทำงาน?
    error, // error message (ถ้ามี)
  } = useWebAuthn()

  if (!isSupported) {
    return (
      <div className="alert alert-warning">
        <p>เบราว์เซอร์นี้ไม่รองรับ WebAuthn</p>
        <p>กรุณาใช้ Chrome, Edge, Safari หรือ Firefox ล่าสุด</p>
      </div>
    )
  }

  const handleRegister = async () => {
    try {
      await register()
      alert('ลงทะเบียน Passkey สำเร็จ!')
    } catch (e) {
      console.error('WebAuthn register failed:', e)
    }
  }

  const handleAuthenticate = async () => {
    try {
      await authenticate()
      alert('เข้าสู่ระบบด้วย Passkey สำเร็จ!')
    } catch (e) {
      console.error('WebAuthn auth failed:', e)
    }
  }

  return (
    <div>
      <h2>Passkeys (WebAuthn)</h2>
      <p>เข้าสู่ระบบด้วยลายนิ้วมือ ใบหน้า หรือ PIN</p>

      <button onClick={handleRegister} disabled={isLoading}>
        ลงทะเบียน Passkey
      </button>
      <button onClick={handleAuthenticate} disabled={isLoading}>
        เข้าสู่ระบบด้วย Passkey
      </button>

      {error && <p className="error">{error}</p>}
    </div>
  )
}
```

### Protected Layout — 3 รูปแบบ

```tsx
// รูปแบบ 1: Server-side redirect
// app/(protected)/layout.tsx
import { auth } from '@ruvyxa/auth'
import { redirect } from 'ruvyxa/server'

export default async function ProtectedLayout({ children }: { children: React.ReactNode }) {
  const session = await auth()

  if (!session) {
    redirect('/auth/login?callbackUrl=/dashboard')
  }

  return (
    <div>
      <nav>
        <span>ยินดีต้อนรับ, {session.user.name}</span>
        <a href="/api/auth/signout">ออกจากระบบ</a>
      </nav>
      <main>{children}</main>
    </div>
  )
}

// รูปแบบ 2: Client-side guard
// app/protected/page.tsx
;('use client')
import { useSession } from '@ruvyxa/auth/client'
import { useRouter } from 'ruvyxa/client'

export default function ProtectedPage() {
  const { data: session, status } = useSession()
  const router = useRouter()

  if (status === 'loading') return <p>กำลังตรวจสอบ...</p>
  if (!session) {
    router.push('/auth/login')
    return null
  }

  return <div>เนื้อหาสำหรับสมาชิกเท่านั้น</div>
}

// รูปแบบ 3: Middleware (edge)
// app/middleware.ts
import { auth } from '@ruvyxa/auth'
import { NextResponse } from 'ruvyxa/server'

export async function middleware(request: Request) {
  const session = await auth()
  const url = new URL(request.url)

  if (!session && url.pathname.startsWith('/dashboard')) {
    return NextResponse.redirect(new URL('/auth/login', request.url))
  }

  return NextResponse.next()
}

export const config = {
  matcher: ['/dashboard/:path*', '/admin/:path*'],
}
```

### Auth Plugin — Auto Endpoints

เมื่อลงทะเบียน `@ruvyxa/auth` plugin, Ruvyxa สร้าง endpoints อัตโนมัติ:

| Endpoint                           | Method | คำอธิบาย                   |
| ---------------------------------- | ------ | -------------------------- |
| `GET /auth/session`                | GET    | ดู session ปัจจุบัน (JSON) |
| `POST /auth/signin/:provider`      | POST   | Sign in ด้วย provider      |
| `GET /auth/signin/:provider`       | GET    | OAuth redirect             |
| `POST /auth/signout`               | POST   | Sign out                   |
| `GET /auth/callback/:provider`     | GET    | OAuth callback             |
| `GET /auth/verify-request`         | GET    | Magic link sent page       |
| `GET /auth/error`                  | GET    | Error page                 |
| `POST /auth/webauthn/register`     | POST   | WebAuthn register          |
| `POST /auth/webauthn/authenticate` | POST   | WebAuthn authenticate      |

### Auth Stores

Session store — ใช้เก็บ session data:

```typescript
// Session store interface
interface SessionStore {
  get(sessionId: string): Promise<Session | null>
  set(sessionId: string, data: Session, expiresAt: Date): Promise<void>
  delete(sessionId: string): Promise<void>
  deleteMany(userId: string): Promise<void> // ลบทุก session ของ user
}

// Built-in stores:
// - 'jwt' — ไม่ต้อง store (JWT token มีข้อมูลครบ)
// - 'database' — ใช้ Prisma หรือ adapter ที่ระบุ
// - 'redis' — ใช้ Redis (ต้องตั้ง connection)

// Custom store
const myStore: SessionStore = {
  async get(id) {
    return db.findSession(id)
  },
  async set(id, data, expiresAt) {
    /* ... */
  },
  async delete(id) {
    /* ... */
  },
  async deleteMany(userId) {
    /* ... */
  },
}

// Config
// ruvyxa.config.ts
plugins: [
  {
    name: '@ruvyxa/auth',
    options: {
      session: {
        strategy: 'jwt',
        store: myStore, // custom store
      },
    },
  },
]
```

Verification store (สำหรับ magic link):

```typescript
interface VerificationStore {
  get(token: string): Promise<VerificationToken | null>
  set(token: string, data: VerificationToken, expiresAt: Date): Promise<void>
  delete(token: string): Promise<void>
}
```

### Auth Events / Callbacks

```typescript
interface AuthCallbacks {
  signIn?: (params: {
    user: UserProfile
    account: Account
    profile?: any
  }) => Promise<boolean | string> // false → reject, string → redirectTo

  redirect?: (params: { url: string; baseUrl: string }) => Promise<string> // ปรับ redirect URL

  session?: (params: { session: Session; token: JWT }) => Promise<Session> // ปรับ session object

  jwt?: (params: { token: JWT; user?: UserProfile; account?: Account }) => Promise<JWT> // ปรับ JWT claims
}

interface AuthEvents {
  createUser?: (params: { user: UserProfile }) => Promise<void>
  linkAccount?: (params: { user: UserProfile; account: Account }) => Promise<void>
  session?: (params: { session: Session }) => Promise<void>
  signOut?: (params: { session: Session }) => Promise<void>
}
```

### Rate Limiting — RUV3102

ทุกครั้งที่มีการพยายามยืนยันตัวตน ระบบจะหักโควตาจาก **สอง bucket ที่แยกกัน** ถ้า bucket ใด bucket
หนึ่งหมด จะได้ `RUV3102 Too many authentication attempts` พร้อม header `Retry-After` (หน่วยวินาที)

| Bucket       | Key                       | โควตา               | กันอะไร                      |
| ------------ | ------------------------- | ------------------- | ---------------------------- |
| ต่อ identity | scope + identity + client | `rateLimit.max`     | ถล่มบัญชีเดียวซ้ำๆ           |
| ต่อ client   | client เท่านั้น           | `rateLimit.max` × 5 | client เดียวไล่กวาดหลายบัญชี |

key ของ bucket แรกมี email อยู่ด้วย ลำพัง bucket นี้จึงยอมให้ client เดียวลองรหัสผ่านได้ `max` ครั้ง
**ต่อบัญชี โดยไม่จำกัดจำนวนบัญชี** ซึ่งเป็นรูปแบบของ credential stuffing
และการกวาดหาบัญชีที่มีอยู่จริง bucket ที่สองจึงมาปิดยอดรวมนั้น โควตาของมันถูกตั้งให้สูงกว่าเพื่อให้
egress ที่แชร์กัน (ออฟฟิศ, mobile carrier, CGNAT) ยังใช้งานได้ เพราะ client
ปกติแทบไม่ล็อกอินด้วยหลาย identity ติดกัน

ทั้งสอง bucket ใช้ client key เดียวกัน ซึ่งเป็น client IP ที่ resolve ได้เมื่อตั้ง `clientIp` ไว้
และถ้าไม่ได้ตั้ง จะ fallback ไปใช้ user-agent ที่ตัดความยาวแล้ว **ควรตั้ง `clientIp` ใน production**
เพราะ user-agent เป็นค่าที่ client กำหนดเองได้ จึงหมุนเปลี่ยนเพื่อหลบ rate limit ได้

---

## @ruvyxa/database

Typed database facade — รองรับ Prisma, DynamoDB, และ custom adapters

### Type Definitions

```typescript
// ===== createDatabase =====

interface DatabaseConfig {
  adapter: 'prisma' | 'dynamodb' | DatabaseAdapter
  url?: string // Connection string (optional)
}

interface DatabaseAdapter {
  name: string
  connect(config: any): Promise<void>
  disconnect(): Promise<void>
  query(model: string, operation: string, args: any[]): Promise<any>
  transaction<T>(fn: (tx: TransactionAdapter) => Promise<T>): Promise<T>
}

interface TransactionAdapter {
  query(model: string, operation: string, args: any[]): Promise<any>
}

// ===== Query Methods (Prisma-style) =====

interface PrismaLikeAPI {
  // CRUD
  findUnique(params: { where: any; select?: any; include?: any }): Promise<any>
  findMany(params: {
    where?: any
    select?: any
    include?: any
    orderBy?: any
    skip?: number
    take?: number
  }): Promise<any[]>
  findFirst(params: { where: any; select?: any; orderBy?: any }): Promise<any>
  create(params: { data: any; select?: any }): Promise<any>
  update(params: { where: any; data: any; select?: any }): Promise<any>
  upsert(params: { where: any; create: any; update: any; select?: any }): Promise<any>
  delete(params: { where: any }): Promise<any>
  deleteMany(params: { where: any }): Promise<{ count: number }>
  count(params: { where?: any }): Promise<number>
  aggregate(params: {
    where?: any
    _count?: any
    _sum?: any
    _avg?: any
    _min?: any
    _max?: any
  }): Promise<any>
  groupBy(params: { by: string[]; where?: any; _count?: any; _sum?: any }): Promise<any[]>
}

// ===== database() return type =====

function database<Models = any>(): Models extends Record<string, any>
  ? { [K in keyof Models]: PrismaLikeAPI }
  : PrismaLikeAPI

// ===== Query Options =====
interface QueryOptions {
  timeout?: number // Query timeout (ms)
  transaction?: TransactionAdapter // Use existing transaction
  raw?: boolean // Return raw result
}
```

### ติดตั้ง

```bash
npm install @ruvyxa/database
npm install @prisma/client    # ถ้าใช้ Prisma
npm install @aws-sdk/client-dynamodb  # ถ้าใช้ DynamoDB
```

### Plugin Registration

```ts
// ruvyxa.config.ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  plugins: [
    {
      name: '@ruvyxa/database',
      options: {
        adapter: 'prisma', // 'prisma' | 'dynamodb' | DatabaseAdapter
        url: process.env.DATABASE_URL, // Optional — สำหรับ custom adapter
      },
    },
  ],
})
```

### Prisma Adapter — ละเอียด

```typescript
// server/db.ts
'use server'

import { database } from '@ruvyxa/database'

const db = database()

// ประเภทข้อมูล — อัตโนมัติจาก Prisma schema
// Schema:
//   model User {
//     id        String   @id @default(cuid())
//     name      String
//     email     String   @unique
//     posts     Post[]
//     createdAt DateTime @default(now())
//   }
//
//   model Post {
//     id        String   @id @default(cuid())
//     title     String
//     content   String?
//     published Boolean  @default(false)
//     author    User     @relation(fields: [authorId], references: [id])
//     authorId  String
//   }

// === CRUD Operations ===

export async function getUser(id: string) {
  return db.user.findUnique({
    where: { id },
    select: {
      id: true,
      name: true,
      email: true,
      createdAt: true,
    },
  })
}

export async function getUsers(params?: {
  limit?: number
  offset?: number
  orderBy?: 'name' | 'email' | 'createdAt'
}) {
  return db.user.findMany({
    take: params?.limit || 10,
    skip: params?.offset || 0,
    orderBy: params?.orderBy ? { [params.orderBy]: 'asc' } : undefined,
    select: { id: true, name: true, email: true },
  })
}

export async function createUser(data: { name: string; email: string }) {
  return db.user.create({ data })
}

export async function updateUser(id: string, data: Partial<{ name: string; email: string }>) {
  return db.user.update({
    where: { id },
    data,
  })
}

export async function deleteUser(id: string) {
  return db.user.delete({ where: { id } })
}

// === Relations ===

export async function getUserWithPosts(userId: string) {
  return db.user.findUnique({
    where: { id: userId },
    include: {
      posts: {
        where: { published: true },
        orderBy: { createdAt: 'desc' },
        take: 10,
      },
    },
  })
}

// === Aggregations ===

export async function getUserStats() {
  return db.user.aggregate({
    _count: true,
    _avg: {/* fields */},
  })
}

// === Transactions ===

export async function createUserWithPost(userData: any, postData: any) {
  return db.$transaction(async (tx) => {
    const user = await tx.user.create({ data: userData })
    const post = await tx.post.create({
      data: { ...postData, authorId: user.id },
    })
    return { user, post }
  })
}
```

### DynamoDB Adapter

```typescript
// server/db.ts
'use server'

import { database } from '@ruvyxa/database'

const db = database()

// === Basic Operations ===

export async function getUser(id: string) {
  return db.get({
    TableName: 'Users',
    Key: { id },
  })
}

export async function createUser(item: {
  id: string
  name: string
  email: string
  createdAt: string
}) {
  return db.put({
    TableName: 'Users',
    Item: item,
    ConditionExpression: 'attribute_not_exists(id)', // ป้องกัน overwrite
  })
}

export async function queryByEmail(email: string) {
  return db.query({
    TableName: 'Users',
    IndexName: 'EmailIndex',
    KeyConditionExpression: 'email = :email',
    ExpressionAttributeValues: {
      ':email': email,
    },
  })
}

// === Batch Operations ===

export async function batchGetUsers(ids: string[]) {
  return db.batchGet({
    RequestItems: {
      Users: {
        Keys: ids.map((id) => ({ id })),
      },
    },
  })
}

export async function batchCreateUsers(items: any[]) {
  return db.batchWrite({
    RequestItems: {
      Users: items.map((item) => ({
        PutRequest: { Item: item },
      })),
    },
  })
}
```

### Custom Adapter — สร้างเอง

```typescript
// server/mongo-adapter.ts
import type { DatabaseAdapter } from '@ruvyxa/database'

export const mongoAdapter: DatabaseAdapter = {
  name: 'mongodb',

  async connect(config) {
    const { MongoClient } = await import('mongodb')
    this.client = new MongoClient(config.url || process.env.MONGODB_URL!)
    await this.client.connect()
    this.db = this.client.db()
  },

  async disconnect() {
    await this.client?.close()
  },

  async query(model, operation, args) {
    const collection = this.db.collection(model)

    switch (operation) {
      case 'findUnique':
        return collection.findOne(args[0].where)
      case 'findMany':
        return collection
          .find(args[0].where || {})
          .sort(args[0].orderBy)
          .skip(args[0].skip || 0)
          .limit(args[0].take || 100)
          .toArray()
      case 'create':
        const result = await collection.insertOne(args[0].data)
        return { ...args[0].data, _id: result.insertedId }
      case 'update':
        await collection.updateOne(args[0].where, { $set: args[0].data })
        return collection.findOne(args[0].where)
      case 'delete':
        return collection.deleteOne(args[0].where)
      default:
        throw new Error(`Unknown operation: ${operation}`)
    }
  },

  async transaction(fn) {
    // MongoDB transactions (replica set required)
    const session = this.client.startSession()
    try {
      session.startTransaction()
      const result = await fn({
        query: (model, op, args) => this.query(model, op, args),
      })
      await session.commitTransaction()
      return result
    } catch (e) {
      await session.abortTransaction()
      throw e
    } finally {
      session.endSession()
    }
  },
}
```

```ts
// ruvyxa.config.ts
import { defineConfig } from 'ruvyxa/config'
import { mongoAdapter } from './server/mongo-adapter'

export default defineConfig({
  plugins: [
    {
      name: '@ruvyxa/database',
      options: {
        adapter: mongoAdapter,
      },
    },
  ],
})
```

### Server-Only — Design Pattern

```typescript
// server/db.ts — Server-only module
// ⚠️ ห้าม import ไฟล์นี้จาก client component

import { database } from '@ruvyxa/database'

const db = database()

export async function getDashboardData(userId: string) {
  const [user, posts, stats] = await Promise.all([
    db.user.findUnique({ where: { id: userId } }),
    db.post.findMany({ where: { authorId: userId }, take: 5 }),
    db.stats.findOne({ userId }),
  ])

  return {
    user,
    recentPosts: posts,
    stats,
  }
}
```

การใช้ผ่าน server action:

```typescript
// app/actions.ts
'use server'

import { action } from 'ruvyxa/server'
import { getDashboardData } from '../server/db'

export const fetchDashboard = action(async (userId: string) => {
  return getDashboardData(userId)
})
```

```tsx
// app/dashboard/page.tsx
'use client'

import { fetchDashboard } from './actions'

export default function DashboardPage({ userId }: { userId: string }) {
  const [data, setData] = useState(null)

  useEffect(() => {
    fetchDashboard(userId).then(setData)
  }, [userId])

  if (!data) return <p>กำลังโหลด...</p>

  return (
    <div>
      <h1>ยินดีต้อนรับ, {data.user.name}</h1>
      {/* ... */}
    </div>
  )
}
```

### Database Error Codes

| Error                      | สาเหตุ                             | วิธีแก้                          |
| -------------------------- | ---------------------------------- | -------------------------------- |
| Prisma connection error    | `DATABASE_URL` ผิด หรือ DB offline | ตรวจ connection string           |
| Prisma model not found     | ไม่ได้รัน `prisma generate`        | `npx prisma generate`            |
| Prisma migration pending   | schema เปลี่ยน                     | `npx prisma migrate deploy`      |
| DynamoDB timeout           | Query ช้า                          | ใช้ index หรือเพิ่ม timeout      |
| DynamoDB capacity exceeded | RCU/WCU หมด                        | เพิ่ม capacity หรือใช้ on-demand |
| Transaction conflict       | Concurrent write conflict          | retry หรือปรับ isolation         |
| Connection pool full       | เกิน max connections               | เพิ่ม pool size ใน config        |
| Query timeout              | Query ใช้เวลา > 30s                | Optimize query, เพิ่ม index      |

---

## @ruvyxa/realtime

WebSocket-based realtime communication — channels, broadcast, presence

### Type Definitions

```typescript
// ===== Server API =====

interface RealtimeConfig {
  path?: string // WebSocket endpoint (default: '/ws')
  maxConnections?: number // Max connections (default: 1000)
  heartbeatInterval?: number // Heartbeat seconds (default: 30)
  authTimeout?: number // Auth timeout ms (default: 5000)
  messageSizeLimit?: number // Message size bytes (default: 1MB)
  rateLimit?: {
    messagesPerSecond: number // Max messages/s per connection
    burstSize?: number // Burst size
  }
  presence?: boolean // เปิด presence tracking (default: false)
}

interface RealtimeInstance {
  // Broadcast
  broadcast(channel: string, event: string, data: any): void
  broadcastToUser(userId: string, event: string, data: any): void
  broadcastToConnections(connectionIds: string[], event: string, data: any): void

  // Channel management
  channel(name: string): Channel
  channels(): string[] // รายชื่อ channels ทั้งหมด
  connections(channel?: string): number // จำนวน connections

  // Connection events
  onConnection(handler: (socket: Socket) => void): void
  onDisconnection(handler: (socket: Socket) => void): void

  // Presence
  presence(): PresenceManager // ถ้าเปิด presence

  // Shutdown
  close(): Promise<void>
}

interface Socket {
  id: string
  userId?: string
  channels: Set<string>
  connectedAt: Date
  lastActivity: Date
  metadata: Record<string, any>

  send(event: string, data: any): void
  join(channel: string): void
  leave(channel: string): void
  disconnect(): void

  on(event: string, handler: (...args: any[]) => void): void
  onDisconnect(handler: () => void): void
}

interface Channel {
  name: string
  connections: number
  broadcast(event: string, data: any, exclude?: string): void
  onSubscribe(handler: (socket: Socket) => void): void
  onUnsubscribe(handler: (socket: Socket) => void): void
}

interface PresenceManager {
  get(channel: string): Map<string, PresenceData>
  update(userId: string, data: Partial<PresenceData>): void
  onJoin(handler: (userId: string, data: PresenceData) => void): void
  onLeave(handler: (userId: string, data: PresenceData) => void): void
  onUpdate(handler: (userId: string, data: PresenceData) => void): void
}

interface PresenceData {
  userId: string
  name?: string
  status?: 'online' | 'away' | 'busy'
  lastSeen: Date
  metadata?: Record<string, any>
}

// ===== Client API =====

interface RealtimeClient {
  connect(): Promise<void>
  disconnect(): void
  subscribe(channel: string, handlers: ChannelHandlers): Unsubscribe
  send(channel: string, event: string, data: any): void
  presence(): ClientPresenceManager
}

interface ChannelHandlers {
  onMessage?: (event: string, data: any) => void
  onError?: (error: Error) => void
  onSubscribed?: () => void
  onUnsubscribed?: () => void
}

// ===== Client Hooks =====

function useChannel(
  channel: string | null, // null = ไม่ subscribe
  handlers: ChannelHandlers,
): {
  send: (event: string, data: any) => void
  connected: boolean
  connectionCount: number
}

function usePresence(channel: string): {
  users: Map<string, PresenceData>
  update: (data: Partial<PresenceData>) => void
}
```

### ติดตั้ง

```bash
npm install @ruvyxa/realtime

# dependencies สำหรับ production
npm install ws    # WebSocket server
```

### Plugin Registration

```ts
// ruvyxa.config.ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  plugins: [
    {
      name: '@ruvyxa/realtime',
      options: {
        path: '/ws', // WebSocket endpoint
        maxConnections: 1000, // สูงสุด 1000 connections
        heartbeatInterval: 30, // heartbeat ทุก 30s
        authTimeout: 5000, // timeout auth 5s
        messageSizeLimit: 1_048_576, // 1MB ต่อ message
        rateLimit: {
          messagesPerSecond: 10, // 10 messages/s ต่อ connection
          burstSize: 20,
        },
        presence: true, // เปิด presence
      },
    },
  ],
})
```

### WebSocket Protocol

```
Client                    Server
  │                         │
  │── WS /ws ─────────────► │  (Upgrade: WebSocket)
  │                         │
  │── auth: { token } ────► │  (Authentication)
  │◄── ack: { ok } ──────── │
  │                         │
  │── subscribe: chat:room1 ►│  (Subscribe to channel)
  │◄── subscribed: chat:room1│
  │                         │
  │── message: {            │
  │     channel: chat:room1,│
  │     event: 'hello',     │
  │     data: { text }      │
  │   } ──────────────────► │
  │                         ├── Broadcast to room
  │◄── message: {           │
  │     channel: chat:room1,│
  │     event: 'hello',     │
  │     data: { text }      │
  │   } ─────────────────── │
  │                         │
  │── ping ───────────────► │
  │◄── pong ─────────────── │
  │                         │
  │── unsubscribe: chat:room1►
  │◄── unsubscribed        │
  │                         │
```

**Message Format**:

```typescript
// Client → Server
interface ClientMessage {
  type: 'subscribe' | 'unsubscribe' | 'message' | 'auth' | 'ping' | 'presence:update'
  channel?: string
  event?: string
  data?: any
  token?: string // สำหรับ auth
}

// Server → Client
interface ServerMessage {
  type:
    | 'message'
    | 'subscribed'
    | 'unsubscribed'
    | 'error'
    | 'pong'
    | 'presence:join'
    | 'presence:leave'
    | 'presence:update'
  channel?: string
  event?: string
  data?: any
  error?: string
  code?: string
}
```

### Server API — ละเอียด

```typescript
// app/realtime/server.ts
'use server'

import { createRealtime } from '@ruvyxa/realtime'

const { realtime } = createRealtime()

// === Broadcast ===

export async function sendMessage(channel: string, message: string, userId: string) {
  const rt = realtime()

  rt.broadcast(channel, 'message', {
    text: message,
    userId,
    timestamp: Date.now(),
  })
}

// === Send to specific user ===

export async function notifyUser(userId: string, notification: any) {
  const rt = realtime()
  rt.broadcastToUser(userId, 'notification', {
    ...notification,
    timestamp: Date.now(),
  })
}

// === Connection events ===

realtime().onConnection((socket) => {
  console.log(`Client connected: ${socket.id}`)
  console.log(`Active connections: ${realtime().connections()}`)

  socket.on('disconnect', () => {
    console.log(`Client disconnected: ${socket.id}`)
  })

  // Per-socket events
  socket.on('typing', (data) => {
    socket.broadcastToChannel('chat:general', 'typing', data)
  })
})

// === Channel management ===

realtime().onConnection((socket) => {
  socket.join('chat:general') // เข้าห้องทันที
  socket.join(`user:${socket.userId}`) // ห้องส่วนตัว
})

// === Presence ===

realtime()
  .presence()
  .onJoin((userId, data) => {
    console.log(`User ${userId} is now online`)
    realtime().broadcast('presence', 'user:online', { userId, data })
  })

realtime()
  .presence()
  .onLeave((userId, data) => {
    console.log(`User ${userId} went offline`)
    realtime().broadcast('presence', 'user:offline', { userId })
  })
```

### Client API — ละเอียด

```tsx
'use client'

import { useChannel, usePresence } from '@ruvyxa/realtime/client'
import { useState, useEffect } from 'react'

// === Chat Component ===

export default function ChatRoom({ roomId, userId }: { roomId: string; userId: string }) {
  const [messages, setMessages] = useState<
    Array<{ text: string; userId: string; timestamp: number }>
  >([])
  const [input, setInput] = useState('')
  const [typingUsers, setTypingUsers] = useState<string[]>([])

  const channel = useChannel(`chat:${roomId}`, {
    onMessage: (event, data) => {
      if (event === 'message') {
        setMessages((prev) => [...prev, data])
      }
      if (event === 'typing') {
        setTypingUsers((prev) => (prev.includes(data.userId) ? prev : [...prev, data.userId]))
        setTimeout(() => {
          setTypingUsers((prev) => prev.filter((id) => id !== data.userId))
        }, 2000)
      }
    },
    onSubscribed: () => {
      console.log('Joined channel:', roomId)
    },
    onError: (error) => {
      console.error('Channel error:', error)
    },
  })

  async function send() {
    if (!input.trim()) return
    await channel.send('message', { text: input, userId })
    setInput('')
  }

  useEffect(() => {
    const timeout = setTimeout(() => {
      channel.send('typing', { userId })
    }, 100)
    return () => clearTimeout(timeout)
  }, [input])

  return (
    <div className="chat-room">
      <div className="messages">
        {messages.map((msg, i) => (
          <div key={i} className={`message ${msg.userId === userId ? 'mine' : 'theirs'}`}>
            <p>{msg.text}</p>
            <small>{new Date(msg.timestamp).toLocaleTimeString('th-TH')}</small>
          </div>
        ))}
      </div>

      {typingUsers.length > 0 && <p className="typing">{typingUsers.length} คนกำลังพิมพ์...</p>}

      <div className="input-area">
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && send()}
          placeholder="พิมพ์ข้อความ..."
        />
        <button onClick={send}>ส่ง</button>
      </div>
    </div>
  )
}

// === Presence (แสดงสถานะผู้ใช้) ===

export function OnlineUsers({ roomId }: { roomId: string }) {
  const { users, update } = usePresence(`chat:${roomId}`)

  useEffect(() => {
    update({ status: 'online' })
  }, [])

  return (
    <div className="online-users">
      <h4>ออนไลน์ ({users.size} คน)</h4>
      <ul>
        {Array.from(users.values()).map((user) => (
          <li key={user.userId} className={`status-${user.status}`}>
            <span className="status-dot" />
            {user.name || user.userId}
          </li>
        ))}
      </ul>
    </div>
  )
}
```

### ตัวอย่าง: Real-time Collaboration (Editor)

```tsx
'use client'

import { useChannel } from '@ruvyxa/realtime/client'
import { useState, useCallback, useRef } from 'react'

interface CursorPosition {
  userId: string
  x: number
  y: number
  color: string
}

export default function CollaborativeEditor({
  docId,
  userId,
  userName,
}: {
  docId: string
  userId: string
  userName: string
}) {
  const [cursors, setCursors] = useState<Map<string, CursorPosition>>(new Map())
  const [content, setContent] = useState('')
  const [users, setUsers] = useState<string[]>([])
  const editorRef = useRef<HTMLTextAreaElement>(null)

  const channel = useChannel(`doc:${docId}`, {
    onMessage: (event, data) => {
      switch (event) {
        case 'cursor':
          setCursors((prev) => {
            const next = new Map(prev)
            if (data.userId === userId) return next
            next.set(data.userId, data)
            return next
          })
          break
        case 'edit':
          setContent(data.content)
          break
        case 'user:join':
          setUsers((prev) => (prev.includes(data.userId) ? prev : [...prev, data.userId]))
          break
        case 'user:leave':
          setUsers((prev) => prev.filter((id) => id !== data.userId))
          setCursors((prev) => {
            const next = new Map(prev)
            next.delete(data.userId)
            return next
          })
          break
      }
    },
    onSubscribed: () => {
      channel.send('user:join', { userId, userName })
    },
  })

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      const rect = editorRef.current?.getBoundingClientRect()
      if (!rect) return
      channel.send('cursor', {
        userId,
        x: e.clientX - rect.left,
        y: e.clientY - rect.top,
        color: userColors[userId % userColors.length],
      })
    },
    [channel, userId],
  )

  const handleEdit = useCallback(
    (newContent: string) => {
      setContent(newContent)
      channel.send('edit', { content: newContent, userId })
    },
    [channel, userId],
  )

  return (
    <div className="editor-container" onMouseMove={handleMouseMove}>
      <div className="editor-toolbar">
        <span>แก้ไขโดย: {userName}</span>
        <span>ผู้ใช้ออนไลน์: {users.length} คน</span>
      </div>

      <div className="editor-wrapper" style={{ position: 'relative' }}>
        <textarea
          ref={editorRef}
          value={content}
          onChange={(e) => handleEdit(e.target.value)}
          className="editor-textarea"
        />

        {/* Cursor overlays */}
        {Array.from(cursors.values()).map((cursor) => (
          <div
            key={cursor.userId}
            className="cursor-overlay"
            style={{
              position: 'absolute',
              left: cursor.x,
              top: cursor.y,
              width: 2,
              height: 20,
              backgroundColor: cursor.color,
              pointerEvents: 'none',
              transition: 'all 0.1s ease',
            }}
          />
        ))}
      </div>
    </div>
  )
}

const userColors = [
  '#ff0000',
  '#00ff00',
  '#0000ff',
  '#ff00ff',
  '#00ffff',
  '#ffa500',
  '#800080',
  '#008000',
]
```

### ตัวอย่าง: Real-time Dashboard

```tsx
'use client'

import { useChannel } from '@ruvyxa/realtime/client'
import { useState, useEffect } from 'react'

interface Metric {
  name: string
  value: number
  previousValue: number
  unit: string
  trend: 'up' | 'down' | 'stable'
  timestamp: number
}

export default function LiveDashboard() {
  const [metrics, setMetrics] = useState<Metric[]>([])
  const [history, setHistory] = useState<Record<string, number[]>>({})

  useChannel('dashboard:metrics', {
    onMessage: (event, data: Metric) => {
      if (event === 'update') {
        setMetrics((prev) => {
          const idx = prev.findIndex((m) => m.name === data.name)
          const updated = { ...data, previousValue: prev[idx]?.value ?? data.value }

          if (idx >= 0) {
            const next = [...prev]
            next[idx] = updated
            return next
          }
          return [...prev, updated]
        })

        // เก็บ history
        setHistory((prev) => ({
          ...prev,
          [data.name]: [...(prev[data.name] || []).slice(-20), data.value],
        }))
      }
    },
  })

  return (
    <div className="dashboard">
      <h1>Live Dashboard</h1>
      <p>อัปเดตแบบ real-time</p>

      <div className="metrics-grid">
        {metrics.map((metric) => (
          <div key={metric.name} className={`metric-card trend-${metric.trend}`}>
            <h3>{metric.name}</h3>
            <p className="metric-value">
              {metric.value.toLocaleString()}
              <span className="metric-unit">{metric.unit}</span>
            </p>
            <div className="metric-trend">
              {metric.trend === 'up' && '↑'}
              {metric.trend === 'down' && '↓'}
              {metric.trend === 'stable' && '→'}{' '}
              {metric.previousValue > 0 && (
                <span>
                  {Math.round(((metric.value - metric.previousValue) / metric.previousValue) * 100)}
                  %
                </span>
              )}
            </div>
            <small>{new Date(metric.timestamp).toLocaleTimeString('th-TH')}</small>
          </div>
        ))}
      </div>

      {/* Mini sparkline charts */}
      <div className="sparklines">
        {Object.entries(history).map(([name, values]) => (
          <div key={name} className="sparkline">
            <h4>{name}</h4>
            <Sparkline data={values} width={200} height={60} />
          </div>
        ))}
      </div>
    </div>
  )
}

function Sparkline({ data, width, height }: { data: number[]; width: number; height: number }) {
  const max = Math.max(...data, 1)
  const min = Math.min(...data, 0)
  const range = max - min || 1
  const points = data
    .map((v, i) => {
      const x = (i / (data.length - 1)) * width
      const y = height - ((v - min) / range) * height
      return `${x},${y}`
    })
    .join(' ')

  return (
    <svg width={width} height={height}>
      <polyline points={points} fill="none" stroke="currentColor" strokeWidth="2" />
    </svg>
  )
}
```

### Server-side Broadcasting — Scheduler

```typescript
// server/realtime/metrics.ts
'use server'

import { createRealtime } from '@ruvyxa/realtime'

const { realtime } = createRealtime()

export async function startMetricBroadcast() {
  // Broadcast metrics ทุก 5 วินาที
  setInterval(async () => {
    const rt = realtime()
    const metrics = await collectMetrics()

    for (const metric of metrics) {
      rt.broadcast('dashboard:metrics', 'update', {
        ...metric,
        timestamp: Date.now(),
      })
    }
  }, 5000)
}

async function collectMetrics() {
  // ตัวอย่าง — ในจริงควรดึงจาก monitoring system
  return [
    {
      name: 'ผู้ใช้กำลังใช้งาน',
      value: Math.floor(Math.random() * 100) + 10,
      unit: 'คน',
      trend: 'up',
    },
    {
      name: 'คำขอ/วินาที',
      value: Math.floor(Math.random() * 200) + 50,
      unit: 'req/s',
      trend: Math.random() > 0.5 ? 'up' : 'down',
    },
    {
      name: 'เวลาเฉลี่ยตอบสนอง',
      value: Math.floor(Math.random() * 100) + 20,
      unit: 'ms',
      trend: 'stable',
    },
  ]
}
```

### Realtime Error Codes

| Error                        | สาเหตุ                         | วิธีแก้                          |
| ---------------------------- | ------------------------------ | -------------------------------- |
| WebSocket connection refused | Server ไม่ได้รัน หรือ path ผิด | ตรวจ `path` ใน config            |
| 401 Unauthorized             | Token ไม่ถูกต้อง               | ตรวจ `AUTH_SECRET`, token expiry |
| 429 Too Many Requests        | Rate limit เกิน                | ลด frequency หรือเพิ่ม limit     |
| 1009 Message too big         | เกิน `messageSizeLimit`        | ลดขนาด message หรือเพิ่ม limit   |
| Channel not found            | Channel ไม่มี                  | สร้าง channel ก่อน subscribe     |
| Connection timeout           | ไม่ได้ heartbeat ภายใน 30s     | ตรวจ network, เปิด heartbeat     |
| Presence not enabled         | ไม่ได้ตั้ง `presence: true`    | เปิดใน config                    |

---

## ตัวอย่างการใช้ร่วมกัน — Todo App เต็มรูปแบบ

### Server Actions

```typescript
// app/actions.ts
'use server'

import { createAuth } from '@ruvyxa/auth'
import { createDatabase } from '@ruvyxa/database'
import { createRealtime } from '@ruvyxa/realtime'

const { auth } = createAuth()
const { database } = createDatabase()
const { realtime } = createRealtime()

export const addTodo = auth(async (text: string) => {
  const session = await auth()
  if (!session) throw new Error('Unauthorized')

  const db = database()
  const todo = await db.todo.create({
    data: {
      text,
      userId: session.user.id,
      done: false,
    },
  })

  const rt = realtime()
  rt.broadcast(`user:${session.user.id}`, 'todo:add', todo)

  return todo
})

export const toggleTodo = auth(async (id: string) => {
  const session = await auth()
  if (!session) throw new Error('Unauthorized')

  const db = database()
  const todo = await db.todo.update({
    where: { id, userId: session.user.id },
    data: { done: true },
  })

  const rt = realtime()
  rt.broadcast(`user:${session.user.id}`, 'todo:toggle', todo)

  return todo
})

export const deleteTodo = auth(async (id: string) => {
  const session = await auth()
  if (!session) throw new Error('Unauthorized')

  const db = database()
  await db.todo.delete({ where: { id, userId: session.user.id } })

  const rt = realtime()
  rt.broadcast(`user:${session.user.id}`, 'todo:delete', { id })

  return { success: true }
})
```

### Client Component

```tsx
'use client'

import { useSession } from '@ruvyxa/auth/client'
import { useChannel } from '@ruvyxa/realtime/client'
import { useState } from 'react'
import { addTodo, toggleTodo, deleteTodo } from './actions'

interface Todo {
  id: string
  text: string
  done: boolean
  userId: string
}

export default function TodoApp() {
  const { data: session } = useSession()
  const [todos, setTodos] = useState<Todo[]>([])
  const [text, setText] = useState('')

  // Subscribe to real-time updates
  useChannel(session ? `user:${session.user.id}` : null, {
    onMessage(event, data) {
      switch (event) {
        case 'todo:add':
          setTodos((prev) => [...prev, data])
          break
        case 'todo:toggle':
          setTodos((prev) => prev.map((t) => (t.id === data.id ? { ...t, done: data.done } : t)))
          break
        case 'todo:delete':
          setTodos((prev) => prev.filter((t) => t.id !== data.id))
          break
      }
    },
  })

  if (!session) {
    return (
      <div className="todo-app">
        <h1>Todo App</h1>
        <p>กรุณาเข้าสู่ระบบเพื่อจัดการงาน</p>
        <a href="/auth/login">เข้าสู่ระบบ</a>
      </div>
    )
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!text.trim()) return
    await addTodo(text)
    setText('')
  }

  return (
    <div className="todo-app">
      <h1>Todo App</h1>
      <p>สวัสดี, {session.user.name}</p>

      <form onSubmit={handleSubmit} className="todo-form">
        <input
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="เพิ่มงานใหม่..."
          className="todo-input"
        />
        <button type="submit" className="todo-add-btn">
          เพิ่ม
        </button>
      </form>

      <ul className="todo-list">
        {todos.map((todo) => (
          <li key={todo.id} className={`todo-item ${todo.done ? 'done' : ''}`}>
            <input type="checkbox" checked={todo.done} onChange={() => toggleTodo(todo.id)} />
            <span className="todo-text">{todo.text}</span>
            <button onClick={() => deleteTodo(todo.id)} className="todo-delete-btn">
              ลบ
            </button>
          </li>
        ))}
      </ul>

      <p className="todo-count">
        เหลือ {todos.filter((t) => !t.done).length} งาน จากทั้งหมด {todos.length} งาน
      </p>
    </div>
  )
}
```

---

## การ Import ที่ถูกต้อง — สรุปสมบูรณ์

| Package            | Server Import      | Client Import             | Server Export                                                                                     | Client Export                                                        |
| ------------------ | ------------------ | ------------------------- | ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `@ruvyxa/auth`     | `@ruvyxa/auth`     | `@ruvyxa/auth/client`     | `createAuth`, `auth`, `signIn`, `signOut`, `generatePKCEChallenge`, `verifyPKCE`, `sendMagicLink` | `useSession`, `signIn`, `signOut`, `useWebAuthn`, `createAuthClient` |
| `@ruvyxa/database` | `@ruvyxa/database` | ❌ (server-only)          | `createDatabase`, `database`                                                                      | ❌                                                                   |
| `@ruvyxa/realtime` | `@ruvyxa/realtime` | `@ruvyxa/realtime/client` | `createRealtime`, `realtime`                                                                      | `useChannel`, `usePresence`, `createRealtimeClient`                  |

---

## ลองทำดู

1. ติดตั้ง `@ruvyxa/auth` และตั้งค่า Google OAuth — ใช้ Client ID + Secret
2. สร้าง protected layout ที่ redirect ไป login ถ้าไม่มี session
3. ใช้ `useSession` ใน client component — แสดงชื่อผู้ใช้
4. ติดตั้ง `@ruvyxa/database` + Prisma — สร้าง schema model
5. สร้าง server action ที่ใช้ database — CRUD ผู้ใช้
6. ติดตั้ง `@ruvyxa/realtime` + สร้าง chat room — broadcast ข้อความ
7. รวม auth + database + realtime ในแอป Todo เดียวกัน
8. ทดสอบ PKCE flow — ใช้กับ native app
9. ตั้งค่า WebAuthn (passkeys) — เข้าสู่ระบบด้วย biometrics
10. สร้าง custom database adapter — MongoDB หรือ SQLite
11. ใช้ presence tracker — แสดงสถานะผู้ใช้ออนไลน์
12. ทดสอบ real-time collaboration editor

---

## Troubleshooting — ฉบับละเอียด

| ปัญหา                           | Error              | สาเหตุ                             | วิธีแก้                                          |
| ------------------------------- | ------------------ | ---------------------------------- | ------------------------------------------------ |
| RUV1007: auth import            | Boundary violation | Import server path ใน client       | ใช้ `/client` path แทน                           |
| Auth session เป็น null          | —                  | ไม่ได้ตั้ง `AUTH_SECRET`           | เพิ่ม `AUTH_SECRET` ใน `.env` (ขั้นต่ำ 32 chars) |
| OAuth callback 404              | —                  | Redirect URI ไม่ตรงกับที่ลงทะเบียน | ตรวจ URL ใน OAuth provider dashboard             |
| OAuth fails with state mismatch | —                  | CSRF state ไม่ตรง                  | ตรวจ cookie support, ล้าง cache                  |
| WebSocket ไม่เชื่อมต่อ          | —                  | Path ไม่ตรงกับ config              | ตรวจ `realtime.path` ใน config                   |
| WebSocket 1006 abnormal close   | —                  | firewall หรือ proxy block          | เปิด port, ตรวจ proxy config                     |
| Prisma model ไม่เจอ             | —                  | ไม่ได้รัน `prisma generate`        | `npx prisma generate`                            |
| Prisma migration pending        | —                  | Schema เปลี่ยน                     | `npx prisma migrate deploy`                      |
| Database connection fail        | —                  | `DATABASE_URL` ผิด                 | ตรวจ connection string, IP whitelist             |
| Realtime lag                    | —                  | มากกว่า max connections            | เพิ่ม `maxConnections`                           |
| Realtime message lost           | —                  | Rate limit                         | ลด frequency หรือเพิ่ม `messagesPerSecond`       |
| Presence ไม่แสดง                | —                  | ไม่ได้เปิด `presence: true`        | เปิดใน config                                    |
| Magic link ไม่ส่ง               | —                  | SMTP ไม่ถูกต้อง                    | ตรวจ SMTP config, spam folder                    |
| WebAuthn ไม่รองรับ              | —                  | Browser เก่า                       | แนะนำ Chrome/Edge/Safari/Firefox ล่าสุด          |
| Auth session ไม่ refresh        | —                  | `updateAge` นานเกิน                | ตั้ง `session.updateAge` เล็กลง                  |
| Transaction deadlock            | —                  | Two writes same row                | รีเทรี่ หรือลด transaction scope                 |
| Connection pool เต็ม            | —                  | เกิน limit                         | เพิ่ม pool size หรือ close idle connections      |

---

## สรุป

- 3 official packages: `@ruvyxa/auth`, `@ruvyxa/database`, `@ruvyxa/realtime`
- ทุก package ลงทะเบียนเป็น plugin — endpoints และ runtime auto
- **Auth**: session (JWT/database), 16+ OAuth providers, PKCE, magic-link, WebAuthn, stores,
  callbacks, events
- **Database**: Prisma, DynamoDB, custom adapter — typed facade CRUD + relations + transactions
- **Realtime**: WebSocket channels, broadcast, presence, hooks, rate limiting, heartbeat
- Import rule ชัดเจน — server-only vs client-safe
- 3 ตัวอย่างรวม: Todo app, chat room, collaborative editor, live dashboard
- Troubleshooting 18 ข้อ — error + สาเหตุ + วิธีแก้

---

## เลือก First-party Package จาก Boundary ที่มันรับผิดชอบ

repository มี first-party integration packages 3 ตัวที่รับผิดชอบต่างกัน:

| Package            | รับผิดชอบ                                                                  | Boundary สำคัญ                                                    |
| ------------------ | -------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `@ruvyxa/auth`     | authentication runtime, provider/session flows และ auth plugin integration | เก็บ credentials และ session handling ไว้ฝั่ง server              |
| `@ruvyxa/database` | typed facade เหนือ database adapter ที่แอปส่งให้                           | ไม่เลือก, install หรือ pool database driver ให้เอง                |
| `@ruvyxa/realtime` | realtime plugin/client integration                                         | มอง delivery เป็น transient event delivery ไม่ใช่ durable storage |

`@ruvyxa/database` เริ่มจาก adapter ที่ระบุชัด แล้วให้ model delegates เช่น `findMany`,
`findUnique`, `create`, `update` และ `delete` โดย validate unsafe model names และบังคับ non-empty
`where` สำหรับ unique/update/delete operations แอปเป็นเจ้าของ connection lifecycle ของ adapter และ
database schema

```ts
import { createDatabase } from '@ruvyxa/database'

const db = createDatabase<AppSchema>(adapter)
const product = await db.Product.findUnique({ where: { id: productId } })
```

`@ruvyxa/auth` ให้ `createAuth()` และ OAuth provider helpers เช่น `google()` กับ `github()` runtime
ทำงานที่ sensitive เช่น same-origin request checks และ session-cookie handling แต่ยังต้องมี
application configuration กับ secure deployment origin อย่า import งาน auth/database ฝั่ง server
เข้า module ที่ client เข้าถึงได้ เพราะ boundary validator มองทั้งสอง package เป็น server-only
specifiers

### Integrate ทีละ Capability

1. install/configure หนึ่ง package ด้วย options ที่เอกสารของมันรองรับ
2. เก็บ secrets ใน server-only module/environment
3. รัน `ruvyxa analyze --format human` เพื่อหา boundary leak
4. ทดสอบ route/action ที่ใช้ integration นั้น
5. รัน `npm run check` ก่อนขยาย integration

ลำดับนี้ช่วยไม่ให้ package ถูกอธิบายเกินว่าเป็น architecture สมบูรณ์ของ auth/database/event delivery
มันให้ framework integration boundary ส่วน data modeling, authorization policy, driver configuration
และ operational monitoring ยังเป็นการตัดสินใจของแอป
