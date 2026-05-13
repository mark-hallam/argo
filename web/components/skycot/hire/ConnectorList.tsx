"use client";

import { OAuthConnectorRow } from "@/components/ui/OAuthConnectorRow";
import type { ConnectorMeta, ConnectorAuthType } from "@/lib/connectors/registry";
import type { ConnectorStatus } from "@/lib/api/connectors";

/**
 * Thin presentational container — renders the per-connector rows for
 * Step 5 (Connect). Lifted out of `Step5Connect.tsx` so the row list
 * is reusable on the worker-profile page later without dragging the
 * Step 5 header / footer / channel section with it.
 *
 * All state and handlers come from the parent. Status and error
 * lookup keys are the connector id; absent keys default to
 * `not_connected`.
 */

export interface ConnectorListRowState {
  status: ConnectorStatus;
  error?: string;
}

export interface ConnectorListProps {
  connectors: ConnectorMeta[];
  /** Map of connectorId → status / last error. */
  stateById: Record<string, ConnectorListRowState | undefined>;
  /** Connector currently in the connecting state, if any. */
  pendingConnectorId: string | null;
  onConnect: (args: { connectorId: string; authType: ConnectorAuthType }) => void;
  onDisconnect: (connectorId: string) => void;
  onRetry: (args: { connectorId: string; authType: ConnectorAuthType }) => void;
}

export function ConnectorList({
  connectors,
  stateById,
  pendingConnectorId,
  onConnect,
  onDisconnect,
  onRetry,
}: ConnectorListProps) {
  if (connectors.length === 0) {
    return (
      <div
        data-testid="hire-step5-connectors-empty"
        className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] p-6 text-center text-sm text-[var(--muted-foreground)]"
      >
        No connectors to set up right now — you can add tools later from the
        teammate&apos;s profile.
      </div>
    );
  }
  return (
    <div data-testid="hire-step5-connectors" className="space-y-2">
      {connectors.map((connector) => {
        const rowState = stateById[connector.id];
        const status: ConnectorStatus = rowState?.status ?? "not_connected";
        const pending = pendingConnectorId === connector.id;
        return (
          <OAuthConnectorRow
            key={connector.id}
            connector={connector}
            status={status}
            pending={pending}
            onConnect={onConnect}
            onDisconnect={onDisconnect}
            onRetry={onRetry}
            errorReason={rowState?.error}
          />
        );
      })}
    </div>
  );
}
