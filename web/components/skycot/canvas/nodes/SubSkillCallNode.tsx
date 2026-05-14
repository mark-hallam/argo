"use client";

import { NodeShell, TextField } from "./shared";
import type { PrimitiveProps } from "./types";

export function SubSkillCallNode({
  node,
  value,
  errors,
  onLabelChange,
  onChange,
}: PrimitiveProps<"sub-skill-call">) {
  return (
    <NodeShell kind="sub-skill-call" title={node.label}>
      <TextField id="node-label" label="Label" value={node.label} onChange={onLabelChange} />
      <TextField
        id="sub-skill-id"
        label="Skill ID"
        value={value.target.skillId}
        error={errors.skillId}
        onChange={(skillId) => onChange({ ...value, target: { ...value.target, skillId } })}
      />
      <TextField
        id="sub-skill-version"
        label="Version"
        value={value.target.version}
        error={errors.version}
        onChange={(version) => onChange({ ...value, target: { ...value.target, version } })}
      />
    </NodeShell>
  );
}
