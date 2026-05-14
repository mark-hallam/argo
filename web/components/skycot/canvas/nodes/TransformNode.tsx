"use client";

import { NodeShell, TextareaField, TextField } from "./shared";
import type { PrimitiveProps } from "./types";

export function TransformNode({
  node,
  value,
  errors,
  onLabelChange,
  onChange,
}: PrimitiveProps<"transform">) {
  return (
    <NodeShell kind="transform" title={node.label}>
      <TextField id="node-label" label="Label" value={node.label} onChange={onLabelChange} />
      <TextareaField
        id="transform-template"
        label="Template"
        value={value.template}
        error={errors.template}
        placeholder="{{input.name}}"
        onChange={(template) => onChange({ ...value, template })}
      />
    </NodeShell>
  );
}
