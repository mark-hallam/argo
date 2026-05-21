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
import { Copy, Settings as SettingsIcon, Trash2, Unlink } from "lucide-react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

import { UnsavedBanner } from "@/components/skycot/builder/UnsavedBanner";
import { KeyboardShortcutsModal } from "@/components/skycot/workflows/KeyboardShortcutsModal";
import TriggerConfigDrawer from "@/components/workflows/triggers/TriggerConfigDrawer";
import type { TriggerConfig, TriggerKind } from "@/components/workflows/triggers/types";
import { WorkflowBuilderSkeleton } from "@/components/skycot/workflows/WorkflowBuilderSkeleton";
import {
  WorkflowContextMenu,
  type ContextMenuItem,
} from "@/components/skycot/workflows/WorkflowContextMenu";
import {
  WorkflowBuilderEmptyState,
  WorkflowBuilderErrorState,
} from "@/components/skycot/workflows/WorkflowEdgeStates";
import { useBuilderHistory, type BuilderSnapshot } from "@/lib/hooks/use-builder-history";
import { useBuilderKeyboard } from "@/lib/hooks/use-builder-keyboard";
import { useWorkflowBuilderState } from "@/lib/hooks/use-workflow-builder-state";
import { getTemplate } from "@/lib/workflow-templates";

import { BuilderAiProposal } from "./BuilderAiProposal";
import { BuilderCanvas, SNAP_GRID_SIZE } from "./BuilderCanvas";
import { BuilderInspector } from "./BuilderInspector";
import { BuilderToolbar } from "./BuilderToolbar";
import {
  isValidConnectionShape,
  newStepNode,
  STEP_KIND_DEFAULTS,
  validateConnection,
} from "./builder-handlers";
import { stepsToFlow } from "./layout";
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
import { useTestRun, type StepInput } from "./useTestRun";

interface ContextMenuState {
  x: number;
  y: number;
  nodeId: string | null;
  edgeId: string | null;
}

function nodeStep(node: Node | undefined): SkyNodeData["step"] | undefined {
  return (node?.data as SkyNodeData | undefined)?.step;
}

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
  const [promptText, setPromptText] = useState("");
  // Active proposal prompt — when set, the AI proposal panel is open
  // and streaming. Cleared on approve / reject / new propose request.
  const [activeProposalPrompt, setActiveProposalPrompt] = useState<string | null>(null);
  const [triggerConfigOpen, setTriggerConfigOpen] = useState(false);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  // One config per step id. Persisted client-side for v1; wired to the
  // gateway step-config save path in a follow-up brief.
  const [triggerConfigs, setTriggerConfigs] = useState<Record<string, TriggerConfig>>({});
  const workflowId = `${verticalId}/${name}`;

  // useReactFlow + history hook only function when the page is wrapped
  // by <ReactFlowProvider> (see WorkflowBuilderPage above).
  const reactFlow = useReactFlow();
  const history = useBuilderHistory();
  // Keep latest nodes/edges in refs so snapshot helpers — bound to
  // keyboard handlers that read once at hook registration — always
  // sample the current graph state.
  const stateRef = useRef({ nodes: builder.nodes, edges: builder.edges });
  stateRef.current = { nodes: builder.nodes, edges: builder.edges };
  // Pre-drag snapshot, captured at onNodeDragStart so the snapshot
  // pushed on drag-stop is the BEFORE-position graph (gotcha #4 in the
  // brief: one snapshot per drag, captured at the source).
  const preDragSnapshotRef = useRef<BuilderSnapshot | null>(null);

  const captureSnapshot = useCallback((label: string): BuilderSnapshot => ({
    nodes: structuredClone(stateRef.current.nodes),
    edges: structuredClone(stateRef.current.edges),
    label,
    takenAt: Date.now(),
  }), []);

  const onNodeClick = useCallback((_e: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
    setTriggerConfigOpen(false);
  }, []);

  const onNodeDragStart = useCallback(() => {
    preDragSnapshotRef.current = captureSnapshot("Move step");
  }, [captureSnapshot]);

  const onNodeDragStop = useCallback(() => {
    const before = preDragSnapshotRef.current;
    preDragSnapshotRef.current = null;
    if (!before) return;
    const movedNodes = stateRef.current.nodes;
    const positionChanged =
      before.nodes.length !== movedNodes.length
      || before.nodes.some((bn, idx) => {
        const mn = movedNodes[idx];
        if (!mn || bn.id !== mn.id) return true;
        return bn.position.x !== mn.position.x || bn.position.y !== mn.position.y;
      });
    if (positionChanged) history.push(before);
  }, [history]);

  // ReactFlow optimistically removes the node before this fires; for
  // single-node deletes we surface an "Undo" toast that re-adds via
  // builder.addNode (the controlled-state hook is idempotent on id).
  // Pushes a history snapshot for keyboard undo (Cmd+Z) too.
  const onNodesDelete = useCallback(
    (deleted: Node[]) => {
      if (deleted.length > 0) {
        const label = `Delete ${deleted.length} step${deleted.length === 1 ? "" : "s"}`;
        history.push(captureSnapshot(label));
      }
      const triggers = deleted.filter((n) => nodeStep(n)?.kind === "trigger");
      const deletable = deleted.filter((n) => nodeStep(n)?.kind !== "trigger");

      if (triggers.length > 0) {
        toast.error("Trigger steps can't be deleted", {
          description: "Change the trigger kind from the side panel instead.",
        });
        for (const t of triggers) builder.addNode(t);
      }
      if (deletable.length === 0) return;

      // Single delete → instant + undoable.
      if (deletable.length === 1) {
        const only = deletable[0];
        builder.removeNode(only.id);
        if (selectedNode?.id === only.id) setSelectedNode(null);
        toast.success("Step deleted", {
          action: {
            label: "Undo",
            onClick: () => builder.addNode(only),
          },
        });
        return;
      }

      // Bulk delete → confirm via window.confirm (cheap, no modal dep).
      const confirmed = typeof window !== "undefined"
        ? window.confirm(
            `Delete ${deletable.length} steps? This also removes edges connecting them.`,
          )
        : true;
      if (!confirmed) {
        for (const n of deletable) builder.addNode(n);
        return;
      }
      for (const n of deletable) builder.removeNode(n.id);
      if (selectedNode && deletable.some((n) => n.id === selectedNode.id)) {
        setSelectedNode(null);
      }
      toast.success(`${deletable.length} steps deleted`);
    },
    [builder, selectedNode, captureSnapshot, history],
  );

  const onEdgesDelete = useCallback(
    (deleted: Edge[]) => {
      for (const edge of deleted) builder.removeEdge(edge.id);
    },
    [builder],
  );

  const duplicateNode = useCallback(
    (nodeId: string) => {
      const original = builder.nodes.find((n) => n.id === nodeId);
      if (!original) return;
      const step = nodeStep(original);
      if (step?.kind === "trigger") {
        toast.error("Trigger steps can't be duplicated", {
          description: "Only one trigger per workflow.",
        });
        return;
      }
      if (!step) return;
      const suffix = typeof crypto !== "undefined" && "randomUUID" in crypto
        ? crypto.randomUUID().slice(0, 8)
        : Math.random().toString(36).slice(2, 10);
      const newId = `step-${suffix}`;
      const duplicated: Node = {
        ...original,
        id: newId,
        position: { x: original.position.x + 60, y: original.position.y + 60 },
        data: { step: { ...step, id: newId, name: `${step.name} (copy)` } },
        selected: false,
      };
      builder.addNode(duplicated);
      toast.success("Step duplicated");
    },
    [builder],
  );

  const onNodeContextMenu = useCallback((event: React.MouseEvent, node: Node) => {
    event.preventDefault();
    setContextMenu({ x: event.clientX, y: event.clientY, nodeId: node.id, edgeId: null });
  }, []);

  const onEdgeContextMenu = useCallback((event: React.MouseEvent, edge: Edge) => {
    event.preventDefault();
    setContextMenu({ x: event.clientX, y: event.clientY, nodeId: null, edgeId: edge.id });
  }, []);

  const onPaneClick = useCallback(() => setContextMenu(null), []);
  const closeContextMenu = useCallback(() => setContextMenu(null), []);

  const contextMenuItems = useMemo<ContextMenuItem[]>(() => {
    if (!contextMenu) return [];
    if (contextMenu.nodeId) {
      const node = builder.nodes.find((n) => n.id === contextMenu.nodeId);
      if (!node) return [];
      const isTrigger = nodeStep(node)?.kind === "trigger";
      return [
        {
          label: "Configure",
          Icon: SettingsIcon,
          onClick: () => {
            setSelectedNode(node);
            setTriggerConfigOpen(false);
          },
        },
        {
          label: "Duplicate",
          Icon: Copy,
          onClick: () => duplicateNode(node.id),
          disabled: isTrigger,
        },
        {
          label: "Delete",
          Icon: Trash2,
          onClick: () => onNodesDelete([node]),
          destructive: true,
          disabled: isTrigger,
        },
      ];
    }
    if (contextMenu.edgeId) {
      const edgeId = contextMenu.edgeId;
      return [
        {
          label: "Disconnect",
          Icon: Unlink,
          onClick: () => builder.removeEdge(edgeId),
          destructive: true,
        },
      ];
    }
    return [];
  }, [contextMenu, builder, duplicateNode, onNodesDelete]);

  const handleUndo = useCallback(() => {
    const prev = history.undo();
    if (!prev) return;
    history.pushFuture({
      nodes: structuredClone(stateRef.current.nodes),
      edges: structuredClone(stateRef.current.edges),
      label: prev.label,
      takenAt: Date.now(),
    });
    builder.setNodes(prev.nodes);
    builder.setEdges(prev.edges);
    toast.message(`Undone: ${prev.label}`);
  }, [builder, history]);

  const handleRedo = useCallback(() => {
    const next = history.redo();
    if (!next) return;
    history.pushPast({
      nodes: structuredClone(stateRef.current.nodes),
      edges: structuredClone(stateRef.current.edges),
      label: next.label,
      takenAt: Date.now(),
    });
    builder.setNodes(next.nodes);
    builder.setEdges(next.edges);
    toast.message(`Redone: ${next.label}`);
  }, [builder, history]);

  const handleSelectAll = useCallback(() => {
    builder.setNodes((nds) => nds.map((n) => ({ ...n, selected: true })));
  }, [builder]);

  const handleDuplicateSelected = useCallback(() => {
    const selected = stateRef.current.nodes.filter((n) => n.selected);
    if (selected.length === 0) return;
    history.push(captureSnapshot(`Duplicate ${selected.length} step${selected.length === 1 ? "" : "s"}`));
    const clones: Node[] = selected.map((n) => {
      const id = (typeof crypto !== "undefined" && "randomUUID" in crypto)
        ? crypto.randomUUID()
        : `${n.id}-copy-${Date.now()}`;
      return {
        ...n,
        id,
        position: { x: n.position.x + SNAP_GRID_SIZE * 2, y: n.position.y + SNAP_GRID_SIZE * 2 },
        selected: false,
      };
    });
    builder.setNodes((nds) =>
      [...nds.map((n) => ({ ...n, selected: false })), ...clones],
    );
  }, [builder, captureSnapshot, history]);

  useBuilderKeyboard(
    useMemo(() => ({
      onUndo: handleUndo,
      onRedo: handleRedo,
      onFitView: () => reactFlow.fitView({ padding: 0.18, duration: 250 }),
      onZoomIn: () => reactFlow.zoomIn({ duration: 150 }),
      onZoomOut: () => reactFlow.zoomOut({ duration: 150 }),
      onSelectAll: handleSelectAll,
      onDuplicateSelected: handleDuplicateSelected,
      onShowShortcuts: () => setShortcutsOpen(true),
    }), [handleUndo, handleRedo, reactFlow, handleSelectAll, handleDuplicateSelected]),
    true,
  );

  const selectedStep = selectedNode ? (selectedNode.data as SkyNodeData).step : null;
  const selectedIsTrigger = selectedStep?.kind === "trigger";
  const selectedTriggerKind: TriggerKind = selectedStep?.triggerKind ?? "manual";

  const handleProposeChange = useCallback(() => {
    const prompt = promptText.trim();
    if (!prompt) return;
    setActiveProposalPrompt(prompt);
  }, [promptText]);

  const handlePickSuggestion = useCallback((next: string) => {
    setPromptText(next);
    setActiveProposalPrompt(next.trim());
  }, []);

  const currentSteps = useMemo<WorkflowStep[]>(
    () => builder.nodes.map((n) => (n.data as SkyNodeData).step),
    [builder.nodes],
  );

  const stepInputs = useMemo<StepInput[]>(
    () => currentSteps.map((s) => ({ id: s.id, name: s.name, kind: s.kind })),
    [currentSteps],
  );
  const testRun = useTestRun(verticalId, name, stepInputs);
  const nodesWithRunState = useMemo(
    () =>
      builder.nodes.map((n) => {
        const sky = n.data as SkyNodeData;
        const next = testRun.stepStates[sky.step.id];
        if (sky.runState === next) return n;
        return { ...n, data: { ...sky, runState: next } };
      }),
    [builder.nodes, testRun.stepStates],
  );

  const handleProposalApprove = useCallback(
    (steps: WorkflowStep[]) => {
      const next = stepsToFlow(steps);
      builder.setNodes(next.nodes);
      builder.setEdges(next.edges);
      setSelectedNode(null);
      setActiveProposalPrompt(null);
      setPromptText("");
      toast.success("Proposal applied to canvas", {
        description: "Review the new graph, then Save + publish to persist.",
      });
    },
    [builder],
  );

  const handleProposalCancel = useCallback(() => {
    setActiveProposalPrompt(null);
  }, []);

  const handleTestRun = useCallback(() => {
    if (testRun.phase === "running") return;
    if (stepInputs.length === 0) {
      toast.error("Add a step before running a test.");
      return;
    }
    testRun.start();
  }, [testRun, stepInputs.length]);

  const handleSaveAndPublish = useCallback(() => {
    // Honest demo stub — wave-6 audit. Wave-13 cleaned up the leaked
    // version identifiers; copy is now neutral.
    toast.message("Saved as draft — preview build", {
      description: "Tenant-wide publish lands in an upcoming release.",
    });
  }, []);

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
          proposing={activeProposalPrompt !== null}
          onPromptTextChange={setPromptText}
          onProposeChange={handleProposeChange}
          onPickSuggestion={handlePickSuggestion}
          onTestRun={handleTestRun}
          onSaveAndPublish={handleSaveAndPublish}
          testRunPhase={testRun.phase}
        />

        <UnsavedBanner
          dirty={builder.dirty}
          onSave={handleSaveAndPublish}
          onDiscard={handleDiscard}
        />

        <div className="builder-stage">
          <BuilderCanvas
            nodes={nodesWithRunState}
            edges={builder.edges}
            onNodesChange={builder.onNodesChange}
            onEdgesChange={builder.onEdgesChange}
            onConnect={onConnect}
            isValidConnection={isValidConnection}
            onNodeClick={onNodeClick}
            onSelectStepKind={handleAddStep}
            currentVerticalId={verticalId}
            currentName={name}
            onNodesDelete={onNodesDelete}
            onEdgesDelete={onEdgesDelete}
            onNodeContextMenu={onNodeContextMenu}
            onEdgeContextMenu={onEdgeContextMenu}
            onPaneClick={onPaneClick}
            onNodeDragStart={onNodeDragStart}
            onNodeDragStop={onNodeDragStop}
            onShowShortcuts={() => setShortcutsOpen(true)}
          />
          {contextMenu && contextMenuItems.length > 0 ? (
            <WorkflowContextMenu
              x={contextMenu.x}
              y={contextMenu.y}
              items={contextMenuItems}
              onClose={closeContextMenu}
            />
          ) : null}
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
          {activeProposalPrompt !== null ? (
            <BuilderAiProposal
              prompt={activeProposalPrompt}
              verticalId={verticalId}
              workerId={`${verticalId}-${name}`}
              currentSteps={currentSteps}
              onApprove={handleProposalApprove}
              onCancel={handleProposalCancel}
            />
          ) : null}
          <RunOverlay
            phase={testRun.phase}
            steps={stepInputs}
            stepStates={testRun.stepStates}
            summary={testRun.summary}
            transportError={testRun.transportError}
            onDismiss={testRun.reset}
          />
        </div>
      </div>
      <KeyboardShortcutsModal open={shortcutsOpen} onOpenChange={setShortcutsOpen} />
    </>
  );
}
