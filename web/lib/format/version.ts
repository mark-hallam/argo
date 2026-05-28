/**
 * Canonical workflow-version label.
 *
 * Two envelopes flow through the SPA with incompatible shapes:
 *   - `WorkflowSummary.versions: number` (e.g. `3`) — integer revision count.
 *   - `WorkflowSpec.version: string` (e.g. `"v6"`, `"1.0.0"`) — gateway-supplied.
 *
 * Both forms get normalised to a single display: `v<integer>` (lowercase).
 * Semver strings reduce to the major-version integer ("1.0.0" → "v1") —
 * for v1.0 there is no semver semantics in the gateway, so collapsing to
 * the leading integer is safe.
 */
export function formatVersion(v: number | string): string {
  if (typeof v === "number") {
    if (!Number.isFinite(v)) return "v0";
    return `v${Math.max(0, Math.trunc(v))}`;
  }
  const trimmed = v.trim();
  const stripped =
    trimmed.startsWith("v") || trimmed.startsWith("V") ? trimmed.slice(1) : trimmed;
  const match = stripped.match(/^(\d+)/);
  if (match) return `v${parseInt(match[1], 10)}`;
  return "v0";
}
