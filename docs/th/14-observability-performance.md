# Observability และ performance

## Observability

ใช้ first-party plugin `observability()` เพื่อเพิ่ม request identifier, W3C `traceparent`,
`Server-Timing` และ structured record ต่อ response request-id header ปริยายคือ `x-request-id`; trace
context, server timing และ logging เปิดโดยปริยาย scope ได้กับ exact/trailing-star route และส่ง
custom logger ได้

```ts
import { config } from 'ruvyxa/config'
import { observability } from 'ruvyxa/plugins'

export default config({
  plugins: [
    observability({ routes: ['/api/*'], logger: (entry) => console.info(JSON.stringify(entry)) }),
  ],
})
```

record มี `requestId`, `traceparent`, `method`, `pathname`, `status` และ `durationMs` logger
ที่ล้มเหลวถูก isolate จึงไม่ทำให้ response ที่ปกติกลายเป็น HTTP failure ให้มองว่านี่คือฐานสำหรับ
telemetry sink ของคุณ ไม่ใช่ metrics/tracing backend ที่สมบูรณ์ `ruvyxa analyze --html` ให้ local
build/route analysis page; `trace` ตรวจ route manifest entry

## Performance control

- เลือก route strategy อย่างตั้งใจ: SSR สำหรับ HTML สดทุก request; SSG สำหรับ build output คงที่;
  ISR สำหรับ freshness ตามเวลา; CSR สำหรับ UI ใน browser; PPR สำหรับ static shell กับ dynamic
  section ที่ stream
- ใช้ `cache(key).ttl(...).swr(...)` สำหรับ data reuse ใน process ที่มีขอบเขต และ invalidate หลัง
  write มันไม่มี cross-process coherence
- เลือก `build.split: 'route'` เมื่ออยากได้ route-level code splitting; วัดก่อนบังคับ `single` หรือ
  `manual`
- build control มี `minify`, `treeShake`, `map`, `workers`, `warm` และ `prerenderCache` image
  control มี quality, lossless mode, variant, worker count และ on-demand transform
- worker runtime มี request coalescing และ operational environment control เริ่มจาก default แล้วใช้
  load test และข้อมูล memory/latency ก่อนเปลี่ยน pool size, concurrency, timeout หรือ memory limit

## ข้อควรระวังเรื่อง cache และ concurrency

core cache ป้องกัน growth ไม่จำกัดที่ 1024 entry และคืน stale value ได้ขณะที่มี background refresh
หนึ่งงาน stale producer error จะเก็บ stale data เมื่อมี; cold failure ยัง throw plugin middleware
worker ไม่ share module state realtime reconnect เป็น client-side และ serverless adapter ไม่ host
native WebSocket realtime ข้อจำกัดเหล่านี้สำคัญเมื่อ scale เกิน process เดียว

**ก่อนหน้า:** [Security](13-security.md) · **ถัดไป:**
[Deploy, run และ operate ใน production](15-deploy-run-and-operate.md)
