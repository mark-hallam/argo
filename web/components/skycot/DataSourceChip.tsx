import { StatusPill } from "@/components/ui/StatusPill";
import type { DataSource } from "@/lib/api/approvals";
import { isDataSourceChipVisible } from "@/lib/skycot/demo-mode";

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
 * Dev-only diagnostic pill surfacing the data-source the approvals
 * inbox is reading (`live`, `mock-offline`, `mock-auth-bypass`). Hidden
 * by default in every build — production, dev, demo-mode, demo-live —
 * because the chip leaked onto camera during the v1.0 demo MP4
 * recording, reading as "broken software" to an investor when the
 * gateway 404'd a not-yet-wired endpoint and the page fell through to
 * mock data. Set `NEXT_PUBLIC_SKYCOT_SHOW_DATA_SOURCE_CHIP=1` to opt
 * back in for developer use; the underlying value stays inspectable
 * via the `data-source` attribute on the page-header wrapper.
 */
export function DataSourceChip({ source }: { source: DataSource | undefined }) {
  if (!isDataSourceChipVisible()) return null;
  if (!source) return null;
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
