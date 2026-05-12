import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  backoffDelay,
  ManagedWebSocket,
  type WebSocketStatus,
} from "./websocket";

/**
 * Stub WebSocket. Captures `new WebSocket(url)` calls and exposes
 * lifecycle triggers (`open()`, `message()`, `close()`) for the test
 * to drive. Instances are pushed onto `StubWebSocket.instances` in
 * construction order so a test can index the Nth connection attempt.
 */
class StubWebSocket {
  static instances: StubWebSocket[] = [];
  static readonly OPEN = 1;
  static readonly CLOSED = 3;
  static readonly CLOSING = 2;
  static readonly CONNECTING = 0;

  readyState = StubWebSocket.CONNECTING;
  sent: string[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((e: MessageEvent) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(public url: string) {
    StubWebSocket.instances.push(this);
  }

  triggerOpen() { this.readyState = StubWebSocket.OPEN; this.onopen?.(); }
  triggerMessage(data: string) {
    this.onmessage?.({ data } as MessageEvent);
  }
  triggerClose() { this.readyState = StubWebSocket.CLOSED; this.onclose?.(); }
  send(data: string) { this.sent.push(data); }
  close() { this.triggerClose(); }
}

function reset() {
  StubWebSocket.instances = [];
}

/** Drain microtasks so awaited `url()` resolves and the new WS is created. */
async function flush() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("backoffDelay", () => {
  it("grows exponentially, capped at maxMs", () => {
    const noJitter = () => 0.5; // jitter delta = 0
    expect(backoffDelay(0, 30_000, noJitter)).toBe(1000);
    expect(backoffDelay(1, 30_000, noJitter)).toBe(2000);
    expect(backoffDelay(2, 30_000, noJitter)).toBe(4000);
    expect(backoffDelay(3, 30_000, noJitter)).toBe(8000);
    expect(backoffDelay(4, 30_000, noJitter)).toBe(16_000);
    expect(backoffDelay(5, 30_000, noJitter)).toBe(30_000);
    expect(backoffDelay(10, 30_000, noJitter)).toBe(30_000);
  });

  it("applies ±25% jitter around the envelope", () => {
    const minJ = backoffDelay(2, 30_000, () => 0); // -25%
    const maxJ = backoffDelay(2, 30_000, () => 1); // +25%
    expect(minJ).toBe(3000);
    expect(maxJ).toBe(5000);
  });
});

describe("ManagedWebSocket", () => {
  beforeEach(() => { reset(); });
  afterEach(() => { vi.useRealTimers(); reset(); });

  it("opens a connection on start and emits status open + message", async () => {
    const events: unknown[] = [];
    const statuses: WebSocketStatus[] = [];
    const conn = new ManagedWebSocket<{ kind: string }>({
      url: () => "ws://test/inbox/ws",
      onMessage: (e) => events.push(e),
      onStatusChange: (s) => statuses.push(s),
      WebSocketCtor: StubWebSocket as unknown as typeof WebSocket,
    });
    conn.start();
    await flush();
    expect(StubWebSocket.instances).toHaveLength(1);
    expect(statuses).toEqual(["connecting"]);
    StubWebSocket.instances[0].triggerOpen();
    expect(statuses).toEqual(["connecting", "open"]);
    StubWebSocket.instances[0].triggerMessage(JSON.stringify({ kind: "hello" }));
    expect(events).toEqual([{ kind: "hello" }]);
    conn.stop();
    expect(statuses[statuses.length - 1]).toBe("closed");
  });

  it("filters pong frames and does not surface them to onMessage", async () => {
    const events: unknown[] = [];
    const conn = new ManagedWebSocket<unknown>({
      url: () => "ws://test/inbox/ws",
      onMessage: (e) => events.push(e),
      WebSocketCtor: StubWebSocket as unknown as typeof WebSocket,
    });
    conn.start();
    await flush();
    StubWebSocket.instances[0].triggerOpen();
    StubWebSocket.instances[0].triggerMessage(JSON.stringify({ type: "pong" }));
    StubWebSocket.instances[0].triggerMessage(JSON.stringify({ type: "approval.created" }));
    expect(events).toEqual([{ type: "approval.created" }]);
    conn.stop();
  });

  it("reconnects after a drop using exponential backoff", async () => {
    vi.useFakeTimers();
    const conn = new ManagedWebSocket<unknown>({
      url: () => "ws://test/inbox/ws",
      onMessage: () => undefined,
      WebSocketCtor: StubWebSocket as unknown as typeof WebSocket,
      rng: () => 0.5, // zero jitter
    });
    conn.start();
    await flush();
    expect(StubWebSocket.instances).toHaveLength(1);
    StubWebSocket.instances[0].triggerOpen();
    // Drop and verify the next attempt fires after ~1s envelope.
    StubWebSocket.instances[0].triggerClose();
    expect(StubWebSocket.instances).toHaveLength(1);
    vi.advanceTimersByTime(999);
    expect(StubWebSocket.instances).toHaveLength(1);
    vi.advanceTimersByTime(2);
    await flush();
    expect(StubWebSocket.instances).toHaveLength(2);
    conn.stop();
  });

  it("sends a ping every heartbeat interval", async () => {
    vi.useFakeTimers();
    const conn = new ManagedWebSocket<unknown>({
      url: () => "ws://test/inbox/ws",
      onMessage: () => undefined,
      WebSocketCtor: StubWebSocket as unknown as typeof WebSocket,
      heartbeatIntervalMs: 1000,
      heartbeatTimeoutMs: 5000,
    });
    conn.start();
    await flush();
    StubWebSocket.instances[0].triggerOpen();
    vi.advanceTimersByTime(1000);
    expect(StubWebSocket.instances[0].sent).toEqual([
      JSON.stringify({ type: "ping" }),
    ]);
    // Pong cancels the deadline so we can witness the next ping cycle.
    StubWebSocket.instances[0].triggerMessage(JSON.stringify({ type: "pong" }));
    vi.advanceTimersByTime(1000);
    expect(StubWebSocket.instances[0].sent).toHaveLength(2);
    conn.stop();
  });

  it("missing-pong after heartbeat-timeout force-closes the socket", async () => {
    vi.useFakeTimers();
    const conn = new ManagedWebSocket<unknown>({
      url: () => "ws://test/inbox/ws",
      onMessage: () => undefined,
      WebSocketCtor: StubWebSocket as unknown as typeof WebSocket,
      heartbeatIntervalMs: 1000,
      heartbeatTimeoutMs: 200,
      rng: () => 0.5,
    });
    conn.start();
    await flush();
    StubWebSocket.instances[0].triggerOpen();
    vi.advanceTimersByTime(1000); // ping fires
    expect(StubWebSocket.instances[0].sent).toHaveLength(1);
    // No pong; deadline fires → ws.close() → reconnect.
    vi.advanceTimersByTime(200);
    expect(StubWebSocket.instances[0].readyState).toBe(StubWebSocket.CLOSED);
    vi.advanceTimersByTime(1100); // next backoff envelope
    await flush();
    expect(StubWebSocket.instances).toHaveLength(2);
    conn.stop();
  });

  it("falls back to polling after N failed connects in the window", async () => {
    vi.useFakeTimers();
    const statuses: WebSocketStatus[] = [];
    const conn = new ManagedWebSocket<unknown>({
      url: () => "ws://test/inbox/ws",
      onMessage: () => undefined,
      onStatusChange: (s) => statuses.push(s),
      WebSocketCtor: StubWebSocket as unknown as typeof WebSocket,
      failuresBeforeFallback: 3,
      fallbackWindowMs: 60_000,
      retryInPollingMs: 30_000,
      rng: () => 0.5,
    });
    conn.start();
    await flush();
    StubWebSocket.instances[0].triggerClose(); // fail #1 → schedule retry
    vi.advanceTimersByTime(1100);
    await flush();
    expect(StubWebSocket.instances).toHaveLength(2);
    StubWebSocket.instances[1].triggerClose(); // fail #2
    vi.advanceTimersByTime(2100);
    await flush();
    expect(StubWebSocket.instances).toHaveLength(3);
    StubWebSocket.instances[2].triggerClose(); // fail #3 → polling
    expect(statuses[statuses.length - 1]).toBe("polling");
    conn.stop();
  });

  it("polling-state retries WS after retryInPollingMs and recovers on success", async () => {
    vi.useFakeTimers();
    const statuses: WebSocketStatus[] = [];
    const conn = new ManagedWebSocket<unknown>({
      url: () => "ws://test/inbox/ws",
      onMessage: () => undefined,
      onStatusChange: (s) => statuses.push(s),
      WebSocketCtor: StubWebSocket as unknown as typeof WebSocket,
      failuresBeforeFallback: 1, // trip immediately
      fallbackWindowMs: 60_000,
      retryInPollingMs: 5_000,
      rng: () => 0.5,
    });
    conn.start();
    await flush();
    StubWebSocket.instances[0].triggerClose(); // → polling
    expect(statuses[statuses.length - 1]).toBe("polling");
    vi.advanceTimersByTime(5000);
    await flush();
    expect(StubWebSocket.instances).toHaveLength(2);
    StubWebSocket.instances[1].triggerOpen();
    expect(statuses[statuses.length - 1]).toBe("open");
    conn.stop();
  });

  it("stop() closes any live socket and emits closed exactly once", async () => {
    const statuses: WebSocketStatus[] = [];
    const conn = new ManagedWebSocket<unknown>({
      url: () => "ws://test/inbox/ws",
      onMessage: () => undefined,
      onStatusChange: (s) => statuses.push(s),
      WebSocketCtor: StubWebSocket as unknown as typeof WebSocket,
    });
    conn.start();
    await flush();
    StubWebSocket.instances[0].triggerOpen();
    conn.stop();
    conn.stop(); // idempotent
    expect(statuses.filter((s) => s === "closed")).toHaveLength(1);
  });
});
