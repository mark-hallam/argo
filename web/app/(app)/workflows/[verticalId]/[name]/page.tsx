"use client";
import "./page.css";

import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useCallback, useEffect, useState } from "react";
import {
  ArrowLeft,
  GitBranch,
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
import { RollbackConfirmModal } from "@/components/skycot/workflows/RollbackConfirmModal";
import { WorkflowVersionHistory } from "@/components/skycot/workflows/WorkflowVersionHistory";
import { useWorkflow } from "@/lib/hooks/use-workflows";
import {
  listWorkflowVersions,
  rollbackWorkflow,
  type WorkflowVersionItem,
} from "@/lib/api/workflow-rollback";
import type { WorkflowStepKind } from "@/lib/sample-workflows";

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
  const stateFlag = searchParams?.get("state") ?? null;

  const { data, isLoading, isError } = useWorkflow(verticalId, name);
  const workflow = data?.workflow ?? null;
  const source = data?.source ?? null;
  const isLive = source === "gateway";

  const [versions, setVersions] = useState<WorkflowVersionItem[]>([]);
  const [rollbackTarget, setRollbackTarget] = useState<WorkflowVersionItem | null>(null);
  const [rollbackNotice, setRollbackNotice] = useState<string | null>(null);

  useEffect(() => {
    if (!isLive) {
      setVersions([]);
      return;
    }
    let cancelled = false;
    listWorkflowVersions({ verticalId, name })
      .then((items) => {
        if (!cancelled) setVersions(items);
      })
      .catch(() => {
        if (!cancelled) setVersions([]);
      });
    return () => {
      cancelled = true;
    };
  }, [isLive, verticalId, name]);

  const headVersion = versions.find((v) => v.lifecycle_state === "active") ?? null;

  const handleConfirmRollback = useCallback(async () => {
    if (!rollbackTarget) return;
    const res = await rollbackWorkflow({
      verticalId,
      name,
      targetVersionId: rollbackTarget.id,
    });
    setRollbackNotice(
      `Rolled back to v${rollbackTarget.version} (now v${res.version}).`,
    );
    setRollbackTarget(null);
    router.refresh();
  }, [rollbackTarget, verticalId, name, router]);

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

        <div className="wfd-section-label">RUN HISTORY · LAST 7 DAYS</div>
        <div
          style={{
            padding: 20,
            background: "white",
            border: "1px solid var(--border, #ece9f5)",
            borderRadius: 12,
            display: "flex",
            gap: 24,
            flexWrap: "wrap",
            fontFamily: "Inter, sans-serif",
            fontSize: 13,
            color: "var(--muted-foreground, #52525b)",
          }}
        >
          <div>
            <div
              style={{
                fontSize: 28,
                fontWeight: 900,
                color: "var(--foreground, #1e1b4b)",
                lineHeight: 1,
              }}
            >
              82
            </div>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                marginTop: 4,
              }}
            >
              Runs
            </div>
          </div>
          <div>
            <div
              style={{
                fontSize: 28,
                fontWeight: 900,
                color: "#14b8a6",
                lineHeight: 1,
              }}
            >
              94%
            </div>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                marginTop: 4,
              }}
            >
              Success rate
            </div>
          </div>
          <div>
            <div
              style={{
                fontSize: 28,
                fontWeight: 900,
                color: "var(--foreground, #1e1b4b)",
                lineHeight: 1,
              }}
            >
              6.2s
            </div>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                marginTop: 4,
              }}
            >
              Median latency
            </div>
          </div>
          <div>
            <div
              style={{
                fontSize: 28,
                fontWeight: 900,
                color: "var(--foreground, #1e1b4b)",
                lineHeight: 1,
              }}
            >
              $0.012
            </div>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                marginTop: 4,
              }}
            >
              Cost / run
            </div>
          </div>
        </div>
        {isLive && versions.length > 0 ? (
          <>
            <div className="wfd-section-label">VERSION HISTORY</div>
            <WorkflowVersionHistory
              versions={versions}
              headVersionId={headVersion?.id ?? null}
              rollbackNotice={rollbackNotice}
              onRequestRollback={(v) => {
                setRollbackNotice(null);
                setRollbackTarget(v);
              }}
            />
          </>
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
      {rollbackTarget && headVersion ? (
        <RollbackConfirmModal
          open={true}
          onClose={() => setRollbackTarget(null)}
          onConfirm={handleConfirmRollback}
          currentVersion={headVersion.version}
          targetVersion={rollbackTarget.version}
          currentToml={headVersion.definition_toml}
          targetToml={rollbackTarget.definition_toml}
        />
      ) : null}
    </>
  );
}
