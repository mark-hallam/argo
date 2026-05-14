"use client";

import { IdListField, NodeShell, TextField } from "./shared";
import type { PrimitiveProps } from "./types";

export function LoopNode({
  node,
  value,
  errors,
  onLabelChange,
  onChange,
}: PrimitiveProps<"loop">) {
  return (
    <NodeShell kind="loop" title={node.label}>
      <TextField id="node-label" label="Label" value={node.label} onChange={onLabelChange} />
      <TextField
        id="loop-iter-var"
        label="Iterator variable"
        value={value.iterVar}
        error={errors.iterVar}
        onChange={(iterVar) => onChange({ ...value, iterVar })}
      />
      <IdListField
        id="loop-body"
        label="Body node IDs"
        value={value.body}
        error={errors.body}
        onChange={(body) => onChange({ ...value, body })}
      />
    </NodeShell>
  );
}
