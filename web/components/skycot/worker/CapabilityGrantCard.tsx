"use client";

import { useCallback, useMemo, useState } from "react";
import { ShieldCheck } from "lucide-react";
import type {
  CapabilityGrantRequest,
  GrantAccess,
  GrantSelection,
} from "@/lib/types/capability-grant";
import {
  CapabilityGrantFlowMap,
  type FlowMapRow,
} from "@/components/skycot/worker/CapabilityGrantFlowMap";

/**
 * Capability-grant / connect-on-demand card — locked `G2 · Elevated`
 * design (design-explorations/2026-06-11/capability-grant, operator
 * locked 2026-06-11; see [[project_capability_grant_design_locked]]).
 *
 * Surfaced IN the teammate's conversation the moment they need a
 * connector they don't yet have. Leads with a one-line plain-language
 * ask, a left-to-right flow map (teammate ◂ connector ▸ scopes —
 * read = teal arrow IN, write = purple→pink arrow BOTH ways, animated),
 * and a per-scope read/write segmented toggle, then Connect / Not now.
 *
 * Purely presentational: NO data fetching here. The host
 * (`WorkerOverviewContent`) passes the pending `request`, wires
 * `onConnect` to the connector-authorize path, and owns the busy/error
 * surface. Toggling a scope live-updates the matching flow-map edge.
 *
 * Brand: warm gradient-bordered card, Audiowide eyebrow, JetBrains-Mono
 * map labels, purple→pink gradient + teal=read — all from globals.css
 * tokens (`--brand-gradient`, `--brand-primary`, `--brand-pink`,
 * `--brand-teal`, `--font-audiowide`, `--font-mono`). No purple→cyan.
 */

export interface CapabilityGrantCardProps {
  /** The pending connect-on-demand ask (>= 1 scope). */
  request: CapabilityGrantRequest;
  /** Connect with the operator's chosen per-scope access. */
  onConnect: (scopes: GrantSelection[]) => void;
  /** Dismiss the ask ("Not now"). */
  onDismiss: () => void;
  /** Disables the buttons + shows progress while the connect round-trips. */
  busy?: boolean;
  className?: string;
}

export function CapabilityGrantCard({
  request,
  onConnect,
  onDismiss,
  busy = false,
  className,
}: CapabilityGrantCardProps) {
  const { connector, teammateName, askLine, scopes } = request;

  // Per-scope access lives here so toggling live-updates both the row
  // and its flow-map edge. Seeded from each scope's requested `access`.
  const [access, setAccess] = useState<Record<string, GrantAccess>>(() =>
    Object.fromEntries(scopes.map((s) => [s.id, s.access])),
  );

  const setScopeAccess = useCallback((id: string, next: GrantAccess) => {
    setAccess((prev) => (prev[id] === next ? prev : { ...prev, [id]: next }));
  }, []);

  const rows: FlowMapRow[] = useMemo(
    () =>
      scopes.map((s) => ({
        id: s.id,
        label: s.label,
        access: access[s.id] ?? s.access,
      })),
    [scopes, access],
  );

  const onConnectClick = useCallback(() => {
    if (busy) return;
    onConnect(scopes.map((s) => ({ id: s.id, access: access[s.id] ?? s.access })));
  }, [busy, onConnect, scopes, access]);

  return (
    <>
      <style jsx global>{`
        .cg-card {
          position: relative;
          max-width: 620px;
          border-radius: 20px;
          padding: 1.5px;
          background:
            linear-gradient(var(--card), var(--card)) padding-box,
            var(--brand-gradient-3) border-box;
          border: 1.5px solid transparent;
          box-shadow:
            0 18px 56px -16px color-mix(in oklab, var(--brand-primary) 35%, transparent),
            0 0 0 4px color-mix(in oklab, var(--brand-primary) 8%, transparent);
          overflow: hidden;
        }
        .cg-inner {
          background: var(--card);
          border-radius: 18.5px;
          overflow: hidden;
        }
        .cg-head {
          display: flex;
          align-items: center;
          gap: 14px;
          padding: 18px 20px 14px;
        }
        .cg-logo {
          width: 46px;
          height: 46px;
          border-radius: 13px;
          flex: none;
          display: grid;
          place-items: center;
          background: #fff;
          border: 1px solid var(--border);
          box-shadow: 0 6px 16px -6px color-mix(in oklab, var(--brand-primary) 30%, transparent);
          overflow: hidden;
        }
        .cg-logo img {
          width: 30px;
          height: 30px;
          object-fit: contain;
        }
        .cg-eyebrow {
          font-family: var(--font-audiowide);
          font-size: 9.5px;
          letter-spacing: 0.14em;
          text-transform: uppercase;
          margin-bottom: 5px;
          background: var(--brand-gradient);
          -webkit-background-clip: text;
          background-clip: text;
          -webkit-text-fill-color: transparent;
          color: transparent;
        }
        .cg-lead {
          font-size: 16.5px;
          font-weight: 700;
          color: var(--foreground);
          line-height: 1.35;
          letter-spacing: -0.01em;
        }
        .cg-sub {
          font-size: 12.5px;
          color: var(--muted-foreground);
          margin-top: 3px;
        }
        .cg-flow {
          margin: 2px 14px 8px;
          padding: 16px 12px 10px;
          border-radius: 14px;
          background: color-mix(in oklab, var(--brand-primary) 3%, var(--card));
          border: 1px solid var(--border);
        }
        .cg-flow svg {
          display: block;
          width: 100%;
          height: auto;
        }
        .cg-flow-cap {
          text-align: center;
          font-size: 11.5px;
          color: var(--muted-foreground);
          margin-top: 9px;
        }
        .cg-flow-cap .r {
          color: #0f766e;
          font-weight: 600;
        }
        .cg-flow-cap .w {
          color: var(--brand-pink);
          font-weight: 600;
        }
        .cg-flow-edge--read {
          opacity: 1;
        }
        .cg-flow-edge--live {
          stroke-dasharray: 5 4;
          animation: cg-dash 1s linear infinite;
        }
        @keyframes cg-dash {
          to {
            stroke-dashoffset: -16;
          }
        }
        @media (prefers-reduced-motion: reduce) {
          .cg-flow-edge--live {
            animation: none;
          }
        }
        .cg-scopes {
          padding: 4px 20px 6px;
        }
        .cg-scope {
          display: flex;
          align-items: center;
          gap: 13px;
          padding: 12px 2px;
        }
        .cg-scope + .cg-scope {
          border-top: 1px solid var(--border);
        }
        .cg-scope-txt {
          flex: 1;
        }
        .cg-scope-txt b {
          font-size: 14.5px;
          color: var(--foreground);
        }
        .cg-scope-txt span {
          display: block;
          font-size: 12px;
          color: var(--muted-foreground);
        }
        .cg-seg {
          display: inline-flex;
          background: var(--muted);
          border: 1px solid var(--border);
          border-radius: 9px;
          padding: 2px;
        }
        .cg-seg button {
          font-family: var(--font-sans);
          font-size: 12px;
          font-weight: 600;
          color: var(--muted-foreground);
          background: transparent;
          border: 0;
          padding: 6px 12px;
          border-radius: 7px;
          cursor: pointer;
          transition: 0.15s;
        }
        .cg-seg button:disabled {
          cursor: not-allowed;
          opacity: 0.6;
        }
        .cg-seg button.cg-on-r {
          background: #e7faf6;
          color: #0f766e;
          box-shadow: 0 1px 3px rgba(15, 118, 110, 0.2);
        }
        .cg-seg button.cg-on-w {
          background: var(--brand-gradient);
          color: #fff;
          box-shadow: 0 3px 10px color-mix(in oklab, var(--brand-pink) 35%, transparent);
        }
        .cg-foot {
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 14px 20px 18px;
          border-top: 1px solid var(--border);
        }
        .cg-foot-note {
          flex: 1;
          font-size: 11.5px;
          color: var(--muted-foreground);
          display: flex;
          align-items: center;
          gap: 6px;
        }
        .cg-btn {
          font-family: var(--font-sans);
          font-size: 13.5px;
          font-weight: 600;
          border-radius: 10px;
          padding: 10px 16px;
          cursor: pointer;
          border: 1px solid var(--border);
          background: var(--card);
          color: var(--foreground);
          transition: 0.15s;
        }
        .cg-btn:disabled {
          cursor: not-allowed;
          opacity: 0.6;
        }
        .cg-btn:not(:disabled):hover {
          border-color: var(--brand-primary);
          color: var(--brand-primary);
        }
        .cg-btn--primary {
          position: relative;
          border: 0;
          background: var(--brand-gradient);
          color: #fff;
          box-shadow: 0 8px 22px -6px color-mix(in oklab, var(--brand-pink) 50%, transparent);
          overflow: hidden;
        }
        .cg-btn--primary:not(:disabled):hover {
          color: #fff;
        }
      `}</style>
      <div
        className={`cg-card${className ? ` ${className}` : ""}`}
        data-testid="capability-grant-card"
        data-busy={busy ? "true" : "false"}
      >
        <div className="cg-inner">
          <div className="cg-head">
            <div className="cg-logo" aria-hidden="true">
              {connector.logoUrl ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img src={connector.logoUrl} alt="" />
              ) : (
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontWeight: 700,
                    fontSize: 15,
                    color: "var(--brand-primary)",
                  }}
                >
                  {(connector.name[0] ?? "?").toUpperCase()}
                </span>
              )}
            </div>
            <div style={{ flex: 1 }}>
              <div className="cg-eyebrow">Connect on demand</div>
              <div className="cg-lead">{askLine}</div>
              <div className="cg-sub">
                Follow the arrows — that&apos;s everything that moves, and which way.
              </div>
            </div>
          </div>

          <div className="cg-flow">
            <CapabilityGrantFlowMap
              teammateName={teammateName}
              connectorName={connector.name}
              rows={rows}
            />
            <div className="cg-flow-cap">
              <span className="r">◂ Read</span> — flows in to{" "}
              {teammateName.split(" ")[0] || teammateName}
              {" · "}
              <span className="w">⇄ Write</span> — can push changes back
            </div>
          </div>

          <div className="cg-scopes" role="list">
            {scopes.map((scope) => {
              const current = access[scope.id] ?? scope.access;
              const canRead = scope.grantable.includes("read");
              const canWrite = scope.grantable.includes("write");
              return (
                <div
                  key={scope.id}
                  className="cg-scope"
                  role="listitem"
                  data-testid={`scope-row-${scope.id}`}
                  data-access={current}
                >
                  <div className="cg-scope-txt">
                    <b>{scope.label}</b>
                    <span>{scope.description}</span>
                  </div>
                  <div
                    className="cg-seg"
                    role="group"
                    aria-label={`Access for ${scope.label}`}
                  >
                    <button
                      type="button"
                      disabled={busy || !canRead}
                      aria-pressed={current === "read"}
                      className={current === "read" ? "cg-on-r" : ""}
                      data-testid={`scope-toggle-read-${scope.id}`}
                      onClick={() => setScopeAccess(scope.id, "read")}
                    >
                      Read
                    </button>
                    <button
                      type="button"
                      disabled={busy || !canWrite}
                      aria-pressed={current === "write"}
                      className={current === "write" ? "cg-on-w" : ""}
                      data-testid={`scope-toggle-write-${scope.id}`}
                      onClick={() => setScopeAccess(scope.id, "write")}
                    >
                      Write
                    </button>
                  </div>
                </div>
              );
            })}
          </div>

          <div className="cg-foot">
            <div className="cg-foot-note">
              <ShieldCheck size={13} aria-hidden="true" />
              Revocable any time · every action logged
            </div>
            <button
              type="button"
              className="cg-btn"
              disabled={busy}
              data-testid="capability-grant-dismiss"
              onClick={onDismiss}
            >
              Not now
            </button>
            <button
              type="button"
              className="cg-btn cg-btn--primary"
              disabled={busy}
              data-testid="capability-grant-connect"
              onClick={onConnectClick}
            >
              {busy ? "Connecting…" : `Connect ${connector.name}`}
            </button>
          </div>
        </div>
      </div>
    </>
  );
}
