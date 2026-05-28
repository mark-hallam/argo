import { StatusPill } from "@/components/ui/StatusPill";
import type { DataSource } from "@/lib/api/approvals";
import { isDemoLive } from "@/lib/skycot/demo-mode";

const LABEL: Record<DataSource, string> = {
  live: "live",
  "mock-offline": "mock (offline)",
  "mock-auth-bypass": "mock (auth-bypass)",
};

const TONE: Record<DataSource, "success" | "warning" | "info"> = {
  live: "success",
  "mock-offline": "warning",
  "mock-auth-bypass": "info",
};

/**
 * Dev-only badge surfacing whether the approvals inbox is reading
 * the real gateway or one of the two mock-fallback modes. Hidden
 * in production builds — production should only ever be `live` and
 * the chip would be visual noise.
 *
 * The `process.env.NODE_ENV` check is statically evaluable, so the
 * production bundle drops the whole component to `null`.
 *
 * Demo-recording fix (2026-05-20): also suppress the `mock-offline`
 * chip in dev. The chip leaks "this data is fake" onto camera during
 * the v1.0 demo runbook walkthrough, where the gateway sometimes
 * 404s the not-yet-wired root-listing endpoints. Operators recording
 * the demo can still surface the chip via `?debug=1` if they want
 * to verify which path the page is reading.
 *
 * Demo-recording fix (2026-05-28): suppress the whole chip when
 * `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` is set. The recording-day SPA runs
 * with auth-bypass on, so the chip would otherwise render "MOCK
 * (AUTH-BYPASS)" onto the camera. The DemoBanner already carries the
 * "Demo mode" disclosure; the chip is redundant + reads as a dev
 * artefact during the v1.0 MP4 recording.
 */
export function DataSourceChip({ source }: { source: DataSource | undefined }) {
  if (process.env.NODE_ENV === "production") return null;
  if (isDemoLive()) return null;
  if (!source) return null;
  if (source === "mock-offline") return null;
  return (
    <StatusPill
      tone={TONE[source]}
      size="sm"
      data-testid="data-source-chip"
      data-source={source}
    >
      {LABEL[source]}
    </StatusPill>
  );
}
