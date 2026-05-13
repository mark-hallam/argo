import { Check, AlertTriangle, XCircle } from "lucide-react";
import { StatusPill } from "@/components/ui/StatusPill";
import type { Verdict } from "@/lib/api/replay";

/**
 * Three-tone verdict pill — the at-a-glance summary chip for the
 * replay sandbox surface. Tone mapping is locked by the brief:
 *  - reproducible          → success
 *  - drifted-aligned       → warning
 *  - drifted-misaligned    → destructive (rendered with the "error" tone)
 *
 * Pure presentation; consumers pass a `Verdict` value and the pill
 * picks tone + icon + label.
 */

export interface VerdictPillProps {
  verdict: Verdict;
  size?: "sm" | "md";
}

const TONE: Record<Verdict, "success" | "warning" | "error"> = {
  reproducible: "success",
  "drifted-aligned": "warning",
  "drifted-misaligned": "error",
};

const LABEL: Record<Verdict, string> = {
  reproducible: "Reproducible",
  "drifted-aligned": "Drifted-aligned",
  "drifted-misaligned": "Drifted-misaligned",
};

function VerdictIcon({ verdict }: { verdict: Verdict }) {
  if (verdict === "reproducible") return <Check aria-hidden className="size-3.5" />;
  if (verdict === "drifted-aligned") return <AlertTriangle aria-hidden className="size-3.5" />;
  return <XCircle aria-hidden className="size-3.5" />;
}

export function VerdictPill({ verdict, size = "md" }: VerdictPillProps) {
  return (
    <StatusPill
      tone={TONE[verdict]}
      size={size}
      dot={false}
      data-testid={`replay-verdict-pill-${verdict}`}
      data-verdict={verdict}
    >
      <VerdictIcon verdict={verdict} />
      <span>{LABEL[verdict]}</span>
    </StatusPill>
  );
}
