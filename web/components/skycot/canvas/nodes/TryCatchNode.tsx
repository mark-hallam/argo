"use client";

import { IdListField, NodeShell, TextField } from "./shared";
import type { PrimitiveProps } from "./types";

export function TryCatchNode({
  node,
  value,
  errors,
  onLabelChange,
  onChange,
}: PrimitiveProps<"try-catch">) {
  return (
    <NodeShell kind="try-catch" title={node.label}>
      <TextField id="node-label" label="Label" value={node.label} onChange={onLabelChange} />
      <IdListField
        id="try-body"
        label="Try body node IDs"
        value={value.tryBody}
        error={errors.tryBody}
        onChange={(tryBody) => onChange({ ...value, tryBody })}
      />
      <IdListField
        id="catch-body"
        label="Catch body node IDs"
        value={value.catchBody}
        error={errors.catchBody}
        onChange={(catchBody) => onChange({ ...value, catchBody })}
      />
    </NodeShell>
  );
}
