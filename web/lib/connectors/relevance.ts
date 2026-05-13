/**
 * Pure relevance ranking — keeps the Step 5 Connect view free of
 * filter logic so the ranking can be unit-tested in isolation and so
 * the gateway swap (`mission-enterprise-connector-allowlist`) drops in
 * cleanly: that brief replaces the static `CONNECTORS` import with a
 * `useConnectorCatalog()` hook, but `relevantConnectorsFor` stays
 * unchanged.
 */
import type { WorkerDraft } from "@/lib/hire/draft-types";
import { CONNECTORS, type ConnectorMeta } from "./registry";

const MAX_ROWS = 6;
const MIN_ROWS = 3;

/**
 * Return the ordered list of connectors relevant to a worker draft.
 *
 * Ranking:
 *  1. High-affinity (vertical slug ∈ `verticalAffinity`).
 *  2. Universal (`universal: true`) that aren't already in (1).
 *  3. Cap at {@link MAX_ROWS}; pad up to {@link MIN_ROWS} from the
 *     remaining catalog if a niche vertical has fewer hits.
 *
 * Ties within each tier resolve by registry order (stable, deterministic).
 */
export function relevantConnectorsFor(
  draft: Pick<WorkerDraft, "suggestedVertical">,
): ConnectorMeta[] {
  const vertical = draft.suggestedVertical;
  const highAffinity: ConnectorMeta[] = [];
  const universals: ConnectorMeta[] = [];
  const rest: ConnectorMeta[] = [];

  for (const c of CONNECTORS) {
    if (c.verticalAffinity.includes(vertical)) {
      highAffinity.push(c);
    } else if (c.universal) {
      universals.push(c);
    } else {
      rest.push(c);
    }
  }

  const out: ConnectorMeta[] = [];
  for (const c of highAffinity) {
    if (out.length >= MAX_ROWS) break;
    out.push(c);
  }
  for (const c of universals) {
    if (out.length >= MAX_ROWS) break;
    out.push(c);
  }
  for (const c of rest) {
    if (out.length >= MIN_ROWS) break;
    if (out.length >= MAX_ROWS) break;
    out.push(c);
  }
  return out;
}
