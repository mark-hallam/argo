"use client";

import { Fragment, useState, type ReactNode } from "react";
import { useViewerPolicy } from "@/lib/hooks/use-viewer-policy";
import { ALL_SENTINELS } from "@/lib/redaction/redact";
import { APPROVER_SENTINEL, COUNTERPARTY_SENTINEL } from "@/lib/redaction/audit-rules";
import { SENTINEL, type RedactionCategory } from "@/lib/redaction/patterns";
import { ElevatedViewRequestDialog } from "./ElevatedViewRequestDialog";
import type { RbacPermission, ViewerPolicy } from "@/lib/redaction/types";

/**
 * Audit-surface text wrapper. Splits the input on any redaction
 * sentinel and renders the sentinels as click-to-request buttons
 * — unlike E1-e's `<RedactedText>`, which only shows a tooltip.
 *
 * The component pre-redacts when given `policy` (e.g. tests / static
 * surfaces); when the consumer has already redacted upstream (which
 * is the common case after `redactAuditRow`/`redactAuditDetail`), the
 * input may already contain sentinels — the wrapper just splits on
 * them and wires the click handlers.
 */

const SENTINEL_SPLIT_RE = new RegExp(
  `(${ALL_SENTINELS.map((s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})`,
  "g",
);

const CATEGORY_BY_SENTINEL: Record<string, RbacPermission> = {
  [SENTINEL.email]: "unredacted-email",
  [SENTINEL.phone]: "unredacted-phone",
  [SENTINEL.card]: "unredacted-card",
  [SENTINEL.iban]: "unredacted-iban",
  [SENTINEL.ird]: "unredacted-ird",
  [APPROVER_SENTINEL]: "unredacted-approver",
  [COUNTERPARTY_SENTINEL]: "unredacted-counterparty",
};

export interface AuditRedactedTextProps {
  children: string;
  /** Surface id forwarded to the dialog audit row. */
  surface?: string;
  /** Override policy (tests / SSR). When omitted, `useViewerPolicy` resolves it. */
  policy?: ViewerPolicy;
  className?: string;
}

export function AuditRedactedText(props: AuditRedactedTextProps) {
  if (props.policy) return <StaticAuditRedactedText {...props} policy={props.policy} />;
  return <DynamicAuditRedactedText {...props} />;
}

function DynamicAuditRedactedText(props: Omit<AuditRedactedTextProps, "policy">) {
  const policy = useViewerPolicy();
  return <StaticAuditRedactedText {...props} policy={policy} />;
}

function StaticAuditRedactedText({
  children,
  surface = "audit",
  className,
}: AuditRedactedTextProps & { policy: ViewerPolicy }) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [requestedCategory, setRequestedCategory] = useState<RbacPermission | null>(null);

  const parts = children.split(SENTINEL_SPLIT_RE);

  function onSentinelClick(perm: RbacPermission) {
    setRequestedCategory(perm);
    setDialogOpen(true);
  }

  return (
    <>
      <span className={className} data-testid="audit-redacted-text">
        {parts.map((part, idx) => renderPart(part, idx, onSentinelClick))}
      </span>
      <ElevatedViewRequestDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        initialCategories={requestedCategory ? [requestedCategory] : []}
        surface={surface}
      />
    </>
  );
}

function renderPart(
  part: string,
  idx: number,
  onClick: (perm: RbacPermission) => void,
): ReactNode {
  const perm = CATEGORY_BY_SENTINEL[part];
  if (perm) {
    return (
      <button
        key={idx}
        type="button"
        data-redacted={categoryShort(perm)}
        data-testid={`audit-sentinel-${perm}`}
        onClick={() => onClick(perm)}
        title="Click to request elevated view"
        className="rounded-sm bg-[var(--muted)] px-1 py-[1px] font-mono text-[0.85em] text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--accent-foreground)]"
      >
        {part}
      </button>
    );
  }
  return <Fragment key={idx}>{part}</Fragment>;
}

function categoryShort(perm: RbacPermission): RedactionCategory | string {
  if (perm.startsWith("unredacted-")) return perm.slice("unredacted-".length);
  return perm;
}
