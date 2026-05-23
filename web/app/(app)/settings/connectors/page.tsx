"use client";

import "./page.css";

import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { CONNECTOR_CATALOG, type ConnectorEntry } from "./connector-catalog";
import { ConnectorCard } from "./connector-card";
import { ConnectModal } from "./connect-modal";
import {
  disconnectTenantConnector,
  listTenantConnectors,
  type TenantConnectorListResponse,
  type TenantConnectorRow,
} from "@/lib/api/connectors";

/**
 * `/settings/connectors` — operator-facing catalog of every shipped
 * connector. Renders the 29 entries in `CONNECTOR_CATALOG`, surfaces
 * tenant-scoped connect/disconnect state, and drives the
 * Connect/Disconnect flows via `ConnectModal`.
 *
 * D3 brand-loud-marketplace register (hero search + gradient + cards).
 * Mirrors the locked Skills marketplace pattern (`/skills`).
 */

const TRY_CHIPS = [
  "email",
  "calendar",
  "chat",
  "payments",
  "support",
  "issues",
];

const QUERY_KEY = ["settings", "connectors"] as const;

/**
 * Logo presence is static — every svg under `web/public/logos/` is
 * listed here so cards know whether to render `<Image>` or the
 * Lucide fallback. Adding a new logo means one line here + the file
 * on disk. Filenames match the catalog `id` exactly.
 */
const LOGO_IDS: ReadonlySet<string> = new Set([
  "asana",
  "calendly",
  "clickup",
  "discord",
  "figma",
  "gcal",
  "github-issues",
  "gmail",
  "google-meet",
  "linear",
  "notion",
  "onedrive",
  "salesforce",
  "slack",
  "stripe",
  "stripe-events",
  "trello",
]);

function logoSrcFor(id: string): string | null {
  return LOGO_IDS.has(id) ? id : null;
}

function matchesQuery(c: ConnectorEntry, q: string): boolean {
  if (q.length === 0) return true;
  return (
    c.name.toLowerCase().includes(q) ||
    c.description.toLowerCase().includes(q) ||
    c.id.toLowerCase().includes(q)
  );
}

export default function ConnectorsPage() {
  const queryClient = useQueryClient();
  const [query, setQuery] = useState("");
  const [activeConnector, setActiveConnector] = useState<ConnectorEntry | null>(null);

  const tenantState = useQuery<TenantConnectorListResponse>({
    queryKey: QUERY_KEY,
    queryFn: ({ signal }) => listTenantConnectors({ signal }),
    staleTime: 30_000,
  });

  const disconnect = useMutation({
    mutationFn: (connectorId: string) => disconnectTenantConnector(connectorId),
    onSuccess: (_, connectorId) => {
      queryClient.setQueryData<TenantConnectorListResponse>(QUERY_KEY, (prev) => {
        if (!prev) return prev;
        return {
          connectors: prev.connectors.filter((row) => row.connectorId !== connectorId),
        };
      });
    },
  });

  const stateById = useMemo(() => {
    const byId = new Map<string, TenantConnectorRow>();
    for (const row of tenantState.data?.connectors ?? []) {
      byId.set(row.connectorId, row);
    }
    return byId;
  }, [tenantState.data?.connectors]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return CONNECTOR_CATALOG.filter((c) => matchesQuery(c, q));
  }, [query]);

  function onConnectedAfterApiKey(connectorId: string) {
    const entry = CONNECTOR_CATALOG.find((c) => c.id === connectorId);
    queryClient.setQueryData<TenantConnectorListResponse>(QUERY_KEY, (prev) => {
      const optimistic: TenantConnectorRow = {
        connectorId,
        status: "connected",
        connectedAt: new Date().toISOString(),
        connectedAs: entry?.name ?? connectorId,
      };
      if (!prev) return { connectors: [optimistic] };
      const others = prev.connectors.filter((row) => row.connectorId !== connectorId);
      return { connectors: [optimistic, ...others] };
    });
  }

  return (
    <div className="connectors-stage" data-testid="connectors-page">
      <span className="connectors-eyebrow">
        <span className="connectors-eyebrow-dot" />
        WORKSPACE CONNECTORS
      </span>

      <h1 className="connectors-hero">
        Wire SKYCOT into the
        <br />
        tools you <strong>already use</strong>.
      </h1>

      <p className="connectors-sub">
        Connect once, and every worker on your team can read, write, and
        act in {CONNECTOR_CATALOG.length} systems — email, calendar,
        chat, support, billing, and more.
      </p>

      <div className="connectors-search-row">
        <div className="connectors-search-icon">
          <Search size={16} aria-hidden />
        </div>
        <input
          type="text"
          className="connectors-search-input"
          placeholder="Search connectors by name, vendor, or category…"
          aria-label="Search connectors"
          data-testid="connectors-search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <span className="connectors-search-count" data-testid="connectors-count">
          {CONNECTOR_CATALOG.length} total · {filtered.length} shown
        </span>
        <span className="connectors-search-kbd" aria-hidden>
          ⌘K
        </span>
      </div>

      <div className="connectors-try-row">
        <span className="connectors-try-label">TRY</span>
        {TRY_CHIPS.map((chip) => (
          <button
            key={chip}
            type="button"
            className="connectors-try-chip"
            onClick={() => setQuery(chip)}
          >
            {chip}
          </button>
        ))}
      </div>

      <div className="connectors-section-label">CATALOG</div>

      {filtered.length === 0 ? (
        <div className="connectors-empty-filter" data-testid="connectors-empty-filter">
          No connectors match &ldquo;{query}&rdquo;.
          <button type="button" onClick={() => setQuery("")}>Clear filter</button>
        </div>
      ) : (
        <div className="connectors-grid">
          {filtered.map((c) => (
            <ConnectorCard
              key={c.id}
              connector={c}
              state={stateById.get(c.id)}
              logoSrc={logoSrcFor(c.id)}
              onConnect={(entry) => setActiveConnector(entry)}
              onDisconnect={(entry) => disconnect.mutate(entry.id)}
              isDisconnecting={disconnect.isPending && disconnect.variables === c.id}
            />
          ))}
        </div>
      )}

      {activeConnector ? (
        <ConnectModal
          connector={activeConnector}
          onClose={() => setActiveConnector(null)}
          onConnected={(id) => onConnectedAfterApiKey(id)}
        />
      ) : null}
    </div>
  );
}

// Re-exported for the snapshot test in `__tests__/settings/connectors-catalog.test.ts`.
export { logoSrcFor };
