# Worker Pool (`NodeWorkerPool`)

**ไฟล์**: `crates/ruvyxa_dev_server/src/worker_pool.rs`

โพรเซส worker Node.js หรือ Bun แบบถาวรที่สื่อสารผ่าน newline-delimited JSON (NDJSON) ทาง
stdin/stdout กำจัดการสร้างโพรเซส JavaScript ต่อคำขอ (~100-500ms) public Rust type ยังคงใช้ชื่อ
`NodeWorkerPool` เพื่อความเข้ากันได้ย้อนหลัง

---

## สถาปัตยกรรม (Architecture)

```
                    ┌──────────────────────┐
                    │    NodeWorkerPool     │
                    │  - workers: Vec<Arc<Worker>>    │
                    │  - next_worker: AtomicU64       │
                    └──────┬───────────────┘
                           │ least in-flight load
                           │ rotating tie-break
         ┌──────────────────┼──────────────────┐
         ▼                  ▼                  ▼
   ┌──────────┐      ┌──────────┐      ┌──────────┐
   │ Worker 0 │      │ Worker 1 │      │ Worker 2 │
   │ Node/Bun │      │ Node/Bun │      │ Node/Bun │
   │ subproc  │      │ subproc  │      │ subproc  │
   └──────────┘      └──────────┘      └──────────┘
        │                  │                  │
   stdin/stdout       stdin/stdout       stdin/stdout
   NDJSON lines       NDJSON lines       NDJSON lines
```

## ค่าคงที่ (Constants)

```rust
const DEFAULT_POOL_SIZE: usize = 4;                       // min 2, max 8
const DEFAULT_WORKER_TIMEOUT_MS: u64 = 30_000;           // interactive requests
const BUILD_WORKER_TIMEOUT_MS: u64 = 300_000;             // prerendering
const MAX_NODE_TIMEOUT_MS: u64 = 2_147_483_647;           // i32::MAX
const WORKER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_PENDING_RESPONSE_FRAMES: usize = 16;             // streaming backpressure
```

ขนาดพูล: ความขนานเริ่มต้นถูกจำกัดไว้ที่ 2-8 โดยตั้งค่าทับได้ด้วย `RUVYXA_WORKER_POOL_SIZE` ค่า
`build.workers` ที่ระบุชัดเจนถูกจำกัดไว้ที่ 1-8
ทำให้บิลด์อายุสั้นที่มีงานเรนเดอร์เดียวไม่ต้องมีโพรเซสว่าง

---

## โครงสร้างข้อมูล (Data Structures)

### `NodeWorkerPool`

```rust
pub struct NodeWorkerPool {
    workers: StdRwLock<Vec<Arc<Worker>>>,
    worker_script: PathBuf,              // packages/ruvyxa/runtime/worker-pool.mjs
    env: BTreeMap<String, String>,
    runtime: JavaScriptRuntime,           // node when available, otherwise bun if unspecified
    next_worker: AtomicU64,               // rotating tie-break cursor
    response_timeout: Duration,           // configurable via RUVYXA_WORKER_TIMEOUT_MS
}
```

### `Worker`

```rust
struct Worker {
    stdin_tx: StdMutex<Option<mpsc::Sender<String>>>,  // None = shutting down
    pending: PendingResponses,           // Arc<PendingResponseSet>
    child: Mutex<Option<Child>>,         // std::process::Child
    alive: Arc<AtomicBool>,
}

struct PendingResponseSet {
    entries: Mutex<BTreeMap<String, PendingResponse>>,
    count: AtomicUsize,                  // lock-free worker load
}

type PendingResponses = Arc<PendingResponseSet>;
```

### `PendingResponse`

```rust
struct PendingResponse {
    sender: mpsc::Sender<WorkerResponse>,   // bounded(16)
    streaming: Arc<AtomicBool>,             // true after api-start frame
}
```

### `WorkerBodyStream`

implement `Stream<Item = Result<Bytes, io::Error>>`:

```rust
struct WorkerBodyStream {
    receiver: mpsc::Receiver<WorkerResponse>,
    idle_deadline: Option<Instant>,    // resets on each frame
    finished: bool,
}
```

การจัดการเฟรม:

- `"api-start"` → เริ่มสตรีมมิ่ง, คืนค่าว่าง
- `"api-chunk"` → base64-decodes `body_base64` → `Bytes`
- `"api-end"` → `None` (สตรีมสิ้นสุด)
- `"api-error"` → `io::Error::new(kind, message)`
- EOF ก่อนกำหนด → `io::ErrorKind::UnexpectedEof`
- หมดเวลาไม่มีการเคลื่อนไหว → `io::ErrorKind::TimedOut`

---

## การเริ่มต้นพูล (Pool Initialization)

### `NodeWorkerPool::start(root, env) → Self`

```rust
pub async fn start(root: &Path, env: BTreeMap<String, String>) -> Result<Arc<Self>> {
    let worker_script = find_worker_script(root)?;
    let pool_size = detect_pool_size();
    let pool = Arc::new(NodeWorkerPool {
        workers: StdRwLock::new(Vec::with_capacity(pool_size)),
        worker_script,
        env,
        next_worker: AtomicU64::new(0),
        response_timeout: Duration::from_millis(DEFAULT_WORKER_TIMEOUT_MS),
    });

    for _ in 0..pool_size {
        let worker = Worker::spawn(&pool.worker_script, &pool.env).await?;
        pool.workers.write().unwrap().push(Arc::new(worker));
    }

    Ok(pool)
}
```

### `find_worker_script(root) → Option<PathBuf>`

ลำดับการค้นหา:

1. เดินไดเรกทอรีขึ้นไปจากไดเรกทอรีปัจจุบันหา `packages/ruvyxa/runtime/<script>` (monorepo)
2. `{root}/node_modules/ruvyxa/runtime/<script>` (แพ็กเกจที่ติดตั้ง)

คืนค่ารายการแรกที่มีอยู่

### `Worker::spawn(script, env) → Result<Self>`

```rust
async fn spawn(worker_script: &Path, env: &BTreeMap<String, String>) -> Result<Self>
```

1. **สร้างโพรเซส Node หรือ Bun ที่เลือก**:

   ```rust
   let mut cmd = Command::new(runtime.executable());
   cmd.arg(worker_script);
   cmd.stdin(Stdio::piped());
   cmd.stdout(Stdio::piped());
   cmd.stderr(Stdio::piped());
   cmd.envs(env);
   cmd.kill_on_drop(true);

   let mut child = cmd.spawn()?;
   ```

2. **งานเขียน stdin (async)**:

   ```rust
   let stdin = child.stdin.take().unwrap();
   tokio::spawn(async move {
       let mut stdin = BufWriter::new(stdin);
       while let Some(line) = rx.recv().await {
           stdin.write_all(line.as_bytes()).await?;
           stdin.write_all(b"\n").await?;
           stdin.flush().await?;
           generation += 1;
       }
       // On channel close → stdin drops, Node sees EOF
   });
   ```

3. **งานระบาย stderr (async)**:

   ```rust
   let stderr = child.stderr.take().unwrap();
   tokio::spawn(async move {
       let reader = BufReader::new(stderr);
       let mut lines = reader.lines();
       while let Some(Ok(line)) = lines.next_line().await {
           tracing::warn!("[worker stderr] {}", line);
       }
   });
   ```

4. **งานอ่าน stdout (async)**:

   ```rust
   let stdout = child.stdout.take().unwrap();
   tokio::spawn(async move {
       let reader = BufReader::new(stdout);
       let mut lines = reader.lines();
       while let Some(Ok(line)) = lines.next_line().await {
           let response: WorkerResponse = serde_json::from_str(&line)?;

           let terminal = response.is_terminal();
           if let Some(pr) = pending.response(&response.id, terminal).await {
               pr.sender.send(response).await.ok();
           }
       }
       // On EOF: drain pending, mark dead
       alive.store(false, Ordering::Release);
       for (_, pr) in pending.take_all().await {
           pr.sender.send(stream_error()).await.ok();
       }
   });
   ```

5. คืนค่า `Worker { stdin_tx, pending, child, alive }`

---

## โปรโตคอลการสื่อสาร (Communication Protocol)

### การทำให้คำขอเป็นอนุกรม (`WorkerRequest`)

JSON ที่มีแท็กผ่าน `#[serde(tag = "type")]`:

```rust
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkerRequest {
    Ssr {
        id: String,
        project_root: PathBuf,        // serde: projectRoot
        app_dir: PathBuf,             // serde: appDir
        page_file: PathBuf,           // serde: pageFile
        request_path: String,         // serde: requestPath
        params: RouteParams,
    },
    Api {
        id: String,
        project_root: PathBuf,
        route_file: PathBuf,          // serde: routeFile
        method: String,
        request_path: String,         // serde: requestPath
        headers: HashMap<String, String>,
        header_pairs: Vec<(String, String)>,  // serde: headerPairs (preserves order)
        body: Option<String>,
        body_base64: Option<String>,  // serde: bodyBase64
        stream_response: bool,        // serde: streamResponse
        params: RouteParams,
    },
    Action {
        id: String,
        project_root: PathBuf,
        action_file: PathBuf,         // serde: actionFile
        action_name: String,          // serde: actionName
        payload_json: String,         // serde: payloadJson
        content_type: String,         // serde: contentType
        request_path: String,         // serde: requestPath
    },
    Client {
        id: String,
        project_root: PathBuf,
        app_dir: PathBuf,
        page_file: PathBuf,
        request_path: String,
        params: RouteParams,
    },
    Invalidate {
        id: String,
        paths: Vec<String>,
    },
    Ping {
        id: String,
    },
    Warmup {
        id: String,
        project_root: PathBuf,
        routes: Vec<WarmupRoute>,
    },
    Ssg {
        id: String,
        project_root: PathBuf,
        app_dir: PathBuf,
        page_file: PathBuf,
        request_path: String,
        params: RouteParams,
        mode: Option<String>,          // "full" | "ppr"
        fresh: Option<bool>,
    },
    StaticParams {
        id: String,
        project_root: PathBuf,
        page_file: PathBuf,           // serde: pageFile
        route_path: String,           // serde: routePath
        segments: Vec<String>,        // dynamic segment names
        routes: Vec<RouteEntry>,   // serde: routes (for global params resolve)
    },
}
```

### การทำให้ตอบกลับเป็นดีซีเรียลไลซ์ (`WorkerResponse`)

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerResponse {
    pub id: String,
    pub ok: bool,
    pub frame: Option<String>,          // "api-start" | "api-chunk" | "api-end" | "api-error"
    pub html: Option<String>,
    pub script: Option<String>,
    pub status: Option<u16>,
    pub headers: Option<HashMap<String, String>>,
    pub header_pairs: Option<Vec<(String, String)>>,  // serde: headerPairs
    pub body: Option<String>,
    pub body_base64: Option<String>,    // serde: bodyBase64
    pub code: Option<String>,           // error code
    pub message: Option<String>,        // error message
    pub stack: Option<String>,          // JS stack trace
    pub pong: Option<bool>,
    pub warmed: Option<usize>,
    pub module_cache_size: Option<usize>,  // serde: moduleCacheSize
    pub params: Option<Vec<RouteParams>>,   // for StaticParams response
    pub dependency_hash: Option<String>,    // serde: dependencyHash
    pub inputs: Option<Vec<PathBuf>>,
}
```

### ตัวอย่างการแลกเปลี่ยน

**คำขอ SSR**:

```
→ {"type":"ssr","id":"abc-123","projectRoot":"/project","appDir":"/project/app","pageFile":"/project/app/page.tsx","requestPath":"/about","params":{}}
← {"id":"abc-123","ok":true,"html":"<!doctype html><html>...</html>"}
```

**คำขอ API (สตรีมมิ่ง)**:

```
→ {"type":"api","id":"def-456","projectRoot":"/project","routeFile":"/project/app/api/stream/route.ts","method":"GET","requestPath":"/api/stream","headers":{},"streamResponse":true,"params":{}}
← {"id":"def-456","ok":true,"frame":"api-start","status":200,"headers":{"content-type":"text/plain"}}
← {"id":"def-456","ok":true,"frame":"api-chunk","bodyBase64":"SGVsbG8="}
← {"id":"def-456","ok":true,"frame":"api-chunk","bodyBase64":"V29ybGQ="}
← {"id":"def-456","ok":true,"frame":"api-end"}
```

**การตอบกลับข้อผิดพลาด**:

```
→ {"type":"ssr","id":"ghi-789",...}
← {"id":"ghi-789","ok":false,"code":"RUV1100","message":"React SSR failed","stack":"Error: ...\n    at ..."}
```

---

## การส่งคำขอ (Sending Requests)

### `send<F>(&self, build_request: F) → Result<WorkerResponse> where F: FnOnce(String) -> WorkerRequest`

```rust
async fn send<F>(&self, build_request: F) -> Result<WorkerResponse>
{
    let (index, worker) = self.select_worker().await?;
    let id = Uuid::new_v4().to_string();
    let request = build_request(id.clone());
    let line = serde_json::to_string(&request)?;

    // Create response channel first (before sending, to avoid race)
    let (tx, rx) = mpsc::channel(MAX_PENDING_RESPONSE_FRAMES);
    worker.pending
        .insert(id.clone(), PendingResponse {
            sender: tx,
            streaming: Arc::new(AtomicBool::new(false)),
        })
        .await;

    // Send to worker stdin
    let stdin = worker.stdin_tx.lock().unwrap();
    if let Some(tx) = stdin.as_ref() {
        tx.send(line).await?;
    } else {
        return Err(io::Error::new(io::ErrorKind::BrokenPipe, "worker stdin closed"));
    }

    // Wait for response with timeout
    let result = tokio::time::timeout(self.response_timeout, async {
        let mut response = rx.recv().await?;
        while response.frame.is_some() && response.frame.as_deref() != Some("api-error") {
            response = rx.recv().await?;
        }
        Ok(response)
    }).await;

    // Cleanup pending entry
    worker.pending.remove(&id).await;

    match result {
        Ok(Ok(resp)) if resp.ok => Ok(resp),
        Ok(Ok(resp)) => Err(worker_error(resp)),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(timeout_error()),
    }
}
```

### `send_streaming<F>(&self, build_request: F) → Result<(WorkerResponse, WorkerBodyStream)>`

รูปแบบเดียวกันแต่คืนค่า `WorkerBodyStream` สำหรับการบริโภคแบบแบ่งส่วน การตอบกลับเริ่มต้นประกอบด้วย
status + headers (เฟรม `api-start`) เฟรมถัดไปจะมาถึงบนสตรีม

---

## การจำกัดพร้อมกันต่อ Worker (Per-Worker Concurrency)

ฝั่ง Rust จำกัด stdin channel แต่ไม่รู้ว่ามีงานเท่าไหร่ที่ทำงานอยู่ _ภายใน_ worker จึงมีขีดจำกัด
อีกชั้นอยู่ใน `worker-pool.mjs`:

```js
const MAX_CONCURRENT_REQUESTS = positiveIntegerEnv(
  'RUVYXA_WORKER_MAX_CONCURRENCY',
  Math.max(2, Math.min(8, availableParallelism())),
)
```

คำขอจะรอ `acquireRequestSlot()` ก่อน dispatch และเรียก `releaseRequestSlot()` เมื่อเสร็จ
ถ้ายังไม่ถึง ขีดจำกัดการขอ slot จะสำเร็จทันที ทำให้กรณีปกติไม่มี latency เพิ่ม
คำขอส่วนเกินจะเข้าคิวและทำงานเมื่อ slot ว่าง `invalidate` และ `ping` ข้ามคิวไปเลย —
เพราะการทำให้แคชเป็นโมฆะที่ล่าช้าจะทำให้ worker เสิร์ฟบันเดิลเก่าตอนที่ยุ่งที่สุด และ health check
ที่ต้องรอต่อท้าย render จะรายงานผลไม่ทัน

คำขอ `ssr`/`ssg` สำหรับหน้าเดียวกันที่เกิดพร้อมกันยังถูกรวมกัน (coalesced):
การรอครั้งที่สองจะเข้าร่วม promise ของการ render ที่กำลังทำงานอยู่ (`renderCoalesceMap`)
แทนที่จะเริ่ม render ใหม่

---

## การเก็บรักษาและ Recycle Module Graph

การ prerender ใน production ใช้ `render_ssg_isolated` ซึ่ง import bundle ภายใต้ module URL ใหม่ต่อ
path เพื่อไม่ให้ state ของ page-module รั่วไหลระหว่าง path ESM registry ของ Node ไม่เคยคืน URL ที่
โหลดแล้ว ดังนั้นการ import แบบ isolated แต่ละครั้งจะเก็บ module graph เพิ่มอีกหนึ่งชุดอย่างถาวร —
ไม่มี cache eviction ภายใน worker ที่จะเรียกคืนได้ worker ติดตามต้นทุนนี้ใน `registeredModuleUrls`
(รายงานผ่าน `ping`)

```rust
const DEFAULT_ISOLATED_RENDERS_PER_WORKER: usize = 32;
const ISOLATED_RENDER_RECYCLE_ENV: &str = "RUVYXA_PRERENDER_RECYCLE_AFTER";
```

Build worker จะเกษียณตัวเองเมื่อให้บริการ isolated render ครบตามงบประมาณ — แต่จะเกิดขึ้นเฉพาะตอนว่าง
เท่านั้น (การเกษียณ worker ที่ยุ่งจะทำให้ sibling render ที่กำลังดำเนินอยู่ล้มเหลว) และนับเฉพาะ
isolated render (render `ssg` ปกติใช้ module URL ที่แคชไว้ซ้ำและไม่เก็บอะไรเพิ่ม) ถ้า replacement
spawn ไม่ได้ worker ที่อิ่มตัวจะถูกเก็บไว้และรีเซ็ตตัวนับแทนการเสียความจุของพูลระหว่าง build dev
server ส่งค่า `None` สำหรับขอบเขตนี้ — มันไม่เคยขอ isolated import จึงไม่เก็บอะไรและไม่เสียต้นทุนใดๆ
`RUVYXA_PRERENDER_RECYCLE_AFTER=0` ปิดการ recycle สำหรับ build เช่นกัน

---

## แคชภายใน Worker

| แคช             | ขอบเขต                            | ถูกล้างโดย                         |
| --------------- | --------------------------------- | ---------------------------------- |
| `bundleCache`   | `RUVYXA_CACHE_MAX_ENTRIES` (256)  | LRU, `invalidate`, memory pressure |
| `moduleCache`   | `RUVYXA_CACHE_MAX_ENTRIES` (256)  | LRU, memory pressure               |
| Compiler cache  | จัดการโดย `compiler.mjs`          | `invalidate`, memory pressure      |
| ESM module URLs | ไม่มีขอบเขต (ดู recycling ด้านบน) | ไม่มี — แทนที่โพรเซสเท่านั้น       |

ทุก 30 วินาทีจะตรวจสอบ `heapUsed` เทียบกับ `RUVYXA_MEMORY_LIMIT_MB` (ค่าเริ่มต้น 512) และถ้าเกิน
จะล้าง `bundleCache` ครึ่งหนึ่ง, ล้าง `moduleCache`, และล้าง compiler cache ตัวจับเวลาถูก `unref`
เพื่อไม่ให้โพรเซสค้างอยู่เพราะมัน

---

## ตัวแปรสภาพแวดล้อม (Environment Variables)

| ตัวแปร                           | ค่าเริ่มต้น          | ผล                                                        |
| -------------------------------- | -------------------- | --------------------------------------------------------- |
| `RUVYXA_WORKER_POOL_SIZE`        | จำนวน CPU (2–8)      | จำนวนโพรเซส worker ในพูล dev/prod                         |
| `RUVYXA_WORKER_MAX_CONCURRENCY`  | จำนวน CPU (2–8)      | จำนวนคำขอที่ worker หนึ่งทำงานพร้อมกัน                    |
| `RUVYXA_WORKER_TIMEOUT_MS`       | 30000 / 300000 build | Deadline ต่อคำขอ ใช้ร่วมกันระหว่าง Rust และ Node watchdog |
| `RUVYXA_PRERENDER_RECYCLE_AFTER` | 32 (`0` ปิดใช้งาน)   | จำนวน isolated prerender ก่อนที่ build worker จะถูกเกษียณ |
| `RUVYXA_CACHE_MAX_ENTRIES`       | 256                  | จำนวนรายการแคช bundle และ module ต่อ worker               |
| `RUVYXA_MEMORY_LIMIT_MB`         | 512                  | ขีดจำกัด heap ที่กระตุ้นการล้างแคชภายใน worker            |

---

## การสังเกตการณ์ (`ping`)

`ping` คืนค่าสถานะที่จำเป็นสำหรับวินิจฉัยพูลที่ค้างจากภายนอก:

| ฟิลด์                   | ความหมาย                                                   |
| ----------------------- | ---------------------------------------------------------- |
| `activeRequests`        | คำขอที่กำลังทำงานอยู่                                      |
| `queuedRequests`        | คำขอที่รอ slot — ถ้าไม่เป็นศูนย์ต่อเนื่องแปลว่าพูลคือคอขวด |
| `maxConcurrentRequests` | ขีดจำกัดการรับคำขอที่ใช้อยู่                               |
| `cacheSize`             | จำนวนรายการ bundle cache                                   |
| `moduleCacheSize`       | จำนวนรายการ module cache                                   |
| `retainedModuleUrls`    | module graph ที่ ESM registry ไม่สามารถคืนได้              |
| `coalesceMapSize`       | จำนวน render ที่กำลังทำงานและถูกแชร์โดยมากกว่าหนึ่งคำขอ    |

---

## การเลือก Worker และการกู้คืนจากความล้มเหลว (Worker Selection & Failure Recovery)

### การเลือกแบบโหลดน้อยที่สุดพร้อมการตัดสินเสมอ

```rust
async fn select_worker(&self) -> Result<(usize, Arc<Worker>)> {
    let workers = {
        let guard = self.workers.read().map_err(|_| worker_pool_lock_error())?;
        guard.clone() // stable worker snapshot; pending counts are atomic
    };
    let start = self.next_worker.fetch_add(1, Ordering::Relaxed) as usize;
    let mut best = None;
    for offset in 0..workers.len() {
        let index = (start + offset) % workers.len();
        let load = workers[index].in_flight();
        if load == 0 { return Ok((index, Arc::clone(&workers[index]))); }
        if best.as_ref().is_none_or(|(_, best_load)| load < *best_load) {
            best = Some((index, load));
        }
    }
    let index = best.unwrap().0;
    Ok((index, Arc::clone(&workers[index])))
}
```

เคอร์เซอร์ตัดสินใจเฉพาะว่าการสแกนเริ่มที่ตำแหน่งใดเมื่อโหลดเท่ากัน worker ที่ไม่ว่างจะถูกข้ามเมื่อ
sibling มีคำขอที่รอน้อยกว่า ส่วน worker ที่ว่างทั้งหมดยังคงหมุนเวียนกันอย่างเป็นธรรม
การเลือกไม่ต้องรอล็อก pending-response map
ดังนั้นการส่งมอบการตอบกลับและการจัดเส้นทางคำขอไม่สร้างสายโซ่การแย่งชิง

### การกู้คืนจากความล้มเหลวใน `send()`

```rust
if !response.ok || response.code.is_some() {
    // Replace failed worker
    let new_worker = replace_failed_worker(&self.workers, index, &worker)?;

    // Retry if idempotent
    if is_idempotent(&request) {
        return self.send_on_worker(&new_worker, build_request).await;
    }
    return Err(error);
}
```

### คำขอที่ทำซ้ำได้ (Idempotent)

| Request type | Idempotent?              |
| ------------ | ------------------------ |
| Ssr          | ใช่ (เหมือน GET)         |
| Ssg          | ใช่ (เรนเดอร์อย่างเดียว) |
| StaticParams | ใช่ (คำนวณอย่างเดียว)    |
| Client       | ใช่ (คอมไพล์อย่างเดียว)  |
| Ping         | ใช่                      |
| Warmup       | ใช่                      |
| Invalidate   | ใช่                      |
| Api          | ไม่ (side effects)       |
| Action       | ไม่ (การเปลี่ยนแปลง)     |

### `replace_failed_worker(workers, index, old_worker) → Result<Arc<Worker>>`

```rust
fn replace_failed_worker(
    workers: &StdRwLock<Vec<Arc<Worker>>>,
    index: usize,
    old_worker: &Arc<Worker>,
) -> Result<Arc<Worker>> {
    let new_worker = Worker::spawn(&self.worker_script, &self.env)?;
    let mut guard = workers.write().unwrap();

    // Check if worker at index still matches old_worker (may have been replaced concurrently)
    if Arc::ptr_eq(&guard[index], old_worker) {
        guard[index] = Arc::new(new_worker);
        Ok(guard[index].clone())
    } else {
        // Already replaced by another caller. Shutdown our spurious replacement.
        shutdown_worker(&new_worker);
        Ok(guard[index].clone())
    }
}
```

---

## การปิด Worker (Worker Shutdown)

```rust
impl NodeWorkerPool {
    pub async fn shutdown(&self) {
        // 1. Send shutdown signal to each worker stdin (close mpsc sender → Node sees EOF)
        for worker in self.workers.read().unwrap().iter() {
            worker.stdin_tx.lock().unwrap().take(); // Drop sender
        }

        // 2. Wait up to WORKER_SHUTDOWN_TIMEOUT (2s)
        tokio::time::sleep(WORKER_SHUTDOWN_TIMEOUT).await;

        // 3. Kill any still-running children
        for worker in self.workers.read().unwrap().iter() {
            if let Ok(mut child) = worker.child.lock() {
                if let Some(ref mut child) = *child {
                    let _ = child.start_kill();
                    let _ = child.wait();
                }
            }
        }
    }
}
```

---

## การทำให้แคชบันเดิลเป็นโมฆะ (Bundle Cache Invalidation)

### แบบอะซิงก์: `invalidate(paths)`

```rust
pub async fn invalidate(&self, paths: &[String]) -> Result<()> {
    let workers = self.workers.read().unwrap().clone();
    let mut join_set = JoinSet::new();

    for worker in &workers {
        let worker = worker.clone();
        let paths = paths.to_vec();
        join_set.spawn(async move {
            worker.send(|id| WorkerRequest::Invalidate { id, paths }).await
        });
    }

    // Wait for all workers to acknowledge invalidation
    while let Some(result) = join_set.join_next().await {
        result??;  // Propagate errors
    }
    Ok(())
}
```

การทำให้เป็นโมฆะแบบขนานในทุก worker: `max(worker_latency)` แทน `sum(worker_latency)`

### แบบซิงก์: `invalidate_from_watcher(paths)`

```rust
pub fn invalidate_from_watcher(&self, paths: &[String]) -> Result<()> {
    // File watcher callback — no tokio runtime available
    let workers = self.workers.read().unwrap();
    for worker in workers.iter() {
        let request = WorkerRequest::Invalidate {
            id: Uuid::new_v4().to_string(),
            paths: paths.to_vec(),
        };
        let line = serde_json::to_string(&request)? + "\n";

        let stdin = worker.stdin_tx.lock().unwrap();
        if let Some(tx) = stdin.as_ref() {
            tx.try_send(line).ok();  // Non-blocking, ignore errors
        }
    }
    Ok(())
}
```

ใช้ `try_send()` — ถ้าช่องสัญญาณเต็ม ให้ทิ้งการทำให้เป็นโมฆะ (ไม่สำคัญ, worker จะได้แคชที่เก่า)
ผู้เรียกจะถอยไปใช้การโหลดใหม่ทั้งหมดถ้าวิธีนี้ล้มเหลว

---

## เมธอด Public API

```rust
impl NodeWorkerPool {
    pub async fn start(root: &Path, env: BTreeMap<String, String>) -> Result<Arc<Self>>;
    pub async fn shutdown(&self);

    // Rendering
    pub async fn render_ssr(&self, root, app_dir, page_file, request_path, params) -> Result<SsrResult>;
    pub async fn render_ssg(&self, root, app_dir, page_file, request_path, params, mode) -> Result<SsgResult>;
    pub async fn render_api(&self, root, route_file, method, path, headers, body, stream, params) -> Result<ApiResult>;
    pub async fn render_action(&self, root, action_file, name, payload, content_type, path) -> Result<ActionResult>;
    pub async fn render_client(&self, root, app_dir, page_file, request_path, params) -> Result<ClientResult>;

    // Maintenance
    pub async fn ping(&self) -> Result<bool>;
    pub async fn warmup(&self, root: PathBuf, routes: Vec<RouteWarmupEntry>) -> Result<usize>;
    pub async fn resolve_static_params(&self, root, page_file, route_path, segments, routes) -> Result<Vec<RouteParams>>;

    // Cache management
    pub async fn invalidate(&self, paths: &[String]) -> Result<()>;
    pub fn invalidate_from_watcher(&self, paths: &[String]) -> Result<()>;

    // Internal
    async fn select_worker(&self) -> Result<(usize, Arc<Worker>)>;
    async fn send<F>(&self, build_request: F) -> Result<WorkerResponse>;
    async fn send_streaming<F>(&self, build_request: F) -> Result<(WorkerResponse, WorkerBodyStream)>;
}
```

---

## การจัดการหมดเวลา (Timeout Handling)

| Request type                              | Timeout                          |
| ----------------------------------------- | -------------------------------- |
| Interactive (dev SSR, API, Action)        | `response_timeout` (default 30s) |
| Build (SSG, StaticParams, Client, Warmup) | `BUILD_WORKER_TIMEOUT_MS` (300s) |
| Config eval                               | `response_timeout`               |
| Invalidate                                | `response_timeout`               |

หมดเวลาสามารถกำหนดค่าได้ผ่าน env var `RUVYXA_WORKER_TIMEOUT_MS` (ทำให้เป็นปกติและส่งต่อไปยัง Node
ด้วย)

เมื่อหมดเวลา:

- ลบรายการที่รอจาก pending map ของ worker
- คืนค่า `io::ErrorKind::TimedOut`
- Worker จะไม่ถูกแทนที่ (หมดเวลา ≠ worker ล้มเหลว)

---

## การจัดการข้อผิดพลาด (Error Handling)

| Failure mode         | Error type                             | Recovery                                           |
| -------------------- | -------------------------------------- | -------------------------------------------------- |
| Worker stdin ปิด     | `BrokenPipe`                           | `replace_failed_worker()`                          |
| Worker stdout EOF    | `UnexpectedEof`                        | `alive = false`, ระบาย pending                     |
| คำขอหมดเวลา          | `TimedOut`                             | ลบ pending, ผู้เรียกจัดการ                         |
| Response `ok: false` | `WorkerError` พร้อม code/message/stack | `replace_failed_worker()` + ลองใหม่ถ้า idempotent  |
| Worker โพรเซส crash  | `alive = false`                        | ตรวจพบตอนส่งครั้งถัดไป → `replace_failed_worker()` |
