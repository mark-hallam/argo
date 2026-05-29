/**
 * Heuristic connector-reference extractor for the workflow detail
 * viewer's preflight card.
 *
 * The current `WorkflowStep` shape (see `lib/sample-workflows.ts`) has
 * no first-class connector field — step kind `mcp_invoke` / `cu_action`
 * + their `server` / `app` fields are tracked in a sibling brief
 * (`fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`).
 * Until that lands, the most useful signal we can surface is a
 * conservative match against a known catalog: scan each step's
 * `name` + `description` (and any future `server`/`app`/`tool` field
 * the gateway may attach) for whole-word occurrences of a catalogued
 * connector id or its display name. Tenants that have not connected
 * a referenced connector get a 🔴 row; the Run-now button is gated
 * accordingly.
 *
 * Heuristic > nothing: a workflow that says "Zendesk fetch thread"
 * but doesn't carry a structured connector field today still gets
 * an actionable preflight signal. False positives are bounded by the
 * catalog — only connectors the platform knows about can match.
 */
import { CONNECTOR_CATALOG } from "@/app/(app)/settings/connectors/connector-catalog";

export interface ConnectorReference {
  /** Catalog id (e.g. `zendesk`, `google-meet`, `macos_notes`). */
  connectorId: string;
  /** Operator-facing display name for the card row. */
  displayName: string;
  /** Step ids that reference this connector. Ordered by first match. */
  stepIds: string[];
}

interface ExtractableStep {
  id?: string;
  kind?: string;
  name?: string;
  description?: string;
  /** Defensive — future `mcp_invoke` step shape. */
  server?: string;
  /** Defensive — future `cu_action` step shape. */
  app?: string;
  /** Already parsed by `parseStepsFromToml` for `tool_call` steps. */
  tool?: string;
}

interface CatalogEntryLite {
  id: string;
  /** Display name used for the card row + the search alias. */
  name: string;
}

/**
 * Build the default catalog from `CONNECTOR_CATALOG`. Exposed
 * separately so tests can stub a small subset without importing
 * the full catalog.
 */
export function defaultConnectorCatalog(): CatalogEntryLite[] {
  return CONNECTOR_CATALOG.map((entry) => ({ id: entry.id, name: entry.name }));
}

/**
 * Walk every step and return the connectors referenced. The catalog
 * argument is injectable so tests can run against a fixed list.
 */
export function extractConnectorReferences(
  steps: readonly ExtractableStep[] | null | undefined,
  catalog: readonly CatalogEntryLite[] = defaultConnectorCatalog(),
): ConnectorReference[] {
  if (!steps || steps.length === 0) return [];

  // Cache one compiled matcher per catalog entry. Two alternates per
  // entry: id (with `_`/`-` flexible) and display name. Word
  // boundaries on both ends.
  const matchers = catalog.map((entry) => ({
    entry,
    pattern: buildMatcherPattern(entry),
  }));

  const byId = new Map<string, ConnectorReference>();
  steps.forEach((step, idx) => {
    const stepId = step.id ?? `step-${idx + 1}`;
    const haystack = [
      step.name ?? "",
      step.description ?? "",
      step.server ?? "",
      step.app ?? "",
      step.tool ?? "",
    ]
      .filter(Boolean)
      .join(" \n ");
    if (!haystack) return;
    for (const { entry, pattern } of matchers) {
      if (!pattern.test(haystack)) continue;
      const existing = byId.get(entry.id);
      if (existing) {
        if (!existing.stepIds.includes(stepId)) existing.stepIds.push(stepId);
        continue;
      }
      byId.set(entry.id, {
        connectorId: entry.id,
        displayName: entry.name,
        stepIds: [stepId],
      });
    }
  });

  return Array.from(byId.values());
}

function buildMatcherPattern(entry: CatalogEntryLite): RegExp {
  const idAlt = escapeForRegex(entry.id).replace(/[_-]/g, "[\\s_-]?");
  const nameAlt = escapeForRegex(entry.name);
  // Word boundary on both ends; `[\\w-]` would let "github-issues"
  // match "github-issues-tracker" — accept that, the leading boundary
  // still keeps us off "logithub-issues". Case-insensitive.
  return new RegExp(`(?:^|[^\\w-])(?:${idAlt}|${nameAlt})(?:$|[^\\w-])`, "i");
}

function escapeForRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
