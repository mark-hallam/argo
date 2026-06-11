/**
 * Capability-grant / connect-on-demand request shape.
 *
 * The data contract behind the locked `G2 · Elevated` capability-grant
 * card (design-explorations/2026-06-11/capability-grant). It surfaces in
 * the teammate's conversation the moment they hit a connector they don't
 * yet have access to ("To pull the Q3 pipeline I'll need to connect to
 * Salesforce"), leading with a one-line ask, a left-to-right flow map,
 * and a per-scope read/write toggle.
 *
 * These types are deliberately framework-free (no React) so they can be
 * shared by the presentational card, the connector-authorize fetcher,
 * and the run-now envelope that carries the request — without coupling
 * `lib/api/*` to a component module.
 *
 * The BACKEND emission of a `capability_request` when a teammate hits a
 * fenced connector mid-run is OUT OF SCOPE here (a named sibling
 * dependency — `backend-emit-capability-request-on-fenced-connector`).
 * This surface renders from whatever request shape the gateway emits /
 * the test mocks; it does not invent the emission.
 */

/** One requested scope on a connector, with the access the teammate is asking for. */
export interface GrantScope {
  /** Stable scope id (e.g. `"contacts"`, `"opportunities"`). */
  id: string;
  /** Plain-language label rendered in the scope row + flow-map node. */
  label: string;
  /** One-line description of what the teammate does with this scope. */
  description: string;
  /** The access the teammate is requesting by default. */
  access: GrantAccess;
  /** Which access levels the operator may toggle between for this scope. */
  grantable: GrantAccess[];
}

/** Access level for a single scope. `read` flows in; `write` flows both ways. */
export type GrantAccess = "read" | "write";

/** A pending connect-on-demand ask surfaced in the teammate's thread. */
export interface CapabilityGrantRequest {
  /** The connector the teammate needs to connect. */
  connector: { id: string; name: string; logoUrl?: string };
  /** Display name of the teammate making the ask ("Nora"). */
  teammateName: string;
  /** The one-line plain-language ask ("Connect Salesforce so Nora can read your pipeline"). */
  askLine: string;
  /** The scopes the teammate is requesting (>= 1). */
  scopes: GrantScope[];
}

/** The per-scope access the operator chose when they hit Connect. */
export interface GrantSelection {
  id: string;
  access: GrantAccess;
}
