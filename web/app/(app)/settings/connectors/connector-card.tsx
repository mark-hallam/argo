"use client";

import Image from "next/image";
import { useState } from "react";
import { CheckCircle2, Loader2, Plug, Trash2 } from "lucide-react";
import type { ConnectorEntry } from "./connector-catalog";
import { tierFor, tierLabel } from "./tier";
import type { TenantConnectorRow } from "@/lib/api/connectors";

interface ConnectorCardProps {
  connector: ConnectorEntry;
  state: TenantConnectorRow | undefined;
  onConnect: (c: ConnectorEntry) => void;
  onDisconnect: (c: ConnectorEntry) => void;
  isDisconnecting: boolean;
  /** Path under `/logos/` (no extension) — null when no shipped logo. */
  logoSrc: string | null;
}

/**
 * One connector tile in the `/settings/connectors` grid.
 *
 * Renders the brand logo (svgl.app SVG if shipped under
 * `public/logos/<id>.svg`, Lucide fallback otherwise), the tier badge,
 * current connection state, and the Connect / Disconnect button.
 *
 * The logo-present check is statically baked into `LOGO_IDS` in
 * `page.tsx` — adding a new SVG requires a one-line append there so
 * Next.js' static optimization doesn't try to fetch missing files.
 */
export function ConnectorCard({
  connector,
  state,
  onConnect,
  onDisconnect,
  isDisconnecting,
  logoSrc,
}: ConnectorCardProps) {
  const tier = tierFor(connector.id);
  const status = state?.status ?? "not_connected";
  const connected = status === "connected";
  const FallbackIcon = connector.fallbackIcon;
  const [logoFailed, setLogoFailed] = useState(false);
  const showLogo = logoSrc !== null && !logoFailed;

  return (
    <div
      className={`connector-card connector-card-${connected ? "on" : "off"}`}
      data-testid={`connector-card-${connector.id}`}
      data-connector-id={connector.id}
      data-status={status}
    >
      <div className="connector-card-head">
        <div className="connector-card-logo">
          {showLogo && logoSrc ? (
            <Image
              src={`/logos/${logoSrc}.svg`}
              alt=""
              width={28}
              height={28}
              onError={() => setLogoFailed(true)}
              unoptimized
            />
          ) : (
            <FallbackIcon size={20} aria-hidden />
          )}
        </div>
        <span className={`connector-card-tier connector-card-tier-${tier}`}>
          {tierLabel(tier)}
        </span>
      </div>

      <h2 className="connector-card-name">{connector.name}</h2>
      <p className="connector-card-blurb">{connector.description}</p>

      <div className="connector-card-state">
        {connected ? (
          <>
            <CheckCircle2 size={13} className="connector-card-state-on" />
            <span>
              Connected
              {state?.connectedAs ? ` as ${state.connectedAs}` : ""}
            </span>
          </>
        ) : status === "connecting" ? (
          <>
            <Loader2 size={13} className="connector-card-state-spin" />
            <span>Connecting…</span>
          </>
        ) : status === "reauth_needed" ? (
          <span className="connector-card-state-warn">Re-auth required</span>
        ) : status === "connect_failed" ? (
          <span className="connector-card-state-warn">Last attempt failed</span>
        ) : (
          <span className="connector-card-state-muted">Not connected</span>
        )}
      </div>

      <div className="connector-card-actions">
        {connected ? (
          <button
            type="button"
            className="connector-btn connector-btn-secondary"
            onClick={() => onDisconnect(connector)}
            disabled={isDisconnecting}
            data-testid={`connector-disconnect-${connector.id}`}
          >
            <Trash2 size={13} /> {isDisconnecting ? "Removing…" : "Disconnect"}
          </button>
        ) : (
          <button
            type="button"
            className="connector-btn connector-btn-primary"
            onClick={() => onConnect(connector)}
            data-testid={`connector-connect-${connector.id}`}
          >
            <Plug size={13} /> Connect
          </button>
        )}
      </div>
    </div>
  );
}
