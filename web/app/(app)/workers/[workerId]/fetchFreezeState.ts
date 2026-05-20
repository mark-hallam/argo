import "server-only";

/**
 * Server-side fetcher for a worker's freeze state. Extracted from
 * the now-redirected index page so the layout (which wraps every
 * worker-profile tab) can render the freeze banner regardless of
 * which tab the operator is on.
 *
 * Returns null when the operator is unauthenticated, the gateway is
 * unreachable, the worker isn't frozen, or the response shape is
 * unexpected. Callers treat null as "not frozen".
 */

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

export interface FreezeState {
  worker_id: string;
  frozen_at: string;
  frozen_by: string;
  reason: string;
}

export async function fetchFreezeState(
  workerId: string,
  token: string | undefined,
): Promise<FreezeState | null> {
  if (!token) return null;
  try {
    const res = await fetch(
      `${gatewayBase()}/api/v1/workers/${encodeURIComponent(workerId)}/freeze`,
      {
        method: "GET",
        headers: { Authorization: `Bearer ${token}`, Accept: "application/json" },
        cache: "no-store",
      },
    );
    if (!res.ok) return null;
    const body = (await res.json()) as Partial<FreezeState>;
    if (
      typeof body.worker_id === "string" &&
      typeof body.frozen_at === "string" &&
      typeof body.frozen_by === "string" &&
      typeof body.reason === "string"
    ) {
      return body as FreezeState;
    }
    return null;
  } catch {
    return null;
  }
}
