"use client";

import type { Node, NodeKind } from "@/lib/types/skill-ir";
import type { FieldErrors } from "./shared";

export type KindOf<T extends NodeKind["type"]> = Extract<NodeKind, { type: T }>;

export interface PrimitiveProps<T extends NodeKind["type"]> {
  node: Node;
  value: KindOf<T>;
  errors: FieldErrors;
  onLabelChange: (label: string) => void;
  onChange: (kind: KindOf<T>) => void;
}
