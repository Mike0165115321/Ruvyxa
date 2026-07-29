# คำสั่ง CLI ทั้ง 13 คำสั่ง

Ruvyxa CLI มีเครื่องมือครบทุกงานพัฒนา — ตั้งแต่ dev server, production build, static analysis,
debug, benchmark, parity testing, ไปจนถึง scaffolding plugins

```
Ruvyxa <COMMAND> [options]

Commands:
  dev           dev server พร้อม HMR
  build         production build
  check         production readiness validation
  start         serve production build
  preview       build + start
  routes        print route table
  analyze       validate routes, imports, boundaries
  doctor        project diagnostics
  clean         ลบ generated output + cache
  trace         inspect route manifest (ละเอียด)
  bench         benchmark (route discovery, analysis, build)
  test:parity   เปรียบเทียบ dev vs prod rendering
  plugin        create plugin package scaffolding
```

---

## Global Options

| Option             | Alias | คำอธิบาย                     | ใช้กับทุกคำสั่ง |
| ------------------ | ----- | ---------------------------- | --------------- |
| `--help`           | `-h`  | แสดง help                    | ✅              |
| `--root <path>`    | -     | รากโปรเจกต์ (default: `.`)   | ✅              |
| `--runtime <name>` | -     | JS runtime (`node` \| `bun`) | ✅              |
| `--verbose`        | `-v`  | แสดง log ละเอียด             | ✅              |
| `--no-color`       | -     | ปิด colored output           | ✅              |
| `--version`        | -     | แสดง version                 | ✅              |

```bash
# ดู help รวม
ruvyxa --help

# ดู help แต่ละคำสั่ง
ruvyxa dev --help
ruvyxa build --help
ruvyxa analyze --help

# ใช้ verbose mode
ruvyxa build --verbose

# ระบุ runtime
ruvyxa dev --runtime bun

# ระบุรากโปรเจกต์
ruvyxa dev --root /home/user/my-project
```

---

## Exit Codes

| Code | ความหมาย                 | คำสั่งที่ใช         |
| ---- | ------------------------ | ------------------- |
| `0`  | Success                  | ทุกคำสั่ง           |
| `1`  | General error            | ทุกคำสั่ง           |
| `2`  | Config validation error  | check, build, dev   |
| `3`  | Build failed             | build, preview      |
| `4`  | Boundary violation found | check, analyze      |
| `5`  | Route conflict found     | check, routes       |
| `6`  | Doctor found issues      | doctor              |
| `7`  | Parity test failed       | test:parity         |
| `8`  | Port in use              | dev, start, preview |
| `9`  | Plugin scaffolding error | plugin create       |
| `10` | Cache clean error        | clean               |
| `11` | Benchmark interrupted    | bench               |

---

## 1. `ruvyxa dev`

เริ่ม dev server พร้อม Hot Module Replacement (HMR)

### Syntax

```bash
ruvyxa dev [options]
```

### Options

| Option         | Type              | Default       | คำอธิบาย                   |
| -------------- | ----------------- | ------------- | -------------------------- |
| `--root`       | `string`          | `.`           | รากโปรเจกต์                |
| `--host`       | `string`          | `'localhost'` | Host ที่ server ผูก        |
| `--port`       | `number`          | `3000`        | Port                       |
| `--runtime`    | `'node' \| 'bun'` | auto          | JS runtime                 |
| `--open`       | `boolean`         | `false`       | เปิด browser อัตโนมัติ     |
| `--https`      | `boolean`         | `false`       | ใช้ HTTPS (ต้องมี cert)    |
| `--cert`       | `string`          | -             | Path ถึง HTTPS certificate |
| `--key`        | `string`          | -             | Path ถึง HTTPS private key |
| `--no-hmr`     | `boolean`         | `false`       | ปิด HMR                    |
| `--no-overlay` | `boolean`         | `false`       | ปิด error overlay          |
| `--inspect`    | `boolean`         | `false`       | เปิด Node.js inspector     |

### ตัวอย่าง

```bash
# พื้นฐาน
npm run dev

# เปลี่ยน host และ port
ruvyxa dev --host 0.0.0.0 --port 8080

# ใช้ Bun runtime
ruvyxa dev --runtime bun

# HTTPS
ruvyxa dev --https --cert ./cert.pem --key ./key.pem

# เปิด browser อัตโนมัติ
ruvyxa dev --open

# Dev + debug inspector
ruvyxa dev --inspect

# Dev without HMR (สำหรับ debug)
ruvyxa dev --no-hmr
```

### ตัวอย่าง Output

```
⚡ Ruvyxa dev server running

  ➜  Local:   http://localhost:3000
  ➜  Network: http://192.168.1.100:3000
  ➜  Runtime: node (v22.5.0)

  ✓ 4 routes scanned       in 12ms
  ✓ 0 conflicts
  ✓ Config valid
  ✓ HMR ready              (WebSocket)
  ✓ Layout chain: root

  Watching for changes...
  [HMR] 2026-07-29 10:00:00 — app/page.tsx modified
  [HMR] 2026-07-29 10:00:01 ✓ Rebuilt (23ms)
```

### Output เมื่อมี Warning

```
⚠ Port 3000 is in use by another process
  Using port 3001 instead

⚠ 2 routes have duplicate slugs
  Run `ruvyxa routes` for details

⚠ 1 boundary violation
  Run `ruvyxa analyze` for details
```

### คุณสมบัติ Dev Server

| Feature                      | รายละเอียด                                       |
| ---------------------------- | ------------------------------------------------ |
| HMR (Hot Module Replacement) | แก้ไฟล์ → browser อัปเดตทันที ไม่ต้อง refresh    |
| Error Overlay                | แสดง error ใน browser แทน white screen           |
| Route Watching               | เพิ่ม route ใหม่ → ขึ้นอัตโนมัติ ไม่ต้อง restart |
| CSS Hot Reload               | เปลี่ยน CSS → inject โดยไม่ refresh              |
| Server Action Debugging      | แสดง server action logs ใน terminal              |
| Fast Refresh                 | React component state ยังคงอยู่                  |
| WebSocket Connection         | HMR ใช้ WebSocket (ข้าม firewall ไม่ได้)         |
| Source Maps                  | แสดง source code จริง (ไม่ใช่ compiled)          |

---

## 2. `ruvyxa build`

Build แอปพลิเคชันสำหรับ production

### Syntax

```bash
ruvyxa build [options]
```

### Options

| Option        | Type                                     | Default  | คำอธิบาย             |
| ------------- | ---------------------------------------- | -------- | -------------------- |
| `--root`      | `string`                                 | `.`      | รากโปรเจกต์          |
| `--target`    | `'node' \| 'bun' \| 'edge' \| 'static'`  | auto     | Build target         |
| `--adapter`   | `string`                                 | auto     | Deployment adapter   |
| `--runtime`   | `'node' \| 'bun'`                        | auto     | JS runtime           |
| `--analyze`   | `boolean`                                | `false`  | เปิด bundle analyzer |
| `--profile`   | `boolean`                                | `false`  | เปิด build profiler  |
| `--sourcemap` | `boolean`                                | `false`  | สร้าง source maps    |
| `--no-minify` | `boolean`                                | `false`  | ปิด minification     |
| `--no-cache`  | `boolean`                                | `false`  | ปิด build cache      |
| `--log-level` | `'error' \| 'warn' \| 'info' \| 'debug'` | `'info'` | ระดับ log            |

### ตัวอย่าง

```bash
# พื้นฐาน
npm run build

# ระบุ target
ruvyxa build --target node
ruvyxa build --target static
ruvyxa build --target edge

# ระบุ adapter
ruvyxa build --adapter vercel

# เปิด bundle analyzer
ruvyxa build --analyze

# Build without cache (clean build)
ruvyxa build --no-cache

# สร้าง source maps
ruvyxa build --sourcemap

# Production debug log
ruvyxa build --log-level debug
```

### ตัวอย่าง Output — Success

```
━━━ Build ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  target    production
  root      /home/user/my-app
  app dir   /home/user/my-app/app
  out dir   /home/user/my-app/.ruvyxa
  adapter   node
  runtime   node

  ✓ routes discovered    4 routes         12ms
  ✓ validated            ok                8ms
  ✓ style collected      3 files          45ms
  ✓ built                compiled          2.3s
  ✓ image optimized      12 variants       1.1s
  ✓ prerendered          2 pages          0.8s

  ✓ Build complete (4.2s)

    ├─ server/                   2.1 MB
    │  ├─ server.js             1.2 MB
    │  └─ chunks/               0.9 MB
    ├─ client/                  890 KB (gzip: 280 KB)
    │  ├─ client.js             420 KB
    │  ├─ pages/                320 KB
    │  └─ chunks/               150 KB
    ├─ prerender/               4 pages
    │  ├─ index.html            12 KB
    │  ├─ about.html            8 KB
    │  └─ blog/                 2 files
    └─ assets/                  12 files
       ├─ images/               10 files
       └─ manifest.json         1 file
```

### ตัวอย่าง Output — Error

```
━━━ Build Error ━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ✗ Build failed (2 errors)

  RUV1008: Environment boundary violation
    File: app/components/danger.tsx:5:18
    Code: process.env.DATABASE_URL used in client component
    Fix: Move to server component or prefix with RUVYXA_PUBLIC_

  RUV1010: Module not found
    File: app/api/route.ts:3:22
    Code: import 'missing-package'
    Fix: npm install missing-package

  ─────────────────────────────────────────────
  ✗ Build exited with code 3
```

### Build Targets Detail

| Target   | คำอธิบาย       | Output                        | ใช้ deploy กับ                      |
| -------- | -------------- | ----------------------------- | ----------------------------------- |
| `node`   | Node.js server | server bundle (Express-based) | VPS, Docker, Railway, Render        |
| `bun`    | Bun runtime    | bun-friendly bundle           | Bun deploy                          |
| `edge`   | Edge runtime   | smallest bundle               | Vercel Edge, Cloudflare Workers     |
| `static` | Static export  | HTML + JS + CSS files         | Any static host (S3, Netlify, etc.) |

### Build Phases

| Phase              | คำอธิบาย                                     | เวลาโดยประมาณ |
| ------------------ | -------------------------------------------- | ------------- |
| Route Discovery    | scan app directory, parse file system routes | 10-50ms       |
| Validation         | check route conflicts, boundary rules        | 5-20ms        |
| Style Collection   | collect CSS entries, process PostCSS         | 20-100ms      |
| Compilation        | transpile TypeScript, bundle JS              | 1-10s         |
| Image Optimization | resize, encode WebP/AVIF                     | 0.5-5s        |
| Prerendering       | SSG pages rendering                          | 0.1-2s        |
| Output             | write files, generate manifest               | 10-50ms       |

---

## 3. `ruvyxa check`

ตรวจสอบ production readiness — routes, config, imports, server/client boundaries

### Syntax

```bash
ruvyxa check [options]
```

### Options

| Option      | Type              | Default | คำอธิบาย                   |
| ----------- | ----------------- | ------- | -------------------------- |
| `--root`    | `string`          | `.`     | รากโปรเจกต์                |
| `--runtime` | `'node' \| 'bun'` | auto    | JS runtime                 |
| `--strict`  | `boolean`         | `false` | ตรวจเข้ม (warning → error) |
| `--json`    | `boolean`         | `false` | JSON output                |

### ตัวอย่าง

```bash
# พื้นฐาน
npm run check

# Strict mode
ruvyxa check --strict

# JSON output สำหรับ CI
ruvyxa check --json
```

### ตัวอย่าง Output — Pass

```
━━━ Check ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ✓ All checks passed (0.8s)

  Checks performed:
    Routes:               4 valid
    Layout chain:         OK
    No ambiguous routes  OK
    Server/client boundaries: OK
    Config:              valid
    Imports:             23 modules
    Dependencies:        all installed
    No critical diagnostics
```

### ตัวอย่าง Output — Fail

```
━━━ Check ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ✗ 3 issues found (use --strict to see warnings)

  RUV1007: Client boundary violation
    app/page.tsx:15 imports from 'server-only' module
    ✗ Fix: move import behind server boundary

  RUV1010: Module not found
    app/api/route.ts:3 imports 'missing-package'
    ✗ Fix: npm install missing-package

  RUV1005: Missing Meta component
    app/contact/page.tsx
    ⚠ Fix: add <Meta> component for SEO

  ─────────────────────────────────────────────
  ✗ Check failed (1.2s)
  ✗ Exit code: 4
```

### Checks Performed

| Check                 | รายละเอียด                  | Error Code |
| --------------------- | --------------------------- | ---------- |
| Route conflicts       | route path ซ้ำ              | RUV1001    |
| Ambiguous routes      | dynamic route ไม่ชัดเจน     | RUV1002    |
| Layout chain          | layout hierarchy ถูกต้อง    | RUV1003    |
| Boundary violations   | server code ใน client       | RUV1007    |
| Private env in client | process.env.XXX ใน client   | RUV1008    |
| Missing modules       | import ไม่มี                | RUV1010    |
| Config validity       | ruvyxa.config.ts validation | RUV1600+   |
| Syntax errors         | TypeScript/JS syntax        | RUV1011    |
| Circular dependencies | import loop                 | RUV1012    |
| Missing dependencies  | npm packages                | RUV1013    |
| SEO metadata          | missing Meta component      | RUV1005    |

---

## 4. `ruvyxa start`

รัน production server จาก build ที่ compile แล้ว (ใช้ไฟล์จาก `.ruvyxa/`)

### Syntax

```bash
ruvyxa start [options]
```

### Options

| Option      | Type              | Default       | คำอธิบาย    |
| ----------- | ----------------- | ------------- | ----------- |
| `--root`    | `string`          | `.`           | รากโปรเจกต์ |
| `--host`    | `string`          | `'localhost'` | Host        |
| `--port`    | `number`          | `3000`        | Port        |
| `--runtime` | `'node' \| 'bun'` | auto          | JS runtime  |

### ตัวอย่าง

```bash
# build ก่อน แล้วค่อย start
npm run build && npm run start

# ระบุ port
ruvyxa start --port 8080

# เปิด network
ruvyxa start --host 0.0.0.0 --port 443
```

### ตัวอย่าง Output

```
━━━ Ruvyxa production server running ━━━━━━━

  ➜  Local:   http://localhost:3000
  ➜  Network: http://192.168.1.100:3000
  ➜  Mode:    production
  ➜  Runtime: node (v22.5.0)

  ✓ Server initialized
  ✓ 4 routes loaded
  ✓ Adapter: node
  ✓ Production ready

  ℹ Press Ctrl+C to stop
```

### ตัวอย่าง Error

```
━━━ Server Error ━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ✗ Build output not found
    Expected .ruvyxa/ directory to exist
    Fix: Run `ruvyxa build` first

  ✗ Port 3000 is in use
    Fix: Use --port 4000 or stop the other process
    ✗ Exit code: 8
```

---

## 5. `ruvyxa preview`

Build + start ในคำสั่งเดียว (ใช้สำหรับ preview ก่อน deploy)

### Syntax

```bash
ruvyxa preview [options]
```

### Options

| Option      | Type                                    | Default       | คำอธิบาย           |
| ----------- | --------------------------------------- | ------------- | ------------------ |
| `--root`    | `string`                                | `.`           | รากโปรเจกต์        |
| `--host`    | `string`                                | `'localhost'` | Host               |
| `--port`    | `number`                                | `3000`        | Port               |
| `--runtime` | `'node' \| 'bun'`                       | auto          | JS runtime         |
| `--target`  | `'node' \| 'bun' \| 'edge' \| 'static'` | auto          | Build target       |
| `--adapter` | `string`                                | auto          | Deployment adapter |

### ตัวอย่าง

```bash
npm run preview
ruvyxa preview --port 4000
```

### ตัวอย่าง Output

```
━━━ Preview ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  [Build Phase]
  ✓ routes discovered     4 routes      12ms
  ✓ validated             ok             8ms
  ✓ built                 compiled      2.3s
  ✓ image optimized       12 variants   1.1s
  ✓ Build complete (3.5s)

  [Server Phase]
  ⚡ Ruvyxa production server running
  ➜  Local:   http://localhost:3000
  ➜  Mode:    production
  ➜  Runtime: node

  ✓ Preview ready (3.6s total)
  ℹ Press Ctrl+C to stop
```

---

## 6. `ruvyxa routes`

แสดง route table ทั้งหมด — path, type, layouts, strategy

### Syntax

```bash
ruvyxa routes [options]
```

### Options

| Option      | Type                             | Default  | คำอธิบาย                  |
| ----------- | -------------------------------- | -------- | ------------------------- |
| `--root`    | `string`                         | `.`      | รากโปรเจกต์               |
| `--runtime` | `'node' \| 'bun'`                | auto     | JS runtime                |
| `--json`    | `boolean`                        | `false`  | JSON output               |
| `--filter`  | `string`                         | -        | กรอง route (glob pattern) |
| `--sort`    | `'path' \| 'type' \| 'strategy'` | `'path'` | เรียงลำดับ                |

### ตัวอย่าง

```bash
npm run routes

# JSON
ruvyxa routes --json

# กรองเฉพาะ blog routes
ruvyxa routes --filter 'blog/*'

# เรียงตาม strategy
ruvyxa routes --sort strategy

# ใช้กับ pipe
ruvyxa routes --json | jq '.routes | length'
```

### ตัวอย่าง Output — Human

```
━━━ Route Manifest ━━━━━━━━━━━━━━━━━━━━━━━━━
  App directory: /home/user/my-app/app

  Path                  Type      Layouts            Strategy
 ─────────────────────────────────────────────────────────────
  /                     page      root                SSG
  /about                page      root                SSR
  /blog                 page      root                SSR
  /blog/[slug]          page      root                SSG
  /api/health           api       -                   -
  /api/users/[id]       api       -                   -
  /api/auth/login       api       -                   -

  Legend: page → 4, api → 3, layout → 1, action → 2
  Total: 7 routes • 0 conflicts • 0 warnings
```

### ตัวอย่าง Output — JSON

```json
{
  "appDir": "/home/user/my-app/app",
  "routes": [
    {
      "path": "/",
      "type": "page",
      "layouts": ["root"],
      "strategy": "SSG",
      "file": "app/page.tsx",
      "params": [],
      "staticPaths": []
    },
    {
      "path": "/blog/[slug]",
      "type": "page",
      "layouts": ["root"],
      "strategy": "SSG",
      "file": "app/blog/[slug]/page.tsx",
      "params": ["slug"],
      "staticPaths": [{ "slug": "hello-world" }, { "slug": "second-post" }]
    },
    {
      "path": "/api/health",
      "type": "api",
      "layouts": [],
      "strategy": null,
      "file": "app/api/health/route.ts",
      "methods": ["GET"]
    }
  ],
  "total": 7,
  "conflicts": 0,
  "warnings": 0
}
```

### Route Type Legend

| Type      | Icon | คำอธิบาย       |
| --------- | ---- | -------------- |
| `page`    | 📄   | หน้าเพจ        |
| `api`     | 🔌   | API route      |
| `layout`  | 🏗️   | Layout wrapper |
| `action`  | ⚡   | Server action  |
| `loading` | ⏳   | Loading UI     |
| `error`   | ❌   | Error boundary |

---

## 7. `ruvyxa analyze`

วิเคราะห์ routes, imports, server/client boundaries, dependencies, การใช้งานรูป

### Syntax

```bash
ruvyxa analyze [options]
```

### Options

| Option      | Type                                     | Default   | คำอธิบาย               |
| ----------- | ---------------------------------------- | --------- | ---------------------- |
| `--root`    | `string`                                 | `.`       | รากโปรเจกต์            |
| `--runtime` | `'node' \| 'bun'`                        | auto      | JS runtime             |
| `--format`  | `'auto' \| 'human' \| 'json' \| 'sarif'` | `'auto'`  | รูปแบบ output          |
| `--output`  | `string`                                 | -         | เขียน output ไปยังไฟล์ |
| `--checks`  | `string[]`                               | all       | เฉพาะ check บางประเภท  |
| `--fail-on` | `'error' \| 'warning'`                   | `'error'` | exit code เมื่อเจอ     |

### ตัวอย่าง

```bash
# พื้นฐาน
npm run analyze

# JSON output
ruvyxa analyze --format json

# SARIF สำหรับ GitHub
ruvyxa analyze --format sarif --output results.sarif

# เฉพาะบาง check
ruvyxa analyze --checks boundaries,images,env

# Fail on warnings
ruvyxa analyze --fail-on warning
```

### ตัวอย่าง Output — Human

```
━━━ Analyze ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Routes
  ──────
  4 pages
  2 api routes
  0 conflicts                ✅
  0 ambiguous                ✅

  Layouts
  ───────
  1 root layout
  2 nested layouts
  Chain valid                ✅

  Imports
  ───────
  23 server modules
  12 client modules
  1 boundary violation        ⚠️

  Environment Variables
  ────────────────────
  5 public (RUVYXA_PUBLIC_*)
  8 private (server-only)
  0 violations               ✅

  Images
  ──────
  8 total
  6 using <Image>
  2 using <img>               ⚠️
  0 external (unoptimized)

  Bundle Size
  ───────────
  Server:  2.1 MB
  Client:  890 KB (gzip: 280 KB)

  Diagnostics: 3 issues
  ───────────────────

  RUV1007: Client boundary violation
    File: app/page.tsx:15:5
    Why: Imports 'server-only' module
    Fix: Move import behind server boundary

  RUV1008: Private env in client
    File: app/utils.ts:3:10
    Why: DATABASE_URL used in client scope
    Fix: Prefix with RUVYXA_PUBLIC_ or move to server

  RUV1005: Missing Meta component
    File: app/contact/page.tsx
    Why: No SEO metadata defined
    Fix: Add <Meta> component

  ─────────────────────────────────────────────
  ℹ 3 issues found (3 warnings, 0 errors)
```

### ตัวอย่าง Output — JSON

```json
{
  "routes": {
    "pages": 4,
    "apis": 2,
    "conflicts": 0,
    "ambiguous": 0
  },
  "imports": {
    "server": 23,
    "client": 12,
    "boundaryViolations": 1
  },
  "env": {
    "public": 5,
    "private": 8,
    "violations": 0
  },
  "images": {
    "total": 8,
    "usingImageComponent": 6,
    "usingImgTag": 2,
    "external": 0
  },
  "diagnostics": [
    {
      "code": "RUV1007",
      "severity": "warning",
      "file": "app/page.tsx",
      "line": 15,
      "message": "Client boundary violation",
      "detail": "Imports 'server-only' module",
      "fix": "Move import behind server boundary"
    }
  ],
  "totalIssues": 3,
  "errors": 0,
  "warnings": 3
}
```

### Output Format Detail

| Format  | คำอธิบาย                                                           | เหมาะกับ                 |
| ------- | ------------------------------------------------------------------ | ------------------------ |
| `auto`  | ตัดสินใจจาก terminal หรือ pipe (human ถ้า terminal, json ถ้า pipe) | ทั่วไป                   |
| `human` | อ่านง่าย สีสัน                                                     | Local dev                |
| `json`  | Structured data                                                    | Tools, scripts           |
| `sarif` | SARIF 2.1.0                                                        | GitHub Code Scanning, CI |

### Check Types

| Check        | คำอธิบาย                               | Default              |
| ------------ | -------------------------------------- | -------------------- |
| `routes`     | ตรวจสอบ route conflicts, ambiguous     | ✅                   |
| `boundaries` | ตรวจ server/client boundary violations | ✅                   |
| `images`     | ตรวจการใช้งานรูป <Image> vs <img>      | ✅                   |
| `env`        | ตรวจ environment variables violations  | ✅                   |
| `imports`    | ตรวจ dependency graph                  | ✅                   |
| `seo`        | ตรวจ missing Meta component            | ✅                   |
| `deps`       | ตรวจ missing npm packages              | ✅                   |
| `size`       | ตรวจ bundle size                       | ❌ (ต้อง build ก่อน) |

---

## 8. `ruvyxa doctor`

วินิจฉัยโปรเจกต์ — Node version, config validity, routes, dependencies, port, adapter

### Syntax

```bash
ruvyxa doctor [options]
```

### Options

| Option                 | Type              | Default | คำอธิบาย              |
| ---------------------- | ----------------- | ------- | --------------------- |
| `--root`               | `string`          | `.`     | รากโปรเจกต์           |
| `--target`             | `string`          | auto    | Build target          |
| `--adapter`            | `string`          | auto    | Deployment adapter    |
| `--runtime`            | `'node' \| 'bun'` | auto    | JS runtime            |
| `--json`               | `boolean`         | `false` | JSON output           |
| `--fix`                | `boolean`         | `false` | auto-fix บางปัญหา     |
| `--generate-env-types` | `boolean`         | `false` | สร้าง ruvyxa-env.d.ts |

### ตัวอย่าง

```bash
npm run doctor

# JSON
ruvyxa doctor --json

# Auto-fix
ruvyxa doctor --fix

# Generate env types
ruvyxa doctor --generate-env-types

# ใช้ใน CI
ruvyxa doctor --json | jq '.ok'
```

### ตัวอย่าง Output

```
━━━ Ruvyxa Doctor ━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Environment
  ───────────
  ✓ Node.js           v22.5.0 (>= 22.x)
  ✓ OS                win32
  ✓ Runtime           node

  Config
  ──────
  ✓ File exists       ruvyxa.config.ts
  ✓ Config valid
  ✓ No unknown fields
  ✓ site.url          https://example.com

  Routes
  ──────
  ✓ Routes            / (SSG), /about (SSR),
                       /blog (SSR), /api/health
  ✓ Layouts           root → valid chain
  ✓ No conflicts
  ✓ No ambiguous

  Adapter
  ───────
  ✓ Detected          node
  ✓ Override          RUVYXA_ADAPTER not set

  Environment Variables
  ────────────────────
  ✓ Files loaded      .env, .env.local
  ✓ Public vars       5 found
  ✓ Private vars      8 found
  ✓ No violations

  Dependencies
  ────────────
  ✓ All installed     (0 missing)
  ✓ Package manager   pnpm

  Port
  ────
  ⚠ Port 3000 in use by another process

  ─────────────────────────────────────────────
  ⚠ 1 issue found
    Fix: use --port 4000 or stop the other process
    ℹ Run `ruvyxa doctor --fix` to auto-resolve
```

### ตัวอย่าง Output — JSON

```json
{
  "ok": false,
  "nodeVersion": "v22.5.0",
  "os": "win32",
  "runtime": "node",
  "configValid": true,
  "routes": 4,
  "adapter": "node",
  "env": {
    "files": [".env", ".env.local"],
    "publicCount": 5,
    "privateCount": 8,
    "violations": 0
  },
  "issues": [
    {
      "code": "RUV1201",
      "severity": "warning",
      "title": "Port 3000 is in use",
      "fix": "Use --port 4000 or stop the other process",
      "autoFixable": true
    }
  ],
  "issueCount": 1,
  "autoFixableCount": 1
}
```

### Doctor Checks

| Check             | ตรวจ                     | แก้ไขอัตโนมัติ      |
| ----------------- | ------------------------ | ------------------- |
| Node.js version   | ≥ 22.x                   | ❌                  |
| Config file       | มีไฟล์ + syntax ถูก      | ❌                  |
| Config validation | ทุก field ถูกต้อง        | ❌                  |
| Routes            | conflicts, ambiguous     | ❌                  |
| Layout chain      | missing layout           | ❌                  |
| Adapter           | auto-detect หรือตั้งค่า  | ❌                  |
| Env vars          | files loaded, violations | ❌                  |
| Dependencies      | missing packages         | ✅ (npm install)    |
| Port              | port 3000 ถูกใช้งาน      | ✅ (suggest --port) |
| .env.example      | มีไฟล์                   | ✅ (generate)       |

---

## 9. `ruvyxa clean`

ลบโฟลเดอร์ `.ruvyxa/` และ cache ทั้งหมด

### Syntax

```bash
ruvyxa clean [options]
```

### Options

| Option      | Type              | Default | คำอธิบาย            |
| ----------- | ----------------- | ------- | ------------------- |
| `--root`    | `string`          | `.`     | รากโปรเจกต์         |
| `--runtime` | `'node' \| 'bun'` | auto    | JS runtime          |
| `--all`     | `boolean`         | `false` | ลบทุกอย่างรวม cache |

### ตัวอย่าง

```bash
npm run clean

# Clean ทั้งหมด
ruvyxa clean --all

# Dry run (แสดงว่าจะลบอะไรบ้าง)
ruvyxa clean --dry-run
```

### ตัวอย่าง Output

```
━━━ Clean ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  ✓ Removed .ruvyxa/            (124.5 MB)
  ✓ Removed .ruvyxa/cache       (23.1 MB)
  ✓ Removed .ruvyxa/assets      (45.2 MB)

  ──────────────────────────────────
  ✓ Freed 192.8 MB

  Directories cleaned:
  - .ruvyxa/         (build output)
  - .ruvyxa/cache/   (build cache)
  - .ruvyxa/assets/  (optimized images)
```

### เมื่อไหร่ควร clean

| สถานการณ์                | เหตุผล                            |
| ------------------------ | --------------------------------- |
| build error แปลก ๆ       | cache เสีย → clean + rebuild      |
| เปลี่ยน config ครั้งใหญ่ | cache เก่า conflict               |
| ต้องการพื้นที่ว่าง       | .ruvyxa/ อาจใหญ่ถึง 200MB+        |
| ก่อน deploy              | clean state → deterministic build |
| เปลี่ยน image sizes      | image cache เก่า                  |
| เปลี่ยน plugins          | plugin ลงทะเบียน hooks ใหม่       |

---

## 10. `ruvyxa trace`

ตรวจสอบ route แต่ละรายการแบบละเอียด — layout chain, server/client modules, static params, rendering

### Syntax

```bash
ruvyxa trace <route-path> [options]
```

### Arguments

| Argument       | Required | คำอธิบาย                                                     |
| -------------- | -------- | ------------------------------------------------------------ |
| `<route-path>` | ✅       | เส้นทาง route (เช่น `/`, `/blog/hello-world`, `/api/health`) |

### Options

| Option   | Type      | Default | คำอธิบาย    |
| -------- | --------- | ------- | ----------- |
| `--root` | `string`  | `.`     | รากโปรเจกต์ |
| `--json` | `boolean` | `false` | JSON output |

### ตัวอย่าง

```bash
ruvyxa trace /
ruvyxa trace /blog/hello-world

# JSON
ruvyxa trace /api/health --json

# Trace route ที่มี params
ruvyxa trace /blog/hello-world
ruvyxa trace /users/123
```

### ตัวอย่าง Output

```
━━━ Trace: /blog/hello-world ━━━━━━━━━━━━━━

  Route
  ─────
  Path:     /blog/[slug]
  Kind:     page
  File:     app/blog/[slug]/page.tsx
  Strategy: SSG (static params: 2)

  Layout Chain
  ─────────────
  Level 1: app/layout.tsx (root)
    Children: <slot/>
  Level 2: app/blog/layout.tsx (nested)
    Children: <slot/>

  Server Modules
  ──────────────
  ✓ app/blog/[slug]/page.tsx
  ✓ app/blog/layout.tsx
  ✓ app/layout.tsx
  ✓ app/actions.ts
  ✓ server/db.ts

  Client Modules
  ──────────────
  ✓ app/blog/[slug]/client-component.tsx
  ✓ components/counter.tsx

  Static Params
  ─────────────
  slug: hello-world
  slug: second-post
  slug: getting-started

  Rendering
  ─────────
  HTML:  /blog/hello-world.html     (prerendered)
  JSON:  /blog/hello-world.json     (static data)
  RSC:   /blog/hello-world.rsc      (RSC payload)

  Imports
  ───────
  app/blog/[slug]/page.tsx
    ├── @ruvyxa/react              (server-compatible)
    ├── ../../components/counter   (client)
    ├── ../../server/db            (server-only)
    └── ../../lib/utils            (shared)
```

### ตัวอย่าง Output — JSON

```json
{
  "route": {
    "path": "/blog/[slug]",
    "kind": "page",
    "file": "app/blog/[slug]/page.tsx",
    "strategy": "SSG",
    "staticParams": [{ "slug": "hello-world" }, { "slug": "second-post" }]
  },
  "layouts": [
    { "level": 1, "file": "app/layout.tsx", "type": "root" },
    { "level": 2, "file": "app/blog/layout.tsx", "type": "nested" }
  ],
  "modules": {
    "server": ["app/blog/[slug]/page.tsx", "app/actions.ts"],
    "client": ["app/blog/[slug]/client-component.tsx"]
  },
  "rendering": {
    "html": ".ruvyxa/prerender/blog/hello-world.html",
    "json": ".ruvyxa/prerender/blog/hello-world.json"
  }
}
```

---

## 11. `ruvyxa bench`

วัดประสิทธิภาพ — route discovery, validation, analysis, style collection

### Syntax

```bash
ruvyxa bench [options]
```

### Options

| Option      | Type       | Default | คำอธิบาย           |
| ----------- | ---------- | ------- | ------------------ |
| `--root`    | `string`   | `.`     | รากโปรเจกต์        |
| `--samples` | `number`   | `3`     | จำนวนครั้งที่ทดสอบ |
| `--json`    | `boolean`  | `false` | JSON output        |
| `--warmup`  | `number`   | `1`     | จำนวน warmup runs  |
| `--phases`  | `string[]` | all     | เฉพาะ phases       |

### ตัวอย่าง

```bash
npm run bench

# 10 samples
ruvyxa bench --samples 10

# JSON สำหรับเก็บสถิติ
ruvyxa bench --json --samples 5 > bench-results.json

# เฉพาะบาง phase
ruvyxa bench --phases discovery,validation

# รวม warmup
ruvyxa bench --samples 5 --warmup 2
```

### ตัวอย่าง Output

```
━━━ Benchmark ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Samples: 3
  Warmup:  1

  Phase                    Avg           Min           Max         p95
 ──────────────────────────────────────────────────────────────────────
  Route discovery          12.3 ms       11.8 ms       13.1 ms     13.0 ms
  Validation                8.7 ms        8.2 ms        9.1 ms      9.0 ms
  Analysis                 45.2 ms       43.1 ms       48.0 ms     47.5 ms
  Style collection         32.1 ms       30.5 ms       34.2 ms     33.9 ms
  Build (no cache)         2.3 s         2.1 s         2.5 s       2.5 s
  Image optimization       1.1 s         1.0 s         1.2 s       1.2 s

  Total (build)            3.5 s         3.3 s         3.7 s       3.7 s
```

### ตัวอย่าง Output — JSON

```json
{
  "samples": 3,
  "warmup": 1,
  "timestamp": "2026-07-29T10:00:00Z",
  "phases": {
    "route_discovery": {
      "avg": 12.3,
      "min": 11.8,
      "max": 13.1,
      "p95": 13.0,
      "unit": "ms"
    },
    "validation": {
      "avg": 8.7,
      "min": 8.2,
      "max": 9.1,
      "p95": 9.0,
      "unit": "ms"
    },
    "analysis": {
      "avg": 45.2,
      "min": 43.1,
      "max": 48.0,
      "p95": 47.5,
      "unit": "ms"
    },
    "total_build": {
      "avg": 3500,
      "min": 3300,
      "max": 3700,
      "p95": 3700,
      "unit": "ms"
    }
  }
}
```

### Benchmark Phases

| Phase        | วัดอะไร                                   | ขึ้นอยู่กับ        |
| ------------ | ----------------------------------------- | ------------------ |
| `discovery`  | Scaning app directory, parsing filesystem | จำนวน routes, ไฟล์ |
| `validation` | Route conflicts, boundary checks          | จำนวน modules      |
| `analysis`   | Import graph, dependency resolution       | จำนวน imports      |
| `style`      | CSS collection, PostCSS processing        | จำนวนไฟล์ CSS      |
| `build`      | Full compilation (ไม่รวม image)           | ขนาดโปรเจกต์       |
| `image`      | Image optimization pipeline               | จำนวนรูป           |

---

## 12. `ruvyxa test:parity`

เปรียบเทียบ dev vs production rendering — ทดสอบว่า output ตรงกัน

### Syntax

```bash
ruvyxa test:parity [options]
```

### Options

| Option       | Type              | Default | คำอธิบาย                     |
| ------------ | ----------------- | ------- | ---------------------------- |
| `--root`     | `string`          | `.`     | รากโปรเจกต์                  |
| `--runtime`  | `'node' \| 'bun'` | auto    | JS runtime                   |
| `--routes`   | `string[]`        | all     | เฉพาะ routes ที่ต้องการทดสอบ |
| `--timeout`  | `number`          | `30000` | Timeout (ms)                 |
| `--approve`  | `boolean`         | `false` | อัปเดต baseline เป็น current |
| `--baseline` | `string`          | -       | Path ถึง baseline directory  |
| `--strict`   | `boolean`         | `false` | เปรียบเทียบทุก byte          |
| `--json`     | `boolean`         | `false` | JSON output                  |

### ตัวอย่าง

```bash
# พื้นฐาน — ต้อง build ก่อน
npm run build && npm run test:parity

# เฉพาะบาง route
ruvyxa test:parity --routes /,/about

# Strict mode (byte-level)
ruvyxa test:parity --strict

# อัปเดต baseline
ruvyxa test:parity --approve

# JSON output สำหรับ CI
ruvyxa test:parity --json
```

### ตัวอย่าง Output — Pass

```
━━━ Parity Test ━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Mode:    html-comparison
  Routes:  4
  Strict:  false

  Routes
  ──────
  ✓ /                  dev = prod (200)    html: ✅  json: ✅
  ✓ /about             dev = prod (200)    html: ✅  json: ✅
  ✓ /blog              dev = prod (200)    html: ✅  json: ✅
  ✓ /blog/hello-world  dev = prod (200)    html: ✅  json: ✅

  ─────────────────────────────────────────────
  ✓ Result: 4/4 passed (1.2s)
  ✓ All routes match between dev and production
```

### ตัวอย่าง Output — Fail

```
━━━ Parity Test ━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Routes
  ──────
  ✓ /                  dev = prod (200)    ✅
  ✓ /about             dev = prod (200)    ✅
  ✗ /blog              dev (200) ≠ prod (404)
    Expected: prod returned 404, dev returned 200
    Difference: /blog page exists in dev but not in build
    Fix: Check route file app/blog/page.tsx

  ✗ /blog/hello-world  dev (200) ≠ prod (500)
    Expected: prod returned 500, dev returned 200
    Difference: Server error in production bundle
    Fix: Check server-side error logs

  ─────────────────────────────────────────────
  ✗ Result: 2/4 passed (0.8s)
  ✗ Parity check failed (exit code: 7)
```

### How It Works

1. `ruvyxa test:parity` เริ่ม dev server (ชั่วคราว) และ production server
2. ส่ง HTTP request ไปยังทั้งสอง server สำหรับแต่ละ route
3. เปรียบเทียบ status code, headers, HTML body
4. ถ้า `--strict`: เปรียบเทียบ HTML byte-level (รวม whitespace)
5. ถ้าไม่ strict: เปรียบเทียบเฉพาะ semantic content

### Parity Check Detail

| Check               | เวอร์ชัน strict | เวอร์ชันปกติ      |
| ------------------- | --------------- | ----------------- |
| Status code         | ตรงกันทุกประการ | ตรงกันทุกประการ   |
| Content-Type header | ตรงกัน          | ตรงกัน            |
| HTML structure      | ตรง byte-level  | semantic เท่านั้น |
| Whitespace          | ตรง             | ไม่สน             |
| React hydration IDs | ตรง             | ไม่สน             |
| Dynamic content     | ตรง             | ตรง               |
| Error pages         | รายงาน mismatch | รายงาน mismatch   |

---

## 13. `ruvyxa plugin create`

สร้าง scaffolding สำหรับแพ็กเกจ plugin ใหม่

### Syntax

```bash
ruvyxa plugin create <name> [options]
```

### Arguments

| Argument | Required | คำอธิบาย                                  |
| -------- | -------- | ----------------------------------------- |
| `<name>` | ✅       | ชื่อ plugin (lowercase, hyphen-separated) |

### Options

| Option           | Type                              | Default                | คำอธิบาย               |
| ---------------- | --------------------------------- | ---------------------- | ---------------------- |
| `--root`         | `string`                          | `.`                    | รากโปรเจกต์            |
| `--dir`          | `string`                          | `ruvyxa-plugin-{name}` | ที่อยู่ของ plugin      |
| `--template`     | `'basic' \| 'advanced' \| 'full'` | `'basic'`              | template               |
| `--with-tests`   | `boolean`                         | `true`                 | สร้าง test files       |
| `--with-docs`    | `boolean`                         | `true`                 | สร้าง README           |
| `--with-example` | `boolean`                         | `false`                | สร้างตัวอย่างการใช้งาน |

### ตัวอย่าง

```bash
# สร้าง plugin พื้นฐาน
ruvyxa plugin create my-redirects

# สร้าง plugin ขั้นสูง พร้อม tests และ docs
ruvyxa plugin create custom-auth --template advanced

# ระบุ directory
ruvyxa plugin create my-plugin --dir ./packages/my-plugin

# Full template พร้อมตัวอย่าง
ruvyxa plugin create full-featured --template full --with-example
```

### ตัวอย่าง Output

```
━━━ Plugin ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  status    created
  plugin    my-redirects
  package   ruvyxa-plugin-my-redirects
  path      ./ruvyxa-plugin-my-redirects
  template  basic

  Generated files:
  ─────────────────
  ✓ package.json        (dependencies ready)
  ✓ tsconfig.json       (TypeScript config)
  ✓ src/index.ts        (main plugin entry)
  ✓ src/hooks.ts        (hook definitions)
  ✓ src/types.ts        (TypeScript types)
  ✓ test/plugin.test.mjs (test file)
  ✓ README.md           (documentation)
  ✓ .gitignore
  ✓ CHANGELOG.md

  ─────────────────────────────────────────────
  Next steps:
  1. cd ruvyxa-plugin-my-redirects
  2. npm install
  3. Implement hook logic in src/index.ts
  4. npm test
  5. Publish to npm when ready

  ✓ Plugin my-redirects is ready to develop
```

### Template Structure

**Basic Template:**

```
ruvyxa-plugin-my-redirects/
├── src/
│   ├── index.ts       # ลงทะเบียน hooks
│   ├── hooks.ts       # hook implementations
│   └── types.ts       # TypeScript interfaces
├── test/
│   └── plugin.test.mjs
├── package.json
├── tsconfig.json
├── README.md
└── .gitignore
```

**Advanced Template:**

```
ruvyxa-plugin-my-plugin/
├── src/
│   ├── index.ts         # main entry
│   ├── hooks/           # hooks directory
│   │   ├── build.ts     # build hooks
│   │   ├── dev.ts       # dev hooks
│   │   └── server.ts    # server hooks
│   ├── utils/
│   │   ├── config.ts    # config parsing
│   │   └── validation.ts
│   └── types.ts
├── test/
│   ├── plugin.test.mjs
│   ├── hooks.test.mjs
│   └── integration.test.mjs
├── examples/
│   └── basic-usage.ts
├── package.json
├── tsconfig.json
├── README.md
└── .gitignore
```

**Full Template:**

```
ruvyxa-plugin-my-plugin/
├── src/
│   ├── index.ts
│   ├── hooks/
│   │   ├── index.ts
│   │   ├── build.ts
│   │   ├── dev.ts
│   │   ├── server.ts
│   │   └── config.ts
│   ├── utils/
│   │   ├── index.ts
│   │   ├── logger.ts
│   │   ├── validation.ts
│   │   └── helpers.ts
│   ├── middleware/
│   │   └── index.ts
│   └── types.ts
├── test/
│   ├── unit/
│   │   ├── hooks.test.mjs
│   │   └── utils.test.mjs
│   ├── integration/
│   │   └── plugin.test.mjs
│   └── fixtures/
│       ├── ruvyxa.config.ts
│       └── app/
├── examples/
│   ├── basic.ts
│   └── advanced.ts
├── docs/
│   ├── API.md
│   └── guide.md
├── package.json
├── tsconfig.json
├── README.md
├── LICENSE
├── CHANGELOG.md
└── .gitignore
```

### Generated `src/index.ts` (Basic)

```ts
import { definePlugin } from 'ruvyxa/plugin'
import type { RuvyxaHooks } from 'ruvyxa/plugin'

interface PluginOptions {
  // Define your plugin options here
  verbose?: boolean
}

export default definePlugin<PluginOptions>({
  name: 'my-redirects',
  version: '0.1.0',
  hooks: {
    // Build hooks
    'build:before': async (config, options) => {
      console.log(`[my-redirects] Build starting with options:`, options)
    },
    'build:after': async (result) => {
      console.log(`[my-redirects] Build completed in ${result.duration}ms`)
    },

    // Dev hooks
    'dev:start': async (server, options) => {
      console.log(`[my-redirects] Dev server started`)
    },

    // Server hooks
    'server:request': async (request, reply) => {
      // Intercept requests
    },
  } satisfies Partial<RuvyxaHooks>,
})
```

---

## Troubleshooting — ทุก Error Code และปัญหา CLI

### CLI Error Codes

| Exit Code | คำสั่ง              | ปัญหา                   | วิธีแก้                 |
| --------- | ------------------- | ----------------------- | ----------------------- |
| 1         | all                 | General error           | ตรวจ log                |
| 2         | check, build, dev   | Config validation error | `ruvyxa doctor`         |
| 3         | build, preview      | Build ล้มเหลว           | ดู build output         |
| 4         | check, analyze      | Boundary violation      | ย้าย server code        |
| 5         | check, routes       | Route conflict          | แก้ route path          |
| 6         | doctor              | มี warning/fix          | `ruvyxa doctor --fix`   |
| 7         | test:parity         | Rendering ต่างกัน       | ตรวจ parity fail output |
| 8         | dev, start, preview | Port ซ้ำ                | `--port 4000`           |
| 9         | plugin create       | สร้าง plugin ไม่ได้     | ใช้ชื่อ lower-hyphen    |
| 10        | clean               | ลบ cache ไม่ได้         | ลบ manual               |
| 11        | bench               | โดนขัดจังหวะ            | รันใหม่                 |

### ปัญหาทั่วไป

| ปัญหา                         | สาเหตุ                       | วิธีแก้                                 |
| ----------------------------- | ---------------------------- | --------------------------------------- |
| `ruvyxa: command not found`   | ไม่ได้ติดตั้ง global         | ใช้ `npx ruvyxa` หรือ `npm i -g ruvyxa` |
| `Address in use`              | port ซ้ำ                     | `--port 4000`                           |
| Build error แปลก ๆ            | cache เสีย                   | `ruvyxa clean && ruvyxa build`          |
| routes ไม่ขึ้น                | ไฟล์ไม่อยู่ใน `app/`         | ตรวจ `appDir` ใน config                 |
| `test:parity` ล้มเหลว         | build ยังไม่เสร็จ            | `ruvyxa build` ก่อน                     |
| Dev server ช้า                | file system watcher overload | `--no-hmr` หรือ `--runtime bun`         |
| `plugin create` error         | ชื่อไม่ถูกต้อง               | lowercase + hyphen (`my-plugin`)        |
| analyze ไม่เจอ error          | ต้อง build ก่อน              | `ruvyxa build` แล้ว `ruvyxa analyze`    |
| `--json` output pipe ไม่ทำงาน | auto-detect เป็น human       | ใช้ `--format json`                     |
| Port 443 ต้อง root            | privileged port              | ใช้ reverse proxy (nginx)               |
| HTTPS cert error              | cert path ไม่ถูก             | ตรวจ `--cert` `--key`                   |

### Debug Commands

```bash
# Verbose mode
ruvyxa build --verbose
ruvyxa dev --verbose

# Debug logging
RUVYXA_DEBUG=* ruvyxa dev
RUVYXA_DEBUG=route,build ruvyxa build
RUVYXA_DEBUG=hmr ruvyxa dev

# Profile build time
ruvyxa build --profile

# Bundle analysis
ruvyxa build --analyze

# Check specific
ruvyxa check --strict

# Doctor with fix
ruvyxa doctor --fix --generate-env-types

# Dry-run clean
ruvyxa clean --dry-run
```

---

## Complete Workflow Examples

### Development Workflow

```bash
# 1. เริ่ม dev
npm run dev

# 2. ดู routes
npm run routes

# 3. วิเคราะห์
npm run analyze

# 4. ตรวจสอบ
npm run check

# 5. แก้ปัญหา
npm run doctor
```

### Production Workflow

```bash
# 1. ตรวจสอบทุกอย่าง
ruvyxa check --strict

# 2. Clean
ruvyxa clean

# 3. Build
ruvyxa build

# 4. Preview
ruvyxa preview

# 5. Parity test (optional)
ruvyxa test:parity

# 6. Start
ruvyxa start
```

### CI Pipeline

```yaml
# .github/workflows/ci.yml
name: Ruvyxa CI
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - run: npm ci
      - run: npx ruvyxa check # exit code 2 = fail
      - run: npx ruvyxa analyze # exit code 4 = fail
      - run: npx ruvyxa doctor --json # exit code 6 = fail

  build:
    needs: validate
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - run: npm ci
      - run: npx ruvyxa build # exit code 3 = fail

  parity:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - run: npm ci
      - run: npx ruvyxa test:parity # exit code 7 = fail
```

---

## ลองทำดู

1. **Dev**
   - `ruvyxa dev` — เริ่ม dev server
   - แก้ไขไฟล์ → ดู HMR ทำงาน
   - `ruvyxa dev --open --port 4000`

2. **Routes & Trace**
   - `ruvyxa routes` — ดู route table
   - `ruvyxa trace /` — ดู root route ละเอียด

3. **Analyze & Doctor**
   - `ruvyxa analyze` — ดู diagnostics
   - `ruvyxa doctor` — ดู environment

4. **Build**
   - `ruvyxa clean && ruvyxa build`
   - `ruvyxa preview` — build + start

5. **Plugin**
   - `ruvyxa plugin create my-plugin`
   - `cd ruvyxa-plugin-my-plugin && npm test`

6. **Benchmark**
   - `ruvyxa bench --samples 5`
   - `ruvyxa bench --json | ConvertFrom-Json`

---

## สรุป

- 13 คำสั่ง CLI ครอบคลุมทุกงานพัฒนา
- `dev` / `build` / `start` = main development loop
- `check` / `analyze` = static analysis + boundary rules
- `doctor` = environment diagnostics
- `trace` / `routes` = route inspection
- `test:parity` = dev vs production consistency
- `bench` = performance measurement
- `plugin create` = scaffolding tool
- Exit codes 0-11 สำหรับ CI/CD integration
- Global options: `--root`, `--runtime`, `--help`, `--verbose`
