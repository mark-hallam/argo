/**
 * Cross-system jump-link types (E5-h).
 *
 * v1 detection is hardcoded per connector via `ConnectorGenerator`
 * implementations under `web/lib/cross-system/generators/`. The
 * enterprise target moves the generators into per-connector manifest
 * declarations; closing brief
 * `mission-enterprise-connector-deep-link-manifest`.
 *
 * Why a separate module: keeps the generator surface small enough
 * for v1.5 fork-and-extend by operators who run additional internal
 * systems alongside Skycot's first-party connector set.
 */

/** Connector systems supported in v1. */
export type ConnectorSystem =
  | "stripe"
  | "slack"
  | "github"
  | "linear"
  | "salesforce"
  | "hubspot"
  | "zendesk"
  | "jira";

/** A single detected reference inside an input string. */
export interface CrossSystemMatch {
  /** Start byte offset of the match in the source text. */
  start: number;
  /** End byte offset (exclusive). */
  end: number;
  /** Which connector this match maps to. */
  system: ConnectorSystem;
  /** Verbatim matched substring (no transformation). */
  raw: string;
  /** Deep-link descriptor — resolved URL + display label. */
  deepLink: { url: string; label: string };
}

/** Generator-internal match — same shape as `CrossSystemMatch` but
 *  without `system` (added by the registry when fanning out). */
export type GeneratorMatch = Omit<CrossSystemMatch, "system">;

/** One connector's detection + URL-generation surface. */
export interface ConnectorGenerator {
  /** Stable system id; matches `ConnectorSystem`. */
  system: ConnectorSystem;
  /** Human-readable label rendered inside `<CrossSystemLink>`. */
  systemLabel: string;
  /** Lucide icon name used for the badge. */
  icon: string;
  /** Returns ordered matches against the source text. Never throws. */
  match: (text: string) => GeneratorMatch[];
  /** Origin allowlist — returns true only for URLs whose origin is
   *  whitelisted for this connector. Rendering layer rejects any
   *  generator output that does not pass `verifyOrigin`. */
  verifyOrigin: (url: string) => boolean;
}

/** Display copy + props the renderer needs from a match. */
export interface RenderedCrossSystemLink {
  system: ConnectorSystem;
  systemLabel: string;
  icon: string;
  url: string;
  label: string;
}
