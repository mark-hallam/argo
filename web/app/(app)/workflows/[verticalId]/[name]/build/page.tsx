"use client";
import "./page.css";

import "@xyflow/react/dist/style.css";
import type { Node } from "@xyflow/react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import TriggerConfigDrawer from "@/components/workflows/triggers/TriggerConfigDrawer";
import type { TriggerConfig, TriggerKind } from "@/components/workflows/triggers/types";
import { WorkflowBuilderSkeleton } from "@/components/skycot/workflows/WorkflowBuilderSkeleton";
import {
  WorkflowBuilderEmptyState,
  WorkflowBuilderErrorState,
} from "@/components/skycot/workflows/WorkflowEdgeStates";
import { useWorkflowBuilderState } from "@/lib/hooks/use-workflow-builder-state";
import { getTemplate } from "@/lib/workflow-templates";

import { BuilderCanvas } from "./BuilderCanvas";
import { BuilderInspector } from "./BuilderInspector";
import { BuilderToolbar } from "./BuilderToolbar";
import { stepsToFlow } from "./layout";
import { SEED_WORKFLOWS } from "./seed-workflows";
import type { SkyNodeData, WorkflowSpec } from "./types";

/**
 * Class-leading visual workflow builder — `/workflows/<v>/<name>/build`.
 *
 * Two editing modalities sit side-by-side:
 *
 *   1. Natural-language AI prompt area at the top — describe a change
 *      in plain English ("add an approval step before posting") and
 *      the AI proposes the diff.
 *   2. Full visual canvas powered by React Flow (@xyflow/react), with
 *      draggable nodes, decision branches, loops, approval gates, and
 *      a side inspector for the currently-selected step.
 *
 * V1 seeds nodes from the same demo workflow data as the read-only
 * viewer at /workflows/<v>/<n>. Real editing wires through to the
 * gateway PUT workflow endpoint in a follow-up brief; this page is
 * the canvas + AI-prompt UX that closes the n8n / UIpath / Replit
 * competitive gap.
 *
 * This file is the orchestrator: it loads the workflow spec, owns the
 * `useWorkflowBuilderState` hook, and threads handlers into the
 * extracted toolbar / canvas / inspector components.
 */

export default function WorkflowBuilderPage({
  params,
}: {
  params: Promise<{ verticalId: string; name: string }>;
}) {
  return (
    <Suspense fallback={<WorkflowBuilderSkeleton />}>
      <BuilderInner params={params} />
    </Suspense>
  );
}

function BuilderInner({
  params,
}: {
  params: Promise<{ verticalId: string; name: string }>;
}) {
  const { verticalId, name } = use(params);
  const router = useRouter();
  const pathname = usePathname() ?? `/workflows/${verticalId}/${name}/build`;
  const searchParams = useSearchParams();
  const templateId = searchParams.get("from_template");
  // `?state=loading|error|empty` is the gateway-fetch-state simulator
  // (real fetch wires in via PR #1860 routes follow-up). It exists so
  // tests + dev/screenshot flows can land each branch without standing
  // up a flaky backend. Empty also fires naturally when the live
  // workflow has zero steps.
  const stateFlag = searchParams.get("state");
  const workflow = useMemo<WorkflowSpec | undefined>(() => {
    if (templateId) {
      const tpl = getTemplate(templateId);
      if (tpl) {
        return {
          name: tpl.name,
          vertical: tpl.vertical,
          description: tpl.description,
          version: "draft",
          risk: tpl.risk,
          steps: tpl.steps,
        };
      }
    }
    return SEED_WORKFLOWS[`${verticalId}/${name}`];
  }, [templateId, verticalId, name]);
  const initial = useMemo(
    () => (workflow ? stepsToFlow(workflow.steps) : { nodes: [], edges: [] }),
    [workflow],
  );
  const builder = useWorkflowBuilderState(initial.nodes, initial.edges);

  useEffect(() => {
    if (!builder.dirty) return;
    const handler = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = "";
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [builder.dirty]);

  const resetState = useCallback(() => {
    const next = new URLSearchParams(searchParams.toString());
    next.delete("state");
    const qs = next.toString();
    router.replace(qs ? `${pathname}?${qs}` : pathname);
  }, [router, pathname, searchParams]);

  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [promptText, setPromptText] = useState("");
  const [proposing, setProposing] = useState(false);
  const [triggerConfigOpen, setTriggerConfigOpen] = useState(false);
  // One config per step id. Persisted client-side for v1; wired to the
  // gateway step-config save path in a follow-up brief.
  const [triggerConfigs, setTriggerConfigs] = useState<Record<string, TriggerConfig>>({});
  const workflowId = `${verticalId}/${name}`;

  const onNodeClick = useCallback((_e: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
    setTriggerConfigOpen(false);
  }, []);

  const selectedStep = selectedNode ? (selectedNode.data as SkyNodeData).step : null;
  const selectedIsTrigger = selectedStep?.kind === "trigger";
  const selectedTriggerKind: TriggerKind = selectedStep?.triggerKind ?? "manual";

  const handleProposeChange = useCallback(() => {
    if (!promptText.trim()) return;
    setProposing(true);
    const prompt = promptText;
    // Honest demo stub — wave-6 action-fires audit (2026-05-21) called
    // out the prior `setTimeout` toast for over-claiming success on a
    // canvas that doesn't actually change. The real Propose live-wire
    // lands via brief `web-workflow-builder-ai-prompt-wired` (#1882 +
    // batch-1 sibling). Until then, surface the action without claiming
    // completion of work that didn't happen.
    const dismissId = toast.loading(`Drafting: "${prompt}"`, {
      description: "Live AI workflow editing — wired in the next deploy.",
    });
    setTimeout(() => {
      toast.message("Preview build — draft not persisted", {
        id: dismissId,
        description: "Live AI editing rolls out next. Canvas updates land then.",
      });
      setProposing(false);
      setPromptText("");
    }, 1400);
  }, [promptText]);

  const handleTestRun = useCallback(() => {
    // Honest demo stub — wave-6 action-fires audit flagged the prior
    // "All 5 steps passed" toast as a buyer-catchable lie. Wave-13
    // audit asked to drop the "v1.x" / internal-PR-ref leaks; this
    // copy stays neutral while keeping the not-yet-wired honesty.
    toast.message("Test run — preview build", {
      description: "Real run streams from the gateway in an upcoming release.",
    });
  }, []);

  const handleSaveAndPublish = useCallback(() => {
    // Honest demo stub — wave-6 audit. Wave-13 cleaned up the leaked
    // version identifiers; copy is now neutral.
    toast.message("Saved as draft — preview build", {
      description: "Tenant-wide publish lands in an upcoming release.",
    });
  }, []);

  if (stateFlag === "loading") {
    return <WorkflowBuilderSkeleton />;
  }

  if (stateFlag === "error") {
    return <WorkflowBuilderErrorState onRetry={resetState} />;
  }

  if (!workflow) {
    return (
      <div style={{ padding: 48, textAlign: "center", color: "var(--muted-foreground)" }}>
        Workflow not found. Try one from <Link href="/workflows" style={{ color: "#8b5cf6" }}>the catalog</Link>.
      </div>
    );
  }

  if (workflow.steps.length === 0 || stateFlag === "empty") {
    return (
      <WorkflowBuilderEmptyState
        onAddStep={() => {
          toast.info("Add-step palette opens in the live builder", {
            description: "Connect the gateway to add real steps.",
          });
        }}
      />
    );
  }

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <div className="builder-shell">
        <BuilderToolbar
          workflow={workflow}
          verticalId={verticalId}
          name={name}
          dirty={builder.dirty}
          promptText={promptText}
          proposing={proposing}
          onPromptTextChange={setPromptText}
          onProposeChange={handleProposeChange}
          onTestRun={handleTestRun}
          onSaveAndPublish={handleSaveAndPublish}
        />

        <div className="builder-stage">
          <BuilderCanvas
            nodes={builder.nodes}
            edges={builder.edges}
            onNodesChange={builder.onNodesChange}
            onEdgesChange={builder.onEdgesChange}
            onNodeClick={onNodeClick}
          />
          {selectedStep ? (
            <BuilderInspector
              step={selectedStep}
              isTrigger={!!selectedIsTrigger}
              onClose={() => setSelectedNode(null)}
              onEdit={() => setTriggerConfigOpen(true)}
            />
          ) : null}
          {selectedIsTrigger && triggerConfigOpen && selectedStep ? (
            <TriggerConfigDrawer
              triggerKind={selectedTriggerKind}
              workflowId={workflowId}
              workspaceTimezone="Australia/Melbourne"
              config={triggerConfigs[selectedStep.id] ?? null}
              onChange={(next) =>
                setTriggerConfigs((prev) => ({ ...prev, [selectedStep.id]: next }))
              }
              onClose={() => setTriggerConfigOpen(false)}
              onSave={() => setTriggerConfigOpen(false)}
            />
          ) : null}
        </div>
      </div>
    </>
  );
}
