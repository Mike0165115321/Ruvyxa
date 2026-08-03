# Dev Server (`ruvyxa_dev_server`)

**ไฟล์**: `crates/ruvyxa_dev_server/src/` (6 ไฟล์, ~8,000 บรรทัด)

เซิร์ฟเวอร์ HTTP ที่ขับเคลื่อนด้วย Axum + Tokio จัดการคำขอ HTTP, WebSocket HMR, การจับคู่เส้นทางผ่าน
Radix trie, แคชการเรนเดอร์, การจัดการ Node worker pool, และการรวบรวมสไตล์

---

## ประเภทคอนฟิก (Configuration Types)

### `ServerConfig`

```rust
pub struct ServerConfig {
    pub root: PathBuf,                          // Project root
    pub app_dir: PathBuf,                       // root/app (dev) or out_dir/server/app (prod)
    pub public_dir: PathBuf,                    // root/public (dev) or out_dir/assets (prod)
    pub client_dir: PathBuf,                    // root/.ruvyxa/client
    pub prerender_dir: PathBuf,                 // root/.ruvyxa/prerender
    pub host: String,                           // default "0.0.0.0"
    pub port: u16,                              // default 3000
    pub watch: bool,                            // true = dev mode
    pub cache_route_manifest: bool,
    pub cache_css: bool,
    pub style_entries: Vec<PathBuf>,            // additional CSS entry points
    pub prebundle_dependencies: bool,
    pub runtime: JavaScriptRuntime,             // Node or Bun
    pub jsx_runtime: JsxRuntime,
    pub error_overlay: bool,                    // show diagnostic overlay in browser
    pub debug_traces: bool,
    pub action_body_limit_bytes: usize,
    pub api_body_limit_bytes: usize,
    pub plugin_response_body_limit_bytes: usize,
    pub action_rate_limit_max: usize,
    pub action_rate_limit_window: Duration,
    pub same_origin_actions: bool,
    pub fetch_metadata_actions: bool,
    pub trusted_proxies: TrustedProxies,        // ที่อยู่ตรงตัว + ช่วง CIDR จับคู่จาก config
    pub security_headers: bool,
    pub middleware: MiddlewareConfig,
    pub plugins_enabled: bool,
    pub plugin_head: Vec<PluginHeadEntry>,      // head tags ที่แทรกโดย TypeScript plugins
    pub default_render_strategy: Option<RenderStrategy>,
    pub default_revalidate: Option<u64>,
}
```

`TrustedProxies` จับคู่ peer กับที่อยู่ตรงตัวและช่วง CIDR โดย unmap IPv4-mapped IPv6 peer ก่อน
เพื่อให้ช่วง IPv4 จับคู่กับรูปแบบ `::ffff:a.b.c.d` ของ listener แบบ dual-stack ได้ Loopback
ถูกเชื่อถือโดยอัตโนมัติไม่ขึ้นกับรายการที่ตั้งค่าไว้

### `AppState`

```rust
struct AppState {
    config: ServerConfig,
    reload_tx: broadcast::Sender<String>,              // HMR WebSocket fan-out
    runtime_cache: Arc<RuntimeCache>,                   // manifest, router, CSS
    action_limiter: Arc<Mutex<ActionRateLimiter>>,
    worker_pool: Arc<NodeWorkerPool>,
    render_cache: Arc<RenderCache>,
    isr_revalidating: Arc<tokio::sync::Mutex<HashSet<String>>>,
    hmr_tracker: Arc<HmrTracker>,
    plugin_runtime: Option<Arc<PluginHost>>,
}

struct RuntimeCache {
    manifest: tokio::sync::RwLock<Option<RouteManifest>>,
    styles: tokio::sync::RwLock<Option<StyleCacheEntry>>,
    router: tokio::sync::RwLock<Option<RadixRouter>>,
}

struct StyleCacheEntry {
    css: String,
    files: BTreeSet<PathBuf>,   // normalized, case-folded on Windows
}
```

---

## `serve(config) → Result<()>` — ลำดับการเริ่มต้นเต็ม

### 1. ตรวจสอบขีดจำกัด

```rust
config.validate_limits()  // body limits > 0, rate limits > 0
```

### 2. ค้นพบเส้นทาง

```rust
let manifest = discover_routes(discover_options(&config))?;
```

### 3. ช่องสัญญาณ Broadcast

```rust
let (reload_tx, _) = broadcast::channel::<String>(64);  // capacity 64, drops oldest
```

### 4. สภาพแวดล้อมรันไทม์

```rust
let env = runtime_env(&config);
// Loads .env + .env.local from project root
// Inserts RUVYXA_JSX_RUNTIME=automatic|classic
```

### 5. Node worker pool

```rust
let worker_pool = NodeWorkerPool::start(&config.root, env).await?;
```

### 6. อุ่นเครื่อง dependencies (dev เท่านั้น)

```rust
if config.watch && config.prebundle_dependencies {
    let warmup_pool = worker_pool.clone();
    let warmup_routes = dependency_warmup_routes(&config, &manifest);
    tokio::spawn(async move {
        warmup_pool.warmup(&warmup_root, warmup_routes).await;
    });
}
```

### 7. แคชการเรนเดอร์

```rust
let render_cache = if config.watch {
    RenderCache::default_dev()   // capacity=1024, TTL=300s
} else {
    RenderCache::default_production()  // capacity=512, TTL=1800s
};
```

ความจุสามารถกำหนดค่าได้ผ่าน env var `RUVYXA_RENDER_CACHE_SIZE` (สูงสุด 16384)

### 8. HMR tracker

```rust
let hmr_tracker = Arc::new(HmrTracker::new());
hmr_tracker.populate_from_manifest(&manifest.routes);
```

### 9. Middleware และปลั๊กอิน

```rust
let middleware_stack = MiddlewareStack::new(config.middleware.clone());
middleware_stack.validate()?;

let plugin_runtime = if !config.plugins.is_empty() {
    Some(Arc::new(PluginHost::start(&config.root, runtime_script, runtime_executable)?))
} else {
    None
};
```

### 10. Axum Router

```rust
let state = Arc::new(AppState { ... });

let router = Router::new()
    .route("/__ruvyxa/hmr", get(hmr_ws))
    .route("/__ruvyxa/client", get(client_bundle))
    .route("/__ruvyxa/action",
        post(action_endpoint)
            .layer(DefaultBodyLimit::max(config.action_body_limit_bytes)))
    .route("/__ruvyxa/trace", get(trace_endpoint))
    .fallback(handle_request)
    .with_state(state.clone());
```

จากนั้นใช้เลเยอร์ middleware stack (compression, CORS, rate limiting, timing, logging, headers,
custom, plugins) + security headers

### 11. ผูก listener

```rust
let mut port = config.port;
let listener = loop {
    match TcpListener::bind((config.host, port)).await {
        Ok(l) => break l,
        Err(_) if port < config.port + 100 => port += 1,
        Err(e) => return Err(e.into()),
    }
};
// Port fallback: try up to config.port + 100
```

### 12. ปิดระบบอย่างราบรื่น

```rust
let (shutdown_tx, shutdown_rx) = watch::channel(false);

axum::serve(listener, router)
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
        // OR watch channel true
    })
    .await?;

// After shutdown:
worker_pool.shutdown().await;  // 5s grace period
```

---

## วงจรชีวิตคำขอ (Request Lifecycle — `handle_request`)

### 1. แยกวิเคราะห์พาธ canonical

```rust
fn canonical_request_path(path: &str) -> Result<String, StatusCode>
```

- แยกด้วย `/`
- Percent-decode แต่ละเซกเมนต์ (`percent_encoding::percent_decode_str`)
- ปฏิเสธ: เซกเมนต์ว่าง, `.`, `..`, `/` หรือ `\` ที่ถูก decode, อักขระควบคุม (0x00-0x1F)
- ปฏิเสธ: percent encoding ที่ไม่ถูกต้อง (hex ไม่ถูกต้อง, ถูกตัดทอน)

### 2. อ่าน body

```rust
if request.method() != Method::GET && request.method() != Method::HEAD {
    let body_bytes = axum::body::to_bytes(body, config.api_body_limit_bytes)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
}
```

### 3. Request middleware

```rust
if let Some(host) = &state.plugin_runtime {
    match host.execute_request(plugin_request).await? {
        MiddlewareRequestResult::Response(response) => return response.into_response(),
        MiddlewareRequestResult::Request(replacement) => apply_request(replacement),
    }
}
```

### 4. การกระจายการเรนเดอร์

```rust
// Try static files first
if let Some(resp) = serve_client_file(&state, &path).await {
    return resp;
}
if let Some(resp) = serve_public_file(&state, &path, &req).await {
    return resp;
}

// Route lookup
let router = state.runtime_cache.router().await;
let Some(route_match) = router.find(&path) else {
    return 404 plain error page;
};

match route_match.route.kind {
    RouteKind::Page => render_page_by_strategy(&state, &route_match, body).await,
    RouteKind::Api  => render_api_pooled(&state, &route_match, method, headers, body).await,
}
```

### 5. การประกอบ HTML (เส้นทาง Page)

```rust
fn compose_document(rendered: &str, head_content: &str, hmr: &str) -> String
```

อัลกอริทึม (การค้นหาแท็กทั้งหมดไม่สนตัวพิมพ์เล็กใหญ่):

1. ถ้าพบ `<html`:
   - ถ้าพบ `<head` → แทรก `head_content` ก่อน `</head>`
   - หรือถ้าพบ `<body` → แทรก `<head>{head_content}</head>` ก่อน `<body>`
   - หรือ → แทรก `<head>{head_content}</head>` หลังแท็กเปิด `<html>`
   - แทรก `hmr` ก่อน `</body>`
2. หรือ → ห่อด้วยโครง HTML เต็ม:
   ```html
   <!doctype html>
   <html lang="en">
     <head>
       <meta charset="utf-8" />
       <meta name="viewport" content="width=device-width,initial-scale=1" />
       {head_content}
     </head>
     <body>
       {rendered}{hmr}
     </body>
   </html>
   ```

**Head content**: `<link rel="icon" href="/ruvyxa.png">` + `<style data-ruvyxa-css>{css}</style>`

**HMR content**: `<script>/* WebSocket HMR */</script>` (dev เท่านั้น)

**Client hydration**: แทรก `__RUVYXA_ROUTE_PARAMS__`, `__RUVYXA_REQUEST_PATH__`, preload hints,
`<script type="module" src="/__ruvyxa/client?path=...">`

### 6. Response middleware

```rust
if let Some(host) = &state.plugin_runtime {
    if let Some(replacement) = host.execute_response(plugin_request, plugin_response).await? {
        return replacement.into_response();
    }
}
```

### 7. Security headers

```rust
fn finalize_security_headers(response: Response) -> Response {
    response.headers_mut().insert("X-Content-Type-Options", "nosniff");
    response.headers_mut().insert("X-Frame-Options", "DENY");
    // ... configured headers
}
```

---

## กลยุทธ์การเรนเดอร์ (Render Strategies)

### SSR (`render_page_ssr`)

```
1. RenderCache::get(ssr_cache_key(path, params))
2. On miss: worker_pool.render_ssr() → compose HTML → RenderCache::put()
3. Return cached/rendered HTML
```

```rust
pub fn ssr_cache_key(request_path: &str, params: &RouteParams) -> String {
    if params.is_empty() {
        format!("ssr:{}", request_path)
    } else {
        format!("ssr:{}?{}", request_path, serde_json::to_string(params).unwrap())
    }
}
```

### SSG (`render_page_ssg`)

```
1. Production: ตรวจสอบ prerender_dir/<path>/index.html
2. Dev: RenderCache::get(ssg_cache_key)
3. On miss: worker_pool.render_ssg(mode="full")
4. แคชไม่มีกำหนด (ไม่มี TTL, เก็บไว้จนกว่าจะถูกทำให้เป็นโมฆะ)
```

### ISR (`render_page_isr`)

```
1. RenderCache::get_stale_with_age(isr_cache_key) → (value, age)
2. ถ้ามีในแคช:
   - ถ้า age >= revalidate_seconds: spawn_isr_revalidation()
   - เสิร์ฟค่าเก่า (แม้จะหมดอายุ)
3. ถ้าไม่มีในแคช:
   - Production prerender: ตรวจสอบ prerender_dir
   - Dev: เรนเดอร์แบบซิงก์
4. คืนค่า HTML
```

**การรวมการตรวจสอบซ้ำ**:

```rust
pub async fn spawn_isr_revalidation(
    state: &AppState, key: String, ...
) {
    let mut in_flight = state.isr_revalidating.lock().await;
    if in_flight.contains(&key) {
        return;  // Already revalidating
    }
    in_flight.insert(key.clone());
    drop(in_flight);

    let state = Arc::clone(state);
    tokio::spawn(async move {
        let html = render_isr_background(&state, ...).await;
        state.render_cache.put(&key, &html).await;
        state.isr_revalidating.lock().await.remove(&key);
    });
}
```

### CSR (`render_page_csr`)

คืนค่า HTML shell ขั้นต่ำ:

```html
<!doctype html>
<html lang="en">
  <head>
    ...{head_content}...
  </head>
  <body>
    <div id="__ruvyxa"></div>
    {hmr}{client_hydration}
  </body>
</html>
```

### PPR (`render_page_ppr`)

```rust
worker_pool.render_ssg(mode="ppr")  // static shell → dynamic slots streamed
```

---

## ตัวจัดการเอนด์พอยต์ (Endpoint Handlers)

เส้นทางที่สงวนไว้สำหรับ framework (ชนกับ route ของแอปจะถูกปฏิเสธ): `/__ruvyxa/hmr`,
`/__ruvyxa/client`, `/__ruvyxa/hydration-loader.js` (สคริปต์ client hydration loader),
`/__ruvyxa/client/route-manifest.json` (ตาราง route แบบสดสำหรับ router ฝั่ง browser),
`/__ruvyxa/action`, `/__ruvyxa/trace`

### `GET /__ruvyxa/hmr` → WebSocket

```rust
async fn hmr_ws(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(|mut socket| async move {
        let mut rx = state.reload_tx.subscribe();
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if socket.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    })
}
```

### `GET /__ruvyxa/client?path=` → JS bundle

```rust
async fn client_bundle(
    State(state): State<Arc<AppState>>,
    Query(ClientBundleQuery { path }): Query<ClientBundleQuery>,
) -> Response {
    match render_client_bundle_pooled(&state, &path).await {
        Ok(js) => (
            StatusCode::OK,
            [("content-type", "text/javascript; charset=utf-8")],
            js,
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", "text/javascript; charset=utf-8")],
            format!("console.error({});", serde_json::to_string(&e.to_string()).unwrap()),
        ).into_response(),
    }
}
```

### `POST /__ruvyxa/action?path=&name=` → Server action

```rust
async fn action_endpoint(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Query(ActionQuery { path, name }): Query<ActionQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // 1. Validate request
    validate_action_request(&state.config, &headers, &body)?;

    // 2. Parse payload
    validate_action_payload(&headers, &body)?;

    // 3. Rate limit
    let key = action_rate_limit_key(peer, &headers, &path, &name);
    if !state.action_limiter.lock().unwrap().allow(&key) {
        let retry = state.action_limiter.lock().unwrap().retry_after_seconds(&key);
        return (StatusCode::TOO_MANY_REQUESTS, [("retry-after", retry.to_string())]).into_response();
    }

    // 4. Execute
    let result = state.worker_pool.render_action(
        &state.config.root, &path, &name,
        payload_json, content_type,
    ).await;

    match result {
        Ok(response) => response.into(),
        Err(e) => error_response(e),
    }
}
```

**`validate_action_request`** ตรวจสอบ:

- ขนาด body ≤ `action_body_limit_bytes`
- Content-Type ถูกต้อง (JSON หรือ form)
- `same_origin_actions` → ตรวจสอบว่า `Origin` header ตรงกับ origin ของเซิร์ฟเวอร์
- `fetch_metadata_actions` → ตรวจสอบว่า `Sec-Fetch-Site` เป็น `same-origin`

### `GET /__ruvyxa/trace?path=`

```rust
async fn trace_endpoint(
    State(state): State<Arc<AppState>>,
    Query(TraceQuery { path }): Query<TraceQuery>,
) -> Response {
    if !state.config.watch || !state.config.debug_traces {
        return StatusCode::NOT_FOUND.into_response();
    }
    let manifest = state.runtime_cache.manifest.read().await;
    let route = manifest.as_ref()
        .and_then(|m| m.routes.iter().find(|r| r.path == path));
    serde_json::to_value(route).unwrap().into_response()
}
```

---

## ซ้อนทับข้อผิดพลาดขณะพัฒนา (Dev Error Overlay)

เมื่อเกิด `Diagnostic` error ในโหมด dev:

```rust
fn dev_diagnostic_overlay(diag: &Diagnostic) -> Response {
    let frame = extract_code_frame(&diag.span);  // 5 lines around error
    render_error_overlay(ErrorOverlayView {
        code: diag.code,
        title: diag.title,
        location: diag.span.as_ref().map(span_to_string),
        detail: &diag.explanation,
        code_frame: frame,
        suggested_fix: &diag.suggested_fix,
        import_chain: &diag.import_chain,
        affected_routes: &diag.affected_routes,
    })
}
```

**`render_error_overlay`** สร้างหน้า HTML เต็มด้วย:

- พื้นหลังสีเข้มพร้อมเบลอ
- การ์ดไดอะล็อก: รหัสข้อผิดพลาด (ป้ายสีแดง), ชื่อ, ตำแหน่ง
- เฟรมซอร์สโค้ด (สไตล์เทอร์มินัลสีเข้ม, บรรทัดข้อผิดพลาดทำเครื่องหมายด้วย `>` และ `← error`)
- คำแนะนำการแก้ไข (กรอบสีเขียวพร้อมหลอดไฟ)
- สายโซ่ import (accordion แบบยุบได้)
- เส้นทางที่ได้รับผลกระทบ (ยุบได้)
- Stack trace (ยุบได้)
- ปุ่มปิด (ซ่อน overlay, แสดงหน้าเว็บด้านล่าง)

ค่าทั้งหมดถูก HTML-escape ผ่าน `escape_html()`

---

## ตัวเฝ้าดูไฟล์ (File Watcher — โหมด dev)

ใช้ `notify::recommended_watcher()` เฝ้าดูรากโปรเจกต์

### ตัวกรองเหตุการณ์

```rust
fn ignored_watch_path(path: &Path) -> bool {
    top_level_ignored(path) ||
    path_contains(path, ".ruvyxa") ||
    path_contains(path, "node_modules")
}
```

### การประมวลผลเหตุการณ์

```rust
fn handle_change(state: &AppState, paths: Vec<PathBuf>) {
    // Filter Access events, filter ignored paths
    let update = hmr_tracker.compute_update(&paths);

    if update.full_reload {
        runtime_cache.invalidate();
        render_cache.invalidate_all_blocking();
    } else {
        runtime_cache.invalidate_styles_for_paths(&paths);
        for route_path in &update.affected_routes {
            render_cache.invalidate_route_blocking(route_path);
        }
    }

    // Notify workers about changed files
    worker_pool.invalidate_from_watcher(&path_strings);

    // Broadcast to browsers
    let payload = serde_json::json!({
        "type": update.event_type.as_str(),
        "paths": paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "affectedRoutes": update.affected_routes,
        "fullReload": update.full_reload,
    });
    let _ = reload_tx.send(payload.to_string());
}
```

Worker invalidation ใช้ `try_send()` (non-blocking, file watcher callback ไม่มี tokio runtime)
ถ้าล้มเหลว → บังคับโหลดใหม่ทั้งหมด

---

## การให้บริการไฟล์สาธารณะ (Public File Serving)

```rust
async fn serve_public_file(state: &AppState, path: &str, req: &Request) -> Option<Response>
```

1. `public_dir.join(path.trim_start_matches('/'))`
2. ตรวจสอบว่าไฟล์มีอยู่, เป็นไฟล์ (ไม่ใช่ไดเรกทอรี)
3. อ่านไฟล์ + คำนวณ blake3 ETag
4. Conditional GET: ถ้า `If-None-Match` ตรงกับ ETag → คืนค่า `304 Not Modified`
5. กำหนด MIME type จากนามสกุล
6. คืนค่าพร้อม `cache-control: public, max-age=31536000, immutable` + ETag

MIME types: `.js`→text/javascript, `.css`→text/css, `.html`→text/html, `.json`→application/json,
`.svg`→image/svg+xml, `.png`/`.jpg`/`.jpeg`/`.webp`/`.ico`→image/*, `.woff2`→font/woff2, ฯลฯ

---

## ตัวจำกัดอัตรา Action (Action Rate Limiter)

```rust
struct ActionRateLimiter {
    /* slot array แบบตายตัวของ sliding-window counters, ACTION_RATE_LIMIT_SLOTS = 8192 */
}
```

คีย์ของแต่ละ rate limit (client IP + action path + action name) จะถูก hash ลงใน slot ตัวนับตายตัว 1
ใน 8192 slot ทำให้หน่วยความจำของ limiter มีขอบเขตแน่นอนและไม่มีการปฏิเสธการเข้าใช้เพราะพื้นที่ไม่พอ
แต่ละ slot เก็บจำนวนของ window ปัจจุบันและก่อนหน้า โดยจำนวนของ window ก่อนหน้าจะถูกถ่วงน้ำหนักตาม
สัดส่วนที่ยังอยู่ใน trailing window การชนกันของ slot ทำให้สองคีย์แชร์งบประมาณเดียวกัน ซึ่งจะจำกัด
client เร็วขึ้นเท่านั้น — ไม่มีทางให้สิทธิ์เกินจริง ตัว hasher ถูก seed แยกต่อโปรเซส
จึงไม่สามารถสร้างคีย์ ให้ชนกับเป้าหมายที่เลือกไว้ได้

รูปแบบคีย์ขึ้นอยู่กับคอนฟิก:

- `key_by: "ip"` → `peer_addr.to_string()`
- `key_by: "header:<name>"` → `headers.get(name).to_str()`

---

## Radix Router

ดูที่ [RadixRouter](#radix-router-internals) สำหรับรายละเอียดการใช้งาน trie: การคอมไพล์จาก
RouteManifest, การจำแนกเซกเมนต์, อัลกอริทึมการค้นหา, การแยก params, ลำดับความสำคัญ static-vs-dynamic

---

## Render Cache

ดูที่ [render_cache.rs](#render-cache-internals) สำหรับรายละเอียดแคช: การทำงาน LRU, การหมดอายุ TTL,
รูปแบบคีย์แคช, ISR stale-while-revalidate, การทำให้เป็นโมฆะแบบ blocking สำหรับ file watcher

---

## HMR Tracker

ดูที่ [hmr_tracker.rs](#hmr-tracker-internals) สำหรับแผนที่สองทิศทาง, อัลกอริทึม compute_update,
การกำหนดประเภทเหตุการณ์ (CssUpdate vs ComponentUpdate vs FullReload)

---

## การรวบรวมสไตล์ (Style Collection)

ดูที่ [style.rs](#style-collection-internals) สำหรับการรวบรวม CSS ที่ขับเคลื่อนด้วยกราฟ import,
การคอมไพล์ Sass, การทำงานร่วมกับ Tailwind, CSS module scoping, การย่อขนาด

---

## Worker Pool

ดูที่ [Worker Pool doc](worker-pool.md) สำหรับรายละเอียดภายในทั้งหมด: การเริ่มต้นพูล, โปรโตคอล
NDJSON, การตอบกลับ API แบบสตรีมมิ่ง, การกู้คืนจากความล้มเหลว, การทำให้แคชบันเดิลเป็นโมฆะ

ข้อจำกัดเพิ่มเติมอีกสองอย่างที่ควรรู้ในชั้นนี้: build worker จะเกษียณตัวเองหลังให้บริการ prerender
แบบ isolated ครบ `RUVYXA_PRERENDER_RECYCLE_AFTER` ครั้ง (ค่าเริ่มต้น 32, `0` ปิดการใช้งาน) เพราะ ESM
registry ของ Node ไม่เคยคืน module URL ทำให้การ import แบบ isolated ต่อ path ระหว่าง build แบบ
static รั่วหน่วยความจำที่กู้คืนได้ด้วยการแทนที่โปรเซสเท่านั้น — dev server ไม่เคยขอ isolated import
จึงไม่ต้องจ่ายต้นทุนนี้ ภายใน worker หนึ่งตัว `RUVYXA_WORKER_MAX_CONCURRENCY` (ค่าเริ่มต้น: จำนวน
CPU จำกัดที่ 2-8) จำกัดจำนวนการเรนเดอร์พร้อมกันเพื่อไม่ให้ burst ของคำขอทำให้ heap หมด
คำขอส่วนเกินจะเข้าคิว ในขณะที่ `invalidate`/`ping`
ข้ามคิวไปเพื่อไม่ให้การทำให้แคชเป็นโมฆะล่าช้าเมื่อ worker กำลังยุ่ง

---

## Realtime Runtime

`RealtimeRuntime { path, heartbeat, tx }` ถูกสร้างจาก realtime descriptor ของ TypeScript plugin host
โดยตรวจสอบตอนเริ่มต้น: path ต้องเป็น absolute ไม่มีอักขระพิเศษของ URL, heartbeat 5-120 วินาที,
capacity 16-4096 และไม่ชนกับ route ที่ framework สงวนไว้

ตัวจัดการ realtime WebSocket ตรวจสอบ Origin แบบเดียวกับ HMR, แยกวิเคราะห์ query
`?channels=comma,separated` (1-16 channels, ไม่เกิน 128 bytes ต่อชื่อ, ตัวอักษร/ตัวเลข บวก
`:. _/-`), กรอง event ที่ broadcast ตามการสมัคร channel, ส่ง heartbeat ping ตามช่วงเวลาที่ตั้งค่า
และส่ง `{"version":1,"type":"resync","reason":"lagged"}` เมื่อ subscriber ตามหลัง broadcast channel
