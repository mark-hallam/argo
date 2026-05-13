"use client";

import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { StatusPill } from "@/components/ui/StatusPill";
import { cn } from "@/lib/utils";
import type {
  ConnectorAuthType,
  ConnectorMeta,
} from "@/lib/connectors/registry";
import type { ConnectorStatus } from "@/lib/api/connectors";

/**
 * Generic connector row used by Step 5 (Connect) and — later — the
 * worker profile page. Renders a tile (icon + name + description),
 * a status pill, and the action button appropriate for the current
 * status. The component is fully controlled: state and click handlers
 * come from the parent.
 */
export interface OAuthConnectorRowProps {
  connector: ConnectorMeta;
  status: ConnectorStatus;
  /** Set true while the popup is open for this row. */
  pending?: boolean;
  /** Connect / Sign-in-on-this-device button click. */
  onConnect: (args: { connectorId: string; authType: ConnectorAuthType }) => void;
  /** Disconnect (with confirm) click. */
  onDisconnect: (connectorId: string) => void;
  /** Retry after a failed connect — same flow as Connect. */
  onRetry: (args: { connectorId: string; authType: ConnectorAuthType }) => void;
  /** Optional human-readable error reason from the last attempt. */
  errorReason?: string;
}

interface StatusVisual {
  label: string;
  tone: "muted" | "info" | "success" | "warning" | "error";
}

function statusVisual(status: ConnectorStatus, pending: boolean): StatusVisual {
  if (pending || status === "connecting") {
    return { label: "Connecting…", tone: "info" };
  }
  switch (status) {
    case "connected":
      return { label: "Connected", tone: "success" };
    case "reauth_needed":
      return { label: "Re-auth needed", tone: "warning" };
    case "connect_failed":
      return { label: "Connect failed", tone: "error" };
    case "not_connected":
    default:
      return { label: "Not connected", tone: "muted" };
  }
}

export function OAuthConnectorRow({
  connector,
  status,
  pending = false,
  onConnect,
  onDisconnect,
  onRetry,
  errorReason,
}: OAuthConnectorRowProps) {
  const Icon = connector.Icon;
  const visual = statusVisual(status, pending);
  const connectLabel =
    connector.authType === "cu" ? "Sign in on this device" : "Connect";
  const isConnected = status === "connected";
  const isFailed = status === "connect_failed";
  const isReauth = status === "reauth_needed";
  const inProgress = pending || status === "connecting";

  return (
    <div
      data-testid={`hire-step5-connector-${connector.id}`}
      data-status={inProgress ? "connecting" : status}
      className={cn(
        "flex items-start gap-3 rounded-[var(--radius-md)] border border-[var(--border)]",
        "bg-[var(--card)] p-4 sm:items-center",
      )}
    >
      <div
        aria-hidden
        className={cn(
          "grid size-10 place-items-center rounded-[var(--radius-md)] shrink-0",
          connector.tintClass,
        )}
      >
        <Icon className="size-5" />
      </div>
      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex items-center gap-2 flex-wrap">
          <p className="text-sm font-medium text-[var(--foreground)]">
            {connector.name}
          </p>
          <StatusPill
            tone={visual.tone}
            size="sm"
            data-testid={`hire-step5-connector-${connector.id}-status`}
          >
            {visual.label}
          </StatusPill>
        </div>
        <p className="text-xs text-[var(--muted-foreground)]">
          {connector.description}
        </p>
        {isFailed && errorReason ? (
          <p
            data-testid={`hire-step5-connector-${connector.id}-error`}
            className="text-xs text-[var(--destructive)]"
          >
            {errorReason}
          </p>
        ) : null}
      </div>
      <div className="shrink-0">
        {isConnected ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            data-testid={`hire-step5-connector-${connector.id}-disconnect`}
            onClick={() => onDisconnect(connector.id)}
          >
            Disconnect
          </Button>
        ) : isFailed ? (
          <Button
            type="button"
            variant="secondary"
            size="sm"
            data-testid={`hire-step5-connector-${connector.id}-retry`}
            onClick={() =>
              onRetry({ connectorId: connector.id, authType: connector.authType })
            }
          >
            Retry
          </Button>
        ) : inProgress ? (
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled
            data-testid={`hire-step5-connector-${connector.id}-pending`}
          >
            <Loader2 className="animate-spin" aria-hidden />
            Connecting…
          </Button>
        ) : (
          <Button
            type="button"
            variant={isReauth ? "secondary" : "primary"}
            size="sm"
            data-testid={`hire-step5-connector-${connector.id}-connect`}
            onClick={() =>
              onConnect({ connectorId: connector.id, authType: connector.authType })
            }
          >
            {isReauth ? "Re-authenticate" : connectLabel}
          </Button>
        )}
      </div>
    </div>
  );
}
