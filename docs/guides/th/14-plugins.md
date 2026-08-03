# Plugins

หน้านี้แยก current implementation contract ของ plugin ออกจาก historical examples ที่เก็บไว้เพื่อ
รักษาความยาวและบริบทเดิม การอ้างว่า production support ได้ต้องตรวจจาก source และ type จริงของ plugin
bridge

## Production contract

### Built-in exports ปัจจุบัน

first-party export อยู่ที่ ruvyxa/plugins และ source ปัจจุบันมี builder 16 ตัว:

```ts
redirects
headers
observability
securityHeaders
cacheRules
pwa
sitemap
robots
feed
searchIndex
contentEngine
openApi
alias
bundleBudget
requireEnv
fonts
```

Built-in plugin ไม่ได้แยก package version ของตัวเอง manifest ของ first-party packages ใน repository
ปัจจุบันใช้ release version 1.0.26 ส่วน realtime@1 เป็น native capability/protocol identifier ไม่ใช่
version ของ plugin แยกต่างหาก

### การลงทะเบียนและ config

```ts
import { config } from 'ruvyxa/config'
import { redirects, headers, observability, securityHeaders, cacheRules } from 'ruvyxa/plugins'

export default config({
  plugins: [
    redirects([{ source: '/old-page', destination: '/new-page', permanent: true }]),
    headers([{ source: '/api/*', headers: { 'cache-control': 'no-store' } }]),
    observability(),
    securityHeaders(),
    cacheRules([{ source: '/assets/*', browser: 'public, max-age=3600' }]),
  ],
})
```

จุดเข้าของ configuration คือ config() ไม่ใช่ defineConfig() route pattern ใช้ exact match, *, หรือ
trailing prefix wildcard ตาม factory นั้น ๆ implementation จะ validate rule shape, header, path และ
option ของ plugin ในช่วง register/build ตามที่ source กำหนด

### Lifecycle hooks

plugin bridge มี build hooks (onStart, onResolve, onLoad, onTransform, onComplete), HTTP hooks
(onRequest, onResponse, route), dev file-change hooks, diagnostics report และ native capability
claim payload ที่แน่นอนอยู่ใน packages/@ruvyxa/core/src/plugin.ts และ types.ts ไม่ควรเดา shape
จากตัวอย่างของ hook อื่น

Build hook ทำงานผ่าน TypeScript plugin worker bridge ส่วน HTTP hook ถูกเรียกโดย middleware bridge
รอบ request/response hook มี timeout default 30 วินาที และ config ได้สูงสุด 300 วินาที timeout เป็น
RUV1700 และ protocol/response ที่ผิดรูปแบบเป็น RUV1701

### พฤติกรรมที่ source รองรับ

- redirects: ส่ง response 307 หรือ 308 และ reject destination ที่เสี่ยง open redirect
- headers: ตั้ง response headers ตาม route ที่ match
- observability: เพิ่ม request ID, trace context, Server-Timing และ structured timing log ได้
  duration ที่วัดได้เป็นข้อมูลของ workload ไม่ใช่ benchmark กลาง
- securityHeaders: ใส่ security policy ตาม option; CSP ต้องออกแบบให้เหมาะกับ application
- cacheRules: ตั้ง browser/CDN cache policy และ Vary โดยไม่อ้าง distributed cache coherence
- pwa, sitemap, robots, feed, searchIndex และ contentEngine: สร้าง build artifacts จาก input ที่
  validate แล้ว
- openApi: สร้าง OpenAPI document จาก operations ที่ application ระบุ
- alias: ลงทะเบียน module-resolution aliases
- bundleBudget: ตรวจ bundle-size limit ที่ตั้งไว้ source ไม่ได้ให้ performance target กลางหรือ RUV
  code เฉพาะสำหรับ budget failure ทุกแบบ
- requireEnv: ตรวจ environment variables ที่จำเป็นตอน build
- fonts: fetch และ self-host Google Fonts stylesheet เมื่อ build environment อนุญาต ต้องตรวจ
  generated assets และ provider availability ใน CI

### การตรวจ production

```bash
npm run check
npm run analyze
npm run build
```

Production review ควรบันทึก plugin ที่เลือก, config, generated artifacts, environment variables,
target adapter และ workload measurement เอกสารนี้ไม่อ้าง latency, throughput, bundle-size, ROI,
พันธมิตร หรือการ promotion/deployment แบบอัตโนมัติ

## Source of truth

- packages/ruvyxa/src/plugins.ts
- packages/@ruvyxa/core/src/plugin.ts
- crates/ruvyxa_middleware/src
- packages/@ruvyxa/core/package.json

---

## Retained detailed draft

เนื้อหาฉบับยาวเดิมเก็บไว้เพื่อรักษาบริบทและความยาวเท่านั้น เป็น non-normative historical draft
ต้องตรวจ API, option, payload, metric และข้อจำกัด provider กับ source ปัจจุบันก่อนนำไปใช้จริง

### Thai plugin draft — historical draft (non-normative)

> **คำเตือน archive:** เนื้อหาด้านล่างเก็บไว้เพื่อประวัติเท่านั้น ไม่ใช่ plugin contract ปัจจุบัน
> ตัวอย่างอาจเก่าหรือไม่รองรับ และห้ามนำไปใช้เป็น code จริง production contract
> ด้านบนเป็นแหล่งอ้างอิงหลัก

## สิ่งที่คุณจะได้เรียนรู้ (What You Will Learn)

- Plugin architecture and socket registry
- All 16 built-in plugins with complete TypeScript types, options, and examples
- `definePlugin()` API: concise declarations and `register()` escape hatch
- Plugin hooks: `build.onResolve`, `build.onLoad`, `build.onTransform`, `build.onStart`,
  `build.onComplete`, `http.onRequest`, `http.onResponse`, `http.route`, `dev.onFileChange`,
  `diagnostics.report`, `native.claim`
- Plugin execution timing and ordering rules
- Response middleware limits (32 MiB default, 256 MiB max)
- Publishing a plugin to npm
- Custom plugin: SEO validator, virtual modules, analytics middleware
- Troubleshooting every plugin failure

---

# ระบบ Plugin ใน Ruvyxa

Ruvyxa มีระบบ plugin ที่ยืดหยุ่น — ตั้งแต่ built-in plugins 16 ตัวที่พร้อมใช้ ไปจนถึงการสร้าง custom
plugin ของคุณเอง ระบบ plugin รองรับทั้งการปรับพฤติกรรม ระหว่าง build (transform, resolve, bundle)
และระหว่าง runtime (middleware, HTTP hooks)

---

## สถาปัตยกรรม Plugin — แบบละเอียด

```
┌────────────────────────────────────────────────────────────┐
│                  Ruvyxa Plugin System                        │
│                                                             │
│  ┌──────────────────┐    ┌────────────────────────────┐    │
│  │  Built-in (Rust)  │    │  TypeScript Plugin Host    │    │
│  │                   │    │  (Node.js / Bun Worker)    │    │
│  │  • redirects      │    │                             │    │
│  │  • headers        │    │  Build Hooks:               │    │
│  │  • securityHeaders│    │  ┌─────────────────────┐  │    │
│  │  • cacheRules     │    │  │ onStart              │  │    │
│  │  • observability  │    │  │ onResolve            │  │    │
│  │  • sitemap        │    │  │ onTransform          │  │    │
│  │  • robots         │    │  │ onComplete           │  │    │
│  │  • feed           │    │  └─────────────────────┘  │    │
│  │  • searchIndex    │    │                             │    │
│  │  • contentEngine  │    │  HTTP Hooks:                │    │
│  │  • openApi        │    │  ┌─────────────────────┐  │    │
│  │  • alias          │    │  │ onRequest            │  │    │
│  │  • bundleBudget   │    │  │ onResponse           │  │    │
│  │  • requireEnv     │    │  └─────────────────────┘  │    │
│  │  • pwa            │    │                             │    │
│  │  • fonts          │    │  Legacy Hooks:              │    │
│  └──────────────────┘    │  ┌─────────────────────┐  │    │
│                           │  │ resolveId           │  │    │
│  Socket Registry          │  │ transform           │  │    │
│  ┌────────────────┐      │  │ buildStart          │  │    │
│  │ bi-directional  │◄────►│  │ buildEnd            │  │    │
│  │ IPC (JSON)      │      │  │ serverStart         │  │    │
│  └────────────────┘      │  │ serverEnd           │  │    │
│                           │  │ middleware          │  │    │
│                           │  └─────────────────────┘  │    │
│                           └────────────────────────────┘    │
└────────────────────────────────────────────────────────────┘
```

### Built-in vs TypeScript Plugins

| ลักษณะ         | Built-in (Rust)                                           | TypeScript (Worker)                               |
| -------------- | --------------------------------------------------------- | ------------------------------------------------- |
| ภาษา           | Rust                                                      | TypeScript/JavaScript                             |
| Performance    | ใช้ native path ใน process เดียวกัน; ไม่มี benchmark กลาง | ทำงานผ่าน worker bridge; latency ขึ้นกับ workload |
| ใช้เมื่อ       | ทั่วไป — redirects, headers                               | custom logic, complex hooks                       |
| ลงทะเบียน      | ชื่อ string → `name: 'redirects'`                         | npm package หรือ inline function                  |
| Socket         | ไม่ต้อง                                                   | ใช้ socket registry                               |
| Response limit | ไม่มี                                                     | 32 MiB default, 256 MiB max                       |

### Socket Registry — การสื่อสารระหว่าง Rust และ JS

```
┌─────────────┐      WebSocket / IPC       ┌──────────────┐
│ Rust Server │ ◄───────────────────────►  │ Node/Bun     │
│ (Main)      │       JSON messages        │ Plugin Worker │
└─────────────┘                            └──────────────┘
```

Socket registry เป็น bi-directional IPC protocol ที่ Rust ใช้คุยกับ JS worker:

**Timing diagram**:

```
Rust Server                    Plugin Worker
    │                              │
    │── buildStart ──────────────► │
    │                              │ (worker ตั้งค่า)
    │◄── ack ───────────────────── │
    │                              │
    │── resolveId (source, imp) ► │
    │◄── result (resolved) ────── │
    │                              │
    │── transform (code, id) ────►│
    │◄── result (transformed) ────│
    │                              │
    │── middleware (request) ────►│
    │◄── result / pass ───────────│
    │                              │
    │── buildEnd ────────────────► │
    │                              │
```

**Socket messages — full protocol**:

| Message       | ทิศทาง          | Payload                        | Response                   |
| ------------- | --------------- | ------------------------------ | -------------------------- |
| `resolveId`   | Server → Worker | `{ source, importer }`         | `{ id: string }` or `null` |
| `transform`   | Server → Worker | `{ code, id }`                 | `{ code, map? }` or pass   |
| `buildStart`  | Server → Worker | `{ root, outDir, config }`     | `ack`                      |
| `buildEnd`    | Server → Worker | `{ manifest, diagnostics }`    | `ack`                      |
| `serverStart` | Server → Worker | `{ config }`                   | `ack`                      |
| `serverEnd`   | Server → Worker | `{}`                           | `ack`                      |
| `middleware`  | Server → Worker | `{ request, response }`        | modified request or pass   |
| `onRequest`   | Server → Worker | `{ request }`                  | modified request           |
| `onResponse`  | Server → Worker | `{ request, response }`        | modified response          |
| `onStart`     | Server → Worker | `{ root, outDir }`             | `ack`                      |
| `onComplete`  | Server → Worker | `{ duration, routes, assets }` | `ack`                      |
| `ping`        | Both            | `{ timestamp }`                | `{ timestamp }`            |

**Timeout ต่อ message**:

- resolveId: 5s
- transform: 30s
- middleware: 30s (configurable via `middleware.timeoutMs`)
- ping: 2s

ถ้า TypeScript plugin worker หรือ hook ไม่ตอบกลับภายใน timeout ให้ตรวจ error ที่ source รายงาน โดย
timeout ของ plugin host ใช้ `RUV1700`; ไม่ควรใช้ `RUV1502` เป็นรหัส timeout ปัจจุบัน

---

## Built-in Plugins (16 ตัว)

ตัวอย่างด้านล่างเป็น historical draft เท่านั้น Built-in plugin factory และ plugin worker bridge มี
execution path ต่างกันตาม factory/hook; source ไม่รับรองว่าไม่มี IPC overhead หรือมี performance
เท่ากันทุก plugin ให้ใช้ current production contract ด้านบนเป็นหลัก

### 1. `redirects` — URL Redirection

จัดการ URL redirections ทั้งแบบ permanent และ temporary พร้อม pattern matching:

```typescript
interface RedirectsPluginOptions {
  redirects: Array<{
    source: string // Source path pattern
    destination: string // Target path หรือ URL
    permanent?: boolean // true → 308, false → 307 (default: false)
    statusCode?: number // Custom status code (301, 302, 307, 308)
    has?: Record<string, string> // ต้องมี header/cookie/query นี้
    missing?: Record<string, string> // ต้องไม่มี header/cookie/query นี้
  }>
}
```

**ตัวอย่าง**:

```ts
plugins: [
  {
    name: 'redirects',
    options: {
      redirects: [
        // Permanent redirect
        { source: '/old-page', destination: '/new-page', permanent: true },
        // Pattern matching
        { source: '/blog/(.*)', destination: '/posts/$1', permanent: false },
        // Custom status
        { source: '/temp', destination: '/elsewhere', statusCode: 302 },
        // Conditional — redirect เฉพาะบางกรณี
        {
          source: '/promo',
          destination: '/promo/new',
          has: { cookie: 'promo_2024' },
        },
      ],
    },
  },
]
```

**Pattern syntax**:

- `(.*)` — capture group → ใช้ `$1` ใน destination
- `:param` — named parameter (เหมือน dynamic routes)
- `*` — wildcard (match ทุกอย่าง)
- `/blog/(.*)` → `/blog/hello-world` → destination `/posts/hello-world`

**Edge cases**:

- Redirect loop → Ruvyxa ตรวจ cycle และ error
- มากกว่า 20 redirects → warning
- Source path ไม่เริ่มด้วย `/` → auto-fix

---

### 2. `headers` — Custom HTTP Headers

ตั้งค่า custom HTTP headers ตาม route pattern:

```typescript
interface HeadersPluginOptions {
  headers: Array<{
    source: string // Route pattern (* = wildcard)
    headers: Array<{
      key: string // Header name
      value: string // Header value
    }>
  }>
}
```

```ts
plugins: [
  {
    name: 'headers',
    options: {
      headers: [
        {
          source: '/api/(.*)',
          headers: [
            { key: 'Cache-Control', value: 'no-store' },
            { key: 'X-API-Version', value: '1.0.0' },
            { key: 'Access-Control-Allow-Origin', value: '*' },
          ],
        },
        {
          source: '/assets/(.*)',
          headers: [{ key: 'Cache-Control', value: 'public, max-age=31536000, immutable' }],
        },
        {
          source: '/blog/(.*)',
          headers: [{ key: 'X-Robots-Tag', value: 'index, follow' }],
        },
      ],
    },
  },
]
```

**Priority**: headers จาก plugin แรกใน array มี priority ก่อน — ถ้าหลาย plugin match source
เดียวกัน, plugin แรกชนะ

---

### 3. `observability` — Health Check

เพิ่ม health check endpoint สำหรับ monitoring:

```typescript
interface ObservabilityPluginOptions {
  endpoint?: string // Path (default: '/api/health')
  detailed?: boolean // แสดงรายละเอียด (default: false)
  checks?: {
    database?: boolean // ตรวจ DB connection
    cache?: boolean // ตรวจ cache
    uptime?: boolean // แสดง uptime
  }
  customChecks?: Array<{
    name: string
    check: () => Promise<{ ok: boolean; message?: string }>
  }>
}
```

```ts
plugins: [
  {
    name: 'observability',
    options: {
      endpoint: '/api/health',
      detailed: true,
      checks: {
        database: true,
        cache: true,
        uptime: true,
      },
    },
  },
]
```

**Response** (detailed mode):

```json
GET /api/health
{
  "status": "ok",
  "uptime": 3600,
  "version": "1.0.0",
  "checks": {
    "database": { "ok": true, "latency": 5 },
    "cache": { "ok": true, "latency": 2 }
  },
  "timestamp": "2026-07-29T10:30:00Z"
}
```

ถ้า check ใดล้มเหลว → status เป็น `degraded` หรือ `down` HTTP status 200 (ok), 503 (degraded/down)

---

### 4. `securityHeaders` — Security Headers

เปิด security headers ทุกตัวที่จำเป็น — default ค่อนข้างเข้มงวด:

```typescript
interface SecurityHeadersPluginOptions {
  contentSecurityPolicy?: string | false // CSP header (false = ปิด)
  xFrameOptions?: 'DENY' | 'SAMEORIGIN' | 'ALLOW-FROM' | false
  xContentTypeOptions?: 'nosniff' | false
  referrerPolicy?: ReferrerPolicy | false
  permissionsPolicy?: string | false
  strictTransportSecurity?: string | false // HSTS (false = ปิด)
  xDNSPrefetchControl?: 'on' | 'off' | false
  xPermittedCrossDomainPolicies?: 'none' | 'master-only' | 'by-content-type' | 'all' | false
}
```

```ts
plugins: [
  {
    name: 'securityHeaders',
    options: {
      contentSecurityPolicy: "default-src 'self'; img-src 'self' https:; script-src 'self'",
      xFrameOptions: 'DENY',
      xContentTypeOptions: 'nosniff',
      strictTransportSecurity: 'max-age=63072000; includeSubDomains; preload',
      permissionsPolicy: 'camera=(), microphone=(), geolocation=(self)',
    },
  },
]
```

**Default headers** (ถ้าไม่ระบุ — จะได้ค่า default):

| Header                              | Default                           | คำอธิบาย              |
| ----------------------------------- | --------------------------------- | --------------------- |
| `X-Frame-Options`                   | `DENY`                            | ป้องกัน clickjacking  |
| `X-Content-Type-Options`            | `nosniff`                         | ป้องกัน MIME sniffing |
| `Referrer-Policy`                   | `strict-origin-when-cross-origin` | จำกัด referrer        |
| `Permissions-Policy`                | `camera=(), microphone=()`        | ปิดกล้อง/ไมค์         |
| `Strict-Transport-Security`         | `max-age=63072000`                | HSTS 2 ปี             |
| `X-DNS-Prefetch-Control`            | `on`                              | DNS prefetch          |
| `X-Permitted-Cross-Domain-Policies` | `none`                            | ป้องกัน cross-domain  |

**คำแนะนำ CSP สำหรับ production**:

| Use case              | CSP                                                                                                                       |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Basic                 | `default-src 'self'`                                                                                                      |
| With Google Analytics | `default-src 'self'; script-src 'self' https://www.googletagmanager.com; img-src 'self' https://www.google-analytics.com` |
| With external images  | `default-src 'self'; img-src 'self' https:`                                                                               |
| Strict                | `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:`                           |

---

### 5. `cacheRules` — CDN Cache Rules

ตั้งค่า cache rules สำหรับ CDN และ browser:

```typescript
interface CacheRulesPluginOptions {
  rules: Array<{
    pattern: string // Path pattern (glob)
    ttl: number // Cache TTL (seconds)
    swr?: number // Stale-while-revalidate (seconds)
    headers?: Record<string, string> // Custom cache headers
    browserTTL?: number // Browser cache TTL (ต่างจาก CDN)
  }>
}
```

```ts
plugins: [
  {
    name: 'cacheRules',
    options: {
      rules: [
        // Blog — cache 1 ชม.
        { pattern: '/blog/**', ttl: 3600, swr: 600 },
        // Static API — cache 1 วัน
        { pattern: '/api/static/**', ttl: 86400, browserTTL: 3600 },
        // Dynamic API — ไม่ cache
        { pattern: '/api/dynamic/**', ttl: 0 },
        // Assets — cache ถาวร
        { pattern: '/assets/**', ttl: 31536000, headers: { 'Cache-Control': 'public, immutable' } },
      ],
    },
  },
]
```

TTL values:

- `0` — ไม่ cache (no-store)
- `3600` — 1 ชั่วโมง
- `86400` — 1 วัน
- `604800` — 1 สัปดาห์
- `2592000` — 30 วัน
- `31536000` — 1 ปี

---

### 6. `pwa` — Progressive Web App

เพิ่ม PWA support — manifest, service worker, offline support:

```typescript
interface PwaPluginOptions {
  manifest: {
    name: string // ชื่อแอป
    short_name: string // ชื่อสั้น
    description: string // คำอธิบาย
    start_url?: string // URL เริ่มต้น (default: '/')
    display?: 'standalone' | 'fullscreen' | 'minimal-ui' | 'browser'
    background_color?: string // สีพื้นหลัง splash screen
    theme_color?: string // สีธีม
    orientation?: 'portrait' | 'landscape' | 'any'
    icons: Array<{
      src: string
      sizes: string // '192x192'
      type: string // 'image/png'
      purpose?: 'any' | 'maskable' | 'monochrome'
    }>
    screenshots?: Array<{
      src: string
      sizes: string
      type: string
      form_factor?: 'narrow' | 'wide'
    }>
    categories?: string[]
    iarc_rating_id?: string
  }
  serviceWorker?: {
    cacheName?: string // ชื่อ cache (default: 'ruvyxa-pwa-v1')
    preload?: string[] // URLs ที่ preload
    offlinePage?: string // หน้าสำหรับ offline
    navigationPreload?: boolean // Navigation preload
    runtimeCache?: Array<{
      urlPattern: RegExp
      handler: 'CacheFirst' | 'NetworkFirst' | 'StaleWhileRevalidate' | 'NetworkOnly' | 'CacheOnly'
      maxEntries?: number
      maxAgeSeconds?: number
    }>
  }
}
```

```ts
plugins: [
  {
    name: 'pwa',
    options: {
      manifest: {
        name: 'My App',
        short_name: 'App',
        description: 'คำอธิบายแอปพลิเคชัน',
        start_url: '/',
        display: 'standalone',
        background_color: '#ffffff',
        theme_color: '#000000',
        orientation: 'portrait',
        icons: [
          { src: '/icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: '/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
        ],
      },
      serviceWorker: {
        cacheName: 'my-app-v1',
        preload: ['/', '/offline'],
        navigationPreload: true,
        runtimeCache: [
          {
            urlPattern: /\/api\/public\//,
            handler: 'CacheFirst',
            maxEntries: 50,
            maxAgeSeconds: 86400,
          },
        ],
      },
    },
  },
]
```

**Output**:

- `.ruvyxa/assets/manifest.json` — Web App Manifest
- `.ruvyxa/assets/sw.js` — Service Worker
- หน้า `offline` — ถ้าระบุใน serviceWorker.offlinePage

---

### 7. `sitemap` — Sitemap XML

สร้าง `sitemap.xml` อัตโนมัติจาก route ทั้งหมด:

```typescript
interface SitemapPluginOptions {
  exclude?: string[] // Routes ที่ไม่เอาใน sitemap
  additionalPaths?: string[] // เส้นทางเพิ่มเติม (นอกเหนือจาก routes)
  defaults?: {
    changeFrequency?: 'always' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly' | 'never'
    priority?: number // 0.0 - 1.0
  }
  overrides?: Record<
    string,
    {
      // Per-path override
      changeFrequency?: string
      priority?: number
      lastModified?: Date
    }
  >
  maxEntries?: number // Sitemap index ถ้าเกิน (default: 50000)
}
```

```ts
plugins: [
  {
    name: 'sitemap',
    options: {
      exclude: ['/draft/*', '/api/*'],
      additionalPaths: ['/custom-page'],
      defaults: {
        changeFrequency: 'weekly',
        priority: 0.5,
      },
      overrides: {
        '/': { priority: 1.0, changeFrequency: 'daily' },
        '/about': { priority: 0.8 },
        '/blog/hello-world': { lastModified: new Date('2026-07-01') },
      },
    },
  },
]
```

**Output**: `.ruvyxa/assets/sitemap.xml`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/</loc>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
  <url>
    <loc>https://example.com/about</loc>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
  </url>
</urlset>
```

ถ้า `maxEntries` เกิน → สร้าง `sitemap-index.xml` ที่อ้างถึง sitemap ย่อย

---

### 8. `robots` — Robots.txt

สร้าง `robots.txt` สำหรับ SEO control:

```typescript
interface RobotsPluginOptions {
  rules: Array<{
    userAgent: string // 'Googlebot', 'GPTBot', '*' ฯลฯ
    allow?: string | string[] // อนุญาต path
    disallow?: string | string[] // ห้าม path
    crawlDelay?: number // หน่วงเวลา (วินาที)
  }>
  sitemap?: string // URL ของ sitemap
  additionalSitemaps?: string[] // Sitemap เพิ่มเติม
}
```

```ts
plugins: [
  {
    name: 'robots',
    options: {
      rules: [
        { userAgent: '*', allow: '/' },
        { userAgent: 'GPTBot', disallow: '/' },
        { userAgent: 'Googlebot', allow: '/', crawlDelay: 1 },
        { userAgent: 'Bingbot', allow: '/' },
        { userAgent: 'PetalBot', disallow: ['/api/', '/admin/'] },
      ],
      sitemap: 'https://example.com/sitemap.xml',
      additionalSitemaps: ['https://example.com/sitemap-news.xml'],
    },
  },
]
```

**Output**: `.ruvyxa/assets/robots.txt`

```
User-agent: *
Allow: /

User-agent: GPTBot
Disallow: /

User-agent: Googlebot
Allow: /
Crawl-delay: 1

Sitemap: https://example.com/sitemap.xml
Sitemap: https://example.com/sitemap-news.xml
```

---

### 9. `feed` — RSS / Atom / JSON Feed

สร้าง feed สำหรับ blog/content:

```typescript
interface FeedPluginOptions {
  type: 'rss' | 'atom' | 'json' // รูปแบบ feed
  title: string
  description: string
  filename?: string // เช่น 'feed.xml' (default: 'feed.xml')
  language?: string // RFC 5646 (default: 'en')
  imageUrl?: string
  faviconUrl?: string
  copyright?: string
  author?: {
    name: string
    email?: string
    link?: string
  }
  categories?: string[]
  items: Array<{
    title: string
    description: string
    link: string
    pubDate: Date
    content?: string // เนื้อหาเต็ม (HTML)
    author?: string
    categories?: string[]
    image?: string
    guid?: string // ถ้าไม่ระบุ → ใช้ link
  }>
  maxItems?: number // จำกัดจำนวน items (default: 100)
  ttl?: number // RSS TTL (นาที)
}
```

```ts
plugins: [
  {
    name: 'feed',
    options: {
      type: 'rss',
      title: 'My Blog',
      description: 'บล็อกของฉัน',
      filename: 'feed.xml',
      language: 'th',
      copyright: '2026 My Blog',
      author: { name: 'ผู้เขียน', email: 'author@example.com' },
      items: [
        {
          title: 'โพสต์แรก',
          description: 'คำอธิบาย',
          link: '/blog/first-post',
          pubDate: new Date('2026-07-01'),
          categories: ['tech'],
        },
      ],
      maxItems: 50,
      ttl: 60,
    },
  },
]
```

**Output**: `.ruvyxa/assets/feed.xml` (หรือ `.atom`, `.json`)

---

### 10. `searchIndex` — Search Index

สร้าง search index JSON สำหรับ client-side search:

```typescript
interface SearchIndexPluginOptions {
  include: string[] // Path patterns ที่รวม
  exclude?: string[] // Path patterns ที่ไม่รวม
  fields: Array<'title' | 'description' | 'content' | 'keywords' | 'author'>
  maxContentLength?: number // จำกัดความยาว content (default: 5000)
  output?: string // Output path (default: '.ruvyxa/assets/search-index.json')
}
```

```ts
plugins: [
  {
    name: 'searchIndex',
    options: {
      include: ['/blog/**', '/docs/**'],
      exclude: ['/draft/**', '/api/**'],
      fields: ['title', 'description', 'content'],
      maxContentLength: 3000,
    },
  },
]
```

**Output**: `.ruvyxa/assets/search-index.json`

```json
[
  {
    "path": "/blog/hello-world",
    "title": "Hello World",
    "description": "โพสต์แรกของฉัน",
    "content": "ย่อหน้าแรกของเนื้อหา... (trimmed to 3000 chars)"
  }
]
```

**การใช้ client-side**:

```tsx
'use client';
import { useState } from 'react';

export default function SearchBar() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState([]);

  async function handleSearch(e: React.ChangeEvent<HTMLInputElement>) {
    const q = e.target.value.toLowerCase();
    setQuery(q);
    if (q.length < 2) return setResults([]);

    const res = await fetch('/assets/search-index.json');
    const index = await res.json();
    setResults(
      index.filter(item =>
        item.title.toLowerCase().includes(q) ||
        item.description.toLowerCase().includes(q)
      )
    );
  }

  return ( /* UI */ );
}
```

---

### 11. `contentEngine` — Content Engine

Structured content management — จัดการ content จาก directory:

```typescript
interface ContentEnginePluginOptions {
  collections: Record<
    string,
    {
      // ชื่อ collection → config
      dir: string // Directory ของ content
      format?: 'md' | 'mdx' | 'json' // รูปแบบ (default: auto-detect)
      schema?: Record<string, any> // Zod/JSON Schema สำหรับ validate
      permalink?: string // URL pattern (เช่น '/posts/:slug')
      sortBy?: string // Field สำหรับ sort (default: 'date')
      sortOrder?: 'asc' | 'desc'
      paginate?: number // จำนวน items ต่อหน้า
    }
  >
  cache?: boolean // Cache content (default: true)
  watch?: boolean // Watch for changes (dev mode)
}
```

```ts
plugins: [
  {
    name: 'contentEngine',
    options: {
      collections: {
        posts: {
          dir: 'content/posts',
          format: 'mdx',
          permalink: '/blog/:slug',
          sortBy: 'date',
          sortOrder: 'desc',
          paginate: 10,
        },
        docs: {
          dir: 'content/docs',
          format: 'md',
          schema: {/* JSON Schema */},
        },
      },
    },
  },
]
```

**Directory structure**:

```
content/
├── posts/
│   ├── hello-world/
│   │   ├── index.mdx
│   │   └── hero.jpg
│   └── second-post.mdx
└── docs/
    ├── getting-started.md
    └── advanced.md
```

**Frontmatter ที่รองรับ**:

```yaml
---
title: Hello World
date: 2026-07-01
tags: [tech, tutorial]
author: Nritro
published: true
draft: false
summary: 'คำอธิบายสั้น'
---
```

---

### 12. `openApi` — OpenAPI Spec Generator

สร้าง OpenAPI specification จาก API routes อัตโนมัติ:

```typescript
interface OpenApiPluginOptions {
  title: string
  version: string
  description?: string
  servers?: Array<{ url: string; description?: string }>
  auth?: {
    type: 'bearer' | 'apiKey' | 'oauth2'
    name?: string
    in?: 'header' | 'query'
  }
  generateExamples?: boolean // สร้าง example responses (default: true)
  includePaths?: string[] // path patterns ที่รวม (default: all /api/*)
  excludePaths?: string[] // path patterns ที่ไม่รวม
}
```

```ts
plugins: [
  {
    name: 'openApi',
    options: {
      title: 'My API',
      version: '1.0.0',
      description: 'REST API documentation',
      servers: [
        { url: 'https://api.production.com', description: 'Production' },
        { url: 'https://staging.api.com', description: 'Staging' },
      ],
      auth: { type: 'bearer', name: 'Authorization', in: 'header' },
      generateExamples: true,
    },
  },
]
```

**Output**: `.ruvyxa/assets/openapi.json`

```json
{
  "openapi": "3.0.3",
  "info": {
    "title": "My API",
    "version": "1.0.0",
    "description": "REST API documentation"
  },
  "paths": {
    "/api/users": {
      "get": {/* ... */},
      "post": {/* ... */}
    }
  }
}
```

**API route annotation** (optional — ช่วยเพิ่มรายละเอียด):

```ts
// app/api/users/route.ts
/** @openapi
 * /api/users:
 *   get:
 *     summary: ดึงรายชื่อผู้ใช้
 *     tags: [Users]
 *     parameters:
 *       - name: limit
 *         in: query
 *         schema: { type: integer }
 *     responses:
 *       200:
 *         description: Success
 */
export async function GET() { ... }
```

---

### 13. `alias` — Import Alias

ตั้งค่า import aliases เพื่อลด relative path:

```typescript
interface AliasPluginOptions {
  [alias: string]: string // key → path
}
```

```ts
plugins: [
  {
    name: 'alias',
    options: {
      '@': './src',
      '@components': './src/components',
      '@utils': './src/utils',
      '@ui': './src/components/ui',
      '@lib': './src/lib',
      '@server': './src/server',
      '@assets': './public/assets',
    },
  },
]
```

จากนั้นใช้ import:

```ts
// ก่อน — relative path ยาว
import Button from '../../../components/ui/Button'
import { formatDate } from '../../../utils/date'

// หลัง — alias สั้น
import Button from '@ui/Button'
import { formatDate } from '@utils/date'
```

---

### 14. `bundleBudget` — Bundle Size Budget

กำหนด budget สำหรับ bundle size — build ล้มเหลวถ้าเกิน:

```typescript
interface BundleBudgetPluginOptions {
  budgets: Array<{
    type: 'total' | 'initial' | 'page' | 'chunk' | 'image' | 'font' | 'css'
    maxSize: string // '500KB', '1MB' ฯลฯ
    pattern?: string // สำหรับ type=page/chunk (glob)
    warning?: boolean // แค่ warning ไม่ fail build (default: false)
    error?: string // Custom error message
  }>
}
```

```ts
plugins: [
  {
    name: 'bundleBudget',
    options: {
      budgets: [
        { type: 'total', maxSize: '500KB' },
        { type: 'initial', maxSize: '200KB' },
        { type: 'page', pattern: '/**', maxSize: '300KB' },
        { type: 'page', pattern: '/dashboard/**', maxSize: '500KB' },
        { type: 'image', maxSize: '1MB' },
        { type: 'css', maxSize: '100KB' },
        { type: 'chunk', maxSize: '250KB' },
      ],
    },
  },
]
```

**Output เมื่อเกิน budget**:

```
RUV1702: Bundle budget exceeded
  /                 total: 520KB > 500KB
  /about            initial: 250KB > 200KB
  /assets/hero.jpg  image: 1.5MB > 1MB

  Tip: Use dynamic import() for large pages, optimize images
```

---

### 15. `requireEnv` — Required Environment Variables

ตรวจสอบ environment variables ก่อน build — ป้องกัน deploy โดยไม่ได้ตั้งค่าที่จำเป็น:

```typescript
interface RequireEnvPluginOptions {
  vars: Array<string | { name: string; message?: string; pattern?: string }>
  mode?: 'build' | 'start' | 'both' // เมื่อไรที่ตรวจ (default: 'build')
  strict?: boolean // Fail build ทันที (default: true)
}
```

```ts
plugins: [
  {
    name: 'requireEnv',
    options: {
      vars: [
        'DATABASE_URL',
        'AUTH_SECRET',
        { name: 'RUVYXA_PUBLIC_API_URL', message: 'API endpoint สำหรับ client' },
        { name: 'NEXT_PUBLIC_GA_ID', pattern: '^UA-|^G-', message: 'Google Analytics ID' },
      ],
      mode: 'build',
      strict: true,
    },
  },
]
```

**Output เมื่อ env ขาด**:

```
Missing required environment variables
  ✗ DATABASE_URL    (ไม่ได้ตั้งค่า)
  ✗ AUTH_SECRET     (ไม่ได้ตั้งค่า)
  ⚠ RUVYXA_PUBLIC_API_URL (API endpoint สำหรับ client)

  Build failed — set these variables before deploying
```

---

### 16. `fonts` — Font Management

จัดการ fonts — Google Fonts, local fonts, variable fonts:

```typescript
interface FontsPluginOptions {
  families: Array<
    | string // Google Fonts string: 'Inter:wght@400;500;700'
    | {
        // Full config
        name: string
        weights: number[]
        display?: 'auto' | 'block' | 'swap' | 'fallback' | 'optional'
        variable?: boolean // Variable font?
        src?: string[] // Local font files
        format?: 'woff2' | 'woff' | 'ttf'
        unicodeRange?: string
      }
  >
  preload?: Array<{
    // Preload fonts
    url: string
    as?: 'font' | 'style'
    crossOrigin?: boolean
  }>
  selfHost?: boolean // Self-host fonts (ไม่ใช้ Google CDN)
}
```

```ts
plugins: [
  {
    name: 'fonts',
    options: {
      families: [
        // Google Fonts — string shorthand
        'Inter:wght@400;500;700',
        'Noto+Sans+Thai:wght@400;700',
        // Full config
        {
          name: 'Kanit',
          weights: [300, 400, 500, 700],
          display: 'swap',
          variable: false,
        },
        // Local font
        {
          name: 'CustomFont',
          weights: [400],
          src: ['/fonts/custom.woff2'],
          format: 'woff2',
          display: 'swap',
        },
      ],
      preload: [{ url: '/fonts/inter-latin.woff2', as: 'font', crossOrigin: true }],
      selfHost: true,
    },
  },
]
```

**Output**:

- Font files ใน `.ruvyxa/assets/fonts/`
- `@font-face` CSS declarations
- Preload `<link>` tags ใน `<head>`

---

## ระบบ Hooks ใหม่ (v0.5+)

Ruvyxa มี 2 ระบบ hooks — Build Hooks (สำหรับระหว่าง build) และ HTTP Hooks (สำหรับ runtime)

### build.onStart

เรียกเมื่อ build เริ่มต้น — ใช้ตั้งค่าเริ่มต้น, ตรวจสอบ conditions:

```typescript
type BuildOnStart = (ctx: {
  root: string // Project root directory
  outDir: string // Output directory (.ruvyxa)
  config: Record<string, any> // Full config object
  env: Record<string, string> // Environment variables snapshot
}) => void | Promise<void>
```

**ตัวอย่าง**: ตรวจสอบ Node.js version

```ts
build: {
  onStart({ root, config }) {
    const nodeMajor = parseInt(process.versions.node, 10);
    if (nodeMajor < 20) {
      throw new Error('Node.js 20+ required');
    }
    console.log(`Building ${config.site?.name || 'app'} from ${root}`);
  },
}
```

### build.onResolve

ปรับเปลี่ยน module resolution — แก้ไข import paths:

```typescript
type BuildOnResolve = (ctx: {
  source: string // Import source: './Button', 'react', etc.
  importer: string // File ที่ import
  resolve: (id: string) => string | null // Default resolver
}) => string | null | undefined // Return resolved path หรือ null
```

**ตัวอย่าง**: แทนที่ moment ด้วย dayjs

```ts
build: {
  onResolve({ source, resolve }) {
    if (source === 'moment') {
      return resolve('dayjs');
    }
    // อย่าลืม return undefined ถ้าไม่ต้องการแก้ไข
  },
}
```

### build.onTransform

แปลง source code ก่อน bundle:

```typescript
type BuildOnTransform = (ctx: {
  code: string // Source code
  id: string // Module path
  resolve: (id: string) => string // Resolver
}) => { code: string; map?: string } | undefined | void
```

**ตัวอย่าง**: ลบ console.log ใน production

```ts
build: {
  onTransform({ code, id }) {
    if (process.env.NODE_ENV === 'production' && id.endsWith('.tsx')) {
      return {
        code: code.replace(/console\.\w+\([^)]*\)/g, '/* removed */'),
      };
    }
  },
}
```

### build.onComplete

เรียกเมื่อ build เสร็จ — ใช้รายงานผล, cleanup:

```typescript
type BuildOnComplete = (ctx: {
  duration: number // Build duration (ms)
  routes: number // จำนวน route
  assets: { count: number; size: number } // Asset stats
  manifest: RouteManifest // Route manifest
  diagnostics: Diagnostic[] // Warnings/errors
}) => void | Promise<void>
```

**ตัวอย่าง**: แจ้ง Slack เมื่อ build เสร็จ

```ts
build: {
  async onComplete({ duration, routes, diagnostics }) {
    const errors = diagnostics.filter(d => d.severity === 'error');
    if (errors.length > 0) {
      await fetch(process.env.SLACK_WEBHOOK!, {
        method: 'POST',
        body: JSON.stringify({
          text: `Build failed: ${errors.length} errors in ${duration}ms`,
        }),
      });
    }
  },
}
```

### http.onRequest

ปรับเปลี่ยน request ก่อนถึง route handler:

```typescript
type HttpOnRequest = (ctx: {
  request: {
    method: string
    url: string
    headers: Record<string, string>
    body?: any
  }
  params: Record<string, string> // Route params
}) => {
  request?: Partial<PluginHttpRequest> // เปลี่ยน request
  response?: PluginHttpResponse // หรือตอบเลย
} | void
```

**ตัวอย่าง**: Rate limiting

```ts
http: {
  onRequest({ request }) {
    const ip = request.headers['x-forwarded-for'] || 'unknown';
    const key = `rate:${ip}`;
    // ตรวจ rate limit...
    if (isRateLimited(key)) {
      return {
        response: { status: 429, body: 'Too Many Requests' },
      };
    }
  },
}
```

### http.onResponse

ปรับเปลี่ยน response ก่อนส่งกลับ client:

```typescript
type HttpOnResponse = (ctx: { request: PluginHttpRequest; response: PluginHttpResponse }) => {
  response?: Partial<PluginHttpResponse>
} | void
```

**ตัวอย่าง**: เพิ่ม custom headers

```ts
http: {
  onResponse({ response }) {
    return {
      response: {
        headers: {
          ...response.headers,
          'X-Powered-By': 'Ruvyxa',
          'X-Response-Time': `${Date.now() - start}ms`,
        },
      },
    };
  },
}
```

### Hooks Legacy (v0.4)

Hooks เดิมยังใช้งานได้ — `definePlugin`:

```typescript
type LegacyHookResolveId = (ctx: {
  source: string
  importer: string
  resolve: (id: string) => string | null
}) => string | null | undefined

type LegacyHookTransform = (ctx: {
  code: string
  id: string
  resolve: (id: string) => string
}) => { code: string; map?: string } | undefined

type LegacyHookBuildStart = (ctx: {
  root: string
  outDir: string
  config: Record<string, any>
}) => void

type LegacyHookBuildEnd = (ctx: { manifest: RouteManifest; diagnostics: Diagnostic[] }) => void

type LegacyHookServerStart = (ctx: { config: ServerConfig }) => void
type LegacyHookServerEnd = (ctx: {}) => void

type LegacyHookMiddleware = (ctx: {
  request: PluginHttpRequest
  response: PluginHttpResponse
  next: () => Promise<void>
}) => Promise<PluginHttpRequestResult | void>
```

---

## การสร้าง Custom Plugin — ฉบับสมบูรณ์

### definePlugin API (แนะนำ)

ใช้ `definePlugin` จาก `ruvyxa/plugins`:

```typescript
// src/index.ts
import { definePlugin } from 'ruvyxa/plugins'
import type { Plugin } from 'ruvyxa/plugins'

interface MyPluginOptions {
  prefix?: string
  debug?: boolean
}

export default definePlugin<MyPluginOptions>('my-plugin', (options = {}) => {
  return {
    name: 'my-plugin',

    // Build hooks (v0.5+)
    build: {
      onStart(ctx) {
        if (options.debug) {
          console.log('Build starting:', ctx.root)
        }
      },

      onResolve(ctx) {
        if (ctx.source.startsWith('my:')) {
          return ctx.resolve(ctx.source.slice(3))
        }
      },

      onTransform(ctx) {
        if (options.debug && ctx.id.endsWith('.tsx')) {
          console.log('Transforming:', ctx.id)
        }
      },

      onComplete(ctx) {
        console.log(`Built ${ctx.routes} routes in ${ctx.duration}ms`)
      },
    },

    // HTTP hooks (v0.5+)
    http: {
      onRequest(ctx) {
        const url = ctx.request.url
        if (url.startsWith(options.prefix || '/api')) {
          console.log('Request:', ctx.request.method, url)
        }
      },

      onResponse(ctx) {
        // Add custom header
        return {
          response: {
            headers: {
              'X-My-Plugin': 'v1',
            },
          },
        }
      },
    },

    // Legacy hooks fallback
    hooks: {
      buildStart(ctx) {
        /* ... */
      },
      middleware(ctx) {
        /* ... */
      },
    },

    // Head contribution
    head: [{ tag: 'meta', attrs: { name: 'my-plugin', content: 'active' } }],
  }
})
```

### Plugin Response Limits

| ขีดจำกัด                               | ค่า Default | Max                              | Error        |
| -------------------------------------- | ----------- | -------------------------------- | ------------ |
| Response body size (plugin middleware) | 32 MiB      | 256 MiB                          | RUV1602      |
| Hook timeout (onTransform)             | 30s         | 300s                             | RUV1602      |
| Hook timeout (onResolve)               | 5s          | 30s                              | RUV1602      |
| Worker count                           | 1           | 8                                | RUV1602      |
| Socket message size                    | 1 MiB       | 16 MiB                           | RUV1503      |
| Plugin name length                     | —           | source ไม่กำหนดเป็น public limit | ไม่ระบุ code |

**การปรับค่า**:

```ts
// ruvyxa.config.ts
export default defineConfig({
  plugins: [myPlugin({/* options */})],
  middleware: {
    workers: 4,
    timeoutMs: 60000,
    pluginLimit: 64 * 1024 * 1024, // 64 MiB
  },
})
```

### Plugin Ordering — Algorithm

Plugin ทำงานตามลำดับใน `plugins` array:

```
plugins: [
  redirects,          // (1) ทำงานก่อน — แก้ไข URL ก่อนใคร
  headers,            // (2) ทำงานหลัง redirects
  securityHeaders,    // (3) ทำงานหลัง headers
  cacheRules,         // (4)
  myPlugin,           // (5) custom plugin
]
```

**กฎ**:

1. onRequest — ทำงานตามลำดับ plugin แรก → สุดท้าย (waterfall)
   - Plugin แรกเห็น request ก่อนคนอื่น
   - ถ้า plugin ก่อนหน้าส่ง response → plugin หลังไม่ถูกเรียก
2. onResponse — ทำงาน reverse order (plugin สุดท้าย → แรก)
   - Plugin สุดท้ายเห็น response ก่อนส่งกลับ
3. Build hooks — ทำงานตามลำดับ array
   - onStart: plugin แรก → สุดท้าย
   - onComplete: plugin สุดท้าย → แรก
4. Middleware (legacy) — ทำงานตามลำดับ `next()` call
   - แต่ละ plugin wrap รอบ `next()` — คล้าย Express/Koa onion

### Head Contribution

Plugin สามารถเพิ่ม elements ใน `<head>`:

```ts
// head array — meta tags, links, scripts
head: [
  { tag: 'meta', attrs: { name: 'author', content: 'Ruvyxa' } },
  { tag: 'link', attrs: { rel: 'canonical', href: 'https://example.com' } },
  { tag: 'script', attrs: { src: '/analytics.js', defer: 'true' } },
  { tag: 'style', content: 'body { margin: 0; }' },
]
```

หรือใน config:

```ts
plugins: [
  {
    name: 'my-plugin',
    head: [{ tag: 'meta', attrs: { name: 'theme-color', content: '#000' } }],
  },
]
```

---

## ลำดับการรันปลั๊กอิน (Plugin Ordering)

Plugins run in declaration order. When multiple plugins hook the same event:

```typescript
plugins: [
  redirects([{ source: '/old', destination: '/new' }]), // 1st: http.onRequest
  securityHeaders({ contentSecurityPolicy: "default-src 'self'" }), // 2nd: http.onResponse
  headers([{ source: '/api/*', headers: { 'x-foo': 'bar' } }]), // 3rd: http.onResponse
]
```

**General rule**: Build-time plugins before server-time plugins. Redirects and security first, then
headers and cache rules, then build-output plugins (sitemap, robots, pwa).

### Ordering Within Same Hook

For `http.onRequest` and `http.onResponse`, handlers registered by earlier plugins run first. Each
handler can call `next()` to pass control. If a handler returns a `Response` without calling
`next()`, subsequent handlers are skipped.

For `build.onResolve`, the first plugin that returns a non-null string wins. Subsequent `onResolve`
handlers are not called for that specifier.

---

## ข้อจำกัดการรันปลั๊กอิน (Plugin Execution Limits)

### Response Body Limit

TypeScript response middleware has a configurable buffer limit:

```typescript
// ruvyxa.config.ts
export default config({
  security: {
    pluginLimit: 33_554_432, // 32 MiB default, max 268_435_456 (256 MiB)
  },
})
```

If response middleware produces a buffered body exceeding this limit, the framework returns a 500
error. Binary streams and large file downloads should skip response middleware.

### Timeout

Plugin hooks have a configurable timeout via `middleware.timeoutMs`:

```typescript
export default config({
  middleware: {
    timeoutMs: 30_000, // 30 seconds default, max 300_000 (5 minutes)
  },
})
```

If exceeded: `RUV1700 TypeScript plugin hook timed out`. The worker is replaced. Timed-out hooks are
not retried.

### Worker Count

```typescript
export default config({
  middleware: {
    workers: 1, // 1-8, default 1
  },
})
```

Workers do not share module-level plugin state. Keep at 1 unless plugins are stateless and
throughput-bottlenecked.

---

## Plugin Registry — npm Publishing

### ขั้นตอนการสร้าง

```bash
# 1. สร้าง plugin project
ruvyxa plugin create my-plugin
cd ruvyxa-plugin-my-plugin

# ดูโครงสร้าง
tree
.
├── src/
│   └── index.ts          # Plugin source
├── test/
│   └── index.test.ts     # Tests
├── package.json
├── tsconfig.json
└── README.md

# 2. พัฒนา
npm run dev

# 3. ทดสอบ
npm test

# 4. Build
npm run build

# 5. Publish
npm publish --access public
```

### การตั้งชื่อ

```
npm:  ruvyxa-plugin-<name>
export default:  <name>
```

ตัวอย่าง:

- `ruvyxa-plugin-request-logger` → `import requestLogger from 'ruvyxa-plugin-request-logger'`
- `ruvyxa-plugin-env-validator` → `import envValidator from 'ruvyxa-plugin-env-validator'`

### package.json

```json
{
  "name": "ruvyxa-plugin-my-plugin",
  "version": "1.0.0",
  "description": "Ruvyxa plugin — คำอธิบาย",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "files": ["dist"],
  "scripts": {
    "build": "tsc",
    "dev": "tsc --watch",
    "test": "vitest run"
  },
  "peerDependencies": {
    "ruvyxa": ">=0.1.0"
  },
  "keywords": ["ruvyxa", "plugin"]
}
```

### การใช้ Custom Plugin

วิธีที่ 1 — Import โดยตรง (แนะนำ):

```ts
// ruvyxa.config.ts
import { defineConfig } from 'ruvyxa/config'
import myPlugin from 'ruvyxa-plugin-my-plugin'

export default defineConfig({
  plugins: [myPlugin({ prefix: '/api', debug: true })],
})
```

วิธีที่ 2 — Name-based (published plugins):

```ts
plugins: [{ name: 'my-plugin', options: { prefix: '/api', debug: true } }]
```

Ruvyxa จะหา `ruvyxa-plugin-<name>` ใน node_modules โดยอัตโนมัติ

วิธีที่ 3 — Inline plugin:

```ts
import { definePlugin } from 'ruvyxa/plugins'

plugins: [
  definePlugin('inline-plugin', () => ({
    name: 'inline-plugin',
    build: {
      onStart(ctx) {
        console.log('Inline plugin ready')
      },
    },
    head: [{ tag: 'meta', attrs: { name: 'inline', content: 'true' } }],
  }))(),
]
```

### การทดสอบ Plugin

```ts
// test/index.test.ts
import { describe, it, expect } from 'vitest'
import myPlugin from '../src'

describe('my-plugin', () => {
  it('returns plugin object with name', () => {
    const plugin = myPlugin({ prefix: '/api' })
    expect(plugin.name).toBe('my-plugin')
  })

  it('has build hooks', () => {
    const plugin = myPlugin()
    expect(plugin.build?.onStart).toBeDefined()
    expect(plugin.build?.onComplete).toBeDefined()
  })

  it('can transform code', async () => {
    const plugin = myPlugin({ debug: true })
    const result = await plugin.build?.onTransform?.({
      code: 'console.log("hello")',
      id: 'test.ts',
      resolve: (id) => id,
    })
    // ถ้า debug → ไม่ modified
    expect(result).toBeUndefined()
  })
})
```

---

## ตัวอย่าง Plugin จริง — 5 ตัว

### 1. Request Logger

```ts
import { definePlugin } from 'ruvyxa/plugins'

interface LoggerOptions {
  format?: 'json' | 'text'
  logHeaders?: boolean
}

export default definePlugin<LoggerOptions>('request-logger', (options = {}) => ({
  name: 'request-logger',

  http: {
    onRequest(ctx) {
      ctx.request.headers['x-request-start'] = Date.now().toString()
    },

    onResponse(ctx) {
      const start = parseInt(ctx.request.headers['x-request-start'] || '0')
      const duration = Date.now() - start
      const { method, url } = ctx.request
      const { status } = ctx.response

      if (options.format === 'json') {
        console.log(JSON.stringify({ method, url, status, duration }))
      } else {
        console.log(`${method} ${url} → ${status} (${duration}ms)`)
      }

      if (options.logHeaders) {
        console.log('Request headers:', ctx.request.headers)
      }
    },
  },
}))
```

### 2. Env Validator

```ts
import { definePlugin } from 'ruvyxa/plugins'

interface EnvOptions {
  required: string[]
  prefix?: string
  strict?: boolean
}

export default definePlugin<EnvOptions>('env-validator', (options) => ({
  name: 'env-validator',

  build: {
    onStart({ config }) {
      const missing = options.required.filter((key) => !process.env[key])

      if (missing.length > 0) {
        if (options.strict !== false) {
          throw new Error(`Missing required env vars: ${missing.join(', ')}`)
        } else {
          console.warn(`Warning: Missing env vars: ${missing.join(', ')}`)
        }
      }

      if (options.prefix) {
        const vars = Object.keys(process.env).filter((k) => k.startsWith(options.prefix!))
        console.log(`Found ${vars.length} ${options.prefix}* variables`)
      }
    },
  },
}))
```

### 3. Cache Buster

```ts
import { definePlugin } from 'ruvyxa/plugins'

export default definePlugin('cache-buster', () => ({
  name: 'cache-buster',

  build: {
    onComplete({ duration, routes }) {
      // สร้าง cache buster file สำหรับ CDN
      const fs = require('fs')
      const hash = Date.now().toString(36)
      fs.writeFileSync(
        '.ruvyxa/assets/cache-version.json',
        JSON.stringify({
          version: hash,
          builtAt: new Date().toISOString(),
          routes,
          duration,
        }),
      )
      console.log(`Cache version: ${hash}`)
    },
  },

  http: {
    onResponse(ctx) {
      // Add cache buster query param
      if (ctx.response.headers['content-type']?.includes('text/html')) {
        // NOOP — cache buster สำหรับ assets
      }
    },
  },
}))
```

### 4. Response Time Header

```ts
import { definePlugin } from 'ruvyxa/plugins'

export default definePlugin('response-time', () => ({
  name: 'response-time',

  http: {
    onRequest(ctx) {
      ctx.request.headers['x-start-time'] = String(performance.now())
    },

    onResponse(ctx) {
      const start = parseFloat(ctx.request.headers['x-start-time'] || '0')
      const elapsed = performance.now() - start
      return {
        response: {
          headers: {
            ...ctx.response.headers,
            'X-Response-Time': `${Math.round(elapsed)}ms`,
          },
        },
      }
    },
  },
}))
```

### 5. S3 Image Upload

```ts
import { definePlugin } from 'ruvyxa/plugins'

interface S3Options {
  bucket: string
  region?: string
  pathPrefix?: string
}

export default definePlugin<S3Options>('s3-upload', (options) => ({
  name: 's3-upload',

  build: {
    async onComplete({ assets }) {
      const { S3Client, PutObjectCommand } = require('@aws-sdk/client-s3')
      const client = new S3Client({ region: options.region || 'ap-southeast-1' })

      for (const asset of assets.images) {
        const key = `${options.pathPrefix || 'assets'}/${asset.name}`
        await client.send(
          new PutObjectCommand({
            Bucket: options.bucket,
            Key: key,
            Body: require('fs').readFileSync(asset.path),
            ContentType: asset.mimeType,
          }),
        )
        console.log(`Uploaded: ${key}`)
      }
    },
  },
}))
```

---

## Troubleshooting — ฉบับละเอียด

| ปัญหา                     | Error Code        | สาเหตุ                        | วิธีแก้                                                          |
| ------------------------- | ----------------- | ----------------------------- | ---------------------------------------------------------------- |
| Plugin ไม่ทำงาน           | RUV1603           | ชื่อ plugin ไม่ถูกต้อง        | ตรวจชื่อใน `plugins` array — ต้องตรงกับ npm package              |
| Built-in plugin ไม่มีผล   | —                 | ชื่อผิดหรือพิมพ์ผิด           | ใช้ชื่อที่ถูกต้อง: `redirects`, `headers`, `securityHeaders` ฯลฯ |
| Hook failure              | ไม่มี code ตายตัว | Error ใน plugin code          | ดู stack trace ใน terminal — ใช้ try/catch                       |
| Socket error              | RUV1503           | Worker disconnected           | เพิ่ม middleware timeout, restart dev server                     |
| Workers เกิน              | RUV1602           | เกินขีดจำกัด                  | ตั้ง `middleware.workers` ≤ 8                                    |
| Response body ใหญ่เกิน    | RUV1602           | Plugin ส่ง response > 32MiB   | จำกัด response body หรือเพิ่ม `pluginLimit`                      |
| Plugin หาไม่เจอ           | RUV1603           | ไม่ได้ติดตั้ง package         | `npm install ruvyxa-plugin-<name>`                               |
| Build ช้าลงมาก            | —                 | Plugin transform หนัก         | optimize hook หรือใช้ built-in plugin แทน                        |
| Middleware ไม่ทำงาน       | —                 | ขาด `next()` call             | เรียก `ctx.next()` ทุกครั้ง                                      |
| onRequest not called      | —                 | HTTP hooks not registered     | ใช้ `definePlugin` API (v0.5+)                                   |
| onResponse ไม่มีผล        | —                 | ต้อง return response object   | `return { response: { headers: {...} } }`                        |
| Head ไม่แสดง              | —                 | Head contribution ไม่ถูกเรียก | ตรวจ `head` array syntax                                         |
| Plugin ordering ผิด       | —                 | เข้าใจผิดเรื่อง priority      | onRequest → array order, onResponse → reverse                    |
| Socket timeout            | RUV1700           | Plugin hook ทำงานเกิน timeout | ตรวจ `middleware.timeoutMs` และลดงานใน hook                      |
| Multiple plugins conflict | —                 | สอง plugin แก้ไขสิ่งเดียวกัน  | เปลี่ยนลำดับหรือ merge logic                                     |

### RUV1602 — Plugin Config Shape Invalid

เกิดเมื่อ plugin options มีค่าผิด:

```ts
// ❌ ผิด
plugins: [
  {
    name: 'redirects',
    options: { redirects: 'ไม่ใช่ array' }, // ต้องเป็น array
  },
]

// ✅ ถูก
plugins: [
  {
    name: 'redirects',
    options: {
      redirects: [{ source: '/old', destination: '/new' }],
    },
  },
]
```

### RUV1602 — Plugin Config Out of Range

เกิดเมื่อค่าเกินขีดจำกัด:

| Field                    | ขั้นต่ำ | สูงสุด                |
| ------------------------ | ------- | --------------------- |
| `middleware.workers`     | 1       | 8                     |
| `middleware.timeoutMs`   | 1       | 300,000               |
| `middleware.pluginLimit` | 1       | 268,435,456 (256 MiB) |
| `security.actionLimit`   | 1       | 1,048,576 (1 MiB)     |
| `security.apiLimit`      | 1       | 5,242,880 (5 MiB)     |
| `security.pluginLimit`   | 1       | 5,242,880 (5 MiB)     |

### RUV1603 — Plugin Not Found

```bash
# ตรวจสอบ
npm ls ruvyxa-plugin-my-plugin  # ติดตั้งไหม?
ls node_modules/ruvyxa-plugin-*  # มี plugin อะไรบ้าง

# แก้ไข
npm install ruvyxa-plugin-my-plugin
```

### Plugin Hook Failure

```ts
// ❌ ต้นเหตุ
hooks: {
  onStart() {
    throw new Error('Oops');
  },
}

// ✅ แก้ไข
hooks: {
  onStart() {
    try {
      doRiskyOperation();
    } catch (e) {
      console.error('Plugin failed:', e);
      // หรือ rethrow เพื่อให้ plugin runtime รายงานข้อผิดพลาด
      throw e;
    }
  },
}
```

---

## ลองทำดู

1. เปิด `ruvyxa.config.ts` และเพิ่ม plugin redirects — redirect `/old` → `/new`
2. ทดลองเพิ่ม `securityHeaders` plugin — ดู headers ใน DevTools
3. เพิ่ม `fonts` plugin ด้วย Google Fonts Inter + Noto Sans Thai
4. สร้าง custom plugin ด้วย `ruvyxa plugin create`
5. ใช้ `build.onTransform` เพื่อแทนที่ text ใน production build
6. ใช้ `http.onRequest` เพื่อเพิ่ม rate limiting
7. ใช้ `head` field เพื่อเพิ่ม Google Analytics script
8. ลงทะเบียน plugin ใน config แล้วรัน dev — ดู logs
9. ทดสอบ plugin ordering — สลับลำดับใน array
10. Publish plugin ของคุณไปยัง npm — `npm publish --access public`
11. ทดลอง `middleware.pluginLimit` — เพิ่มเป็น 64MiB
12. ใช้ definePlugin API สำหรับ plugin ใหม่ทั้งหมด

---

## สรุป

- 16 built-in plugins — redirects, headers, observability, securityHeaders, cacheRules, pwa,
  sitemap, robots, feed, searchIndex, contentEngine, openApi, alias, bundleBudget, requireEnv, fonts
- TypeScript plugin system — 2 API sets: definePlugin (new) + hooks (legacy)
- Build hooks: onStart, onResolve, onTransform, onComplete
- HTTP hooks: onRequest, onResponse
- Socket registry — bi-directional IPC ระหว่าง Rust ↔ JS Worker
- Plugin ordering — array order for onRequest, reverse for onResponse
- Response limits — 32 MiB default, 256 MiB max
- Plugin naming: `ruvyxa-plugin-<name>` บน npm
- Head contribution — SEO, analytics, custom tags
- 5 ตัวอย่าง plugin จริง — request logger, env validator, cache buster, response time, S3 upload
- Troubleshooting — 14 ปัญหาพร้อม error codes และวิธีแก้

---

## Error Codes (RUV1600-1699, RUV2000-2102)

| Code         | Title                               | Source           | Fix                                     |
| ------------ | ----------------------------------- | ---------------- | --------------------------------------- |
| RUV1007-1010 | Plugin boundary violation           | Graph/bundler    | Fix the reported server/client boundary |
| RUV1700      | Plugin hook timeout or host failure | Plugin runtime   | Inspect the hook error and timeout      |
| RUV1701      | Plugin bridge/protocol error        | Plugin runtime   | Inspect the plugin response             |
| RUV2102      | Invalid plugin definition           | `definePlugin()` | Return a valid plugin object            |
| RUV2103      | Font self-hosting warning           | `fonts()` plugin | Check the font URL/network              |

---

## Plugin Boundaries และ Minimal Safe Plugin

public plugin constructor คือ `definePlugin()` จาก `@ruvyxa/core/plugin` (re-export ผ่าน
`ruvyxa/plugin`) plugin ต้องมีชื่อที่ไม่ว่าง และมี behavior อย่างน้อยหนึ่งอย่าง: registration
callback, HTTP behavior, build hooks, development file-change behavior, diagnostics, native
capability หรือ head entries constructor จะ validate ก่อน register plugin

```ts
import { definePlugin, withResponseHeader } from '@ruvyxa/core/plugin'

export default definePlugin({
  name: 'example:request-id',
  http: {
    match: '/api/*',
    onResponse({ response }) {
      return withResponseHeader(response, 'x-example-plugin', 'enabled')
    },
  },
})
```

นำค่าที่คืนมาไป register ใน `ruvyxa.config.ts` route pattern `*` match ทุก path, trailing `*` เป็น
prefix pattern และรูปแบบอื่นเป็น exact match เก็บ request/response work ให้มีขอบเขต เพราะ plugin
runtime communication เป็น system boundary; hook ที่แพงหรือกว้างกระทบทุก matched request

### เลือก Capability ไม่ใช่ชื่อเชิงการตลาด

first-party `ruvyxa/plugins` ปัจจุบัน export `redirects`, `headers`, `observability`,
`securityHeaders`, `cacheRules`, `pwa`, `sitemap`, `robots`, `feed`, `searchIndex`, `contentEngine`,
`openApi`, `alias`, `bundleBudget`, `requireEnv` และ `fonts` ให้อ่าน options type ของ capability
ที่ใช้จริง เพราะชื่อ plugin ใน tutorial ไม่ใช่สิ่งแทน current contract

### Scaffold แล้วพิสูจน์ Behavior ที่เล็กที่สุด

```bash
ruvyxa plugin create @acme/request-id --dir packages/request-id
ruvyxa analyze --format human
ruvyxa build
```

CLI scaffolder สร้าง publishable package structure แต่ไม่ได้ register package ใน application config
หรือ publish ไป npm ให้เพิ่ม behavior เดียว, ทดสอบ request/build path ที่ match ก่อน แล้วจึงเพิ่ม
hooks ที่กว้างขึ้นหรือ native capability

## ขั้นตอนถัดไป (Next Steps)

- **[11-configuration.md](./11-configuration.md)** — Plugin config in detail
- **[12-cli-commands.md](./12-cli-commands.md)** — `ruvyxa plugin create` command
- **[15-official-packages.md](./15-official-packages.md)** — Official packages with plugins
- **[16-error-handling.md](./16-error-handling.md)** — Plugin error codes
