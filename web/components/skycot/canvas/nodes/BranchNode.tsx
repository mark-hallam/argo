"use client";

import type { BranchArm } from "@/lib/types/skill-ir";
import { NodeShell, TextareaField, TextField } from "./shared";
import type { PrimitiveProps } from "./types";

export function BranchNode({
  node,
  value,
  errors,
  onLabelChange,
  onChange,
}: PrimitiveProps<"branch">) {
  return (
    <NodeShell kind="branch" title={node.label}>
      <TextField id="node-label" label="Label" value={node.label} onChange={onLabelChange} />
      <TextareaField
        id="branch-arms"
        label="Arms"
        value={formatArms(value.arms)}
        error={errors.arms}
        placeholder="approved: status == 'approved'"
        onChange={(raw) => onChange({ ...value, arms: parseArms(raw) })}
      />
    </NodeShell>
  );
}

function formatArms(arms: BranchArm[]) {
  return arms.map((arm) => `${arm.label}${arm.pattern ? `: ${arm.pattern}` : ""}`).join("\n");
}

function parseArms(raw: string): BranchArm[] {
  return raw
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const [label, ...pattern] = line.split(":");
      return {
        label: label.trim(),
        pattern: pattern.join(":").trim() || undefined,
      };
    });
}
