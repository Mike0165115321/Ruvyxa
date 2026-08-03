# สายการคอมไพล์ — Compilation Pipeline (`ruvyxa_bundler`)

**ไฟล์**: `crates/ruvyxa_bundler/src/` (17 ไฟล์, ~10000 บรรทัด)

บัณฑเลอร์แปลงซอร์ส TypeScript/JSX/MD/MDX เป็นบันเดิล JavaScript ที่ปรับแต่งแล้ว (client hydration
หรือ SSR) สายการทำงาน: แก้ไข → คอมไพล์ → ตรวจสอบขอบเขต → เชื่อมโยง → ย่อขนาด → เอาต์พุต

---

## นิยามประเภท (`types.rs`)

```rust
pub enum BundleTarget { Client, Ssr }
pub enum JsxRuntime { Classic, Automatic }               // default Automatic
pub enum SplitStrategy { Single, Route }                  // default Single
pub enum EsTarget { Es2018, Es2019, Es2020, Es2022, EsNext }  // default Es2022

pub struct BundleInput {
    pub entry: PathBuf,              // page/route file
    pub project_root: PathBuf,
    pub app_dir: PathBuf,
    pub layouts: Vec<PathBuf>,       // ancestor layout files
    pub request_path: String,
    pub target: BundleTarget,
    pub options: BundleOptions,
}

pub struct BundleOptions {
    pub minify: bool,                          // default true
    pub source_map: bool,                      // default false
    pub tree_shaking: bool,                    // default true
    pub jsx_runtime: JsxRuntime,               // default Automatic
    pub es_target: EsTarget,                   // default Es2022
    pub split_strategy: SplitStrategy,         // default Single
    pub emit_chunk_manifest: bool,             // default false
    pub collect_module_manifest: bool,         // default false
}

pub struct BundleOutput {
    pub code: String,
    pub source_map: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub stats: BundleStats,
    pub chunk_manifest: Option<ChunkManifest>,
    pub chunks: Vec<OutputChunk>,
}

pub struct BundleStats {
    pub module_count: usize,
    pub output_bytes: usize,
    pub estimated_gz_bytes: usize,       // output_bytes * 0.35
    pub minified: bool,
    pub tree_shaken: bool,
    pub duration_ms: u64,
    pub tree_shaken_modules: usize,
    pub cache_hits: usize,
}

pub struct SharedRouteBundleOutput {
    pub code: String,                    // module registry on globalThis.__RUVYXA_SHARED_MODULES__
    pub modules: Vec<PathBuf>,
}

pub struct ChunkManifest {
    pub bundle_id: String,              // blake3(code)[..16]
    pub route: String,
    pub modules: Vec<String>,
    pub output_file: String,            // "{bundle_id}.js"
    pub source_map_file: Option<String>,
    pub size_bytes: usize,
    pub dynamic_imports: Vec<DynamicImportChunk>,
}

pub struct DynamicImportChunk {
    pub importer: String,
    pub module: String,
    pub file: String,
}

pub struct OutputChunk {
    pub file_name: String,
    pub code: String,
    pub modules: Vec<String>,
    pub kind: OutputChunkKind,            // DynamicImport | SharedRoute
}

pub enum BundleError {
    Diagnostic(Box<Diagnostic>),
    Io(std::io::Error),
    Compiler(String),
    Unresolved { specifier: String, importer: PathBuf },
    CircularDependency { cycle: String },
}
```

---

## API ระดับบน (`lib.rs`)

### `bundle(input: &BundleInput) → Result<BundleOutput>`

**สายการทำงานเต็ม**:

```
 1. build_entry_source(input)                  → virtual entry (entry_source, entry_label)
 2. resolve_graph_with_hooks(entry, ...)       → Vec<ResolvedModule>
 3. compile_graph_with_pipeline_and_maps(...)  → (Vec<CompiledModule>, BTreeMap<PathBuf, String>)
 4. boundary::check(modules, input, &mut diag) → diagnostics appended
 5. plan_dynamic_chunk_files(...)              → BTreeMap<PathBuf, String>  (client + chunk_manifest only)
 6. static_entry_modules(...)                 → Vec<CompiledModule>
 7. link_parallel_with_dynamic_imports_and_shared_modules(...)  → String
 8. tree_shake_exports(linked)                → String
 9. minify_with_options(linked, target, tree_shaking)  → String
10. output::wrap(linked, input)              → String + source_map → output wrapping
11. Source map generation (optional)
12. Chunk manifest + output chunk building (optional)
```

### `prepare_bundle(input) → PreparedBundle`

สายการทำงานบางส่วน: แก้ไข + คอมไพล์ + ตรวจสอบขอบเขต หยุดก่อนเชื่อมโยง/ย่อขนาด
เพื่อให้ค้นพบโมดูลที่ใช้ร่วมกันข้ามเส้นทาง

```rust
let prepared: Vec<PreparedRoute> = routes.iter().map(prepare_bundle).collect();
let shared = extract_shared_modules(&prepared);
let outputs = prepared.into_iter().map(|p| bundle_prepared(p, &shared)).collect();
```

### `bundle_shared_route_modules(modules, input) → SharedRouteBundleOutput`

คอมไพล์เซตของโมดูลที่ใช้ร่วมกันเป็น registry ที่เข้าถึง `globalThis.__RUVYXA_SHARED_MODULES__`

---

## 1. ตัวแก้ไข (Resolver — `resolver.rs`)

### `resolve_graph_with_cache(entry_source, entry_label, project_root, app_dir, cache) → Vec<ResolvedModule>`

**BFS แบบสองเฟสขนาน**:

**เฟส 1 (sequential)**: แก้ไข dependencies ของโมดูลเริ่มต้น จัดเก็บใน
`visited: BTreeMap<PathBuf, ResolvedModule>`

**เฟส 2 (BFS ขนาน)**:

```
while !frontier.is_empty() {
    frontier = frontier.par_iter()
        .filter_map(|dep_path| {
            cache.read_source(dep_path)?;
            // Fold NODE_ENV for client node_modules modules
            // Compile MD/MDX content first for dep extraction
            // Extract deps via collect_deps_cached()
            // Determine is_external (SSR only)
        })
        .collect();
    // Collect results, build next_frontier from unvisited deps
    // Repeat
}
```

คืนค่าโมดูลตามลำดับการค้นพบ BFS

### `ResolvedModule`

```rust
pub struct ResolvedModule {
    pub path: PathBuf,            // Canonical absolute path
    pub source: String,           // Original source text
    pub deps: Vec<PathBuf>,       // Resolved absolute dependency paths
    pub is_external: bool,        // From node_modules
}
```

### `ResolveGraphCache` (lock-free, ใช้ DashMap)

```rust
pub struct ResolveGraphCache {
    resolutions: Arc<DashMap<(Arc<str>, Arc<str>), Option<PathBuf>>>,  // (base_dir, specifier) → resolved
    sources: Arc<DashMap<PathBuf, CachedSource>>,
    tsconfigs: Arc<DashMap<PathBuf, CachedTsConfig>>,
    dependencies: Arc<DashMap<DependencyCacheKey, Arc<[PathBuf]>>>,     // dep lists cached
    stable_snapshot: bool,  // for_build(): skip metadata revalidation — filesystem immutable
}
```

- DashMap: 64 shards ภายใน, `RwLock` ต่อ shard ไม่มีการแย่งชิง `Mutex` เดียว
- **`MMAP_THRESHOLD_BYTES = 65536`** (64KB): ไฟล์ที่ >=64KB ใช้ `memmap2::Mmap` fallback เป็น
  `fs::read_to_string` ถ้า mmap ล้มเหลว
- Source cache ตรวจสอบ `modified_time + len` ก่อนคืนค่ารายการที่เก่า

### ลำดับการแก้ไข (ต่อ specifier)

```
1. ฮุก Plugin resolve_id()              — รายการแรกที่ตรงชนะ
2. พาธสัมพัทธ์ (./, ../)               → การลองนามสกุล (20 รูปแบบ)
3. พาธสัมบูรณ์ (/)                     → project_root.join()
4. tsconfig paths / baseUrl             → alias แบบ @/Component
5. Bare specifier                       → map "exports" ของ package.json
6. Fallback เทียบกับโปรเจกต์            → root.join(specifier)
```

### การลองนามสกุล (`resolve_file_candidate`)

ตามลำดับ:

```
<exact_path>
<path>.ts, .tsx, .js, .jsx, .mts, .cts, .mjs, .cjs, .md, .mdx
<path>/index.ts, index.tsx, index.js, index.jsx, index.mts, index.cts, index.mjs, index.cjs, index.md, index.mdx
```

การกรอง asset: `.css`/`.scss`/`.sass` ที่ไม่ใช่ CSS module ถูกแยกจาก dependency edges (side-effect
เท่านั้น, ไม่เพิ่มในกราฟ)

### การแก้ไข package exports (`PackageJsonValue`)

```rust
enum PackageJsonValue {
    Null,                              // Explicitly blocked (no access)
    String(String),
    Array(Vec<Self>),                  // Fallback array (try each)
    Object(Vec<(String, Self)>),       // ORDERED entries (preserving declaration order matters)
    Unsupported,                        // boolean, number
}
```

**`resolve_package_exports(pkg_name, export_key, target)`**:

1. อ่าน `node_modules/<pkg>/package.json`, แยกฟิลด์ `exports`
2. **การจับคู่ subpath** (คีย์ที่ขึ้นต้นด้วย `.`):
   - จับคู่ตรงก่อน
   - จับคู่ wildcard `*`: prefix + suffix ยาวที่สุดรวมกันชนะ
   - ค่า `Null` → ถูกบล็อก
3. **การจับคู่เงื่อนไข** (objects ที่ใช้เงื่อนไขเป็นคีย์):
   - target ไคลเอ็นต์: `["browser", "import", "module", "default", "require"]`
   - target SSR: `["node", "import", "module", "default", "require"]`
   - เงื่อนไขแรกที่มีค่าที่ไม่ใช่ null ชนะ
4. **การตรวจสอบ target**: target ที่แก้ไขแล้วต้องเริ่มต้นด้วย `./`, ไม่มี `..` หลบหนี,
   อยู่ภายในรากแพ็กเกจ

### tsconfig paths

```rust
pub struct TsConfigPaths {
    pub config_dir: PathBuf,
    pub base_url: Option<PathBuf>,
    pub paths: Vec<(String, Vec<String>)>,  // e.g. ("@/*", ["./src/*"])
}
```

อัลกอริทึมการแก้ไข:

1. สำหรับแต่ละ `(pattern, targets)`:
   - ถ้า alias จับคู่ตรง → แก้ไข targets ตามที่ให้
   - ถ้า alias มี suffix `*` → จับคู่ prefix, แยก suffix, แทนที่ `*` ใน targets ด้วย suffix
2. ถ้าไม่มี alias ที่จับคู่และ specifier เป็น bare + มี `base_url` → `base_url.join(specifier)`
3. ตรวจสอบระบบไฟล์สำหรับแต่ละ candidate ที่แก้ไขแล้ว

### `collect_deps_cached` / `collect_deps_uncached`

1. แยก specifiers ผ่าน `ast::parse_module(source).import_specifiers()`
2. สำหรับแต่ละ specifier:
   - ฮุก Plugin `resolve_id` (รายการแรกที่ตรงชนะ)
   - Relative → `resolve_specifier` พร้อมลองนามสกุล
   - Absolute → `resolve_project_specifier`
   - Bare → tsconfig paths → package exports → fallback เทียบกับโปรเจกต์
   - External ถ้าทั้งหมดล้มเหลวและเป็น bare
3. แคชรายการ dependencies (blake3 ของ source + target) ถ้าไม่มี plugins ที่ทำงาน

---

## 2. ตัวคอมไพล์ (Compiler — `compiler.rs`)

### `compile_graph_with_cache(graph, input, cache) → Vec<CompiledModule>`

คอมไพล์แบบขนานผ่าน `rayon::par_iter()` โมดูลถูกจัดสรรตามประเภทไฟล์:

| File type                       | Pipeline                                                           |
| ------------------------------- | ------------------------------------------------------------------ |
| `.module.css`/`.module.scss`    | `compile_css_module` → `css_module_javascript` → ข้าม Oxc          |
| `.md`/`.mdx`                    | `compile_content_module` → plugin transforms → Oxc                 |
| `.js`/`.mjs`/`.cjs` (ไม่มี JSX) | เฉพาะ plugin transforms → ข้าม Oxc                                 |
| พาธ virtual `ruvyxa:`           | เฉพาะ plugin transforms → ข้าม Oxc                                 |
| `.ts`/`.tsx`/`.jsx` (มี JSX)    | Plugin → `ast::parse_module` → ค้นหาแคช → `transform_with_options` |

### `CompiledModule`

```rust
pub struct CompiledModule {
    pub path: PathBuf,             // Canonical or virtual ("ruvyxa:bundle-entry.tsx")
    pub js: String,                // Plain JS after Oxc transformation
    pub deps: Vec<PathBuf>,        // Resolved absolute dependency paths
    pub is_external: bool,         // From node_modules
    pub cache_hit: bool,           // CompileCache hit
}
```

### สายการทำงาน Oxc

```rust
let source_type = SourceType::mjs()
    .with_typescript(true)
    .with_jsx(has_jsx);

// Parse
let parser = Parser::new(&allocator, &source, source_type);
let result = parser.parse();

// Semantic analysis
let semantic = SemanticBuilder::new_compiler()
    .with_enum_eval(true)
    .build(&result.program);

// Transform options
let mut options = TransformOptions::default();
options.jsx.runtime = match jsx_runtime {
    Classic => OxcJsxRuntime::Classic,
    Automatic => OxcJsxRuntime::Automatic,
};
options.jsx.jsx_plugin = has_jsx;
options.jsx.throw_if_namespace = false;
options.jsx.pure = false;
options.typescript.optimize_const_enums = false;
options.typescript.optimize_enums = false;

// Transform
Transformer::new(&allocator, Path::new("ruvyxa:module.tsx"), &options)
    .build_with_scoping(semantic.semantic.into_scoping(), &mut result.program);

// Codegen
let code = Codegen::new().build(&result.program).code;
```

**Pre-pass การลบ Decorator**: รูปแบบ `@Decorator(args?)` แบบเก่าถูกลบผ่านสแกนเนอร์ระดับอักขระ
(จัดการ strings, parens ซ้อนกัน, รักษาตำแหน่งบรรทัด) ใช้ก่อนการแยกวิเคราะห์ Oxc

### `transform(source, has_jsx) → Result<String, String>`

ชอร์ตแฮนด์ที่ใช้ JSX แบบ classic เรียก `transform_with_options`

### `transform_with_options(source, has_jsx, jsx_runtime) → Result<String, String>`

สายการทำงาน Oxc เต็มตามด้านบน คืนค่าเป็นสตริง JS ธรรมดา

### `CompileCache` (`cache.rs`)

```rust
const MEMORY_CACHE_LIMIT: usize = 512;
const COMPILER_VERSION: &str = concat!("ruvyxa_bundler:", env!("CARGO_PKG_VERSION"), ":ast-plugin-pipeline");

pub struct CompileCache {
    cache_dir: PathBuf,                      // .ruvyxa/cache/bundler/
    enabled: bool,
    namespace: String,
    memory: Arc<Mutex<HashMap<String, MemEntry>>>,  // LRU: 512 entries
    generation: AtomicU64,                    // LRU timing
}

pub enum CacheLookup { Hit(String), Miss(String) }
```

**คีย์แคช**:
`blake3(source + "\0" + has_jsx_flag + "\0" + jsx_runtime + "\0" + COMPILER_VERSION + "\0" + namespace)[..32]`
เป็น hex

**พื้นที่จัดเก็บดิสก์**: `.ruvyxa/cache/bundler/<key>.js` การเขียนแบบอะตอมมิกผ่าน temp file + rename

**การขับ LRU**: ตัวนับ `generation` เพิ่มขึ้นทุกครั้งที่เข้าถึง ขับ `last_used` ที่น้อยที่สุดเมื่อ
memory > 512

---

## 3. ตัวตรวจสอบขอบเขต (Boundary Checker — `boundary.rs`)

### `check(modules, input, out: &mut Vec<Diagnostic>) → Result<()>`

ตรวจสอบกราฟที่คอมไพล์แล้วซ้ำสำหรับการละเมิดขอบเขต สองโหมด:

#### การตรวจสอบบันเดิลไคลเอ็นต์

| Check                      | วิธี                                                                                                                                                    | รหัส    | ความรุนแรง |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | ---------- |
| `"server-only"` import     | `ast::parse_module(source).imports` มี `specifier == "server-only"`                                                                                     | RUV1007 | Error      |
| การเข้าถึง env ส่วนตัว     | สแกนเนอร์ระดับไบต์สำหรับ `process.env.<ID>` และ `process.env['<ID>']` ข้าม strings, comments, template literals อนุญาต `NODE_ENV` และ `RUVYXA_PUBLIC_*` | RUV1008 | Error      |
| import ไดเรกทอรี `server/` | เส้นทางไฟล์ขึ้นต้นด้วย `<root>/server/` (เฉพาะระดับรากโปรเจกต์, ไม่ใช่ `app/.../server/`)                                                               | RUV1010 | Error      |

#### การตรวจสอบบันเดิล SSR

| Check                  | วิธี                                                                | รหัส    | ความรุนแรง |
| ---------------------- | ------------------------------------------------------------------- | ------- | ---------- |
| `"client-only"` import | `ast::parse_module(source).imports` มี `specifier == "client-only"` | RUV1009 | Warning    |

### `private_env_reads(source: &str) → Vec<String>`

สแกนเนอร์ระดับไบต์ รู้จำ:

- `process.env.NAME` — จับ NAME
- `process.env["NAME"]` หรือ `process.env['NAME']` — จับ NAME

จัดการ:

- สตริงลิเทอรัล (ข้ามไป, แต่ `${expr}` จะถูกเรียกซ้ำ)
- เทมเพลตลิเทอรัล (นับความลึกสำหรับ `${}` ที่ซ้อนกัน)
- คอมเมนต์แบบบล็อก `/* */`
- คอมเมนต์แบบบรรทัด `//`

---

## 4. ตัวเชื่อมโยง (Linker — `linker.rs`)

### `detect_cycles(modules) → Result<()>`

DFS พร้อมติดตามสแต็กอย่างชัดเจน:

- `visited: BTreeSet<PathBuf>` (เซตดำ)
- `stack: Vec<PathBuf>` (เซตเทา, เส้นทางปัจจุบัน)
- ถ้า `stack.contains(path)` → แยกวงจร → `BundleError::CircularDependency { cycle_string }`

### `ordered_project_modules(modules) → Vec<&CompiledModule>`

การเรียงลำดับแบบทอพอโลยีด้วย DFS `visiting: BTreeSet` (ก่อน), `visited: BTreeSet` (หลัง)
ผลักโมดูลหลังจากที่ dep ทั้งหมดถูกเยี่ยมชม (post-order) ผลลัพธ์: dependencies ก่อน importers

### `module_id(path: &Path) → String`

`format!("__ruv_{:016x}__", blake3(path_str)[..8])` — กำหนดตายตัว, ใช้พาธ ตัวอย่าง:
`__ruv_abcdef1234567890__`

### การเขียน import/export ใหม่

สแกนเนอร์ทีละบรรทัด เปลี่ยนรูป:

| Pattern                                | การเขียนใหม่                                                   |
| -------------------------------------- | -------------------------------------------------------------- |
| `import "./styles.css"`                | `// [bundled] import "./styles.css"`                           |
| `import Default from "./mod"`          | `const Default = __ruv_xxx__.default`                          |
| `import { a, b } from "./mod"`         | `const a = __ruv_xxx__.a; const b = __ruv_xxx__.b`             |
| `import * as ns from "./mod"`          | `const ns = __ruv_xxx__`                                       |
| `import Default, { a } from "./mod"`   | `const Default = __ruv_xxx__.default; const a = __ruv_xxx__.a` |
| `import Default, * as ns from "./mod"` | `const Default = __ruv_xxx__.default; const ns = __ruv_xxx__`  |
| `import type { T } from "./mod"`       | คอมเมนต์ออก                                                    |
| `import "side-effect"` (external)      | คงไว้ถ้า `!drop_external_imports`                              |

| `export default expr` | `__exports.default = expr` | | `export default function Foo() {}` |
`function Foo() {} __exports.default = Foo` | | `export { a, b as c } from "./mod"` |
`__exports.a = __ruv_xxx__.a; __exports.c = __ruv_xxx__.b` | | `export * from "./mod"` |
`Object.assign(__exports, __ruv_xxx__)` | | `export const name = val` |
`const name = val; __exports.name = name` | | `export function name() {}` |
`function name() {} __exports.name = name` | | `export { a, b }` |
`__exports.a = a; __exports.b = b` |

| `require("./module")` | `__ruv_xxx__` | | `import("./lazy")` (ใน chunk map) |
`import("./chunk.<hash>.js").then(m => m.default)` | | `import("./lazy")` (ไม่อยู่ใน chunk map) |
`Promise.resolve(__ruv_xxx__)` |

### IIFE wrapper (ต่อโมดูล)

```javascript
var __ruv_<hex16>__ = (function() {
  "use strict";
  var __exports = {};
  var module = { exports: __exports };
  var exports = module.exports;
  var process = globalThis.process || { env: { NODE_ENV: "production" } };
  // ... rewritten module source ...
  return module.exports;
})();
```

### เอาต์พุต SSR

ต่อท้าย `export const render = __ruv_<hex16>__.render;` หลังรายการ IIFE

### Shared module registry

```
// shared.js:
var __ruvyxa_shared_modules__ = globalThis.__RUVYXA_SHARED_MODULES__ || (globalThis.__RUVYXA_SHARED_MODULES__ = {});
__ruvyxa_shared_modules__["<module_id>"] = <module_exports>;

// route bundle:
var __shared_<id>__ = __ruvyxa_shared_modules__["<module_id>"];
```

### `link_parallel(modules, input) → Result<String>`

การเชื่อมโยงแบบขนานสำหรับ >=8 โมดูล: สร้างส่วน IIFE ขนานผ่าน `rayon::par_iter()`
แล้วต่อผลลัพธ์ตามลำดับ

### ฟังก์ชันเริ่มต้น

- **`link()`**: sequential เรียก `detect_cycles` + `link_inner`
- **`link_parallel()`**: ขนานสำหรับ >=8 โมดูล
- **`link_parallel_with_dynamic_imports()`**: ส่ง chunk map สำหรับการเขียน `import()` ใหม่
- **`link_parallel_with_dynamic_imports_and_shared_modules()`**: จัดการ shared registry ด้วย
- **`link_shared_route_modules()`**: สร้างเฉพาะ shared registry

---

## 5. ตัวย่อขนาด (Minifier — `minifier.rs`)

### `minify(source, target) → Result<String>`

### `minify_with_options(source, target, tree_shaking) → Result<String>`

Oxc AST minifier:

- **มี tree-shaking**: `MinifierOptions::default()` — mangle + compress เต็ม
- **ไม่มี tree-shaking**:
  `MinifierOptions { mangle: default, compress: Some(CompressOptions::safest()) }` +
  `CodegenOptions::minify()`

### `tree_shake_exports(source) → String`

การตัด export ระดับบรรทัด:

1. **`collect_used_members(source)`**: สแกนหารูปแบบ `__ruv_<hex16>__.<member>` ทั้งหมดในบันเดิล →
   `BTreeSet<"module_id.member">`
2. **การประมวลผลต่อบรรทัด**:
   - ติดตามโมดูลปัจจุบันผ่านรูปแบบ `var __ruv_xxx__ = (function() {`
   - สิ้นสุดโมดูลที่ `})();`
   - ถ้าบรรทัดคือ `__exports.<name> = <name>;` และ `<name>` ไม่อยู่ใน `used_members` และ
     `<name> != "default"`: แทนที่ด้วย `// [tree-shaken] __exports.<name> = <name>;`

Default exports ถูกเก็บไว้เสมอ

### `fold_production_node_env(source) → String`

การพับสาขา `NODE_ENV` แบบ CommonJS ระดับข้อความสำหรับ client bundles ของ node_modules รูปแบบที่รู้จำ
(หลังจาก normalize quotes+whitespace):

```
process.env.NODE_ENV === "production"     → keep consequent
process.env.NODE_ENV == "production"      → keep consequent
"production" === process.env.NODE_ENV     → keep consequent
"production" == process.env.NODE_ENV      → keep consequent
process.env.NODE_ENV !== "production"     → keep alternative
process.env.NODE_ENV != "production"      → keep alternative
"production" !== process.env.NODE_ENV     → keep alternative
"production" != process.env.NODE_ENV      → keep alternative
```

ลูปแบบมีขอบเขต (สูงสุด 64 iteration): หาบล็อก `if(cond){...}[else{...}]` ที่ซ้อนในสุดที่ตรงกัน
แทนที่ด้วยเนื้อหาสาขาที่เหมาะสม จัดการ nested guards, strings, comments, regexes
ด้วยการจับคู่ตัวคั่นเต็มรูปแบบ

---

## 6. การแบ่งเป็นชิ้น (Chunking — `chunking.rs`)

### `plan_dynamic_chunk_files(compiled, entry) → BTreeMap<PathBuf, String>`

**ตรรกะการซ้อนทับของ transitive closure**:

1. `dynamic_roots(compiled)` — หาโมดูลที่ถูกเรียกโดย `import()`
2. สำหรับแต่ละ root, คำนวณ **static transitive closure** (ตาม static import, re-export, side-effect,
   CommonJS edges; ไม่รวม dynamic edges)
3. **เกณฑ์การแยก**: root จะถูกแยกเป็น chunk ของตัวเองก็ต่อเมื่อ closure ของมัน:
   - ไม่ซ้อนทับกับ static closure ของ entry
   - ไม่ซ้อนทับกับ closure ของ dynamic root อื่นใด
   - ถ้าซ้อนทับกับสิ่งใด → root ยังคงอยู่ในบันเดิล entry

### `graph_fingerprint(compiled) → String`

Blake3 hash ของพาธ + เนื้อหา JS ของโมดูลที่ไม่ใช่ external ทั้งหมด เป็น input สำหรับ hash ชื่อไฟล์
chunk — เมื่อ dependency ใดเปลี่ยนแปลง ชื่อไฟล์ chunk ทั้งหมดจะเปลี่ยน (หลีกเลี่ยง stale reference)

### ชื่อไฟล์ Chunk

`format!("chunk.{:016x}.js", blake3(fingerprint + "\0" + root_path)[..8])`

### `dynamic_import_chunks(compiled, dynamic_import_files) → Vec<DynamicImportChunk>`

คืนค่า `{ importer, module, file }` สำหรับ dynamic import ที่แบ่งเป็น chunk แต่ละรายการ

### `build_dynamic_output_chunks(compiled, input, dynamic_import_files) → Vec<OutputChunk>`

สำหรับแต่ละ chunk: เชื่อมโยงกับ dynamic sub-imports, ต่อท้าย `export default <module_id>;`,
ย่อขนาดถ้าเลือก (ไม่ tree-shaking)

---

## 7. เอาต์พุต (Output — `output.rs`)

### `build_entry_source(input) → (String, String)` (`output.rs:32`)

สร้างโมดูลเริ่มต้นแบบ virtual คืนค่า `(entry_source, entry_label="ruvyxa:bundle-entry.tsx")`

**Client entry**:

```javascript
import React from "react";
import { hydrateRoot } from "react-dom/client";
import Page from "<page_absolute_path>";
import Layout0 from "<layout0_absolute_path>";
// ... more layouts ...

const params = globalThis.__RUVYXA_ROUTE_PARAMS__ ?? {};
const currentPath = globalThis.__RUVYXA_REQUEST_PATH__ ?? "/";

let tree = React.createElement(Page, { params, requestPath: currentPath });
for (const Layout of [Layout0, ...].reverse()) {
    tree = React.createElement(Layout, null, tree);
}

if (globalThis.__RUVYXA_ROOT__) {
    globalThis.__RUVYXA_ROOT__.render(tree);
} else {
    globalThis.__RUVYXA_ROOT__ = hydrateRoot(document, tree);
}
window.__RUVYXA_HYDRATED = true;
```

Layouts วนซ้ำในลำดับ **ย้อนกลับ** — layout ชั้นนอกสุดห่อหุ้มชั้นในสุด

**SSR entry**:

```javascript
import React from "react";
import { renderToString } from "react-dom/server";
import Page from "<page_absolute_path>";
import Layout0 from "<layout0_absolute_path>";
// ...

export async function render(ctx) {
    let tree = React.createElement(Page, {
        params: ctx.params ?? {},
        requestPath: ctx.path
    });
    for (const Layout of [Layout0, ...].reverse()) {
        tree = React.createElement(Layout, null, tree);
    }
    return "<!doctype html>" + renderToString(tree);
}
```

### `wrap(linked, input) → String`

| Target     | พฤติกรรม                                                                                                        |
| ---------- | --------------------------------------------------------------------------------------------------------------- |
| **Client** | ส่งผ่าน (linker สร้างโค้ดแบบ IIFE-wrapper แล้ว; เบราว์เซอร์โหลดผ่าน `<script type="module">`)                   |
| **SSR**    | เติมหน้า `// Ruvyxa SSR bundle\n` linker ย้าย ESM imports ไปไว้ข้างบนแล้ว + ต่อท้าย `export const render = ...` |

---

## 8. สแกนเนอร์ AST (`ast.rs`)

### `parse_module(source: &str) → ModuleAst`

สแกนเนอร์ระดับไบต์ (ไม่ใช่ parser เต็มรูปแบบ) เดินซอร์สแบบเชิงเส้น ติดตามข้อเท็จจริง:

```rust
pub struct ModuleAst {
    pub imports: Vec<ImportEdge>,
    pub exports: Vec<String>,        // Named export identifiers
    pub has_jsx: bool,
    pub has_typescript: bool,
    pub has_decorators: bool,
    pub has_enums: bool,
}

pub struct ImportEdge {
    pub specifier: String,
    pub kind: ImportKind,            // Static | Dynamic | Require | ReExport | SideEffect
}
```

**ข้อเท็จจริงที่สแกน**:

- คีย์เวิร์ด `import` → กำหนด `ImportKind`:
  - `import(` → `Dynamic`
  - `import "..."` หรือ `import '...'` → `SideEffect`
  - `import type` → ข้ามทั้งหมด (ไม่ใช่ import รันไทม์)
  - `import {x} from "..."` → `Static` (ใช้ `find_from_specifier`)
- `export {x} from "..."` → `ReExport`
- `require("...")` → `Require` (ถ้าไม่นำหน้าด้วย `.`)
- `@` ในบรรทัดของตัวเอง → `has_decorators = true`
- `<` ตามด้วย identifier → `has_jsx = true`
- คีย์เวิร์ด `enum` → `has_enums = true`, `has_typescript = true`
- `interface`, `type`, `satisfies`, `implements`, `declare`, `abstract`, `readonly`, `public`,
  `private`, `protected`, `override` → `has_typescript = true`
- `as` หลังจาก non-whitespace → `has_typescript = true`
- `export default/async function/class/const/let/var <name>` → จับชื่อ export

ข้าม: strings (ทุกประเภท quote), comments (บรรทัด + บล็อก), whitespace

---

## 9. CSS Modules (`style_module.rs`)

### `is_css_module_path(path) → bool`

ลงท้ายด้วย `.module.css`, `.module.scss`, หรือ `.module.sass` (ไม่สนตัวพิมพ์เล็กใหญ่)

### `is_sass_path(path) → bool`

ลงท้ายด้วย `.scss` หรือ `.sass` (ไม่สนตัวพิมพ์เล็กใหญ่)

### `compile_sass_file(path, project_root) → Result<String>`

ใช้ crate `grass`:
`Options::default().style(Expanded).load_path(&project_root).load_path(project_root.join("node_modules"))`

### `compile_css_module(path, project_root) → Result<CssModule>`

ถ้า Sass → `compile_sass_file` ก่อน, แล้ว `scope_css_module` ถ้า CSS → `scope_css_module` โดยตรง

### `scope_css_module(css, path, project_root) → CssModule`

สแกนเนอร์ระดับอักขระพร้อม state machine สถานะที่ติดตาม:

- `quote: Option<char>` — ภายในสตริงลิเทอรัล
- `in_comment: bool` — ภายใน `/* */`
- `block_allows_rules: Vec<bool>` — สแต็กติดตามว่าบล็อกปัจจุบันอนุญาต selector rules หรือไม่
- `rule_local_classes: Vec<Vec<String>>` — คลาสท้องถิ่นต่อกฎที่ซ้อนกัน
- `prelude: String` — ข้อความ selector ก่อน `{`

**การแปลง**:

1. `.local-class` → `.scoped-class__hash`
2. `:global(.selector)` → `.selector` (ส่งผ่านโดยไม่แก้ไข)
3. `composes: class1 class2` → ต่อท้ายชื่อคลาสที่ถูก scoped กับเซต `class` ของกฎที่บรรจุ

### `scoped_class_name(path, project_root, local) → String`

การตั้งชื่อแบบกำหนดตายตัว:

1. `relative` = พาธ normalized ตัวพิมพ์เล็กสัมพัทธ์กับ project_root (เครื่องหมายทับไปข้างหน้า)
2. `digest` = fnv1a_64(format!("{relative}:{local}"))
3. `stem` = file_stem ที่เป็นตัวอักษรและตัวเลขเท่านั้น (ลบ suffix `.module`)
4. ผลลัพธ์: `{stem}_{local}__{digest:016x}`

### `css_module_javascript(module) → Result<String>`

คืนค่า: `export default {"local":"scoped","other":"scoped_other__hex"};`

---

## 10. การคอมไพล์เนื้อหา (Content Compilation — `content.rs`)

### `compile_content_module(source, path) → Result<String>`

1. `split_frontmatter(source)` → แยก YAML ระหว่างตัวคั่น `---`
2. `parse_frontmatter(yaml)` → `serde_yaml_ng::from_str<Value>` ต้องเป็น mapping
3. แยกตามนามสกุล:
   - `.md`: `markdown::to_mdast(body, ParseOptions::gfm())` → สร้าง `createElement` ของ React
   - `.mdx`: `markdown::to_mdast(body, mdx_parse_options())` → รวบรวม ESM + สร้าง `createElement`
     ของ React

### ตัวเลือกการแยกวิเคราะห์ MDX

```
Constructs: GFM ลบ autolink, code_indented, html_flow, html_text
บวก: mdx_esm, mdx_expression_flow, mdx_expression_text, mdx_jsx_flow, mdx_jsx_text
mdx_esm_parse: ใช้ Oxc Parser (TypeScript + JSX MJS) เพื่อตรวจสอบบล็อกไวยากรณ์ ESM
```

### โมดูลเอาต์พุต

```javascript
import React from "react";
// ... MDX ESM blocks (if any) ...
export const frontmatter = { /* parsed YAML */ };
export const meta = frontmatter;
export const headings = [ { depth, text, id }, ... ];
export const contentFormat = "md" | "mdx";
export default function RuvyxaContentPage({ components = {} } = {}) {
    return React.createElement("article", { className: "ruvyxa-content", ... }, ...children);
}
```

**การ Deduplicate ชื่อ export**: ถ้า MDX ESM export `frontmatter`, `meta`, `headings` หรือ
`contentFormat` อยู่แล้ว, auto-generated export จะถูกละเว้น

### การลดรูป AST node

ทุกโหนด Markdown/MDX ถูกลดรูปเป็น `React.createElement`:

- Intrinsic HTML → สตริง `"tag"`
- คอมโพเนนต์ MDX → `(components["Tag"] || "Tag")` สำหรับตัวพิมพ์เล็ก, bare identifier
  สำหรับตัวพิมพ์ใหญ่
- Dotted custom elements → ชื่อดิบ
- Fragment → `React.Fragment`
- นิพจน์ MDX → `({expression})` (โหนดนิพจน์แบบอินไลน์)
- HTML → `React.createElement("span", null, escaped_string)` (XSS-safe escaping)
- ตาราง → `<table><thead>...</thead><tbody>...</tbody></table>` พร้อมสไตล์การจัดตำแหน่ง
- บล็อกโค้ด → `<pre><code className="language-xxx">...</code></pre>`
- รูปภาพ → `<img src="..." alt="..." loading="lazy" decoding="async" />`
- Checkbox → `<input type="checkbox" disabled readOnly />`
- คณิตศาสตร์ → `<span className="math math-inline">` / `<div className="math math-display">`
- เชิงอรรถ → `<aside role="doc-footnote">` พร้อมลิงก์กลับ

### แคชเนื้อหา

```rust
static CONTENT_MODULE_CACHE: OnceLock<Mutex<ContentModuleCache>>;
// HashMap<String, Arc<str>> + VecDeque<String> (insertion order)
// Max 512 entries, LRU eviction
// Key: blake3(extension + "\0" + source).to_hex()
```

---

## 11. ฮุกบิลด์ (Build hooks — `hooks.rs`)

### trait `BuildHooks`

```rust
pub trait BuildHooks: Send + Sync {
    fn name(&self) -> &str;

    fn resolve_id(
        &self, specifier: &str, importer: Option<&Path>, ctx: &BuildHookContext
    ) -> Result<Option<PathBuf>>;  // default: Ok(None)

    fn transform(
        &self, code: &str, id: &Path, ctx: &BuildHookContext
    ) -> Result<Option<TransformResult>>;  // default: Ok(None)
}

pub struct BuildHookContext {
    pub project_root: PathBuf,
    pub importer: Option<PathBuf>,
    pub target: BundleTarget,
}

pub struct TransformResult {
    pub code: String,
    pub map: Option<String>,
}
```

### `BuildHookPipeline`

```rust
pub struct BuildHookPipeline {
    hosts: Arc<Vec<Arc<dyn BuildHooks>>>,
}
```

ฮุกทำงานตามลำดับ `resolve_id` รายการแรกที่ตรงชนะ `transform_with_map` เชื่อมโยงการแปลง, รักษา source
map ที่ไม่ใช่ None รายการสุดท้าย

---

## 12. โมดูลเสริม

### `BundleContext` (`context.rs`)

```rust
pub struct BundleContext {
    compile_cache: CompileCache,
    graph_cache: ResolveGraphCache,
    incremental: IncrementalGraphCache,
    build_hooks: BuildHookPipeline,
}
```

คอนสตรัคเตอร์: `new(project_root)`, `with_caches(...)`, `with_all_caches(...)`, และ
`with_build_hooks(...)`

### `IncrementalGraphCache` (`incremental.rs`)

```rust
pub struct IncrementalGraphCache {
    manifest_path: PathBuf,          // .ruvyxa/cache/graph/manifest.json
    previous: GraphManifest,
    current: GraphManifest,
    enabled: bool,
}

pub struct CachedModuleEntry {
    pub content_hash: String,        // blake3[..32]
    pub size: u64,
    pub mtime_secs: u64,
    pub deps: Vec<PathBuf>,
    pub compile_key: Option<String>,
}
```

ค่า validity: version == 1, dependency_hash match, render_context_hash match,
ลายนิ้วมือไฟล์ทั้งหมดตรงกัน Dirty set: BFS บนกราฟ reverse dependency จากพาธที่เปลี่ยน

### `SourceMapBuilder` (`sourcemap.rs`)

```rust
pub struct SourceMapBuilder {
    file: String,
    sources: Vec<String>,
    sources_content: Vec<Option<String>>,
    mappings: Vec<Mapping>,
    source_root: PathBuf,
    names: Vec<String>,
    ignore_list: Vec<u32>,
}

pub struct Mapping {
    pub gen_line: u32, pub gen_col: u32,
    pub source_idx: u32, pub orig_line: u32, pub orig_col: u32,
}
```

รองรับ: identity mappings, vlq encoding/decoding, `x_google_ignoreList`, การนำเข้า v3 source maps
พร้อม line offset
