/**
 * Shared field primitives for trigger config forms. Match the
 * builder's existing visual register (Inter body, JetBrains Mono
 * eyebrows, violet/pink gradient accents).
 */
"use client";

import type { ReactNode, ChangeEvent } from "react";

const ROW = { marginBottom: 16 } as const;

const LABEL = {
  display: "block",
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  letterSpacing: "0.16em",
  textTransform: "uppercase" as const,
  fontWeight: 700,
  color: "var(--muted-foreground)",
  marginBottom: 6,
} as const;

const INPUT = {
  width: "100%",
  padding: "9px 12px",
  borderRadius: 8,
  border: "1px solid #ece9f5",
  background: "white",
  fontFamily: "Inter, sans-serif",
  fontSize: 14,
  color: "var(--foreground)",
  outline: "none",
  boxSizing: "border-box" as const,
} as const;

const ERROR_TEXT = {
  marginTop: 6,
  fontSize: 12,
  color: "#dc2626",
} as const;

const HINT_TEXT = {
  marginTop: 6,
  fontSize: 12,
  color: "var(--muted-foreground)",
} as const;

interface FieldShellProps {
  label: string;
  htmlFor?: string;
  error?: string;
  hint?: ReactNode;
  children: ReactNode;
}

export function FieldShell({ label, htmlFor, error, hint, children }: FieldShellProps) {
  return (
    <div style={ROW}>
      <label htmlFor={htmlFor} style={LABEL}>{label}</label>
      {children}
      {error ? <div role="alert" style={ERROR_TEXT}>{error}</div> : null}
      {!error && hint ? <div style={HINT_TEXT}>{hint}</div> : null}
    </div>
  );
}

interface TextFieldProps {
  id: string;
  label: string;
  value: string;
  onChange: (next: string) => void;
  placeholder?: string;
  type?: "text" | "password" | "email";
  hint?: ReactNode;
  error?: string;
  readOnly?: boolean;
}

export function TextField({ id, label, value, onChange, placeholder, type = "text", hint, error, readOnly }: TextFieldProps) {
  return (
    <FieldShell label={label} htmlFor={id} error={error} hint={hint}>
      <input
        id={id}
        type={type}
        value={value}
        readOnly={readOnly}
        placeholder={placeholder}
        onChange={(e: ChangeEvent<HTMLInputElement>) => onChange(e.target.value)}
        style={{ ...INPUT, background: readOnly ? "var(--background)" : "white" }}
      />
    </FieldShell>
  );
}

interface NumberFieldProps {
  id: string;
  label: string;
  value: number;
  min?: number;
  max?: number;
  onChange: (next: number) => void;
  hint?: ReactNode;
  error?: string;
}

export function NumberField({ id, label, value, min, max, onChange, hint, error }: NumberFieldProps) {
  return (
    <FieldShell label={label} htmlFor={id} error={error} hint={hint}>
      <input
        id={id}
        type="number"
        value={value}
        min={min}
        max={max}
        onChange={(e: ChangeEvent<HTMLInputElement>) => {
          const parsed = e.target.value === "" ? Number.NaN : Number(e.target.value);
          onChange(parsed);
        }}
        style={INPUT}
      />
    </FieldShell>
  );
}

interface TextareaFieldProps {
  id: string;
  label: string;
  value: string;
  onChange: (next: string) => void;
  rows?: number;
  placeholder?: string;
  hint?: ReactNode;
  error?: string;
}

export function TextareaField({ id, label, value, onChange, rows = 3, placeholder, hint, error }: TextareaFieldProps) {
  return (
    <FieldShell label={label} htmlFor={id} error={error} hint={hint}>
      <textarea
        id={id}
        value={value}
        rows={rows}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        style={{ ...INPUT, fontFamily: "Inter, sans-serif", resize: "vertical" }}
      />
    </FieldShell>
  );
}

interface ToggleFieldProps {
  id: string;
  label: string;
  checked: boolean;
  onChange: (next: boolean) => void;
  hint?: ReactNode;
}

export function ToggleField({ id, label, checked, onChange, hint }: ToggleFieldProps) {
  return (
    <div style={{ ...ROW, display: "flex", alignItems: "flex-start", gap: 10 }}>
      <input
        id={id}
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        style={{ marginTop: 4, width: 16, height: 16, cursor: "pointer" }}
      />
      <div style={{ flex: 1 }}>
        <label htmlFor={id} style={{ fontFamily: "Inter, sans-serif", fontSize: 13.5, color: "var(--foreground)", fontWeight: 600 }}>
          {label}
        </label>
        {hint ? <div style={HINT_TEXT}>{hint}</div> : null}
      </div>
    </div>
  );
}

export const triggerStyles = {
  row: ROW,
  label: LABEL,
  input: INPUT,
  errorText: ERROR_TEXT,
  hintText: HINT_TEXT,
};

interface PillButtonProps {
  id?: string;
  active: boolean;
  onClick: () => void;
  children: ReactNode;
  tone?: "default" | "danger";
}

export function PillButton({ id, active, onClick, children, tone = "default" }: PillButtonProps) {
  const dangerActive = tone === "danger" && active;
  return (
    <button
      id={id}
      type="button"
      onClick={onClick}
      style={{
        padding: "6px 12px",
        borderRadius: 999,
        border: active ? "1px solid var(--primary)" : "1px solid #ece9f5",
        background: dangerActive ? "#fee2e2" : active ? "rgba(139,92,246,0.1)" : "white",
        color: dangerActive ? "#b91c1c" : active ? "#5b21b6" : "#3f3f46",
        fontFamily: "Inter, sans-serif",
        fontSize: 12.5,
        fontWeight: 600,
        cursor: "pointer",
      }}
    >
      {children}
    </button>
  );
}

interface PrimaryButtonProps {
  onClick: () => void;
  children: ReactNode;
  disabled?: boolean;
  testId?: string;
}

export function PrimaryButton({ onClick, children, disabled, testId }: PrimaryButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      data-testid={testId}
      style={{
        padding: "9px 16px",
        borderRadius: 9,
        background: disabled ? "var(--border)" : "linear-gradient(135deg, var(--primary), var(--gradient-end))",
        color: "white",
        border: "none",
        fontFamily: "Inter, sans-serif",
        fontSize: 13,
        fontWeight: 700,
        cursor: disabled ? "not-allowed" : "pointer",
        opacity: disabled ? 0.7 : 1,
      }}
    >
      {children}
    </button>
  );
}

interface SecondaryButtonProps {
  onClick: () => void;
  children: ReactNode;
  disabled?: boolean;
  testId?: string;
}

export function SecondaryButton({ onClick, children, disabled, testId }: SecondaryButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      data-testid={testId}
      style={{
        padding: "9px 14px",
        borderRadius: 9,
        background: "white",
        color: "var(--foreground)",
        border: "1px solid #ece9f5",
        fontFamily: "Inter, sans-serif",
        fontSize: 13,
        fontWeight: 600,
        cursor: disabled ? "not-allowed" : "pointer",
        opacity: disabled ? 0.5 : 1,
      }}
    >
      {children}
    </button>
  );
}
