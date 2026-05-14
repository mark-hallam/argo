"use client";

import { NodeShell, TextField } from "./shared";
import type { PrimitiveProps } from "./types";

export function FetchNode({
  node,
  value,
  errors,
  onLabelChange,
  onChange,
}: PrimitiveProps<"fetch">) {
  return (
    <NodeShell kind="fetch" title={node.label}>
      <TextField id="node-label" label="Label" value={node.label} onChange={onLabelChange} />
      <TextField
        id="fetch-target"
        label="Target"
        value={value.target}
        error={errors.target}
        placeholder="mcp://server/tool"
        onChange={(target) => onChange({ ...value, target })}
      />
      <TextField
        id="fetch-method"
        label="Method"
        value={value.method ?? ""}
        error={errors.method}
        placeholder="GET"
        onChange={(method) => onChange({ ...value, method: method || undefined })}
      />
    </NodeShell>
  );
}
