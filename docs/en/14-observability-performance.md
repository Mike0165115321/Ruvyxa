# Observability and performance

## Observability

Use the first-party `observability()` plugin to add a request identifier, W3C `traceparent`,
`Server-Timing`, and a structured record per response. Default request-id header is `x-request-id`;
trace context, server timing, and logging default to enabled. You can scope it to
exact/trailing-star routes and provide a custom logger.

```ts
import { config } from 'ruvyxa/config'
import { observability } from 'ruvyxa/plugins'

export default config({
  plugins: [
    observability({ routes: ['/api/*'], logger: (entry) => console.info(JSON.stringify(entry)) }),
  ],
})
```

The record has `requestId`, `traceparent`, `method`, `pathname`, `status`, and `durationMs`. A
failed logger is isolated so it cannot turn a valid response into an HTTP failure. Treat this as a
foundation for your telemetry sink, not a complete metrics/tracing backend. In a generated
application, `npm run analyze:html` provides a local build/route analysis page; `npm run trace -- /`
inspects a route manifest entry.

## Performance controls

- Select the route strategy intentionally: SSR for request-fresh HTML; SSG for immutable build
  output; ISR for time-bounded freshness; CSR for browser-only UI; PPR for a static shell with
  streamed dynamic sections.
- Use `cache(key).ttl(...).swr(...)` for bounded process-local data reuse and invalidate after
  writes. It has no cross-process coherence.
- Prefer `build.split: 'route'` when route-level code splitting is desired; measure before forcing
  `single` or `manual`.
- Build controls include `minify`, `treeShake`, `map`, `workers`, `warm`, and `prerenderCache`.
  Image controls include quality, lossless mode, variants, worker count, and on-demand transforms.
- The worker runtime has request coalescing and operational environment controls. Start with
  defaults, then use load tests and memory/latency data before changing pool size, concurrency,
  timeout, or memory limit.

## Cache and concurrency cautions

The core cache prevents unbounded growth at 1024 entries and can serve stale values while one
background refresh runs. A stale producer error keeps stale data when present; a cold failure still
throws. Plugin middleware workers do not share module state. Realtime reconnect behavior is
client-side and a serverless adapter cannot host native WebSocket realtime. These constraints matter
when scaling past one process.

**Previous:** [Security](13-security.md) · **Next:**
[Deploy, run, and operate in production](15-deploy-run-and-operate.md)
