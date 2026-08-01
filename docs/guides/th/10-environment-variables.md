# Environment Variables ใน Ruvyxa

Ruvyxa จัดการ environment variables อย่างปลอดภัย — แยกตัวแปร client-side (สาธารณะ) ออกจาก
server-side (ส่วนตัว) โดยอัตโนมัติ ป้องกัน secrets รั่วไหลไปยังเบราว์เซอร์ พร้อม TypeScript
declarations, prefix detection algorithm, client scanner, และ validation full pipeline

---

## หลักการสำคัญ

```
┌──────────────────────────────────────────────────┐
│               SERVER (Node.js Runtime)            │
│                                                   │
│  RUVYXA_PUBLIC_API_URL=https://api.example.com   │  ✅ เข้าถึงได้
│  DATABASE_URL=postgres://user:pass@localhost/db   │  ✅ เข้าถึงได้
│  AUTH_SECRET=sk-xxxxxxxxxxxxxxxx                 │  ✅ เข้าถึงได้
│  STRIPE_API_KEY=sk_live_xxxxx                    │  ✅ เข้าถึงได้
│                                                   │
└──────────┬───────────────────────────────────────┘
           │ ส่งเฉพาะ RUVYXA_PUBLIC_* + NODE_ENV
           │ ไปยัง client bundle (build-time injection)
           ▼
┌──────────────────────────────────────────────────┐
│               CLIENT (Browser Runtime)            │
│                                                   │
│  RUVYXA_PUBLIC_API_URL=https://api.example.com   │  ✅ เข้าถึงได้
│  NODE_ENV=production                              │  ✅ เข้าถึงได้
│  DATABASE_URL=postgres://...                      │  ❌ undefined
│  AUTH_SECRET=sk-...                               │  ❌ undefined
│  STRIPE_API_KEY=sk_live_...                       │  ❌ undefined
│                                                   │
└──────────────────────────────────────────────────┘
```

### กฎเหล็ก

1. **ตัวแปรที่ขึ้นต้นด้วย `RUVYXA_PUBLIC_`** → ปลอดภัยให้ client เห็น (ถูกฝังใน JS bundle)
2. **ตัวแปรอื่น ๆ ทั้งหมด** → server-only (ไม่ถูกส่งไป client)
3. **`NODE_ENV`** → special case: ใช้ใน client ได้ (แต่ Ruvyxa จัดการให้อัตโนมัติ)
4. **`import.meta.env.MODE`** → alias ของ `NODE_ENV` สำหรับ ESM

---

## ระบบโหลด `.env` — Loading Order Algorithm

### ลำดับการโหลด (Priority: สูง → ต่ำ)

```
Priority สูงสุด
    ↑
    │  1. Shell Environment (process.env ที่มีอยู่แล้วเมื่อรัน CLI)
    │     - Windows: $env:VAR_NAME หรือ set VAR_NAME=value
    │     - Linux/Mac: export VAR_NAME=value
    │     - CI/CD: built-in secrets ใน GitHub Actions, GitLab CI ฯลฯ
    │
    │  2. .env.{environment}.local  (เฉพาะ local — ห้าม commit)
    │     - .env.development.local  → dev
    │     - .env.production.local   → build/start
    │     - .env.test.local         → test
    │
    │  3. .env.local  (ทุก environment — ห้าม commit)
    │
    │  4. .env.{environment}  (เฉพาะ environment)
    │     - .env.development  → dev
    │     - .env.production   → build/start
    │     - .env.test         → test
    │
    │  5. .env  (ค่า default — commit ได้)
    │
    ↓
Priority ต่ำสุด
```

### อัลกอริทึมการโหลด

```
function loadEnvFiles(rootDir: string, mode: 'development' | 'production' | 'test') {
  // 1. กำหนดไฟล์ทั้งหมดที่จะโหลด (เรียงตาม priority ต่ำ → สูง)
  const envFiles = [
    '.env',                                    // #5 default
    `.env.${mode}`,                            // #4 environment-specific
    '.env.local',                              // #3 local override
    `.env.${mode}.local`,                      // #2 environment + local
  ];

  // 2. ตัวแปรที่มีอยู่แล้วใน shell (priority สูงสุด)
  const existingEnv = { ...process.env };

  // 3. parse แต่ละไฟล์
  for (const file of envFiles) {
    const fullPath = path.join(rootDir, file);

    if (!fs.existsSync(fullPath)) continue;

    const parsed = dotenv.parse(fs.readFileSync(fullPath, 'utf-8'));

    for (const [key, value] of Object.entries(parsed)) {
      // IMPORTANT: ตัวแปรที่มีค่าอยู่แล้วจะไม่ถูกแทนที่
      if (!(key in process.env)) {
        process.env[key] = value;
      }
    }
  }

  // 4. กรณีตัวแปรซ้ำ — shell env ชนะเสมอ
  //    (เพราะไม่ได้ overwrite ถ้ามีอยู่แล้ว)
}
```

### ตัวอย่างลำดับการทับ

```bash
# .env — ค่า default (priority 5)
DATABASE_URL=postgres://localhost/dev
API_URL=http://localhost:3000

# .env.production — ค่า production (priority 4)
DATABASE_URL=postgres://prod-server/proddb
API_URL=https://api.example.com

# .env.local — local override (priority 3)
API_URL=http://localhost:8080

# .env.production.local — production + local (priority 2)
DATABASE_URL=postgres://local-prod/proddb

# Shell env (priority 1)
# export API_URL=https://custom.example.com

# ผลลัพธ์:
# DATABASE_URL = postgres://local-prod/proddb   (จาก .env.production.local)
# API_URL      = https://custom.example.com     (จาก shell — priority สูงสุด)
```

### ไฟล์ที่ควร commit vs ไม่ควร

| ไฟล์                     | Commit? | เหตุผล                        |
| ------------------------ | ------- | ----------------------------- |
| `.env`                   | ✅      | ค่า default ที่ปลอดภัย        |
| `.env.example`           | ✅      | Template สำหรับ teammate      |
| `.env.development`       | ✅      | ค่า dev                       |
| `.env.production`        | ✅      | ค่า production (ไม่มี secret) |
| `.env.test`              | ✅      | ค่าสำหรับ test                |
| `.env.local`             | ❌      | Secrets เฉพาะเครื่อง          |
| `.env.*.local`           | ❌      | Secrets เฉพาะเครื่อง          |
| `.env.development.local` | ❌      | Secrets เฉพาะเครื่อง          |
| `.env.production.local`  | ❌      | Secrets production จริง       |

```gitignore
# .gitignore
.env.local
.env.*.local
```

---

## Prefix Detection Algorithm — กลไกภายใน

Ruvyxa ใช้ algorithm นี้เพื่อตรวจสอบว่า env var ใดบ้างที่ส่งไป client:

```
function isClientAccessible(varName: string): boolean {
  // ขั้นตอนที่ 1: Special cases
  if (varName === 'NODE_ENV') return true;
  if (varName === 'RUVYXA_RUNTIME') return false;  // server-only runtime info

  // ขั้นตอนที่ 2: Prefix check
  if (varName.startsWith('RUVYXA_PUBLIC_')) return true;

  // ขั้นตอนที่ 3: Explicit allowlist (Ruvyxa internal)
  const ALLOWED_CLIENT_PREFIXES = [
    'NEXT_PUBLIC_',      // Next.js compatibility
    'PUBLIC_',           // SvelteKit compatibility
    'VITE_',             // Vite compatibility
  ];
  if (ALLOWED_CLIENT_PREFIXES.some(prefix => varName.startsWith(prefix))) {
    return true;
  }

  // ขั้นตอนที่ 4: Everything else is server-only
  return false;
}

function collectClientEnvVars(allVars: Record<string, string>): Record<string, string> {
  const clientVars: Record<string, string> = {};

  for (const [key, value] of Object.entries(allVars)) {
    if (isClientAccessible(key)) {
      clientVars[key] = value;
    }
  }

  return clientVars;
}
```

### Live Example

```bash
# Input env vars
RUVYXA_PUBLIC_API_URL=https://api.example.com    → client ✅
RUVYXA_PUBLIC_GA_ID=G-XXXXXXXXXX                  → client ✅
NODE_ENV=development                               → client ✅ (special)
DATABASE_URL=postgres://localhost/db               → server-only ❌
AUTH_SECRET=sk-xxxx                                 → server-only ❌
STRIPE_API_KEY=sk_live_xxxxx                       → server-only ❌
MY_APP_SECRET=secret                                → server-only ❌
PUBLIC_STRIPE_KEY=pk_test_xxxxx                    → client ✅ (Vite compat)
```

---

## process.env vs import.meta.env — ความแตกต่าง

| คุณสมบัติ                           | `process.env`                                     | `import.meta.env`            |
| ----------------------------------- | ------------------------------------------------- | ---------------------------- |
| Runtime                             | Node.js (server) + Browser (client, เฉพาะ public) | ESM (ทั้ง server และ client) |
| Server Components                   | ✅                                                | ✅                           |
| Client Components                   | ✅ (เฉพาะ RUVYXA_PUBLIC_*)                        | ✅ (เฉพาะ RUVYXA_PUBLIC_*)   |
| Type Safety                         | `NodeJS.ProcessEnv` interface                     | `ImportMetaEnv` interface    |
| Auto-complete                       | ✅ ถ้ามี declaration                              | ✅ ถ้ามี declaration         |
| Build-time replacement              | ✅ Ruvyxa แทนที่ค่าตอน build                      | ✅ Ruvyxa แทนที่ค่าตอน build |
| Dynamic access (`process.env[var]`) | ✅ (แต่ไม่ recommend)                             | ❌ (ต้อง static string)      |
| Tree-shaking                        | ✅                                                | ✅ ดีกว่า (static analysis)  |

### ตัวอย่างเปรียบเทียบ

```tsx
// Server Component — ใช้ได้ทั้งสองแบบ
export default function ServerPage() {
  // process.env (Node.js style)
  console.log(process.env.RUVYXA_PUBLIC_API_URL)
  console.log(process.env.DATABASE_URL) // ✅ server-only

  // import.meta.env (ESM style)
  console.log(import.meta.env.RUVYXA_PUBLIC_API_URL)
  console.log(import.meta.env.DATABASE_URL) // ✅ server-only
  console.log(import.meta.env.MODE) // 'development' | 'production'

  return <div>Server Component</div>
}
```

```tsx
// Client Component
'use client'

export default function ClientPage() {
  // process.env — เฉพาะ RUVYXA_PUBLIC_* + NODE_ENV
  console.log(process.env.RUVYXA_PUBLIC_API_URL) // ✅
  console.log(process.env.NODE_ENV) // ✅
  console.log(process.env.DATABASE_URL) // ❌ RUV1008

  // import.meta.env — เฉพาะ public
  console.log(import.meta.env.RUVYXA_PUBLIC_API_URL) // ✅
  console.log(import.meta.env.MODE) // ✅
  console.log(import.meta.env.DATABASE_URL) // ❌ RUV1008

  return <div>Client Component</div>
}
```

---

## การเข้าถึง Environment Variables (Accessing Environment Variables)

### At Runtime (ขณะรันไทม์)

```tsx
// ใช้งานได้ทุกที่ (ทั้งฝั่ง Server และ Client)
const siteUrl = process.env.RUVYXA_PUBLIC_SITE_URL
const siteUrl = import.meta.env.RUVYXA_PUBLIC_SITE_URL // ESM alias

// ใช้งานเฉพาะ Server เท่านั้น -- หากใช้ในโค้ดฝั่ง Client จะเกิด RUV1008
const dbUrl = process.env.DATABASE_URL
```

### ใน Server Components

```tsx
// app/page.tsx -- เป็น Server component จึงปลอดภัย
export default async function HomePage() {
  const dbUrl = process.env.DATABASE_URL
  const data = await fetchData(dbUrl)
  return <div>{/* render */}</div>
}
```

### ใน Client Components

```tsx
// 'use client' -- อนุญาตให้ใช้ตัวแปรแบบ public เท่านั้น
'use client'

export default function AnalyticsTracker() {
  const gaId = process.env.RUVYXA_PUBLIC_GA_ID
  return <Script src={`https://www.googletagmanager.com/gtag/js?id=${gaId}`} />
}
```

### ใน API Routes

```tsx
// app/api/payment/route.ts -- เฉพาะฝั่ง Server ปลอดภัย
export async function POST(request: Request) {
  const stripeKey = process.env.STRIPE_SECRET_KEY
  // ... ประมวลผลการชำระเงิน
  return Response.json({ success: true })
}
```

### ใน Server Actions

```ts
// app/actions/email/action.ts
'use server'

import { action } from 'ruvyxa/server'

export const sendNewsletter = action(async (formData: FormData) => {
  const apiKey = process.env.SENDGRID_API_KEY
  // ... ส่งอีเมล
})
```

---

## TypeScript Declarations — ทุกแบบ

### 1. `ruvyxa-env.d.ts` (สำหรับ `process.env`)

```ts
// ruvyxa-env.d.ts — วางที่รากโปรเจกต์
declare namespace NodeJS {
  interface ProcessEnv {
    // Public (client-safe)
    RUVYXA_PUBLIC_API_URL: string
    RUVYXA_PUBLIC_SITE_URL: string
    RUVYXA_PUBLIC_GA_ID: string

    // Private (server-only)
    DATABASE_URL: string
    AUTH_SECRET: string
    AUTH_GOOGLE_ID: string
    AUTH_GOOGLE_SECRET: string
    STRIPE_API_KEY: string
    STRIPE_WEBHOOK_SECRET: string
    SMTP_HOST: string
    SMTP_PORT: string
    SMTP_USER: string
    SMTP_PASS: string
    REDIS_URL: string

    // Environment
    NODE_ENV: 'development' | 'production' | 'test'
    RUVYXA_RUNTIME?: 'node' | 'bun'
    RUVYXA_ADAPTER?: string
  }
}
```

### 2. `env.d.ts` (สำหรับ `import.meta.env`)

```ts
// env.d.ts หรือ ruvyxa-env.d.ts (รวม)
interface ImportMetaEnv {
  // Public (client-safe)
  readonly RUVYXA_PUBLIC_API_URL: string
  readonly RUVYXA_PUBLIC_SITE_URL: string
  readonly RUVYXA_PUBLIC_GA_ID: string

  // Mode
  readonly MODE: 'development' | 'production' | 'test'
  readonly DEV: boolean
  readonly PROD: boolean

  // Base URL
  readonly BASE_URL: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
```

### 3. Auto-generated Type Declarations

Ruvyxa สามารถสร้าง `ruvyxa-env.d.ts` อัตโนมัติจากไฟล์ `.env`:

```bash
# สร้าง type declarations จาก .env
ruvyxa doctor --generate-env-types
```

Output:

```
━━━ Generate Env Types ━━━━━━━━━━━━━━━━━━━━━━
  ✓ Scanned .env, .env.production
  ✓ Generated ruvyxa-env.d.ts (14 variables)
  ✓ Type safety for all env vars
  ⚠ Private vars: 8 (no RUVYXA_PUBLIC_ prefix)
```

---

## Public Variables (`RUVYXA_PUBLIC_*`) — เจาะลึก

### ตัวแปรที่ส่งไป client ได้

| ตัวแปร                                | Example Value               | การใช้งาน              |
| ------------------------------------- | --------------------------- | ---------------------- |
| `RUVYXA_PUBLIC_API_URL`               | `https://api.example.com`   | API endpoint           |
| `RUVYXA_PUBLIC_SITE_URL`              | `https://example.com`       | Site URL               |
| `RUVYXA_PUBLIC_GA_ID`                 | `G-XXXXXXXXXX`              | Google Analytics ID    |
| `RUVYXA_PUBLIC_SENTRY_DSN`            | `https://xxx@sentry.io/xxx` | Sentry DSN (public)    |
| `RUVYXA_PUBLIC_GTM_ID`                | `GTM-XXXXXXX`               | Google Tag Manager     |
| `RUVYXA_PUBLIC_STRIPE_KEY`            | `pk_live_xxxxx`             | Stripe publishable key |
| `RUVYXA_PUBLIC_ALGOLIA_ID`            | `XXXXX`                     | Algolia app ID         |
| `RUVYXA_PUBLIC_MAPBOX_TOKEN`          | `pk.xxxxx`                  | Mapbox public token    |
| `RUVYXA_PUBLIC_POSTHOG_KEY`           | `phc_xxxxx`                 | PostHog public key     |
| `RUVYXA_PUBLIC_CLERK_PUBLISHABLE_KEY` | `pk_test_xxxxx`             | Clerk auth key         |
| `RUVYXA_PUBLIC_VERCEL_ANALYTICS_ID`   | `xxxxx`                     | Vercel Analytics       |
| `RUVYXA_PUBLIC_ENVIRONMENT`           | `production`                | Custom env flag        |

### ตัวอย่างการประกาศ TypeScript (public ทั้งหมด)

```ts
// ruvyxa-env.d.ts
declare namespace NodeJS {
  interface ProcessEnv {
    // Public — client-safe
    RUVYXA_PUBLIC_API_URL: string
    RUVYXA_PUBLIC_SITE_URL: string
    RUVYXA_PUBLIC_GA_ID: string
    RUVYXA_PUBLIC_GTM_ID: string
    RUVYXA_PUBLIC_SENTRY_DSN: string
    RUVYXA_PUBLIC_STRIPE_KEY: string
    RUVYXA_PUBLIC_ENVIRONMENT: 'development' | 'staging' | 'production'
  }
}
```

### ตัวอย่างการใช้ใน Client Component

```tsx
'use client'

export default function AnalyticsProvider({ children }: { children: React.ReactNode }) {
  const gaId = process.env.RUVYXA_PUBLIC_GA_ID
  const gtmId = process.env.RUVYXA_PUBLIC_GTM_ID

  useEffect(() => {
    if (typeof window !== 'undefined' && gaId) {
      // Load Google Analytics
      const script = document.createElement('script')
      script.src = `https://www.googletagmanager.com/gtag/js?id=${gaId}`
      script.async = true
      document.head.appendChild(script)

      window.dataLayer = window.dataLayer || []
      function gtag(...args: unknown[]) {
        window.dataLayer.push(args)
      }
      gtag('js', new Date())
      gtag('config', gaId)
    }
  }, [gaId])

  return <>{children}</>
}
```

---

## Private Variables (Server-Only) — เจาะลึก

### ตัวแปรที่ห้ามส่งไป client โดยเด็ดขาด

| Category       | Examples                                                | ผลเสียถ้ารั่ว            |
| -------------- | ------------------------------------------------------- | ------------------------ |
| Database       | `DATABASE_URL`, `MONGODB_URI`, `PGHOST`                 | สูญเสียข้อมูล            |
| Authentication | `AUTH_SECRET`, `JWT_SECRET`, `AUTH_GOOGLE_SECRET`       | ปลอมแปลง session         |
| API Keys       | `STRIPE_API_KEY`, `OPENAI_API_KEY`, `AWS_ACCESS_KEY_ID` | เสียค่าใช้จ่าย, ถูกโจมตี |
| Encryption     | `ENCRYPTION_KEY`, `SALT`                                | ข้อมูลรั่วไหล            |
| Infrastructure | `REDIS_URL`, `SQS_QUEUE_URL`, `CLOUDAMQP_URL`           | โจมตี infrastructure     |
| Email          | `SMTP_PASS`, `SENDGRID_API_KEY`                         | ส่ง email spam           |

### วิธีใช้ Private Variables อย่างปลอดภัย

#### ✅ Server Component (ปลอดภัย)

```tsx
// app/dashboard/page.tsx — Server Component
import { PrismaClient } from '@prisma/client'

const prisma = new PrismaClient({
  datasourceUrl: process.env.DATABASE_URL, // ✅ Server Component — ปลอดภัย
})

export default async function DashboardPage() {
  const users = await prisma.user.findMany()

  return (
    <div>
      <h1>แดชบอร์ด</h1>
      <p>ผู้ใช้ทั้งหมด: {users.length}</p>
    </div>
  )
}
```

#### ✅ Server Action (ปลอดภัย)

```tsx
// app/actions.ts
'use server'

export async function createUser(formData: FormData) {
  const dbUrl = process.env.DATABASE_URL // ✅ Server Action — ปลอดภัย
  const stripeKey = process.env.STRIPE_API_KEY // ✅ Server Action

  // ... database operations
  return { success: true }
}
```

#### ✅ Route Handler (ปลอดภัย)

```tsx
// app/api/webhook/stripe/route.ts
import { NextRequest, NextResponse } from 'next/server'

export async function POST(request: NextRequest) {
  const stripeSecret = process.env.STRIPE_WEBHOOK_SECRET // ✅ Route Handler

  const signature = request.headers.get('stripe-signature')
  const event = stripe.webhooks.constructEvent(await request.text(), signature!, stripeSecret)

  return NextResponse.json({ received: true })
}
```

---

## RUV1008 Error — Environment Boundary Violation

### Error ที่เกิดขึ้น

```tsx
'use client'

// ❌ RUV1008: Private environment variable in client code
const dbUrl = process.env.DATABASE_URL

export default function DangerousComponent() {
  const apiKey = process.env.AUTH_SECRET // ❌ RUV1008

  return (
    <div>
      <p>Database: {dbUrl}</p>
      <p>API Key: {apiKey}</p>
    </div>
  )
}
```

### Output Error เต็มรูปแบบ (ตอน build)

```
━━━ Build Error ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  RUV1008: Environment boundary violation
  ──────────────────────────────────────
  Private environment variable `DATABASE_URL` is exposed to client code.

  File: app/components/dangerous.tsx:3:18
  Code: const dbUrl = process.env.DATABASE_URL;

  Why this is dangerous:
    DATABASE_URL contains database credentials.
    If exposed to the browser, anyone can read your database URL.

  Fix options:
    1. Prefix with RUVYXA_PUBLIC_ → RUVYXA_PUBLIC_DATABASE_URL
       (only if the value is safe to expose)

    2. Move usage to a server-only scope:
       - Server Component (default)
       - Server Action ('use server')
       - Route Handler (app/api/)
       - Server-only file (server/ directory)

    3. Use server action to access private data:
       'use server' action that returns data to client

  Learn more: https://ruvyxa.dev/docs/guides/environment-variables

  Total violations: 2
```

### วิธีแก้ RUV1008 — ทุกกรณี

#### กรณีที่ 1: ตัวแปรนั้นปลอดภัย → เปลี่ยนชื่อ

```bash
# .env — เปลี่ยนชื่อ
# Before
DATABASE_URL=postgres://localhost/db

# After
RUVYXA_PUBLIC_DATABASE_URL=postgres://localhost/db
```

```tsx
// components/db-client.tsx — ใช้ได้ทั้ง client และ server
export default function DBStatus() {
  return <div>DB URL: {process.env.RUVYXA_PUBLIC_DATABASE_URL} ✅</div>
}
```

#### กรณีที่ 2: ตัวแปรนั้นเป็น secret → ย้าย logic ไป server

```tsx
// app/actions/db.ts
'use server'

import { PrismaClient } from '@prisma/client'

const prisma = new PrismaClient({
  datasourceUrl: process.env.DATABASE_URL, // ✅ Server-only
})

export async function getUsers() {
  return await prisma.user.findMany()
}
```

```tsx
// app/page.tsx
'use client'

import { getUsers } from './actions/db'
import { useEffect, useState } from 'react'

export default function UsersPage() {
  const [users, setUsers] = useState([])

  useEffect(() => {
    getUsers().then(setUsers) // ✅ เรียกผ่าน server action — ปลอดภัย
  }, [])

  return (
    <ul>
      {users.map((user) => (
        <li key={user.id}>{user.name}</li>
      ))}
    </ul>
  )
}
```

#### กรณีที่ 3: ใช้ Server Component ที่เรียก Client Component

```tsx
// app/profile/page.tsx — Server Component
import { PrismaClient } from '@prisma/client'

const prisma = new PrismaClient({
  datasourceUrl: process.env.DATABASE_URL, // ✅ Server Component
})

export default async function ProfilePage() {
  const user = await prisma.user.findFirst()

  return (
    <div>
      <h1>{user?.name}</h1>
      {/* ส่งค่า (ไม่ใช่ env) ไปยัง client component */}
      <ProfileEditor user={user} />
    </div>
  )
}

// ProfileEditor — Client Component
;('use client')

function ProfileEditor({ user }: { user: { id: number; name: string } }) {
  // ไม่ต้องใช้ process.env.DATABASE_URL — รับ props จาก server
  return <div>Edit: {user.name}</div>
}
```

---

## Client Scanner — กลไกตรวจสอบ RUV1008

Ruvyxa มี static analysis scanner ที่ตรวจหา `process.env.*` ใน client code:

### Scanning Algorithm

```
function scanForEnvViolations(sourceCode: string, filePath: string): Violation[] {
  const violations: Violation[] = [];

  // 1. Parse source เป็น AST
  const ast = parse(sourceCode, { jsx: true });

  // 2. Traverse AST
  walk(ast, {
    MemberExpression(node) {
      // ตรวจหา process.env.XXX
      if (
        node.object.type === 'MemberExpression' &&
        node.object.object.name === 'process' &&
        node.object.property.name === 'env'
      ) {
        const varName = node.property.name;

        if (!isClientAccessible(varName)) {
          violations.push({
            code: 'RUV1008',
            file: filePath,
            line: node.loc.start.line,
            column: node.loc.start.column,
            variable: varName,
          });
        }
      }
    },

    // ตรวจหา import.meta.env.XXX
    MemberExpression(node) {
      if (
        node.object.type === 'MemberExpression' &&
        node.object.object.type === 'MetaProperty' &&
        node.object.property.name === 'env'
      ) {
        const varName = node.property.name;

        if (!isClientAccessible(varName)) {
          violations.push({
            code: 'RUV1008',
            file: filePath,
            line: node.loc.start.line,
            variable: varName,
          });
        }
      }
    },
  });

  return violations;
}
```

### Thresholds

| Detection Method                             | Coverage                                    |
| -------------------------------------------- | ------------------------------------------- |
| Static AST scan                              | `process.env.X`, `import.meta.env.X` — 100% |
| Dynamic access `process.env[X]`              | พบบางส่วน (แนะนำให้ใช้ static string)       |
| String interpolation `env['X']`              | ครอบคลุม (ต้องเป็น string literal)          |
| Re-exported vars `const db = process.env.DB` | 100% (trace ถึงต้นทาง)                      |

---

## Allowed Client Variables — รายการทั้งหมด

| ตัวแปร                     | ชนิด                                      | Example                     | ใช้ใน client |
| -------------------------- | ----------------------------------------- | --------------------------- | ------------ |
| `NODE_ENV`                 | `'development' \| 'production' \| 'test'` | `'production'`              | ✅           |
| `RUVYXA_PUBLIC_*`          | `string`                                  | `'https://api.example.com'` | ✅           |
| `PUBLIC_*`                 | `string`                                  | `'pk_test_xxxx'`            | ✅ (compat)  |
| `NEXT_PUBLIC_*`            | `string`                                  | `'https://api.example.com'` | ✅ (compat)  |
| `VITE_*`                   | `string`                                  | `'https://api.example.com'` | ✅ (compat)  |
| `import.meta.env.MODE`     | `string`                                  | `'development'`             | ✅           |
| `import.meta.env.DEV`      | `boolean`                                 | `true`                      | ✅           |
| `import.meta.env.PROD`     | `boolean`                                 | `false`                     | ✅           |
| `import.meta.env.BASE_URL` | `string`                                  | `'/'`                       | ✅           |

---

## Runtime Environment Variables

### `MODE` และ `NODE_ENV` — Full Reference

| ตัวแปร                 | `ruvyxa dev`    | `ruvyxa build` | `ruvyxa start` | `ruvyxa test:parity` |
| ---------------------- | --------------- | -------------- | -------------- | -------------------- |
| `NODE_ENV`             | `'development'` | `'production'` | `'production'` | `'test'`             |
| `import.meta.env.MODE` | `'development'` | `'production'` | `'production'` | `'test'`             |
| `import.meta.env.DEV`  | `true`          | `false`        | `false`        | `false`              |
| `import.meta.env.PROD` | `false`         | `true`         | `true`         | `false`              |

### การใช้ใน Code

```tsx
// app/layout.tsx
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="th">
      <head>
        {/* โหลด CSS เฉพาะ dev */}
        {process.env.NODE_ENV === 'development' && <link rel="stylesheet" href="/dev-styles.css" />}
      </head>
      <body>
        {children}

        {/* Dev Tools — แสดงเฉพาะ dev */}
        {import.meta.env.DEV && (
          <div id="dev-tools">
            <RuvyxaDebugPanel />
          </div>
        )}
      </body>
    </html>
  )
}
```

### Module-Level Conditional (Tree-shakeable)

```ts
// utils/logger.ts
const isDev = import.meta.env.DEV

export function logger(...args: unknown[]) {
  if (isDev) {
    console.log('[Ruvyxa Dev]', ...args)
    // ใน production, logger() เป็น no-op (tree-shaken)
  }
}
```

---

## Full App Example: Environment Variables

### `.env` — ไฟล์ default

```bash
# ──────────── Public (client-safe) ────────────
RUVYXA_PUBLIC_API_URL=http://localhost:3000/api
RUVYXA_PUBLIC_SITE_URL=http://localhost:3000
RUVYXA_PUBLIC_GA_ID=G-XXXXXXXXXX
RUVYXA_PUBLIC_GTM_ID=GTM-XXXXXXX
RUVYXA_PUBLIC_SENTRY_DSN=https://example@sentry.io/123
RUVYXA_PUBLIC_STRIPE_KEY=pk_test_xxxxxxxxxxxxx
RUVYXA_PUBLIC_ENVIRONMENT=development

# ──────────── Private (server-only) ────────────
DATABASE_URL=postgres://user:pass@localhost:5432/myapp_dev
DATABASE_POOL_MIN=2
DATABASE_POOL_MAX=10

AUTH_SECRET=dev-secret-do-not-use-in-prod
AUTH_GOOGLE_ID=123456789-xxxxx.apps.googleusercontent.com
AUTH_GOOGLE_SECRET=GOCSPX-xxxxxxxxxxxxx

STRIPE_API_KEY=sk_test_xxxxxxxxxxxxx
STRIPE_WEBHOOK_SECRET=whsec_xxxxxxxxxxxxx

SMTP_HOST=smtp.sendgrid.net
SMTP_PORT=587
SMTP_USER=apikey
SMTP_PASS=SG.xxxxxxxxxxxxx

REDIS_URL=redis://localhost:6379
AWS_ACCESS_KEY_ID=AKIAxxxxxxxxxx
AWS_SECRET_ACCESS_KEY=xxxxxxxxxxxxx
```

### `.env.production` — ค่า production

```bash
RUVYXA_PUBLIC_API_URL=https://api.myapp.com
RUVYXA_PUBLIC_SITE_URL=https://myapp.com
RUVYXA_PUBLIC_GA_ID=G-YYYYYYYYYY
RUVYXA_PUBLIC_ENVIRONMENT=production

DATABASE_URL=postgres://user:@prod-db.amazonaws.com:5432/myapp_prod
```

### `.env.example` — Template

```bash
# ──────────── Ruvyxa Environment Variables ────────────
# คัดลอกไฟล์นี้ไปเป็น .env แล้วเติมค่าของคุณ
#
# Public Variables (client-safe — ใช้ RUVYXA_PUBLIC_ prefix)
# Private Variables (server-only — ไม่มี prefix พิเศษ)
# ───────────────────────────────────────────────────────

# === Public ===
RUVYXA_PUBLIC_API_URL=http://localhost:3000/api
RUVYXA_PUBLIC_SITE_URL=http://localhost:3000
RUVYXA_PUBLIC_GA_ID=
RUVYXA_PUBLIC_STRIPE_KEY=

# === Private (Server-Only) ===
# DATABASE_URL=postgres://user:pass@localhost:5432/myapp
# AUTH_SECRET=generate-a-random-secret
# STRIPE_API_KEY=sk_live_xxxxxxxxx

# === Platform ===
# NODE_ENV=development
```

### `ruvyxa-env.d.ts` — TypeScript

```ts
// ruvyxa-env.d.ts
declare namespace NodeJS {
  interface ProcessEnv {
    // Public
    RUVYXA_PUBLIC_API_URL: string
    RUVYXA_PUBLIC_SITE_URL: string
    RUVYXA_PUBLIC_GA_ID: string
    RUVYXA_PUBLIC_GTM_ID: string
    RUVYXA_PUBLIC_SENTRY_DSN: string
    RUVYXA_PUBLIC_STRIPE_KEY: string
    RUVYXA_PUBLIC_ENVIRONMENT: 'development' | 'staging' | 'production'

    // Private
    DATABASE_URL: string
    DATABASE_POOL_MIN: string
    DATABASE_POOL_MAX: string
    AUTH_SECRET: string
    AUTH_GOOGLE_ID: string
    AUTH_GOOGLE_SECRET: string
    STRIPE_API_KEY: string
    STRIPE_WEBHOOK_SECRET: string
    SMTP_HOST: string
    SMTP_PORT: string
    SMTP_USER: string
    SMTP_PASS: string
    REDIS_URL: string
    AWS_ACCESS_KEY_ID: string
    AWS_SECRET_ACCESS_KEY: string

    // Built-in
    NODE_ENV: 'development' | 'production' | 'test'
    RUVYXA_RUNTIME?: 'node' | 'bun'
    RUVYXA_ADAPTER?: string
  }
}
```

### `app/page.tsx` — ใช้ public vars

```tsx
import { Link } from '@ruvyxa/react'

export default function HomePage() {
  return (
    <div>
      <h1>ยินดีต้อนรับ</h1>
      <p>API: {process.env.RUVYXA_PUBLIC_API_URL}</p>
      <p>Environment: {process.env.RUVYXA_PUBLIC_ENVIRONMENT}</p>
      <Link href="/users">ดูผู้ใช้</Link>
    </div>
  )
}
```

### `app/actions.ts` — ใช้ private vars

```ts
'use server'

import { PrismaClient } from '@prisma/client'
import Stripe from 'stripe'

const prisma = new PrismaClient({
  datasourceUrl: process.env.DATABASE_URL,
})

const stripe = new Stripe(process.env.STRIPE_API_KEY, {
  apiVersion: '2025-02-24',
})

export async function createCheckoutSession(priceId: string) {
  const session = await stripe.checkout.sessions.create({
    mode: 'payment',
    line_items: [{ price: priceId, quantity: 1 }],
    success_url: `${process.env.RUVYXA_PUBLIC_SITE_URL}/success`,
    cancel_url: `${process.env.RUVYXA_PUBLIC_SITE_URL}/cancel`,
  })

  return { url: session.url }
}

export async function getUsers() {
  return await prisma.user.findMany({
    select: { id: true, name: true, email: true },
  })
}
```

### `server/db.ts` — ไฟล์ server-only

```ts
import { PrismaClient } from '@prisma/client'

const globalForPrisma = globalThis as unknown as {
  prisma: PrismaClient | undefined
}

export const prisma =
  globalForPrisma.prisma ??
  new PrismaClient({
    datasourceUrl: process.env.DATABASE_URL,
    log: process.env.NODE_ENV === 'development' ? ['query'] : [],
  })

if (process.env.NODE_ENV !== 'production') {
  globalForPrisma.prisma = prisma
}
```

---

## Build-Time Variables (ตัวแปรช่วงบิลด์)

ตัวแปร Environment เหล่านี้จะถูกแทนที่ด้วยค่าที่แท้จริงในช่วงที่มีการบิลด์ (Inline):

```ts
// ส่วนนี้จะถูกแทนที่ตอนทำ Build-time ด้วยค่าที่แท้จริง
const apiUrl = process.env.RUVYXA_PUBLIC_API_URL
// หลังจากรัน Build จะกลายเป็น: const apiUrl = "https://api.example.com";
```

### ผลที่ตามมา (Implications)

1. **คุณต้องทำ Build ใหม่** ทุกครั้งหากต้องการให้เห็นค่า Environment (ที่เป็นกลุ่ม Public)
   ที่ถูกเปลี่ยนแปลงไป
2. **การคัดแยกโค้ดที่ไม่ได้ใช้ (Dead code elimination)** จะทำงานได้อย่างเต็มที่ --
   บล็อกเงื่อนไขที่อ้างอิงจากตัวแปรเหล่านี้จะถูกคัดทิ้งไปหากไม่เข้าเงื่อนไข:

```ts
if (process.env.RUVYXA_PUBLIC_FEATURE_FLAG === 'enabled') {
  // บล็อกคำสั่งนี้อาจถูกคัดทิ้งออกไปเลยในระบบ Production หากฟีเจอร์นี้ถูกปิดเอาไว้
  registerFeature()
}
```

สำหรับตัวแปรที่ใช้เฉพาะบน Server (Server-only vars) จะไม่มีการแทนที่ค่าเหล่านี้ในตอนทำ Build --
ค่าเหล่านี้จะถูกอ่านมาจากตัวแปรบนระบบตามจริงในตอนรันไทม์

### Stability (ความเสถียร)

ระบบประมวลผลช่วง Build จะใช้ฟังก์ชัน `stable_process_env()` เพื่อรวบรวม snapshot ของตัวแปร
Environment ทุกตัวให้สามารถคาดเดาและใช้แฮช (Hash) เข้ากระบวนการแคช (Cache) ได้:

```rust
fn prerender_context_hash(
    root: &Path,
    styles: &str,
    client_assets: &BTreeMap<...>,
    build: &BuildConfigOptions,
    project_env: &BTreeMap<String, String>,
) -> String {
    // นำตัวแปร RUVYXA_PUBLIC_* เข้าร่วมการแฮชด้วย เพื่อให้การแคช (Cache) ถูกล้างออกไปใหม่หากมีการแก้ไข
}
```

---

## Validation & Defaults (การตรวจสอบค่าและค่าเริ่มต้น)

### การใช้งาน requireEnv Plugin

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'

export default config({
  plugins: [
    {
      name: 'requireEnv',
      options: {
        variables: ['DATABASE_URL', 'STRIPE_SECRET_KEY', 'SENDGRID_API_KEY'],
        strict: true, // ทำให้เกิด error ถ้าไม่ได้ระบุตัวแปรนี้
      },
    },
  ],
})
```

### Default Values (การกำหนดค่าตั้งต้น)

```ts
const port = process.env.PORT ?? '3000'
const logLevel = process.env.LOG_LEVEL || 'info'
```

### Config Renderer Environment

เมื่อ Ruvyxa ประมวลผลและเช็คค่าคอนฟิก `ruvyxa.config.ts` จะมีการกำหนด:

```bash
RUVYXA_RUNTIME=node     # หรืออาจเป็น bun เป็นต้น
```

นี่คือค่าที่ถูกแทรกเข้าไปผ่านฟังก์ชัน `run_config_renderer()`:

```rust
ProcessCommand::new(runtime.executable())
    .arg(renderer)
    .arg(root)
    .env("RUVYXA_RUNTIME", runtime.command())
    .output()?;
```

---

## Best Practices — เต็มรูปแบบ

### 1. `.env.example` — Template ที่ควร commit

```bash
# .env.example — commit ไว้ใน repository
# ใช้เป็น template สำหรับนักพัฒนาทุกคนในทีม

# ⚠️ คำแนะนำ:
# - ค่า secrets ให้เว้นว่างหรือใช้ placeholder
# - ใส่ comment อธิบายแต่ละตัวแปร
# - แยก public/private ให้ชัดเจน

# ── Public ──
RUVYXA_PUBLIC_API_URL=http://localhost:3000/api
RUVYXA_PUBLIC_SITE_URL=http://localhost:3000
RUVYXA_PUBLIC_GA_ID=G-XXXXXXXXXX

# ── Private ──
DATABASE_URL=postgres://user:pass@localhost:5432/myapp
AUTH_SECRET=change-me-to-a-random-secret
STRIPE_API_KEY=sk_live_xxxxxxxxx
```

### 2. Gitignore — ห้าม commit secrets

```gitignore
# .gitignore
.env.local
.env.*.local
*.local

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db
```

### 3. Validation ด้วย `requireEnv`

ใช้ built-in plugin ตรวจสอบ env vars ตอน build:

```ts
// ruvyxa.config.ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  plugins: [
    {
      name: 'requireEnv',
      options: {
        vars: ['DATABASE_URL', 'AUTH_SECRET', 'STRIPE_API_KEY', 'RUVYXA_PUBLIC_API_URL'],
        // mode: 'strict' | 'warn'
        mode: 'strict', // ถ้าไม่มีค่า → build ล้มเหลว
      },
    },
  ],
})
```

**Error output เมื่อ env var หาย:**

```
Required environment variable `DATABASE_URL` is not set
  Defined in requireEnv plugin configuration
  Fix: Add DATABASE_URL to your .env file or set it in the environment
```

### 4. ตั้งชื่อให้มีความหมาย

```bash
# ❌ ไม่ดี — ไม่รู้ว่าใช้ทำอะไร
SECRET=xxx
KEY=yyy
TOKEN=zzz
URL=www.example.com

# ✅ ดี — บอกชัดเจนว่าคืออะไร
AUTH_JWT_SECRET=xxx
STRIPE_API_KEY=sk_live_yyy
GITHUB_ACCESS_TOKEN=ghp_zzz
RUVYXA_PUBLIC_API_URL=https://api.example.com
```

### 5. แยกตาม Environment

```
.env              # ค่า default สำหรับทุก environment (commit ได้)
.env.development  # dev-specific (commit ได้ — ไม่มี secret จริง)
.env.production   # production-specific (commit ได้ — ถ้าไม่มี secret)
.env.local        # local override (ห้าม commit)
.env.staging      # staging-specific
.env.test         # test-specific
```

### 6. ตรวจสอบ Environment Variables ตอน CI/CD

```yaml
# .github/workflows/check-env.yml
name: Check Environment Variables

on:
  pull_request:
    paths:
      - '.env.example'
      - 'ruvyxa.config.ts'

jobs:
  validate-env:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4

      - name: Install dependencies
        run: npm ci

      - name: Validate env vars
        run: npx ruvyxa check

      - name: Check missing env vars
        run: |
          # ตรวจสอบว่าตัวแปรที่จำเป็นมีครบ
          required_vars="DATABASE_URL AUTH_SECRET RUVYXA_PUBLIC_API_URL"
          for var in $required_vars; do
            if [ -z "${!var}" ]; then
              echo "::warning::Missing required env var: $var"
            fi
          done
```

### 7. เข้ารหัส Secrets สำหรับ Production

```bash
# ใช้ 1Password CLI, Vault, หรือ platform-specific tools

# Vercel
vercel env add DATABASE_URL --secret
vercel env add AUTH_SECRET --secret

# GitHub Actions — ใช้ encrypted secrets
gh secret set DATABASE_URL
gh secret set AUTH_SECRET

# AWS Parameter Store / Secrets Manager
aws ssm put-parameter --name DATABASE_URL --value "postgres://..." --type SecureString
```

### 8. การ Rotate Secrets

```bash
# Script: rotate-secrets.sh
#!/bin/bash

# สร้าง secret ใหม่
NEW_AUTH_SECRET=$(openssl rand -base64 32)
NEW_STRIPE_KEY=$(stripe api-keys create)

# อัปเดตใน platform
echo "$NEW_AUTH_SECRET" | vercel env add AUTH_SECRET --secret
echo "$NEW_STRIPE_KEY" | vercel env add STRIPE_API_KEY --secret

# แจ้งทีม
echo "Secrets rotated. Please update your .env.local files."
```

### 9. Logging — ห้าม log secrets

```ts
// ❌ ไม่ดี — log env vars ทั้งหมด
console.log('Env:', process.env)

// ✅ ดี — log เฉพาะชื่อ ไม่ใช่ค่า
console.log(
  'Config loaded:',
  Object.keys(process.env).filter((k) => k.startsWith('RUVYXA')),
)

// ✅ ดี — log public vars
console.log('Public vars:', {
  API_URL: process.env.RUVYXA_PUBLIC_API_URL,
  SITE_URL: process.env.RUVYXA_PUBLIC_SITE_URL,
})
```

### 10. Dynamic Access — หลีกเลี่ยง

```ts
// ❌ ไม่ดี — dynamic access ทำให้ Ruvyxa ตรวจไม่ได้
const key = 'DATABASE_URL'
const value = process.env[key] // Scanner ไม่เจอ → อาจรั่วไหล

// ✅ ดี — static access
const value = process.env.DATABASE_URL

// ✅ ดี — ถ้าต้องใช้ dynamic จริง ๆ — ใช้ใน server เท่านั้น
// (ไฟล์ server-only ที่ไม่มี client import)
```

---

## Platform Detection — Environment Variables

| Platform         | Detection Variable                          | Adapter Name |
| ---------------- | ------------------------------------------- | ------------ |
| Vercel           | `VERCEL`, `VERCEL_ENV`                      | `vercel`     |
| Netlify          | `NETLIFY`, `CONTEXT`                        | `netlify`    |
| Cloudflare Pages | `CF_PAGES`, `CF_PAGES_URL`                  | `cloudflare` |
| Railway          | `RAILWAY_PROJECT_ID`, `RAILWAY_ENVIRONMENT` | `railway`    |
| Render           | `RENDER`, `RENDER_EXTERNAL_URL`             | `render`     |
| Firebase         | `FIREBASE_CONFIG`                           | `firebase`   |
| AWS Amplify      | `AWS_APP_ID`                                | `aws`        |
| Fly.io           | `FLY_APP_NAME`                              | `fly`        |
| Koyeb            | `KOYEB_APP_NAME`                            | `koyeb`      |

### ตัวอย่างการตรวจสอบ Platform

```ts
// utils/platform.ts
export function getPlatform(): string {
  if (process.env.VERCEL) return 'vercel'
  if (process.env.NETLIFY) return 'netlify'
  if (process.env.CF_PAGES) return 'cloudflare'
  if (process.env.RAILWAY_PROJECT_ID) return 'railway'
  if (process.env.RENDER) return 'render'
  if (process.env.FIREBASE_CONFIG) return 'firebase'
  if (process.env.FLY_APP_NAME) return 'fly'
  return 'node' // local หรือ bare metal
}

export function isProduction(): boolean {
  return process.env.NODE_ENV === 'production'
}

export function isServerless(): boolean {
  const platform = getPlatform()
  return ['vercel', 'netlify', 'cloudflare'].includes(platform)
}
```

---

## Production Deployment — การตั้งค่า Environment Variables

### Vercel

```bash
# CLI
vercel env add RUVYXA_PUBLIC_API_URL
vercel env add DATABASE_URL --secret
vercel env add AUTH_SECRET --secret

# UI: Project Settings → Environment Variables
```

```yaml
# vercel.json
{ 'env': { 'RUVYXA_PUBLIC_API_URL': 'https://api.example.com' } }
```

### Netlify

```bash
# CLI
netlify env:set RUVYXA_PUBLIC_API_URL https://api.example.com
netlify env:set DATABASE_URL postgres://... --secret

# UI: Site settings → Environment variables
```

### Cloudflare Pages

```bash
# wrangler CLI
wrangler pages secret put DATABASE_URL
wrangler pages secret put AUTH_SECRET

# Dashboard: Pages → {project} → Settings → Environment variables
```

### Docker

```dockerfile
# Dockerfile
FROM node:22-alpine
WORKDIR /app
COPY . .
RUN npm ci && npm run build

# ค่า production env
ENV NODE_ENV=production
ENV RUVYXA_PUBLIC_API_URL=https://api.example.com

EXPOSE 3000
CMD ["npm", "run", "start"]
```

```yaml
# docker-compose.yml
services:
  app:
    build: .
    ports:
      - '3000:3000'
    environment:
      - RUVYXA_PUBLIC_API_URL=https://api.example.com
      - DATABASE_URL=postgres://user:pass@db:5432/myapp
      - AUTH_SECRET=${AUTH_SECRET} # จาก .env
    env_file:
      - .env.production
```

---

## `ruvyxa doctor` — ตรวจสอบ Environment Variables

```bash
npm run doctor
```

Output:

```
━━━ Ruvyxa Doctor ━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Environment Variables
  ─────────────────────
  ✓ Env files loaded: .env, .env.local, .env.development
  ✓ Public vars:      5 found (RUVYXA_PUBLIC_*)
  ✓ Private vars:     8 found (server-only)
  ✓ No boundary violations
  ✓ NODE_ENV:         development

  ⚠ Missing .env.example
    Add .env.example to help teammates set up their environment

  ⚠ 2 private vars without .env.example entry
    - AUTH_GOOGLE_SECRET
    - AWS_SECRET_ACCESS_KEY
```

### `--json` flag

```json
{
  "env": {
    "filesLoaded": [".env", ".env.local", ".env.development"],
    "publicCount": 5,
    "privateCount": 8,
    "violations": 0,
    "missingExample": true,
    "missingFromExample": ["AUTH_GOOGLE_SECRET", "AWS_SECRET_ACCESS_KEY"]
  }
}
```

---

## CI/CD Integration

### GitHub Actions — Full Pipeline

```yaml
name: CI/CD Pipeline
on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest

    env:
      RUVYXA_PUBLIC_API_URL: https://api.staging.example.com
      RUVYXA_PUBLIC_SITE_URL: https://staging.example.com
      DATABASE_URL: postgres://test:test@localhost:5432/testdb
      AUTH_SECRET: test-secret-for-ci
      NODE_ENV: test

    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
          POSTGRES_DB: testdb
        options: >-
          --health-cmd pg_isready --health-interval 10s --health-timeout 5s --health-retries 5

    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22

      - name: Install dependencies
        run: npm ci

      - name: Check env vars
        run: npx ruvyxa check

      - name: Run tests
        run: npm test

      - name: Check for env violations
        run: npx ruvyxa analyze --format json | jq '.diagnostics | map(select(.code == "RUV1008"))'

      - name: Build
        run: npm run build
        env:
          NODE_ENV: production

  deploy:
    needs: test
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Deploy to Vercel
        run: npx vercel --prod
        env:
          VERCEL_TOKEN: ${{ secrets.VERCEL_TOKEN }}
          # Environment variables are managed in Vercel dashboard
```

### GitLab CI

```yaml
# .gitlab-ci.yml
variables:
  RUVYXA_PUBLIC_API_URL: https://api.staging.example.com
  NODE_ENV: test

stages:
  - validate
  - build
  - deploy

validate:
  stage: validate
  script:
    - npm ci
    - npx ruvyxa check
    - npx ruvyxa analyze --format json

build:
  stage: build
  variables:
    NODE_ENV: production
  script:
    - npm ci
    - npm run build
  artifacts:
    paths:
      - .ruvyxa/

deploy:
  stage: deploy
  script:
    - npx ruvyxa start
  environment: production
```

---

## Troubleshooting — ทุก Error Code และปัญหาที่พบบ่อย

### Error Codes

| Code              | ปัญหา                            | สาเหตุ                                       | วิธีแก้                          |
| ----------------- | -------------------------------- | -------------------------------------------- | -------------------------------- |
| RUV1008           | Private env in client            | ใช้ `process.env.DB_URL` ใน client component | เปลี่ยน prefix หรือย้ายไป server |
| RUV1009           | Public env not allowed on server | ไม่มี error — public ใช้ server ได้เสมอ      | -                                |
| ไม่มี code ตายตัว | Required env var missing         | ขาด env var ที่ plugin `requireEnv` กำหนด    | เพิ่ม env var                    |
| RUV1201           | Config validation error          | .env syntax ผิด (format ไม่ถูก)              | ตรวจสอบรูปแบบ .env               |

### ปัญหาทั่วไป

| ปัญหา                      | สาเหตุ                              | วิธีแก้                              |
| -------------------------- | ----------------------------------- | ------------------------------------ |
| env var เป็น `undefined`   | ไฟล์ `.env` ไม่ถูกโหลด              | ตรวจชื่อไฟล์ `ls .env*`              |
| env var ไม่เปลี่ยนหลังแก้  | ต้อง restart dev server             | `Ctrl+C` → `npm run dev`             |
| RUV1008 error              | Private var ถึง client              | ย้ายไป server component/action       |
| `$` ใน password ถูกตีความ  | Shell interpolation                 | ใช้ single quotes หรือ escape `\$`   |
| มีช่องว่างในค่า            | `.env` parse ผิด                    | `KEY=value` (ไม่มี space รอบ `=`)    |
| `ruvyxa-env.d.ts` ไม่ทำงาน | TypeScript ไม่รู้จักไฟล์            | ตรวจ `tsconfig.json` include         |
| secrets รั่วใน commit      | Commit `.env` หรือ `.env.local`     | เพิ่มใน `.gitignore`, rotate secrets |
| production env var ผิด     | `.env.production` ไม่ถูกโหลด        | override ด้วย platform dashboard     |
| export/import หาย          | env var ไม่ propagate ไปยัง process | ตรวจ loading order                   |
| การใช้ `dotenv` ซ้ำซ้อน    | Ruvyxa จัดการ .env ให้แล้ว          | ไม่ต้องใช้ `dotenv/config` เอง       |
| `` ` `` backtick ในค่า     | Shell expansion                     | ใช้ single quotes หรือ escape        |
| `#` ในค่า                  | Comment ใน `.env`                   | ใช้ single quotes                    |
| `\n` newline ในค่า         | multiline value                     | ใช้ double quotes                    |

### วิธี Debug

```bash
# ดูว่า env vars ใดถูกโหลดบ้าง
RUVYXA_DEBUG=env ruvyxa dev

# ดูว่า vars ใดไป client
RUVYXA_DEBUG=client-env ruvyxa build

# ดู scanner detection
RUVYXA_DEBUG=scanner ruvyxa analyze

# ดูทั้งหมด
RUVYXA_DEBUG=* ruvyxa dev
```

---

## ลองทำดู

1. **สร้างไฟล์ .env**
   - `.env` พร้อม `RUVYXA_PUBLIC_API_URL` และ `DATABASE_URL`
   - `.env.local` override ค่า API URL
   - `ruvyxa doctor` → ดูว่าโหลดถูกต้อง

2. **TypeScript Declarations**
   - สร้าง `ruvyxa-env.d.ts` พร้อม ProcessEnv interface
   - ประกาศตัวแปรทุกตัวที่ใช้
   - ตรวจสอบ auto-complete ใน VS Code

3. **Public vs Private**
   - ใช้ `RUVYXA_PUBLIC_*` ใน client component
   - ใช้ private var ใน server action
   - รัน `ruvyxa build` → ดู RUV1008 ถ้าผิดพลาด

4. **CI/CD**
   - เพิ่ม `.env.example` ใน repo
   - ตั้งค่า GitHub Actions secrets
   - เพิ่ม `requireEnv` plugin ใน ruvyxa.config.ts

5. **Platform Detection**
   - ตรวจสอบ `process.env.VERCEL` ฯลฯ
   - ใช้ conditional logic ตาม platform

---

## สรุป

- `RUVYXA_PUBLIC_*` = client-safe, ตัวอื่น = server-only
- `NODE_ENV` เป็น special exception
- ไฟล์ `.env.*` เรียง priority: shell > .env.{env}.local > .env.local > .env.{env} > .env
- `process.env` และ `import.meta.env` ใช้ได้ทั้งสองแบบ
- RUV1008 เตือนเมื่อ private var ถึง client
- TypeScript declarations ผ่าน `ruvyxa-env.d.ts` หรือ `interface ImportMetaEnv`
- Client scanner ตรวจหา `process.env.X` ด้วย static AST analysis
- `requireEnv` plugin ตรวจสอบตอน build
- Platform auto-detection (Vercel, Netlify, Cloudflare ฯลฯ)
- อย่า commit secrets — ใช้ `.env.example` แทน

---

## Contract การโหลด Environment ปัจจุบัน

## ขั้นตอนถัดไป

- [03-server-client-components.md](./03-server-client-components.md) -- ทำความเข้าใจ Server/client
  boundary
- [11-configuration.md](./11-configuration.md) -- คู่มือใช้งานคอนฟิก ruvyxa.config.ts แบบเต็ม
- [14-plugins.md](./14-plugins.md) -- การจัดการปลั๊กอิน requireEnv และการประเมินค่า env
- [16-error-handling.md](./16-error-handling.md) -- รายละเอียดของ RUV1008 และข้อผิดพลาดที่เกี่ยวข้อง
