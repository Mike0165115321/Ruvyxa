# เริ่มต้นใช้งาน Ruvyxa

Ruvyxa คือ React full-stack framework สำหรับ production พัฒนาให้ dev experience ดีที่สุด รองรับ
marketing site, SaaS dashboard, blog, API backend — ใช้เครื่องมือชุดเดียว ควบคุมทุกอย่างได้
ไม่ต้องต่อ framework หลายตัว

คู่มือนี้ให้คุณตั้งแต่ศูนย์ถึงแอปที่รันได้จริง พร้อมรายละเอียดทุก API และทุกการตั้งค่า

---

## ความต้องการระบบ

| สิ่งที่ต้องมี  | เวอร์ชันขั้นต่ำ       | หมายเหตุ                      |
| -------------- | --------------------- | ----------------------------- |
| Node.js        | 22.x ขึ้นไป           | ใช้ `node --run` mode ข้างใน  |
| npm            | 10+                   | มากับ Node.js                 |
| pnpm           | 9+                    | ไม่จำเป็น แต่แนะนำ            |
| Yarn           | 4+                    | ไม่จำเป็น                     |
| Bun            | 1.2+                  | ไม่จำเป็น ใช้เป็น runtime ได้ |
| ระบบปฏิบัติการ | Windows, macOS, Linux | Windows ใช้ PowerShell 7+     |

เช็คเวอร์ชัน:

```bash
node -v
# ต้อง v22.0.0 ขึ้นไป
```

Ruvyxa รองรับ JavaScript runtime สองตัว: **Node** (ค่าเริ่มต้น) และ **Bun** การเลือกรันไทม์กำหนดโดย
`RUVYXA_RUNTIME` env, `ruvyxa.config.ts` field `runtime`, หรือ `--runtime` CLI flag

```bash
# ใช้ Bun แทน Node
ruvyxa dev --runtime bun
```

### Bun runtime

Bun runtime ถูกเลือกอัตโนมัติเมื่อ:

1. ไม่พบ Node.js ใน PATH
2. `--runtime bun` ถูกส่งผ่าน CLI
3. `RUVYXA_RUNTIME=bun` ใน environment
4. `config.runtime: 'bun'` ใน ruvyxa.config.ts

---

## สร้างโปรเจคแรก

### npm create

```bash
npm create ruvyxa@latest my-app
```

CLI จะแสดง prompt ให้เลือก template แบบ interactive:

```
? Select a starter template (Use arrow keys)
❯ minimal     – โครงสะอาด, มีแค่กระดูก
  blog        – MDX blog พร้อม posts, tags, RSS
  crud        – CRUD เต็มรูปแบบ พร้อม database และ auth
  api-backend – API ล้วนๆ มีแต่ route.ts endpoints
  empty       – มีแค่ config
```

### ตัวเลือก npm create

| ตัวเลือก     | ค่าที่ยอมรับ                                      | ค่าเริ่มต้น | คำอธิบาย                              |
| ------------ | ------------------------------------------------- | ----------- | ------------------------------------- |
| `--template` | `minimal`, `blog`, `crud`, `api-backend`, `empty` | `minimal`   | เลือก template โดยไม่ต้อง interactive |
| `--pm`       | `npm`, `pnpm`, `yarn`, `bun`                      | อัตโนมัติ   | กำหนด package manager                 |
| `--yes`      | —                                                 | —           | ข้าม prompt ทั้งหมด ใช้ค่าเริ่มต้น    |

```bash
# สร้าง blog โดยไม่ต้อง interactive
npm create ruvyxa@latest my-blog -- --template blog

# ใช้ pnpm
npm create ruvyxa@latest my-app -- --pm pnpm

# ข้าม prompt ทั้งหมด
npm create ruvyxa@latest my-app -- --yes
```

package manager ถูกตรวจสอบโดยอัลกอริทึมนี้:

1. ดู `npm_config_user_agent` env (process ที่เรียก)
2. อ่าน `packageManager` field จาก `package.json` ของ parent directory (Corepack)
3. ดู convention files: `pnpm-workspace.yaml`, `.yarnrc.yml`, `bunfig.toml`
4. ดู lockfile ที่มี `.mtime` ล่าสุด: `pnpm-lock.yaml` > `yarn.lock` > `bun.lock` >
   `package-lock.json`
5. fallback เป็น `npm`

### Validation ก่อนสร้าง

create-ruvyxa ตรวจสอบ 8 อย่างก่อน copy template:

1. project name ต้องไม่ว่าง
2. template ต้องมีค่าใน `STARTER_TEMPLATES`
3. directory name ห้ามมี `< > : " | ? * \x00-\x1f`
4. ห้ามเป็น reserved Windows names: `con`, `prn`, `aux`, `nul`, `com1`-`com9`, `lpt1`-`lpt9`
5. ห้ามลงท้ายด้วย `.` หรือ space
6. ความยาวสูงสุด 128 ตัวอักษร
7. ห้ามขึ้นต้นด้วย `.` หรือ `-`
8. target directory ต้อง empty (ถ้ามีอยู่แล้ว)

---

## โครงสร้างโปรเจคทุกรูปแบบ

### minimal

```
my-app/
├── app/
│   ├── layout.tsx        # Root layout — หุ้มทุกหน้า
│   ├── page.tsx          # หน้าแรกที่ /
│   └── globals.css       # CSS หลัก
├── public/
│   └── favicon.ico       # static assets
├── ruvyxa.config.ts      # Framework config
├── tsconfig.json
├── package.json
├── .gitignore
├── AGENTS.md
└── CLAUDE.md
```

### blog

```
my-blog/
├── app/
│   ├── layout.tsx
│   ├── page.tsx          # หน้าแรก (รายการโพสต์)
│   ├── globals.css
│   ├── blog/
│   │   ├── layout.tsx    # Blog layout
│   │   ├── [slug]/
│   │   │   └── page.mdx  # แต่ละโพสต์
│   │   └── page.tsx      # /blog (index)
│   ├── tags/
│   │   └── [tag]/
│   │       └── page.tsx  # /tags/:tag
│   └── rss/
│       └── route.ts      # /rss.xml
├── posts/                 # MDX source files
│   ├── hello-world.md
│   └── ...
├── public/
├── ruvyxa.config.ts
└── package.json
```

### crud

```
my-crud/
├── app/
│   ├── layout.tsx
│   ├── page.tsx
│   ├── globals.css
│   ├── dashboard/
│   │   ├── layout.tsx
│   │   └── page.tsx
│   ├── items/
│   │   ├── page.tsx       # รายการ items
│   │   ├── new/
│   │   │   └── page.tsx   # สร้างใหม่
│   │   └── [id]/
│   │       ├── page.tsx   # ดู/แก้ไข
│   │       └── action.ts  # Server actions
│   └── auth/
│       ├── login/
│       │   └── page.tsx
│       └── action.ts
├── db/
│   └── schema.ts
├── ruvyxa.config.ts
└── package.json
```

### api-backend

```
my-api/
├── app/
│   ├── api/
│   │   ├── users/
│   │   │   ├── route.ts     # GET/POST /api/users
│   │   │   └── [id]/
│   │   │       └── route.ts # GET/PUT/DELETE /api/users/:id
│   │   └── webhooks/
│   │       └── route.ts     # POST /api/webhooks
│   └── health/
│       └── route.ts         # GET /health
├── ruvyxa.config.ts
└── package.json
```

### empty

```
my-empty/
├── ruvyxa.config.ts
├── tsconfig.json
├── package.json
└── .gitignore
```

---

## เนื้อหา .gitignore แบบเต็ม

```gitignore
# Ruvyxa build output
.ruvyxa/

# Dependencies
node_modules/

# Environment files
.env
.env.local
.env.development.local
.env.test.local
.env.production.local

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Debug logs
npm-debug.log*
yarn-debug.log*
yarn-error.log*
pnpm-debug.log*

# TypeScript
*.tsbuildinfo

# Test coverage
coverage/

# Build artifacts
dist/
build/
.cache/
```

---

## ruvyxa.config.ts — ทุกฟิลด์พร้อมค่าเริ่มต้น

```ts
import { config, type RuvyxaConfig } from 'ruvyxa/config'

const settings: RuvyxaConfig = {
  // ─── Core ─────────────────────────────────────────────────────
  appDir: 'app', // relative path ไปยัง app directory
  outDir: '.ruvyxa', // output directory (cache + build)
  runtime: 'node', // 'node' | 'bun' | 'edge' | 'static'

  // ─── Server ───────────────────────────────────────────────────
  server: {
    host: 'localhost', // host ที่ bind
    port: 3000, // port ที่ listen
  },

  // ─── Build ────────────────────────────────────────────────────
  build: {
    minify: true, // minify output
    map: false, // สร้าง source maps
    treeShake: true, // tree-shaking
    split: 'route', // 'single' | 'route' | 'manual'
    workers: 4, // จำนวน build workers
    jsx: 'automatic', // 'classic' | 'automatic'
    target: 'esnext', // 'es2018' | 'es2019' | 'es2020' | 'es2022' | 'esnext'
    manifest: true, // emit route manifest
    warm: true, // precompile ใน dev
    prerenderCache: true, // reuse valid prerender HTML
  },

  // ─── Render ───────────────────────────────────────────────────
  render: {
    strategy: 'ssr', // 'ssr' | 'ssg' | 'isr' | 'csr' | 'ppr'
    revalidate: 60, // default ISR TTL (seconds)
  },

  // ─── Cache ────────────────────────────────────────────────────
  cache: {
    routes: true, // cache route manifest
    css: true, // cache compiled CSS
    // dir: '.cache',               // custom cache directory
  },

  // ─── CSS ──────────────────────────────────────────────────────
  css: {
    entries: [], // global CSS files/directories
  },

  // ─── Image ────────────────────────────────────────────────────
  image: {
    optimize: true, // convert to WebP
    quality: 82, // 1-100
    lossless: false, // lossless WebP
    keepOriginal: true, // keep PNG/JPEG
    variantWidths: [640, 750, 828, 1080, 1200, 1920, 2048, 3840],
    workers: 0, // 0 = CPU count
  },

  // ─── Security ─────────────────────────────────────────────────
  security: {
    actionLimit: 1048576, // 1 MiB
    apiLimit: 10485760, // 10 MiB
    pluginLimit: 33554432, // 32 MiB (max 268435456)
    actionRateLimit: {
      max: 600, // requests
      window: 60, // seconds
    },
    sameOrigin: false,
    fetchMeta: false,
    headers: true,
  },

  // ─── Debug ────────────────────────────────────────────────────
  debug: {
    overlay: true, // error overlay ใน dev
    traces: false, // runtime route traces
  },

  // ─── Site (robots.txt + sitemap) ──────────────────────────────
  site: {
    url: 'https://example.com', // ใช้ RUVYXA_SITE_URL fallback
    sitemap: true, // true | false | SiteSitemapConfig
    robots: true, // true | false | SiteRobotsConfig
  },

  // ─── Middleware ────────────────────────────────────────────────
  middleware: {
    workers: 1, // 1-8 workers
    timeoutMs: 30000, // max 300000
    builtin: {
      cors: {
        // CORS config
        origins: [],
        methods: [],
        headers: [],
        credentials: false,
        maxAge: 0,
      },
      timing: false,
      log: false,
      rate: {
        // rate limiting
        max: 100,
        window: 60,
        key: 'ip',
      },
      headers: {}, // custom response headers
    },
  },

  // ─── Adapter ──────────────────────────────────────────────────
  adapter: undefined, // node | bun | static | vercel | netlify | cloudflare | railway | render | firebase | aws
  adapterOptions: {},
  plugins: [],
}

export default config(settings)
```

### ข้อจำกัดของแต่ละ field

| Field                             | ข้อจำกัด                                  | Error code |
| --------------------------------- | ----------------------------------------- | ---------- |
| `security.actionLimit`            | 1 - 16,777,216 bytes                      | RUV1601    |
| `security.apiLimit`               | 1 - 268,435,456 bytes                     | RUV1601    |
| `security.pluginLimit`            | ≤ 268,435,456 bytes                       | RUV1602    |
| `security.actionRateLimit.max`    | 1 - 10,000                                | RUV1601    |
| `security.actionRateLimit.window` | 1 - 86,400 seconds                        | RUV1601    |
| `build.jsxRuntime`                | `classic` หรือ `automatic` เท่านั้น       | RUV1601    |
| `build.esTarget`                  | `es2018` ถึง `esnext` เท่านั้น            | RUV1601    |
| `build.splitStrategy`             | `single`, `route`, หรือ `manual`          | RUV1601    |
| `middleware.workers`              | 1 - 8                                     | RUV1602    |
| `middleware.timeoutMs`            | 1 - 300,000                               | RUV1602    |
| `security.trustedProxyIps`        | ต้องเป็น valid IP addresses               | RUV1602    |
| `appDir`                          | ต้องเป็น relative path ภายใน project root | RUV1601    |

---

## ruvyxa-env.d.ts — TypeScript declarations

ไฟล์นี้ถูกสร้างโดยอัตโนมัติใน `.ruvyxa/` เพื่อให้ TypeScript รู้จัก env vars และ module declarations
ที่ Ruvyxa สร้าง:

```ts
/// <reference types="ruvyxa/types" />

declare namespace NodeJS {
  interface ProcessEnv {
    RUVYXA_SITE_URL?: string
    RUVYXA_PUBLIC_APP_NAME?: string
    RUVYXA_PUBLIC_APP_DESCRIPTION?: string
    RUVYXA_RUNTIME?: 'node' | 'bun'
    [key: `RUVYXA_PUBLIC_${string}`]: string | undefined
  }
}

declare module '*.md' {
  import type { MDXContent } from 'ruvyxa/mdx'
  const content: MDXContent
  export { metadata } from '*.md'
  export default content
}

declare module '*.mdx' {
  import type { MDXContent } from 'ruvyxa/mdx'
  const content: MDXContent
  export default content
}

declare module '*.css' {
  const content: Record<string, string>
  export default content
}

declare module '*.module.css' {
  const classes: Record<string, string>
  export default classes
}

declare module '*.module.scss' {
  const classes: Record<string, string>
  export default classes
}

declare module '*.module.sass' {
  const classes: Record<string, string>
  export default classes
}
```

---

## package.json — ทุกคำสั่ง

```json
{
  "name": "my-app",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "ruvyxa dev",
    "build": "ruvyxa build",
    "start": "ruvyxa start",
    "check": "ruvyxa check",
    "typecheck": "tsc --noEmit",
    "preview": "ruvyxa preview",
    "routes": "ruvyxa routes",
    "analyze": "ruvyxa analyze",
    "doctor": "ruvyxa doctor",
    "clean": "ruvyxa clean",
    "trace": "ruvyxa trace",
    "bench": "ruvyxa bench",
    "test:parity": "ruvyxa test:parity"
  },
  "dependencies": {
    "@ruvyxa/react": "workspace:*",
    "react": "^19.1.0",
    "react-dom": "^19.1.0",
    "ruvyxa": "workspace:*"
  },
  "devDependencies": {
    "@types/react": "^19.1.0",
    "@types/react-dom": "^19.1.0",
    "typescript": "^5.7.0"
  }
}
```

| คำสั่ง        | CLI จริง             | หน้าที่                                  |
| ------------- | -------------------- | ---------------------------------------- |
| `dev`         | `ruvyxa dev`         | เริ่ม dev server พร้อม HMR               |
| `build`       | `ruvyxa build`       | Build สำหรับ production ไปที่ `.ruvyxa/` |
| `start`       | `ruvyxa start`       | เสิร์ฟ production build                  |
| `typecheck`   | `tsc --noEmit`       | ตรวจ TypeScript โดยไม่สร้างไฟล์          |
| `check`       | `ruvyxa check`       | typecheck + parity + smoke render        |
| `preview`     | `ruvyxa preview`     | Build + start ในคำสั่งเดียว              |
| `routes`      | `ruvyxa routes`      | แสดง route table                         |
| `analyze`     | `ruvyxa analyze`     | รายงานวิเคราะห์ bundle + boundaries      |
| `doctor`      | `ruvyxa doctor`      | วินิจฉัย project setup                   |
| `clean`       | `ruvyxa clean`       | ลบ `.ruvyxa/` และ cache                  |
| `trace`       | `ruvyxa trace`       | ดู route manifest entry                  |
| `bench`       | `ruvyxa bench`       | ทดสอบ route discovery + build            |
| `test:parity` | `ruvyxa test:parity` | เปรียบเทียบ routes dev/prod              |

---

## ทุก CLI flag

### ruvyxa dev / start / preview

| Flag        | Type        | Default                | Description            |
| ----------- | ----------- | ---------------------- | ---------------------- |
| `--root`    | `path`      | `.`                    | Project root directory |
| `--host`    | `string`    | จาก config (localhost) | Host ที่ bind          |
| `--port`    | `number`    | จาก config (3000)      | Port ที่ listen        |
| `--runtime` | `node\|bun` | อัตโนมัติ              | JavaScript runtime     |

### ruvyxa build

| Flag        | Type                  | Default      | Description               |
| ----------- | --------------------- | ------------ | ------------------------- |
| `--root`    | `path`                | `.`          | Project root              |
| `--target`  | `production\|preview` | `production` | Build target              |
| `--adapter` | `string`              | —            | Adapter name หรือ package |
| `--runtime` | `node\|bun`           | อัตโนมัติ    | JavaScript runtime        |

Adapter names ที่รู้จัก: `node`, `bun`, `static`, `vercel`, `netlify`, `cloudflare`, `railway`,
`render`, `firebase`, `aws` หรือ package name เช่น `@scope/ruvyxa-adapter-deno`

### ruvyxa check

| Flag        | Type        | Default   | Description        |
| ----------- | ----------- | --------- | ------------------ |
| `--root`    | `path`      | `.`       | Project root       |
| `--runtime` | `node\|bun` | อัตโนมัติ | JavaScript runtime |

### ruvyxa analyze

| Flag        | Type                       | Default   | Description        |
| ----------- | -------------------------- | --------- | ------------------ |
| `--root`    | `path`                     | `.`       | Project root       |
| `--runtime` | `node\|bun`                | อัตโนมัติ | JavaScript runtime |
| `--format`  | `auto\|human\|json\|sarif` | `auto`    | รูปแบบรายงาน       |
| `--output`  | `path`                     | —         | เขียนผลลัพธ์ไปไฟล์ |

### ruvyxa doctor

| Flag        | Type                  | Default   | Description        |
| ----------- | --------------------- | --------- | ------------------ |
| `--root`    | `path`                | `.`       | Project root       |
| `--target`  | `production\|preview` | —         | Production target  |
| `--adapter` | `string`              | —         | ตรวจสอบ adapter    |
| `--runtime` | `node\|bun`           | อัตโนมัติ | JavaScript runtime |
| `--json`    | —                     | —         | รายงานเป็น JSON    |

### ruvyxa trace

| Flag     | Type     | Default | Description               |
| -------- | -------- | ------- | ------------------------- |
| `--root` | `path`   | `.`     | Project root              |
| `--path` | `string` | จำเป็น  | Route path ที่ต้องการตรวจ |

### ruvyxa bench

| Flag     | Type   | Default | Description  |
| -------- | ------ | ------- | ------------ |
| `--root` | `path` | `.`     | Project root |

---

## ลำดับการเริ่มต้น dev server

เมื่อรัน `ruvyxa dev` เกิดอะไรขึ้นตามลำดับ:

### เฟส 1: Config load

```
1. อ่าน ruvyxa.config.ts
2. Validate config fields (type + bounds)
3. ตรวจจับ JavaScript runtime (Node/Bun)
4. ตรวจสอบ Node.js version (ต้อง ≥ 22.x)
5. ตรวจสอบ package manager dependencies
6. ตรวจสอบ tsconfig.json
```

### เฟส 2: Port scan

```
1. ใช้ port จาก config.server.port (default 3000)
2. ถ้า port ว่าง → bind
3. ถ้า port ไม่ว่าง → fallback scan
   - ลอง port +1, +2, ... จนถึง +50
   - ถ้าเจอ port ว่าง → bind
   - ถ้าไม่เจอ → RUV1204 PortConflictError
4. ถ้า --port flag ถูกระบุ → ใช้ port นั้นโดยไม่ fallback
```

### เฟส 3: Route scan

```
1. ค้นหาไฟล์ใน appDir (WalkDir)
2. Filter directory: ข้าม _ และ @ prefix
3. จับคู่ไฟล์: page.tsx, page.jsx, page.md, page.mdx → Page
                 route.ts, route.js → API
4. สร้าง route path จาก directory structure
5. ตรวจสอบ route conflicts (RUV1003)
6. ตรวจสอบ layout chain
7. ตรวจจับ render strategy (CSR/PPR/ISR/SSG/SSR)
8. สร้าง RouteManifest
```

### เฟส 4: Server start

```
1. สร้าง Axum Router
2. ลงทะเบียน routes:
   - page routes → SSR/ISR/PPR/SSG handler
   - API routes → HTTP handler
   - static assets → public/ directory
   - HMR WebSocket endpoint
   - error overlay endpoint
3. เริ่ม worker pool (render workers)
4. เริ่ม file watcher (notify crate)
5. เริ่ม HMR tracker
6. เริ่ม HTTP server
```

### เฟส 5: Ready

```
✓ 2 routes scanned
✓ 0 conflicts
✓ HMR ready
➜  Local:   http://localhost:3000
➜  Network: http://192.168.x.x:3000
```

---

## ลำดับการเริ่มต้น dev server (แผนภาพ)

```
ruvyxa dev
    │
    ▼
┌─────────────┐
│ Config Load │← ruvyxa.config.ts + env
└──────┬──────┘
       ▼
┌─────────────┐
│ Port Scan   │← fallback ถ้า port ไม่ว่าง
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

เมื่อคุณแก้ไขไฟล์ Ruvyxa ส่ง WebSocket message ไปยัง browser:

### HMR event types

| Event type     | Trigger                    | Action                             |
| -------------- | -------------------------- | ---------------------------------- |
| `route-change` | ไฟล์ route ใหม่/ลบ         | Route table reload + page refresh  |
| `page-update`  | แก้ไข page component       | Hot-replace component, ไม่ refresh |
| `style-update` | แก้ไข CSS/SCSS             | Inject stylesheet, ไม่ refresh     |
| `full-reload`  | Config หรือ layout เปลี่ยน | Full page reload                   |

HMR ทำงานผ่าน:

1. File watcher (notify crate) ตรวจจับการเปลี่ยนแปลง
2. HMR tracker สรุป event type
3. WebSocket ส่ง event ไป browser
4. Browser runtime จัดการ hot update

---

## TypeScript type definitions

### RuvyxaConfig (จาก ruvyxa/config)

```ts
import { config, type RuvyxaConfig } from 'ruvyxa/config'

// config() เป็น identity function — แค่ให้ type inference
function config<TConfig extends RuvyxaConfig>(config: TConfig): TConfig
```

### RuvyxaConfig interface

```ts
interface RuvyxaConfig {
  appDir?: string // default: 'app'
  outDir?: string // default: '.ruvyxa'
  runtime?: 'node' | 'bun' | 'edge' | 'static' // default: 'node'
  react?: boolean
  typescript?: { strict?: boolean }
  css?: { entries?: string[] }
  server?: { port?: number; host?: string }
  build?: {
    minify?: boolean
    map?: boolean
    treeShake?: boolean
    split?: 'single' | 'route' | 'manual'
    workers?: number
    jsx?: 'classic' | 'automatic'
    target?: 'es2018' | 'es2019' | 'es2020' | 'es2022' | 'esnext'
    manifest?: boolean
    warm?: boolean
    prerenderCache?: boolean
  }
  render?: RenderConfig
  debug?: { overlay?: boolean; traces?: boolean }
  image?: ImageConfig
  security?: {
    actionLimit?: number
    apiLimit?: number
    pluginLimit?: number
    actionRateLimit?: { max?: number; window?: number }
    sameOrigin?: boolean
    fetchMeta?: boolean
    trustedProxyIps?: string[]
    headers?: boolean
  }
  cache?: { routes?: boolean; css?: boolean; dir?: string }
  site?: SiteConfig
  middleware?: MiddlewareConfig
  adapter?: Adapter
  adapterOptions?: Record<string, unknown>
  plugins?: RuvyxaPlugin[]
}
```

### RenderConfig

```ts
interface RenderConfig {
  strategy?: 'ssr' | 'ssg' | 'isr' | 'csr' | 'ppr' // default: 'ssr'
  revalidate?: number // default: 60
}
```

### ImageConfig

```ts
interface ImageConfig {
  optimize?: boolean // default: true
  quality?: number // default: 82 (1-100)
  lossless?: boolean // default: false
  keepOriginal?: boolean // default: true
  variantWidths?: number[] // default: [640, 750, 828, 1080, 1200, 1920, 2048, 3840]
  workers?: number // default: 0 (CPU count)
}
```

### SiteConfig

```ts
interface SiteConfig {
  url?: string // ใช้ RUVYXA_SITE_URL fallback
  sitemap?: boolean | SiteSitemapConfig // default: true
  robots?: boolean | SiteRobotsConfig // default: true
}
```

### SiteSitemapConfig

```ts
interface SiteSitemapConfig {
  exclude?: string[] // exact paths หรือ wildcard *
  additionalPaths?: string[]
  defaults?: {
    lastModified?: string | Date
    changeFrequency?: 'always' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly' | 'never'
    priority?: number // 0.0 - 1.0
  }
  entries?: Array<{
    url: string
    alternates?: { languages?: Record<string, string> }
    images?: string[]
    videos?: SiteSitemapVideo[]
  }>
}
```

---

## การทำงานภายใต้ hood: radix trie routing

Ruvyxa ใช้ **Radix Tree** (compressed trie) สำหรับการจับคู่ URL กับ route:

1. Route path ถูกแปลงเป็น trie nodes
2. Static segment จับคู่ตรง
3. Dynamic segment `[param]` จับคู่ value ใดๆ หนึ่งระดับ
4. Catch-all `[...param]` จับคู่ทุก segment ที่เหลือ
5. Optional catch-all `[[...param]]` จับคู่ทุก segment หรือไม่มีเลย
6. Priority: static > dynamic > catch-all > optional

โครงสร้าง Radix Router ใน `crates/ruvyxa_dev_server/src/router.rs`

---

## Error codes

| Code      | ความหมาย                           | สาเหตุ                                                          | วิธีแก้                                                        |
| --------- | ---------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------- |
| `RUV1001` | App directory not found            | ไม่มีโฟลเดอร์ app/                                              | สร้าง app/ หรือตั้ง appDir ใน config                           |
| `RUV1002` | Invalid dynamic route segment      | รูปแบบ dynamic segment ผิด หรือ catch-all ไม่อยู่ตำแหน่งสุดท้าย | ใช้ `[name]` ไม่ใช่ `:name`; วาง catch-all ไว้ segment สุดท้าย |
| `RUV1003` | Conflicting route paths            | สองไฟล์ match URL shape เดียวกัน                                | ใช้ `npm run routes` หาตัวซ้ำ                                  |
| `RUV1004` | Page missing default export        | page.tsx ไม่มี `export default`                                 | เพิ่ม `export default function Page() {}`                      |
| `RUV1007` | Server-only module in client graph | client component import `server-only` module                    | ย้าย import ไป server component                                |
| `RUV1008` | Private env var in client graph    | client component ใช้ `process.env.PRIVATE`                      | ใช้ `process.env.RUVYXA_PUBLIC_*` แทน                          |
| `RUV1009` | Client-only module in server graph | server component import `client-only` module                    | ย้าย browser-only code                                         |
| `RUV1010` | Server directory reached by client | client import จากโฟลเดอร์ server/                               | ย้าย shared code ไว้นอก server/                                |
| `RUV1100` | React SSR failed                   | Server-side render error                                        | ดู stack trace ใน console                                      |
| `RUV1102` | SSR renderer not found             | Build output ขาด server handler                                 | รัน `npm run build` ใหม่                                       |
| `RUV1200` | API route execution failed         | route.ts runtime error                                          | ดู error message                                               |
| `RUV1201` | API route error (runtime)          | API route throw exception                                       | ตรวจสอบ route handler                                          |
| `RUV1204` | Port conflict                      | Port ไม่ว่าง                                                    | ใช้ `--port` flag หรือ kill process                            |
| `RUV1205` | Prerender path conflict            | static path ชนกับ build output                                  | เปลี่ยน outDir                                                 |
| `RUV1300` | Client hydration bundling failed   | Build client bundle error                                       | ดู compiler output                                             |
| `RUV1303` | Client route not found             | Request client bundle ที่ไม่มี route                            | เช็ค route path                                                |
| `RUV1304` | Client bundle for non-page route   | ขอ client bundle ของ API route                                  | ใช้เฉพาะ page routes                                           |
| `RUV1400` | Tailwind CSS compilation failed    | Tailwind CLI error                                              | ตรวจสอบ Tailwind config                                        |
| `RUV1401` | Tailwind CLI not found             | ขาด Tailwind dependency                                         | `npm install tailwindcss`                                      |
| `RUV1402` | Sass compilation failed            | ไฟล์ .scss syntax error                                         | ตรวจสอบไฟล์ SCSS                                               |
| `RUV1403` | CSS entry not found                | ไฟล์ CSS ใน config ไม่มีอยู่                                    | เช็ค path                                                      |
| `RUV1404` | CSS entry outside project root     | CSS entry path อยู่นอก project                                  | ใช้ relative path ใน project                                   |
| `RUV1500` | SSG/ISR render failed              | Static generation error                                         | ดู error detail                                                |
| `RUV1501` | Route action not found             | ขาด action.ts ใน route                                          | สร้าง action.ts                                                |
| `RUV1550` | PPR render failed                  | PPR streaming error                                             | ดู error detail                                                |
| `RUV1600` | Config validation error            | ruvyxa.config.ts ผิด format                                     | รัน `ruvyxa doctor`                                            |
| `RUV1601` | Config value out of range          | Field value ไม่อยู่ในช่วงที่ยอมรับ                              | ปรับค่าให้อยู่ในช่วง                                           |
| `RUV1602` | Config value exceeds maximum       | Field value เกินขีดจำกัด                                        | ลดค่าลง                                                        |
| `RUV1700` | TypeScript plugin error            | Plugin runtime error                                            | ตรวจสอบ plugin code                                            |
| `RUV1701` | TypeScript plugin protocol error   | Plugin ส่งข้อมูลรูปแบบผิด                                       | ตรวจสอบ plugin implementation                                  |
| `RUV1702` | Worker pool script not found       | ขาด runtime script                                              | รัน `npm run build` ใหม่                                       |
| `RUV1704` | Worker pool error                  | Worker crash                                                    | ดู worker log                                                  |
| `RUV2200` | Adapter build failed               | Adapter runtime error                                           | ตรวจสอบ adapter                                                |
| `RUV2202` | Strategy not supported             | Adapter ไม่รองรับ render strategy                               | เปลี่ยน strategy หรือ adapter                                  |
| `RUV2203` | Adapter package missing            | ไม่พบ adapter package                                           | `npm install @ruvyxa/adapter-*`                                |
| `RUV9999` | Internal error                     | Compiler internal error                                         | รายงาน bug                                                     |

---

## Troubleshooting เต็มรูปแบบ

### Dev server ไม่เริ่ม

```
Error: Address already in use (os error 10048)
```

สาเหตุ: Port 3000 ถูกใช้งานแล้ว วิธีแก้:

```bash
# 1. เปลี่ยน port
ruvyxa dev --port 4000

# 2. หรือ kill process ที่ใช้ port 3000
# Windows:
netstat -ano | findstr :3000
taskkill /PID <PID> /F

# macOS/Linux:
lsof -i :3000
kill -9 <PID>
```

### Route ไม่ขึ้น

```
RUV1003: Conflicting route paths
```

สาเหตุ: สองไฟล์ match URL shape เดียวกัน วิธีแก้:

```bash
# ดู route table
npm run routes

# ลบ route ที่ซ้ำ หรือเปลี่ยนชื่อไฟล์
```

### "RUV1007: Client boundary violation"

สาเหตุ: `'use client'` component import `server-only` module วิธีแก้:

1. สร้าง API route (`app/api/.../route.ts`) ที่เรียก database
2. client component fetch จาก API นั้น
3. หรือย้าย data fetching logic ไป server component

### Build error แปลกๆ

```
Error: RUV1300: Client hydration bundling failed
```

สาเหตุ: Build error ที่ไม่ชัดเจน วิธีแก้:

```bash
# clean + rebuild
npm run clean && npm run build

# หรือดู error ละเอียด
npm run build -- --verbose
```

### TypeScript error หลังสร้างโปรเจค

สาเหตุ: tsconfig ไม่ตรงกับ Ruvyxa expectations วิธีแก้:

```bash
npm run doctor
# จะแนะนำ tsconfig ที่ถูกต้อง
```

### HMR ไม่ทำงาน

สาเหตุ: WebSocket ไม่สามารถเชื่อมต่อ (network config, proxy, firewall) วิธีแก้:

- เปิดดู browser console (F12) มี WebSocket error หรือไม่
- ถ้าใช้ reverse proxy, ต้องส่งต่อ WebSocket
- ตั้ง HMR endpoint ใน config (ถ้ามี)

### Dependency problems

สาเหตุ: node_modules หรือ lockfile เสีย วิธีแก้:

```bash
# npm
rm -rf node_modules package-lock.json && npm install

# pnpm
rm -rf node_modules pnpm-lock.yaml && pnpm install

# yarn
rm -rf node_modules yarn.lock && yarn install
```

### "RUV1008: Server-only hook"

สาเหตุ: ใช้ `useState`, `useEffect`, หรือ event handlers ใน server component วิธีแก้: เพิ่ม
`'use client'` directive ที่บรรทัดแรกของไฟล์

---

## อภิธานศัพท์

| คำศัพท์           | ความหมาย                                                 |
| ----------------- | -------------------------------------------------------- |
| **Route**         | URL path ที่ถูกจัดการโดยไฟล์ใน app/                      |
| **Layout**        | component หุ้ม (layout.tsx) ที่ persist ข้าม child pages |
| **HMR**           | Hot Module Replacement — อัพเดต browser โดยไม่โหลดใหม่   |
| **SSR**           | Server-Side Rendering — HTML ทุก request                 |
| **SSG**           | Static Site Generation — HTML ตอน build                  |
| **ISR**           | Incremental Static Regeneration — SSG + cache TTL        |
| **CSR**           | Client-Side Rendering — HTML ขั้นต่ำ, JS render ทั้งหมด  |
| **PPR**           | Partial Pre-Rendering — static shell + dynamic streaming |
| **API Route**     | ไฟล์ route.ts ที่คืน JSON หรือ Response                  |
| **Server Action** | ฟังก์ชันใน action.ts ที่รันบน server                     |
| **Adapter**       | ปลั๊กอิน deploy: Vercel, Netlify, Cloudflare, Node ฯลฯ   |
| **.ruvyxa/**      | Output directory (cache + build artifacts)               |
| **RUV####**       | Error code เช่น RUV1003 — ค้นหาใน docs                   |
| **Boundary**      | เส้นแบ่งระหว่าง server code กับ client code              |
| **Directive**     | คำสั่ง 'use client' หรือ 'use server' ที่บอก bundler     |
| **Radix Trie**    | data structure สำหรับ route matching                     |
| **Meta**          | Metadata export สำหรับ SEO: title, description, OG       |

---

## ขั้นตอนถัดไป

- **[02-routing.md](./02-routing.md)** — File-based router แบบละเอียด
- **[03-server-client-components.md](./03-server-client-components.md)** — Server vs Client
  components
- **[04-rendering-strategies.md](./04-rendering-strategies.md)** — SSR, SSG, ISR, PPR, CSR
- **[05-data-loading-cache.md](./05-data-loading-cache.md)** — Data loading and cache
- **[06-server-actions.md](./06-server-actions.md)** — Server actions
- **[07-api-routes.md](./07-api-routes.md)** — REST/GraphQL endpoints
- **[08-styling.md](./08-styling.md)** — CSS, SCSS, CSS Modules
