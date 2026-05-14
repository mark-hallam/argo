"use client";

import { NodeShell, TextareaField, TextField } from "./shared";
import type { PrimitiveProps } from "./types";

export function ConditionNode({
  node,
  value,
  errors,
  onLabelChange,
  onChange,
}: PrimitiveProps<"condition">) {
  return (
    <NodeShell kind="condition" title={node.label}>
      <TextField id="node-label" label="Label" value={node.label} onChange={onLabelChange} />
      <TextareaField
        id="condition-expression"
        label="Expression"
        value={value.expression}
        error={errors.expression}
        placeholder="amount <= 500"
        onChange={(expression) => onChange({ ...value, expression })}
      />
    </NodeShell>
  );
}
