# ภาพรวมระบบ Ruvyxa

**หลักการออกแบบ**: Rust จัดการทุกอย่างก่อนขั้นตอนการเรนเดอร์ (route discovery, bundling, resolution,
minification, serving) ส่วน Node.js หรือ Bun จัดการการเรนเดอร์ (React SSR, API execution, config
evaluation) สถาปัตยกรรมแบบผสมผสานนี้ให้ทั้งความเร็วและความปลอดภัยของชนิดข้อมูล (type safety) จาก
Rust ควบคู่กับการเข้าถึงระบบนิเวศ JavaScript

## สถาปัตยกรรมระดับสูง

```
┌──────────────────────────────────────────────────────────┐
│                     ruvyxa_cli                           │
│  (clap command dispatch, config loading, build orchestr) │
├─────────┬──────────┬──────────────┬───────────┬─────────┤
│ruvyxa_   │ruvyxa_   │ruvyxa_dev_   │ruvyxa_    │ruvyxa_  │
│graph     │bundler   │server        │middleware │diag-    │
│(route    │(TS/JSX   │(Axum + HMR + │(Tower     │nostics  │
│disc+val) │comp+link)│router+cache) │+TS host)  │(RUV####)│
└─────────┴──────────┴──────────────┴───────────┴─────────┘
       │         │              │           │
       └─────────┴──────────────┴───────────┘
                       │
             ┌─────────▼─────────┐
             │ Node/Bun Workers  │
             │  (SSR, SSG, API,   │
             │   Action, Config)  │
             └───────────────────┘
```

## ผัง Dependency ของ Crate

```
ruvyxa_diagnostics       (พื้นฐาน: serde + thiserror เท่านั้น)
    ↑
    ├── ruvyxa_graph     (ขึ้นกับ: diagnostics)
    ├── ruvyxa_bundler   (ขึ้นกับ: diagnostics, oxc, grass, dashmap, rayon, memmap2, blake3)
    ├── ruvyxa_middleware (ขึ้นกับ: diagnostics)
    └── ruvyxa_dev_server (ขึ้นกับ: diagnostics, bundler, graph, middleware, axum, notify, tokio)
         │
         └── ruvyxa_cli (ขึ้นกับ: ทุก crate, binary entry ผ่าน clap)
```

## การตัดสินใจออกแบบที่สำคัญ

| การตัดสินใจ                               | เหตุผล                                                                                                                                                         |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Rust core, Node/Bun renderers**         | Rust: เริ่มต้นเร็ว, ความปลอดภัยตอน compile, binary เดียว เมื่อไม่มี Node ให้เลือก Bun โดยอัตโนมัติ Workers ไม่ต้อง spawn กระบวนการใหม่ต่อ request (~100-500ms) |
| **Oxc สำหรับ TS/JSX** (แทน Babel/SWC/TSC) | เร็วกว่า 10-100 เท่า, binary เดียว, ไม่ต้องพึ่งพา Node สำหรับ bundling                                                                                         |
| **Persistent JavaScript worker pool**     | Node หรือ Bun pool: 2-8 workers (ค่าเริ่มต้นตามจำนวน CPU) สื่อสารผ่าน NDJSON ทาง stdin/stdout                                                                  |
| **Radix trie router**                     | O(ความลึกของ path) เทียบกับ O(n) แบบ linear scan, recompile เมื่อ manifest เปลี่ยน                                                                             |
| **Content-hashed assets**                 | ลายนิ้วมือ Blake3 → cache แบบ immutable (max-age=31536000)                                                                                                     |
| **Staging + atomic commit**               | Build เขียนไปยัง staging dir → atomic rename ไม่มี output เสียหาย                                                                                              |
| **Deterministic CSS scoping**             | fnv1a_64(project_relative_path + class_name) — build ได้ผลลัพธ์เหมือนเดิมเสมอ                                                                                  |
| **Strict config**                         | `deny_unknown_fields` — ผิดพลาดทันที ไม่ใช้ค่าเริ่มต้นเงียบ                                                                                                    |

## กลยุทธ์การเรนเดอร์ (Rendering Strategies)

| กลยุทธ์ | ตัวกระตุ้น                              | ลักษณะการทำงาน                                       |
| ------- | --------------------------------------- | ---------------------------------------------------- |
| **CSR** | `"use client"` directive                | HTML เปลือกบาง, hydrate ฝั่ง client                  |
| **PPR** | `export const ppr = true`               | Static shell + streaming dynamic slots               |
| **ISR** | `export const revalidate = <n>`         | Cache + stale-while-revalidate, refresh ในเบื้องหลัง |
| **SSG** | `getStaticParams` หรือ static candidate | Pre-render ตอน build, cache ถาวร                     |
| **SSR** | ค่าเริ่มต้น                             | Server render ต่อ request, cache ชั่วคราว            |

## ขอบเขต Server/Client

มีสองระดับการบังคับใช้: ระดับ graph (source scan ใน `ruvyxa_graph::validate_app`) และระดับ bundle
(compiled output ใน `ruvyxa_bundler::boundary`)

| กฎ                                               | รหัส    | ความรุนแรง |
| ------------------------------------------------ | ------- | ---------- |
| `"server-only"` ใน client bundle                 | RUV1007 | Error      |
| Private `process.env` ใน client                  | RUV1008 | Error      |
| `"client-only"` ใน SSR bundle                    | RUV1009 | Warning    |
| โฟลเดอร์ `server/` ใน client graph               | RUV1010 | Error      |
| อนุญาตเฉพาะ `RUVYXA_PUBLIC_*` env vars ใน client | —       | Convention |

## รูปแบบไฟล์ต้นฉบับ

| รูปแบบ                          | ชนิด               | URL                      |
| ------------------------------- | ------------------ | ------------------------ |
| `app/page.tsx`                  | Page               | `/`                      |
| `app/about/page.tsx`            | Page               | `/about`                 |
| `app/blog/[slug]/page.tsx`      | Dynamic            | `/blog/:slug`            |
| `app/docs/[...rest]/page.tsx`   | Catch-all          | `/docs/*`                |
| `app/shop/[[...cats]]/page.tsx` | Optional catch-all | `/shop` หรือ `/shop/a/b` |
| `app/api/route.ts`              | API                | `/api`                   |
| `app/layout.tsx`                | Layout             | ห่อหุ้ม children         |
| `app/(group)/page.tsx`          | Route group        | `/` (วงเล็บถูกละเว้น)    |
| `app/@modal/page.tsx`           | Parallel slot      | _ไม่สนใจ_                |
| `app/_private/page.tsx`         | Private dir        | _ไม่สนใจ_                |
| `app/action.ts`                 | Server action      | อยู่คู่กับ page          |
| `app/server.ts`                 | Server module      | อยู่คู่กับ page          |
| `app/client.tsx`                | Client module      | อยู่คู่กับ page          |
| `app/page.md` / `.mdx`          | Content page       | `/`                      |

## แพ็กเกจ NPM

| แพ็กเกจ             | บทบาท                                                                                       |
| ------------------- | ------------------------------------------------------------------------------------------- |
| `ruvyxa`            | CLI launcher + runtime bridge                                                               |
| `create-ruvyxa`     | Project scaffold                                                                            |
| `@ruvyxa/core`      | Core runtime utilities                                                                      |
| `@ruvyxa/react`     | React components (Image, SEO, error boundaries, hydration, loaders)                         |
| `@ruvyxa/database`  | Typed CRUD/transaction facade รองรับ Prisma, DynamoDB, และ custom adapter                   |
| `@ruvyxa/auth`      | Sessions, credentials, OAuth PKCE, magic-link, และ WebAuthn                                 |
| `@ruvyxa/realtime`  | WebSocket transport ที่ขับเคลื่อนด้วย action และ browser subscriptions                      |
| `@ruvyxa/adapter-*` | Platform adapters (bun, cloudflare, netlify, node, static, vercel)                          |
| `@ruvyxa/cli-*`     | Native binaries ตามแพลตฟอร์ม (darwin-arm64, linux-arm64, linux-x64, win32-arm64, win32-x64) |

## เอกสารสถาปัตยกรรมเพิ่มเติม

- [การค้นพบและตรวจสอบเส้นทาง](graph.md) — ภายใน `ruvyxa_graph`
- [ไปป์ไลน์การคอมไพล์](bundler.md) — resolver, compiler, linker, minifier ของ `ruvyxa_bundler`
- [Dev Server](dev-server.md) — Axum server, router, render cache, HMR, styles ของ
  `ruvyxa_dev_server`
- [CLI และไปป์ไลน์ build](cli.md) — คำสั่ง, config, build orchestration ของ `ruvyxa_cli`
- [Middleware](middleware.md) — Tower stack ในตัวและ plugin bridge
- [ปลั๊กอิน](plugins.md) — unified setup registry และ lifecycle
- [แพ็กเกจทางการ Data/Auth/Realtime](official-plugins.md) — state ownership, security, flows
  และความเข้ากันได้กับ deployment
- [Worker Pool](worker-pool.md) — โปรโตคอล Node/Bun worker pool, streaming, การกู้คืนเมื่อล้มเหลว
- [รหัส Diagnostic](diagnostics.md) — สารบัญ RUV#### error
- [โมเดลการทำงานพร้อมกัน](concurrency.md) — locks, parallelism, ลักษณะประสิทธิภาพ
- [โปรโตคอล Wire](protocols.md) — NDJSON, WebSocket HMR, และ payloads ของ Fetch
- [โมเดลความปลอดภัย](security.md) — env isolation, rate limiting, และขอบเขตของปลั๊กอิน
