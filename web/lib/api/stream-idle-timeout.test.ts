import { describe, expect, it, vi } from "vitest";
import {
  DEFAULT_STREAM_IDLE_MS,
  iterateWithIdleTimeout,
  readChunkWithIdleTimeout,
} from "./stream-idle-timeout";
import { GatewayTimeoutError } from "./types";

/**
 * Build a fake reader whose `read()` never resolves — the wedged-socket
 * failure mode: the gateway accepted the request, returned 200, then went
 * silent forever. The idle timer is what must win.
 */
function neverResolvingReader<T>(): ReadableStreamDefaultReader<T> {
  return {
    read: () => new Promise<ReadableStreamReadResult<T>>(() => {}),
    cancel: () => Promise.resolve(),
    releaseLock: () => {},
    closed: new Promise<undefined>(() => {}),
  } as unknown as ReadableStreamDefaultReader<T>;
}

/** A reader that yields the given chunks then closes — the healthy path. */
function chunkReader<T>(chunks: T[]): ReadableStreamDefaultReader<T> {
  let i = 0;
  return {
    read: () =>
      Promise.resolve(
        i < chunks.length
          ? { value: chunks[i++], done: false }
          : { value: undefined, done: true },
      ),
    cancel: () => Promise.resolve(),
    releaseLock: () => {},
    closed: Promise.resolve(undefined),
  } as unknown as ReadableStreamDefaultReader<T>;
}

describe("readChunkWithIdleTimeout", () => {
  it("rejects with GatewayTimeoutError when no chunk arrives within idleMs", async () => {
    vi.useFakeTimers();
    try {
      const reader = neverResolvingReader<Uint8Array>();
      const promise = readChunkWithIdleTimeout(reader, { idleMs: 45_000 });
      // Surface the rejection without an unhandled-rejection warning.
      const assertion = expect(promise).rejects.toBeInstanceOf(GatewayTimeoutError);
      await vi.advanceTimersByTimeAsync(45_000);
      await assertion;
    } finally {
      vi.useRealTimers();
    }
  });

  it("carries the idle budget + path on the thrown timeout error", async () => {
    vi.useFakeTimers();
    try {
      const reader = neverResolvingReader<Uint8Array>();
      const promise = readChunkWithIdleTimeout(reader, {
        idleMs: 1_000,
        path: "/api/v1/genesis",
      });
      const caught: Promise<unknown> = promise.catch((e) => e);
      await vi.advanceTimersByTimeAsync(1_000);
      const err = await caught;
      expect(err).toBeInstanceOf(GatewayTimeoutError);
      expect((err as GatewayTimeoutError).timeoutMs).toBe(1_000);
      expect((err as GatewayTimeoutError).path).toBe("/api/v1/genesis");
    } finally {
      vi.useRealTimers();
    }
  });

  it("aborts the supplied controller on timeout so the socket is freed", async () => {
    vi.useFakeTimers();
    try {
      const controller = new AbortController();
      const reader = neverResolvingReader<Uint8Array>();
      const promise = readChunkWithIdleTimeout(reader, {
        idleMs: 2_000,
        controller,
      });
      const settled = promise.catch(() => {});
      expect(controller.signal.aborted).toBe(false);
      await vi.advanceTimersByTimeAsync(2_000);
      await settled;
      expect(controller.signal.aborted).toBe(true);
    } finally {
      vi.useRealTimers();
    }
  });

  it("resolves with the chunk when one arrives before the deadline (healthy path)", async () => {
    const reader = chunkReader<string>(["hello"]);
    const result = await readChunkWithIdleTimeout(reader, { idleMs: 45_000 });
    expect(result.done).toBe(false);
    expect(result.value).toBe("hello");
  });

  it("treats idleMs <= 0 as unbounded (returns the read directly)", async () => {
    const reader = chunkReader<string>(["x"]);
    const result = await readChunkWithIdleTimeout(reader, { idleMs: 0 });
    expect(result.value).toBe("x");
  });
});

describe("iterateWithIdleTimeout", () => {
  it("yields every chunk then stops at stream end (normal stream)", async () => {
    const reader = chunkReader<string>(["a", "b", "c"]);
    const seen: string[] = [];
    for await (const chunk of iterateWithIdleTimeout(reader, { idleMs: 45_000 })) {
      seen.push(chunk);
    }
    expect(seen).toEqual(["a", "b", "c"]);
  });

  it("throws GatewayTimeoutError mid-iteration when the stream stalls", async () => {
    vi.useFakeTimers();
    try {
      // First chunk resolves; the second read never does → idle timeout.
      let call = 0;
      const reader = {
        read: () =>
          call++ === 0
            ? Promise.resolve({ value: "first", done: false })
            : new Promise<ReadableStreamReadResult<string>>(() => {}),
        cancel: () => Promise.resolve(),
        releaseLock: () => {},
        closed: new Promise<undefined>(() => {}),
      } as unknown as ReadableStreamDefaultReader<string>;

      const run = (async () => {
        const seen: string[] = [];
        for await (const chunk of iterateWithIdleTimeout(reader, {
          idleMs: 5_000,
        })) {
          seen.push(chunk);
        }
        return seen;
      })();
      const caught = run.catch((e) => e as Error);
      // Let the first (already-resolved) read flow through, then stall.
      await vi.advanceTimersByTimeAsync(5_000);
      const err = await caught;
      expect(err).toBeInstanceOf(GatewayTimeoutError);
    } finally {
      vi.useRealTimers();
    }
  });

  it("exports a sane default idle budget", () => {
    expect(DEFAULT_STREAM_IDLE_MS).toBeGreaterThanOrEqual(20_000);
  });
});
