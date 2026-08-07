/**
 * Runs once per server process, before the first request is served.
 *
 * This is where an OpenTelemetry SDK, an error reporter, or a metrics exporter
 * is installed — anything that has to be in place process-wide rather than
 * per request. It runs in the process that renders: the worker under
 * `ruvyxa dev` and `ruvyxa start`, and the function instance after a deploy.
 */
export async function register(): Promise<void> {
  if (process.env.RUVYXA_DEMO_QUIET === '1') return
  // stderr, not stdout: stdout in this process is the worker's NDJSON
  // response channel, and a line here would corrupt the next response a
  // request is waiting on.
  console.error(`[demo] instrumentation registered on pid ${process.pid}`)
}
