import { useCallback, useEffect, useState } from "react";
import { decide, fetchPending, ApiError } from "../api";
import type { ApprovalRow, AppSettings } from "../types";

interface Props {
  settings: AppSettings;
  highlightApprovalId: string | null;
  onOpenSettings: () => void;
}

const POLL_MS = 5000;

export function ApprovalsList({ settings, highlightApprovalId, onOpenSettings }: Props) {
  const [rows, setRows] = useState<ApprovalRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!settings.bearer_token) {
      setError("Set a bearer token in Settings first.");
      return;
    }
    try {
      const next = await fetchPending(settings.gateway_url, settings.bearer_token);
      setRows(next);
      setError(null);
    } catch (e) {
      const msg = e instanceof ApiError ? e.message : (e as Error).message;
      setError(msg);
    }
  }, [settings.bearer_token, settings.gateway_url]);

  useEffect(() => {
    refresh();
    const id = window.setInterval(refresh, POLL_MS);
    return () => window.clearInterval(id);
  }, [refresh]);

  const onDecide = useCallback(
    async (row: ApprovalRow, decision: "approve" | "reject") => {
      setBusy(row.approval_id);
      try {
        await decide(settings.gateway_url, settings.bearer_token, row.approval_id, decision);
        setRows((prev) => prev.filter((r) => r.approval_id !== row.approval_id));
      } catch (e) {
        setError(e instanceof ApiError ? e.message : (e as Error).message);
      } finally {
        setBusy(null);
      }
    },
    [settings.bearer_token, settings.gateway_url],
  );

  return (
    <div className="app">
      <header>
        <h1>Pending approvals</h1>
        <button className="link" onClick={onOpenSettings} aria-label="Settings">
          Settings
        </button>
      </header>
      <main>
        {error ? <div className="error">{error}</div> : null}
        {rows.length === 0 ? (
          <p className="empty">Nothing to approve right now.</p>
        ) : (
          rows.map((row) => (
            <ApprovalCard
              key={row.approval_id}
              row={row}
              busy={busy === row.approval_id}
              highlighted={row.approval_id === highlightApprovalId}
              onApprove={() => onDecide(row, "approve")}
              onReject={() => onDecide(row, "reject")}
            />
          ))
        )}
      </main>
    </div>
  );
}

interface CardProps {
  row: ApprovalRow;
  busy: boolean;
  highlighted: boolean;
  onApprove: () => void;
  onReject: () => void;
}

function ApprovalCard({ row, busy, highlighted, onApprove, onReject }: CardProps) {
  const riskClass = riskClassFor(row.risk_level);
  const cardStyle: React.CSSProperties | undefined = highlighted
    ? { outline: "2px solid var(--primary)" }
    : undefined;
  return (
    <section className="card" style={cardStyle} data-approval-id={row.approval_id}>
      <div className="meta">
        <span>action {row.action_id || row.approval_id}</span>
        <span className={riskClass}>risk {row.risk_level || "unknown"}</span>
      </div>
      <p>{row.prompt_body || "(no payload preview from inbox snapshot)"}</p>
      <div className="actions">
        <button className="primary" onClick={onApprove} disabled={busy}>
          {busy ? "…" : "Approve"}
        </button>
        <button className="danger" onClick={onReject} disabled={busy}>
          {busy ? "…" : "Deny"}
        </button>
      </div>
    </section>
  );
}

function riskClassFor(risk: string): string {
  const lower = risk.toLowerCase();
  if (lower.includes("high")) return "risk-high";
  if (lower.includes("low")) return "risk-low";
  return "risk-medium";
}
