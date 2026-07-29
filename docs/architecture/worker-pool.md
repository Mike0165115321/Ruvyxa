# Worker Pool · กลุ่มผู้ทำงาน

**Module**: `crates/ruvyxa_dev_server/src/worker_pool.rs`  
**Crate**: `ruvyxa_dev_server`

## สรุป

Worker pool คือ thread pool สำหรับรัน React SSR rendering แบบขนาน แต่ละ worker มี Tokio runtime
ของตัวเอง แยกจาก server I/O เพื่อป้องกัน CPU-bound rendering ขัดขวาง HTTP accept

---

## Why a Dedicated Pool

React SSR rendering is:

- **CPU-bound** — string concatenation, VDOM walk, component function calls. No I/O wait.
- **Single-threaded per render** — React renders one component tree at a time on one thread.
- **Variable duration** — A simple page renders in <5ms. A complex page with 1000 components may
  take 200ms.

Running SSR on the Tokio async runtime would block the HTTP accept loop. A dedicated thread pool
prevents that.

---

## Core Types

### WorkerPool

```rust
pub struct WorkerPool {
    workers: Vec<WorkerHandle>,
    sender: crossbeam_channel::Sender<WorkerTask>,
    receiver: crossbeam_channel::Receiver<WorkerResult>,
    next_id: AtomicU64,
}

impl WorkerPool {
    pub fn new(count: usize) -> Self;
    pub fn dispatch(&self, task: WorkerTask);
    pub fn collect(&self) -> Vec<WorkerResult>;
    pub fn try_collect_one(&self) -> Option<WorkerResult>;
    pub fn shutdown(self);
}
```

### WorkerHandle

```rust
pub struct WorkerHandle {
    pub id: usize,
    pub thread: JoinHandle<()>,
}
```

### WorkerTask

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerTask {
    pub id: u64,
    pub route_id: String,
    pub url: String,
    pub params: HashMap<String, String>,
}
```

### WorkerResult

```rust
#[derive(Debug, Clone)]
pub struct WorkerResult {
    pub id: u64,
    pub html: String,
    pub status: StatusCode,
    pub headers: HeaderMap,
}
```

---

## Lifecycle

### Initialization

```rust
pub fn new(count: usize) -> Self {
    let (tx_task, rx_task) = crossbeam_channel::bounded::<WorkerTask>(count * 2);
    let (tx_result, rx_result) = crossbeam_channel::bounded::<WorkerResult>(count * 2);

    let workers = (0..count)
        .map(|id| {
            let rx = rx_task.clone();
            let tx = tx_result.clone();
            let thread = std::thread::spawn(move || {
                // Each worker has its own Tokio runtime
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                while let Ok(task) = rx.recv() {
                    let result = rt.block_on(render_route(task));
                    tx.send(result).ok();
                }
            });
            WorkerHandle { id, thread }
        })
        .collect();

    Self { workers, sender: tx_task, receiver: rx_result, next_id: AtomicU64::new(0) }
}
```

### Dispatch

```rust
pub fn dispatch(&self, entry: &RouteEntry, url: &str, params: HashMap<String, String>) -> u64 {
    let task_id = self.next_id.fetch_add(1, Ordering::Relaxed);
    let task = WorkerTask {
        id: task_id,
        route_id: entry.id.clone(),
        url: url.to_string(),
        params,
    };
    self.sender.send(task).ok();
    task_id
}
```

`dispatch()` is non-blocking — it enqueues the task and returns immediately. The sender channel is
bounded at `2 * worker_count`, preventing unbounded queue growth.

### Collection

```rust
pub fn try_collect_one(&self) -> Option<WorkerResult> {
    self.receiver.try_recv().ok()
}
```

The server's accept loop calls `try_collect_one()` after each `dispatch()`. This is a try-recv — it
does not block if no results are ready. Multiple results may be available; the loop drains the
receiver channel each iteration.

### Shutdown

```rust
pub fn shutdown(self) {
    drop(self.sender);
    for worker in self.workers {
        worker.thread.join().ok();
    }
}
```

Dropping the sender causes worker threads to exit their `recv()` loop. `join()` waits for each
thread to finish.

---

## Rendering on the Worker

```rust
async fn render_route(task: WorkerTask) -> WorkerResult {
    // task contains route_id, url, params
    // Worker loads compiled server module for route_id
    // Calls renderToString() or renderToPipeableStream()
    // Returns HTML string + status + headers
}
```

Each worker has its own copy of:

- The server module registry (Arc'd, shared via clone-on-write)
- A React SSR `renderToString` / `renderToPipeableStream` adapter
- The style collector (CSS extraction post-process)

Module registry is `Arc<RwLock<HashMap<String, CompiledModule>>>` — workers share read access. When
HMR updates a module, the lock is write-acquired for that single entry.

---

## Thread Count Selection

```rust
pub fn optimal_worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .saturating_sub(1)  // reserve one thread for HTTP I/O
        .max(1)
}
```

Default: `available_parallelism - 1`. Override via `ServerConfig.worker_count` or
`RUVYXA_WORKER_COUNT` env var.

Guidelines:

| CPU Cores | Workers | Rationale                           |
| --------- | ------- | ----------------------------------- |
| 1         | 1       | No parallelism possible             |
| 2         | 1       | One for HTTP, one for rendering     |
| 4         | 3       | Three render threads, one HTTP      |
| 8         | 7       | Seven concurrent renders            |
| 16+       | 15      | Ditto; diminishing returns after ~8 |

---

## Error Isolation

Worker panic does not crash the server:

```rust
// In worker thread:
let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    rt.block_on(render_route(task))
}));
match result {
    Ok(r) => tx.send(r),
    Err(_) => tx.send(WorkerResult::error(task.id)),
}
```

Panic recovery sends an error `WorkerResult` (status 500, empty body). The worker thread exits. A
replacement worker is spawned lazily on next dispatch.

---

## Comparison

| Approach                       | Pros                             | Cons                        |
| ------------------------------ | -------------------------------- | --------------------------- |
| Single-threaded SSR            | Simple, no sync overhead         | Blocks HTTP on every render |
| `rayon` work-stealing          | No manual pool                   | No per-worker Tokio runtime |
| Dedicated thread pool (chosen) | Isolated runtimes, bounded queue | Fixed thread count          |
| Actor model (per-worker task)  | Fine-grained control             | Higher complexity           |

---

## Why This Design

1. **crossbeam channel** — Bounded, multi-producer multi-consumer. Workers pull tasks when ready;
   server pushes results when done. No mutex contention (channels are lock-free).
2. **Per-worker Tokio runtime** — Workers can independently drive async components (data fetching,
   streaming). A shared runtime would require work-stealing coordination.
3. **Fixed thread count** — Predictable resource usage. No dynamic pool growth surprises in
   production. User controls count via config.
4. **try_recv collection** — Server never blocks waiting for slow renders. If a render takes 500ms,
   other requests continue being accepted and dispatched.
