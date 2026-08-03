# การค้นพบและตรวจสอบเส้นทาง (`ruvyxa_graph`)

**ไฟล์**: `crates/ruvyxa_graph/src/lib.rs` (1696 บรรทัด, ไฟล์เดียว)

การค้นพบเส้นทางเดินไดเรกทอรี `app/` ในระบบไฟล์, จัดประเภทไฟล์ตามหลักการตั้งชื่อ,
ตรวจจับกลยุทธ์การเรนเดอร์จากซอร์สโค้ด, ตรวจสอบขอบเขต server/client, และสร้าง `RouteManifest`

---

## นิยามชนิดข้อมูล

```rust
pub type RouteParams = BTreeMap<String, serde_json::Value>;
// JSON-shaped: catch-all segments → Value::Array
// Omitted optional catch-all → no entry

pub struct RouteManifest {
    pub app_dir: PathBuf,
    pub routes: Vec<RouteEntry>,
}

pub struct RouteEntry {
    pub id: String,                    // e.g. "app/blog/[slug]/page"
    pub path: String,                  // e.g. "/blog/[slug]"
    pub kind: RouteKind,               // Page | Api
    pub file: PathBuf,                 // absolute path to page/route file
    pub layout_chain: Vec<String>,     // route IDs of ancestor layouts
    pub server_modules: Vec<String>,   // sibling server.ts, action.ts
    pub client_modules: Vec<String>,   // sibling client.tsx
    pub runtime: RuntimeTarget,        // Node | Edge | Static (all Node currently)
    pub render: RenderMeta,
}

pub enum RouteKind { Page, Api }       // serde: kebab-case
pub enum RuntimeTarget { Node, Edge, Static }
pub enum RenderStrategy { Ssr, Ssg, Isr, Csr, Ppr } // default: Ssr

pub struct RenderMeta {
    pub strategy: RenderStrategy,
    pub revalidate: Option<u64>,       // ISR seconds
    pub has_static_params: bool,       // getStaticParams/staticParams export
    pub static_paths: Vec<String>,     // resolved static param combos
    pub has_dynamic_slots: bool,       // PPR Suspense boundaries
    pub hydrate: bool,                 // default true; `export const hydrate = false` = zero-JS
    pub hydration: HydrationMode,      // Load (ค่าเริ่มต้น) | Idle | Visible | None
}

pub enum HydrationMode { Load, Idle, Visible, None } // serde: kebab-case

pub struct DiscoverOptions {
    pub app_dir: PathBuf,
    pub default_render_strategy: Option<RenderStrategy>,
    pub default_revalidate: Option<u64>,
}

pub struct ValidationReport {
    pub routes: usize,
    pub page_routes: usize,
    pub api_routes: usize,
    pub client_modules: usize,
    pub server_modules: usize,
    pub diagnostics: Vec<Diagnostic>,
    // is_ok() → diagnostics.is_empty()
}
```

---

## `discover_routes(options) → Result<RouteManifest>`

### ขั้นตอนที่ 1: ตรวจสอบ

```
if !app_dir.exists() → RUV1001 "App directory not found"
```

### ขั้นตอนที่ 2: เดินระบบไฟล์

```
WalkDir::new(&app_dir)
  .filter_entry: skip dirs starting with "_" or "@"
  .filter_map(Result::ok)
```

### ขั้นตอนที่ 3: จับคู่ไฟล์

สำหรับแต่ละรายการไฟล์, จับคู่ `file_name`:

| ชื่อไฟล์                                      | RouteKind         |
| --------------------------------------------- | ----------------- |
| `page.tsx`, `page.jsx`, `page.md`, `page.mdx` | `Page`            |
| `route.ts`, `route.js`                        | `Api`             |
| อื่น                                          | `continue` (ข้าม) |

**หมายเหตุ**: `action.ts`, `action.js`, `server.ts`, `server.js`, `client.tsx` จะไม่ถูกจับคู่ที่นี่
— พวกมันถูกค้นพบเป็น **sibling modules** ของไฟล์ route ที่จับคู่

### ขั้นตอนที่ 4: คำนวณฟิลด์

**`path = route_path_from_dir(relative_dir)`**

1. แยก `relative_dir` เป็น components
2. กรองเฉพาะ `Component::Normal`:
   - **ละ** route groups `(name)` — วงเล็บ, เนื้อหาถูกละใน URL
   - **ละ** parallel slots `@name` — เครื่องหมาย at, ถูกละ
3. สำหรับแต่ละ segment ที่เหลือ, เรียก `route_segment(segment, is_last)`
4. ถ้าไม่มี segments เหลือ → `"/"`
5. เชื่อมด้วย `/`, เติม `/` นำหน้า

**`route_segment(segment: &str, is_last: bool) → Result<String>`**

| รูปแบบ                           | การจัดประเภท       | กฎ                                                                                           |
| -------------------------------- | ------------------ | -------------------------------------------------------------------------------------------- |
| `[[...name]]`                    | Optional catch-all | ต้องเป็น segment สุดท้าย ลบ `[[...` และ `]]` ออก `validate_dynamic_name(name)` คืนค่าตามเดิม |
| `[...name]`                      | Required catch-all | ต้องเป็น segment สุดท้าย ลบ `[...` และ `]` ออก `validate_dynamic_name(name)` คืนค่าตามเดิม   |
| `[name]`                         | Dynamic param      | `validate_dynamic_name(name)` คืนค่าตามเดิม                                                  |
| มี `[`/`]` แต่ไม่ตรงรูปแบบข้างบน | ไม่ถูกต้อง         | → RUV1002                                                                                    |
| ข้อความธรรมดา                    | Static             | คืนค่าไม่เปลี่ยนแปลง                                                                         |

**`validate_dynamic_name(name: &str) → Result<()>`**

- ต้องไม่ว่าง
- ต้องไม่มี `[` หรือ `]`
- ต้องไม่ขึ้นต้นด้วย `.`

**`id = route_id(app_dir, file)`**

ลบ prefix `app_dir` จาก `file` ลบนามสกุล เชื่อม components ด้วย `/` เติม `app/` นำหน้า กรอง เฉพาะ
`Component::Normal`

**`layout_chain = layout_chain(app_dir, route_dir)`**

1. เริ่มที่ `current = app_dir`
2. ถ้า `current/layout.tsx` มีอยู่ → push `route_id(app_dir, current/layout.tsx)`
3. เดิน components `relative` จาก `route_dir.strip_prefix(app_dir)`:
   - สำหรับแต่ละ `Component::Normal`: `current.push(component)`
   - ถ้า `current/layout.tsx` มีอยู่ → push `route_id(...)`
4. คืนค่ารายการแบบเรียงลำดับ: root layout ก่อน, innermost ทีหลัง

**`server_modules = sibling_modules(route_dir, &["server.ts", "server.js", "action.ts", "action.js"])`**

ตรวจสอบแต่ละชื่อไฟล์ที่ `route_dir/name` push path ถ้ามีอยู่

**`client_modules = sibling_module(route_dir, "client.tsx")`**

ตรวจสอบว่า `route_dir/client.tsx` มีอยู่หรือไม่ คืนค่า Vec ที่มี 0 หรือ 1 รายการ

### ขั้นตอนที่ 5: การตรวจจับการเรนเดอร์ (เฉพาะ Page)

Page routes เรียก
`apply_rendering_defaults(detect_render_strategy(...), default_strategy, default_revalidate)`

API routes → `RenderMeta::default()` (SSR)

### ขั้นตอนที่ 6: เรียงลำดับและกำจัดซ้ำ

Routes เรียงตาม `path` แล้วตาม `id`

### ขั้นตอนที่ 7: ตรวจจับความขัดแย้ง

`detect_conflicts(routes)`:

1. สร้าง `BTreeMap<match_shape, RouteEntry>`
2. `route_match_shape(path)`:
   - `[[...name]]` → `*?`
   - `[...name]` → `*`
   - `[name]` → `:`
   - Literals → ไม่เปลี่ยนแปลง
3. ถ้าพบการชน → RUV1003 พร้อม route IDs ทั้งสองใน `affected_routes`

ตัวอย่าง: `/blog/[slug]` และ `/blog/[id]` ต่างก็แมปไปยัง `/blog/:` → ขัดแย้ง

---

## `detect_render_strategy(file, layout_chain) → RenderMeta`

การจับคู่แบบเรียงลำดับครั้งแรก หยุดที่รายการแรกที่ตรง

### 1. Client-Side Rendering (CSR)

```
"use client" directive in original source
  → RenderMeta { strategy: CSr, ..default() }
```

อ่านซอร์ส **ต้นฉบับ** (ไม่ถูกดัดแปลง) ตรวจสอบว่าบรรทัดแรกหลังจากตัดช่องว่างขึ้นต้นด้วย
`"use client"` หรือ `'use client'`

### 2. Partial Pre-Rendering (PPR)

```
export const ppr = true
  → RenderMeta { strategy: Ppr, has_dynamic_slots: true, ..default() }
```

### 3. Incremental Static Regeneration (ISR)

```
export const revalidate = <number>
  → RenderMeta { strategy: Isr, revalidate: Some(seconds), has_static_params: check_for_static_params(), ..default() }
```

ดึงข้อมูลผ่าน regex: `export const revalidate = (\d+)` (หลังจากลบ comments/strings)

### 4. Static Site Generation (SSG) — แบบชัดแจ้ง

```
getStaticParams or staticParams export in source
  → RenderMeta { strategy: Ssg, has_static_params: true, ..default() }
```

ตรวจสอบ `getStaticParams` หรือ `staticParams` ในโค้ดที่สกัดเฉพาะ export-names

### 5. Static Site Generation (SSG) — อัตโนมัติ

```
No dynamic segments in path
  AND no dynamic data markers in reachable code (page + layout chain)
  → RenderMeta { strategy: Ssg, ..default() }
```

**Reachable code**: `collect_relative_graph(page_file + all layout files)` → ต่อกัน → ลบ
strings/comments → ตรวจสอบ markers

**Dynamic data markers** (ใดๆ ต่อไปนี้ → ไม่ใช่ static):

- `fetch(`, `headers(`, `cookies(`, `searchParams`
- `Date.now(`, `Math.random(`
- `process.env.` (การอ่าน runtime env ใดๆ ทำให้ไม่เป็น static)

### 6. Server-Side Rendering (SSR) — ค่าเริ่มต้น

```
None of the above
  → RenderMeta::default()  // strategy: Ssr, everything false/None
```

### การกำหนดเวลา Hydration (`parse_hydration_mode`)

แยกจากการจับคู่กลยุทธ์ข้างต้น `detect_render_strategy` ยังสแกนหา `export const hydrate` และตั้งค่า
`RenderMeta.hydration` / `RenderMeta.hydrate`:

- `hydrate = false` หรือ `hydration = 'none'` → `HydrationMode::None`, `hydrate: false` (route แบบ
  zero-JS ไม่มีการสร้าง client bundle)
- `hydration = 'idle'` → `HydrationMode::Idle` (ขอ bundle ผ่าน `requestIdleCallback`)
- `hydration = 'visible'` → `HydrationMode::Visible` (ขอ bundle เมื่อ root ปรากฏในวิวพอร์ต)
- อื่นๆ / ไม่ระบุ → `HydrationMode::Load` (ค่าเริ่มต้น, eager)

ฟิลด์ boolean `hydrate` ถูกเก็บไว้เพื่อความเข้ากันได้กับ manifest รุ่นเก่า ส่วน `hydration`
คือค่าจริงที่ ใช้กำหนดตารางเวลา ใช้ได้กับทุกกลยุทธ์ที่เรนเดอร์ฝั่งเซิร์ฟเวอร์ (SSR, SSG, ISR, PPR) —
หน้า CSR (`'use client'`) จะไม่สนใจค่านี้และ hydrate แบบ eager เสมอ

### `apply_rendering_defaults(render, default_strategy, default_revalidate)`

ถ้า `render.strategy` ไม่ใช่ `Ssr` → คืนค่าไม่เปลี่ยนแปลง (strategy แบบชัดแจ้งชนะ)

ถ้า `default_strategy` เป็น `Some` → นำไปใช้กับ meta ถ้าเป็น ISR และไม่ได้ตั้ง `revalidate` →
ค่าเริ่มต้น 60 วินาที

---

## `validate_app(root, manifest) → Result<ValidationReport>`

เรียกหลังจาก `discover_routes` สแกนซอร์สของทุก route เพื่อหาการละเมิดขอบเขต

### การตรวจสอบ Page

สำหรับแต่ละ Page route:

1. อ่านซอร์ส ถ้าเป็น `.md`/`.mdx` → ข้ามการตรวจสอบ default-export (content compilation จัดเตรียมให้)
2. ตรวจสอบ `export default` มีอยู่ → ถ้าไม่มี RUV1004
3. `collect_relative_graph(page_file + layout_chain)` → BFS relative imports
4. ตรวจสอบแต่ละ module ในกราฟผ่าน `validate_client_module()`

### การตรวจสอบ API

สำหรับแต่ละ API route:

1. `collect_relative_graph(route_file)` → BFS relative imports
2. ตรวจสอบแต่ละ module ในกราฟผ่าน `validate_server_module()`

### การตรวจสอบ module แบบชัดแจ้ง

- แต่ละ `server_module` → `validate_server_module()`
- แต่ละ `client_module` → `validate_client_module()`

### `validate_client_module(source, file_path, root) → Vec<Diagnostic>`

| การตรวจสอบ             | กฎ                                                                                                                   | รหัส    |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------- | ------- |
| import `"server-only"` | สแกนข้อความหา `import "server-only"` หรือ `import 'server-only'` → ข้อผิดพลาด                                        | RUV1007 |
| การเข้าถึง env ส่วนตัว | `process.env.<NAME>` โดยที่ NAME ไม่ขึ้นต้นด้วย `RUVYXA_PUBLIC_` จัดการ `process.env["<NAME>"]` ด้วย ข้าม `NODE_ENV` | RUV1008 |
| import จาก `server/`   | path ไฟล์ (canonicalized) ขึ้นต้นด้วย `<root>/server/` เฉพาะ `server/` ที่รากโปรเจกต์ ไม่ใช่ `app/server/`           | RUV1010 |

### `validate_server_module(source, file_path) → Vec<Diagnostic>`

| การตรวจสอบ             | กฎ                                                                         | รหัส    |
| ---------------------- | -------------------------------------------------------------------------- | ------- |
| import `"client-only"` | สแกนข้อความหา `import "client-only"` หรือ `import 'client-only'` → คำเตือน | RUV1009 |

---

## `collect_relative_graph(entry: &Path) → BTreeSet<PathBuf>`

BFS จากไฟล์ entry รวบรวม transitive closure ของ **relative** imports

1. คิว: เริ่มด้วย entry
2. `visited: BTreeSet<PathBuf>`
3. ขณะที่คิวไม่ว่าง:
   - ดึงหน้าคิว ข้ามถ้า visited แล้ว
   - อ่านซอร์ส สกัด import specifiers ผ่าน `import_specifiers(source)`
   - สำหรับแต่ละ specifier:
     - **ข้าม** ถ้าไม่ขึ้นต้นด้วย `.` (relative เท่านั้น, ไม่มี bare/node_modules)
     - `resolve_relative_import(from, specifier)` → `Option<PathBuf>`
     - ถ้า resolved, push เข้าคิว
4. คืนค่า visited set

### `import_specifiers(source: &str) → Vec<String>`

1. `code_for_import_specifiers(source)` — เก็บ strings ที่ตามหลัง `from`, `import`, `import(`,
   `require(` ลบ strings อื่น, template literals, block comments, line comments
2. สแกนบรรทัดหา:
   - ` from "..."` หรือ ` from '...'` → สกัด quoted specifier
   - `import "..."` หรือ `import '...'` ที่ต้นบรรทัด → สกัด quoted specifier
   - `import(` → สกัด specifier จาก `import("...")`
   - `require(` → สกัด specifier จาก `require("...")`

### `resolve_relative_import(from: &Path, specifier: &str) → Option<PathBuf>`

ฐาน = `from.parent() / specifier` ลองตัวเลือก:

1. bare path (ตรง)
2. `<bare>.ts`, `<bare>.tsx`, `<bare>.js`, `<bare>.jsx`, `<bare>.md`, `<bare>.mdx`
3. `<bare>/index.ts`, `<bare>/index.tsx`, `<bare>/index.js`, `<bare>/index.jsx`, `<bare>/index.md`,
   `<bare>/index.mdx`

คืนค่ารายการแรกที่ `is_file()` พยายาม `canonicalize()`

---

## ฟังก์ชันช่วยเหลือ

### `private_env_reads(source) → BTreeSet<String>`

สแกนหา `process.env.NAME` และ `process.env['NAME']` คืนค่าชื่อที่ไม่มี prefix `RUVYXA_PUBLIC_`
ไม่รวม `NODE_ENV`

ใช้ scanner ระดับไบต์ ข้าม strings, comments, template literals (แต่เจาะลึกเข้าไปใน `${}`
expressions) จัดการ bracket notation ทั้ง single และ double quotes

### `code_without_strings_and_comments(source) → String`

ลบ:

- Double-quoted strings (`"..."`) — จัดการ escape sequences
- Single-quoted strings (`'...'`) — จัดการ escape sequences
- Template literals (`` `...` ``) — จัดการ `${}` nesting ผ่าน depth counter
- Line comments `//` → จบสิ้นบรรทัด
- Block comments `/* ... */`

ใช้โดย `detect_render_strategy` ขั้นตอนที่ 5 (static candidate) และ ISR regex

### `code_for_export_scanning(source) → String`

เหมือน `code_without_strings_and_comments` แต่เก็บบริบทของคีย์เวิร์ด `export` คืนค่าบรรทัด ที่มี
`export` สำหรับการจับคู่รูปแบบ

### การทำให้เป็นอนุกรม

```rust
pub fn write_manifest(manifest: &RouteManifest, output_file: &Path) -> Result<()>
    // serde::to_writer_pretty → output_file

pub fn read_manifest(manifest_file: &Path) -> Result<RouteManifest>
    // serde::from_reader → RouteManifest
```

---

## รหัสวินิจฉัย (โมดูล graph)

| รหัส    | เงื่อนไข                                              | คำแนะนำ                                                                                      |
| ------- | ----------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| RUV1001 | ไม่พบไดเรกทอรี App                                    | สร้าง `app/` หรือตั้ง `appDir` ใน config                                                     |
| RUV1002 | dynamic segment ไม่ถูกต้อง (`[a b]`, `[]`, วงเล็บผิด) | ใช้ `[param]`, `[...rest]`, `[[...opt]]`                                                     |
| RUV1003 | รูปแบบ match shape ซ้ำกัน                             | เปลี่ยนชื่อ route segment; dynamic params ที่ระดับเดียวกันต้องใช้ static prefixes ที่ต่างกัน |
| RUV1004 | Page ขาด `export default`                             | เพิ่ม default export ให้กับ page component                                                   |
| RUV1007 | โมดูล server-only ถูก import ใน client graph          | ย้าย server logic ไปยัง `server/` หรือ `action.ts`                                           |
| RUV1008 | ตัวแปรสภาพแวดล้อมส่วนตัวถูกใช้ใน client graph         | เปลี่ยนชื่อเป็น `RUVYXA_PUBLIC_*` หรือย้ายไปยัง server-only code                             |
| RUV1009 | โมดูล client-only ถูก import ใน server graph          | ลบ client-only dependency จาก API/server code                                                |
| RUV1010 | โมดูลในไดเรกทอรี server ถูก client graph เข้าถึง      | อย่า import จาก `server/` ใน client-reachable code                                           |
