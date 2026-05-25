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
import { useWorkflow } from "@/lib/hooks/use-workflows";
import { useWorkflowBuilderState } from "@/lib/hooks/use-workflow-builder-state";
import { isDemoMode } from "@/lib/skycot/demo-mode";

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
import { resolveBuilderWorkflowSource } from "./resolve-workflow";
import { RunOverlay } from "./RunOverlay";
import {
  STEP_KIND_META,
  type SkyNodeData,
  type StepKind,
  type SubWorkflowRef,
  type WorkflowSpec,
  type WorkflowStep,
} from "./types";
import { usePaletteHandlers } from "./use-palette-handlers";

const [NEW_WORKFLOW_SENTINEL, DEFAULT_NEW_WORKFLOW_NAME] = ["__new__", "new_workflow"];

export default function WorkflowBuilderPage({ params }: {
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

function BuilderInner({ params }: {
  params: Promise<{ verticalId: string; name: string }>;
}) {
  const { verticalId, name } = use(params);
  const isNewWorkflow = name === NEW_WORKFLOW_SENTINEL;
  const router = useRouter();
  const pathname = usePathname() ?? `/workflows/${verticalId}/${name}/build`;
  const searchParams = useSearchParams();
  const templateId = searchParams.get("from_template");
  const stateFlag = searchParams.get("state");
  const [draftName, setDraftName] = useState(DEFAULT_NEW_WORKFLOW_NAME);
  const workflowName = useMemo(() => {
    if (!isNewWorkflow) return name;
    return draftName.toLowerCase().replace(/[^a-z0-9_]+/g, "_").replace(/^_+|_+$/g, "")
      || DEFAULT_NEW_WORKFLOW_NAME;
  }, [draftName, isNewWorkflow, name]);
  const workflowQuery = useWorkflow(
    templateId || isNewWorkflow ? null : verticalId,
    templateId || isNewWorkflow ? null : name,
  );
  const workflow = useMemo<WorkflowSpec | undefined>(() => {
      if (isNewWorkflow) {
        return {
          name: workflowName,
          vertical: verticalId,
          description: "New workflow draft.",
          version: "draft",
          risk: "autonomous",
          steps: [],
        };
      }
      return resolveBuilderWorkflowSource({
        verticalId,
        name,
        templateId,
        data: workflowQuery.data as { workflow: WorkflowSpec | null } | undefined,
        demoMode: isDemoMode(),
      });
    },
    [isNewWorkflow, workflowName, templateId, verticalId, name, workflowQuery.data],
  );
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
  const [pinModalStepId, setPinModalStepId] = useState<string | null>(null);
  const [triggerConfigs, setTriggerConfigs] = useState<Record<string, TriggerConfig>>({});
  const annotations = useBuilderAnnotations();
  const workflowId = `${verticalId}/${workflowName}`;

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
    name: workflowName,
    nodes: builder.nodes,
    currentSteps,
    annotations,
    onRunComplete: runHistoryHandlers.handleRunComplete,
  });

  const savePublishHandlers = useSavePublishHandlers({
    workflow,
    verticalId,
    name: workflowName,
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

  if (stateFlag === "loading" || (!templateId && workflowQuery.isLoading)) {
    return <WorkflowBuilderSkeleton />;
  }

  if (stateFlag === "error" || (!templateId && workflowQuery.isError && !workflow)) {
    return <WorkflowBuilderErrorState onRetry={resetState} />;
  }

  if (!workflow) {
    return (
      <div style={{ padding: 48, textAlign: "center", color: "var(--muted-foreground)" }}>
        Workflow not found. Try one from <Link href="/workflows" style={{ color: "#8b5cf6" }}>the catalog</Link>.
      </div>
    );
  }

  if (!isNewWorkflow && (workflow.steps.length === 0 || stateFlag === "empty")) {
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
          name={workflowName}
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
            currentName={workflowName}
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
          ) : isNewWorkflow ? (
            <aside className="builder-panel">
              <div className="builder-panel-head">
                <span className="builder-panel-eyebrow">Draft setup</span>
              </div>
              <h2 className="builder-panel-title">Name workflow</h2>
              <label className="builder-panel-section">
                <span className="builder-panel-section-label">Workflow name</span>
                <input
                  value={draftName}
                  onChange={(e) => setDraftName(e.target.value)}
                  data-testid="builder-new-workflow-name"
                  aria-label="Workflow name"
                  className="mt-1.5 block w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 py-2.5 text-sm font-medium normal-case text-[var(--foreground)]"
                />
              </label>
            </aside>
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
              workerId={`${verticalId}-${workflowName}`}
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
