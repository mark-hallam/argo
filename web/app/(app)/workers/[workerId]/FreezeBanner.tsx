import { Ban } from "lucide-react";
import type { FreezeState } from "./fetchFreezeState";

/**
 * Red banner rendered above the worker hero whenever a worker is
 * frozen. Visible on every tab of the worker profile because the
 * surrounding layout (server component) fetches freeze state and
 * passes it down.
 */
export function FreezeBanner({ state }: { state: FreezeState }) {
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
