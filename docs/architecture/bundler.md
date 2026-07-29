# Bundler · การรวมโค้ด

**Crate**: `ruvyxa_bundler`  
**Modules**: `crates/ruvyxa_bundler/src/{lib,types,resolver,compiler,boundary,linker}.rs`

## สรุป

`ruvyxa_bundler` รับ RouteManifest + source files → สร้างชุด bundles ที่พร้อม deploy (IIFE สำหรับ
client, ESM/CJS สำหรับ server) ใช้ Oxc เป็น parser/minifier (เร็วกว่า SWC ~3x), path resolution
แบบกำหนดเอง, circular dep detection

---

## Core Types

### BundleTarget

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BundleTarget {
    Client,
    Server,
}
```

### BundleProfile

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleProfile {
    pub target: BundleTarget,
    pub es_target: EsTarget,
    pub jsx_runtime: JsxRuntime,
    pub split_strategy: SplitStrategy,
    pub minify: bool,
    pub source_maps: bool,
    pub env: HashMap<String, String>,
}
```

### EsTarget

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EsTarget {
    Es2015,
    Es2016,
    Es2017,
    Es2018,
    Es2019,
    Es2020,
    Es2021,
    Es2022,
    #[default]
    EsNext,
}
```

### JsxRuntime

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum JsxRuntime {
    #[default]
    Automatic,
    Classic,
}
```

### SplitStrategy

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SplitStrategy {
    #[default]
    Single,
    Vendor,
    Route,
    Granular,
}
```

### BundleOptions

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleOptions {
    pub profiles: Vec<BundleProfile>,
    pub entries: Vec<BundleInput>,
    pub base: PathBuf,
    pub out_dir: PathBuf,
    pub route_manifest: Option<RouteManifest>,
    pub node_modules_externals: bool,
}
```

### BundleInput

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleInput {
    pub name: String,
    pub path: PathBuf,
    pub target: BundleTarget,
}
```

---

## Pipeline

```
BundleOptions
  ↓
resolve_entries()         — normalize paths, check file existence
  ↓
build_resolver()          — create OxcResolver with custom extensions
  ↓
create_compiler()         — init OxcCompiler with options
  ↓
compile_modules()         — parse → transform (JSX, TS, env) → collect
  ↓
check_boundary()          — validate server/client import graph boundaries
  ↓
link_modules()            — IIFE wrap for client, CJS/ESM for server
  ↓
minify()                  — Oxc minifier
  ↓
emit()                    — write to out_dir
```

---

## Resolution

### Extensions

```rust
const RUVYXA_EXTENSIONS: &[&str] = &[
    ".tsx", ".ts", ".jsx", ".js", ".mjs", ".cjs",
    ".json", ".node",
];
```

### Resolver

`build_resolver()` wraps `OxcResolver::new()` with:

- Custom extensions (`.tsx`, `.ts`, `.jsx`, `.js`, `.mjs`, `.cjs`)
- `node_modules` lookup from project root AND framework package
- Remaining extensions from `oxc_resolver` default set (`.json`, `.node`)
- `tsconfig.json` path alias resolution
- Built-in module map for node builtins (`fs`, `path`, `os` → empty stub on client)

```rust
fn resolve_module(specifier: &str, dir: &Path) -> Result<PathBuf>

fn resolve_module_with_extensions(
    specifier: &str,
    base_dir: &Path,
    extensions: &[&str],
) -> Option<PathBuf>
```

Resolution order:

1. Absolute path / relative path → check with extension probing
2. Bare specifier (`react`, `lodash`) → `node_modules` lookup
3. Deep import (`react/jsx-runtime`) → `node_modules/package/` resolution
4. Built-in → stub map

---

## Compilation

### Oxc Pipeline

```rust
pub struct RuvyxaCompiler {
    inner: OxcCompiler,
    options: BundleOptions,
}

fn compile_file(&self, input: &BundleInput) -> Result<CompiledModule> {
    // 1. OxcParser: source → AST
    // 2. OxcTransformer: JSX → React.createElement (classic) / jsx() (automatic)
    // 3. TypeScript stripping (no type checking)
    // 4. OxcCodegen: AST → output code
}
```

### CompiledModule

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledModule {
    pub name: String,
    pub code: String,
    pub map: Option<String>,
    pub deps: Vec<String>,
    pub exports: Vec<String>,
}
```

The `name` field matches `BundleInput.name`. Dependencies are resolved to absolute paths during
compilation and tracked for circular detection.

---

## Boundary Checking

### `check_boundary()`

```rust
pub fn check_boundary(
    module: &CompiledModule,
    graph: &[CompiledModule],
    base: &Path,
) -> Result<Vec<Diagnostic>>
```

### Rules enforced

| Rule                  | Check                                                         | Diagnostic |
| --------------------- | ------------------------------------------------------------- | ---------- |
| Server-only imports   | Module graph from client entry contains `server-only`         | RUV1007    |
| Private `process.env` | Module graph accesses `process.env.*` not in `RUVYXA_PUBLIC_` | RUV1008    |
| Client-only in server | Module graph from server entry contains `client-only`         | RUV1009    |
| Server dir in client  | Import chain reaches `server/` directory from client entry    | RUV1010    |

The check traverses the compiled module graph (BFS) and flags any prohibited import.
`NodeModulesExternal` option can whitelist known packages.

---

## Linking Strategy

### Client: IIFE

```rust
fn produce_iife(module: &CompiledModule) -> String {
    let deps = module.deps.iter().map(|d| format!("\"{}\"", d)).collect::<Vec<_>>().join(", ");
    let factory = &module.code;
    format!(
        "(function({{ {} }}) {{ {} }})",
        deps, factory
    )
}
```

All client bundles are wrapped in IIFEs and concatenated with a lightweight module registry
(`__ruvyxa_modules`):

### Server: CJS

```rust
fn produce_server_module(module: &CompiledModule) -> String {
    let requires = module.deps.iter()
        .map(|d| format!("var _{} = require('{}');", d.replace('/', '_'), d))
        .collect::<Vec<_>>()
        .join("\n");
    format!("'use strict';\n{}\n{}", requires, module.code)
}
```

Server modules use CommonJS (`require`/`module.exports`) for Node.js compatibility. ESM output is
optional via `BundleProfile.es_target`.

---

## Circular Dependency Detection

```rust
pub fn detect_cycles(modules: &[CompiledModule]) -> Result&lt;()&gt;
```

Tarjan's algorithm on the compiled module list. Each detected cycle produces a `Diagnostic` but does
not halt the build — the linker handles circular references by emitting `undefined` for
not-yet-initialized module refs.

---

## Minification

Oxc minifier pipeline:

```rust
fn minify(source: &str, target: EsTarget) -> String {
    // OxcMinifier runs:
    //   1. Dead code elimination
    //   2. Constant folding
    //   3. Identifier shortening
    //   4. Whitespace removal
}
```

Source maps are generated alongside minified output if `BundleProfile.source_maps` is `true`.

---

## Emit

```rust
pub fn emit(options: &BundleOptions, modules: &[CompiledModule]) -> Result<()> {
    // Structure:
    //   out_dir/
    //     client/
    //       <entry-name>.js
    //       <entry-name>.js.map
    //     server/
    //       <entry-name>.js
    //       <entry-name>.js.map
    //     chunks/
    //       <chunk-hash>.js
}
```

---

## Why This Design

1. **Oxc over SWC** — 3× faster parsing, lower memory footprint, Rust-native. No NAPI bridge
   overhead.
2. **IIFE for client** — No ESM module system dependency in the browser. Works in all environments
   including workers, edge, and sandboxes.
3. **Custom resolver, not webpack** — No need for webpack's plugin system complexity. `ruvyxa` has 3
   entry points per route (page, client, server module); resolution is simple enough for a dedicated
   resolver.
4. **Boundary check at bundle time** — Prevents server-only code from leaking into browser bundles
   before they reach the user. Catches the error during `build`, not at runtime.
5. **Tarjan for cycles** — Cycle detection is a developer DX feature, not a correctness requirement.
   The linker handles cycles gracefully; the detection simply warns.
