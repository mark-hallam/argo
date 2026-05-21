"use client";
import { Pin, PinOff, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";

interface Props {
  stepId: string;
  stepName: string;
  initialJson: string | null;
  onSave: (jsonText: string) => void;
  onClear: () => void;
  onClose: () => void;
}

const EXAMPLE = `{\n  "customer_id": "cus_demo_1234",\n  "amount_cents": 4995,\n  "currency": "AUD"\n}`;

/**
 * Per-step JSON editor. The brief calls for "Pinned data overrides any
 * live input during test runs" — pinning is owned by useBuilderAnnotations
 * and surfaced through this modal. JSON is validated live (parse error
 * shown inline, Save disabled until parse succeeds) and pretty-printed
 * on save so the stored text round-trips back into the editor cleanly.
 * Unpin is exposed only when a pin already exists.
 */
export function PinDataModal({
  stepId,
  stepName,
  initialJson,
  onSave,
  onClear,
  onClose,
}: Props) {
  const [text, setText] = useState<string>(initialJson ?? EXAMPLE);

  const parseError = useMemo<string | null>(() => {
    const trimmed = text.trim();
    if (trimmed.length === 0) return "JSON is empty.";
    try {
      JSON.parse(trimmed);
      return null;
    } catch (err) {
      return err instanceof Error ? err.message : "Invalid JSON.";
    }
  }, [text]);

  const handleSave = useCallback(() => {
    if (parseError) return;
    try {
      const parsed = JSON.parse(text);
      onSave(JSON.stringify(parsed, null, 2));
    } catch {
      // parseError already shown; defensive only.
    }
  }, [text, parseError, onSave]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  const hasExistingPin = initialJson !== null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby={`pin-data-title-${stepId}`}
      data-testid="pin-data-modal"
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(15, 23, 42, 0.45)",
        display: "grid",
        placeItems: "center",
        zIndex: 30,
        fontFamily: "Inter, sans-serif",
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        style={{
          width: "min(640px, 92vw)",
          maxHeight: "min(80vh, 720px)",
          background: "white",
          borderRadius: 14,
          boxShadow: "0 24px 60px -12px rgba(15,23,42,0.35)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            padding: "16px 20px",
            borderBottom: "1px solid rgba(15,23,42,0.08)",
            display: "flex",
            alignItems: "center",
            gap: 10,
          }}
        >
          <Pin size={16} style={{ color: "#8b5cf6" }} />
          <div style={{ flex: 1, minWidth: 0 }}>
            <div
              id={`pin-data-title-${stepId}`}
              style={{ fontSize: 14, fontWeight: 700, color: "#1e1b4b" }}
            >
              Pin input data
            </div>
            <div
              style={{
                fontSize: 11.5,
                color: "#52525b",
                fontFamily: "var(--font-mono)",
                marginTop: 2,
              }}
            >
              {stepName} · {stepId}
            </div>
          </div>
          <button
            type="button"
            aria-label="Close"
            onClick={onClose}
            style={{
              border: "none",
              background: "transparent",
              color: "#52525b",
              cursor: "pointer",
              padding: 4,
            }}
          >
            <X size={16} />
          </button>
        </div>

        <div style={{ padding: "14px 20px", flex: 1, overflow: "auto" }}>
          <p style={{ fontSize: 12.5, color: "#3f3f46", margin: "0 0 10px", lineHeight: 1.5 }}>
            Pinned JSON replaces the live input for this step during test runs.
            Useful for reproducing a tricky fixture without re-firing the trigger.
          </p>
          <textarea
            data-testid="pin-data-textarea"
            aria-label="Pinned JSON for this step"
            value={text}
            onChange={(e) => setText(e.target.value)}
            spellCheck={false}
            style={{
              width: "100%",
              minHeight: 240,
              padding: "10px 12px",
              borderRadius: 8,
              border: parseError
                ? "1px solid rgba(244,63,94,0.45)"
                : "1px solid rgba(15,23,42,0.12)",
              fontFamily: "var(--font-mono), monospace",
              fontSize: 12.5,
              lineHeight: 1.5,
              resize: "vertical",
              color: "#0f172a",
              background: parseError ? "rgba(254, 226, 226, 0.35)" : "#f8fafc",
            }}
          />
          {parseError ? (
            <div
              data-testid="pin-data-error"
              role="alert"
              style={{
                marginTop: 8,
                padding: "6px 9px",
                borderRadius: 6,
                background: "rgba(244,63,94,0.08)",
                border: "1px solid rgba(244,63,94,0.3)",
                color: "#9f1239",
                fontSize: 11.5,
                fontFamily: "var(--font-mono)",
              }}
            >
              {parseError}
            </div>
          ) : (
            <div
              style={{
                marginTop: 8,
                fontSize: 11,
                color: "#15803d",
                fontFamily: "var(--font-mono)",
              }}
            >
              ✓ Valid JSON
            </div>
          )}
        </div>

        <div
          style={{
            padding: "12px 20px",
            borderTop: "1px solid rgba(15,23,42,0.08)",
            display: "flex",
            gap: 10,
            justifyContent: "flex-end",
            alignItems: "center",
          }}
        >
          <button
            type="button"
            data-testid="pin-data-unpin"
            onClick={() => {
              onClear();
              onClose();
            }}
            disabled={!hasExistingPin}
            style={{
              marginRight: "auto",
              padding: "8px 12px",
              borderRadius: 8,
              background: "transparent",
              color: hasExistingPin ? "#9f1239" : "rgba(15,23,42,0.3)",
              border: "1px solid",
              borderColor: hasExistingPin
                ? "rgba(244,63,94,0.35)"
                : "rgba(15,23,42,0.1)",
              cursor: hasExistingPin ? "pointer" : "not-allowed",
              fontSize: 12.5,
              fontWeight: 600,
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
            }}
          >
            <PinOff size={13} /> Unpin
          </button>
          <button
            type="button"
            onClick={onClose}
            style={{
              padding: "8px 14px",
              borderRadius: 8,
              background: "white",
              color: "#1e1b4b",
              border: "1px solid rgba(15,23,42,0.12)",
              cursor: "pointer",
              fontSize: 12.5,
              fontWeight: 600,
            }}
          >
            Cancel
          </button>
          <button
            type="button"
            data-testid="pin-data-save"
            onClick={handleSave}
            disabled={!!parseError}
            style={{
              padding: "8px 14px",
              borderRadius: 8,
              background: parseError
                ? "rgba(139,92,246,0.4)"
                : "linear-gradient(135deg, #8b5cf6, #ec4899)",
              color: "white",
              border: "none",
              cursor: parseError ? "not-allowed" : "pointer",
              fontSize: 12.5,
              fontWeight: 700,
            }}
          >
            Save pin
          </button>
        </div>
      </div>
    </div>
  );
}
