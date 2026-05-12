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
