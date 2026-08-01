# ตั้งค่า Ruvyxa ด้วย `ruvyxa.config.ts`

`ruvyxa.config.ts` คือศูนย์กลางการตั้งค่าทั้งหมดของโปรเจกต์ Ruvyxa — ควบคุมทุกอย่างตั้งแต่ directory
structure, server, render strategy, build, cache, debug, CSS, image optimization, security,
middleware, plugins, ไปจนถึง deployment adapters

---

## สิ่งที่คุณจะได้เรียนรู้ (What You Will Learn)

- โครงสร้างของ config object แบบเต็ม
- ทุก field พร้อมระบุ TypeScript type, Rust type, ค่าเริ่มต้น (default), การตรวจสอบ (validation)
  และพฤติกรรม
- กฎการตรวจสอบความถูกต้องของ Configuration (RUV1600-1602)
- การจับคู่ field กับโครงสร้าง Rust `ProjectConfig` สำหรับทุก field
- วิธีปรับแต่งเซิร์ฟเวอร์, build, render, cache, image, security, middleware, และ plugins
- การคอนฟิก Plugin และ Adapter
- รหัส Validation error และเงื่อนไขที่ทำให้เกิด
- ตัวอย่างคอนฟิกขั้นต่ำ (Minimal) และแบบใช้งานจริง (Production)

---

## ฟังก์ชัน Config (The Config Function)

```ts
// ruvyxa.config.ts
import { config } from 'ruvyxa/config'

export default config({
  // ... การตั้งค่าของคุณ
})
```

ฟังก์ชัน `config()` ช่วยให้มี autocomplete และตรวจสอบความถูกต้อง (validation)
ฟิลด์ที่ไม่รู้จักจะเกิด type error และ runtime warning `RUV1200`

### ลำดับการโหลด Config

```
1. CLI อ่าน --root (ค่าเริ่มต้น: ".")
2. เรียก load_project_config(root)
3. ค้นหา find_runtime_script(root, "config-renderer.mjs")
4. หากไม่พบ: คืนค่า ProjectConfig::default() พร้อมค่า default ต่างๆ
5. หากพบ: สร้าง process Node/Bun เพื่อรัน config-renderer.mjs
6. Config renderer ประมวลผล ruvyxa.config.ts, พ่น JSON ออกทาง stdout
7. ข้อมูลจาก Config renderer ถูกแปลง -> โครงสร้าง ProjectConfig
8. เรียก validate_paths() บน config ที่ถูกพาร์สแล้ว
9. การเขียนทับค่ารันไทม์ (Runtime override ผ่าน --runtime flag) หากมี
10. คำนวณ dependency_hash สำหรับทำ cache invalidation
```

Config renderer จะถูกรันสองครั้งหากรันไทม์ที่ถูกเลือกแตกต่างไปจาก bootstrap runtime

---

## รูปแบบ Config แบบเต็ม (Full Config Type - TypeScript)

```ts
type RuvyxaConfig = {
  appDir?: string
  outDir?: string
  runtime?: 'node' | 'bun' | 'edge' | 'static'
  server?: {
    host?: string
    port?: number
  }
  site?: {
    sitemap?: {
      defaults?: {
        changefreq?: 'always' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly' | 'never'
        priority?: number
      }
      entries?: Array<{
        path: string
        changefreq?: string
        priority?: number
        lastmod?: string
        images?: string[]
      }>
    }
  }
  render?: {
    strategy?: 'ssr' | 'ssg' | 'isr' | 'csr' | 'ppr'
    revalidate?: number
  }
  build?: {
    minify?: boolean
    map?: boolean
    treeShake?: boolean
    split?: 'route' | 'single' | 'manual'
    workers?: number
    jsx?: 'automatic' | 'classic'
    target?: 'es2018' | 'es2019' | 'es2020' | 'es2022' | 'esnext'
    manifest?: boolean
    warm?: boolean
    prerenderCache?: boolean
  }
  cache?: {
    routes?: boolean
    css?: boolean
    dir?: string
  }
  debug?: {
    overlay?: boolean
    traces?: boolean
  }
  css?: {
    entries?: string[]
  }
  image?: {
    optimize?: boolean
    quality?: number
    lossless?: boolean
    keepOriginal?: boolean
    variantWidths?: number[]
    workers?: number
  }
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
  middleware?: {
    workers?: number
    timeoutMs?: number
  }
  plugins?: Array<{
    name: string
    options?: any
    head?: Array<{ tag: string; attrs?: Record<string, string>; content?: string }>
  }>
  adapter?: string
  adapterOptions?: Record<string, any>
}
```

---

## โครงสร้าง Rust ProjectConfig Struct Mapping

โครงสร้าง Rust แบบเต็มอยู่ที่ `crates/ruvyxa_cli/src/main.rs`:

```rust
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectConfig {
    app_dir: Option<String>,
    out_dir: Option<String>,
    runtime: Option<BuildTarget>,
    #[serde(default, rename = "render")]
    rendering: RenderingConfigOptions,
    #[serde(default)]
    server: ServerConfigOptions,
    #[serde(default)]
    css: CssConfigOptions,
    #[serde(default)]
    build: BuildConfigOptions,
    #[serde(default)]
    debug: DebugConfigOptions,
    #[serde(default, rename = "image")]
    images: ImageOptimizationOptions,
    #[serde(default)]
    security: SecurityConfigOptions,
    #[serde(default)]
    cache: CacheConfigOptions,
    #[serde(default)]
    site: SiteConfigOptions,
    #[serde(default)]
    middleware: MiddlewareConfig,
    #[serde(default)]
    plugins: Vec<BuildPluginConfig>,
    #[serde(rename = "adapter")]
    adapter: Option<serde_json::Value>,
    #[serde(rename = "adapterOptions")]
    adapter_options: Option<serde_json::Value>,
    // ข้ามฟิลด์สำหรับใช้งานภายใน
}
```

จุดสังเกต: `deny_unknown_fields` หมายความว่าฟิลด์ใดๆ ที่ไม่มีในโครงสร้างนี้จะทำให้เกิด error ของ
deserialization ในตอนที่ทำการโหลด config

---

## ข้อมูลฟิลด์พื้นฐาน (Common Fields)

### appDir

```ts
appDir: 'app' // ค่าเริ่มต้น
appDir: 'src/app' // กำหนดเอง
```

| Property       | Value                                                              |
| -------------- | ------------------------------------------------------------------ |
| TS type        | `string`                                                           |
| Rust field     | `app_dir: Option<String>`                                          |
| Rust default   | `None` -> แปลงเป็น `"app"` โดย `app_dir()`                         |
| Validation     | ต้องเป็น relative path (ไม่มี `/` นำหน้า, ไม่มี `..`), และห้ามว่าง |
| Error on empty | `RUV1601: config field 'appDir' must not be empty`                 |

### outDir

```ts
outDir: '.ruvyxa' // ค่าเริ่มต้น
outDir: 'dist' // กำหนดเอง
```

| Property     | Value                                                           |
| ------------ | --------------------------------------------------------------- |
| TS type      | `string`                                                        |
| Rust field   | `out_dir: Option<String>`                                       |
| Rust default | `None` -> แปลงเป็น `".ruvyxa"` โดย `out_dir()`                  |
| Validation   | ต้องเป็น relative path และต้องไม่ชี้ไปยังไฟล์ที่อยู่ภายนอก root |

### runtime

```ts
runtime: 'node' // กำหนดรันไทม์แบบเจาะจง
```

| Property   | Value                                                   |
| ---------- | ------------------------------------------------------- |
| TS type    | `'node'                                                 | 'bun' | 'edge' | 'static'` |
| Rust field | `runtime: Option<BuildTarget>`                          |
| Default    | ตรวจจับอัตโนมัติตาม Environment (เช็คจาก `bun`, `node`) |
| Override   | ธงคำสั่ง `--runtime` ใน CLI จะทำการทับค่าฟิลด์นี้       |

---

## Server Configuration

```ts
server: {
  host: '0.0.0.0', // เปิดรับจากภายนอก
  port: 8080       // กำหนดพอร์ต
}
```

| Field  | TS Type  | Default     | Validation | Error   | Behavior                                                            |
| ------ | -------- | ----------- | ---------- | ------- | ------------------------------------------------------------------- |
| `host` | `string` | `localhost` | Valid IP   | RUV1602 | bind host                                                           |
| `port` | `number` | `3000`      | 1024-65535 | RUV1602 | ถ้าใช้งานอยู่ CLI จะแจ้งเตือน RUV1201 เว้นแต่จะระบุ port = 0 (สุ่ม) |

---

## Site & SEO

```ts
site: {
  sitemap: {
    defaults: { priority: 0.8 },
    entries: [ { path: '/about', priority: 1.0 } ]
  }
}
```

ใช้ร่วมกับปลั๊กอิน `sitemap` และปลั๊กอิน `robots` เพื่อรวบรวม SEO metadata ตอนที่ Build เสร็จ
ค่าที่ถูกกำหนดใน `entries` จะไปทับค่า `defaults`

---

## Render Configuration

```ts
render: {
  strategy: 'ssr',
  revalidate: 60
}
```

| Field        | TS Type  | Default | Validation      | Behavior                             |
| ------------ | -------- | ------- | --------------- | ------------------------------------ |
| `strategy`   | `enum`   | `ssr`   | ssr,ssg,isr,... | รูปแบบการเรนเดอร์มาตรฐานของทุก Route |
| `revalidate` | `number` | `60`    | ≥ 1             | เวลาหน่วยวินาทีสำหรับ ISR            |

---

## Build Configuration

```ts
build: {
  minify: true,
  split: 'route',
  workers: 4
}
```

ควบคุม `oxc` compiler ภายใต้การทำงาน:

| Field       | Default  | Description                                         |
| ----------- | -------- | --------------------------------------------------- |
| `minify`    | `true`   | ย่อขนาดโค้ด (Minification) ผ่าน Oxc AST             |
| `treeShake` | `true`   | ลบโค้ดที่ไม่ได้ถูกใช้ (Dead code elimination)       |
| `split`     | `route`  | กลยุทธ์การแบ่ง Bundle ('route', 'single', 'manual') |
| `workers`   | CPUs     | จำนวนเทรดของ Oxc                                    |
| `target`    | `es2022` | JavaScript language target                          |

---

## Cache Configuration

```ts
cache: {
  routes: true,
  css: true,
  dir: '.ruvyxa/cache'
}
```

- `routes`: บันทึกผลลัพธ์ของ Oxc AST และการแปลงรูปแบบ JSX ของแต่ละไฟล์ เพื่อให้รีโหลดขณะ HMR
  ได้เร็วขึ้น
- `css`: เก็บผลลัพธ์ของ PostCSS เพื่อไม่ต้องประมวลผล Tailwind/Autoprefixer
  ใหม่ถ้าไฟล์นั้นไม่ได้ถูกแก้
- `dir`: ปรับแต่งสถานที่เก็บแคช

---

## Debug Configuration

```ts
debug: {
  overlay: true, // แสดง popup แจ้งเตือน error ใน browser
  traces: false  // เปิดการ log ประสิทธิภาพระดับ Oxc/Rust
}
```

---

## CSS Configuration

```ts
css: {
  entries: ['src/styles/global.css', 'src/styles/fonts.scss']
}
```

ชี้ตำแหน่งไฟล์ CSS เพิ่มเติมที่ไม่ถูกลิงก์ด้วย Layout หรือไม่ได้ประกาศใน `page.tsx`
ส่วนที่ถูกประกาศไว้ใน `entries` จะถูกเพิ่มเข้าไปที่หัวเอกสารอัตโนมัติ

---

## Image Configuration

```ts
image: {
  optimize: true,
  quality: 82,
  variantWidths: [640, 750, 1080, 1920]
}
```

ควบคุมกระบวนการ Build-time Image Optimization ผ่านไลบรารี Sharp:

- ถ้ากำหนด `optimize: false` คอมโพเนนต์ `<Image>` จะถูกแปลงกลับเป็น `<img loading="lazy">` ธรรมดา
- ภาพจะถูกปรับขนาดตามอาร์เรย์ `variantWidths` สำหรับทำแอตทริบิวต์ `srcset`

---

## Security Configuration

```ts
security: {
  actionLimit: 1048576, // 1MB
  apiLimit: 10485760,   // 10MB
  pluginLimit: 33554432, // 32MB
  sameOrigin: true
}
```

ควบคุม Body size parsers ที่ฝั่ง Server (เพื่อป้องกัน DDOS จาก Payload มหาศาล):

- RUV1602 จะถูกโยนขึ้นมา (throw) หากระบุเกินขีดจำกัดสูงสุด: `actionLimit > 10MB`, `apiLimit > 50MB`,
  `pluginLimit > 256MB`

---

## Middleware Configuration

```ts
middleware: {
  workers: 2,
  timeoutMs: 5000
}
```

กำหนดค่าการทำงานใน `middleware.ts`:

- ควบคุมจำนวน V8 Isolate workers เพื่อรัน middleware แบบคู่ขนาน
- ควบคุม Timeout (RUV2001 timeout error) หาก middleware ค้างและไม่คืนค่า

---

## Plugin Configuration

```ts
plugins: [
  {
    name: 'google-analytics',
    options: { trackingId: 'G-XXXX' },
    head: [{ tag: 'script', attrs: { src: '...' } }],
  },
]
```

ถูกทำ Schema Validate ในฝั่ง Rust ทันที หากชื่อปลั๊กอินว่างเปล่าหรือตั้งชื่อซ้ำ จะทำให้เกิด RUV1601

---

## Adapter Configuration

```ts
adapter: 'node',
adapterOptions: {
  outDir: 'build'
}
```

ตั้งเป้าหมายของการ Build หากไม่ได้ระบุ จะตรวจจับอัตโนมัติตามสภาพแวดล้อม:

1. หากพบ `VERCEL` -> กำหนด adapter `vercel` อัตโนมัติ
2. หากพบ `NETLIFY` -> กำหนด adapter `netlify` อัตโนมัติ
3. ค่าเริ่มต้นสุดท้าย -> ใช้ adapter `node`

---

## Configuration Validation

Ruvyxa ตรวจสอบ `ruvyxa.config.ts` **ก่อน** ที่จะรันคำสั่งอื่นใดเสมอ

- ฟิลด์ที่ไม่มีอยู่จริง: Error (`deny_unknown_fields` ในฝั่ง Rust)
- ผิดประเภทตัวแปร: Error (ผ่าน Serde)
- ค่าของฟิลด์ผิด: Error RUV1601 หรือ RUV1602 (จาก `validate_paths()` และกฎต่างๆ)

---

## Minimal Complete Config (คอนฟิกขั้นต่ำ)

นี่คือโครงร่างที่สะอาดที่สุด:

```ts
import { config } from 'ruvyxa/config'
export default config({})
```

ทุกอย่างจะโหลดค่าเริ่มต้น (Defaults) ที่สมเหตุสมผลให้คุณ

---

## Full Production Config (คอนฟิกใช้งานจริง)

```ts
import { config } from 'ruvyxa/config'

export default config({
  appDir: 'src/app',
  server: { port: 8080 },
  render: { strategy: 'isr', revalidate: 3600 },
  build: { split: 'manual', treeShake: true },
  image: { quality: 90, lossless: true },
  security: { sameOrigin: true },
  adapter: 'vercel',
})
```

---

## คำสั่ง Doctor (Doctor Command)

หากคุณไม่แน่ใจว่า Config ของคุณมีหน้าตาอย่างไรหลังจากถูกโหลดและผสานค่าเริ่มต้น:

```bash
ruvyxa doctor --config
```

จะพ่นรายละเอียดของการตั้งค่า และเช็ค RUV16xx ให้อย่างสมบูรณ์

## Validation Rules — Complete Reference (Rust)

### รหัส Error RUV1600-RUV1699

| Code    | เงื่อนไข                       | ฟิลด์                                     | วิธีแก้                   |
| ------- | ------------------------------ | ----------------------------------------- | ------------------------- |
| RUV1601 | ค่าไม่ถูกต้อง (invalid)        | หลายฟิลด์                                 | ตรวจค่าตามที่กำหนด        |
| RUV1602 | ค่าเกินขีดจำกัด (out of range) | หลายฟิลด์                                 | ปรับค่าให้อยู่ในช่วง      |
| RUV1603 | ฟิลด์ไม่รู้จัก (unknown field) | ทั้ง config                               | ตรวจ camelCase            |
| RUV1602 | config มีโครงสร้างไม่ถูกต้อง   | plugins.name ซ้ำหรือ field ไม่ถูกต้อง     | แก้ schema และชื่อ plugin |
| RUV1603 | adapter definition ไม่ถูกต้อง  | adapter ไม่มี `build(context)` ที่ถูกต้อง | แก้ adapter contract      |

### Validation Matrix

| Config Field | RUV1601 | RUV1602 | หมายเหตุ | | ------------------------------------ | -----------
| ---------------- | ------- | ---------------------- | | `appDir` empty/absolute | ✅ | - | ✅ |
relative path required | | `outDir` empty/absolute | ✅ | - | ✅ | relative path required | |
`server.port` 0 | ✅ | ✅ (ถ้า > 65535) | ✅ | 1024-65535 | | `server.host` invalid | - | ✅ | ✅ |
valid hostname/IP | | `site.url` invalid | - | ✅ | ✅ | origin เท่านั้น | |
`site.sitemap.defaults.priority` | - | ✅ (0-1) | ✅ | float | | `build.parallelism` 0 | ✅ | ✅
(ถ้า > 64) | ✅ | 1-64 | | `build.splitStrategy` invalid | ✅ | - | ✅ | auto/route/vendor/all | |
`build.jsxRuntime` invalid | ✅ | - | ✅ | automatic/classic | | `build.esTarget` invalid | ✅ | - |
✅ | es2020-esnext | | `security.actionLimit` 0 | ✅ | ✅ (>10MB) | ✅ | 1B-10MB | |
`security.apiLimit` 0 | ✅ | ✅ (>50MB) | ✅ | 1B-50MB | | `security.pluginLimit` 0 | ✅ | ✅
(>50MB) | ✅ | 1B-50MB | | `security.maxBodySize` 0 | ✅ | ✅ (>100MB) | ✅ | 1B-100MB | |
`security.trustedProxyIps[]` invalid | - | ✅ | ✅ | valid IP/CIDR | |
`security.actionRateLimit.max` 0 | ✅ | - | ✅ | ≥ 1 | | `security.actionRateLimit.window` 0 | ✅
| - | ✅ | ≥ 1 | | `middleware.workers` 0 | ✅ | ✅ (>8) | ✅ | 1-8 | | `middleware.timeoutMs` 0 |
✅ | ✅ (>300s) | ✅ | 1ms-300s | | `image.quality` out of range | ✅ (0/100+) | - | ✅ | 1-100 | |
`image.avifQuality` out of range | ✅ (0/100+) | - | ✅ | 1-100 | | `image.sizes[]` 0 | ✅ | ✅
(>10000) | ✅ | 1-9999 | | `image.formats` empty | ✅ | - | ✅ | ≥ 1 format | | `css.entries[]`
absolute | ✅ | - | ✅ | relative path | | `cache.buildDir` absolute | ✅ | - | ✅ | relative path |
| `adapter` unknown | ✅ | - | ✅ | ดู AdapterType | | `plugins[].name` empty/duplicate | ✅ | - |
✅ | unique, non-empty |

---

## Troubleshooting — ทุก Error และวิธีแก้

| Error Code | ปัญหา                      | สาเหตุ                                  | วิธีแก้                                   |
| ---------- | -------------------------- | --------------------------------------- | ----------------------------------------- |
| RUV1601    | Config field invalid       | ค่าไม่ถูกต้อง (0, empty, absolute path) | ตรวจค่าที่กำหนดในตาราง                    |
| RUV1602    | Config value out of range  | ค่าเกินขีดจำกัด                         | ปรับค่าให้อยู่ในช่วง                      |
| RUV1603    | Unknown field              | พิมพ์ชื่อฟิลด์ผิด                       | ตรวจ camelCase (`appDir` ไม่ใช่ `appdir`) |
| RUV1602    | Invalid config structure   | plugin ชื่อซ้ำหรือชนิดข้อมูลไม่ถูกต้อง  | ตรวจ schema และชื่อ plugin                |
| RUV1603    | Invalid adapter definition | adapter contract ไม่ถูกต้อง             | ตรวจ `build(context)` และผลลัพธ์          |

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

---

## Configuration เป็น Typed Contract

public contract คือ `RuvyxaConfig` จาก `@ruvyxa/core` ซึ่งปกติเขียนผ่าน `config()` จาก
`ruvyxa/config` top-level fields คือ `appDir`, `outDir`, `runtime`, `react`, `typescript`, `css`,
`server`, `build`, `render`, `debug`, `image`, `security`, `cache`, `site`, `middleware`, `adapter`,
`adapterOptions` และ `plugins` ควรตั้งค่าให้แคบ: ไม่ต้อง copy "full production" object
ที่เดาไว้เมื่อ defaults เดิมตรงกับแอปแล้ว

```ts
import { config, type RuvyxaConfig } from 'ruvyxa/config'

const settings: RuvyxaConfig = {
  server: { host: 'localhost', port: 3000 },
  build: { minify: true, split: 'route', workers: 4 },
  render: { strategy: 'ssr' },
  css: { entries: ['styles/print.css'] },
}

export default config(settings)
```

`appDir`, `outDir` และทุกค่าใน `css.entries` เป็น project-relative paths CLI จะปฏิเสธ path ว่าง,
absolute path หรือ path ที่หนีออกนอก project แทนการ resolve ออกไปอย่างเงียบ ๆ นี่เป็น safety
boundary: ใช้ relative directory ภายในแอป ไม่ใช่ absolute path ที่ผูกกับ OS

### Precedence เป็นราย Input ไม่ใช่ Global Override

คำสั่งที่รับ `--runtime` ให้ CLI value ชนะ `RUVYXA_RUNTIME` และ `config.runtime` สำหรับ `dev`,
`start` และ `preview`, `--host`/`--port` ชนะ server config ส่วน build target และ adapter มี CLI
overrides ของตัวเอง จึงไม่ควรเหมารวม precedence นี้ไปยัง config fields อื่น

```bash
ruvyxa dev --port 4000 --runtime bun
ruvyxa build --target static --adapter static
ruvyxa doctor --adapter cloudflare --json
```

### Validate ก่อน Deploy

ใช้ `doctor` เพื่อตรวจ configuration/runtime/adapter compatibility และใช้ `analyze` ตรวจ
route/import boundaries เพราะรับผิดชอบคนละเรื่อง จึงไม่แทนกัน:

```bash
ruvyxa doctor
ruvyxa analyze --format human
npm run check
```

configuration validation บังคับ positive bounded limits เช่น action/API payload limits และค่า
trusted-proxy IP/CIDR ที่ถูกต้อง หากค่าถูกปฏิเสธให้แก้ field นั้น ไม่ควรเพิ่ม environment override
ที่ เอกสารไม่ได้รองรับ
