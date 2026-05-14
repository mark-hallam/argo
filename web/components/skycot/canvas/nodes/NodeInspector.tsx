"use client";

import type { Node, NodeKind } from "@/lib/types/skill-ir";
import { ActionNode } from "./ActionNode";
import { AwaitApprovalNode } from "./AwaitApprovalNode";
import { BranchNode } from "./BranchNode";
import { ConditionNode } from "./ConditionNode";
import { FetchNode } from "./FetchNode";
import { LoopNode } from "./LoopNode";
import { SubSkillCallNode } from "./SubSkillCallNode";
import { TransformNode } from "./TransformNode";
import { TryCatchNode } from "./TryCatchNode";
import type { FieldErrors } from "./shared";

export interface NodeInspectorProps {
  node: Node;
  graphNodeIds?: readonly string[];
  onChange: (node: Node) => void;
}

export function NodeInspector({ node, graphNodeIds = [], onChange }: NodeInspectorProps) {
  const errors = validateNodeKind(node.kind, graphNodeIds);
  const onLabelChange = (label: string) => onChange({ ...node, label });
  const onKindChange = (kind: NodeKind) => onChange({ ...node, kind });

  return (
    <div data-testid="node-inspector" className="grid gap-2">
      {renderNode(node, errors, onLabelChange, onKindChange)}
      {Object.values(errors).some(Boolean) ? (
        <p role="alert" className="text-xs text-[var(--destructive)]">
          Fix schema errors before saving this node.
        </p>
      ) : null}
    </div>
  );
}

export function validateNodeKind(kind: NodeKind, graphNodeIds: readonly string[] = []): FieldErrors {
  switch (kind.type) {
    case "action":
      return requireText("instruction", kind.instruction);
    case "condition":
      return requireText("expression", kind.expression);
    case "branch":
      return kind.arms.length && kind.arms.every((arm) => arm.label.trim())
        ? {}
        : { arms: "Add at least one labeled branch arm." };
    case "await-approval":
      return {
        ...requireText("approverRole", kind.approverRole),
        ...(kind.timeoutSeconds !== undefined && kind.timeoutSeconds <= 0
          ? { timeoutSeconds: "Timeout must be greater than zero." }
          : {}),
      };
    case "fetch":
      return {
        ...requireText("target", kind.target),
        ...(kind.method && !/^[A-Z]+$/.test(kind.method)
          ? { method: "Method must be uppercase." }
          : {}),
      };
    case "transform":
      return requireText("template", kind.template);
    case "sub-skill-call":
      return {
        ...requireText("skillId", kind.target.skillId),
        ...requireText("version", kind.target.version),
      };
    case "loop":
      return {
        ...requireText("iterVar", kind.iterVar),
        ...validateIds("body", kind.body, graphNodeIds),
      };
    case "try-catch":
      return {
        ...validateIds("tryBody", kind.tryBody, graphNodeIds),
        ...validateIds("catchBody", kind.catchBody, graphNodeIds),
      };
  }
}

function renderNode(
  node: Node,
  errors: FieldErrors,
  onLabelChange: (label: string) => void,
  onKindChange: (kind: NodeKind) => void,
) {
  switch (node.kind.type) {
    case "action":
      return <ActionNode node={node} value={node.kind} errors={errors} onLabelChange={onLabelChange} onChange={onKindChange} />;
    case "condition":
      return <ConditionNode node={node} value={node.kind} errors={errors} onLabelChange={onLabelChange} onChange={onKindChange} />;
    case "branch":
      return <BranchNode node={node} value={node.kind} errors={errors} onLabelChange={onLabelChange} onChange={onKindChange} />;
    case "await-approval":
      return <AwaitApprovalNode node={node} value={node.kind} errors={errors} onLabelChange={onLabelChange} onChange={onKindChange} />;
    case "fetch":
      return <FetchNode node={node} value={node.kind} errors={errors} onLabelChange={onLabelChange} onChange={onKindChange} />;
    case "transform":
      return <TransformNode node={node} value={node.kind} errors={errors} onLabelChange={onLabelChange} onChange={onKindChange} />;
    case "sub-skill-call":
      return <SubSkillCallNode node={node} value={node.kind} errors={errors} onLabelChange={onLabelChange} onChange={onKindChange} />;
    case "loop":
      return <LoopNode node={node} value={node.kind} errors={errors} onLabelChange={onLabelChange} onChange={onKindChange} />;
    case "try-catch":
      return <TryCatchNode node={node} value={node.kind} errors={errors} onLabelChange={onLabelChange} onChange={onKindChange} />;
  }
}

function requireText(field: string, value: string): FieldErrors {
  return value.trim() ? {} : { [field]: "Required by the L1 Skill IR schema." };
}

function validateIds(field: string, ids: string[], graphNodeIds: readonly string[]): FieldErrors {
  const known = new Set(graphNodeIds);
  const missing = ids.filter((id) => known.size > 0 && !known.has(id));
  return missing.length ? { [field]: `Unknown node id: ${missing.join(", ")}` } : {};
}
