# Worker Pool · กลุ่มผู้ทำงาน

**Modules**: `crates/ruvyxa_dev_server/src/worker_pool.rs`,
`packages/ruvyxa/runtime/worker-pool.mjs`  
**Crate**: `ruvyxa_dev_server`

## สรุป

Worker pool คือกลุ่ม process ของ Node/Bun ที่อยู่ยาว ทำหน้าที่รัน JavaScript ทั้งหมดของแอป — SSR,
SSG, API routes, server actions, client bundle — โดยคุยกับฝั่ง Rust ผ่าน NDJSON บน stdin/stdout ฝั่ง
Rust เป็นผู้เลือก worker และจัดการ lifecycle ส่วนฝั่ง Node เป็นผู้จำกัด concurrency ภายในตัวเอง

---

## Why Separate Processes

Rendering has to happen in a JavaScript runtime: it calls React, the app's own modules, and whatever
those import. That rules out doing it inside the Rust process. The remaining choice is how the
JavaScript runs.

- **A process per request** pays full Node startup plus a cold module graph on every render.
- **One long-lived process** keeps the bundle and module caches warm but serializes every render
  behind one event loop, and one bad render takes the whole server with it.
- **A pool of long-lived processes** (chosen) amortizes startup, keeps caches warm per worker,
  renders in parallel across processes, and contains a crash to one worker.

Each worker is an OS process, so a segfault, an OOM kill, or a runaway render is isolated and its
slot can be replaced without touching the others.

---

## Protocol

One JSON object per line, in both directions. Every request carries an `id`; the response echoes it,
which is what lets several requests be in flight on one worker's stdin at once.

Request types (`WorkerRequest`, serialized with `#[serde(tag = "type")]`):

| Type           | Purpose                                    | Idempotent |
| -------------- | ------------------------------------------ | ---------- |
| `ssr`          | Render a page for a request path           | ✅         |
| `ssg`          | Render a page for prerendering             | ✅         |
| `staticParams` | Resolve `generateStaticParams` for a route | ✅         |
| `client`       | Build the browser bundle for a route       | ✅         |
| `api`          | Execute an API route handler               | ❌         |
| `action`       | Execute a server action                    | ❌         |
| `warmup`       | Pre-populate the bundle cache              | ✅         |
| `invalidate`   | Drop cache entries for changed files       | ✅         |
| `ping`         | Health check and worker statistics         | ✅         |

`is_idempotent()` is the single source of truth for what may be retried. `api` and `action` are
excluded because re-running them would duplicate side effects.

An `api` response is **framed** rather than a single message: `api-start`, then any number of
`api-chunk` frames (body bytes base64-encoded, 64 KiB each), then a terminal frame. This is what
makes streaming responses possible across the boundary. `WorkerResponse::is_terminal()` decides when
a response is complete, and `header_pairs` is preferred over the legacy `headers` map so repeated
`Set-Cookie` values survive.

---

## Pool Size

```rust
const DEFAULT_POOL_SIZE: usize = 4;
const MIN_POOL_SIZE: usize = 2;
const MAX_POOL_SIZE: usize = 8;
```

The dev and production servers use `available_parallelism()` clamped to 2–8, overridable with
`RUVYXA_WORKER_POOL_SIZE`. A build passes an explicit count instead, clamped to 1–8: a build with
one prerender job should not start an idle second process just because a long-lived server wants a
higher minimum.

Workers are spawned concurrently — each spawn does blocking process setup — and the pool pings
worker 0 before returning, so a pool that cannot execute JavaScript fails at startup rather than on
the first request.

---

## Worker Selection

```rust
async fn select_worker(&self) -> Result<(usize, Arc<Worker>)>
```

Least-loaded by in-flight count, with a rotating start offset to break ties, short-circuiting on the
first idle worker.

Blind round-robin would ignore load: a burst can stack several requests behind one worker still
blocked in a CPU-bound `renderToString` while a sibling sits idle. The rotating offset preserves
round-robin behaviour when every worker is equally idle.

The `Arc<Worker>` handles are cloned out of the lock before probing, so worker replacement cannot
race with a selection snapshot.

---

## Per-Worker Concurrency

The Rust side bounds its stdin channel but has no view of how much work is in flight _inside_ a
worker, so the limit lives in `worker-pool.mjs`:

```js
const MAX_CONCURRENT_REQUESTS = positiveIntegerEnv(
  'RUVYXA_WORKER_MAX_CONCURRENCY',
  Math.max(2, Math.min(8, availableParallelism())),
)
```

A request awaits `acquireRequestSlot()` before dispatch and calls `releaseRequestSlot()` when it
finishes, which starts the longest-waiting queued request. Below the limit the acquire resolves
synchronously, so the common case does not wait in the request-slot queue. This is a control-flow
property, not a measured guarantee about wall-clock latency.

Renders are CPU-bound and each one holds a React tree, a compiled bundle, and its response buffer.
Admitting a whole burst at once exhausts the heap or thrashes the CPU into timeouts that present as
hangs. `invalidate` and `ping` bypass the queue — delaying a cache invalidation would leave the
worker serving stale bundles exactly when it is busiest, and a health check that queues behind
renders cannot report on them.

Concurrent `ssr` and `ssg` requests for the same page are also **coalesced**: the second await joins
the first render's promise rather than starting its own (`renderCoalesceMap`).

---

## Failure Recovery

```rust
pub async fn send(&self, request: WorkerRequest) -> Result<WorkerResponse>
async fn replace_failed_worker(&self, index: usize, failed: &Arc<Worker>) -> Option<Arc<Worker>>
```

| Scenario                                   | Behavior                                                 |
| ------------------------------------------ | -------------------------------------------------------- |
| Worker returns an error response           | Propagated to the caller; worker left alive              |
| Send/receive fails, request idempotent     | Worker replaced, request retried once on the replacement |
| Send/receive fails, request not idempotent | Worker replaced, error returned                          |
| Response times out                         | Error returned; the Node watchdog fails the request too  |
| Replacement cannot spawn                   | Logged; the pool continues at reduced capacity           |

The worker slot is replaced before the retry decision, so the failed worker cannot be selected again
by a concurrent request. A streaming API response that was already in flight when its worker exited
is delivered a body error (`RUV1704`) rather than a clean EOF, so a truncated response is never
mistaken for a complete one.

---

## Module Graph Retention and Recycling

Production prerendering uses `render_ssg_isolated`, which imports the bundle under a **fresh module
URL** so page-module state cannot leak between paths. Node's ESM registry never releases a loaded
URL, so each isolated import permanently retains one more module graph. No cache eviction inside the
worker can reclaim it — the worker tracks the cost in `registeredModuleUrls` and reports it through
`ping`, but replacing the process is the only operation that frees it.

```rust
const DEFAULT_ISOLATED_RENDERS_PER_WORKER: usize = 32;
const ISOLATED_RENDER_RECYCLE_ENV: &str = "RUVYXA_PRERENDER_RECYCLE_AFTER";

fn retains_an_isolated_module_graph(&self) -> bool  // Ssg { fresh: true }
async fn retire_worker_if_saturated(&self, index: usize, worker: &Arc<Worker>)
```

A build worker is retired once it has served its budget of isolated renders. Two conditions guard
it:

- **Only isolated renders count.** A normal `ssg` render reuses a cached module URL and retains
  nothing.
- **Only an idle worker is retired.** `shutdown` clears pending requests, so retiring a busy worker
  would fail sibling renders that were progressing fine. Deferring costs nothing — the counter stays
  over budget until the worker is next idle.

If the replacement cannot spawn, the saturated worker is kept and its counter reset: losing pool
capacity mid-build is strictly worse than carrying the retained graphs.

The dev server passes `None`, disabling recycling. It never requests isolated imports, so it retains
nothing to reclaim and pays nothing for the bound. `RUVYXA_PRERENDER_RECYCLE_AFTER=0` disables it
for builds too.

---

## In-Worker Caches

| Cache           | Bound                            | Evicted by                         |
| --------------- | -------------------------------- | ---------------------------------- |
| `bundleCache`   | `RUVYXA_CACHE_MAX_ENTRIES` (256) | LRU, `invalidate`, memory pressure |
| `moduleCache`   | `RUVYXA_CACHE_MAX_ENTRIES` (256) | LRU, memory pressure               |
| Compiler cache  | Managed by `compiler.mjs`        | `invalidate`, memory pressure      |
| ESM module URLs | **Unbounded** (see above)        | Nothing — process replacement only |

A 30-second interval checks `heapUsed` against `RUVYXA_MEMORY_LIMIT_MB` (512) and, above it, evicts
half of `bundleCache`, clears `moduleCache`, and clears the compiler cache. The timer is `unref`'d
so it never keeps the process alive.

---

## Shutdown

`shutdown(reason)` writes the reason to stderr, refuses new admissions, clears the admission queue,
and exits once active requests drain — or after 5 seconds regardless. `SIGTERM` and `SIGINT` both
route through it. On the Rust side, `NodeWorkerPool::shutdown()` closes each worker's stdin, clears
its pending responses, and waits up to 2 seconds for the child to exit before terminating it, so a
wedged worker cannot hold up server shutdown.

---

## Environment Variables

| Variable                         | Default              | Effect                                                     |
| -------------------------------- | -------------------- | ---------------------------------------------------------- |
| `RUVYXA_WORKER_POOL_SIZE`        | CPU count (2–8)      | Worker processes in the dev/prod pool                      |
| `RUVYXA_WORKER_MAX_CONCURRENCY`  | CPU count (2–8)      | Requests one worker executes at once                       |
| `RUVYXA_WORKER_TIMEOUT_MS`       | 30000 / 300000 build | Per-request deadline, shared by Rust and the Node watchdog |
| `RUVYXA_PRERENDER_RECYCLE_AFTER` | 32 (`0` disables)    | Isolated prerenders before a build worker is retired       |
| `RUVYXA_CACHE_MAX_ENTRIES`       | 256                  | Bundle and module cache entries per worker                 |
| `RUVYXA_MEMORY_LIMIT_MB`         | 512                  | Heap threshold that triggers in-worker cache eviction      |

---

## Observability

`ping` returns the state a stuck pool needs to be diagnosed from the outside:

| Field                   | Meaning                                                                            |
| ----------------------- | ---------------------------------------------------------------------------------- |
| `activeRequests`        | Requests currently executing                                                       |
| `queuedRequests`        | Requests parked on a slot — persistently non-zero means the pool is the bottleneck |
| `maxConcurrentRequests` | The admission limit in effect                                                      |
| `cacheSize`             | Bundle cache entries                                                               |
| `moduleCacheSize`       | Module cache entries                                                               |
| `retainedModuleUrls`    | Module graphs the ESM registry cannot free                                         |
| `coalesceMapSize`       | In-flight renders being shared by more than one request                            |

---

## Why This Design

1. **Processes, not threads** — JavaScript needs a runtime; a process is the only unit that isolates
   one crashing render from the rest of the pool.
2. **NDJSON over stdin/stdout** — No port to bind, no socket to secure, no serialization format to
   version beyond JSON. Works identically for Node and Bun.
3. **Request IDs, not request/response lockstep** — Several requests occupy one worker concurrently,
   which is what makes a data-fetching render's I/O wait useful instead of idle.
4. **Least-loaded selection** — A single slow render cannot serialize unrelated requests behind it.
5. **Retry gated on idempotency** — Recovery never turns one action into two.
6. **Recycling gated on isolation and idleness** — The only unbounded resource is bounded, without
   dropping work in progress or charging the dev server for a cost it does not incur.
