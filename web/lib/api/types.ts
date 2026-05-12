/**
 * Shared API types for the gateway client.
 *
 * The Rust gateway returns errors as:
 *   { "error": { "code": "...", "message": "..." } }
 * `ApiError` matches that envelope. On 5xx or parse failure the body
 * is captured raw and the code is left undefined.
 */

export class ApiError extends Error {
  status: number;
  code?: string;
  body?: unknown;

  constructor(status: number, message: string, code?: string, body?: unknown) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.body = body;
  }
}

export interface ApiErrorEnvelope {
  error: {
    code: string;
    message: string;
  };
}

export interface PaginatedResponse<T> {
  items: T[];
  next_cursor?: string | null;
  total?: number;
}

/** Some endpoints wrap their successful payloads in `{ data: T }`. */
export interface ApiEnvelope<T> {
  data: T;
}

/**
 * Shape of the `payload` field on the `/inbox/ws` `approval.created`
 * frame after `mission-gateway-ws-emit-full-approval-row`.
 *
 * When `partial` is omitted the payload is shape-compatible with the
 * `ApprovalRowJson` returned by `GET /api/v1/approvals` — the SPA can
 * map it through the same `rowToItem` adapter and surgically insert
 * into the TanStack cache. When the gateway can't enrich (e.g.
 * worker-deleted race) the payload still carries `approval_id` plus
 * the bare `WebInboxRow` fields with `partial: true`, so the consumer
 * falls back to `invalidateQueries`.
 *
 * Subscribers that don't know about new fields ignore them; the
 * legacy minimum (`approval_id`, `risk_level`, `enqueued_at`,
 * `expiry`) is always present.
 */
export interface WireApprovalCreatedPayload {
  /** Always present. */
  approval_id: string;
  /** Set to `true` on the partial-fallback shape (no enrichment). */
  partial?: boolean;
  /** Mirrors `approval_id` on the full shape; absent on partial. */
  id?: string;
  /** Worker reference; absent on partial. */
  worker?: {
    id: string;
    name: string;
    avatar_url?: string | null;
  };
  /** "wants to <verb>". */
  verb?: string;
  /** Approval detail body. */
  detail?: string;
  /** Consequence shown next to the approve button. */
  consequence_if_yes?: string;
  /** Consequence shown next to the refuse button. */
  consequence_if_no?: string;
  /** Free-form reason / context. */
  reason?: string | null;
  /** Source channel id (e.g. "webinbox", "slack"). */
  channel?: string;
  /** Manifest-stamped risk level. */
  risk_level?: "low" | "medium" | "high";
  /** ISO timestamp the gateway received the approval. */
  received_at?: string;
  /** ISO expiry timestamp. */
  expires_at?: string;
  /** Legacy bare-row passthrough for partial fallbacks. */
  enqueued_at?: string;
  /** Legacy bare-row passthrough for partial fallbacks. */
  expiry?: string;
  /** Legacy bare-row passthrough for partial fallbacks. */
  prompt_body?: string;
}
