import { StatusPill } from "@/components/ui/StatusPill";
import type { DataSource } from "@/lib/api/approvals";

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
 */
export function DataSourceChip({ source }: { source: DataSource | undefined }) {
  if (process.env.NODE_ENV === "production") return null;
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
