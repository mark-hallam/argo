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
import { Copy, Settings as SettingsIcon, StickyNote as StickyNoteIcon, Trash2, Unlink } from "lucide-react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

import { UnsavedBanner } from "@/components/skycot/builder/UnsavedBanner";
import { AlertDialog } from "@/components/ui/AlertDialog";
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
import { MobileBuilderGate } from "@/components/skycot/workflows/MobileBuilderGate";
import { useBuilderAnnotations } from "@/lib/hooks/use-builder-annotations";
import { useBuilderHistory, type BuilderSnapshot } from "@/lib/hooks/use-builder-history";
import { useBuilderKeyboard } from "@/lib/hooks/use-builder-keyboard";
import { useWorkflowBuilderState } from "@/lib/hooks/use-workflow-builder-state";
import { useWorkflowSavePublish } from "@/lib/hooks/use-workflow-save-publish";
import { getTemplate } from "@/lib/workflow-templates";

import { BuilderAiProposal } from "./BuilderAiProposal";
import { BuilderCanvas, SNAP_GRID_SIZE } from "./BuilderCanvas";
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
import { stepsToFlow } from "./layout";
import { RunOverlay } from "./RunOverlay";
import { SEED_WORKFLOWS } from "./seed-workflows";
import { encodeStepsToToml } from "./step-encoder";
import {
  STEP_KIND_META,
  type SkyNodeData,
  type StepKind,
  type SubWorkflowRef,
  type WorkflowSpec,
  type WorkflowStep,
} from "./types";
import { usePaletteHandlers } from "./use-palette-handlers";
import { useTestRun, type StepInput } from "./useTestRun";

interface ContextMenuState {
  x: number;
  y: number;
  nodeId: string | null;
  edgeId: string | null;
  // Flow-space coords captured at right-click on empty canvas, used as
  // the drop position for "Add sticky note". null when the menu was
  // opened on a node/edge.
  paneFlow: { x: number; y: number } | null;
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
      // Stickies live in `annotations.notes`, not `builder.nodes`. Route
      // their deletes through the annotations hook so the source of
      // truth updates; otherwise the next render rebuilds the sticky.
      const stickies = deleted.filter((n) => n.type === "sticky");
      for (const s of stickies) annotations.deleteNote(s.id);

      const steps = deleted.filter((n) => n.type !== "sticky");
      if (steps.length === 0) return;
      const label = `Delete ${steps.length} step${steps.length === 1 ? "" : "s"}`;
      history.push(captureSnapshot(label));
      const triggers = steps.filter((n) => nodeStep(n)?.kind === "trigger");
      const deletable = steps.filter((n) => nodeStep(n)?.kind !== "trigger");

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
    [builder, selectedNode, captureSnapshot, history, annotations],
  );

  const onEdgesDelete = useCallback(
    (deleted: Edge[]) => {
      for (const edge of deleted) builder.removeEdge(edge.id);
    },
    [builder],
  );

  // Stickies live outside `builder.nodes` (their source is
  // `annotations.notes`). React Flow doesn't know that — it fires
  // position + remove changes for stickies alongside step nodes. Route
  // sticky-id changes to the annotations hook and pass step-node
  // changes through to the builder's controlled state.
  const onNodesChange = useCallback<typeof builder.onNodesChange>(
    (changes) => {
      const forBuilder: typeof changes = [];
      for (const change of changes) {
        if (change.type === "position" && annotations.isNoteId(change.id)) {
          if (change.position) {
            annotations.updateNotePosition(
              change.id,
              change.position.x,
              change.position.y,
            );
          }
          continue;
        }
        if (change.type === "remove" && annotations.isNoteId(change.id)) {
          annotations.deleteNote(change.id);
          continue;
        }
        forBuilder.push(change);
      }
      if (forBuilder.length > 0) builder.onNodesChange(forBuilder);
    },
    [builder, annotations],
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
    setContextMenu({ x: event.clientX, y: event.clientY, nodeId: node.id, edgeId: null, paneFlow: null });
  }, []);

  const onEdgeContextMenu = useCallback((event: React.MouseEvent, edge: Edge) => {
    event.preventDefault();
    setContextMenu({ x: event.clientX, y: event.clientY, nodeId: null, edgeId: edge.id, paneFlow: null });
  }, []);

  const onPaneClick = useCallback(() => setContextMenu(null), []);
  const closeContextMenu = useCallback(() => setContextMenu(null), []);

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();
      const flow = reactFlow.screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        nodeId: null,
        edgeId: null,
        paneFlow: flow,
      });
    },
    [reactFlow],
  );

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
    if (contextMenu.paneFlow) {
      const { x, y } = contextMenu.paneFlow;
      return [
        {
          label: "Add sticky note",
          Icon: StickyNoteIcon,
          onClick: () => {
            annotations.addNote(x, y);
            toast.success("Sticky note added", {
              description: "Double-click to edit, drag to reposition.",
            });
          },
        },
      ];
    }
    return [];
  }, [contextMenu, builder, duplicateNode, onNodesDelete, annotations]);

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
  const nodesWithRunState = useMemo(() => {
    const decorated = annotations.decorateWithPins(
      builder.nodes.map((n) => {
        const sky = n.data as SkyNodeData;
        const next = testRun.stepStates[sky.step.id];
        if (sky.runState === next) return n;
        return { ...n, data: { ...sky, runState: next } };
      }),
    );
    return [...decorated, ...annotations.noteNodes];
  }, [builder.nodes, testRun.stepStates, annotations]);

  const pinnedStepNames = useMemo(() => {
    const ids = Object.keys(annotations.pinnedData);
    if (ids.length === 0) return [] as string[];
    const idToName = new Map(currentSteps.map((s) => [s.id, s.name] as const));
    return ids.map((id) => idToName.get(id) ?? id);
  }, [annotations.pinnedData, currentSteps]);

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
    if (pinnedStepNames.length > 0) {
      toast.message(`Using pinned data for: ${pinnedStepNames.join(", ")}`, {
        description: "Live inputs are overridden for the pinned steps.",
      });
    }
    testRun.start();
  }, [testRun, stepInputs.length, pinnedStepNames]);

  const savePublish = useWorkflowSavePublish();
  // After every successful save we record the draft id/version AND a
  // JSON snapshot of the canvas at save time. Publish then compares
  // the current canvas to that snapshot — equal means "safe to publish
  // the saved draft"; diverged means "user edited again, must save first".
  // This works around the phase-1 builder-state hook not resetting its
  // own `dirty` baseline on save (out of scope for this brief; the
  // local snapshot tracking is self-contained in this page).
  const [savedDraft, setSavedDraft] = useState<
    { version_id: string; version: number; snapshot: string } | null
  >(null);
  const canvasSnapshot = useMemo(
    () => JSON.stringify({ nodes: builder.nodes, edges: builder.edges }),
    [builder.nodes, builder.edges],
  );
  const canPublish =
    savedDraft !== null && savedDraft.snapshot === canvasSnapshot;
  const [publishConfirmOpen, setPublishConfirmOpen] = useState(false);

  const performSaveDraft = useCallback(async (): Promise<void> => {
    if (!workflow) return;
    const definitionToml = encodeStepsToToml(currentSteps, {
      workflowName: workflow.name,
      workflowDescription: workflow.description,
    });
    const snapshot = JSON.stringify({
      nodes: builder.nodes,
      edges: builder.edges,
    });
    const result = await savePublish.saveDraft({
      vertical_id: verticalId,
      name,
      definition_toml: definitionToml,
    });
    setSavedDraft({
      version_id: result.version_id,
      version: result.version,
      snapshot,
    });
    toast.success(`Saved (v${result.version})`);
  }, [savePublish, currentSteps, workflow, verticalId, name, builder.nodes, builder.edges]);
  const handleSaveAndPublish = useCallback(() => {
    performSaveDraft().catch((err) => {
      const message = err instanceof Error && err.message
        ? err.message
        : "Network error";
      toast.error("Couldn't save workflow", {
        description: message,
        action: { label: "Retry", onClick: () => { void performSaveDraft(); } },
      });
    });
  }, [performSaveDraft]);

  const performPublishHead = useCallback(async (): Promise<void> => {
    if (!savedDraft) return;
    const result = await savePublish.publishHead({
      vertical_id: verticalId,
      name,
      version_id: savedDraft.version_id,
    });
    toast.success(`Published v${result.head_version} to head`);
    router.push(`/workflows/${verticalId}/${name}`);
  }, [savePublish, savedDraft, verticalId, name, router]);
  const handlePublishConfirm = useCallback(() => {
    setPublishConfirmOpen(false);
    performPublishHead().catch((err) => {
      const message = err instanceof Error && err.message
        ? err.message
        : "Network error";
      toast.error("Couldn't publish workflow", {
        description: message,
        action: {
          label: "Retry",
          onClick: () => { void performPublishHead(); },
        },
      });
    });
  }, [performPublishHead]);
  const handlePublishRequest = useCallback(() => {
    if (!canPublish) return;
    setPublishConfirmOpen(true);
  }, [canPublish]);

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
          savedDraftVersion={savedDraft?.version ?? null}
          canPublish={canPublish}
          promptText={promptText}
          proposing={activeProposalPrompt !== null}
          onPromptTextChange={setPromptText}
          onProposeChange={handleProposeChange}
          onPickSuggestion={handlePickSuggestion}
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
            onNodesChange={onNodesChange}
            onEdgesChange={builder.onEdgesChange}
            onConnect={onConnect}
            isValidConnection={isValidConnection}
            onNodeClick={onNodeClick}
            onSelectStepKind={handleAddStep}
            onPaletteDrop={handlePaletteDrop}
            currentVerticalId={verticalId}
            currentName={name}
            onNodesDelete={onNodesDelete}
            onEdgesDelete={onEdgesDelete}
            onNodeContextMenu={onNodeContextMenu}
            onEdgeContextMenu={onEdgeContextMenu}
            onPaneClick={onPaneClick}
            onPaneContextMenu={onPaneContextMenu}
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
              pinnedJson={annotations.pinnedData[selectedStep.id] ?? null}
              onClose={() => setSelectedNode(null)}
              onEdit={() => setTriggerConfigOpen(true)}
              onOpenPin={() => setPinModalStepId(selectedStep.id)}
              onClearPin={() => annotations.clearPin(selectedStep.id)}
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
      <AlertDialog
        open={publishConfirmOpen}
        onOpenChange={setPublishConfirmOpen}
        title={
          savedDraft
            ? `Publish v${savedDraft.version} to head?`
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
