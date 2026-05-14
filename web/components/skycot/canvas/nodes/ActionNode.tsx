"use client";

import { NodeShell, TextareaField, TextField } from "./shared";
import type { PrimitiveProps } from "./types";

export function ActionNode({
  node,
  value,
  errors,
  onLabelChange,
  onChange,
}: PrimitiveProps<"action">) {
  return (
    <NodeShell kind="action" title={node.label}>
      <TextField id="node-label" label="Label" value={node.label} onChange={onLabelChange} />
      <TextareaField
        id="action-instruction"
        label="Instruction"
        value={value.instruction}
        error={errors.instruction}
        onChange={(instruction) => onChange({ ...value, instruction })}
      />
    </NodeShell>
  );
}
