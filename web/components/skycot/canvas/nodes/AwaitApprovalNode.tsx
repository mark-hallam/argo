"use client";

import { NodeShell, TextField } from "./shared";
import type { PrimitiveProps } from "./types";

export function AwaitApprovalNode({
  node,
  value,
  errors,
  onLabelChange,
  onChange,
}: PrimitiveProps<"await-approval">) {
  return (
    <NodeShell kind="await-approval" title={node.label}>
      <TextField id="node-label" label="Label" value={node.label} onChange={onLabelChange} />
      <TextField
        id="approval-role"
        label="Approver role"
        value={value.approverRole}
        error={errors.approverRole}
        onChange={(approverRole) => onChange({ ...value, approverRole })}
      />
      <TextField
        id="approval-timeout"
        label="Timeout seconds"
        value={value.timeoutSeconds?.toString() ?? ""}
        error={errors.timeoutSeconds}
        placeholder="3600"
        onChange={(raw) =>
          onChange({ ...value, timeoutSeconds: raw ? Number(raw) : undefined })
        }
      />
    </NodeShell>
  );
}
