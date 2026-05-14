"use client";

import type { ReactNode } from "react";
import { KIND_VISUALS, type NodeKindTag } from "../NodeRenderer";

export type FieldErrors = Record<string, string | undefined>;

export interface FieldProps {
  id: string;
  label: string;
  value: string;
  error?: string;
  placeholder?: string;
  onChange: (value: string) => void;
}

export function NodeShell({
  kind,
  title,
  children,
}: {
  kind: NodeKindTag;
  title: string;
  children: ReactNode;
}) {
  const visual = KIND_VISUALS[kind];
  const { Icon } = visual;

  return (
    <section
      data-testid={`${kind}-node-primitive`}
      className={`rounded-[var(--radius-md)] border p-3 ${visual.bodyClass} ${visual.borderClass}`}
    >
      <div className="mb-3 flex items-center gap-2">
        <Icon className={`size-4 ${visual.iconClass}`} aria-hidden="true" />
        <div>
          <div className="text-[10px] uppercase text-[var(--muted-foreground)]">
            {visual.label}
          </div>
          <div className="text-sm font-medium text-[var(--foreground)]">{title}</div>
        </div>
      </div>
      <div className="grid gap-2">{children}</div>
    </section>
  );
}

export function TextField({
  id,
  label,
  value,
  error,
  placeholder,
  onChange,
}: FieldProps) {
  return (
    <label className="grid gap-1 text-xs text-[var(--muted-foreground)]">
      {label}
      <input
        data-testid={id}
        value={value}
        placeholder={placeholder}
        aria-invalid={error ? "true" : undefined}
        onChange={(event) => onChange(event.target.value)}
        className="rounded border border-[var(--border)] bg-[var(--background)] px-2 py-1 text-sm text-[var(--foreground)]"
      />
      {error ? <span className="text-[var(--destructive)]">{error}</span> : null}
    </label>
  );
}

export function TextareaField(props: FieldProps) {
  return (
    <label className="grid gap-1 text-xs text-[var(--muted-foreground)]">
      {props.label}
      <textarea
        data-testid={props.id}
        value={props.value}
        placeholder={props.placeholder}
        aria-invalid={props.error ? "true" : undefined}
        onChange={(event) => props.onChange(event.target.value)}
        className="min-h-20 rounded border border-[var(--border)] bg-[var(--background)] px-2 py-1 text-sm text-[var(--foreground)]"
      />
      {props.error ? <span className="text-[var(--destructive)]">{props.error}</span> : null}
    </label>
  );
}

export function IdListField({
  id,
  label,
  value,
  error,
  onChange,
}: Omit<FieldProps, "value" | "onChange"> & {
  value: string[];
  onChange: (value: string[]) => void;
}) {
  return (
    <TextField
      id={id}
      label={label}
      value={value.join(", ")}
      error={error}
      placeholder="node_a, node_b"
      onChange={(raw) =>
        onChange(raw.split(",").map((item) => item.trim()).filter(Boolean))
      }
    />
  );
}
