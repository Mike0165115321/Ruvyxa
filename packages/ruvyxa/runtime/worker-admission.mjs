/**
 * Bounded FIFO admission control for the JavaScript render worker.
 *
 * This module deliberately knows nothing about NDJSON, rendering, or process
 * shutdown. `worker-pool.mjs` owns those concerns and uses this class to keep
 * admission state transitions small, synchronous, and directly testable.
 */

export class WorkerAdmissionController {
  #activeRequests = 0
  #closed = false
  #queue = []
  #rejectedRequests = 0

  /**
   * @param {{ maxConcurrentRequests: number, maxQueuedRequests: number }} options
   */
  constructor({ maxConcurrentRequests, maxQueuedRequests }) {
    assertPositiveInteger('maxConcurrentRequests', maxConcurrentRequests)
    assertPositiveInteger('maxQueuedRequests', maxQueuedRequests)
    this.maxConcurrentRequests = maxConcurrentRequests
    this.maxQueuedRequests = maxQueuedRequests
  }

  /** Number of requests that currently own an execution slot. */
  get activeRequests() {
    return this.#activeRequests
  }

  /**
   * Acquire an execution slot or join the bounded FIFO wait queue.
   *
   * `false` means the queue is full or the controller is closed. Closing is
   * not counted as overload because it is an intentional lifecycle event.
   *
   * @returns {true | false | Promise<boolean>}
   */
  acquire() {
    if (this.#closed) return false
    if (this.#activeRequests < this.maxConcurrentRequests) {
      this.#activeRequests++
      return true
    }
    if (this.#queue.length >= this.maxQueuedRequests) {
      this.#rejectedRequests++
      return false
    }
    return new Promise((resolve) => this.#queue.push(resolve))
  }

  /** Release one owned slot and admit the oldest waiter, if any. */
  release() {
    if (this.#activeRequests === 0) {
      throw new Error('Worker admission release called without an active request')
    }
    const next = this.#queue.shift()
    if (next) {
      // Ownership transfers directly; the active count stays unchanged.
      next(true)
      return
    }
    this.#activeRequests--
  }

  /**
   * Stop future admission and settle every parked waiter as not admitted.
   * Active owners keep their slots until they finish and call `release()`.
   */
  close() {
    if (this.#closed) return
    this.#closed = true
    const queued = this.#queue.splice(0)
    for (const settle of queued) settle(false)
  }

  /** Return an immutable operational snapshot for health checks and metrics. */
  snapshot() {
    return Object.freeze({
      activeRequests: this.#activeRequests,
      queuedRequests: this.#queue.length,
      maxConcurrentRequests: this.maxConcurrentRequests,
      maxQueuedRequests: this.maxQueuedRequests,
      rejectedRequests: this.#rejectedRequests,
      admissionClosed: this.#closed,
    })
  }
}

function assertPositiveInteger(name, value) {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new TypeError(`${name} must be a positive safe integer`)
  }
}
