import { StatusPill } from "@/components/ui/StatusPill";
import type { LiveStatus } from "@/lib/hooks/use-approvals-live";

const LABEL: Record<LiveStatus, string> = {
  live: "live",
  polling: "polling (offline)",
  connecting: "connecting…",
  offline: "offline",
};

const TONE: Record<LiveStatus, "success" | "warning" | "muted"> = {
  live: "success",
  polling: "warning",
  connecting: "muted",
  offline: "muted",
};

/**
 * Tiny pulsing-dot pill next to the approvals page subtitle that
 * mirrors the `/inbox/ws` connection state. `● live` (success tone)
 * when the WebSocket is connected; `● polling (offline)` (warning
 * tone) when the wrapper has fallen back to HTTP polling; `●
 * connecting…` (muted tone) on initial connect + reconnect attempts.
 *
 * The dot uses the same `bg-current` trick as `StatusPill`'s default
 * dot — Tailwind's animate-pulse provides the subtle breathing
 * animation that distinguishes "live data" from a static status.
 */
export function LiveIndicator({ status }: { status: LiveStatus }) {
  return (
    <StatusPill
      tone={TONE[status]}
      size="sm"
      dot={false}
      data-testid="live-indicator"
      data-status={status}
    >
      <span
        aria-hidden
        className={`inline-block size-1.5 rounded-full bg-current ${status === "live" ? "animate-pulse" : ""}`}
      />
      {LABEL[status]}
    </StatusPill>
  );
}
