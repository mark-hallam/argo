/**
 * Reusable WebSocket client: auto-reconnect, heartbeat, exponential
 * backoff with jitter, polling-fallback. Used by `useApprovalsLive`
 * against the gateway's `/inbox/ws` (J3 structured events) but
 * generic so future surfaces can re-use it. Wire-format parsing is
 * the caller's via the `parse` option.
 *
 * Browser WebSocket can't set custom headers — callers embed the
 * bearer in `?token=<jwt>` (the gateway also accepts the
 * `argo_operator_token` cookie sent automatically by the browser;
 * see `ws_inbox_events::authorize`).
 *
 * @example
 * ```ts
 * const conn = new ManagedWebSocket<MyEvent>({
 *   url: () => `${base}/inbox/ws?token=${token}`,
 *   onMessage: (e) => console.log(e),
 * });
 * conn.start();
 * // ... conn.stop() on unmount
 * ```
 */
export type WebSocketStatus = "connecting" | "open" | "polling" | "closed";

export interface ManagedWebSocketOptions<T> {
  /** URL builder; async so callers can fetch a fresh bearer token. */
  url: () => string | Promise<string>;
  /** Called for each parsed application event (ping/pong filtered out). */
  onMessage: (event: T) => void;
  /** Status-change callback. Fires once per distinct status. */
  onStatusChange?: (status: WebSocketStatus) => void;
  /** Custom parser. Defaults to `JSON.parse` with pong-frame filtering. */
  parse?: (raw: string) => T | null;
  heartbeatIntervalMs?: number;
  heartbeatTimeoutMs?: number;
  maxReconnectDelayMs?: number;
  failuresBeforeFallback?: number;
  fallbackWindowMs?: number;
  retryInPollingMs?: number;
  /** Test seams. */
  WebSocketCtor?: typeof WebSocket;
  now?: () => number;
  rng?: () => number;
}

const DEFAULTS = {
  heartbeatIntervalMs: 30_000,
  heartbeatTimeoutMs: 10_000,
  maxReconnectDelayMs: 30_000,
  failuresBeforeFallback: 3,
  fallbackWindowMs: 60_000,
  retryInPollingMs: 30_000,
} as const;

/**
 * Exponential backoff with ±25% jitter, capped at `maxMs`. Jitter
 * prevents thundering-herd reconnects after a fleet-wide blip.
 */
export function backoffDelay(
  attempt: number,
  maxMs: number,
  rng: () => number = Math.random,
): number {
  const base = Math.min(1000 * 2 ** attempt, maxMs);
  const jitter = (rng() - 0.5) * 0.5 * base;
  return Math.max(0, Math.round(base + jitter));
}

type Timer = ReturnType<typeof setTimeout>;

export class ManagedWebSocket<T> {
  private readonly opts: Required<
    Pick<ManagedWebSocketOptions<T>, keyof typeof DEFAULTS>
  > &
    ManagedWebSocketOptions<T>;
  private readonly now: () => number;
  private readonly rng: () => number;
  private readonly Ctor: typeof WebSocket;
  private ws: WebSocket | null = null;
  private status: WebSocketStatus = "closed";
  private attempt = 0;
  private failures: number[] = [];
  private stopped = false;
  private reconnectTimer: Timer | null = null;
  private heartbeatTimer: Timer | null = null;
  private heartbeatDeadline: Timer | null = null;
  private pollingRetryTimer: Timer | null = null;

  constructor(opts: ManagedWebSocketOptions<T>) {
    this.opts = { ...DEFAULTS, ...opts };
    this.now = opts.now ?? (() => Date.now());
    this.rng = opts.rng ?? Math.random;
    this.Ctor = opts.WebSocketCtor ?? (globalThis.WebSocket as typeof WebSocket);
  }

  /** Open the connection (idempotent if already started). */
  start(): void {
    if (this.stopped) return;
    if (this.ws || this.reconnectTimer || this.pollingRetryTimer) return;
    void this.connect();
  }

  /** Tear down for unmount. Idempotent. */
  stop(): void {
    this.stopped = true;
    this.clearTimers();
    if (this.ws) {
      try { this.ws.close(); } catch { /* ignore */ }
      this.ws = null;
    }
    this.setStatus("closed");
  }

  get currentStatus(): WebSocketStatus { return this.status; }

  private setStatus(next: WebSocketStatus): void {
    if (this.status === next) return;
    this.status = next;
    this.opts.onStatusChange?.(next);
  }

  private async connect(): Promise<void> {
    if (this.stopped) return;
    if (this.status !== "polling") this.setStatus("connecting");
    let url: string;
    try { url = await this.opts.url(); } catch { this.recordFailure(); return; }
    let ws: WebSocket;
    try { ws = new this.Ctor(url); } catch { this.recordFailure(); return; }
    this.ws = ws;
    ws.onopen = () => this.handleOpen();
    ws.onmessage = (e: MessageEvent) => this.handleMessage(e);
    ws.onclose = () => this.handleClose();
    ws.onerror = () => { /* close handler runs after error */ };
  }

  private handleOpen(): void {
    if (this.stopped) return;
    this.attempt = 0;
    this.failures = [];
    if (this.pollingRetryTimer) {
      clearTimeout(this.pollingRetryTimer);
      this.pollingRetryTimer = null;
    }
    this.setStatus("open");
    this.startHeartbeat();
  }

  private handleMessage(e: MessageEvent): void {
    if (this.stopped) return;
    const raw = typeof e.data === "string" ? e.data : "";
    if (!raw) return;
    // Any inbound frame proves the link is live → cancel pong deadline.
    this.bumpHeartbeatDeadline();
    let parsed: T | null = null;
    if (this.opts.parse) {
      parsed = this.opts.parse(raw);
    } else {
      try {
        const v = JSON.parse(raw) as unknown;
        if (typeof v === "object" && v !== null && "type" in v &&
            (v as { type: unknown }).type === "pong") return;
        parsed = v as T;
      } catch { return; }
    }
    if (parsed !== null) this.opts.onMessage(parsed);
  }

  private handleClose(): void {
    if (this.stopped) return;
    this.stopHeartbeat();
    this.ws = null;
    this.recordFailure();
  }

  private recordFailure(): void {
    if (this.stopped) return;
    const t = this.now();
    this.failures = this.failures.filter((ts) => t - ts < this.opts.fallbackWindowMs);
    this.failures.push(t);
    if (this.failures.length >= this.opts.failuresBeforeFallback) {
      this.fallbackToPolling();
      return;
    }
    this.scheduleReconnect();
  }

  private scheduleReconnect(): void {
    if (this.stopped) return;
    const delay = backoffDelay(this.attempt, this.opts.maxReconnectDelayMs, this.rng);
    this.attempt += 1;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      void this.connect();
    }, delay);
  }

  private fallbackToPolling(): void {
    this.setStatus("polling");
    this.failures = [];
    this.attempt = 0;
    if (this.pollingRetryTimer) clearTimeout(this.pollingRetryTimer);
    this.pollingRetryTimer = setTimeout(() => {
      this.pollingRetryTimer = null;
      void this.connect();
    }, this.opts.retryInPollingMs);
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();
    this.heartbeatTimer = setInterval(() => {
      if (!this.ws || this.ws.readyState !== this.Ctor.OPEN) return;
      try { this.ws.send(JSON.stringify({ type: "ping" })); } catch { /* ignore */ }
      this.heartbeatDeadline = setTimeout(() => {
        try { this.ws?.close(); } catch { /* ignore */ }
      }, this.opts.heartbeatTimeoutMs);
    }, this.opts.heartbeatIntervalMs);
  }

  private bumpHeartbeatDeadline(): void {
    if (this.heartbeatDeadline) {
      clearTimeout(this.heartbeatDeadline);
      this.heartbeatDeadline = null;
    }
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) { clearInterval(this.heartbeatTimer); this.heartbeatTimer = null; }
    if (this.heartbeatDeadline) { clearTimeout(this.heartbeatDeadline); this.heartbeatDeadline = null; }
  }

  private clearTimers(): void {
    if (this.reconnectTimer) { clearTimeout(this.reconnectTimer); this.reconnectTimer = null; }
    if (this.pollingRetryTimer) { clearTimeout(this.pollingRetryTimer); this.pollingRetryTimer = null; }
    this.stopHeartbeat();
  }
}
