"use client";

import { useState } from "react";
import { ArrowUp, ArrowDown, Trash2, Plus, Copy, Check } from "lucide-react";

import {
  formConfigSchema,
  formUrl,
  FORM_FIELD_TYPES,
  type FormConfig,
  type FormFieldDef,
  type FormFieldType,
} from "./types";
import {
  TextField,
  TextareaField,
  FieldShell,
  PillButton,
  SecondaryButton,
  triggerStyles,
} from "./shared";

interface Props {
  config: FormConfig;
  onChange: (next: FormConfig) => void;
}

const TYPE_LABEL: Record<FormFieldType, string> = {
  text: "Text",
  textarea: "Long text",
  email: "Email",
  number: "Number",
  date: "Date",
  select: "Select",
  checkbox: "Checkbox",
  file: "File",
};

export default function FormConfigForm({ config, onChange }: Props) {
  const [copied, setCopied] = useState(false);
  const parsed = formConfigSchema.safeParse(config);
  const errors = parsed.success ? {} : flattenErrors(parsed.error.issues);
  const url = formUrl(config);

  function patch(p: Partial<FormConfig>) {
    onChange({ ...config, ...p });
  }

  function addField() {
    const id = `f${Date.now()}`;
    const next: FormFieldDef = {
      id,
      label: "New field",
      type: "text",
      required: false,
      placeholder: "",
    };
    patch({ fields: [...config.fields, next] });
  }

  function removeField(id: string) {
    patch({ fields: config.fields.filter((f) => f.id !== id) });
  }

  function patchField(id: string, p: Partial<FormFieldDef>) {
    patch({ fields: config.fields.map((f) => (f.id === id ? { ...f, ...p } : f)) });
  }

  function moveField(id: string, dir: -1 | 1) {
    const idx = config.fields.findIndex((f) => f.id === id);
    if (idx < 0) return;
    const target = idx + dir;
    if (target < 0 || target >= config.fields.length) return;
    const next = config.fields.slice();
    const [moved] = next.splice(idx, 1);
    next.splice(target, 0, moved);
    patch({ fields: next });
  }

  async function handleCopyUrl() {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // noop
    }
  }

  return (
    <div data-testid="form-config-form">
      <FieldShell label="Form URL" hint="Send this URL to the people you want to capture data from.">
        <div style={{ display: "flex", gap: 8 }}>
          <input
            readOnly
            value={url}
            data-testid="form-url"
            style={{ ...triggerStyles.input, background: "var(--background)", fontFamily: "JetBrains Mono, monospace", fontSize: 12.5 }}
          />
          <SecondaryButton onClick={handleCopyUrl} testId="form-url-copy">
            {copied ? <Check size={13} /> : <Copy size={13} />} {copied ? "Copied" : "Copy"}
          </SecondaryButton>
        </div>
      </FieldShell>

      <FieldShell label="Visibility">
        <div style={{ display: "flex", gap: 6 }}>
          <PillButton active={config.visibility === "public"} onClick={() => patch({ visibility: "public" })}>
            Public — anyone with the URL
          </PillButton>
          <PillButton active={config.visibility === "private"} onClick={() => patch({ visibility: "private" })}>
            Private — SKYCOT login required
          </PillButton>
        </div>
      </FieldShell>

      <FieldShell label="Fields" error={errors.fields}>
        <div data-testid="form-field-list">
          {config.fields.map((field, idx) => (
            <FieldRow
              key={field.id}
              field={field}
              isFirst={idx === 0}
              isLast={idx === config.fields.length - 1}
              onChange={(p) => patchField(field.id, p)}
              onRemove={() => removeField(field.id)}
              onMoveUp={() => moveField(field.id, -1)}
              onMoveDown={() => moveField(field.id, 1)}
            />
          ))}
        </div>
        <button
          type="button"
          onClick={addField}
          data-testid="form-add-field"
          style={{
            marginTop: 10,
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            padding: "8px 14px",
            borderRadius: 9,
            background: "rgba(139,92,246,0.08)",
            color: "#5b21b6",
            border: "1px dashed var(--primary)",
            fontSize: 13,
            fontWeight: 600,
            cursor: "pointer",
          }}
        >
          <Plus size={13} /> Add field
        </button>
      </FieldShell>

      <TextField
        id="form-submit-label"
        label="Submit button label"
        value={config.submitLabel}
        onChange={(v) => patch({ submitLabel: v })}
        placeholder="Submit"
        error={errors.submitLabel}
      />

      <TextareaField
        id="form-success-message"
        label="Success message"
        value={config.successMessage}
        onChange={(v) => patch({ successMessage: v })}
        placeholder="Thanks! We'll be in touch."
        rows={2}
        error={errors.successMessage}
      />
    </div>
  );
}

interface FieldRowProps {
  field: FormFieldDef;
  isFirst: boolean;
  isLast: boolean;
  onChange: (p: Partial<FormFieldDef>) => void;
  onRemove: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}

function FieldRow({ field, isFirst, isLast, onChange, onRemove, onMoveUp, onMoveDown }: FieldRowProps) {
  return (
    <div
      data-testid={`form-field-row-${field.id}`}
      style={{
        padding: "12px 12px 6px",
        marginBottom: 10,
        borderRadius: 10,
        border: "1px solid #ece9f5",
        background: "var(--background)",
      }}
    >
      <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
        <input
          aria-label="Field label"
          value={field.label}
          onChange={(e) => onChange({ label: e.target.value })}
          placeholder="Field label"
          style={{ ...triggerStyles.input, flex: 2 }}
        />
        <select
          aria-label="Field type"
          value={field.type}
          onChange={(e) => onChange({ type: e.target.value as FormFieldType })}
          style={{ ...triggerStyles.input, flex: 1 }}
        >
          {FORM_FIELD_TYPES.map((t) => (
            <option key={t} value={t}>{TYPE_LABEL[t]}</option>
          ))}
        </select>
      </div>
      <input
        aria-label="Placeholder"
        value={field.placeholder ?? ""}
        onChange={(e) => onChange({ placeholder: e.target.value })}
        placeholder="Placeholder text (optional)"
        style={{ ...triggerStyles.input, marginBottom: 8 }}
      />
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <label style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 12.5, color: "#3f3f46", cursor: "pointer" }}>
          <input
            type="checkbox"
            checked={field.required}
            onChange={(e) => onChange({ required: e.target.checked })}
          />
          Required
        </label>
        <div style={{ marginLeft: "auto", display: "flex", gap: 4 }}>
          <IconButton aria-label="Move up" disabled={isFirst} onClick={onMoveUp}>
            <ArrowUp size={13} />
          </IconButton>
          <IconButton aria-label="Move down" disabled={isLast} onClick={onMoveDown}>
            <ArrowDown size={13} />
          </IconButton>
          <IconButton aria-label="Delete field" tone="danger" onClick={onRemove}>
            <Trash2 size={13} />
          </IconButton>
        </div>
      </div>
    </div>
  );
}

interface IconButtonProps {
  ["aria-label"]: string;
  disabled?: boolean;
  onClick: () => void;
  children: React.ReactNode;
  tone?: "default" | "danger";
}

function IconButton({ "aria-label": ariaLabel, disabled, onClick, children, tone = "default" }: IconButtonProps) {
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      onClick={onClick}
      disabled={disabled}
      style={{
        width: 30,
        height: 30,
        borderRadius: 7,
        border: "1px solid #ece9f5",
        background: tone === "danger" ? "#fee2e2" : "white",
        color: tone === "danger" ? "#b91c1c" : "#3f3f46",
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        cursor: disabled ? "not-allowed" : "pointer",
        opacity: disabled ? 0.4 : 1,
      }}
    >
      {children}
    </button>
  );
}

function flattenErrors(issues: Array<{ path: (string | number)[]; message: string }>): Record<string, string> {
  const out: Record<string, string> = {};
  for (const issue of issues) {
    const key = issue.path.join(".");
    if (!out[key]) out[key] = issue.message;
  }
  return out;
}
