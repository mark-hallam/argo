"use client";

import { Suspense, useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { ChevronLeft, Loader2 } from "lucide-react";
import { ActionInputsCard } from "@/components/skycot/audit/ActionInputsCard";
import { ActionOutputCard } from "@/components/skycot/audit/ActionOutputCard";
import { ActionSummaryHero } from "@/components/skycot/audit/ActionSummaryHero";
import { AuditActionNotFound } from "@/components/skycot/audit/AuditActionNotFound";
import { DecisionChainCard } from "@/components/skycot/audit/DecisionChainCard";
import { EvidenceBundleExporter } from "@/components/skycot/audit/EvidenceBundleExporter";
import { ProvenanceBlock } from "@/components/skycot/audit/ProvenanceBlock";
import { singleActionScope } from "@/lib/sample-export";
import { Breadcrumb, BreadcrumbItem, BreadcrumbSeparator } from "@/components/ui/Breadcrumb";
import { useAuditDetail } from "@/lib/hooks/use-audit-detail";
import { AuditDetailNotFoundError } from "@/lib/api/audit-detail";
import { useRedactionPolicy } from "@/lib/hooks/use-redaction-policy";
import { policyMasksAnything } from "@/lib/redaction/redact";

/**
 * `/audit/action/<actionId>` — action drill-in surface. CONTRACT PR
 * for the E5-c slice. The page composes 5 cards (summary hero +
 * inputs + decision chain + provenance + output) plus a top-of-page
 * Breadcrumb / Back button.
 *
 * Routing notes:
 *  - Replay button targets `/audit/action/<id>/replay` which E5-d
 *    will implement; until then the route 404s.
 *  - Export button surfaces an E5-e placeholder toast; the dialog
 *    lands in E5-e.
 *  - Back button uses `router.back()` so audit-feed filter state
 *    survives the round-trip. If history is empty (deep-link), the
 *    button degrades to a static link back to `/audit`.
 *  - Action-not-found uses `<AuditActionNotFound>` — an in-shell
 *    404 rather than the Next.js framework not-found.
 */

const TOAST_TTL_MS = 2500;

export default function AuditActionDetailPage() {
  return (
    <Suspense fallback={<DetailFallback />}>
      <AuditActionDetailContent />
    </Suspense>
  );
}

function DetailFallback() {
  return (
    <div
      data-testid="audit-detail-loading"
      className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-12 text-center"
    >
      <Loader2 className="size-5 mx-auto animate-spin text-[var(--muted-foreground)]" />
      <p className="mt-2 text-sm text-[var(--muted-foreground)]">Loading action…</p>
    </div>
  );
}

function AuditActionDetailContent() {
  const params = useParams<{ actionId: string }>();
  const actionId = decodeURIComponent(params?.actionId ?? "");
  const router = useRouter();
  const policy = useRedactionPolicy();
  const detailQuery = useAuditDetail(actionId);
  const [toast, setToast] = useState<string | null>(null);
  const [exporterOpen, setExporterOpen] = useState(false);

  useEffect(() => {
    if (!toast) return;
    const handle = window.setTimeout(() => setToast(null), TOAST_TTL_MS);
    return () => window.clearTimeout(handle);
  }, [toast]);

  const onReplay = useCallback(() => {
    router.push(`/audit/action/${encodeURIComponent(actionId)}/replay`);
  }, [router, actionId]);

  const onExport = useCallback(() => {
    setExporterOpen(true);
  }, []);

  if (detailQuery.isPending) {
    return <DetailFallback />;
  }
  if (detailQuery.isError) {
    const isNotFound = detailQuery.error instanceof AuditDetailNotFoundError;
    if (isNotFound) {
      return <AuditActionNotFound actionId={actionId} />;
    }
    return (
      <DetailErrorState
        message={detailQuery.error?.message ?? "Couldn't load action"}
        onRetry={() => detailQuery.refetch()}
      />
    );
  }

  const detail = detailQuery.data;
  if (!detail) {
    return <AuditActionNotFound actionId={actionId} />;
  }

  const verbSummary = truncate(detail.verb, 64);
  const showPiiBanner = policyMasksAnything(policy);

  return (
    <div
      data-testid="audit-detail-page"
      data-action-id={detail.id}
      className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-6 space-y-4"
    >
      <div className="flex items-center justify-between gap-4">
        <Breadcrumb>
          <BreadcrumbItem href="/audit">Audit</BreadcrumbItem>
          <BreadcrumbSeparator />
          <BreadcrumbItem current>{verbSummary}</BreadcrumbItem>
        </Breadcrumb>

        <BackButton onBack={() => router.back()} />
      </div>

      {showPiiBanner ? (
        <div
          role="status"
          data-testid="audit-detail-pii-banner"
          className="flex flex-wrap items-center justify-between gap-3 rounded-[var(--radius-md)] border border-[var(--warning)]/30 bg-[var(--warning)]/10 px-3 py-2 text-xs text-[var(--foreground)]"
        >
          <span>
            <strong className="font-medium">PII redacted</strong> — your role shows
            masked customer data.
          </span>
          <Link
            href="/account/permissions"
            data-testid="audit-detail-elevate-link"
            className="font-medium text-[var(--primary)] underline-offset-4 hover:underline"
          >
            Request elevated view →
          </Link>
        </div>
      ) : null}

      <ActionSummaryHero
        detail={detail}
        policy={policy}
        onReplay={onReplay}
        onExport={onExport}
      />

      <div className="grid gap-4 lg:grid-cols-2">
        <ActionInputsCard inputs={detail.inputs} policy={policy} />
        <ProvenanceBlock data={detail.provenance} detail={detail} />
      </div>

      <DecisionChainCard steps={detail.decisionChain} policy={policy} />

      <ActionOutputCard output={detail.output} policy={policy} />

      {exporterOpen ? (
        <EvidenceBundleExporter
          open={exporterOpen}
          onOpenChange={setExporterOpen}
          tenantId="tenant_default"
          tenantLabel="Default tenant"
          initialScope={singleActionScope(detail.id, detail.timestamp)}
          mode="single-action"
          onToast={setToast}
        />
      ) : null}

      {toast ? (
        <div
          role="status"
          data-testid="audit-detail-toast"
          className="fixed bottom-6 left-1/2 -translate-x-1/2 z-30 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-4 py-2 text-sm text-[var(--foreground)] shadow-md"
        >
          {toast}
        </div>
      ) : null}
    </div>
  );
}

function BackButton({ onBack }: { onBack: () => void }) {
  return (
    <button
      type="button"
      data-testid="audit-detail-back"
      onClick={onBack}
      className="inline-flex items-center gap-1 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-2.5 py-1 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40"
    >
      <ChevronLeft aria-hidden className="size-4" /> Back
    </button>
  );
}

function DetailErrorState({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div
      data-testid="audit-detail-error"
      className="mx-auto max-w-2xl px-4 sm:px-6 lg:px-8 py-12 text-center space-y-3"
    >
      <p className="text-sm font-medium text-[var(--foreground)]">Couldn&apos;t load action</p>
      <p className="text-xs text-[var(--muted-foreground)]">{message}</p>
      <button
        type="button"
        data-testid="audit-detail-retry"
        onClick={onRetry}
        className="text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
      >
        Try again
      </button>
    </div>
  );
}

function truncate(value: string, max: number): string {
  if (value.length <= max) return value;
  return `${value.slice(0, max - 1)}…`;
}
