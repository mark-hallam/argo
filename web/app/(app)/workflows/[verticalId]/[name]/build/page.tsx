"use client";
import "./page.css";

import "@xyflow/react/dist/style.css";
import type { Connection, Edge, IsValidConnection, Node } from "@xyflow/react";
import { Copy, Settings as SettingsIcon, Trash2, Unlink } from "lucide-react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import { UnsavedBanner } from "@/components/skycot/builder/UnsavedBanner";
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
import { useWorkflowBuilderState } from "@/lib/hooks/use-workflow-builder-state";
import { getTemplate } from "@/lib/workflow-templates";

import { BuilderCanvas } from "./BuilderCanvas";
import { BuilderInspector } from "./BuilderInspector";
import { BuilderToolbar } from "./BuilderToolbar";
import {
  isValidConnectionShape,
  newStepNode,
  STEP_KIND_DEFAULTS,
  validateConnection,
} from "./builder-handlers";
import { stepsToFlow } from "./layout";
import { SEED_WORKFLOWS } from "./seed-workflows";
import { STEP_KIND_META, type SkyNodeData, type StepKind, type WorkflowSpec } from "./types";

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
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  // One config per step id. Persisted client-side for v1; wired to the
  // gateway step-config save path in a follow-up brief.
  const [triggerConfigs, setTriggerConfigs] = useState<Record<string, TriggerConfig>>({});
  const workflowId = `${verticalId}/${name}`;

  const onNodeClick = useCallback((_e: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
    setTriggerConfigOpen(false);
  }, []);

  // ReactFlow optimistically removes the node before this fires; for
  // single-node deletes we surface an "Undo" toast that re-adds via
  // builder.addNode (the controlled-state hook is idempotent on id).
  const onNodesDelete = useCallback(
    (deleted: Node[]) => {
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
    [builder, selectedNode],
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
    (kind: StepKind) => {
      const suffix = crypto.randomUUID().slice(0, 8);
      const node = newStepNode(kind, builder.nodes, suffix);
      builder.addNode(node);
      toast.success(`${STEP_KIND_META[kind].label} added — drag to position.`, {
        description: STEP_KIND_DEFAULTS[kind].description,
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
          proposing={proposing}
          onPromptTextChange={setPromptText}
          onProposeChange={handleProposeChange}
          onTestRun={handleTestRun}
          onSaveAndPublish={handleSaveAndPublish}
        />

        <UnsavedBanner
          dirty={builder.dirty}
          onSave={handleSaveAndPublish}
          onDiscard={handleDiscard}
        />

        <div className="builder-stage">
          <BuilderCanvas
            nodes={builder.nodes}
            edges={builder.edges}
            onNodesChange={builder.onNodesChange}
            onEdgesChange={builder.onEdgesChange}
            onConnect={onConnect}
            isValidConnection={isValidConnection}
            onNodeClick={onNodeClick}
            onSelectStepKind={handleAddStep}
            onNodesDelete={onNodesDelete}
            onEdgesDelete={onEdgesDelete}
            onNodeContextMenu={onNodeContextMenu}
            onEdgeContextMenu={onEdgeContextMenu}
            onPaneClick={onPaneClick}
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
        </div>
      </div>
    </>
  );
}
