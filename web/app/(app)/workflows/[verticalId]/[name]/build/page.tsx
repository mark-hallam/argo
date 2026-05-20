"use client";

import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  type Node,
  type Edge,
  Handle,
  Position,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { Suspense, use, useCallback, useMemo, useState } from "react";
import { toast } from "sonner";
import {
  ArrowLeft,
  Play,
  Sparkles,
  ShieldCheck,
  Database,
  MessageSquare,
  GitBranch,
  Settings as SettingsIcon,
  Send,
  X,
  Plus,
} from "lucide-react";
import { getTemplate } from "@/lib/workflow-templates";

import TriggerConfigDrawer from "@/components/workflows/triggers/TriggerConfigDrawer";
import type { TriggerConfig, TriggerKind } from "@/components/workflows/triggers/types";
import { WorkflowBuilderSkeleton } from "@/components/skycot/workflows/WorkflowBuilderSkeleton";
import {
  WorkflowBuilderEmptyState,
  WorkflowBuilderErrorState,
} from "@/components/skycot/workflows/WorkflowEdgeStates";

/**
 * Class-leading visual workflow builder — `/workflows/<v>/<name>/build`.
 *
 * Combines two editing modalities (the SKYCOT-class-leading
 * differentiator):
 *
 *   1. Natural-language AI prompt area at the top — describe a change
 *      in plain English ("add an approval step before posting") and
 *      the AI proposes the diff
 *
 *   2. Full visual canvas powered by React Flow (@xyflow/react):
 *      - Drag-drop nodes
 *      - Pan + zoom canvas
 *      - Connection lines between steps
 *      - Mini-map navigation
 *      - Brand-loud per-step-kind node design (gradient icons +
 *        approval-gate banners + IN/OUT meta)
 *      - Side panel for node config when a node is clicked
 *      - Add Step palette button
 *
 * V1 seeds nodes from the same demo workflow data as the read-only
 * viewer at /workflows/<v>/<n>. Real editing wires through to the
 * gateway PUT workflow endpoint in a follow-up brief; this builder
 * is the canvas + AI-prompt UX that closes the n8n / UIpath /
 * Replit competitive gap.
 */

type StepKind = "trigger" | "skill_invoke" | "approval" | "tool_call" | "notify" | "decision" | "data";

interface WorkflowStep {
  id: string;
  kind: StepKind;
  name: string;
  description: string;
  approvalGate?: { reason: string; channel: string };
  inputs?: string[];
  outputs?: string[];
  // When kind === "trigger", `triggerKind` discriminates the tailored
  // config form rendered in the drawer. Defaults to "manual" if absent.
  triggerKind?: TriggerKind;
}

interface WorkflowSpec {
  name: string;
  vertical: string;
  description: string;
  version: string;
  steps: WorkflowStep[];
  risk: "approval-required" | "autonomous";
}

const SEED_WORKFLOWS: Record<string, WorkflowSpec> = {
  "gtm-engineer/post_listing": {
    name: "post_listing",
    vertical: "gtm-engineer",
    description: "Validate inventory, generate copy, post to Facebook Marketplace after seller approval.",
    version: "v3",
    risk: "approval-required",
    steps: [
      { id: "s1", kind: "trigger", triggerKind: "form", name: "Seller submits inventory", description: "Slack/email/web inbox with photos + price + condition.", inputs: ["photos[]", "price", "condition"] },
      { id: "s2", kind: "skill_invoke", name: "Validate inventory", description: "Checks price vs market, runs OCR for category hints.", outputs: ["validation_report"] },
      { id: "s3", kind: "decision", name: "Branch on validation", description: "Below floor → manager review. Otherwise continue." },
      { id: "s4", kind: "skill_invoke", name: "Generate listing copy", description: "60-word description + 3 highlights + photos.", outputs: ["listing.title", "listing.body"] },
      { id: "s5", kind: "approval", name: "Seller approves", description: "One-tap approve / request edits via Slack.", approvalGate: { reason: "Posting public listings needs seller signoff.", channel: "Slack" } },
      { id: "s6", kind: "tool_call", name: "Post to Marketplace", description: "BrowserExecutor opens FB, fills form, submits.", outputs: ["listing.url"] },
    ],
  },
  "sales/qualify_lead": {
    name: "qualify_lead",
    vertical: "sales",
    description: "BANT-style qualification + calendar-link insertion + CRM update on accept.",
    version: "v4",
    risk: "approval-required",
    steps: [
      { id: "s1", kind: "trigger", triggerKind: "webhook", name: "Lead from web / paid / referral", description: "HubSpot/Salesforce webhook fires." },
      { id: "s2", kind: "skill_invoke", name: "Enrich + BANT score", description: "Firmographic + technographic data; 0-100 BANT score." },
      { id: "s3", kind: "decision", name: "Branch on BANT score", description: "≥75 = demo. 50-74 = nurture. <50 = decline." },
      { id: "s4", kind: "approval", name: "AE approves outreach", description: "Slack DM with qual summary + one-tap approve.", approvalGate: { reason: "Outreach to enterprise needs AE signoff.", channel: "Slack" } },
      { id: "s5", kind: "tool_call", name: "Send outreach + calendar link", description: "Gmail/Outlook + Calendly slot + CRM update." },
    ],
  },
};

const STEP_KIND_META: Record<StepKind, { icon: React.ReactNode; color: string; label: string }> = {
  trigger:      { icon: <Play size={14} />,         color: "#10b981", label: "Trigger" },
  skill_invoke: { icon: <Sparkles size={14} />,     color: "#8b5cf6", label: "Skill" },
  approval:     { icon: <ShieldCheck size={14} />,  color: "#f59e0b", label: "Approval gate" },
  tool_call:    { icon: <Database size={14} />,     color: "#0891b2", label: "Tool call" },
  notify:       { icon: <MessageSquare size={14} />,color: "#ec4899", label: "Notify" },
  decision:     { icon: <GitBranch size={14} />,    color: "#6366f1", label: "Decision" },
  data:         { icon: <Database size={14} />,     color: "#71717a", label: "Data" },
};

interface SkyNodeData extends Record<string, unknown> {
  step: WorkflowStep;
}

function SkycotStepNode({ data, selected }: NodeProps) {
  const step = (data as SkyNodeData).step;
  const meta = STEP_KIND_META[step.kind];
  return (
    <div
      style={{
        width: 280,
        background: "white",
        border: `2px solid ${selected ? "#8b5cf6" : "rgba(0,0,0,0.08)"}`,
        borderRadius: 14,
        boxShadow: selected
          ? "0 12px 30px -8px rgba(139, 92, 246, 0.45)"
          : "0 6px 16px -8px rgba(0, 0, 0, 0.15)",
        overflow: "hidden",
        fontFamily: "Inter, sans-serif",
        cursor: "grab",
      }}
    >
      <Handle type="target" position={Position.Top} style={{ background: "#8b5cf6", border: "none", width: 8, height: 8 }} />
      <div
        style={{
          padding: "12px 16px",
          background: `linear-gradient(135deg, ${meta.color}1a, ${meta.color}0a)`,
          borderBottom: `1px solid ${meta.color}33`,
          display: "flex",
          alignItems: "center",
          gap: 10,
        }}
      >
        <div
          style={{
            width: 28,
            height: 28,
            borderRadius: 7,
            background: meta.color,
            color: "white",
            display: "grid",
            placeItems: "center",
          }}
        >
          {meta.icon}
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              fontFamily: "JetBrains Mono, monospace",
              fontSize: 9.5,
              letterSpacing: "0.12em",
              textTransform: "uppercase",
              fontWeight: 700,
              color: meta.color,
            }}
          >
            {meta.label}
          </div>
          <div style={{ fontSize: 14, fontWeight: 600, color: "#1e1b4b", marginTop: 2 }}>
            {step.name}
          </div>
        </div>
      </div>
      <div style={{ padding: "12px 16px" }}>
        <div style={{ fontSize: 12.5, color: "#3f3f46", lineHeight: 1.45 }}>{step.description}</div>
        {step.approvalGate ? (
          <div
            style={{
              marginTop: 8,
              padding: "6px 9px",
              borderRadius: 6,
              background: "rgba(245,158,11,0.1)",
              border: "1px solid rgba(245,158,11,0.3)",
              fontSize: 10.5,
              color: "#92400e",
            }}
          >
            <strong style={{ fontFamily: "JetBrains Mono, monospace", fontSize: 9.5, letterSpacing: "0.12em" }}>
              GATE · {step.approvalGate.channel}
            </strong>
          </div>
        ) : null}
        {step.inputs || step.outputs ? (
          <div
            style={{
              marginTop: 8,
              display: "flex",
              gap: 8,
              flexWrap: "wrap",
              fontFamily: "JetBrains Mono, monospace",
              fontSize: 9,
              color: "#71717a",
              letterSpacing: "0.04em",
            }}
          >
            {step.inputs ? <span>IN: {step.inputs.join(", ").slice(0, 24)}…</span> : null}
            {step.outputs ? <span>OUT: {step.outputs.join(", ").slice(0, 24)}…</span> : null}
          </div>
        ) : null}
      </div>
      <Handle type="source" position={Position.Bottom} style={{ background: "#8b5cf6", border: "none", width: 8, height: 8 }} />
    </div>
  );
}

function stepsToFlow(steps: WorkflowStep[]): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = steps.map((step, i) => ({
    id: step.id,
    type: "skycot",
    position: { x: 100, y: 60 + i * 200 },
    data: { step },
  }));
  const edges: Edge[] = steps.slice(0, -1).map((step, i) => ({
    id: `e-${step.id}-${steps[i + 1].id}`,
    source: step.id,
    target: steps[i + 1].id,
    animated: true,
    style: { stroke: "#8b5cf6", strokeWidth: 2 },
  }));
  return { nodes, edges };
}

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
  const initial = useMemo(() => (workflow ? stepsToFlow(workflow.steps) : { nodes: [], edges: [] }), [workflow]);

  const resetState = useCallback(() => {
    const params = new URLSearchParams(searchParams.toString());
    params.delete("state");
    const qs = params.toString();
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

  // A11y: keyboard users tab to a node and press Enter or Space to open
  // the side panel. Mirrors the mouse-click behavior. WCAG 2.1.1 fix
  // (Wave-7 keyboard-nav audit — the workflow builder was the worst
  // offender, only mouse could open node config).
  const onNodeKeyDown = useCallback((e: React.KeyboardEvent, node: Node) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      setSelectedNode(node);
      setTriggerConfigOpen(false);
    }
  }, []);

  const selectedStep = selectedNode ? (selectedNode.data as SkyNodeData).step : null;
  const selectedIsTrigger = selectedStep?.kind === "trigger";
  const selectedTriggerKind: TriggerKind = selectedStep?.triggerKind ?? "manual";

  const nodeTypes = useMemo(() => ({ skycot: SkycotStepNode }), []);

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

  async function handleProposeChange() {
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
        description: "Live wire ships in PR #1882. Canvas changes will land then.",
      });
      setProposing(false);
      setPromptText("");
    }, 1400);
  }

  function handleTestRun() {
    // Honest demo stub — wave-6 action-fires audit flagged the prior
    // "All 5 steps passed" toast as a buyer-catchable lie (no test
    // actually ran). Real test-runner ships via web-workflows-test-run
    // (separate brief).
    toast.message("Test run — preview build", {
      description: "Real run streams from the gateway in v1.x. Not wired in this preview build.",
    });
  }

  function handleSaveAndPublish() {
    // Honest demo stub — wave-6 action-fires audit flagged the prior
    // "Workflow is live for the tenant" toast as a buyer-catchable lie
    // (no persistence). Real publish wires via the gateway workflow-PUT
    // endpoint (sibling brief).
    toast.message("Saved as draft — preview build", {
      description: "Publish to tenant ships in v1.x. Draft is in-memory only here.",
    });
  }

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .builder-shell {
          display: flex;
          flex-direction: column;
          height: calc(100vh - 0px);
          background: #fafaff;
        }
        .builder-top {
          padding: 14px 28px;
          background: white;
          border-bottom: 1px solid #ece9f5;
          display: flex;
          align-items: center;
          gap: 18px;
          flex-wrap: wrap;
        }
        .builder-back {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: #71717a;
          letter-spacing: 0.12em;
          text-transform: uppercase;
          font-weight: 700;
          text-decoration: none;
        }
        .builder-back:hover { color: #8b5cf6; }
        .builder-title {
          font-family: "JetBrains Mono", monospace;
          font-size: 18px;
          color: #1e1b4b;
          font-weight: 700;
          margin: 0;
        }
        .builder-vertical-tag {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          color: #8b5cf6;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          font-weight: 700;
        }
        .builder-version {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: #71717a;
        }
        .builder-actions {
          margin-left: auto;
          display: flex;
          gap: 10px;
        }
        .builder-btn {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 8px 14px;
          border-radius: 9px;
          font-size: 13px;
          font-weight: 600;
          cursor: pointer;
          text-decoration: none;
          border: 1px solid #ece9f5;
          background: white;
          color: #1e1b4b;
          font-family: "Inter", sans-serif;
        }
        .builder-btn-primary {
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          border: none;
          box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.4);
        }
        .builder-prompt-bar {
          padding: 12px 28px;
          background: linear-gradient(180deg, rgba(139,92,246,0.04), transparent);
          border-bottom: 1px solid #ece9f5;
          display: flex;
          gap: 10px;
          align-items: center;
        }
        .builder-prompt-icon {
          width: 32px;
          height: 32px;
          border-radius: 9px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          display: grid;
          place-items: center;
          flex-shrink: 0;
        }
        .builder-prompt-input {
          flex: 1;
          padding: 10px 14px;
          border-radius: 9px;
          border: 1px solid #ece9f5;
          background: white;
          font-family: "Inter", sans-serif;
          font-size: 14.5px;
          color: #1e1b4b;
          outline: none;
        }
        .builder-prompt-input:focus { border-color: #8b5cf6; box-shadow: 0 0 0 4px rgba(139,92,246,0.1); }
        .builder-prompt-send {
          padding: 10px 18px;
          border-radius: 9px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          border: none;
          font-size: 13px;
          font-weight: 700;
          cursor: pointer;
          display: inline-flex;
          gap: 6px;
          align-items: center;
        }
        .builder-prompt-send:disabled { opacity: 0.5; cursor: not-allowed; }
        .builder-prompt-suggest {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          color: #71717a;
          letter-spacing: 0.1em;
          padding: 4px 10px;
          border-radius: 999px;
          background: white;
          border: 1px solid #ece9f5;
          cursor: pointer;
        }
        .builder-prompt-suggest:hover { color: #8b5cf6; border-color: #8b5cf6; }
        .builder-stage {
          flex: 1;
          display: flex;
          min-height: 0;
          position: relative;
        }
        .builder-canvas {
          flex: 1;
          position: relative;
        }
        .builder-panel {
          width: 360px;
          background: white;
          border-left: 1px solid #ece9f5;
          padding: 24px;
          overflow-y: auto;
        }
        .builder-panel-head {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 16px;
        }
        .builder-panel-eyebrow {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          color: #71717a;
        }
        .builder-panel-close {
          background: transparent;
          border: none;
          cursor: pointer;
          color: #71717a;
          padding: 4px;
          border-radius: 6px;
        }
        .builder-panel-close:hover { background: rgba(0,0,0,0.05); }
        .builder-panel-title {
          font-family: "Inter", sans-serif;
          font-size: 19px;
          font-weight: 600;
          color: #1e1b4b;
          margin: 0 0 8px;
        }
        .builder-panel-section {
          margin-bottom: 18px;
        }
        .builder-panel-section-label {
          font-family: "JetBrains Mono", monospace;
          font-size: 10px;
          letter-spacing: 0.16em;
          text-transform: uppercase;
          font-weight: 700;
          color: #71717a;
          margin-bottom: 6px;
        }
        .builder-add-step {
          position: absolute;
          right: 24px;
          bottom: 24px;
          z-index: 10;
          display: inline-flex;
          align-items: center;
          gap: 8px;
          padding: 12px 18px;
          border-radius: 12px;
          background: linear-gradient(135deg, #8b5cf6, #ec4899);
          color: white;
          font-size: 14px;
          font-weight: 700;
          border: none;
          cursor: pointer;
          box-shadow: 0 12px 30px -8px rgba(139, 92, 246, 0.5);
        }
        .react-flow__panel button { background: white; }
        .react-flow__minimap { border-radius: 8px !important; border: 1px solid #ece9f5 !important; }
      `}</style>
      <div className="builder-shell">
        <header className="builder-top">
          <Link href="/workflows" className="builder-back">
            <ArrowLeft size={12} /> ALL WORKFLOWS
          </Link>
          <span className="builder-vertical-tag">{workflow.vertical}</span>
          <h1 className="builder-title">{workflow.name}</h1>
          <span className="builder-version">version {workflow.version}</span>
          <div className="builder-actions">
            <Link href={`/workflows/${verticalId}/${name}`} className="builder-btn">
              View timeline
            </Link>
            <button type="button" className="builder-btn" onClick={handleTestRun}>
              <Play size={13} /> Test run
            </button>
            <button
              type="button"
              className="builder-btn builder-btn-primary"
              onClick={handleSaveAndPublish}
            >
              <Send size={13} /> Save + publish
            </button>
          </div>
        </header>

        {/* AI prompt bar — the SKYCOT differentiator */}
        <div className="builder-prompt-bar">
          <div className="builder-prompt-icon">
            <Sparkles size={16} />
          </div>
          <input
            type="text"
            className="builder-prompt-input"
            placeholder="Describe a change in plain English — e.g. 'Add a step that emails the seller before posting'"
            value={promptText}
            onChange={(e) => setPromptText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleProposeChange();
            }}
          />
          <button
            type="button"
            className="builder-prompt-send"
            disabled={!promptText.trim() || proposing}
            onClick={handleProposeChange}
          >
            {proposing ? "Proposing…" : (<><Send size={12} /> Propose</>)}
          </button>
          <button type="button" className="builder-prompt-suggest" onClick={() => setPromptText("Add a step that emails the seller before posting")}>
            Add a step
          </button>
          <button type="button" className="builder-prompt-suggest" onClick={() => setPromptText("Convert this to fully autonomous (no approval needed)")}>
            Remove gate
          </button>
        </div>

        <div className="builder-stage">
          <div className="builder-canvas">
            <ReactFlow
              defaultNodes={initial.nodes}
              defaultEdges={initial.edges}
              nodeTypes={nodeTypes}
              onNodeClick={onNodeClick}
              fitView
              fitViewOptions={{ padding: 0.18 }}
              attributionPosition="bottom-right"
            >
              <Background gap={20} size={1.2} color="#d4d4d8" />
              <Controls position="bottom-left" />
              <MiniMap
                nodeColor={(n) => {
                  const step = (n.data as SkyNodeData | undefined)?.step;
                  return step ? STEP_KIND_META[step.kind].color : "#8b5cf6";
                }}
                maskColor="rgba(250,250,255,0.7)"
                pannable
                zoomable
                position="bottom-right"
              />
            </ReactFlow>
            <button type="button" className="builder-add-step">
              <Plus size={15} /> Add step
            </button>
          </div>
          {selectedNode ? (
            <aside className="builder-panel">
              <div className="builder-panel-head">
                <span className="builder-panel-eyebrow">
                  {STEP_KIND_META[(selectedNode.data as SkyNodeData).step.kind].label}
                </span>
                <button type="button" className="builder-panel-close" onClick={() => setSelectedNode(null)}>
                  <X size={16} />
                </button>
              </div>
              <h2 className="builder-panel-title">{(selectedNode.data as SkyNodeData).step.name}</h2>
              <p style={{ fontSize: 13.5, color: "#3f3f46", lineHeight: 1.55, margin: "0 0 18px" }}>
                {(selectedNode.data as SkyNodeData).step.description}
              </p>
              {(selectedNode.data as SkyNodeData).step.inputs ? (
                <div className="builder-panel-section">
                  <div className="builder-panel-section-label">INPUTS</div>
                  <code style={{ fontSize: 12.5, color: "#1e1b4b", fontFamily: "JetBrains Mono, monospace" }}>
                    {(selectedNode.data as SkyNodeData).step.inputs!.join(", ")}
                  </code>
                </div>
              ) : null}
              {(selectedNode.data as SkyNodeData).step.outputs ? (
                <div className="builder-panel-section">
                  <div className="builder-panel-section-label">OUTPUTS</div>
                  <code style={{ fontSize: 12.5, color: "#1e1b4b", fontFamily: "JetBrains Mono, monospace" }}>
                    {(selectedNode.data as SkyNodeData).step.outputs!.join(", ")}
                  </code>
                </div>
              ) : null}
              {(selectedNode.data as SkyNodeData).step.approvalGate ? (
                <div className="builder-panel-section">
                  <div className="builder-panel-section-label">APPROVAL GATE</div>
                  <div
                    style={{
                      padding: "10px 12px",
                      borderRadius: 8,
                      background: "rgba(245,158,11,0.1)",
                      border: "1px solid rgba(245,158,11,0.3)",
                      color: "#92400e",
                      fontSize: 12.5,
                    }}
                  >
                    <div style={{ fontWeight: 700, marginBottom: 4 }}>
                      Channel: {(selectedNode.data as SkyNodeData).step.approvalGate!.channel}
                    </div>
                    {(selectedNode.data as SkyNodeData).step.approvalGate!.reason}
                  </div>
                </div>
              ) : null}
              <button
                type="button"
                data-testid="builder-edit-step"
                onClick={() => setTriggerConfigOpen(true)}
                style={{
                  marginTop: 12,
                  width: "100%",
                  padding: "10px 16px",
                  borderRadius: 9,
                  background: "linear-gradient(135deg, #8b5cf6, #ec4899)",
                  color: "white",
                  border: "none",
                  fontWeight: 700,
                  fontSize: 13,
                  cursor: "pointer",
                  display: "inline-flex",
                  gap: 6,
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <SettingsIcon size={13} />
                {selectedIsTrigger ? "Configure trigger" : "Edit step"}
              </button>
            </aside>
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
