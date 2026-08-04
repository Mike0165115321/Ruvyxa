# CLI และ Build Pipeline (`ruvyxa_cli`)

**ไฟล์**: `crates/ruvyxa_cli/src/main.rs` (~7500 บรรทัด) พร้อมโมดูลข้างเคียง
`crates/ruvyxa_cli/src/image_optimizer.rs` (443 บรรทัด), `image_usage.rs`, และ `site_discovery.rs`

การกระจายคำสั่งผ่าน clap, การโหลดค่าตั้งค่าจาก `ruvyxa.config.ts` (ประเมินผลโดย Node/Bun runtime
ที่เลือก), การจัดลำดับงาน build, และการปรับแต่งภาพให้เหมาะสม

---

## โครงสร้างคำสั่ง

```rust
#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

pub enum Command {
    Dev(ServerArgs),              // --root, --host, --port
    Build(BuildArgs),             // --root, --target
    Check(ProjectArgs),           // --root
    Start(ServerArgs),            // --root, --host, --port
    Preview(ServerArgs),          // --root, --host, --port
    Routes(ProjectArgs),          // --root
    Analyze(ProjectArgs),         // --root
    Add(AddArgs),                 // --root, --template, --force
    Doctor(ProjectArgs),          // --root
    Clean(ProjectArgs),           // --root
    Trace(TraceArgs),             // <route> --root
    Bench(BenchArgs),             // --root, --samples (3), --json
    TestParity(ProjectArgs),      // --root  (alias: parity)
    Plugin(PluginArgs),           // plugin create <name> [--dir <relative-path>]
}

pub struct ProjectArgs { pub root: PathBuf }            // default "."
pub struct ServerArgs { pub root: PathBuf, pub host: Option<String>, pub port: Option<u16>, pub runtime: Option<CliRuntime> }
pub struct BuildArgs { pub root: PathBuf, pub target: Option<BuildTarget>, pub adapter: Option<String>, pub runtime: Option<CliRuntime> }
pub struct AddArgs { pub templates: Vec<AddTemplate>, pub root: PathBuf, pub runtime: Option<CliRuntime>, pub force: bool }
pub struct TraceArgs { pub route: String, pub root: PathBuf }
pub struct BenchArgs { pub root: PathBuf, pub samples: usize, pub json: bool }
pub struct PluginArgs { pub command: PluginCommand }
pub enum PluginCommand { Create(PluginCreateArgs) }
pub struct PluginCreateArgs { pub name: String, pub root: PathBuf, pub dir: Option<PathBuf> }

```

| คำสั่ง          | หน้าที่                                            |
| --------------- | -------------------------------------------------- |
| `dev`           | เริ่ม dev server พร้อม HMR                         |
| `build`         | Production build ไปยัง `.ruvyxa/`                  |
| `check`         | `tsc --noEmit` + `test:parity`                     |
| `start`         | ให้บริการ production build                         |
| `preview`       | ดูตัวอย่าง production build ในเครื่อง              |
| `routes`        | แสดงตารางเส้นทางที่ค้นพบ                           |
| `analyze`       | ตรวจสอบ routes/imports/boundaries                  |
| `add`           | สร้างโค้ดเริ่มต้นสำหรับฟอร์ม ตารางข้อมูล หรือ Auth |
| `doctor`        | ตรวจสอบการตั้งค่าโปรเจกต์                          |
| `clean`         | ลบ `.ruvyxa/`                                      |
| `trace`         | ตรวจสอบเส้นทางเดียวตาม path (JSON)                 |
| `bench`         | ทดสอบประสิทธิภาพ (discovery, analysis, build)      |
| `test:parity`   | เปรียบเทียบ route และ smoke renders                |
| `plugin create` | สร้างแพ็กเกจ plugin ที่เผยแพร่ได้                  |

---

## การ Normalize อาร์กิวเมนต์

ก่อนที่ clap จะแยกวิเคราะห์อาร์กิวเมนต์ของโปรเซส `normalized_cli_args()`
จะปรับตัวพิมพ์ของชื่อคำสั่งย่อยและ flag ให้เป็นรูปแบบมาตรฐานด้วยการจับคู่แบบไม่สนตัวพิมพ์เล็ก-ใหญ่
ทำให้ `ruvyxa BUILD --Target node` มีผลเทียบเท่ากับ `ruvyxa build --target node`

---

## คำสั่ง Dev

```rust
fn dev(args: ServerArgs) -> Result<()> {
    let config = load_project_config(&args.root)?;
    let server_config = dev_server_config(&args, &config);
    ruvyxa_dev_server::serve(server_config).await
}
```

แมปฟิลด์ของ `ProjectConfig` ไปยัง `ServerConfig::dev()`

## คำสั่ง Build

```rust
fn build(args: BuildArgs) -> Result<()> {
    build_with_output(args, true)  // produce output = true
}
```

Pipeline เต็มรูปแบบ (ดู [Build Pipeline](#build-pipeline) ด้านล่าง)

## คำสั่ง Check

```rust
fn check(args: ProjectArgs) -> Result<()> {
    run_typecheck(&args.root)?;      // tsc --noEmit
    test_parity(ProjectArgs { ... }).await  // full parity test
}
```

## คำสั่ง Start

```rust
fn start(args: ServerArgs) -> Result<()> {
    let config = load_project_config(&args.root)?;
    let server_config = production_server_config(&args, &config);
    // app_dir → out_dir/server/app
    // public_dir → out_dir/assets
    ruvyxa_dev_server::serve(server_config).await
}
```

---

## ระบบค่าตั้งค่า

### การโหลดสองเฟส

**เฟส 1: การประเมินผลด้วย Node/Bun**

```rust
fn load_project_config(root: &Path) -> Result<ProjectConfig> {
    let renderer = find_runtime_script(root, "config-renderer.mjs")?;
    // If not found → return ProjectConfig::default() with hash "no-config"

    let output = Command::new(runtime.executable())
        .arg(&renderer)
        .arg(root)
        .output()?;

    let result: ConfigRendererOutput = serde_json::from_slice(&output.stdout)?;
    if !result.ok {
        return Err(RuvyxaError::Message(format!(
            "config evaluation failed: {} - {}",
            result.code.unwrap_or_default(),
            result.message.unwrap_or_default()
        )));
    }

    let mut config = result.config.unwrap_or_default();
    config.config_dependency_hash = result.dependency_hash.unwrap_or_default();
    config.validate_paths(root)?;
    Ok(config)
}
```

**ConfigRendererOutput**:

```rust
struct ConfigRendererOutput {     // #[serde(rename_all = "camelCase")]
    ok: bool,
    config: Option<ProjectConfig>,
    code: Option<String>,
    message: Option<String>,
    stack: Option<String>,
    dependency_hash: Option<String>,
}
```

`config-renderer.mjs` ประเมิน `ruvyxa.config.ts`, แมปผลลัพธ์ของ `defineConfig(...)`, ทำให้เป็น
serialized JSON ไปยัง stdout

**เฟส 2: การตรวจสอบด้วย Rust**

```rust
impl ProjectConfig {
    fn validate_paths(&self, root: &Path) -> Result<()> {
        validate_project_relative_path("appDir", &self.app_dir(), root)?;
        validate_project_relative_path("outDir", &self.out_dir(), root)?;
        for (i, entry) in self.css.entries.iter().enumerate() {
            validate_project_relative_path(&format!("css.entries[{}]", i), entry, root)?;
        }
        validate_bounded_limit("actionLimit", self.security.action_body_limit_bytes)?;
        validate_bounded_limit("apiLimit", self.security.api_body_limit_bytes)?;
        validate_plugin_response_limit(self.security.plugin_response_body_limit_bytes)?;
        validate_trusted_proxy_ips(&self.security.trusted_proxy_ips)?;
        self.parse_jsx_runtime()?;
        Ok(())
    }
}
```

- `validate_project_relative_path`: ปฏิเสธ absolute path, root-dir, parent-dir ปฏิเสธ path traversal
  (`..`)
- `validate_bounded_limit`: ต้อง > 0 และ ≤ `MAX_BODY_LIMIT`
- `validate_plugin_response_limit`: ต้อง > 0 และ ≤ `MAX_PLUGIN_RESPONSE_BODY_LIMIT_BYTES`
- `validate_trusted_proxy_ips`: แต่ละรายการแยกเป็น `std::net::IpAddr`
- `parse_jsx_runtime`: ค่า config `"jsx"` → `JsxRuntime::Automatic` (ค่าเริ่มต้น) หรือ `Classic`

### โครงสร้าง `ProjectConfig`

```rust
#[derive(Debug, Clone, Default, Deserialize)]      // #[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    pub app_dir: Option<String>,                    // default "app"
    pub out_dir: Option<String>,                    // default ".ruvyxa"
    pub runtime: Option<BuildTarget>,
    #[serde(rename = "react")]
    pub _react: Option<serde_json::Value>,          // reserved, unused
    #[serde(rename = "typescript")]
    pub _typescript: Option<serde_json::Value>,     // reserved, unused

    #[serde(default, rename = "render")]
    pub rendering: RenderingConfigOptions,
    #[serde(default)]
    pub server: ServerConfigOptions,
    #[serde(default)]
    pub css: CssConfigOptions,
    #[serde(default)]
    pub build: BuildConfigOptions,
    #[serde(default)]
    pub debug: DebugConfigOptions,
    #[serde(default, rename = "image")]
    pub images: ImageOptimizationOptions,
    #[serde(default)]
    pub security: SecurityConfigOptions,
    #[serde(default)]
    pub cache: CacheConfigOptions,
    #[serde(default)]
    pub middleware: MiddlewareConfig,
    #[serde(default)]
    pub plugins: Vec<BuildPluginConfig>,
    #[serde(rename = "adapter")]
    pub adapter: Option<serde_json::Value>,
    #[serde(rename = "adapterOptions")]
    pub adapter_options: Option<serde_json::Value>,

    #[serde(skip)]
    pub config_dependency_hash: String,
    #[serde(skip)]
    pub javascript_runtime_override: Option<JavaScriptRuntime>,
}

// Sub-config structs:
pub struct RenderingConfigOptions {
    #[serde(rename = "strategy")]
    pub default_strategy: Option<RenderStrategy>,
    #[serde(rename = "revalidate")]
    pub default_revalidate: Option<u64>,
}

pub struct ServerConfigOptions {
    pub host: Option<String>,
    pub port: Option<u16>,
}

pub struct CssConfigOptions {
    #[serde(default)]
    pub entries: Vec<String>,
}

pub struct BuildConfigOptions {
    pub minify: Option<bool>,
    #[serde(rename = "map")]
    pub sourcemap: Option<bool>,
    #[serde(rename = "treeShake")]
    pub tree_shaking: Option<bool>,
    #[serde(rename = "split")]
    pub split_strategy: Option<String>,
    #[serde(rename = "workers")]
    pub parallelism: Option<usize>,
    #[serde(rename = "jsx")]
    pub jsx_runtime: Option<String>,
    #[serde(rename = "target")]
    pub es_target: Option<String>,
    #[serde(rename = "manifest")]
    pub emit_chunk_manifest: Option<bool>,
    #[serde(rename = "warm")]
    pub prebundle_dependencies: Option<bool>,
    #[serde(rename = "prerenderCache")]
    pub prerender_cache: Option<bool>,
}

pub struct DebugConfigOptions {
    pub overlay: Option<bool>,
    pub traces: Option<bool>,
}

pub struct ImageOptimizationOptions {
    pub optimize: Option<bool>,    // default true
    pub quality: Option<u8>,       // default 82
    pub lossless: Option<bool>,    // default false
    pub workers: Option<usize>,    // default 0 = rayon global
}

pub struct SecurityConfigOptions {
    #[serde(rename = "actionLimit")]
    pub action_body_limit_bytes: Option<usize>,
    #[serde(rename = "apiLimit")]
    pub api_body_limit_bytes: Option<usize>,
    #[serde(rename = "pluginLimit")]
    pub plugin_response_body_limit_bytes: Option<usize>,
    #[serde(rename = "actionRateLimit")]
    pub action_rate_limit: Option<ActionRateLimitOptions>,
    #[serde(rename = "sameOrigin")]
    pub same_origin_actions: Option<bool>,
    #[serde(rename = "fetchMeta")]
    pub fetch_metadata_actions: Option<bool>,
    #[serde(default, rename = "trustedProxyIps")]
    pub trusted_proxy_ips: Vec<String>,
    #[serde(rename = "headers")]
    pub security_headers: Option<bool>,
}

pub struct ActionRateLimitOptions {
    pub max: Option<usize>,
    pub window: Option<u64>,
}

pub struct CacheConfigOptions {
    #[serde(rename = "routes")]
    pub route_manifest: Option<bool>,
    pub css: Option<bool>,
    #[serde(rename = "dir")]
    pub build_dir: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPluginConfig {
    pub name: String,
}
```

---

## Build Pipeline (`build_with_output`)

```rust
fn build_with_output(args: BuildArgs, produce_output: bool) -> Result<()>
```

### เฟส 1: โหลดค่าตั้งค่า

```rust
let config = load_project_config(&args.root)?;
```

### เฟส 2: ค้นพบเส้นทาง

```rust
let discover_opts = DiscoverOptions::new(app_dir)
    .with_rendering_defaults(
        config.rendering.default_strategy,
        config.rendering.default_revalidate,
    );
let manifest = discover_routes(discover_opts)?;
```

### เฟส 3: การตรวจสอบ

```rust
let report = validate_app(&args.root, &manifest)?;
if !report.is_ok() {
    // Print diagnostics, bail
}
```

### เฟส 4: รวบรวมสไตล์

```rust
let styles = collect_styles(&args.root, &app_dir, &config.style_entries)?;
```

### เฟส 5: ไดเรกทอรี staging

```rust
let out_dir = args.root.join(config.out_dir());
let staging = out_dir.join(".ruvyxa-staging-<random>");
// Copy directories:
copy_dir(app_dir, staging.join("server/app"))?;
copy_dir_if_exists(components_dir, staging.join("server/components"))?;
copy_dir_if_exists(server_dir, staging.join("server/server"))?;
// Copy style files to staging
```

### เฟส 6: การปรับแต่งภาพให้เหมาะสม

```rust
if produce_output {
    optimize_public_images(
        &args.root.join("public"),
        &staging.join("assets"),
        &out_dir.join("cache/images"),
        &config.images,
    )?;
}
```

### เฟส 7: เขียน manifest

```rust
write_manifest(&manifest, staging.join("manifest.json"))?;
```

### เฟส 8: สร้าง client bundles

```rust
if produce_output {
    let client_manifest = emit_client_bundles(
        &manifest, &config, &args.root, &app_dir, &staging,
    )?;
    write_json(staging.join("client/manifest.json"), &client_manifest)?;
}
```

**รายละเอียด `emit_client_bundles_with_runtime`**:

```rust
fn emit_client_bundles_with_runtime(
    root: &Path, app_dir: &Path,
    manifest: &RouteManifest, client_dir: &Path,
    build: &BuildConfigOptions, plugins: &[BuildPluginConfig],
    cache: RuvyxaBuildCache<'_>, runtime: JavaScriptRuntime,
) -> Result<serde_json::Value>
```

1. กรองเฉพาะ page routes
2. กำหนด parallelism: `config.build.parallelism.unwrap_or_else(num_cpus::get)`
3. สร้าง `BundleContext` (พร้อม caches) ถ้ามี plugins ให้สร้าง ordered plugin hook host
4. กลยุทธ์การแบ่ง:
   - **Route** (ค่าเริ่มต้น): เตรียมทุก routes พร้อมกัน → ตรวจจับ shared modules ที่ใช้ใน >=2 routes
     → สร้าง `shared.js` → สร้าง per-route bundles ที่ import shared registry
   - **Single**: สร้างแต่ละ route แยกอิสระ
5. เขียน output JS + source maps + chunk manifest
6. แสดงสถิติ build (จำนวน module, ขนาด, cache hits)

### เฟส 9: Pre-render สำหรับ static routes

```rust
if produce_output {
    let prerender_result = prerender_static_routes(
        &manifest, config, &staging, &args.root,
    )?;
    write_json(staging.join("prerender/manifest.json"), &prerender_result)?;
}
```

**รายละเอียด `prerender_static_routes`**:

กรอง routes แบบ SSG/ISR/PPR/CSR สำหรับแต่ละ route:

| กลยุทธ์                | การทำงาน                                                              |
| ---------------------- | --------------------------------------------------------------------- |
| CSR                    | สร้าง HTML shell ขั้นต่ำ                                              |
| SSG with params        | `resolve_static_params()` ผ่าน worker pool → render แต่ละ param combo |
| SSG static             | Render เดียว                                                          |
| ISR                    | Render เดียว (dev) หรือข้าม (prod — ตอน request)                      |
| PPR with params        | `resolve_static_params()` → `render_ssg(mode="ppr")`                  |
| PPR static             | `render_ssg(mode="ppr")`                                              |
| Dynamic without params | ข้าม (ตอน request เท่านั้น)                                           |

งาน prerender ถูกกระจายด้วย parallelism สูงสุด 2 แต่ละงานเขียน HTML ไปยัง
`prerender/<path>/index.html`

**แคช prerender artifact**:

```rust
struct PrerenderArtifactCache {
    directory: PathBuf,                    // out_dir/cache/prerender
    dependency_hash: String,
    render_context_hash: String,
    fingerprints: Arc<ArtifactFingerprintCache>,
    enabled: bool,
}
```

การตรวจสอบแคช: version==1, dependency_hash ตรงกัน, render_context_hash ตรงกัน, file fingerprints
ทั้งหมดตรงกัน ถ้าตรง → hardlink HTML ที่แคชไว้ไปยัง output ถ้าไม่ตรง → render + เขียนไปยังแคช

### เฟส 10: ข้อมูลเมตา build

```rust
let build_info = BuildInfo {
    version: env!("CARGO_PKG_VERSION"),
    timestamp: chrono::Utc::now(),
    routes: manifest.routes.len(),
    page_routes: report.page_routes,
    api_routes: report.api_routes,
    target: args.target,
    config_hash: config.config_dependency_hash.clone(),
    image_report: image_report.summary(),
    client_bundle_manifest: ...,
    prerender_manifest: ...,
};
write_json(staging.join("build.json"), &build_info);
```

### เฟส 11: การ commit แบบอะตอมมิก

```rust
commit_staged_build_outputs(&staging, &out_dir)?;
// Replaces/creates out_dir atomically:
// 1. If out_dir exists: rename to out_dir + ".old"
// 2. Rename staging → out_dir
// 3. Remove old (failure → restore from old)
```

### เฟส 12: แสดงรายงาน

```rust
print_build_report(&build_info, &styles, elapsed);
// Route table, sizes, timing summary, image report
```

---

## Image Optimizer (`image_optimizer.rs`)

### `ImageOptimizationOptions`

```rust
pub struct ImageOptimizationOptions {
    pub optimize: bool,        // default true
    pub quality: u8,           // default 82
    pub lossless: bool,        // default false
    pub parallelism: usize,    // default 0 = rayon global default
}
```

### `optimize_public_images(public_dir, assets_dir, cache_dir, options) → ImageReport`

```rust
pub struct ImageReport {
    pub input_files: usize,
    pub output_files: usize,
    pub optimized: usize,
    pub copied: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub duration_ms: u64,
}
```

อัลกอริทึม:

1. **ค้นหา**: เดิน `public_dir` แบบเรียกซ้ำ รวบรวมไฟล์ทั้งหมด
2. **ตรวจสอบการชนกัน**: ถ้ามีทั้ง `name.png` และ `name.jpg` → ข้อผิดพลาด (ชื่อ output stem เดียวกัน)
3. **ประมวลผลแต่ละไฟล์**:

```rust
for entry in entries {
    let ext = entry.extension().to_lowercase();

    // Non-optimizable formats → copy as-is
    if !matches!(ext, "png" | "jpg" | "jpeg") || !options.optimize {
        fs::copy(entry, assets_dir.join(entry.file_name()))?;
        continue;
    }

    // Decode
    let img = match image::open(&entry) {
        Ok(img) => img,
        Err(_) => {
            // Decode failed → copy as-is
            fs::copy(entry, assets_dir.join(entry.file_name()))?;
            continue;
        }
    };

    // Cache key
    let cache_key = blake3::hash(format!(
        "{}:{}\n{}",
        options.quality,
        options.lossless as u8,
        blake3::hash(&fs::read(&entry)?)
    )).to_hex();

    let cache_file = cache_dir.join(&cache_key).with_extension("webp");

    // Cache hit → hardlink
    if cache_file.exists() {
        fs::hard_link(&cache_file, output_file)?;
        continue;
    }

    // Encode WebP
    let encoder = webp::Encoder::from_image(&img)?;
    let webp = if options.lossless {
        encoder.encode_lossless()
    } else {
        encoder.encode(options.quality as f32)
    };

    // Write to cache + hardlink to output
    fs::write(&cache_file, &*webp)?;
    fs::hard_link(&cache_file, output_file)?;
}
```

4. **การทำงานแบบขนาน**: `rayon::ThreadPoolBuilder::new().num_threads(options.workers).build()` ถ้า
   มีการระบุ workers, มิฉะนั้นใช้ global pool เริ่มต้น

5. **เขียน manifest**: `.ruvyxa-images.json` พร้อม
   `{ files: [{ input, output, width, height, format, optimized }] }`

---

## สะพานเชื่อม Plugin Build Hook

เชื่อมระบบ plugin ของ Rust bundler ไปยัง JS plugins ที่กำหนดค่าใน `ruvyxa.config.ts` trait
`BuildHooks` กำหนดขอบเขตภายใน:

```rust
pub struct BuildHookContext {
    pub project_root: PathBuf,
    pub importer: Option<PathBuf>,
    pub target: BundleTarget,
}

pub struct TransformOutput {
    pub code: String,
    pub map: Option<String>,
}

pub trait BuildHooks: Send + Sync {
    fn host_name(&self) -> &str;

    fn resolve_id(
        &self, specifier: &str, importer: Option<&Path>,
        context: &BuildHookContext,
    ) -> Result<Option<PathBuf>>;  // default: Ok(None)

    fn transform(
        &self, code: &str, id: &Path,
        context: &BuildHookContext,
    ) -> Result<Option<TransformOutput>>;  // default: Ok(None)
}
```

**`BuildHookPipeline`**: pipeline ของ hosts แบบเรียงลำดับ ดำเนินการ hooks ตามลำดับการลงทะเบียน

```rust
pub struct BuildHookPipeline {
    hosts: Arc<Vec<Arc<dyn BuildHooks>>>,
}
```

- `resolve_id`: วนซ้ำ hosts, ตัวแรกที่คืนค่า `Some(path)` เป็นผู้ชนะ
- `transform_with_map`: เชื่อม transforms — แต่ละ host ได้รับ output ก่อนหน้า; source map
  ตัวสุดท้ายที่ไม่ใช่ None จะถูกเก็บไว้

Node/Bun runtime แบบถาวรหนึ่งตัวเป็นเจ้าของ registry การตั้งค่า ดังนั้น closures และ module-level
plugin state จะถูกแชร์ข้ามการเรียก build ต่างๆ `build.onComplete` ทำงานหลังจาก output ที่ commit
แล้ว

---

## การแมป Dev/Production Server Config

### `dev_server_config(args, config) → ServerConfig`

```rust
ServerConfig {
    root: args.root,
    app_dir: root / config.app_dir(),
    public_dir: root / "public",
    client_dir: out_dir / "client",
    prerender_dir: Some(out_dir / "prerender"),
    host: args.host.unwrap_or(config.server.host.unwrap_or(DEFAULT_HOST)),
    port: args.port.unwrap_or(config.server.port.unwrap_or(DEFAULT_PORT)),
    watch: true,
    error_overlay: config.debug.overlay.unwrap_or(true),
    debug_traces: config.debug.traces.unwrap_or(false),
    action_body_limit_bytes: config.security.action_body_limit_bytes.unwrap_or(DEFAULT),
    action_rate_limit_max: action_rate.max,
    action_rate_limit_window: Duration::from_secs(action_rate.window),
    // ... map all other fields from config
}
```

### `production_server_config(args, config) → ServerConfig`

โครงสร้างเดียวกันแต่:

- `app_dir = out_dir / "server" / config.app_dir()` (output ที่ compile แล้ว)
- `public_dir = out_dir / "assets"` (ภาพที่ปรับแต่งแล้ว)
- `watch = false`
- `error_overlay = false`
