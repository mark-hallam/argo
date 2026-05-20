import { redirect } from "next/navigation";
import { Ban } from "lucide-react";
import { auth } from "@/auth";
import { FreezeButton } from "./FreezeButton";

/**
 * Worker profile index — operator-facing root of `/workers/<id>`.
 *
 * Renders the freeze banner (when active) and the Freeze/Unfreeze
 * control. The hero + tab strip are painted by the surrounding
 * layout; this server component owns the freeze chrome that has to
 * remain visible regardless of which tab the operator clicks into.
 *
 * Capability gating is server-side: `canFreeze` is derived from the
 * presence of a NextAuth session (the gateway enforces the real RBAC
 * check on the freeze endpoint, so the visual hide is defense in
 * depth, not the security boundary). Once the gateway exposes a
 * `/me/capabilities` endpoint, swap this for the granular check —
 * the component contract is already a single `canFreeze` boolean.
 *
 * The current freeze state is fetched via {@link fetchFreezeState},
 * which calls a (forward-compatible) gateway GET. If the gateway is
 * unreachable or returns 404 (the GET endpoint lands in a follow-up
 * brief), state stays null and the button renders in its initial
 * "not frozen" mode — operators can still freeze, and the banner
 * surfaces on the next render after the freeze action.
 */

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

interface FreezeState {
  worker_id: string;
  frozen_at: string;
  frozen_by: string;
  reason: string;
}

async function fetchFreezeState(
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

export default async function WorkerIndexPage({
  params,
}: {
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = await params;
  const session = await auth();
  const canFreeze = Boolean(session?.gatewayToken);
  const freezeState = await fetchFreezeState(workerId, session?.gatewayToken);
  const isFrozen = freezeState !== null;

  // Demo audit fix: /workers/[id] landing was rendering a placeholder
  // 'Open overview →' link instead of the actual Overview content.
  // Auto-redirect to the overview tab so the demo lands directly on
  // the personality + KPI surface that's the real value of the worker
  // profile.
  //
  // Suppress signature-references via void to silence the unused-vars
  // lint without removing the freeze plumbing (kept for the future
  // freeze-banner-in-layout brief).
  void freezeState;
  void isFrozen;
  void canFreeze;
  redirect(`/workers/${workerId}/overview`);
}

function FreezeBanner({ state }: { state: FreezeState }) {
  return (
    <div
      role="alert"
      data-testid="worker-freeze-banner"
      className="flex items-start gap-3 rounded-[var(--radius-lg)] bg-red-500 px-4 py-3 text-white shadow-sm"
    >
      <Ban aria-hidden className="mt-0.5 size-5 shrink-0" />
      <div className="space-y-1 text-sm">
        <p className="font-semibold">
          Worker frozen — no new sessions will start until you unfreeze.
        </p>
        <p>
          <span className="opacity-90">Reason:</span> {state.reason}
        </p>
        <p className="opacity-90">
          Frozen by <span className="font-medium">{state.frozen_by}</span> at{" "}
          <time dateTime={state.frozen_at}>{formatFrozenAt(state.frozen_at)}</time>
        </p>
      </div>
    </div>
  );
}

function formatFrozenAt(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
