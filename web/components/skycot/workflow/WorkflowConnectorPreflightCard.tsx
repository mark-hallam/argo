/**
 * Workflow connector preflight card — rendered between the meta row
 * and the steps list on `/workflows/<v>/<n>`. One row per connector
 * the workflow references, each carrying the tenant-side health
 * signal so the operator knows "will Run now actually work?" before
 * pressing the button.
 *
 * Health states map from {@link ConnectorStatus}:
 *   connected      → ✅ connected
 *   reauth_needed  → ⚠️ needs reconnect
 *   not_connected  → 🔴 not connected
 *   connect_failed → 🔴 connect failed
 *   connecting     → 🔵 in progress
 *
 * No data → skeleton row per detected connector (no layout flicker).
 *
 * Card is render-suppressed when the workflow has zero detected
 * connector references. Acceptance criterion 5: workflows with no
 * dependencies must not produce an empty placeholder.
 */
import Link from "next/link";
import {
  AlertTriangle,
  CheckCircle2,
  CircleSlash2,
  Loader2,
  Plug,
} from "lucide-react";
import type { ConnectorStatus, TenantConnectorRow } from "@/lib/api/connectors";
import type { ConnectorReference } from "@/lib/workflow/extract-connectors";

export interface WorkflowConnectorPreflightCardProps {
  references: readonly ConnectorReference[];
  /** Latest tenant-side state keyed by connector id. */
  statuses?: Readonly<Record<string, TenantConnectorRow>>;
  /** True while the tenant connector list is still loading. */
  isLoading?: boolean;
  /** Override for the per-connector deep-link. Defaults to /settings/connectors. */
  settingsHrefFor?: (connectorId: string) => string;
}

type Health = "ok" | "warn" | "blocked" | "pending";

interface HealthMeta {
  label: string;
  health: Health;
  icon: React.ReactNode;
  fg: string;
  bg: string;
}

const HEALTH_META: Record<Health, Omit<HealthMeta, "label">> = {
  ok: {
    health: "ok",
    icon: <CheckCircle2 size={12} aria-hidden />,
    fg: "#047857",
    bg: "#ecfdf5",
  },
  warn: {
    health: "warn",
    icon: <AlertTriangle size={12} aria-hidden />,
    fg: "#92400e",
    bg: "#fef3c7",
  },
  blocked: {
    health: "blocked",
    icon: <CircleSlash2 size={12} aria-hidden />,
    fg: "#b91c1c",
    bg: "#fee2e2",
  },
  pending: {
    health: "pending",
    icon: <Loader2 size={12} aria-hidden />,
    fg: "#1d4ed8",
    bg: "#dbeafe",
  },
};

export function metaForStatus(status: ConnectorStatus | undefined): HealthMeta {
  switch (status) {
    case "connected":
      return { ...HEALTH_META.ok, label: "Connected" };
    case "reauth_needed":
      return { ...HEALTH_META.warn, label: "Needs reconnect" };
    case "connecting":
      return { ...HEALTH_META.pending, label: "In progress" };
    case "connect_failed":
      return { ...HEALTH_META.blocked, label: "Connect failed" };
    case "not_connected":
    case undefined:
    default:
      return { ...HEALTH_META.blocked, label: "Not connected" };
  }
}

/**
 * `true` if every row in `references` resolves to an `ok` health
 * state against `statuses`. Empty references → vacuously ok.
 * Reused by the page to gate the Run-now button.
 */
export function allConnectorsHealthy(
  references: readonly ConnectorReference[],
  statuses?: Readonly<Record<string, TenantConnectorRow>>,
): boolean {
  for (const ref of references) {
    const meta = metaForStatus(statuses?.[ref.connectorId]?.status);
    if (meta.health !== "ok") return false;
  }
  return true;
}

export function WorkflowConnectorPreflightCard({
  references,
  statuses,
  isLoading,
  settingsHrefFor,
}: WorkflowConnectorPreflightCardProps) {
  if (references.length === 0) return null;
  const hrefFor =
    settingsHrefFor ?? ((id: string) => `/settings/connectors/${encodeURIComponent(id)}`);

  return (
    <section
      data-testid="wfd-connector-preflight"
      aria-label="Connector preflight"
      style={{
        margin: "16px 0",
        padding: "16px 20px",
        border: "1px solid var(--border, #ece9f5)",
        borderRadius: 12,
        background: "var(--surface, #ffffff)",
      }}
    >
      <header
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          marginBottom: 12,
        }}
      >
        <Plug size={14} aria-hidden style={{ color: "#8b5cf6" }} />
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            letterSpacing: "0.14em",
            textTransform: "uppercase",
            color: "var(--muted-foreground, #52525b)",
          }}
        >
          Preflight · {references.length}{" "}
          {references.length === 1 ? "connector" : "connectors"}
        </span>
      </header>
      <ul
        data-testid="wfd-connector-preflight-list"
        style={{
          listStyle: "none",
          padding: 0,
          margin: 0,
          display: "flex",
          flexDirection: "column",
          gap: 8,
        }}
      >
        {references.map((ref) => (
          <PreflightRow
            key={ref.connectorId}
            reference={ref}
            row={statuses?.[ref.connectorId]}
            isLoading={!!isLoading && !statuses?.[ref.connectorId]}
            href={hrefFor(ref.connectorId)}
          />
        ))}
      </ul>
    </section>
  );
}

interface PreflightRowProps {
  reference: ConnectorReference;
  row: TenantConnectorRow | undefined;
  isLoading: boolean;
  href: string;
}

function PreflightRow({ reference, row, isLoading, href }: PreflightRowProps) {
  const meta = metaForStatus(row?.status);
  const rowTestId = `wfd-connector-preflight-row-${reference.connectorId}`;
  const stepLabel =
    reference.stepIds.length === 1
      ? `Used in 1 step`
      : `Used in ${reference.stepIds.length} steps`;

  return (
    <li
      data-testid={rowTestId}
      data-connector-id={reference.connectorId}
      data-health={isLoading ? "loading" : meta.health}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        padding: "10px 12px",
        borderRadius: 8,
        background: "var(--surface-muted, #fafafa)",
      }}
    >
      <span
        aria-hidden
        style={{
          width: 26,
          height: 26,
          borderRadius: 6,
          background: "#eef2ff",
          color: "#4338ca",
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
          fontSize: 12,
          fontWeight: 700,
          fontFamily: "var(--font-mono)",
        }}
      >
        {reference.displayName.slice(0, 1).toUpperCase()}
      </span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontSize: 13,
            fontWeight: 600,
            color: "var(--foreground, #1f1d2b)",
          }}
        >
          {reference.displayName}
        </div>
        <div
          style={{
            fontSize: 11,
            color: "var(--muted-foreground, #52525b)",
          }}
        >
          {stepLabel}
        </div>
      </div>
      {isLoading ? (
        <span
          data-testid={`${rowTestId}-skeleton`}
          aria-label="Loading connector status"
          style={{
            width: 110,
            height: 22,
            borderRadius: 999,
            background:
              "linear-gradient(90deg, #f1f5f9 0%, #e2e8f0 50%, #f1f5f9 100%)",
          }}
        />
      ) : (
        <Link
          href={href}
          data-testid={`${rowTestId}-link`}
          style={{
            textDecoration: "none",
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            padding: "3px 10px",
            borderRadius: 999,
            background: meta.bg,
            color: meta.fg,
            fontSize: 11,
            fontWeight: 700,
            letterSpacing: 0,
            whiteSpace: "nowrap",
          }}
        >
          {meta.icon}
          <span>{meta.label}</span>
        </Link>
      )}
    </li>
  );
}
