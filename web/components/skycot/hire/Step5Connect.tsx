"use client";

import { useCallback, useMemo, useState } from "react";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { Button } from "@/components/ui/Button";
import { ConnectorList } from "@/components/skycot/hire/ConnectorList";
import {
  type ChannelKind,
  ChannelPreferenceRow,
  detectDefaultPrimary,
} from "@/components/skycot/hire/ChannelPreferenceRow";
import { apiPatch } from "@/lib/api/client";
import {
  disconnectConnector as apiDisconnectConnector,
  type ConnectorStatus,
} from "@/lib/api/connectors";
import { relevantConnectorsFor } from "@/lib/connectors/relevance";
import type { ConnectorAuthType } from "@/lib/connectors/registry";
import { useConnectFlow } from "@/lib/hire/use-connect-flow";
import type { WorkerDraft } from "@/lib/hire/draft-types";

/**
 * Hire flow — Step 5 (Connect).
 *
 * Surfaces the vertical-relevant OAuth / computer-use connectors, lets
 * the user wire them to the freshly-hired worker, picks the
 * primary + fallback notification channels, then either continues to
 * `/hire/done/<workerId>` or skips with an inert flag.
 *
 * Connector status state is local to this view — the gateway's
 * `argo-connector` crate persists across reloads; on next visit the
 * worker-profile page rehydrates from the gateway directly. Skipping
 * commits the worker in an "inert" state visible on the Team page.
 */

export interface Step5ConnectProps {
  workerDraft: WorkerDraft & { workerId: string };
  /** Cookie/session hint for the default primary channel (e.g. "slack"). */
  defaultChannelHint?: string | null;
  /** Continue with state captured at the gateway. */
  onContinue: (workerId: string) => void;
  /** Skip — caller routes to `/hire/done/<workerId>?inert=1`. */
  onSkip: (workerId: string) => void;
  /** Back → Step 4. */
  onBack: () => void;
}

interface RowState {
  status: ConnectorStatus;
  error?: string;
}

export function Step5Connect({
  workerDraft,
  defaultChannelHint,
  onContinue,
  onSkip,
  onBack,
}: Step5ConnectProps) {
  const { workerId, suggestedName } = workerDraft;
  const connectors = useMemo(
    () => relevantConnectorsFor(workerDraft),
    [workerDraft],
  );

  const [stateById, setStateById] = useState<Record<string, RowState>>({});
  const [skipOpen, setSkipOpen] = useState(false);
  const [continueBusy, setContinueBusy] = useState(false);
  const [disconnectTarget, setDisconnectTarget] = useState<string | null>(null);

  const defaultPrimary = useMemo<ChannelKind>(
    () => detectDefaultPrimary(defaultChannelHint),
    [defaultChannelHint],
  );
  const [selected, setSelected] = useState<ChannelKind[]>([defaultPrimary]);
  const [primary, setPrimary] = useState<ChannelKind | null>(defaultPrimary);

  const updateRow = useCallback((id: string, next: RowState) => {
    setStateById((prev) => ({ ...prev, [id]: next }));
  }, []);

  const connectFlow = useConnectFlow({
    workerId,
    onResult: (result) => {
      if (result.status === "connected") {
        updateRow(result.connectorId, { status: "connected" });
      } else if (result.status === "connect_failed") {
        updateRow(result.connectorId, {
          status: "connect_failed",
          error: result.error ?? "Something went wrong. Try again.",
        });
      } else {
        // not_connected (popup dismissed early) — quiet revert.
        updateRow(result.connectorId, { status: "not_connected" });
      }
    },
  });

  const handleConnect = useCallback(
    (args: { connectorId: string; authType: ConnectorAuthType }) => {
      updateRow(args.connectorId, { status: "connecting" });
      connectFlow.connect(args);
    },
    [connectFlow, updateRow],
  );

  const handleDisconnect = useCallback((connectorId: string) => {
    setDisconnectTarget(connectorId);
  }, []);

  const confirmDisconnect = useCallback(() => {
    const id = disconnectTarget;
    if (!id) return;
    setDisconnectTarget(null);
    updateRow(id, { status: "not_connected" });
    void apiDisconnectConnector(workerId, id).catch(() => {
      updateRow(id, {
        status: "connect_failed",
        error: "Couldn't disconnect right now. Try again.",
      });
    });
  }, [disconnectTarget, updateRow, workerId]);

  const handleChannelChange = useCallback(
    (next: { selected: ChannelKind[]; primary: ChannelKind | null }) => {
      setSelected(next.selected);
      setPrimary(next.primary);
    },
    [],
  );

  const handleContinue = useCallback(() => {
    if (continueBusy) return;
    setContinueBusy(true);
    const payload = {
      channels: {
        primary,
        fallback: selected.filter((c) => c !== primary),
      },
    };
    void (async () => {
      try {
        await apiPatch(`/api/v1/workers/${encodeURIComponent(workerId)}`, payload);
      } catch {
        // Save failure is non-blocking — the worker is already hired.
        // The worker-profile page can re-edit channels post-hire.
      } finally {
        onContinue(workerId);
      }
    })();
  }, [continueBusy, primary, selected, workerId, onContinue]);

  return (
    <section
      data-testid="hire-step5-connect"
      className="mx-auto max-w-2xl space-y-6 pb-32"
    >
      <header className="space-y-2">
        <h1 className="text-2xl font-semibold tracking-tight text-[var(--foreground)]">
          {suggestedName} needs access to a few things.
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          You can skip — she&apos;ll work with whatever you have.
        </p>
      </header>

      <ConnectorList
        connectors={connectors}
        stateById={stateById}
        pendingConnectorId={connectFlow.pendingConnectorId}
        onConnect={handleConnect}
        onDisconnect={handleDisconnect}
        onRetry={handleConnect}
      />

      <ChannelPreferenceRow
        selected={selected}
        primary={primary}
        onChange={handleChannelChange}
      />

      <footer
        data-testid="hire-step5-footer"
        className="fixed inset-x-0 bottom-0 z-20 border-t border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md"
      >
        <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-4 flex items-center justify-between gap-2">
          <Button
            variant="ghost"
            onClick={onBack}
            data-testid="hire-step5-back"
          >
            Back
          </Button>
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              data-testid="hire-step5-skip"
              onClick={() => setSkipOpen(true)}
            >
              Skip for now
            </Button>
            <Button
              variant="primary"
              data-testid="hire-step5-continue"
              loading={continueBusy}
              onClick={handleContinue}
            >
              Continue
            </Button>
          </div>
        </div>
      </footer>

      <AlertDialog
        open={skipOpen}
        onOpenChange={setSkipOpen}
        title={`${suggestedName} will be hired but won't have anything to work on yet.`}
        description="You can connect tools later from her profile. Continue?"
        confirmLabel="Yes, skip for now"
        cancelLabel="Cancel"
        confirmVariant="primary"
        onConfirm={() => onSkip(workerId)}
      />

      <AlertDialog
        open={disconnectTarget !== null}
        onOpenChange={(open) => {
          if (!open) setDisconnectTarget(null);
        }}
        title="Disconnect this tool?"
        description={`${suggestedName} won't be able to use it until you reconnect.`}
        confirmLabel="Disconnect"
        cancelLabel="Keep connected"
        onConfirm={confirmDisconnect}
      />
    </section>
  );
}
