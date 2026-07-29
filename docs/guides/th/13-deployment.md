# Deploy แอป Ruvyxa สู่ Production

Ruvyxa มีระบบ adapter ที่ทำให้ deploy ไปยัง platform ต่าง ๆ ได้ง่าย — Vercel, Netlify, Cloudflare,
Node.js, Bun, Static hosting, Docker และอื่น ๆ อีกมากมาย รวม 10 adapters รองรับตั้งแต่ serverless
จนถึง VPS

---

## Build Output Structure

เมื่อรัน `ruvyxa build` ผลลัพธ์ถูกเขียนไปยังโฟลเดอร์ `.ruvyxa/`: โฟลเดอร์นี้คือ artifact
ที่สมบูรณ์สำหรับ production — ประกอบด้วยทุกอย่างตั้งแต่ server bundle, client chunks, prerendered
HTML, assets, cache, และ metadata

```
.ruvyxa/
├── server/                    # Server bundle — entry point + routes
│   ├── app/                   # Routes + components (server)
│   │   ├── layout.js          # Root layout
│   │   ├── page.js            # Index page
│   │   ├── blog/
│   │   │   ├── page.js        # /blog
│   │   │   └── [slug]/
│   │   │       └── page.js    # /blog/:slug
│   │   ├── api/
│   │   │   └── users/
│   │   │       └── route.js   # /api/users
│   │   └── (group)/
│   │       └── page.js        # Route group
│   ├── components/            # Shared server components
│   │   ├── header.js
│   │   ├── footer.js
│   │   └── layout-wrapper.js
│   ├── server/                # Server-only modules
│   │   ├── db.js              # Database client
│   │   ├── auth.js            # Auth logic
│   │   └── utils.js
│   ├── actions/               # Server actions
│   │   ├── user-actions.js
│   │   └── post-actions.js
│   ├── entry.js               # Server entry point
│   ├── middleware.js           # Edge/Server middleware
│   └── routes-manifest.json   # Route table สำหรับ router
│
├── client/                    # Client bundle (เบราว์เซอร์)
│   ├── chunks/                # Split chunks — โหลดตามต้องการ
│   │   ├── main.abc123.js     # Main entry chunk
│   │   ├── vendor.def456.js   # Vendor (React, etc.)
│   │   ├── pages/             # Page-specific chunks
│   │   │   ├── index.ghi789.js
│   │   │   ├── about.jkl012.js
│   │   │   └── blog.mno345.js
│   │   ├── components/        # Lazy-loaded component chunks
│   │   └── styles/            # Extracted CSS chunks
│   │       └── main.pqr678.css
│   ├── pages/                 # Page-specific bundles (SSR)
│   ├── runtime/               # Ruvyxa runtime — hydration, router
│   │   ├── ruvyxa-runtime.js
│   │   ├── router.js
│   │   └── hmr-client.js      # (dev only)
│   ├── assets/                # Client-side assets
│   │   └── images/            # Small inlined images
│   ├── entry.js               # Client entry point
│   └── entry.css              # Global CSS
│
├── prerender/                 # HTML ที่ prerender แล้ว (SSG)
│   ├── index.html             # / — หน้าแรก
│   ├── about.html             # /about
│   ├── blog/
│   │   ├── index.html         # /blog
│   │   └── hello-world.html   # /blog/hello-world
│   ├── 404.html               # Fallback 404
│   └── 500.html               # Fallback 500 error (ถ้ามี)
│
├── assets/                    # Static assets — รูป, ฟอนต์, ไฟล์
│   ├── images/                # รูปที่ optimize แล้ว
│   │   ├── hero.webp
│   │   ├── logo-192.png
│   │   ├── logo-512.png
│   │   └── favicon.ico
│   ├── fonts/                 # Font files ที่ self-host
│   │   ├── inter-latin.woff2
│   │   └── noto-sans-thai.woff2
│   ├── robots.txt             # SEO
│   ├── sitemap.xml            # SEO
│   └── search-index.json      # Search index (ถ้าใช้ searchIndex plugin)
│
├── cache/                     # Cache artifacts — reuse ใน rebuild
│   ├── swc/                   # SWC transform cache
│   ├── images/                # Image optimization cache
│   ├── route-graph.json       # Route dependency graph
│   └── manifest-cache.json    # Cached manifest
│
├── build.json                 # Metadata สำหรับ production — schema ด้านล่าง
├── manifest.json              # Chunk manifest (ถ้าเปิด)
├── sourcemaps/                # Source maps (ถ้าเปิด)
│   ├── server/
│   └── client/
└── trace.json                 # Build trace (ถ้าใช้ `ruvyxa trace`)
```

### `build.json` — Schema เต็ม

ไฟล์ `build.json` คือ metadata header ที่ production server ใช้อ่านข้อมูล build ปัจจุบัน — version,
routes, assets, timing, config snapshot

```typescript
interface BuildJson {
  /** Ruvyxa version ที่ใช้ build */
  version: string
  /** Timestamp build */
  builtAt: string // ISO 8601
  /** Platform target */
  target: 'server' | 'serverless' | 'edge' | 'static'
  /** Adapter name */
  adapter:
    | 'vercel'
    | 'netlify'
    | 'cloudflare'
    | 'node'
    | 'bun'
    | 'static'
    | 'railway'
    | 'render'
    | 'firebase'
    | 'aws'
  /** Runtime environment */
  runtime: 'node' | 'bun' | 'workerd' | 'deno'
  /** จำนวน route ทั้งหมด */
  routes: number
  /** หน้าที่ prerender แล้ว */
  prerenderedPages: number
  /** ข้อมูล assets */
  assets: {
    total: number // จำนวนไฟล์ทั้งหมด
    size: number // ขนาดรวม (bytes)
    images: number // จำนวนรูป
    fonts: number // จำนวนฟอนต์
    documents: number // robots.txt, sitemap, etc.
  }
  /** ข้อมูล client bundle */
  client: {
    entry: string // Path ไปยัง entry
    chunks: string[] // List ของ chunk paths
    totalSize: number // ขนาดรวม client JS (bytes)
    initialSize: number // ขนาด initial load (bytes)
    cssSize: number // ขนาด CSS (bytes)
  }
  /** ข้อมูล server bundle */
  server: {
    entry: string // Server entry path
    size: number // ขนาด (bytes)
    middleware: boolean // มี middleware ไหม
  }
  /** ข้อมูล prerender */
  prerender: {
    enabled: boolean
    pages: number
    totalSize: number // ขนาดรวม HTML (bytes)
  }
  /** Build duration */
  duration: {
    total: number // milliseconds
    transform: number
    bundle: number
    optimize: number
    adapter: number
  }
  /** Config snapshot (เฉพาะฟิลด์ที่ส่งผลต่อ build) */
  config: {
    appDir: string
    output: string
    target: string
    adapter: string
    runtime: string
    sourceMaps: boolean
    minify: boolean
    prerender: boolean
  }
}
```

ตัวอย่างไฟล์จริง:

```json
{
  "version": "0.1.0",
  "builtAt": "2026-07-29T10:30:00Z",
  "target": "serverless",
  "adapter": "vercel",
  "runtime": "node",
  "routes": 12,
  "prerenderedPages": 5,
  "assets": {
    "total": 48,
    "size": 12567890,
    "images": 18,
    "fonts": 4,
    "documents": 3
  },
  "client": {
    "entry": "client/entry.js",
    "chunks": [
      "client/chunks/main.a1b2c3.js",
      "client/chunks/vendor.d4e5f6.js",
      "client/chunks/pages/index.g7h8i9.js",
      "client/chunks/pages/blog.j0k1l2.js"
    ],
    "totalSize": 567890,
    "initialSize": 123450,
    "cssSize": 45670
  },
  "server": {
    "entry": "server/entry.js",
    "size": 234560,
    "middleware": true
  },
  "prerender": {
    "enabled": true,
    "pages": 5,
    "totalSize": 45670
  },
  "duration": {
    "total": 12340,
    "transform": 3400,
    "bundle": 5600,
    "optimize": 2100,
    "adapter": 1240
  },
  "config": {
    "appDir": "app",
    "output": ".ruvyxa",
    "target": "serverless",
    "adapter": "vercel",
    "runtime": "node",
    "sourceMaps": false,
    "minify": true,
    "prerender": true
  }
}
```

### `manifest.json` — Chunk Manifest

```typescript
interface ManifestJson {
  version: string
  routes: Record<
    string,
    {
      file: string // Server file path
      clientChunks: string[] // Client chunks สำหรับ route นี้
      prerendered: boolean
      strategy: 'ssr' | 'ssg' | 'isr' | 'csr'
      metadata?: {
        title?: string
        description?: string
      }
    }
  >
  shared: {
    server: string[]
    client: string[]
  }
  css: string[] // Global CSS files
}
```

---

## Adapter System

Ruvyxa ใช้ adapter เพื่อปรับ output ให้เข้ากับแต่ละ platform:

```
┌─────────────────┐
│  ruvyxa build   │   — transform, bundle, optimize
└────────┬────────┘
         ▼
┌─────────────────┐
│  Build Output   │   (.ruvyxa/) — output กลาง (generic)
│  (generic)      │
└────────┬────────┘
         ▼  Adapter transform
┌─────────────────┐
│  Platform       │   — artifacts เฉพาะ platform
│  Artifacts      │
└─────────────────┘
```

### การเลือก Adapter

1. **Auto-detect**: Ruvyxa ตรวจสอบ environment variables ของ platform — ถ้าตรวจเจอจะใช้ adapter
   นั้นทันที
2. **Config**: ระบุ `adapter` ใน `ruvyxa.config.ts`
3. **CLI flag**: `--adapter` ใน `ruvyxa build`
4. **Env var**: `RUVYXA_ADAPTER` — มี priority สูงสุด

ลำดับความสำคัญ: CLI flag > `RUVYXA_ADAPTER` > Config > Auto-detect > default (node)

### Algorithm Auto-Detection

```mermaid
flowchart TD
    A[ruvyxa build เริ่ม] --> B{RUVYXA_ADAPTER env?}
    B -->|มี| C[ใช้ค่านั้นทันที]
    B -->|ไม่มี| D{--adapter flag?}
    D -->|มี| E[ใช้ flag]
    D -->|ไม่มี| F{config adapter?}
    F -->|มี| G[ใช้จาก config]
    F -->|ไม่มี| H{ตรวจ Platform env vars}
    H -->|VERCEL env| I[vercel]
    H -->|NETLIFY env| J[netlify]
    H -->|CF_PAGES env| K[cloudflare]
    H -->|RAILWAY_PROJECT_ID| L[railway]
    H -->|RENDER_EXTERNAL_URL| M[render]
    H -->|AWS_LAMBDA_RUNTIME_API| N[aws]
    H -->|FIREBASE_CONFIG env| O[firebase]
    H -->|BUN_RUNTIME| P[bun]
    H -->|ไม่มีเลย| Q{output type?}
    Q -->|static output| R[static]
    Q -->|server output| S[node (default)]
```

Environment variables ที่ใช้ auto-detect:

| Platform           | Env Var                  | ค่า               |
| ------------------ | ------------------------ | ----------------- |
| Vercel             | `VERCEL`                 | `1`               |
| Netlify            | `NETLIFY`                | `true`            |
| Cloudflare Pages   | `CF_PAGES`               | `1`               |
| Cloudflare Workers | `CF_WORKER`              | `true`            |
| Railway            | `RAILWAY_PROJECT_ID`     | project ID string |
| Render             | `RENDER_EXTERNAL_URL`    | URL string        |
| AWS Lambda         | `AWS_LAMBDA_RUNTIME_API` | API endpoint      |
| Firebase           | `FIREBASE_CONFIG`        | JSON string       |
| Bun                | `BUN_RUNTIME`            | `1`               |

### ตาราง Adapter เปรียบเทียบ

| Adapter      | Runtime | ชนิด Deployment               | SSR | SSG | ISR | API | Middleware | Image Opt |
| ------------ | ------- | ----------------------------- | --- | --- | --- | --- | ---------- | --------- |
| `vercel`     | Node.js | Serverless (Edge + Functions) | ✓   | ✓   | ✓   | ✓   | ✓          | ✓         |
| `netlify`    | Node.js | Serverless Functions          | ✓   | ✓   | ✓   | ✓   | ✓          | ✓         |
| `cloudflare` | workerd | Edge Workers/Pages            | ✓   | ✓   | ✗   | ✓   | ✓          | ✓         |
| `node`       | Node.js | Long-running server           | ✓   | ✓   | ✗   | ✓   | ✓          | ✓         |
| `bun`        | Bun     | Long-running server           | ✓   | ✓   | ✗   | ✓   | ✓          | ✓         |
| `static`     | —       | Static files                  | ✗   | ✓   | ✗   | ✗   | ✗          | ✓         |
| `railway`    | Node.js | Long-running server           | ✓   | ✓   | ✗   | ✓   | ✓          | ✓         |
| `render`     | Node.js | Long-running server           | ✓   | ✓   | ✗   | ✓   | ✓          | ✓         |
| `firebase`   | Node.js | Cloud Functions               | ✓   | ✓   | ✗   | ✓   | ✓          | ✓         |
| `aws`        | Node.js | Lambda + S3 + CloudFront      | ✓   | ✓   | ✓   | ✓   | ✓          | ✓         |

---

### ตารางอ้างอิง Adapter ฉบับสมบูรณ์

ตารางรวมทุก adapter พร้อมฟังก์ชัน เป้าหมาย สิ่งที่รองรับ ตัวเลือก และแพลตฟอร์ม:

| Package                      | ฟังก์ชัน              | เป้าหมาย   | รองรับ                       | ตัวเลือก                                                                                     |
| ---------------------------- | --------------------- | ---------- | ---------------------------- | -------------------------------------------------------------------------------------------- |
| `@ruvyxa/adapter-vercel`     | `vercelAdapter()`     | serverless | SSR, SSG, ISR, CSR, PPR, API | `functionsDir`, `projectOutput: true`, `runtime: 'nodejs20.x'`, `maxDuration: 10`, `regions` |
| `@ruvyxa/adapter-node`       | `nodeAdapter()`       | node       | SSR, SSG, ISR, CSR, PPR, API | `entry`                                                                                      |
| `@ruvyxa/adapter-static`     | `staticAdapter()`     | static     | SSG, CSR                     | `outputDir: 'static'`                                                                        |
| `@ruvyxa/adapter-aws`        | `awsAdapter()`        | serverless | SSR, SSG, ISR, CSR, PPR, API | `runtime: 'nodejs22.x'`, `projectOutput: true`                                               |
| `@ruvyxa/adapter-bun`        | `bunAdapter()`        | node       | SSR, SSG, ISR, CSR, PPR, API | `entry`                                                                                      |
| `@ruvyxa/adapter-cloudflare` | `cloudflareAdapter()` | edge       | SSR, SSG, CSR, API           | `workerEntry`, `projectConfig: false`, `compatibilityDate: '2025-09-01'`                     |
| `@ruvyxa/adapter-firebase`   | `firebaseAdapter()`   | serverless | SSR, SSG, ISR, CSR, PPR, API | `functionName: 'ruvyxaServer'`, `region: 'us-central1'`, `projectConfig: true`               |
| `@ruvyxa/adapter-netlify`    | `netlifyAdapter()`    | serverless | SSR, SSG, ISR, CSR, PPR, API | `functionsDir`, `projectConfig: false`, `frameworksApi: true`                                |
| `@ruvyxa/adapter-railway`    | `railwayAdapter()`    | node       | SSR, SSG, ISR, CSR, PPR, API | `projectConfig: true`                                                                        |
| `@ruvyxa/adapter-render`     | `renderAdapter()`     | node       | SSR, SSG, ISR, CSR, PPR, API | `serviceName: 'ruvyxa-app'`, `projectConfig: true`                                           |

#### รายละเอียดแต่ละ Adapter

**@ruvyxa/adapter-vercel**

- **Output**: `.vercel/output/` — รูปแบบ Build Output API v3 ฟังก์ชัน serverless ใน
  `functions/__ruvyxa.func/` ไฟล์ static อยู่ใต้ `static/`
- **Auto-detection**: มี `vercel.json` ใน project root หรือตั้งค่า `VERCEL` env var
- **Runtime dependency**: ไม่มี ใช้ `nodejs20.x` ของแพลตฟอร์ม (ปรับได้)
- **Error codes**: `RUV1700` (ไม่ได้ติดตั้ง), `RUV1704` (route ไม่เข้ากัน)
- **หมายเหตุ**: ISR ใช้ `os.tmpdir()` เป็น cache — อยู่ตราบเท่าที่ function instance ทำงาน รองรับ
  preview deployments ผ่าน Git integration

**@ruvyxa/adapter-node**

- **Output**: `dist/` — HTTP server แบบ standalone มี `server.js`, route modules, client assets,
  prerendered HTML และ `package.json` สำหรับติดตั้ง dependencies ใน production
- **Auto-detection**: ไม่มี (fallback เริ่มต้น)
- **Runtime dependency**: มี — ต้องใช้ `ruvyxa` runtime package ใน `package.json`
- **Error codes**: `RUV1705` (ไม่พบ entry)
- **หมายเหตุ**: รองรับ WebSocket ผ่าน realtime plugin และ cluster mode ผ่าน PM2 หรือ `node:cluster`

**@ruvyxa/adapter-static**

- **Output**: `dist/` — ไฟล์ static HTML, assets, `_redirects`, `404.html`, `sitemap.xml`,
  `robots.txt`
- **Auto-detection**: ไม่มี ต้องระบุเอง
- **Runtime dependency**: ไม่มี — เป็น static files ล้วน
- **Error codes**: `RUV1706` (SSR route ใน static build)
- **หมายเหตุ**: รวมเฉพาะ SSG และ CSR routes เท่านั้น Routes ที่ใช้ SSR, ISR, หรือ PPR จะถูกตัดออกตอน
  build พร้อม warning

**@ruvyxa/adapter-aws**

- **Output**: `.amplify/` — `amplify.yml` build spec, `dist/` สำหรับ static assets,
  `functions/ruvyxa-server/` สำหรับ Lambda bundle (มี `index.mjs`, `package.json`, `node_modules/`)
- **Auto-detection**: `AWS_EXECUTION_ENV` env var
- **Runtime dependency**: Dependencies ถูก bundle ไปใน Lambda zip
- **Error codes**: `RUV1700`, `RUV1704`
- **หมายเหตุ**: รองรับ Lambda@Edge สำหรับ SSR ISR อยู่ในแผนพัฒนา — ใช้ Lambda ร่วมกับ CloudFront
  cache invalidation

**@ruvyxa/adapter-bun**

- **Output**: `dist/` — Bun server ไฟล์เดียว (`server.js`) และ client assets ไม่ต้องมี
  `package.json` — Bun อ่าน `bun.lock` จาก project root
- **Auto-detection**: ไม่มี ต้องระบุเอง
- **Runtime dependency**: ไม่มี — ใช้ transpiler ในตัวของ Bun, SQLite, และ `fetch` ที่เร็วขึ้น
- **Error codes**: `RUV1700`, `RUV1704`
- **หมายเหตุ**: Performance ดีกว่า Node.js 2-4 เท่า Output เป็น server ไฟล์เดียวที่พร้อมรัน

**@ruvyxa/adapter-cloudflare**

- **Output**: `.cloudflare/` — Pages Functions handler (`__ruvyxa.js`), `_routes.json`, `_headers`,
  `_redirects`, static assets, SSG fallback HTML
- **Auto-detection**: มี `wrangler.toml` ใน project root หรือ `CF_PAGES` env var
- **Runtime dependency**: ไม่มี — ใช้ Cloudflare Workers runtime (workerd)
- **Error codes**: `RUV2210` (ISR/PPR ถูก reject — ต้องมี KV/Durable Objects), `RUV1704`
- **หมายเหตุ**: รองรับเฉพาะ SSR, SSG, CSR, และ API เท่านั้น ISR และ PPR ถูก reject ตอน build Worker
  มี RAM limit 128MB

**@ruvyxa/adapter-firebase**

- **Output**: `.firebase/` — `firebase.json` (hosting config), `.firebaserc` (project alias),
  `dist/` (static assets + SSG), `functions/` (Cloud Function entry, `package.json`,
  `node_modules/`)
- **Auto-detection**: มี `firebase.json` ใน project root หรือ `FIREBASE_CONFIG` env var
- **Runtime dependency**: Dependencies ถูก bundle ไปกับ Cloud Functions
- **Error codes**: `RUV1700`, `RUV1704`
- **หมายเหตุ**: ใช้ Firebase Hosting rewrites เพื่อส่ง request ทั้งหมดไปยัง Cloud Function ISR/PPR
  ไม่รองรับ (ไม่มี writable filesystem ใน Cloud Functions)

**@ruvyxa/adapter-netlify**

- **Output**: `.netlify/` — `deploy.config`, `dist/` (publish directory), `functions/__ruvyxa/`
  (serverless handler + route modules + prerender) และสร้าง `netlify.toml` อัตโนมัติ
- **Auto-detection**: มี `netlify.toml` ใน project root หรือ `NETLIFY` env var
- **Runtime dependency**: ไม่มี — แพลตฟอร์มมี Node.js runtime ให้
- **Error codes**: `RUV1700`, `RUV1704`
- **หมายเหตุ**: ISR และ PPR ไม่รองรับ — Netlify ไม่มี writable filesystem เปิด Edge Functions
  ได้ด้วย `edgeFunctions: true`

**@ruvyxa/adapter-railway**

- **Output**: `dist/` — Node server (`server.js`) และ `railway.json` สำหรับ platform configuration
  รวม build command, start command, และ health check path
- **Auto-detection**: มี `railway.json` ใน project root หรือ `RAILWAY_ENVIRONMENT` env var
- **Runtime dependency**: ใช้ `ruvyxa` runtime package ใน `package.json`
- **Error codes**: `RUV1700`, `RUV1704`
- **หมายเหตุ**: สร้างจาก Node adapter output Health check path default ที่ `/api/health` ใช้
  Nixpacks builder

**@ruvyxa/adapter-render**

- **Output**: `dist/` — Node server (`server.js`) และ `render.yaml` Blueprint พร้อม service name,
  plan, region, health check, และ environment variables
- **Auto-detection**: มี `render.yaml` ใน project root หรือ `RENDER` env var
- **Runtime dependency**: ใช้ `ruvyxa` runtime package ใน `package.json`
- **Error codes**: `RUV1700`, `RUV1704`
- **หมายเหตุ**: สร้างจาก Node adapter output รองรับแผน: starter, professional, advanced ภูมิภาค:
  oregon, frankfurt, singapore, virginia

---

## Adapter แต่ละตัว — ละเอียด

### 1. Vercel Adapter

Adapter ที่นิยมมากที่สุด — รองรับ Edge Functions, Serverless Functions, ISR, Image Optimization

```bash
# Auto-detect — ถ้ามี VERCEL env จะใช้ vercel adapter อัตโนมัติ
npm run build

# หรือระบุเอง
RUVYXA_ADAPTER=vercel npm run build
npm run build -- --adapter vercel

# Deploy
npx vercel --prod
# หรือใช้ Git integration — push ไป GitHub → Vercel auto-deploy
```

**Output structure**:

```
.ruvyxa/
├── .vercel/
│   ├── output/
│   │   ├── config.json           # Vercel build output config
│   │   ├── functions/            # Serverless functions
│   │   │   ├── index.func/
│   │   │   │   ├── .vc-config.json
│   │   │   │   └── index.js
│   │   │   ├── blog.[slug].func/
│   │   │   └── api.users.func/
│   │   ├── static/               # Static assets
│   │   │   ├── index.html
│   │   │   └── assets/
│   │   └── edge/                 # Edge functions (ถ้ามี)
│   │       └── middleware.js
│   └── project.json
├── .vercelignore
└── build.json
```

**Vercel-specific env vars**:

| ตัวแปร          | คำอธิบาย                               |
| --------------- | -------------------------------------- |
| `VERCEL`        | auto-detect flag                       |
| `VERCEL_ENV`    | `production`, `preview`, `development` |
| `VERCEL_URL`    | Deployment URL                         |
| `VERCEL_REGION` | Region ที่ function ทำงาน              |

**Config ใน ruvyxa.config.ts**:

```ts
// ruvyxa.config.ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  adapter: 'vercel',
  vercel: {
    regions: ['iad1', 'hnd1'], // Serverless function regions
    edge: true, // เปิด Edge Functions
    isr: { revalidate: 60 }, // ISR default
    imageOptimization: true, // Opt-in image optimization
    maxDuration: 30, // Max function duration (s)
  },
})
```

**ข้อจำกัด Vercel**:

- Serverless function: max 30s (Hobby), 60s (Pro), 900s (Enterprise)
- Edge function: max 30s CPU, 50ms startup
- Bundle size: max 50MB (รวม node_modules)
- Response size: max 4.5MB (Edge), Unlimited (Serverless)

---

### 2. Netlify Adapter

```bash
RUVYXA_ADAPTER=netlify npm run build

# Deploy
npx netlify deploy --prod
# หรือใช้ Git integration
```

**Output**:

```
.ruvyxa/
├── netlify.toml               # สร้างอัตโนมัติ — build config
├── .netlify/
│   ├── functions/             # Netlify Functions
│   │   ├── render/
│   │   │   ├── index.js
│   │   │   └── package.json
│   │   └── ...
│   └── edge-functions/        # Edge Functions (ถ้าเปิด)
│       └── middleware.js
├── dist/                      # Static output
│   ├── index.html
│   └── assets/
└── build.json
```

**netlify.toml** ที่สร้างอัตโนมัติ:

```toml
[build]
  command = "npm run build"
  publish = ".ruvyxa/dist"
  functions = ".ruvyxa/.netlify/functions"

[[edge_functions]]
  function = "middleware"
  path = "/*"
```

**Config**:

```ts
export default defineConfig({
  adapter: 'netlify',
  netlify: {
    edgeFunctions: true,
    imageOptimization: true,
  },
})
```

**ข้อจำกัด Netlify**:

- Function: max 26s (Hobby), 60s (Pro)
- Edge function: max 50ms CPU
- Deploy: max 1000 functions
- Bandwidth: 100GB/month (Hobby)

---

### 3. Cloudflare Adapter

รองรับทั้ง Cloudflare Pages และ Cloudflare Workers

```bash
RUVYXA_ADAPTER=cloudflare npm run build
```

**Output (Pages)**:

```
.ruvyxa/
├── _worker.js                 # Cloudflare Worker — entry point
├── _routes.json               # Route config — static vs worker
├── _headers                   # Custom headers
├── _redirects                 # Custom redirects
├── public/                    # Static files
│   ├── index.html
│   ├── assets/
│   └── 404.html
└── build.json
```

**\_routes.json**:

```json
{
  "version": 1,
  "include": ["/api/*", "/blog/*"],
  "exclude": ["/assets/*", "/favicon.ico"]
}
```

**Cloudflare Dashboard Settings**:

```
Build command: npx ruvyxa build --adapter cloudflare
Build output:  .ruvyxa
Root directory: /
```

**Config**:

```ts
export default defineConfig({
  adapter: 'cloudflare',
  cloudflare: {
    type: 'pages', // 'pages' | 'workers'
    workerName: 'my-app', // (workers only)
    route: 'example.com/*', // (workers only)
    compatibilityDate: '2026-01-01', // CF compatibility date
    compatibilityFlags: ['nodejs_compat'],
  },
})
```

**ข้อจำกัด Cloudflare**:

- Worker: 128MB RAM, 30s CPU (50ms free tier)
- Worker size: 1MB uncompressed (5MB for Paid)
- Pages: 500 builds/month (free), 5000 (paid)
- KV: 1000 reads/s, 1000 writes/s

---

### 4. Node.js Adapter (VPS, Railway, Render)

สำหรับ long-running Node.js server — เหมาะกับ VPS, Railway, Render, Docker

```bash
RUVYXA_ADAPTER=node npm run build
```

**Output**:

```
.ruvyxa/
├── server.js                  # Entry point — app.listen()
├── server/
│   ├── app/
│   ├── components/
│   └── entry.js
├── client/
│   ├── chunks/
│   └── runtime/
├── prerender/
├── assets/
├── build.json
└── package.json               # Dependencies — ใช้กับ npm install ตอน deploy
```

`server.js`:

```js
// .ruvyxa/server.js — entry ที่ import โดยตรง
const { createServer } = require('./server/entry')
const port = process.env.PORT || 3000

createServer().then((server) => {
  server.listen(port, () => {
    console.log(`Ruvyxa running on http://localhost:${port}`)
  })
})
```

**Config**:

```ts
export default defineConfig({
  adapter: 'node',
  node: {
    port: 3000,
    host: '0.0.0.0',
    cluster: true, // เปิด multi-core
    clusterWorkers: 4, // จำนวน worker processes
    gracefulShutdown: 10000, // ms รอ pending requests
  },
})
```

**Start script**:

```json
// package.json
{
  "scripts": {
    "start": "node .ruvyxa/server.js"
  }
}
```

---

### 5. Bun Adapter

สำหรับ Bun runtime — performance สูง

```bash
RUVYXA_ADAPTER=bun npm run build -- --runtime bun
```

**Output**:

```
.ruvyxa/
├── server.bun.js              # Bun entry point
├── server/
├── client/
├── prerender/
├── assets/
├── build.json
└── bun.lock                   # Bun lockfile
```

**Config**:

```ts
export default defineConfig({
  adapter: 'bun',
  runtime: 'bun',
  bun: {
    port: 3000,
    host: '0.0.0.0',
  },
})
```

**Start**:

```bash
bun run .ruvyxa/server.bun.js
```

**ข้อดีของ Bun**:

- Hot reload ที่รวดเร็ว
- Native JavaScript/TypeScript runtime (ไม่ต้อง transpile)
- Node.js API compatibility ~90%
- Performance สูงกว่า Node.js 2-4x

---

### 6. Static Adapter

Export เป็น static HTML — ไม่ต้องใช้ server

```bash
RUVYXA_ADAPTER=static npm run build
```

**Output**:

```
.ruvyxa/
├── index.html                 # /
├── about.html                 # /about
├── blog/
│   ├── index.html             # /blog
│   └── hello-world.html       # /blog/hello-world
├── assets/
│   ├── images/
│   ├── fonts/
│   └── robots.txt
├── 404.html                   # 404 fallback
├── sitemap.xml                # SEO
└── build.json
```

**Config**:

```ts
export default defineConfig({
  adapter: 'static',
  static: {
    trailingSlash: true, // /about → /about/index.html
    fallback: '404.html', // 404 fallback
    spaFallback: false, // SPA fallback (ส่ง index.html)
    cleanUrls: true, // ไม่มี .html → /about
  },
})
```

**ข้อควรระวัง Static**:

- Dynamic routes ต้องใช้ `generateStaticParams` — Ruvyxa ต้องรู้ params ทั้งหมดตอน build
- API routes จะไม่ทำงาน — ต้องใช้ external API
- Server actions จะไม่ทำงาน — ต้องใช้ client mutations

**Deploy destinations**:

- GitHub Pages
- Netlify (static mode)
- Cloudflare Pages (static mode)
- AWS S3 + CloudFront
- Nginx / Apache
- Surge.sh
- Any static hosting

---

### 7. Railway Adapter

```bash
RUVYXA_ADAPTER=railway npm run build
```

**railway.json** (Ruvyxa สร้างให้):

```json
{
  "build": {
    "builder": "nixpacks",
    "buildCommand": "npm run build"
  },
  "deploy": {
    "startCommand": "npm run start",
    "healthcheckPath": "/api/health",
    "restartPolicyType": "ON_FAILURE",
    "restartPolicyMaxRetries": 3
  }
}
```

**Config**:

```ts
export default defineConfig({
  adapter: 'railway',
  railway: {
    healthPath: '/api/health',
  },
})
```

Railway env vars อัตโนมัติ:

- `PORT` — ตั้งค่าโดย Railway
- `RAILWAY_PROJECT_ID`
- `RAILWAY_SERVICE_NAME`

---

### 8. Render Adapter

```bash
RUVYXA_ADAPTER=render npm run build
```

**render.yaml** (Ruvyxa สร้างให้):

```yaml
services:
  - type: web
    name: my-app
    env: node
    plan: starter
    buildCommand: npm run build
    startCommand: npm run start
    healthCheckPath: /api/health
    autoDeploy: true
    envVars:
      - key: RUVYXA_PUBLIC_API_URL
        sync: false # set manually
```

**Config**:

```ts
export default defineConfig({
  adapter: 'render',
  render: {
    healthPath: '/api/health',
    autoDeploy: true,
  },
})
```

---

### 9. Firebase Adapter

```bash
RUVYXA_ADAPTER=firebase npm run build
```

**Output**:

```
.ruvyxa/
├── firebase.json              # Firebase config
├── .firebaserc                # Firebase project
├── functions/                 # Cloud Functions
│   ├── index.js
│   ├── package.json
│   └── node_modules/
├── public/                    # Static hosting
└── build.json
```

**Config**:

```ts
export default defineConfig({
  adapter: 'firebase',
  firebase: {
    projectId: 'my-project',
    region: 'asia-southeast1',
    memory: '1GB', // '128MB' | '256MB' | '512MB' | '1GB'
    minInstances: 0,
    maxInstances: 10,
  },
})
```

**Deploy**:

```bash
npx firebase deploy --only hosting,functions
```

---

### 10. AWS Adapter

```bash
RUVYXA_ADAPTER=aws npm run build
```

**Output**:

```
.ruvyxa/
├── cloudformation.yaml        # CloudFormation template
├── lambda/                    # Lambda functions
│   ├── server.zip
│   └── middleware.zip
├── s3/                        # S3 static assets
│   ├── index.html
│   ├── assets/
│   └── 404.html
└── build.json
```

**Config**:

```ts
export default defineConfig({
  adapter: 'aws',
  aws: {
    region: 'ap-southeast-1',
    bucket: 'my-app-static',
    functionName: 'my-app-server',
    cloudfront: true, // CloudFront distribution
    certificateArn: 'arn:aws:...', // Custom domain cert
    memorySize: 1024, // MB
    timeout: 30, // seconds
  },
})
```

**Deploy**:

```bash
# ใช้ AWS CDK หรือ CloudFormation
npx cdk deploy

# หรือ deploy ด้วย aws cli
aws s3 sync .ruvyxa/s3/ s3://my-app-static/
aws lambda update-function-code --function-name my-app-server --zip-file fileb://.ruvyxa/lambda/server.zip
```

---

## การ Deploy แบบ Staging + Blue-Green

Ruvyxa มีระบบ staging deploy ที่ใช้หลักการ atomic commit — เปลี่ยน version ทีละ step โดยไม่กระทบ
production จนกว่า build ใหม่จะพร้อม

### Blue-Green Deployment Algorithm

```
Staging Area: /.ruvyxa-staging/

1. Build → .ruvyxa-staging/
   - Build ใหม่ใส่ staging directory
   - Production ยังใช้ version เก่าที่ .ruvyxa/

2. Validate → .ruvyxa-staging/
   - ตรวจสอบ build.json
   - ตรวจ manifest integrity
   - ตรวจ route ทั้งหมด match
   - ตรวจ prerendered pages ครบ

3. Health Check → .ruvyxa-staging/
   - Start staging server
   - เรียก health endpoint
   - เรียก sample routes (200?)
   - วัด response time

4. Atomic Swap → .ruvyxa/ ⬅ .ruvyxa-staging/
   - Rename .ruvyxa/ → .ruvyxa-prev/
   - Rename .ruvyxa-staging/ → .ruvyxa/
   - Stop staging server

5. Rollback Ready
   - ถ้า production ล้ม → swap กลับ
   - .ruvyxa-prev/ พร้อม rollback ตลอด 30 นาที
```

### ใช้ CLI

```bash
# Staging deploy
ruvyxa deploy:stage                     # Build + validate staging
ruvyxa deploy:stage --adapter vercel    # ระบุ adapter
ruvyxa deploy:stage --no-health         # ข้าม health check

# Swap to production
ruvyxa deploy:swap                      # Atomic swap

# Rollback
ruvyxa deploy:rollback                  # กลับไป version ก่อนหน้า

# ตรวจสอบสถานะ
ruvyxa deploy:status

# Output:
# ━━━ Deployment Status ━━━━━━━━━━━━━━━━━━━━━━
#   Active:    .ruvyxa/ (built 2026-07-29 10:30)
#   Staging:   .ruvyxa-staging/ (built 2026-07-29 11:00)
#   Previous:  .ruvyxa-prev/ (built 2026-07-29 09:00)
```

### ใช้ Custom Script

```ts
// scripts/deploy.ts
import { stage, swap, rollback, status } from 'ruvyxa/deploy'

async function deploy() {
  // Build + validate
  await stage({ adapter: 'vercel' })

  // Health check
  const health = await checkHealth('http://localhost:3001/api/health')
  if (!health.ok) {
    console.error('Health check failed')
    process.exit(1)
  }

  // Swap ไป production
  await swap()

  // Cleanup
  await cleanupPrevious()
}
```

---

## Production Checklist — ฉบับสมบูรณ์

ก่อน deploy production — ตรวจสอบ 12 ข้อนี้:

### 1. Environment Variables

```bash
# ตรวจว่าทุกตัวแปรถูกตั้งค่าใน platform
# --- Production core ---
RUVYXA_PUBLIC_API_URL=https://api.production.com
RUVYXA_PUBLIC_SITE_URL=https://production.com
DATABASE_URL=postgres://user:pass@prod-host:5432/db
AUTH_SECRET=xxxxxxxxxxxx

# --- Auth providers (ถ้าใช้) ---
AUTH_GOOGLE_ID=xxx.apps.googleusercontent.com
AUTH_GOOGLE_SECRET=GOCSPX-xxxx
AUTH_GITHUB_ID=Ov23li...
AUTH_GITHUB_SECRET=xxxx

# --- Optional ---
RUVYXA_PUBLIC_ANALYTICS_ID=UA-XXXXX
REDIS_URL=redis://...
S3_BUCKET=my-app-uploads
```

### 2. `site.url` ถูกต้อง

```ts
// ruvyxa.config.ts
site: {
  url: 'https://production.com',  // เปลี่ยนเป็น production URL
  name: 'My App',                  // ชื่อเว็บ
  description: 'คำอธิบาย',        // SEO
  defaultLocale: 'th',            // ภาษา default
}
```

### 3. Security Headers

```ts
security: {
  headers: true,                     // เปิด security headers
  sameOrigin: true,                  // จำกัด action เฉพาะ origin เดียวกัน
  contentSecurityPolicy: "default-src 'self'",
  xFrameOptions: 'DENY',
  hsts: true,                        // HTTP Strict Transport Security
  permittedCrossDomainPolicies: 'none',
}
```

### 4. Production Build

```bash
ruvyxa check       # ตรวจสอบความพร้อม —
                   # validation config, routes, boundary, env
ruvyxa doctor      # ตรวจทุกอย่างแบบละเอียด
ruvyxa build       # production build — optimize, minify, bundle
ruvyxa analyze     # ตรวจสอบ bundle size
ruvyxa start       # ทดสอบ local — รัน production server
```

### 5. Error Pages — ครบทุกไฟล์

```tsx
// app/error.tsx — Error boundary ระดับ global
'use client';
export default function ErrorPage({ error, reset }: { ... }) { ... }

// app/not-found.tsx — 404
export default function NotFoundPage() { ... }

// app/loading.tsx — Loading indicator
export default function LoadingPage() { ... }
```

### 6. Performance Baseline

```bash
ruvyxa analyze     # bundle size analysis
ruvyxa bench       # benchmark — SSR latency, throughput, TTFB

# ตรวจว่าค่าอยู่ในเกณฑ์:
# - Initial JS < 200KB
# - TTFB < 200ms (SSR)
# - Lighthouse score > 80
```

### 7. SEO — robots.txt + sitemap.xml

```bash
curl https://production.com/robots.txt
curl https://production.com/sitemap.xml

# ตรวจ:
# - robots.txt ต้อง allow search engines
# - sitemap.xml ต้องมีทุกหน้า
# - canonical URLs ถูกต้อง
```

### 8. Image Optimization

```ts
images: {
  formats: ['webp', 'avif'],    // รองรับ next-gen formats
  sizes: [640, 1080, 1920],     // ขนาดที่ generate
  placeholder: 'blur',          // placeholder ระหว่างโหลด
  remotePatterns: [              // รูปจาก external sources
    { hostname: 'images.unsplash.com' },
  ],
}
```

### 9. Cache Strategy

```ts
cache: {
  ssr: { ttl: 60 },              // SSR cache (วินาที)
  images: { ttl: 86400 },        // Image cache (1 วัน)
  api: { ttl: 0 },               // API — ไม่ cache
}
```

### 10. Database Migrations

```bash
# ตรวจ migration status
npx prisma migrate status
npx prisma migrate deploy    # deploy migrations

# หรือถ้าใช้ raw SQL
psql $DATABASE_URL -f migrations/001_init.sql
```

### 11. Monitoring & Logging

```bash
# ตรวจ logging ถูกตั้งค่า
ruvyxa doctor --logging

# Production logging ควรมี:
# - request/response logs
# - error tracking (Sentry, etc.)
# - performance metrics
# - uptime monitoring
```

### 12. Pre-deploy Health Check

```bash
# ทดสอบก่อน deploy จริง
curl -I https://staging.production.com
curl https://staging.production.com/api/health

# ตรวจ response headers
curl -I https://staging.production.com
# ควรได้: 200, security headers, cache headers
```

---

## การตั้งค่า Environment Variables ใน Production

| Platform   | วิธีตั้งค่า                                            |
| ---------- | ------------------------------------------------------ |
| Vercel     | Dashboard → Project → Settings → Environment Variables |
| Netlify    | Site settings → Build & deploy → Environment variables |
| Cloudflare | Pages → Project → Settings → Environment variables     |
| Railway    | Dashboard → Variables → New Variable                   |
| Render     | Dashboard → Environment → Secret Files                 |
| Docker     | `-e` flags หรือ `--env-file`                           |
| AWS Lambda | AWS Console → Lambda → Environment variables           |
| Firebase   | Firebase Console → Functions → Environment variables   |

### Production-specific Variables File

```bash
# .env.production — ใช้ตอน build เท่านั้น
RUVYXA_PUBLIC_API_URL=https://api.production.com
RUVYXA_PUBLIC_SITE_URL=https://production.com
DATABASE_URL=postgres://prod-user:****@prod-host/db
AUTH_SECRET=production-secret
```

**ข้อสำคัญ**: อย่า commit `.env.production` ลง git — ใช้ platform's secret manager สำหรับ production
secrets

---

## CI/CD Pipeline

### GitHub Actions — ฉบับสมบูรณ์

```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: 'npm'
      - run: npm ci
      - run: npm run check # ตรวจสอบ config + routes
      - run: npm run lint # lint

  build:
    needs: quality
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - run: npm ci
      - run: npm run build
        env:
          RUVYXA_PUBLIC_API_URL: ${{ secrets.RUVYXA_PUBLIC_API_URL }}
          DATABASE_URL: ${{ secrets.DATABASE_URL }}
      - uses: actions/upload-artifact@v4
        with:
          name: build-output
          path: .ruvyxa/

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          name: build-output
          path: .ruvyxa/
      - name: Deploy to Vercel
        run: npx vercel --prod --token ${{ secrets.VERCEL_TOKEN }}
```

### GitLab CI

```yaml
# .gitlab-ci.yml
image: node:22

stages:
  - quality
  - build
  - deploy

quality:
  stage: quality
  script:
    - npm ci
    - npm run check
    - npm run lint

build:
  stage: build
  script:
    - npm ci
    - npm run build
  artifacts:
    paths:
      - .ruvyxa/
  environment:
    name: production

deploy:
  stage: deploy
  script:
    - npx vercel --prod --token $VERCEL_TOKEN
  environment:
    name: production
    url: https://production.com

staging:
  stage: deploy
  script:
    - npx vercel --token $VERCEL_TOKEN
  environment:
    name: staging
    url: https://staging.vercel.app
  only:
    - develop
```

---

## Docker — Production Image

### Multi-stage Dockerfile

```dockerfile
# ===== Stage 1: Build =====
FROM node:22-alpine AS builder
WORKDIR /app

# Install dependencies (cache layer)
COPY package*.json ./
RUN npm ci --production=false

# Copy source + build
COPY . .
RUN npm run build

# Prune dev dependencies
RUN npm prune --production

# ===== Stage 2: Production =====
FROM node:22-alpine AS runner
WORKDIR /app

# Security: non-root user
RUN addgroup --system --gid 1001 ruvyxa && \
    adduser --system --uid 1001 ruvyxa

# Copy only what's needed
COPY --from=builder /app/.ruvyxa ./.ruvyxa
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/node_modules ./node_modules

USER ruvyxa
EXPOSE 3000

ENV NODE_ENV=production
ENV PORT=3000

CMD ["node", ".ruvyxa/server.js"]
```

### docker-compose.yml

```yaml
version: '3.8'
services:
  app:
    build: .
    ports:
      - '3000:3000'
    environment:
      - DATABASE_URL=postgres://user:pass@db:5432/app
      - AUTH_SECRET=${AUTH_SECRET}
      - RUVYXA_PUBLIC_API_URL=https://api.production.com
    env_file:
      - .env.production
    depends_on:
      db:
        condition: service_healthy
    healthcheck:
      test: ['CMD', 'curl', '-f', 'http://localhost:3000/api/health']
      interval: 30s
      timeout: 10s
      retries: 3
    restart: unless-stopped

  db:
    image: postgres:16-alpine
    volumes:
      - pgdata:/var/lib/postgresql/data
    environment:
      POSTGRES_USER: user
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: app
    healthcheck:
      test: ['CMD-SHELL', 'pg_isready -U user']
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  pgdata:
```

### Build & Run

```bash
# Build
docker build -t my-app:latest .
docker build -t my-app:$(node -p "require('./package.json').version") .

# Run
docker run -p 3000:3000 --env-file .env.production my-app

# Docker Compose
docker compose up -d
docker compose logs -f
```

---

## Adapter-Specific Troubleshooting

| ปัญหา                                | สาเหตุ                           | วิธีแก้                                  |
| ------------------------------------ | -------------------------------- | ---------------------------------------- |
| **Vercel**: 504 Gateway Timeout      | Function timeout                 | เพิ่ม `maxDuration` หรือใช้ ISR          |
| **Vercel**: Edge function 500        | Import ที่ไม่รองรับ Edge         | ใช้ `edge: false`, use serverless        |
| **Netlify**: Function cold start ช้า | ไม่มี warm requests              | ใช้ `warmKeepAlive` flag                 |
| **Cloudflare**: 100% CPU             | Loop ใน middleware               | ตรวจ `next()` call                       |
| **Cloudflare**: Script too large     | Worker > 1MB                     | ลด bundle, tree-shake                    |
| **Node**: Memory leak                | Server-side memory               | ใช้ `--max-old-space-size`               |
| **Bun**: Missing API                 | Bun ไม่รองรับ API นั้น           | ใช้ Node adapter แทน                     |
| **Static**: Dynamic route 404        | ไม่ได้ใช้ `generateStaticParams` | เพิ่ม params ที่ต้องการ                  |
| **Railway**: Port conflict           | PORT env ไม่ถูก                  | ตรวจ Railway auto-port                   |
| **Render**: Deploy ล้มเหลว           | Build memory ไม่พอ               | Upgrade plan                             |
| **Firebase**: Function timeout       | ฟังก์ชันทำงานนาน                 | เพิ่ม `timeoutSeconds`                   |
| **AWS**: Lambda cold start           | ใหญ่เกินไป                       | ใช้ `--pre warm` หรือเพิ่ม min instances |
| **AWS**: S3 403                      | Bucket policy ผิด                | เปิด public read สำหรับ static           |
| **Docker**: Container restart        | health check ล้มเหลว             | ตรวจ health endpoint                     |
| **Docker**: Can't connect            | env var ขาด                      | ใช้ `--env-file`                         |

---

## ruuvyxa doctor --adapter

ตรวจสอบว่า adapter และ build output พร้อม deploy:

```bash
ruvyxa doctor --adapter vercel
```

Output:

```
━━━ Adapter Inspection ━━━━━━━━━━━━━━━━━━━━━
  Adapter:   vercel
  Target:    serverless
  Runtime:   node
  Platform:  Vercel

  Supports:
    ✓ SSR
    ✓ SSG
    ✓ ISR
    ✓ API Routes
    ✓ Server Actions
    ✓ Middleware
    ✓ Image Optimization
    ✓ Edge Functions

  Requirements:
    ✓ Node.js 22+
    ✓ No native modules
    ✓ Output < 50MB
    ✓ Functions count < 12

  Warnings:
    ⚠ No .vercelignore — large files may be uploaded

  Recommendations:
    ✓ Enable ISR for blog routes
    ✓ Add error pages
    ✓ Set production env vars
```

---

## Production Performance Benchmarks

```bash
ruvyxa bench
```

| Metric                  | ค่าเป้าหมาย | ค่าที่ควรได้ |
| ----------------------- | ----------- | ------------ |
| SSR Response Time (p50) | < 100ms     | 45ms         |
| SSR Response Time (p99) | < 500ms     | 280ms        |
| TTFB (First Byte)       | < 200ms     | 120ms        |
| Throughput (req/s)      | > 1000      | 2450         |
| Bundle Size (initial)   | < 200KB     | 128KB        |
| Bundle Size (total)     | < 500KB     | 340KB        |
| Asset Size (images)     | < 1MB       | 680KB        |
| Prerender Time/page     | < 1s        | 0.3s         |

---

## การ Migrate Production URL

เมื่อย้าย production URL:

```ts
// ruvyxa.config.ts
// 1. อัปเดต site.url
site: {
  url: 'https://new-domain.com',
  previousUrl: 'https://old-domain.com',  // สำหรับ redirect
}

// 2. ตั้ง redirects plugin
plugins: [
  {
    name: 'redirects',
    options: {
      redirects: [
        { source: '/(.*)', destination: 'https://new-domain.com/$1', permanent: true },
      ],
    },
  },
]
```

ตรวจสอบ:

```bash
curl -I https://old-domain.com/about
# ควรได้: 301 → https://new-domain.com/about
```

---

## ลองทำดู

1. รัน `ruvyxa build` แล้วดูโครงสร้าง `.ruvyxa/` — ทำความเข้าใจแต่ละ directory
2. รัน `ruvyxa build --adapter static` → เปิด `.ruvyxa/index.html` ใน browser
3. ทดสอบ `ruvyxa doctor --adapter vercel` — ดู warning ที่แนะนำ
4. สร้าง Dockerfile และ docker-compose.yml สำหรับ production
5. ตั้งค่า CI/CD ด้วย GitHub Actions — รวม quality + build + deploy
6. Deploy ไปยัง platform ที่เลือก — ใช้ staging ก่อน production
7. ทดสอบ `ruvyxa deploy:stage && ruvyxa deploy:swap`
8. ตรวจ production checklist ทุกข้อก่อน deploy จริง
9. รัน `ruvyxa bench` และ `ruvyxa analyze` หลัง deploy
10. ตั้ง monitoring: uptime check, error tracking, performance alert

---

## สรุป

- Build output อยู่ที่ `.ruvyxa/` — 8 directories พร้อม metadata ใน `build.json`
- 10 adapters — vercel, netlify, cloudflare, node, bun, static, railway, render, firebase, aws
- Auto-detect จาก platform environment variables — 8 env vars ที่รู้จัก
- Adapter auto-detection algorithm — 6 ขั้นตอน จาก env → config → CLI → fallback
- Staging deploy system — blue-green, atomic swap, rollback
- Production checklist — 12 ข้อจาก env vars ถึง monitoring
- Docker multi-stage build — production image ~100MB
- CI/CD พร้อม GitHub Actions และ GitLab CI
- 12 adapter-specific troubleshooting entries
- Performance benchmarks — TTFB < 200ms, throughput > 1000 req/s
