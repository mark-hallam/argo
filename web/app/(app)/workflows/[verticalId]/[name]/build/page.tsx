"use client";
import "./page.css";

import "@xyflow/react/dist/style.css";
import {
  ReactFlowProvider,
  useReactFlow,
  type Connection,
  type Edge,
  type IsValidConnection,
  type Node,
} from "@xyflow/react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import { TestRunHistoryPanel } from "@/components/skycot/builder/TestRunHistoryPanel";
import { UnsavedBanner } from "@/components/skycot/builder/UnsavedBanner";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { KeyboardShortcutsModal } from "@/components/skycot/workflows/KeyboardShortcutsModal";
import TriggerConfigDrawer from "@/components/workflows/triggers/TriggerConfigDrawer";
import type { TriggerConfig, TriggerKind } from "@/components/workflows/triggers/types";
import { WorkflowBuilderSkeleton } from "@/components/skycot/workflows/WorkflowBuilderSkeleton";
import { WorkflowContextMenu } from "@/components/skycot/workflows/WorkflowContextMenu";
import {
  WorkflowBuilderEmptyState,
  WorkflowBuilderErrorState,
} from "@/components/skycot/workflows/WorkflowEdgeStates";
import { MobileBuilderGate } from "@/components/skycot/workflows/MobileBuilderGate";
import { useBuilderAnnotations } from "@/lib/hooks/use-builder-annotations";
import { useBuilderKeyboard } from "@/lib/hooks/use-builder-keyboard";
import { useWorkflowBuilderState } from "@/lib/hooks/use-workflow-builder-state";
import { getTemplate } from "@/lib/workflow-templates";

import { BuilderAiProposal } from "./BuilderAiProposal";
import { BuilderCanvas } from "./BuilderCanvas";
import { BuilderInspector } from "./BuilderInspector";
import { BuilderSidebarPalette } from "./BuilderSidebarPalette";
import { BuilderToolbar } from "./BuilderToolbar";
import { PinDataModal } from "./PinDataModal";
import {
  isValidConnectionShape,
  newStepNode,
  STEP_KIND_DEFAULTS,
  validateConnection,
} from "./builder-handlers";
import { useDeleteDuplicateHandlers } from "./builder-handlers/delete-duplicate";
import { useHistoryHandlers } from "./builder-handlers/history";
import { useProposalHandlers } from "./builder-handlers/proposal";
import { useSavePublishHandlers } from "./builder-handlers/save-publish";
import { useTestRunHandlers } from "./builder-handlers/test-run";
import { buildRunOverlayProps, useTestRunHistoryHandlers } from "./builder-handlers/test-run-history";
import { stepsToFlow } from "./flow-layout";
import { RunOverlay } from "./RunOverlay";
import { SEED_WORKFLOWS } from "./seed-workflows";
import {
  STEP_KIND_META,
  type SkyNodeData,
  type StepKind,
  type SubWorkflowRef,
  type WorkflowSpec,
  type WorkflowStep,
} from "./types";
import { usePaletteHandlers } from "./use-palette-handlers";

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
 * This file is the orchestrator: it loads the workflow spec, owns the
 * `useWorkflowBuilderState` hook, and threads handlers (extracted into
 * `builder-handlers/` sibling files) into the canvas / toolbar /
 * inspector components.
 */

export default function WorkflowBuilderPage({
  params,
}: {
  params: Promise<{ verticalId: string; name: string }>;
}) {
  return (
    <Suspense fallback={<WorkflowBuilderSkeleton />}>
      <ReactFlowProvider>
        <BuilderInner params={params} />
      </ReactFlowProvider>
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
  const [triggerConfigOpen, setTriggerConfigOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  // Step id whose Pin-data modal is currently open, or null when closed.
  const [pinModalStepId, setPinModalStepId] = useState<string | null>(null);
  // One config per step id. Persisted client-side for v1; wired to the
  // gateway step-config save path in a follow-up brief.
  const [triggerConfigs, setTriggerConfigs] = useState<Record<string, TriggerConfig>>({});
  const annotations = useBuilderAnnotations();
  const workflowId = `${verticalId}/${name}`;

  // useReactFlow + history hook only function when the page is wrapped
  // by <ReactFlowProvider> (see WorkflowBuilderPage above).
  const reactFlow = useReactFlow();

  const historyHandlers = useHistoryHandlers(
    {
      nodes: builder.nodes,
      edges: builder.edges,
      setNodes: builder.setNodes,
      setEdges: builder.setEdges,
    },
    reactFlow,
  );

  const deleteDuplicate = useDeleteDuplicateHandlers({
    builder,
    annotations,
    reactFlow,
    selectedNode,
    setSelectedNode,
    setTriggerConfigOpen,
    captureSnapshot: historyHandlers.captureSnapshot,
    pushHistory: historyHandlers.history.push,
  });

  const proposal = useProposalHandlers({
    setNodes: builder.setNodes,
    setEdges: builder.setEdges,
    clearSelectedNode: useCallback(() => setSelectedNode(null), []),
  });

  const onNodeClick = useCallback((_e: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
    setTriggerConfigOpen(false);
  }, []);

  useBuilderKeyboard(
    useMemo(() => ({
      onUndo: historyHandlers.handleUndo,
      onRedo: historyHandlers.handleRedo,
      onFitView: historyHandlers.onFitView,
      onZoomIn: historyHandlers.onZoomIn,
      onZoomOut: historyHandlers.onZoomOut,
      onSelectAll: historyHandlers.handleSelectAll,
      onDuplicateSelected: historyHandlers.handleDuplicateSelected,
      onShowShortcuts: () => setShortcutsOpen(true),
    }), [historyHandlers]),
    true,
  );

  // Re-derive `selectedStep` from the live `builder.nodes` so inline
  // edits to the selected step (HTTP / Code config) flow back into the
  // inspector immediately — `selectedNode` itself is a snapshot taken
  // at click time and would otherwise show stale values.
  const selectedStep = selectedNode
    ? ((builder.nodes.find((n) => n.id === selectedNode.id)?.data as SkyNodeData | undefined)?.step
        ?? (selectedNode.data as SkyNodeData).step)
    : null;
  const selectedIsTrigger = selectedStep?.kind === "trigger";
  const selectedTriggerKind: TriggerKind = selectedStep?.triggerKind ?? "manual";

  const currentSteps = useMemo<WorkflowStep[]>(
    () => builder.nodes.map((n) => (n.data as SkyNodeData).step),
    [builder.nodes],
  );

  const runHistoryHandlers = useTestRunHistoryHandlers(workflowId);

  const { testRun, stepInputs, nodesWithRunState, handleTestRun } = useTestRunHandlers({
    verticalId,
    name,
    nodes: builder.nodes,
    currentSteps,
    annotations,
    onRunComplete: runHistoryHandlers.handleRunComplete,
  });

  const savePublishHandlers = useSavePublishHandlers({
    workflow,
    verticalId,
    name,
    currentSteps,
    nodes: builder.nodes,
    edges: builder.edges,
  });
  const {
    savePublish,
    savedDraftVersion,
    canPublish,
    publishConfirmOpen,
    setPublishConfirmOpen,
    handleSaveAndPublish,
    handlePublishRequest,
    handlePublishConfirm,
  } = savePublishHandlers;

  const handleDiscard = useCallback(() => {
    builder.reset(initial.nodes, initial.edges);
  }, [builder, initial]);

  const onConnect = useCallback(
    (connection: Connection) => {
      const result = validateConnection(connection, builder.nodes, builder.edges);
      if (!result.ok) {
        if (result.reason === "self_loop") {
          toast.error("A step can't connect to itself.");
        } else if (result.reason === "duplicate") {
          toast.error("These steps are already connected.");
        } else if (result.reason === "trigger_incoming") {
          toast.error("Triggers can't have incoming connections.");
        }
        return;
      }
      builder.addEdge(result.edge);
    },
    [builder],
  );

  const isValidConnection: IsValidConnection<Edge> = useCallback(
    (edgeOrConnection) =>
      isValidConnectionShape(
        {
          source: edgeOrConnection.source,
          target: edgeOrConnection.target,
          sourceHandle: edgeOrConnection.sourceHandle ?? null,
          targetHandle: edgeOrConnection.targetHandle ?? null,
        },
        builder.nodes,
      ),
    [builder.nodes],
  );

  const handleAddStep = useCallback(
    (kind: StepKind, subWorkflow?: SubWorkflowRef) => {
      const suffix = crypto.randomUUID().slice(0, 8);
      const node = newStepNode(kind, builder.nodes, suffix, subWorkflow);
      builder.addNode(node);
      toast.success(`${STEP_KIND_META[kind].label} added — drag to position.`, {
        description: subWorkflow
          ? `Calls ${subWorkflow.verticalId}/${subWorkflow.name}.`
          : STEP_KIND_DEFAULTS[kind].description,
      });
    },
    [builder],
  );

  const { handlePaletteDrop, handlePaletteSelect } = usePaletteHandlers({
    reactFlow,
    nodes: builder.nodes,
    addNode: builder.addNode,
    addEdge: builder.addEdge,
  });

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
      <div className="builder-shell">
        <BuilderToolbar
          workflow={workflow}
          verticalId={verticalId}
          name={name}
          dirty={builder.dirty}
          saving={savePublish.isSaving}
          publishing={savePublish.isPublishing}
          savedDraftVersion={savedDraftVersion}
          canPublish={canPublish}
          promptText={proposal.promptText}
          proposing={proposal.activeProposalPrompt !== null}
          onPromptTextChange={proposal.setPromptText}
          onProposeChange={proposal.handleProposeChange}
          onPickSuggestion={proposal.handlePickSuggestion}
          onTestRun={handleTestRun}
          onSaveAndPublish={handleSaveAndPublish}
          onPublish={handlePublishRequest}
          testRunPhase={testRun.phase}
        />

        <UnsavedBanner
          dirty={builder.dirty}
          onSave={handleSaveAndPublish}
          onDiscard={handleDiscard}
        />

        <div className="builder-stage">
          <BuilderSidebarPalette onSelect={handlePaletteSelect} />
          <BuilderCanvas
            nodes={nodesWithRunState}
            edges={builder.edges}
            onNodesChange={deleteDuplicate.onNodesChange}
            onEdgesChange={builder.onEdgesChange}
            onConnect={onConnect}
            isValidConnection={isValidConnection}
            onNodeClick={onNodeClick}
            onSelectStepKind={handleAddStep}
            onPaletteDrop={handlePaletteDrop}
            currentVerticalId={verticalId}
            currentName={name}
            onNodesDelete={deleteDuplicate.onNodesDelete}
            onEdgesDelete={deleteDuplicate.onEdgesDelete}
            onNodeContextMenu={deleteDuplicate.onNodeContextMenu}
            onEdgeContextMenu={deleteDuplicate.onEdgeContextMenu}
            onPaneClick={deleteDuplicate.onPaneClick}
            onPaneContextMenu={deleteDuplicate.onPaneContextMenu}
            onNodeDragStart={historyHandlers.onNodeDragStart}
            onNodeDragStop={historyHandlers.onNodeDragStop}
            onShowShortcuts={() => setShortcutsOpen(true)}
          />
          {deleteDuplicate.contextMenu && deleteDuplicate.contextMenuItems.length > 0 ? (
            <WorkflowContextMenu
              x={deleteDuplicate.contextMenu.x}
              y={deleteDuplicate.contextMenu.y}
              items={deleteDuplicate.contextMenuItems}
              onClose={deleteDuplicate.closeContextMenu}
            />
          ) : null}
          {selectedStep ? (
            <BuilderInspector
              step={selectedStep}
              isTrigger={!!selectedIsTrigger}
              pinnedJson={annotations.pinnedData[selectedStep.id] ?? null}
              onClose={() => setSelectedNode(null)}
              onEdit={() => setTriggerConfigOpen(true)}
              onOpenPin={() => setPinModalStepId(selectedStep.id)}
              onClearPin={() => annotations.clearPin(selectedStep.id)}
              onStepChange={(next) => {
                builder.setNodes((nds) =>
                  nds.map((n) =>
                    n.id === next.id
                      ? { ...n, data: { ...(n.data as SkyNodeData), step: next } }
                      : n,
                  ),
                );
              }}
            />
          ) : null}
          {pinModalStepId
            ? (() => {
                const target = currentSteps.find((s) => s.id === pinModalStepId);
                if (!target) return null;
                return (
                  <PinDataModal
                    stepId={target.id}
                    stepName={target.name}
                    initialJson={annotations.pinnedData[target.id] ?? null}
                    onSave={(json) => {
                      annotations.savePin(target.id, json);
                      setPinModalStepId(null);
                      toast.success(`Pinned input for ${target.name}`);
                    }}
                    onClear={() => annotations.clearPin(target.id)}
                    onClose={() => setPinModalStepId(null)}
                  />
                );
              })()
            : null}
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
          {proposal.activeProposalPrompt !== null ? (
            <BuilderAiProposal
              prompt={proposal.activeProposalPrompt}
              verticalId={verticalId}
              workerId={`${verticalId}-${name}`}
              currentSteps={currentSteps}
              onApprove={proposal.handleProposalApprove}
              onCancel={proposal.handleProposalCancel}
            />
          ) : null}
          <RunOverlay {...buildRunOverlayProps(runHistoryHandlers, testRun, stepInputs)} />
        </div>
        <div style={{ padding: "12px 28px 24px" }}>
          <TestRunHistoryPanel
            entries={runHistoryHandlers.runHistory.entries}
            onView={(entry) => testRun.phase !== "running" && runHistoryHandlers.openReplay(entry)}
          />
        </div>
      </div>
      <KeyboardShortcutsModal open={shortcutsOpen} onOpenChange={setShortcutsOpen} />
      <AlertDialog
        open={publishConfirmOpen}
        onOpenChange={setPublishConfirmOpen}
        title={
          savedDraftVersion !== null
            ? `Publish v${savedDraftVersion} to head?`
            : "Publish draft to head?"
        }
        description="Publishing makes this version the live workflow for every run from now on. Existing runs in flight are unaffected."
        confirmLabel="Publish to head"
        cancelLabel="Cancel"
        confirmVariant="primary"
        onConfirm={handlePublishConfirm}
      />
      <MobileBuilderGate workflow={workflow} />
    </>
  );
}
