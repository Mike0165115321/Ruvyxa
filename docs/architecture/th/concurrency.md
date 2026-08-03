# โมเดลการทำงานพร้อมกันและประสิทธิภาพ

วิธีการใช้ threads, locks, channels, และ parallelism ของ Ruvyxa ในชั้น Rust

---

## แผนที่ Lock และการซิงค์

| คอมโพเนนต์                         | กลไก                                             | Crate       | เหตุผล                                          |
| ---------------------------------- | ------------------------------------------------ | ----------- | ----------------------------------------------- |
| **ResolveGraphCache.resolutions**  | `DashMap<(Arc<str>, Arc<str>), Option<PathBuf>>` | dashmap     | อ่านหนัก, อ่านแบบไม่มี lock, 64 shards          |
| **ResolveGraphCache.sources**      | `DashMap<PathBuf, CachedSource>`                 | dashmap     | อ่าน source พร้อมกัน                            |
| **ResolveGraphCache.tsconfigs**    | `DashMap<PathBuf, CachedTsConfig>`               | dashmap     | อ่าน tsconfig ไม่บ่อย                           |
| **ResolveGraphCache.dependencies** | `DashMap<DependencyCacheKey, Arc<[PathBuf]>>`    | dashmap     | รายการ dep ที่แคชไว้                            |
| **CompileCache.memory**            | `Arc<Mutex<HashMap<String, MemEntry>>>`          | std::sync   | เขียนไม่บ่อย, LRU ต้องเรียงลำดับถูกต้อง         |
| **CompileCache.disk**              | Atomic file writes (temp + rename)               | std::fs     | ไม่มีการเขียนพร้อมกันไปยัง key เดียว            |
| **RenderCache.entries**            | `tokio::sync::RwLock<HashMap<...>>`              | tokio       | เข้าถึงแบบ async, อ่านเป็นส่วนใหญ่              |
| **RenderCache.order**              | `tokio::sync::RwLock<VecDeque<...>>`             | tokio       | ถือร่วมกับ entries ระหว่างเขียน                 |
| **RenderCache.hits/misses**        | `AtomicU64`                                      | std::sync   | Relaxed ordering, สถิติเท่านั้น                 |
| **HmrTracker.file_to_routes**      | `parking_lot::RwLock<BTreeMap<...>>`             | parking_lot | ใช้แบบซิงค์ (notify callback, ไม่มี tokio)      |
| **HmrTracker.route_to_files**      | `parking_lot::RwLock<BTreeMap<...>>`             | parking_lot | เหมือนข้างบน                                    |
| **RuntimeCache.manifest**          | `tokio::sync::RwLock<Option<...>>`               | tokio       | อ่าน/เขียน manifest แบบ async                   |
| **RuntimeCache.styles**            | `tokio::sync::RwLock<Option<...>>`               | tokio       | อ่าน/invalidate styles แบบ async                |
| **RuntimeCache.router**            | `tokio::sync::RwLock<Option<...>>`               | tokio       | สร้าง router ใหม่เมื่อ manifest เปลี่ยน         |
| **WorkerPool.workers**             | `StdRwLock<Vec<Arc<Worker>>>`                    | std::sync   | เขียนไม่บ่อย (กู้คืนเมื่อล้มเหลว)               |
| **WorkerPool.next_worker**         | `AtomicU64`                                      | std::sync   | cursor สำหรับการเลือก worker ที่โหลดน้อยที่สุด  |
| **Worker.stdin_tx**                | `StdMutex<Option<mpsc::Sender<String>>>`         | std::sync   | Drop = ส่งสัญญาณปิด                             |
| **Worker.pending.entries**         | `Arc<Mutex<BTreeMap<String, PendingResponse>>>`  | tokio       | ทำให้การเปลี่ยนแปลง lifecycle request เป็นลำดับ |
| **Worker.pending.count**           | `AtomicUsize`                                    | std::sync   | สังเกตโหลดแบบไม่ต้อง lock ระหว่างเลือก worker   |
| **Worker.child**                   | `Mutex<Option<Child>>`                           | std::sync   | ป้องกัน kill_on_drop + shutdown                 |
| **ISR revalidating set**           | `tokio::sync::Mutex<HashSet<String>>`            | tokio       | Async lock, รวมการ revalidate พร้อมกัน          |
| **Action rate limiter**            | `Arc<Mutex<ActionRateLimiter>>`                  | std::sync   | ผู้เขียนเดียว, ส่วนสั้น                         |
| **Content module cache**           | `OnceLock<Mutex<HashMap<...>>>`                  | std::sync   | Global แชร์, เริ่มต้นขี้เกียจ                   |
| **PluginHost.worker**              | `tokio::sync::Mutex<PluginWorker>`               | tokio       | ทำให้การเรียกไปยัง plugin runtime เป็นลำดับ     |

---

## ชนิดและความจุของ Channel

| Channel             | ชนิด                                       | ความจุ                           | จุดประสงค์                                                             |
| ------------------- | ------------------------------------------ | -------------------------------- | ---------------------------------------------------------------------- |
| HMR broadcast       | `tokio::sync::broadcast::Sender<String>`   | 64                               | ส่ง HMR events ไปยัง WebSocket clients ทั้งหมด ทิ้งอันเก่าสุดเมื่อเต็ม |
| Worker stdin        | `mpsc::Sender<String>` ต่อ worker          | 256                              | ส่งคำขอไปยัง Node subprocess                                           |
| Worker response     | `mpsc::Sender<WorkerResponse>` ต่อ request | 16 (MAX_PENDING_RESPONSE_FRAMES) | ช่อง response ต่อ request, backpressure สำหรับ streaming               |
| Dev server shutdown | `tokio::sync::watch::Sender<bool>`         | 1                                | ส่งสัญญาณปิดเซิร์ฟเวอร์                                                |

---

## โมเดลการทำงานแบบขนาน (Parallelism)

### Rayon parallelism (CPU-bound)

| งาน                             | รูปแบบ                                         | หมายเหตุ                                                 |
| ------------------------------- | ---------------------------------------------- | -------------------------------------------------------- |
| Module resolution (phase 2 BFS) | `frontier.par_iter()`                          | ผสม I/O + CPU; แต่ละระดับ frontier แก้พร้อมกัน           |
| Module compilation              | `compiled.par_iter()` → Oxc transform          | คอมไพล์แต่ละโมดูลแบบขนานเต็มที่                          |
| Linker (parallel)               | `modules.par_chunks()` → IIFE generation       | สำหรับ >=8 modules; สร้าง segments พร้อมกัน, ต่อสายเรียง |
| Image optimization              | `entries.par_iter()` → decode + encode WebP    | `workers` จำกัด thread pool ได้                          |
| Build: prepare bundles          | `routes.par_iter()` → `prepare_bundle()`       | ทุก routes แก้+คอมไพล์พร้อมกัน                           |
| Prerender rendering             | `route_groups.par_iter()` → worker pool render | Max parallelism 2 (build mode ใช้ dedicated pool)        |

### Tokio async parallelism (I/O-bound)

| งาน                           | รูปแบบ                             | หมายเหตุ                           |
| ----------------------------- | ---------------------------------- | ---------------------------------- |
| Dev server: request handling  | Axum handlers พร้อมกัน             | หนึ่ง task ต่อ request             |
| Worker pool: ส่งไปยัง workers | `JoinSet` พร้อมกัน                 | invalidate workers ทั้งหมดพร้อมกัน |
| ISR revalidation              | `tokio::spawn()` ต่อ revalidation  | Background refresh แบบไม่บล็อก     |
| File watcher                  | notify callback → sync → broadcast | notify ทำงานบน thread ของ OS       |
| Worker stdin/out/stderr       | 3 tokio tasks ต่อ worker           | reader/writer/drain อิสระ          |

---

## เส้นทางวิกฤตและการวิเคราะห์คอขวด

### เส้นทางร้อน (Hot paths — ต่อ request, dev server)

1. **Route lookup**: `RadixRouter::find()` — O(ความลึกของ path) Radix trie พร้อมลูกแบบ linear scan
   ยอมรับได้สำหรับเส้นทางทั่วไป (ความลึก < 10)
2. **Render cache get**: `RwLock<HashMap>.read()` + `VecDeque` promote — O(1) Tokio RwLock
   จัดการอ่านพร้อมกันได้ดี
3. **Worker pool send**: ตรวจสอบ pending count ของแต่ละ worker โดยไม่ต้องล็อก response-map เริ่มที่
   rotating cursor แล้วใช้ worker ที่โหลดน้อยที่สุด การสแกนจำกัดที่ 2-8 workers และการเขียน NDJSON
   channel เป็น O(1)
4. **HTML composition**: string search (`find_ascii_case`) + `format!()` — เล็กน้อย
5. **Style collection refresh** (เมื่อ CSS เปลี่ยน): import graph BFS + `grass` Sass compilation —
   อาจใช้ 50-200ms โดยปกติแคชไว้, คำนวณใหม่เมื่อ invalidate

### เส้นทางร้อน (ต่อ request, production)

เหมือน dev แต่ไม่มี HMR + error overlay overhead Cache TTL สูงกว่า (1800s vs 300s)

### เส้นทางร้อนของ Build

1. **Route discovery**: `WalkDir` ของ `app/` ความลึกทั่วไป < 5, จำนวนไฟล์ < 500
2. **Client bundling**: เตรียม routes แบบขนาน (ส่วนใหญ่ใช้เวลา Oxc transforms) Oxc เร็วกว่า
   Babel/SWC 10-100x โดยทั่วไป 50-500ms ต่อ route ขึ้นอยู่กับความลึกของ import
3. **Image optimization**: `image::open()` decode + `webp::Encoder` งานที่หนักที่สุดต่อไฟล์ ขนานผ่าน
   rayon
4. **Prerendering**: Node worker rendering (SSG/ISR/PPR) I/O-bound, max parallelism 2 เพื่อป้องกัน
   worker pool หมด

---

## สถานการณ์การแย่งชิง Lock

| สถานการณ์                                | ความเสี่ยง | การบรรเทา                                                           |
| ---------------------------------------- | ---------- | ------------------------------------------------------------------- |
| อ่าน render cache พร้อมกัน               | ต่ำ        | tokio RwLock, อ่านเป็นส่วนใหญ่                                      |
| เขียน render cache + invalidate พร้อมกัน | ปานกลาง    | ล็อกทั้งสองพร้อมกันช่วงสั้น; เส้นทางเขียนไม่บ่อยใน prod (แคชไว้)    |
| เพิ่ม/ลบ Worker pending map              | ต่ำ        | Mutex ทำให้ lifecycle writes เป็นลำดับ; การเลือกอ่าน atomic counter |
| Compile cache LRU eviction               | ต่ำ        | Local Mutex, ถือเฉพาะระหว่าง check-and-evict                        |
| ResolveGraphCache ความพร้อมกันสูง        | ต่ำมาก     | DashMap: 64 shards, RwLock ต่อ shard, เฉลี่ย 1/64 contention        |
| HMR event ระหว่างเขียน render cache      | ต่ำ        | ชนิด lock ต่างกัน (parking_lot vs tokio)                            |
| ISR revalidation set                     | ต่ำ        | Tokio Mutex ถือเฉพาะ insert/remove; ส่วนสั้น                        |

---

## ตัวปรับแต่งประสิทธิภาพ

| พารามิเตอร์                       | ตั้งที่ไหน              | ค่าเริ่มต้น         | สูงสุด            | ผล                                         |
| --------------------------------- | ----------------------- | ------------------- | ----------------- | ------------------------------------------ |
| `build.workers`                   | `ruvyxa.config.ts`      | ตามจำนวน CPU        | —                 | ขนาน bundling และ prerendering             |
| `middleware.workers`              | `ruvyxa.config.ts`      | 1                   | 8                 | กระบวนการ plugin middleware ที่ไม่มี state |
| `middleware.timeoutMs`            | `ruvyxa.config.ts`      | 30000               | 300000            | timeout สำหรับหนึ่ง middleware hook        |
| `RUVYXA_RENDER_CACHE_SIZE`        | Env var                 | 1024 dev / 512 prod | 16384             | cache มากขึ้น = SSR render น้อยลง          |
| `RUVYXA_WORKER_TIMEOUT_MS`        | Env var                 | 30000               | i32::MAX          | timeout สำหรับ workers ที่ค้าง             |
| `RUVYXA_JSX_RUNTIME`              | Env var (ตั้งอัตโนมัติ) | automatic           | automatic/classic | JSX transform runtime                      |
| `security.actionRateLimit.max`    | Config                  | แตกต่างกัน          | —                 | การกระทำต่อ window ต่อ key                 |
| `security.actionRateLimit.window` | Config                  | แตกต่างกัน          | —                 | หน้าต่าง rate limit (วินาที)               |
| `image.quality`                   | Config                  | 82                  | 0-100             | คุณภาพ WebP (ต่ำ = เร็ว, เล็ก)             |
| `image.parallelism`               | Config                  | 0 (global)          | —                 | จำนวน thread สำหรับรูปภาพ                  |

---

## ลักษณะการใช้หน่วยความจำ

| คอมโพเนนต์                            | หน่วยความจำโดยประมาณต่อหน่วย                     |
| ------------------------------------- | ------------------------------------------------ |
| โมดูลที่คอมไพล์แล้ว (compile cache)   | ~50-200KB (string JS + deps)                     |
| โมดูลที่ resolve แล้ว (resolve cache) | ~10-100KB (source + deps)                        |
| Render cache entry                    | ~5-500KB (string HTML)                           |
| Source file cache                     | ~ขนาดไฟล์ (mmap สำหรับ >64KB)                    |
| Compile cache (disk)                  | ~500KB-2MB ต่อ key (JS ที่มี blake3 key บนดิสก์) |
| Worker process (Node)                 | ~50-150MB ต่อ worker                             |

รวมต่อ dev session: ~200-500MB (4 workers + compile cache + render cache + source cache)
