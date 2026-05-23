"use client";
import "./page.css";

import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useCallback, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { executeWorkflow } from "@/lib/api/workflow-execute";
import {
  ArrowLeft,
  Code2,
  GitBranch,
  Globe,
  Play,
  Pencil,
  CheckCircle2,
  MessageSquare,
  ShieldCheck,
  Sparkles,
  Database,
} from "lucide-react";
import { WorkflowViewerSkeleton } from "@/components/skycot/workflows/WorkflowViewerSkeleton";
import { WorkflowViewerErrorState } from "@/components/skycot/workflows/WorkflowEdgeStates";
import { WorkflowProposeModal } from "@/components/skycot/WorkflowProposeModal";
import { WorkflowVersionsPanel } from "@/components/skycot/workflows/WorkflowVersionsPanel";
import { useWorkflow } from "@/lib/hooks/use-workflows";
import {
  fetchWorkflowRuns,
  type FetchWorkflowRunsResult,
} from "@/lib/api/workflow-runs";
import type { WorkflowStepKind } from "@/lib/sample-workflows";
import { WorkflowRunStats, WorkflowRunsTable } from "./run-history";
import {
  WorkflowRunsFilterBar,
  DEFAULT_FILTER_STATE,
  type RunsFilterState,
  type RunsRangePreset,
  type RunsPageSize,
  PAGE_SIZES,
} from "./runs-filter-bar";
import type { WorkflowRunStatus } from "@/lib/api/workflow-runs";
import { WORKFLOW_RUN_STATUSES } from "@/lib/api/workflow-runs";

/**
 * Visual workflow detail viewer — `/workflows/<v>/<name>`.
 *
 * Lives off `useWorkflow(verticalId, name)` which hits
 * `GET /api/v1/verticals/<v>/workflows/<n>` and decodes the
 * `definition_toml` payload into the visual-step shape. When the
 * gateway 404s on the head row we fall through to the local seed in
 * `lib/sample-workflows`. Edge states (loading, error, 0 steps,
 * unknown workflow) all surface so the demo recording never crashes.
 */

const STEP_KIND_META: Record<
  WorkflowStepKind,
  { icon: React.ReactNode; color: string; label: string }
> = {
  trigger: { icon: <Play size={14} />, color: "#10b981", label: "Trigger" },
  skill_invoke: { icon: <Sparkles size={14} />, color: "#8b5cf6", label: "Skill" },
  approval: { icon: <ShieldCheck size={14} />, color: "#f59e0b", label: "Approval gate" },
  tool_call: { icon: <Database size={14} />, color: "#0891b2", label: "Tool call" },
  notify: { icon: <MessageSquare size={14} />, color: "#ec4899", label: "Notify" },
  decision: { icon: <GitBranch size={14} />, color: "#6366f1", label: "Decision" },
  data: { icon: <Database size={14} />, color: "#52525b", label: "Data" },
  http_request: { icon: <Globe size={14} />, color: "#0891b2", label: "HTTP request" },
  code: { icon: <Code2 size={14} />, color: "#3b82f6", label: "Code" },
};

export default function WorkflowDetailPage({
  params,
}: {
  params: Promise<{ verticalId: string; name: string }>;
}) {
  return (
    <Suspense fallback={<WorkflowViewerSkeleton />}>
      <ViewerInner params={params} />
    </Suspense>
  );
}

function ViewerInner({
  params,
}: {
  params: Promise<{ verticalId: string; name: string }>;
}) {
  const { verticalId, name } = use(params);
  const router = useRouter();
  const pathname = usePathname() ?? `/workflows/${verticalId}/${name}`;
  const searchParams = useSearchParams();
  const [editChatOpen, setEditChatOpen] = useState(false);
  const [running, setRunning] = useState(false);
  const stateFlag = searchParams?.get("state") ?? null;

  const { data, isLoading, isError } = useWorkflow(verticalId, name);
  const workflow = data?.workflow ?? null;
  const source = data?.source ?? null;
  const isLive = source === "gateway";

  const filterState = parseFilterStateFromUrl(searchParams);
  const { sinceTs, untilTs } = resolveDateRange(filterState);

  const runsQuery = useQuery<FetchWorkflowRunsResult>({
    queryKey: [
      "workflow-runs",
      verticalId,
      name,
      filterState.range,
      filterState.customSince,
      filterState.customUntil,
      filterState.statuses.join(","),
      filterState.page,
      filterState.pageSize,
    ],
    queryFn: () =>
      fetchWorkflowRuns({
        verticalId,
        name,
        page: filterState.page,
        pageSize: filterState.pageSize,
        statuses: filterState.statuses,
        sinceTs,
        untilTs,
      }),
    enabled: !!verticalId && !!name,
    staleTime: 30_000,
  });
  const runsSummary = runsQuery.data?.summary ?? null;
  const totalFiltered = runsSummary?.total_filtered ?? runsSummary?.runs.length ?? 0;
  const visibleRunCount = runsSummary?.runs.length ?? 0;
  const filtersActive =
    filterState.statuses.length > 0 ||
    filterState.range !== DEFAULT_FILTER_STATE.range ||
    filterState.page > 1;

  const updateFilterState = useCallback(
    (next: RunsFilterState) => {
      const merged = new URLSearchParams(searchParams?.toString() ?? "");
      writeFilterStateToParams(merged, next);
      const qs = merged.toString();
      router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
    },
    [router, pathname, searchParams],
  );

  const clearFilters = useCallback(() => {
    updateFilterState({ ...DEFAULT_FILTER_STATE });
  }, [updateFilterState]);


  const handleRunWorkflow = useCallback(async () => {
    if (running) return;
    setRunning(true);
    try {
      const report = await executeWorkflow({ verticalId, name });
      toast.success(
        `Workflow run ${report.ok ? "succeeded" : "completed with failures"} · ${report.run_id.slice(0, 8)}`,
      );
      runsQuery.refetch();
    } catch (err) {
      const message = err instanceof Error ? err.message : "unknown error";
      toast.error(`Run failed: ${message}`);
    } finally {
      setRunning(false);
    }
  }, [running, verticalId, name, runsQuery]);

  const resetState = useCallback(() => {
    const next = new URLSearchParams(searchParams?.toString() ?? "");
    next.delete("state");
    const qs = next.toString();
    router.replace(qs ? `${pathname}?${qs}` : pathname);
  }, [router, pathname, searchParams]);

  // URL-flag state simulators preserved from the seed-only version so
  // screenshots + tests can reach each edge branch deterministically.
  if (stateFlag === "loading" || isLoading) {
    return <WorkflowViewerSkeleton />;
  }

  if (stateFlag === "error" || isError) {
    return <WorkflowViewerErrorState onRetry={resetState} />;
  }

  if (!workflow) {
    return (
      <div style={{ padding: 48, textAlign: "center", color: "var(--muted-foreground)" }}>
        Workflow not found. Try one from <Link href="/workflows" style={{ color: "#8b5cf6" }}>the catalog</Link>.
      </div>
    );
  }

  return (
    <>
      <div className="wfd-stage">
        <Link href="/workflows" className="wfd-back">
          <ArrowLeft size={12} />
          ALL WORKFLOWS
        </Link>

        <div className="wfd-card">
          <div className="wfd-meta-row">
            <span className="wfd-vertical">{workflow.vertical}</span>
            <span
              className="wfd-risk"
              style={{
                background:
                  workflow.risk === "approval-required" ? "#f59e0b" : "#14b8a6",
              }}
            >
              {workflow.risk === "approval-required" ? "Approval required" : "Autonomous"}
            </span>
            <span className="wfd-version">version {workflow.version}</span>
            <span
              data-testid="wfd-source-pill"
              data-source={source ?? "unknown"}
              style={{
                marginLeft: "auto",
                padding: "3px 8px",
                borderRadius: 999,
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                background: isLive ? "#dcfce7" : "#fef3c7",
                color: isLive ? "#166534" : "#92400e",
              }}
            >
              {isLive ? "Live" : "Mock"}
            </span>
          </div>
          <h1 className="wfd-name">{workflow.name}</h1>
          <p className="wfd-desc">{workflow.description}</p>
          <div className="wfd-actions">
            <Link
              href={`/workflows/${workflow.vertical}/${workflow.name}/build`}
              className="wfd-btn wfd-btn-primary"
            >
              <Pencil size={13} /> Open builder
            </Link>
            <button
              type="button"
              className="wfd-btn"
              data-testid="wfd-edit-in-chat"
              onClick={() => setEditChatOpen(true)}
            >
              <MessageSquare size={13} /> Edit in chat
            </button>
            <Link
              href={`/verticals/${workflow.vertical}/workflows/${workflow.name}`}
              className="wfd-btn"
            >
              <GitBranch size={13} /> Version history
            </Link>
            <button type="button" className="wfd-btn">
              <Play size={13} /> Test run
            </button>
            <button
              type="button"
              className="wfd-btn wfd-btn-primary"
              data-testid="wfd-run-workflow"
              onClick={handleRunWorkflow}
              disabled={running || !isLive}
              title={
                isLive
                  ? "Execute against the live gateway workflow"
                  : "Available once this workflow is published to the gateway"
              }
            >
              <Play size={13} /> {running ? "Running…" : "Run workflow"}
            </button>
          </div>
        </div>

        <div className="wfd-section-label">
          STEPS · {workflow.steps.length} TOTAL
        </div>

        {workflow.steps.length === 0 ? (
          <div
            data-testid="wfd-no-steps"
            style={{
              padding: "32px 24px",
              border: "1px dashed var(--border, #ece9f5)",
              borderRadius: 12,
              textAlign: "center",
              color: "var(--muted-foreground, #52525b)",
              fontSize: 14,
            }}
          >
            <CheckCircle2
              size={28}
              style={{ display: "block", margin: "0 auto 8px", color: "#14b8a6" }}
            />
            This workflow has no steps yet. Open the builder to add one.
          </div>
        ) : (
          <div className="wfd-timeline">
            {workflow.steps.map((step, idx) => {
              const meta = STEP_KIND_META[step.kind] ?? STEP_KIND_META.tool_call;
              return (
                <div className="wfd-step" key={step.id}>
                  <div className="wfd-step-icon" style={{ background: meta.color }}>
                    {meta.icon}
                  </div>
                  <div className="wfd-step-body">
                    <div className="wfd-step-head">
                      <span
                        className="wfd-step-kind"
                        style={{
                          background: `${meta.color}1a`,
                          color: meta.color,
                        }}
                      >
                        {idx + 1}. {meta.label}
                      </span>
                      <h2 className="wfd-step-name">{step.name}</h2>
                      <button
                        type="button"
                        className="wfd-step-edit"
                        aria-label={`Edit ${step.name}`}
                      >
                        <Pencil size={13} />
                      </button>
                    </div>
                    <p className="wfd-step-desc">{step.description}</p>
                    {step.inputs && step.inputs.length > 0 ? (
                      <div className="wfd-step-meta">
                        <span className="wfd-step-meta-row">
                          <span className="wfd-step-meta-key">In:</span>
                          {step.inputs.join(", ")}
                        </span>
                        {step.outputs && step.outputs.length > 0 ? (
                          <span className="wfd-step-meta-row">
                            <span className="wfd-step-meta-key">Out:</span>
                            {step.outputs.join(", ")}
                          </span>
                        ) : null}
                      </div>
                    ) : step.outputs && step.outputs.length > 0 ? (
                      <div className="wfd-step-meta">
                        <span className="wfd-step-meta-row">
                          <span className="wfd-step-meta-key">Out:</span>
                          {step.outputs.join(", ")}
                        </span>
                      </div>
                    ) : null}
                    {step.approvalGate ? (
                      <div className="wfd-approval-banner">
                        <strong>Approval gate · {step.approvalGate.channel}</strong>
                        <br />
                        {step.approvalGate.reason}
                      </div>
                    ) : null}
                  </div>
                </div>
              );
            })}
          </div>
        )}

        <div className="wfd-section-label">
          RUN HISTORY · {sectionLabelForRange(filterState.range)}
        </div>
        <WorkflowRunStats query={runsQuery} summary={runsSummary} />
        <WorkflowRunsFilterBar
          state={filterState}
          onChange={updateFilterState}
          totalFiltered={totalFiltered}
          visibleCount={visibleRunCount}
          isLoading={runsQuery.isPending}
        />
        <WorkflowRunsTable
          isPending={runsQuery.isPending}
          isError={runsQuery.isError}
          runs={runsSummary?.runs ?? []}
          filtersActive={filtersActive}
          onClearFilters={clearFilters}
        />
        {isLive ? (
          <WorkflowVersionsPanel
            verticalId={verticalId}
            name={name}
            onAfterRollback={() => router.refresh()}
          />
        ) : null}
      </div>
      <WorkflowProposeModal
        open={editChatOpen}
        onOpenChange={setEditChatOpen}
        editingWorkflow={{
          workflowName: workflow.name,
          verticalId: workflow.vertical,
        }}
      />
    </>
  );
}

const SECTION_LABEL_BY_RANGE: Record<RunsRangePreset, string> = {
  "24h": "LAST 24 HOURS",
  "7d": "LAST 7 DAYS",
  "30d": "LAST 30 DAYS",
  custom: "CUSTOM RANGE",
};

function sectionLabelForRange(r: RunsRangePreset): string {
  return SECTION_LABEL_BY_RANGE[r] ?? SECTION_LABEL_BY_RANGE["7d"];
}

const VALID_RANGES: ReadonlySet<RunsRangePreset> = new Set([
  "24h",
  "7d",
  "30d",
  "custom",
]);

function parseFilterStateFromUrl(
  searchParams: ReturnType<typeof useSearchParams>,
): RunsFilterState {
  const rawRange = searchParams?.get("range");
  const range =
    rawRange && VALID_RANGES.has(rawRange as RunsRangePreset)
      ? (rawRange as RunsRangePreset)
      : DEFAULT_FILTER_STATE.range;
  const rawStatuses = (searchParams?.get("status") ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter((s): s is WorkflowRunStatus =>
      WORKFLOW_RUN_STATUSES.includes(s as WorkflowRunStatus),
    );
  const rawPage = Number.parseInt(searchParams?.get("page") ?? "", 10);
  const rawPageSize = Number.parseInt(searchParams?.get("page_size") ?? "", 10);
  const pageSize =
    PAGE_SIZES.includes(rawPageSize as RunsPageSize)
      ? (rawPageSize as RunsPageSize)
      : DEFAULT_FILTER_STATE.pageSize;
  return {
    range,
    customSince: searchParams?.get("since") ?? null,
    customUntil: searchParams?.get("until") ?? null,
    statuses: rawStatuses,
    page: Number.isFinite(rawPage) && rawPage > 0 ? rawPage : 1,
    pageSize,
  };
}

function writeFilterStateToParams(
  params: URLSearchParams,
  state: RunsFilterState,
): void {
  if (state.range === DEFAULT_FILTER_STATE.range) {
    params.delete("range");
  } else {
    params.set("range", state.range);
  }
  if (state.range === "custom") {
    if (state.customSince) params.set("since", state.customSince);
    else params.delete("since");
    if (state.customUntil) params.set("until", state.customUntil);
    else params.delete("until");
  } else {
    params.delete("since");
    params.delete("until");
  }
  if (state.statuses.length > 0) {
    params.set("status", state.statuses.join(","));
  } else {
    params.delete("status");
  }
  if (state.page === 1) params.delete("page");
  else params.set("page", String(state.page));
  if (state.pageSize === DEFAULT_FILTER_STATE.pageSize) {
    params.delete("page_size");
  } else {
    params.set("page_size", String(state.pageSize));
  }
}

const HOURS_24_MS = 24 * 60 * 60 * 1_000;
const DAYS_7_MS = 7 * HOURS_24_MS;
const DAYS_30_MS = 30 * HOURS_24_MS;

function resolveDateRange(state: RunsFilterState): {
  sinceTs: number | undefined;
  untilTs: number | undefined;
} {
  const now = Date.now();
  if (state.range === "custom") {
    return {
      sinceTs: state.customSince ? Date.parse(state.customSince) * 1_000 : undefined,
      untilTs: state.customUntil
        ? (Date.parse(state.customUntil) + HOURS_24_MS) * 1_000
        : undefined,
    };
  }
  const offsetMs =
    state.range === "24h"
      ? HOURS_24_MS
      : state.range === "30d"
        ? DAYS_30_MS
        : DAYS_7_MS;
  return { sinceTs: (now - offsetMs) * 1_000, untilTs: undefined };
}

