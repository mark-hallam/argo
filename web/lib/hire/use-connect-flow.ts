"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import {
  buildConnectUrl,
  type ConnectorStatus,
} from "@/lib/api/connectors";
import type { ConnectorAuthType } from "@/lib/connectors/registry";

/**
 * Hook that owns the popup + postMessage lifecycle for a single
 * connect attempt at a time. Step 5 (Connect) calls `connect()` per
 * row click; the hook centers a popup at the gateway URL, transitions
 * the row to "Connecting…", listens for the gateway's
 * `oauth-result` postMessage, and resolves to a final status.
 *
 * Popup-closed-before-completion is detected by polling
 * `popup.closed` while the popup is open. The gateway's redirect-back
 * page is responsible for posting the success / failure result.
 */

export interface ConnectFlowResult {
  /** The connector that just completed (or failed). */
  connectorId: string;
  /** Terminal status after the popup closes. */
  status: ConnectorStatus;
  /** Failure reason; only populated when status === "connect_failed". */
  error?: string;
}

export interface UseConnectFlowOptions {
  /** Server-assigned worker id. Required for the OAuth scoping. */
  workerId: string;
  /** Fires once the popup closes (success, fail, or dismissed). */
  onResult: (result: ConnectFlowResult) => void;
}

export interface UseConnectFlowResult {
  /** Open the connect popup for a connector. */
  connect: (args: { connectorId: string; authType: ConnectorAuthType }) => void;
  /** Connector id currently in-flight, or `null`. */
  pendingConnectorId: string | null;
}

const POPUP_FEATURES = "width=520,height=720,menubar=no,toolbar=no";
const POPUP_CLOSED_POLL_MS = 500;

interface OAuthResultMessage {
  type: "oauth-result";
  connectorId: string;
  status: ConnectorStatus;
  error?: string;
}

function isOAuthResultMessage(value: unknown): value is OAuthResultMessage {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return (
    v.type === "oauth-result" &&
    typeof v.connectorId === "string" &&
    typeof v.status === "string"
  );
}

export function useConnectFlow(opts: UseConnectFlowOptions): UseConnectFlowResult {
  const { workerId, onResult } = opts;
  const [pendingConnectorId, setPendingConnectorId] = useState<string | null>(null);
  const popupRef = useRef<Window | null>(null);
  const pollHandleRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const onResultRef = useRef(onResult);
  useEffect(() => {
    onResultRef.current = onResult;
  }, [onResult]);

  const cleanup = useCallback(() => {
    if (pollHandleRef.current !== null) {
      clearInterval(pollHandleRef.current);
      pollHandleRef.current = null;
    }
    popupRef.current = null;
    setPendingConnectorId(null);
  }, []);

  // postMessage listener — the gateway's return page posts here.
  useEffect(() => {
    function handler(event: MessageEvent) {
      // CRITICAL: origin check. Silently drop messages from elsewhere.
      if (event.origin !== window.location.origin) return;
      if (!isOAuthResultMessage(event.data)) return;
      const payload = event.data;
      onResultRef.current({
        connectorId: payload.connectorId,
        status: payload.status,
        error: payload.error,
      });
      try {
        popupRef.current?.close();
      } catch {
        /* popup may already be gone */
      }
      cleanup();
    }
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [cleanup]);

  // Clean up an in-flight popup on unmount.
  useEffect(() => cleanup, [cleanup]);

  const connect = useCallback<UseConnectFlowResult["connect"]>(
    ({ connectorId, authType }) => {
      const url = buildConnectUrl({
        authType,
        connectorId,
        workerId,
        returnOrigin: window.location.origin,
      });
      const popup = window.open(url, `skycot-connect-${connectorId}`, POPUP_FEATURES);
      if (!popup) {
        onResultRef.current({
          connectorId,
          status: "connect_failed",
          error: "Popup blocked by the browser. Allow popups and try again.",
        });
        return;
      }
      popupRef.current = popup;
      setPendingConnectorId(connectorId);
      try {
        popup.focus();
      } catch {
        /* cross-origin focus is best-effort */
      }
      // Poll for the user closing the popup without completing.
      pollHandleRef.current = setInterval(() => {
        if (popup.closed) {
          if (pollHandleRef.current !== null) {
            clearInterval(pollHandleRef.current);
            pollHandleRef.current = null;
          }
          // No postMessage means dismissed; revert to not_connected.
          if (popupRef.current === popup) {
            onResultRef.current({ connectorId, status: "not_connected" });
            cleanup();
          }
        }
      }, POPUP_CLOSED_POLL_MS);
    },
    [workerId, cleanup],
  );

  return { connect, pendingConnectorId };
}
