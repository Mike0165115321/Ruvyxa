# Concurrency Model · โมเดลการทำงานพร้อมกัน

**Scope**: Cross-crate (dev server, bundler, diagnostics)

## สรุป

Ruvyxa uses three distinct concurrency domains: (1) async Tokio for I/O, (2) dedicated OS threads
for SSR rendering, (3) parallel compilation via rayon for bundling. Each domain is designed for its
workload — no one-size-fits-all runtime.

---

## Domain 1: Async I/O (Tokio)

### Where

- Dev server HTTP accept loop
- HMR WebSocket connections
- Static file serving
- Server action handlers

### Mechanism

```rust
use tokio::net::TcpListener;
use axum::{Router, serve};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handler));

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    serve(listener, app).await.unwrap();
}
```

### Runtime

Multiple Tokio runtimes exist:

| Runtime            | Scope                | Thread count                                  |
| ------------------ | -------------------- | --------------------------------------------- |
| Main runtime       | Dev server, CLI      | Multi-thread (default: available_parallelism) |
| Per-worker runtime | SSR rendering thread | Current-thread (1 worker = 1 runtime)         |
| Build runtime      | CLI build pipeline   | Current-thread (sequential)                   |

### Why Tokio (not smol, monoio)

- Axum is built on Tokio. Using a different async runtime would require a bridging layer.
- Tokio's work-stealing scheduler distributes I/O across available cores. One accept loop does not
  starve others.
- `tokio::task::spawn_blocking` is available for CPU-bound operations that cannot be async (e.g.,
  heavy JSON serialization).

---

## Domain 2: Dedicated Thread Pool (SSR Rendering)

### Where

- React `renderToString` / `renderToPipeableStream`
- Build-time static page generation (`ruvyxa build` for SSG routes)

### Mechanism

```rust
pub struct WorkerPool {
    workers: Vec<WorkerHandle>,
    sender: crossbeam_channel::Sender<WorkerTask>,
    receiver: crossbeam_channel::Receiver<WorkerResult>,
}
```

### Why Not Tokio `spawn_blocking`

`spawn_blocking` uses Tokio's blocking thread pool. This pool is unbounded by default and shared
with all other blocking operations. A burst of SSR renders could exhaust the pool, stalling other
blocking tasks (e.g., file I/O). A dedicated bounded pool provides isolation.

### Bounded Channels

```
Sender (bounded, 2× worker_count) → Workers pull tasks
                                   → Workers push results
Receiver (bounded, 2× worker_count) ← Results
```

Bounded channels provide backpressure. If all workers are busy and the queue is full, `dispatch()`
blocks the server accept loop — this is intentional: it signals overload to the client via
connection queueing rather than accepting more work than can be handled.

---

## Domain 3: Parallel Compilation (rayon)

### Where

- File compilation during build (multiple `.tsx` / `.ts` files)
- Module graph analysis (traversal is parallelized per-module)
- Static path generation (one path per rayon job)

### Mechanism

```rust
use rayon::prelude::*;

fn compile_all(inputs: &[BundleInput]) -> Vec<CompiledModule> {
    inputs.par_iter()
        .map(|input| compile_file(input))
        .collect()
}
```

### Why rayon (not manual threads)

- Work-stealing: if one file takes longer to compile, other threads steal remaining work.
- Global thread pool: rayon maintains a thread pool matching `available_parallelism`. No thread
  creation overhead per `par_iter` call.
- No `Send + Sync` gymnastics: rayon handles data distribution.

### Parallelism Boundaries

| Phase                 | Parallelism                     | Method                    |
| --------------------- | ------------------------------- | ------------------------- |
| Route discovery       | Sequential (single walk)        | Not parallelizable        |
| Module compilation    | File-level parallel             | rayon `par_iter`          |
| Module linking        | Sequential (IIFE concatenation) | Single-threaded           |
| Boundary checking     | Module-graph BFS                | Sequential (single graph) |
| Static page rendering | Path-level parallel             | Worker pool dispatch      |
| Minification          | File-level parallel             | rayon `par_iter`          |

---

## Domain 4: Shared State Concurrency

### Module Registry

```rust
pub type ModuleRegistry = Arc<RwLock<HashMap<String, CompiledModule>>>;
```

- Readers (workers rendering routes): `read()` lock — multiple concurrent reads.
- Writer (HMR updating a module): `write()` lock — exclusive, blocks readers.
- Granularity: per-module lock? No — single `RwLock` over the entire registry. HMR updates are rare
  (one file change at a time). The write lock is held briefly (insert one entry). Readers contend
  for the read lock but do not block each other.

### Diagnostic Collector

```rust
pub type SharedCollector = Arc<Mutex<DiagnosticCollector>>;
```

- `Mutex`, not `RwLock`: writes are more frequent than reads during the build phase.
- Contention is low: diagnostics are pushed in sequence, not in tight loops.

### Cache

```rust
pub type SharedCache = Arc<Mutex<LruCache<String, CacheEntry>>>;
```

- `Mutex` protects LRU internals (linked list reordering on access).
- Cache hits are fast (microseconds). Mutex hold time is negligible.

---

## Lock Ordering

```
ModuleRegistry (RwLock) → never acquired while holding
  DiagnosticCollector (Mutex) or
  SharedCache (Mutex)

Acquire order:
  1. SharedCache
  2. ModuleRegistry (read or write)
  3. DiagnosticCollector
```

Enforced by code review. Deadlock is impossible if this order is maintained (no cycle in the
resource allocation graph).

---

## Atomic Operations

```rust
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(0);
```

Task IDs are monotonically increasing, generated without a mutex. No ABA problem (IDs are never
reused).

---

## Concurrency by Crate

| Crate                | Sync primitives                       | Threading model                   |
| -------------------- | ------------------------------------- | --------------------------------- |
| `ruvyxa_graph`       | None (pure functions)                 | Sequential                        |
| `ruvyxa_bundler`     | `rayon`                               | Parallel (file-level)             |
| `ruvyxa_dev_server`  | `Arc<RwLock>`, `Arc<Mutex>`, channels | Mixed (async I/O + thread pool)   |
| `ruvyxa_middleware`  | `Arc<PluginHost>`                     | Async (Tokio tower layers)        |
| `ruvyxa_diagnostics` | `Arc<Mutex>`                          | Sequential                        |
| `ruvyxa_cli`         | Tokio runtime                         | Async (main) + sequential (build) |

---

## Why This Design

1. **Three separate concurrency strategies** — Async I/O, thread pool rendering, and parallel
   compilation each have different optimal strategies. A single `#[tokio::main]` for everything
   would bottleneck CPU-bound SSR rendering against I/O-bound HTTP serving.
2. **Bounded channels for backpressure** — When the system is overloaded, bounded channels cause the
   accept loop to block, which causes the OS to queue TCP connections. This is the correct behavior
   — better to queue at the network layer than to accept requests that will time out.
3. **`rayon` over async compilation** — Compilation is pure CPU work with no I/O waiting. `rayon` is
   more efficient than `tokio::task::spawn_blocking` for CPU parallelism because it uses
   work-stealing and avoids Tokio's task scheduling overhead.
4. **Granular `RwLock` per registry** — HMR updates are write-heavy but infrequent. Multiple
   concurrent reads (from the worker pool) do not block each other. A single `Mutex` would serialize
   all reads.
