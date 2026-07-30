# ตั้งค่า Ruvyxa ด้วย `ruvyxa.config.ts`

`ruvyxa.config.ts` คือศูนย์กลางการตั้งค่าทั้งหมดของโปรเจกต์ Ruvyxa — ควบคุมทุกอย่างตั้งแต่ directory
structure, server, render strategy, build, cache, debug, CSS, image optimization, security,
middleware, plugins, ไปจนถึง deployment adapters

---

## ภาพรวมระบบ Config

### โครงสร้างไฟล์

```ts
// ruvyxa.config.ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  // ─── Directory ───
  appDir: 'app',
  outDir: '.ruvyxa',

  // ─── Server ───
  server: {
    host: 'localhost',
    port: 3000,
  },

  // ─── Site ───
  site: {
    url: 'https://example.com',
  },

  // ─── Build ───
  build: {
    minify: true,
    sourcemap: false,
    // ...
  },

  // ─── Other ───
  render: { strategy: 'ssr' },
  cache: { routeManifest: true },
  debug: { overlay: true },
  css: { entries: [] },
  image: { formats: ['webp', 'avif'] },
  security: { headers: true },
  middleware: { workers: 1 },
  plugins: [],
  adapter: undefined,
  adapterOptions: {},
})
```

### `defineConfig()` — Type Safety

```ts
// ruzyxa/config — type definitions
import { defineConfig } from 'ruvyxa/config'

// TypeScript signature
function defineConfig(config: RuvyxaConfig): RuvyxaConfig

// Full config type
interface RuvyxaConfig {
  appDir?: string
  outDir?: string
  server?: ServerConfig
  site?: SiteConfig
  build?: BuildConfig
  render?: RenderConfig
  cache?: CacheConfig
  debug?: DebugConfig
  css?: CSSConfig
  image?: ImageConfig
  security?: SecurityConfig
  middleware?: MiddlewareConfig
  plugins?: PluginConfig[]
  adapter?: AdapterType
  adapterOptions?: Record<string, unknown>
}
```

---

## ทุก Config Field — Full Reference

---

### `appDir`

| Field    | TypeScript Type | Rust Type | Default | Required | Validation                                          |
| -------- | --------------- | --------- | ------- | -------- | --------------------------------------------------- |
| `appDir` | `string`        | `String`  | `'app'` | ❌       | ต้องเป็น relative path, ห้ามเป็น absolute หรือ `..` |

```ts
appDir: 'app' // default — ใช้โฟลเดอร์ app/ ที่ราก
appDir: 'src/app' // ถ้าแยก src directory
appDir: 'src/pages' // custom directory
```

**Validation Rules:**

| Condition                                 | Error Code                  |
| ----------------------------------------- | --------------------------- |
| `appDir` ว่าง (`''`)                      | RUV1601                     |
| `appDir` เป็น absolute (`/home/user/app`) | RUV1601                     |
| `appDir` มี `..` (`../app`)               | RUV1601                     |
| `appDir` ชี้ไปที่ไม่มีอยู่                | Warning (สร้างให้อัตโนมัติ) |

---

### `outDir`

| Field    | TypeScript Type | Rust Type | Default     | Required | Validation                                |
| -------- | --------------- | --------- | ----------- | -------- | ----------------------------------------- |
| `outDir` | `string`        | `String`  | `'.ruvyxa'` | ❌       | ต้องเป็น relative path, ห้ามเป็น absolute |

```ts
outDir: '.ruvyxa' // default
outDir: 'dist' // output ไป dist/
outDir: 'build' // output ไป build/
```

**Warning:** โฟลเดอร์ `outDir` ถูกลบตอน `ruvyxa clean` — ห้ามเก็บไฟล์สำคัญไว้ที่นี่

**Validation Rules:**

| Condition              | Error Code |
| ---------------------- | ---------- |
| `outDir` ว่าง          | RUV1601    |
| `outDir` เป็น absolute | RUV1601    |

---

### `server`

```ts
interface ServerConfig {
  host?: string // default: 'localhost'
  port?: number // default: 3000
}
```

| Field  | TypeScript Type | Rust Type | Default       | Required | Validation                                       |
| ------ | --------------- | --------- | ------------- | -------- | ------------------------------------------------ |
| `host` | `string`        | `String`  | `'localhost'` | ❌       | valid hostname/IP                                |
| `port` | `number`        | `u16`     | `3000`        | ❌       | 1024-65535 (privileged ports ต้องใช้ sudo/admin) |

```ts
server: {
  host: '0.0.0.0',      // เปิดให้เข้าถึงจากเครือข่าย (production)
  port: 8080,            // ใช้ port 8080
}
```

**Special Values:**

| host          | ความหมาย                              |
| ------------- | ------------------------------------- |
| `'localhost'` | localhost เท่านั้น (ปลอดภัย, default) |
| `'0.0.0.0'`   | ทุก network interface (production)    |
| `'127.0.0.1'` | loopback เท่านั้น                     |
| `'::'`        | IPv6 ทุก interface                    |

**Validation Rules:**

| Condition                     | Error Code                |
| ----------------------------- | ------------------------- |
| `port` < 1024 (non-admin)     | Warning — อาจต้องใช้ sudo |
| `port` > 65535                | RUV1601                   |
| `port` = 0                    | RUV1601                   |
| `host` ไม่ใช่ IP/domain valid | RUV1602                   |

---

### `site`

```ts
interface SiteConfig {
  url?: string // auto-detect
  sitemap?: boolean | SitemapConfig // default: true
  robots?: boolean | RobotsConfig // default: true
}

interface SitemapConfig {
  exclude?: string[] // glob patterns
  additionalPaths?: string[]
  defaults?: {
    lastModified?: string // ISO 8601
    changeFrequency?: 'always' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly' | 'never'
    priority?: number // 0.0 - 1.0
  }
}

interface RobotsConfig {
  rules?: RobotRule[]
}

interface RobotRule {
  userAgent: string
  allow?: string
  disallow?: string
}
```

| Field     | TypeScript Type            | Rust Type       | Default     | Required | Validation                |
| --------- | -------------------------- | --------------- | ----------- | -------- | ------------------------- |
| `url`     | `string`                   | `String`        | auto-detect | ❌       | ต้องเป็น origin ที่ valid |
| `sitemap` | `boolean \| SitemapConfig` | `SitemapConfig` | `true`      | ❌       | -                         |
| `robots`  | `boolean \| RobotsConfig`  | `RobotsConfig`  | `true`      | ❌       | -                         |

```ts
site: {
  url: 'https://ruvyxa.dev',
  sitemap: {
    exclude: ['/draft/*', '/admin/*'],
    additionalPaths: ['/custom-landing', '/promo/summer'],
    defaults: {
      lastModified: '2026-07-29',
      changeFrequency: 'weekly',
      priority: 0.7,
    },
  },
  robots: {
    rules: [
      { userAgent: '*', allow: '/' },
      { userAgent: 'GPTBot', disallow: '/' },
      { userAgent: 'Googlebot', allow: '/public', disallow: '/private' },
    ],
  },
}
```

**Auto-detect URL Source Priority:**

| Priority    | Source                          | Example                |
| ----------- | ------------------------------- | ---------------------- |
| 1 (highest) | `RUVYXA_SITE_URL` env           | `https://myapp.com`    |
| 2           | `VERCEL_PROJECT_PRODUCTION_URL` | `myapp.vercel.app`     |
| 3           | `VERCEL_URL`                    | `myapp-git.vercel.app` |
| 4           | `NETLIFY` + `URL`               | `myapp.netlify.app`    |
| 5           | `CF_PAGES_URL`                  | `myapp.pages.dev`      |
| 6           | `RENDER_EXTERNAL_URL`           | `myapp.onrender.com`   |
| 7           | `RAILWAY_STATIC_URL`            | `myapp.up.railway.app` |

**Validation Rules:**

| Condition                                             | Error Code              |
| ----------------------------------------------------- | ----------------------- |
| `url` ไม่ใช่ origin ที่ถูกต้อง                        | Config Parse Error      |
| `url` มี path (`https://x.com/path`)                  | Warning (ไม่ควรมี path) |
| `sitemap.exclude` glob pattern ผิด                    | RUV1602                 |
| `robots.rules` userAgent ว่าง                         | RUV1602                 |
| `sitemap.defaults.priority` < 0.0 หรือ > 1.0          | RUV1602                 |
| `sitemap.defaults.changeFrequency` ไม่ใช่ค่าที่รองรับ | RUV1602                 |

---

### `build`

```ts
interface BuildConfig {
  minify?: boolean // default: true
  sourcemap?: boolean // default: false
  treeShaking?: boolean // default: true
  splitStrategy?: 'auto' | 'route' | 'vendor' | 'all' // default: 'auto'
  parallelism?: number // default: CPU cores
  jsxRuntime?: 'automatic' | 'classic' // default: 'automatic'
  esTarget?: 'es2020' | 'es2021' | 'es2022' | 'esnext' // default: 'es2022'
  emitChunkManifest?: boolean // default: false
  prebundleDependencies?: boolean // default: true
  prerenderCache?: boolean // default: true
}
```

| Field                   | TypeScript Type                                | Rust Type       | Default       | Validation           |
| ----------------------- | ---------------------------------------------- | --------------- | ------------- | -------------------- |
| `minify`                | `boolean`                                      | `bool`          | `true`        | -                    |
| `sourcemap`             | `boolean`                                      | `bool`          | `false`       | -                    |
| `treeShaking`           | `boolean`                                      | `bool`          | `true`        | -                    |
| `splitStrategy`         | `'auto' \| 'route' \| 'vendor' \| 'all'`       | `SplitStrategy` | `'auto'`      | ต้องเป็นค่าที่รองรับ |
| `parallelism`           | `number`                                       | `u32`           | CPU cores     | 1-64                 |
| `jsxRuntime`            | `'automatic' \| 'classic'`                     | `JSXRuntime`    | `'automatic'` | -                    |
| `esTarget`              | `'es2020' \| 'es2021' \| 'es2022' \| 'esnext'` | `ESTarget`      | `'es2022'`    | -                    |
| `emitChunkManifest`     | `boolean`                                      | `bool`          | `false`       | -                    |
| `prebundleDependencies` | `boolean`                                      | `bool`          | `true`        | -                    |
| `prerenderCache`        | `boolean`                                      | `bool`          | `true`        | -                    |

```ts
build: {
  minify: true,
  sourcemap: false,           // เปิดเฉพาะ dev ถ้าต้องการ debug
  treeShaking: true,          // ตัด dead code
  splitStrategy: 'route',     // split ตาม route
  parallelism: 4,             // 4 workers
  jsxRuntime: 'automatic',    // React 18+ JSX transform
  esTarget: 'es2022',         // target browser ปี 2022+
  emitChunkManifest: true,    // เขียน manifest.json
  prebundleDependencies: true, // prebundle node_modules
  prerenderCache: true,       // cache output ที่ prerender
}
```

**Split Strategy Detail:**

| Strategy   | การทำงาน                        | เหมาะกับ        |
| ---------- | ------------------------------- | --------------- |
| `'auto'`   | Ruvyxa ตัดสินใจเอง              | ทั่วไป          |
| `'route'`  | แยก chunk ตาม route             | แอปหลายหน้า     |
| `'vendor'` | แยก vendor/library ไว้ต่างหาก   | Production app  |
| `'all'`    | แต่ละ component เป็น chunk เล็ก | Micro-frontends |

**Validation Rules:**

| Condition                 | Error                       |
| ------------------------- | --------------------------- |
| `parallelism` < 1         | RUV1601                     |
| `parallelism` > 64        | RUV1602 (แนะนำ ≤ CPU cores) |
| `esTarget` ไม่รองรับ      | Config parse error          |
| `jsxRuntime` ไม่รองรับ    | Config parse error          |
| `splitStrategy` ไม่รองรับ | Config parse error          |

---

### `render`

```ts
interface RenderConfig {
  strategy?: RenderStrategy // default: 'ssr'
  revalidate?: number // default: undefined (ไม่ใช้ ISR)
}

type RenderStrategy = 'ssr' | 'ssg' | 'isr' | 'ppr' | 'csr'
```

| Field        | TypeScript Type  | Rust Type        | Default     | Validation           |
| ------------ | ---------------- | ---------------- | ----------- | -------------------- |
| `strategy`   | `RenderStrategy` | `RenderStrategy` | `'ssr'`     | ต้องเป็นค่าที่รองรับ |
| `revalidate` | `number`         | `Option<u32>`    | `undefined` | ≥ 1 วินาที           |

```ts
render: {
  strategy: 'ssr',       // default
  revalidate: 60,        // ISR: revalidate ทุก 60 วินาที
}
```

**Render Strategies Detail:**

| Strategy | เต็มชื่อ                        | การทำงาน                     | เหมาะกับ               |
| -------- | ------------------------------- | ---------------------------- | ---------------------- |
| `'ssr'`  | Server-Side Rendering           | เรนเดอร์ทุก request          | ต้องข้อมูลสดตลอด       |
| `'ssg'`  | Static Site Generation          | เรนเดอร์ตอน build            | content ไม่เปลี่ยนบ่อย |
| `'isr'`  | Incremental Static Regeneration | SSG แต่ revalidate ตามเวลา   | content friendly       |
| `'ppr'`  | Partial Prerendering            | static shell + dynamic slots | speed + dynamic        |
| `'csr'`  | Client-Side Rendering           | client เรนเดอร์เอง           | dashboard, SPA         |

**Override ต่อ route:** แต่ละ `page.tsx` สามารถ export `render` strategy ของตัวเอง:

```tsx
// app/blog/[slug]/page.tsx — override strategy
export const render = {
  strategy: 'isr' as const,
  revalidate: 3600, // 1 ชั่วโมง
}
```

**Validation Rules:**

| Condition                     | Error                     |
| ----------------------------- | ------------------------- |
| `strategy` ไม่ใช่ค่าที่รองรับ | Config parse error        |
| `revalidate` < 1              | RUV1601                   |
| `revalidate` ใช้กับ `'csr'`   | Warning (CSR ไม่มี cache) |

---

### `cache`

```ts
interface CacheConfig {
  routeManifest?: boolean // default: true
  css?: boolean // default: true
  buildDir?: string // default: '.ruvyxa/cache'
  image?: boolean // default: true
  mdx?: boolean // default: true
}
```

| Field           | TypeScript Type | Rust Type | Default           | Validation    |
| --------------- | --------------- | --------- | ----------------- | ------------- |
| `routeManifest` | `boolean`       | `bool`    | `true`            | -             |
| `css`           | `boolean`       | `bool`    | `true`            | -             |
| `buildDir`      | `string`        | `String`  | `'.ruvyxa/cache'` | relative path |
| `image`         | `boolean`       | `bool`    | `true`            | -             |
| `mdx`           | `boolean`       | `bool`    | `true`            | -             |

```ts
cache: {
  routeManifest: true,
  css: true,
  buildDir: '.ruvyxa/cache',
  image: true,
  mdx: true,            // 512-entry LRU cache สำหรับ MDX
}
```

**Env Var Override:**

```bash
# RUVYXA_BUILD_CACHE_DIR มี priority สูงกว่า config field
RUVYXA_BUILD_CACHE_DIR=/tmp/ruvyxa-cache npm run build
```

**Validation Rules:**

| Condition           | Error   |
| ------------------- | ------- |
| `buildDir` absolute | RUV1601 |
| `buildDir` ว่าง     | RUV1601 |

---

### `debug`

```ts
interface DebugConfig {
  overlay?: boolean // default: true
  traces?: boolean // default: false
  sourceMap?: boolean // default: auto (dev = true, prod = false)
  profiler?: boolean // default: false
}
```

| Field       | TypeScript Type | Rust Type | Default | Validation |
| ----------- | --------------- | --------- | ------- | ---------- |
| `overlay`   | `boolean`       | `bool`    | `true`  | -          |
| `traces`    | `boolean`       | `bool`    | `false` | -          |
| `sourceMap` | `boolean`       | `bool`    | auto    | -          |
| `profiler`  | `boolean`       | `bool`    | `false` | -          |

```ts
debug: {
  overlay: true,        // แสดง error ในเบราว์เซอร์
  traces: false,        // ปิด debug traces ใน production
  sourceMap: true,      // เปิด source maps (dev)
  profiler: false,      // ปิด profiler
}
```

**Debug Features:**

| Feature                  | `overlay: true` | `traces: true` |
| ------------------------ | --------------- | -------------- |
| Error overlay in browser | ✅              | -              |
| Console stack traces     | -               | ✅             |
| Route resolution log     | -               | ✅             |
| SSR timing               | -               | ✅             |
| Build performance        | -               | ✅             |

**Env Var Override:**

```bash
# เปิด traces เฉพาะตอน debug
RUVYXA_DEBUG=1 npm run dev
# เปิดเฉพาะบาง module
RUVYXA_DEBUG=route,image npm run dev
# ปิดทั้งหมด
RUVYXA_DEBUG=0 npm run dev
```

---

### `css`

```ts
interface CSSConfig {
  entries?: string[] // default: []
  modules?: boolean // default: true (CSS Modules)
  postcss?: boolean // default: true
  lightningcss?: boolean // default: true (ใช้ Lightning CSS)
  tailwind?: boolean // default: auto-detected
}
```

| Field          | TypeScript Type | Rust Type     | Default | Validation                    |
| -------------- | --------------- | ------------- | ------- | ----------------------------- |
| `entries`      | `string[]`      | `Vec<String>` | `[]`    | relative paths, ห้าม absolute |
| `modules`      | `boolean`       | `bool`        | `true`  | -                             |
| `postcss`      | `boolean`       | `bool`        | `true`  | -                             |
| `lightningcss` | `boolean`       | `bool`        | `true`  | -                             |
| `tailwind`     | `boolean`       | `bool`        | auto    | auto-detect จาก dependencies  |

```ts
css: {
  entries: [
    'src/styles/global.css',
    'src/styles/fonts.css',
    'node_modules/highlight.js/styles/github-dark.css',
  ],
  modules: true,        // เปิด CSS Modules (*.module.css)
  postcss: true,        // ใช้ PostCSS (autoprefixer)
  lightningcss: false,  // ใช้ traditional CSS processor
}
```

**`entries` Behavior:** ไฟล์ใน `entries` จะถูก inject ในทุกหน้าโดยอัตโนมัติ — ไม่ต้อง import ใน
layout

**Validation Rules:**

| Condition                      | Error                      |
| ------------------------------ | -------------------------- |
| `entries[]` เป็น absolute path | RUV1601                    |
| `entries[]` file ไม่มีอยู่     | Warning (ignore ถ้าไม่เจอ) |
| `entries[]` path มี `..`       | RUV1602                    |

---

### `image`

```ts
interface ImageConfig {
  formats?: ('webp' | 'avif')[] // default: ['webp', 'avif']
  sizes?: number[] // default: [640, 1280, 1920]
  quality?: number // default: 80 (1-100)
  avifQuality?: number // default: 65 (1-100)
  lazy?: boolean // default: true
  placeholder?: 'blur' | 'empty' // default: 'empty'
  encoder?: {
    jpeg?: 'mozjpeg' | 'guetzli' | 'libjpeg' // default: 'mozjpeg'
    png?: 'oxipng' | 'pngquant' | 'libpng' // default: 'oxipng'
    jpegQuality?: number // default: 80
    pngQuality?: number // default: 85
    pngCompressionLevel?: number // default: 3
  }
  cache?: {
    enabled?: boolean // default: true
    directory?: string // default: '.ruvyxa/cache/images'
  }
}
```

| Field                         | TypeScript Type                       | Rust Type          | Default                  | Validation               |
| ----------------------------- | ------------------------------------- | ------------------ | ------------------------ | ------------------------ |
| `formats`                     | `('webp' \| 'avif')[]`                | `Vec<ImageFormat>` | `['webp', 'avif']`       | ต้องมี ≥ 1               |
| `sizes`                       | `number[]`                            | `Vec<u32>`         | `[640, 1280, 1920]`      | แต่ละค่า > 0 และ < 10000 |
| `quality`                     | `number`                              | `u8`               | `80`                     | 1-100                    |
| `avifQuality`                 | `number`                              | `u8`               | `65`                     | 1-100                    |
| `lazy`                        | `boolean`                             | `bool`             | `true`                   | -                        |
| `placeholder`                 | `'blur' \| 'empty'`                   | `PlaceholderMode`  | `'empty'`                | -                        |
| `encoder.jpeg`                | `'mozjpeg' \| 'guetzli' \| 'libjpeg'` | `JpegEncoder`      | `'mozjpeg'`              | -                        |
| `encoder.png`                 | `'oxipng' \| 'pngquant' \| 'libpng'`  | `PngEncoder`       | `'oxipng'`               | -                        |
| `encoder.jpegQuality`         | `number`                              | `u8`               | `80`                     | 1-100                    |
| `encoder.pngQuality`          | `number`                              | `u8`               | `85`                     | 1-100 (pngquant)         |
| `encoder.pngCompressionLevel` | `number`                              | `u8`               | `3`                      | 0-6 (oxipng)             |
| `cache.enabled`               | `boolean`                             | `bool`             | `true`                   | -                        |
| `cache.directory`             | `string`                              | `String`           | `'.ruvyxa/cache/images'` | relative                 |

```ts
image: {
  formats: ['webp', 'avif'],
  sizes: [320, 640, 960, 1280, 1920, 2560],
  quality: 85,
  avifQuality: 70,
  lazy: true,
  placeholder: 'blur',
  encoder: {
    jpeg: 'mozjpeg',
    png: 'oxipng',
    jpegQuality: 80,
    pngCompressionLevel: 4,
  },
  cache: {
    enabled: true,
    directory: '.ruvyxa/cache/images',
  },
}
```

**Validation Rules:**

| Condition                                  | Error              |
| ------------------------------------------ | ------------------ |
| `quality` < 1 หรือ > 100                   | RUV1601            |
| `avifQuality` < 1 หรือ > 100               | RUV1601            |
| `sizes[]` < 1 หรือ > 10000                 | RUV1601            |
| `formats` empty array                      | RUV1601            |
| `encoder.pngCompressionLevel` < 0 หรือ > 6 | RUV1601            |
| `encoder.jpeg` ไม่ใช่ค่าที่รองรับ          | Config parse error |
| `encoder.png` ไม่ใช่ค่าที่รองรับ           | Config parse error |

---

### `security`

```ts
interface SecurityConfig {
  actionLimit?: number // default: 1_048_576 (1MB)
  apiLimit?: number // default: 5_242_880 (5MB)
  pluginLimit?: number // default: 5_242_880 (5MB)
  actionRateLimit?: {
    max?: number // default: undefined (ไม่จำกัด)
    window?: number // default: undefined (วินาที)
    key?: 'ip' | 'user' | 'route' // default: 'ip'
  }
  sameOrigin?: boolean // default: true
  fetchMeta?: boolean // default: false
  trustedProxyIps?: string[] // default: []
  headers?: boolean // default: true
  csrf?: boolean // default: true
  xssProtection?: boolean // default: true
  maxBodySize?: number // default: 10_485_760 (10MB)
}
```

| Field                    | TypeScript Type             | Rust Type      | Default      | Validation   |
| ------------------------ | --------------------------- | -------------- | ------------ | ------------ |
| `actionLimit`            | `number`                    | `u64`          | `1_048_576`  | ≥ 1, ≤ 10MB  |
| `apiLimit`               | `number`                    | `u64`          | `5_242_880`  | ≥ 1, ≤ 50MB  |
| `pluginLimit`            | `number`                    | `u64`          | `5_242_880`  | ≥ 1, ≤ 50MB  |
| `actionRateLimit.max`    | `number`                    | `u64`          | undefined    | ≥ 1          |
| `actionRateLimit.window` | `number`                    | `u64`          | undefined    | ≥ 1 (วินาที) |
| `actionRateLimit.key`    | `'ip' \| 'user' \| 'route'` | `RateLimitKey` | `'ip'`       | -            |
| `sameOrigin`             | `boolean`                   | `bool`         | `true`       | -            |
| `fetchMeta`              | `boolean`                   | `bool`         | `false`      | -            |
| `trustedProxyIps`        | `string[]`                  | `Vec<String>`  | `[]`         | IP หรือ CIDR |
| `headers`                | `boolean`                   | `bool`         | `true`       | -            |
| `csrf`                   | `boolean`                   | `bool`         | `true`       | -            |
| `xssProtection`          | `boolean`                   | `bool`         | `true`       | -            |
| `maxBodySize`            | `number`                    | `u64`          | `10_485_760` | ≥ 1, ≤ 100MB |

```ts
security: {
  actionLimit: 2_097_152,           // 2MB สำหรับ server actions
  apiLimit: 10_485_760,             // 10MB สำหรับ API routes
  pluginLimit: 1_048_576,           // 1MB สำหรับ plugin response
  actionRateLimit: {
    max: 100,                        // 100 requests
    window: 60,                      // ต่อ 60 วินาที
    key: 'ip',                       // rate limit โดย IP
  },
  sameOrigin: true,                  // action ต้องมาจาก origin เดียวกัน
  fetchMeta: true,                   // ตรวจ Sec-Fetch-* headers
  trustedProxyIps: [
    '10.0.0.1',                      // address ตรงตัว
    '172.16.0.0/12',                 // CIDR range
    '2001:db8::/32',                 // IPv6 ก็ได้
  ],
  headers: true,                     // เพิ่ม security headers อัตโนมัติ
  csrf: true,                        // ป้องกัน CSRF
  xssProtection: true,               // XSS filter
  maxBodySize: 20_971_520,           // 20MB max body
}
```

**`trustedProxyIps` — รูปแบบที่รับได้:**

รายการที่ไม่มี `/` ถือเป็น host route (`/32` สำหรับ IPv4, `/128` สำหรับ IPv6) bit ที่ต่ำกว่า prefix
จะถูก mask ทิ้ง ดังนั้น `10.1.2.3/8` กับ `10.0.0.0/8` หมายถึง range เดียวกัน range แบบ IPv4 จะ match
peer แบบ IPv4-mapped (`::ffff:10.0.0.9`) ด้วย ซึ่งเป็นรูปแบบที่ dual-stack listener รายงาน client
IPv4

loopback เชื่อถือได้เสมอ ไม่จำเป็นต้องใส่ในรายการ

ค่าที่ผิดรูปจะได้ error:
`RUV1602 config field 'security.trustedProxyIps' contains invalid IP or CIDR range 'xyz'`

**Security Headers ที่ Ruvyxa เพิ่มอัตโนมัติ:**

| Header                      | Value                             | เมื่อ `headers: true`          |
| --------------------------- | --------------------------------- | ------------------------------ |
| `X-Content-Type-Options`    | `nosniff`                         | ✅                             |
| `X-Frame-Options`           | `SAMEORIGIN`                      | ✅                             |
| `X-XSS-Protection`          | `1; mode=block`                   | ✅ (ถ้า `xssProtection: true`) |
| `Referrer-Policy`           | `strict-origin-when-cross-origin` | ✅                             |
| `Permissions-Policy`        | ตาม config                        | ✅                             |
| `Strict-Transport-Security` | `max-age=31536000`                | ✅ (production)                |
| `Content-Security-Policy`   | กำหนดเองผ่าน middleware           | ❌ (ต้องตั้งค่าเอง)            |

**Validation Rules (Rust):**

| Condition                              | Error Code |
| -------------------------------------- | ---------- |
| `actionLimit` < 1                      | RUV1601    |
| `actionLimit` > 10_485_760 (10MB)      | RUV1602    |
| `apiLimit` < 1                         | RUV1601    |
| `apiLimit` > 52_428_800 (50MB)         | RUV1602    |
| `pluginLimit` < 1                      | RUV1601    |
| `pluginLimit` > 52_428_800 (50MB)      | RUV1602    |
| `actionRateLimit.max` < 1 (ถ้าตั้ง)    | RUV1601    |
| `actionRateLimit.window` < 1 (ถ้าตั้ง) | RUV1601    |
| `trustedProxyIps[]` ไม่ใช่ IP/CIDR     | RUV1602    |
| `maxBodySize` < 1                      | RUV1601    |
| `maxBodySize` > 104_857_600 (100MB)    | RUV1602    |

---

### `middleware`

```ts
interface MiddlewareConfig {
  builtin?: {
    cors?: CorsConfig | boolean // default: false (ปิด)
    timing?: boolean // default: true
    log?: boolean // default: true
    rate?: RateConfig | boolean // default: false (ปิด)
    headers?: Record<string, string> // default: {}
  }
  workers?: number // default: 1 (1-8)
  timeoutMs?: number // default: 30000 (30s)
}

interface CorsConfig {
  origins?: string[] // allowed origins
  methods?: string[] // allowed HTTP methods
  headers?: string[] // allowed headers
  credentials?: boolean // allow cookies
  maxAge?: number // preflight cache (seconds)
}

interface RateConfig {
  max: number // max requests
  window: number // time window (seconds)
  key?: 'ip' | 'user' | 'route' // rate limit key
}
```

| Field             | TypeScript Type          | Rust Type            | Default | Validation |
| ----------------- | ------------------------ | -------------------- | ------- | ---------- |
| `builtin.cors`    | `CorsConfig \| boolean`  | `Option<CorsConfig>` | `false` | -          |
| `builtin.timing`  | `boolean`                | `bool`               | `true`  | -          |
| `builtin.log`     | `boolean`                | `bool`               | `true`  | -          |
| `builtin.rate`    | `RateConfig \| boolean`  | `Option<RateConfig>` | `false` | -          |
| `builtin.headers` | `Record<string, string>` | `HashMap`            | `{}`    | -          |
| `workers`         | `number`                 | `u8`                 | `1`     | 1-8        |
| `timeoutMs`       | `number`                 | `u64`                | `30000` | 1-300000   |

```ts
middleware: {
  builtin: {
    cors: {
      origins: ['https://example.com', 'https://api.example.com'],
      methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
      headers: ['Content-Type', 'Authorization', 'X-Request-ID'],
      credentials: true,
      maxAge: 86400,           // cache preflight 1 วัน
    },
    timing: true,               // X-Response-Time header
    log: true,                  // Request logging
    rate: {                     // Rate limiting
      max: 100,
      window: 60,
      key: 'ip',
    },
    headers: {
      'X-Custom-Header': 'value',
      'X-Api-Version': '1.0',
    },
  },
  workers: 2,                   // 2 worker processes
  timeoutMs: 15000,             // 15 วินาที
}
```

**Built-in Middleware Reference:**

| Middleware | ฟังก์ชัน                                         | Default | Headers                        |
| ---------- | ------------------------------------------------ | ------- | ------------------------------ |
| `cors`     | Cross-Origin Resource Sharing (CORS) + preflight | ปิด     | `Access-Control-*`             |
| `timing`   | Response time measurement                        | เปิด    | `X-Response-Time`              |
| `log`      | Request logging (x-request-id)                   | เปิด    | `X-Request-ID`                 |
| `rate`     | Rate limiting (token bucket algorithm)           | ปิด     | `X-RateLimit-*`, `Retry-After` |
| `headers`  | Custom response headers                          | ปิด     | ตามกำหนด                       |

**Validation Rules:**

| Condition                     | Error Code             |
| ----------------------------- | ---------------------- |
| `workers` < 1                 | RUV1601                |
| `workers` > 8                 | RUV1602                |
| `timeoutMs` < 1               | RUV1601                |
| `timeoutMs` > 300_000 (5 min) | RUV1602                |
| `cors.origins` empty array    | RUV1601 (ถ้าเปิด cors) |
| `rate.max` < 1 (ถ้าเปิด)      | RUV1601                |
| `rate.window` < 1 (ถ้าเปิด)   | RUV1601                |

---

### `plugins`

```ts
interface PluginConfig {
  name: string
  options?: Record<string, unknown>
  enabled?: boolean // default: true
}
```

| Field     | TypeScript Type  | Rust Type           | Default | Validation       |
| --------- | ---------------- | ------------------- | ------- | ---------------- |
| `plugin`  | `PluginConfig[]` | `Vec<PluginConfig>` | `[]`    | -                |
| `name`    | `string`         | `String`            | -       | required, unique |
| `options` | `object`         | `HashMap`           | `{}`    | -                |
| `enabled` | `boolean`        | `bool`              | `true`  | -                |

**All Built-in Plugins:**

| Plugin Name     | ฟังก์ชัน                  | Options                                           |
| --------------- | ------------------------- | ------------------------------------------------- |
| `redirects`     | URL redirect rules        | `redirects: [{ source, destination, permanent }]` |
| `headers`       | Custom response headers   | `headers: [{ source, headers }]`                  |
| `observability` | Health check, metrics     | `endpoint: '/api/health'`                         |
| `pwa`           | Progressive Web App       | `manifest: { name, short_name, icons, ... }`      |
| `fonts`         | Google Fonts optimization | `families: ['Inter', 'Noto Sans Thai']`           |
| `sitemap`       | Sitemap generation        | `exclude: [...], defaults: {...}`                 |
| `robots`        | Robots.txt generation     | `rules: [...]`                                    |
| `feed`          | RSS/Atom feed             | `type: 'rss' \| 'atom', title, description`       |
| `search-index`  | Search index JSON         | `exclude: [...], fields: [...]`                   |
| `open-api`      | OpenAPI/Swagger           | `title, version, description`                     |
| `requireEnv`    | Required env vars check   | `vars: [...], mode: 'strict' \| 'warn'`           |
| `compress`      | Response compression      | `algorithm: 'gzip' \| 'brotli', level: 6`         |
| `web-vitals`    | Web Vitals analytics      | `endpoint: '/api/vitals'`                         |
| `image-cdn`     | External image CDN        | `provider: 'cloudinary' \| 'imgix', options`      |
| `analytics`     | Analytics integration     | `provider: 'ga4' \| 'plausible' \| 'umami'`       |
| `i18n`          | Internationalization      | `locales: [...], defaultLocale: 'th'`             |

```ts
plugins: [
  // Redirects
  {
    name: 'redirects',
    options: {
      redirects: [
        { source: '/old-page', destination: '/new-page', permanent: true },
        { source: '/blog/:slug', destination: '/articles/:slug', permanent: false },
      ],
    },
  },

  // Custom headers
  {
    name: 'headers',
    options: {
      headers: [
        {
          source: '/(.*)',
          headers: [
            { key: 'X-Frame-Options', value: 'DENY' },
            { key: 'X-Content-Type-Options', value: 'nosniff' },
          ],
        },
      ],
    },
  },

  // Observability
  {
    name: 'observability',
    options: {
      endpoint: '/api/health',
      metrics: true,
      tracing: false,
    },
  },

  // PWA
  {
    name: 'pwa',
    options: {
      manifest: {
        name: 'My App',
        short_name: 'App',
        description: 'แอปพลิเคชันของฉัน',
        start_url: '/',
        display: 'standalone',
        background_color: '#ffffff',
        theme_color: '#000000',
        icons: [
          { src: '/icons/icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: '/icons/icon-512.png', sizes: '512x512', type: 'image/png' },
        ],
      },
      serviceWorker: {
        enabled: true,
        cacheStrategy: 'network-first',
      },
    },
  },

  // Fonts
  {
    name: 'fonts',
    options: {
      families: [
        'Inter:wght@400;500;600;700',
        'Noto Sans Thai:wght@300;400;500;700',
        'IBM Plex Sans Thai:wght@400;700',
      ],
      display: 'swap',
      preload: true,
    },
  },

  // Environment validation
  {
    name: 'requireEnv',
    options: {
      vars: ['DATABASE_URL', 'AUTH_SECRET', 'RUVYXA_PUBLIC_API_URL'],
      mode: 'strict',
    },
  },

  // Compression
  {
    name: 'compress',
    options: {
      algorithm: 'brotli',
      level: 6,
      threshold: 1024, // compress เฉพาะ > 1KB
    },
  },

  // i18n
  {
    name: 'i18n',
    options: {
      locales: ['th', 'en', 'zh', 'ja'],
      defaultLocale: 'th',
      detectFromPath: true,
      detectFromBrowser: true,
    },
  },
]
```

---

### `adapter`

```ts
type AdapterType =
  | 'node' // Node.js (default)
  | 'bun' // Bun runtime
  | 'vercel' // Vercel Functions
  | 'netlify' // Netlify Functions
  | 'cloudflare' // Cloudflare Pages/Workers
  | 'static' // Static export (HTML files)
  | 'railway' // Railway
  | 'render' // Render
  | 'firebase' // Firebase Functions
  | 'aws' // AWS Lambda
  | 'fly' // Fly.io
  | 'koyeb' // Koyeb
  | undefined // auto-detect
```

| Field            | TypeScript Type           | Rust Type         | Default     | Validation            |
| ---------------- | ------------------------- | ----------------- | ----------- | --------------------- |
| `adapter`        | `AdapterType`             | `Option<Adapter>` | auto-detect | ต้องเป็นชื่อที่รองรับ |
| `adapterOptions` | `Record<string, unknown>` | `HashMap`         | `{}`        | ขึ้นกับ adapter       |

```ts
adapter: 'node',
adapterOptions: {
  // Node.js specific
  cluster: true,
  workers: 4,
}

// Vercel
adapter: 'vercel',
adapterOptions: {
  regions: ['iad1', 'hkg1', 'sin1'],
  imageOptimization: true,
}

// Cloudflare
adapter: 'cloudflare',
adapterOptions: {
  entryPoint: 'cloudflare-entry.ts',
  routes: [{ pattern: '*.example.com/*', zone: 'example.com' }],
}

// Static
adapter: 'static',
adapterOptions: {
  trailingSlash: true,
  cleanUrls: false,
}

// AWS
adapter: 'aws',
adapterOptions: {
  region: 'ap-southeast-1',
  memory: 512,    // MB
  timeout: 30,    // seconds
}
```

**Auto-detect Logic:**

```rust
fn auto_detect_adapter() -> Option<Adapter> {
    if env!("VERCEL").is_ok() { return Some(Adapter::Vercel); }
    if env!("NETLIFY").is_ok() { return Some(Adapter::Netlify); }
    if env!("CF_PAGES").is_ok() { return Some(Adapter::Cloudflare); }
    if env!("RAILWAY_PROJECT_ID").is_ok() { return Some(Adapter::Railway); }
    if env!("RENDER").is_ok() { return Some(Adapter::Render); }
    if env!("FLY_APP_NAME").is_ok() { return Some(Adapter::Fly); }
    Some(Adapter::Node)  // fallback
}
```

**Env Var Override:**

```bash
RUVYXA_ADAPTER=vercel npm run build
```

---

## Validation Rules — Complete Reference (Rust)

### รหัส Error RUV1600-RUV1699

| Code    | เงื่อนไข                       | ฟิลด์        | วิธีแก้              |
| ------- | ------------------------------ | ------------ | -------------------- |
| RUV1601 | ค่าไม่ถูกต้อง (invalid)        | หลายฟิลด์    | ตรวจค่าตามที่กำหนด   |
| RUV1602 | ค่าเกินขีดจำกัด (out of range) | หลายฟิลด์    | ปรับค่าให้อยู่ในช่วง |
| RUV1603 | ฟิลด์ไม่รู้จัก (unknown field) | ทั้ง config  | ตรวจ camelCase       |
| RUV1604 | ฟิลด์ซ้ำ (duplicate)           | plugins.name | เปลี่ยนชื่อ plugin   |
| RUV1605 | ชนิดข้อมูลผิด (type mismatch)  | ทุกฟิลด์     | ใช้ชนิดที่ถูกต้อง    |
| RUV1606 | ฟิลด์ required ขาดหาย          | -            | เพิ่มฟิลด์ที่จำเป็น  |

### Validation Matrix

| Config Field                         | RUV1601     | RUV1602          | RUV1605 | หมายเหตุ               |
| ------------------------------------ | ----------- | ---------------- | ------- | ---------------------- |
| `appDir` empty/absolute              | ✅          | -                | ✅      | relative path required |
| `outDir` empty/absolute              | ✅          | -                | ✅      | relative path required |
| `server.port` 0                      | ✅          | ✅ (ถ้า > 65535) | ✅      | 1024-65535             |
| `server.host` invalid                | -           | ✅               | ✅      | valid hostname/IP      |
| `site.url` invalid                   | -           | ✅               | ✅      | origin เท่านั้น        |
| `site.sitemap.defaults.priority`     | -           | ✅ (0-1)         | ✅      | float                  |
| `build.parallelism` 0                | ✅          | ✅ (ถ้า > 64)    | ✅      | 1-64                   |
| `build.splitStrategy` invalid        | ✅          | -                | ✅      | auto/route/vendor/all  |
| `build.jsxRuntime` invalid           | ✅          | -                | ✅      | automatic/classic      |
| `build.esTarget` invalid             | ✅          | -                | ✅      | es2020-esnext          |
| `security.actionLimit` 0             | ✅          | ✅ (>10MB)       | ✅      | 1B-10MB                |
| `security.apiLimit` 0                | ✅          | ✅ (>50MB)       | ✅      | 1B-50MB                |
| `security.pluginLimit` 0             | ✅          | ✅ (>50MB)       | ✅      | 1B-50MB                |
| `security.maxBodySize` 0             | ✅          | ✅ (>100MB)      | ✅      | 1B-100MB               |
| `security.trustedProxyIps[]` invalid | -           | ✅               | ✅      | valid IP/CIDR          |
| `security.actionRateLimit.max` 0     | ✅          | -                | ✅      | ≥ 1                    |
| `security.actionRateLimit.window` 0  | ✅          | -                | ✅      | ≥ 1                    |
| `middleware.workers` 0               | ✅          | ✅ (>8)          | ✅      | 1-8                    |
| `middleware.timeoutMs` 0             | ✅          | ✅ (>300s)       | ✅      | 1ms-300s               |
| `image.quality` out of range         | ✅ (0/100+) | -                | ✅      | 1-100                  |
| `image.avifQuality` out of range     | ✅ (0/100+) | -                | ✅      | 1-100                  |
| `image.sizes[]` 0                    | ✅          | ✅ (>10000)      | ✅      | 1-9999                 |
| `image.formats` empty                | ✅          | -                | ✅      | ≥ 1 format             |
| `css.entries[]` absolute             | ✅          | -                | ✅      | relative path          |
| `cache.buildDir` absolute            | ✅          | -                | ✅      | relative path          |
| `adapter` unknown                    | ✅          | -                | ✅      | ดู AdapterType         |
| `plugins[].name` empty/duplicate     | ✅          | -                | ✅      | unique, non-empty      |

---

## Config Examples — Full Scenarios

### 1. Minimal Config

```ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  site: { url: 'https://myapp.com' },
})
```

### 2. Static Blog (MDX + SSG)

```ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  appDir: 'app',
  outDir: '.ruvyxa',
  site: {
    url: 'https://blog.example.com',
    sitemap: {
      defaults: {
        changeFrequency: 'daily',
        priority: 0.7,
      },
    },
  },
  render: {
    strategy: 'ssg',
  },
  image: {
    formats: ['webp', 'avif'],
    sizes: [640, 960, 1280, 1920],
    quality: 85,
    avifQuality: 70,
    lazy: true,
    encoder: {
      jpeg: 'mozjpeg',
      png: 'oxipng',
    },
  },
  plugins: [{ name: 'feed', options: { type: 'rss', title: 'My Blog' } }, { name: 'search-index' }],
})
```

### 3. E-commerce (SSR + ISR + Security)

```ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  appDir: 'app',
  server: {
    host: '0.0.0.0',
    port: 3000,
  },
  site: {
    url: 'https://shop.example.com',
    sitemap: {
      exclude: ['/cart', '/checkout', '/account/*'],
      defaults: { changeFrequency: 'hourly', priority: 0.5 },
    },
    robots: {
      rules: [
        { userAgent: '*', allow: '/' },
        { userAgent: 'GPTBot', disallow: '/' },
      ],
    },
  },
  build: {
    minify: true,
    sourcemap: false,
    treeShaking: true,
    splitStrategy: 'vendor',
    prebundleDependencies: true,
  },
  render: {
    strategy: 'isr',
    revalidate: 300, // 5 นาที
  },
  security: {
    actionLimit: 2_097_152, // 2MB
    apiLimit: 10_485_760, // 10MB
    sameOrigin: true,
    fetchMeta: true,
    actionRateLimit: {
      max: 30, // 30 requests
      window: 60, // ต่อ 60 วินาที
      key: 'ip',
    },
    trustedProxyIps: ['10.0.0.0/8', '172.16.0.0/12'],
    headers: true,
    csrf: true,
  },
  image: {
    formats: ['webp', 'avif'],
    sizes: [320, 640, 960, 1280, 1920],
    quality: 80,
    lazy: true,
    placeholder: 'blur',
  },
  middleware: {
    builtin: {
      cors: {
        origins: ['https://shop.example.com', 'https://api.stripe.com'],
        methods: ['GET', 'POST'],
        credentials: true,
      },
      rate: {
        max: 100,
        window: 60,
        key: 'ip',
      },
      timing: true,
      log: true,
    },
    workers: 2,
    timeoutMs: 10000,
  },
  plugins: [
    {
      name: 'requireEnv',
      options: {
        vars: ['DATABASE_URL', 'STRIPE_API_KEY', 'AUTH_SECRET'],
        mode: 'strict',
      },
    },
    {
      name: 'pwa',
      options: {
        manifest: {
          name: 'My Shop',
          short_name: 'Shop',
          display: 'standalone',
        },
      },
    },
  ],
  adapter: 'node',
})
```

### 4. API Backend

```ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  site: { url: 'https://api.example.com' },
  server: {
    host: '0.0.0.0',
    port: 4000,
  },
  build: {
    minify: true,
    sourcemap: true,
    target: 'node', // server-only bundle
  },
  security: {
    apiLimit: 52_428_800, // 50MB สำหรับ file upload
    sameOrigin: false, // API ต้องรับจากทุก origin
    actionRateLimit: {
      max: 1000,
      window: 60,
      key: 'ip',
    },
    trustedProxyIps: ['0.0.0.0/0'],
  },
  middleware: {
    builtin: {
      cors: {
        origins: ['*'],
        methods: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'],
        headers: ['Content-Type', 'Authorization'],
        credentials: true,
      },
      rate: {
        max: 1000,
        window: 60,
        key: 'ip',
      },
    },
  },
  plugins: [
    {
      name: 'open-api',
      options: {
        title: 'My API',
        version: '1.0.0',
        description: 'REST API documentation',
      },
    },
    {
      name: 'observability',
      options: {
        endpoint: '/api/health',
        metrics: true,
      },
    },
  ],
})
```

### 5. Full-Stack Dashboard (PPR + CSR)

```ts
import { defineConfig } from 'ruvyxa/config'

export default defineConfig({
  appDir: 'app',
  render: {
    strategy: 'ppr',
  },
  build: {
    splitStrategy: 'route',
    parallel: 8,
  },
  debug: {
    overlay: true,
    traces: true,
  },
  css: {
    entries: ['src/styles/global.css', 'src/styles/dashboard.css'],
    tailwind: true,
  },
  plugins: [
    {
      name: 'fonts',
      options: {
        families: ['Inter:wght@400;500;600;700'],
        display: 'swap',
      },
    },
    {
      name: 'web-vitals',
      options: {
        endpoint: '/api/vitals',
      },
    },
  ],
  adapter: 'vercel',
  adapterOptions: {
    regions: ['sin1', 'hkg1'],
  },
})
```

---

## ตรวจสอบ Config

### `ruvyxa doctor`

```bash
npm run doctor
```

Output:

```
━━━ Ruvyxa Doctor ━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Config
  ──────
  ✓ ruvyxa.config.ts (file exists)
  ✓ Parsed successfully
  ✓ defineConfig() valid

  Fields
  ──────
  ✓ appDir:           "app" (exists)
  ✓ outDir:           ".ruvyxa"
  ✓ server.host:      "localhost"
  ✓ server.port:      3000
  ✓ site.url:         "https://example.com"
  ✓ build.minify:     true
  ✓ build.sourcemap:  false
  ✓ No unknown fields
  ✓ All paths valid
  ✓ Middleware workers: 2 (valid)

  Warnings
  ────────
  ⚠ site.url uses auto-detect — set explicitly for production
  ⚠ security.headers enabled, but no CSP defined
```

### `--json` flag

```json
{
  "ok": true,
  "config": {
    "appDir": "app",
    "outDir": ".ruvyxa",
    "server": { "host": "localhost", "port": 3000 },
    "site": { "url": "auto-detect" }
  },
  "validations": {
    "passed": 14,
    "warnings": 2,
    "errors": 0
  }
}
```

---

## Troubleshooting — ทุก Error และวิธีแก้

| Error Code | ปัญหา                     | สาเหตุ                                  | วิธีแก้                                   |
| ---------- | ------------------------- | --------------------------------------- | ----------------------------------------- |
| RUV1601    | Config field invalid      | ค่าไม่ถูกต้อง (0, empty, absolute path) | ตรวจค่าที่กำหนดในตาราง                    |
| RUV1602    | Config value out of range | ค่าเกินขีดจำกัด                         | ปรับค่าให้อยู่ในช่วง                      |
| RUV1603    | Unknown field             | พิมพ์ชื่อฟิลด์ผิด                       | ตรวจ camelCase (`appDir` ไม่ใช่ `appdir`) |
| RUV1604    | Duplicate plugin name     | plugin ชื่อซ้ำ                          | เปลี่ยนชื่อ plugin                        |
| RUV1605    | Type mismatch             | ชนิดข้อมูลผิด (string แทน number)       | ตรวจชนิดที่ถูกต้อง                        |
| RUV1606    | Required field missing    | ฟิลด์จำเป็นขาด                          | เพิ่มฟิลด์ที่ต้องการ                      |

| ปัญหาทั่วไป                    | สาเหตุ                     | วิธีแก้                                                       |
| ------------------------------ | -------------------------- | ------------------------------------------------------------- |
| Config ไม่ถูกโหลด              | syntax error ในไฟล์        | ตรวจ `defineConfig()` และ `,`                                 |
| `site.url` error               | URL ไม่ใช่ origin          | ใช้เฉพาะ origin (`https://x.com` ไม่ใช่ `https://x.com/path`) |
| Port ถูกใช้                    | port ซ้ำ                   | เปลี่ยน `server.port`                                         |
| Unknown field                  | camelCase ผิด              | `sourcemap` ไม่ใช่ `sourceMap`                                |
| Plugin ไม่ทำงาน                | ชื่อ plugin ไม่ถูกต้อง     | ตรวจชื่อในตาราง                                               |
| Adapter auto-detect ผิด        | env var ไม่ตั้ง            | ใช้ `adapter: 'name'` explicit                                |
| CSS entries ไม่ inject         | path ผิด                   | ตรวจว่า path มีอยู่จริง                                       |
| Image optimization ไม่ได้      | encoder ไม่รองรับไฟล์      | ตรวจตาราง encoder                                             |
| Security headers ไม่มา         | `headers: false`           | ตั้ง `headers: true`                                          |
| middleware rate limit ไม่ work | ไม่ได้กำหนด `max`/`window` | เพิ่ม rate config                                             |

### Debug Config

```bash
# ดูว่า config ไหนถูกโหลด
RUVYXA_DEBUG=config ruvyxa dev

# ดู validation steps
RUVYXA_DEBUG=validate ruvyxa dev

# ดู plugin loading
RUVYXA_DEBUG=plugin ruvyxa dev
```

---

## ลองทำดู

1. **Config พื้นฐาน**
   - เปิด `ruvyxa.config.ts`
   - เปลี่ยน `server.port` เป็น 4000 → `npm run dev`

2. **Security**
   - ตั้ง `security.sameOrigin: true`
   - ตั้ง `security.actionLimit: 2_097_152`
   - ทดลองส่ง request ใหญ่เกิน limit

3. **Image**
   - ตั้งค่า `image.encoder.jpeg: 'guetzli'`
   - `npm run build` → สังเกตเวลาที่เพิ่มขึ้น

4. **Middleware**
   - เปิด CORS ด้วย origins แค่ `['https://example.com']`
   - ทดสอบจาก origin อื่น

5. **Plugin**
   - เพิ่ม plugin `redirects` → redirect `/old` → `/new`
   - เพิ่ม plugin `requireEnv` → ตั้ง `DATABASE_URL`

6. **ตรวจสอบ**
   - `npm run doctor` — ดูผล validation
   - `npm run doctor --json` — ดู JSON output

---

## สรุป

- `ruvyxa.config.ts` = ศูนย์รวม config
- `defineConfig()` = type safety + auto-complete
- validation code: RUV1600-RUV1699
- adapter auto-detect จาก platform env vars
- plugins 16 ตัวพร้อมใช้
- ทุกฟิลด์มี default + validation
- Rust backend validation ที่ robust
- `npm run doctor` ตรวจสอบทุกอย่าง
