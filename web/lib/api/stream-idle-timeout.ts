/**
 * Idle-timeout wrapper for streaming response reads.
 *
 * The `client.ts` request wrapper bounds every *unary* read at 15s
 * (the perpetual-skeleton fix). Streaming reads bypass that chokepoint:
 * they call `response.body.getReader()` directly and loop on
 * `reader.read()`. That loop has NO backstop — if the gateway accepts
 * the POST, returns 200, then never emits a byte (a wedged upstream, a
 * stalled LLM generation), the `await reader.read()` blocks FOREVER. The
 * page that drives it (e.g. `/genesis`, a primary nav item) sits on its
 * progress strip permanently, with no error and no retry path.
 *
 * This helper restores the same structural guarantee for streams: the
 * read can no longer hang indefinitely. It resets a timer on EVERY
 * chunk (an "idle" timeout, not a total-duration cap — a long-but-
 * healthy generation that keeps streaming progress events is never cut
 * off), and if `idleMs` elapses with no new chunk it aborts the
 * underlying controller (freeing the socket) and throws
 * {@link GatewayTimeoutError}. The driving page's existing `.catch`
 * → error state + "Try again" then takes over.
 *
 * Default `idleMs` is 45_000 — comfortably above a ~20s real Genesis
 * generation's inter-event gap, tight enough that a truly wedged stream
 * surfaces an error in under a minute instead of never.
 */

import { GatewayTimeoutError } from "./types";

export interface IdleTimeoutOptions {
  /**
   * Maximum milliseconds to wait for the NEXT chunk before treating the
   * stream as stalled. Resets on every received chunk. Defaults to
   * {@link DEFAULT_STREAM_IDLE_MS}. `<= 0` disables the timeout (the
   * loop waits indefinitely — only useful in tests).
   */
  idleMs?: number;
  /**
   * The controller whose `abort()` frees the underlying socket when the
   * idle timeout fires. Pass the SAME controller whose `.signal` was
   * handed to `fetch`, so a timeout actually tears the request down
   * rather than leaking a wedged socket.
   */
  controller?: AbortController;
  /** Request path, threaded into the thrown error for diagnostics. */
  path?: string;
}

/** Default idle budget for a streaming read, in milliseconds. */
export const DEFAULT_STREAM_IDLE_MS = 45_000;

/**
 * Read the next chunk from `reader`, racing it against an idle timer.
 *
 * Resolves with the `reader.read()` result if a chunk (or stream-end)
 * arrives within `idleMs`. Rejects with {@link GatewayTimeoutError} if
 * the timer wins — and aborts `controller` first so the wedged socket is
 * released. The losing fetch/read promise's eventual rejection is
 * swallowed so it never surfaces as an unhandled rejection after the
 * timeout has already won.
 */
export async function readChunkWithIdleTimeout<T>(
  reader: ReadableStreamDefaultReader<T>,
  { idleMs = DEFAULT_STREAM_IDLE_MS, controller, path }: IdleTimeoutOptions = {},
): Promise<ReadableStreamReadResult<T>> {
  const readPromise = reader.read();
  if (!idleMs || idleMs <= 0) return readPromise;

  let timer: ReturnType<typeof setTimeout> | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => {
      // Free the socket so the gateway connection isn't leaked while the
      // page surfaces its error state.
      controller?.abort();
      reject(new GatewayTimeoutError(idleMs, path));
    }, idleMs);
  });
  // The read promise loses the race on timeout; swallow its eventual
  // (abort) rejection so it doesn't bubble as an unhandled rejection.
  readPromise.catch(() => {});

  try {
    return await Promise.race([readPromise, timeout]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/**
 * Async-generator convenience over {@link readChunkWithIdleTimeout}:
 * yields each chunk from `reader` until the stream closes, enforcing the
 * idle timeout between chunks. Throws {@link GatewayTimeoutError} if any
 * single inter-chunk gap exceeds `idleMs`.
 *
 * Consumers keep their own framing logic (SSE `\n\n` blocks, NDJSON
 * lines): this only governs WHEN a chunk must arrive, not how it's
 * parsed.
 */
export async function* iterateWithIdleTimeout<T>(
  reader: ReadableStreamDefaultReader<T>,
  options: IdleTimeoutOptions = {},
): AsyncGenerator<T, void, unknown> {
  while (true) {
    const { value, done } = await readChunkWithIdleTimeout(reader, options);
    if (done) return;
    // `value` is only `undefined` when `done` is true; the guard above
    // already returned in that case.
    yield value as T;
  }
}
