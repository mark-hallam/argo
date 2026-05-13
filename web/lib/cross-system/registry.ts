/**
 * Connector deep-link registry (v1).
 *
 * Aggregates the 8 v1 generators (Stripe / Slack / GitHub / Linear /
 * Salesforce / HubSpot / Zendesk / Jira) and exposes a single entry
 * point — `getRegisteredGenerators()` — consumed by `detect.ts`. The
 * v1 implementation is hardcoded; the enterprise target loads the
 * same shape from each connector's manifest, replacing the imports
 * below with a manifest-driven loader.
 *
 * Security note: every generator must pass its own `verifyOrigin`
 * before its emitted URL is rendered. `detect.ts` enforces that
 * gate centrally; generators themselves only assert origin via
 * their match regexes.
 */
import type { ConnectorGenerator, ConnectorSystem } from "./types";
import { stripeGenerator } from "./generators/stripe";
import { slackGenerator } from "./generators/slack";
import { githubGenerator } from "./generators/github";
import { linearGenerator } from "./generators/linear";
import { salesforceGenerator } from "./generators/salesforce";
import { hubspotGenerator } from "./generators/hubspot";
import { zendeskGenerator } from "./generators/zendesk";
import { jiraGenerator } from "./generators/jira";

const GENERATORS: ConnectorGenerator[] = [
  stripeGenerator,
  slackGenerator,
  githubGenerator,
  linearGenerator,
  salesforceGenerator,
  hubspotGenerator,
  zendeskGenerator,
  jiraGenerator,
];

const BY_SYSTEM: Map<ConnectorSystem, ConnectorGenerator> = new Map(
  GENERATORS.map((g) => [g.system, g]),
);

/** Returns the ordered list of registered generators. Ordering is
 *  significant only as a tiebreaker for matches at the same
 *  position — `detect.ts` sorts primarily by start offset. */
export function getRegisteredGenerators(): readonly ConnectorGenerator[] {
  return GENERATORS;
}

/** Look up a generator by system. Returns `undefined` if the system
 *  is not registered in v1. */
export function getGenerator(
  system: ConnectorSystem,
): ConnectorGenerator | undefined {
  return BY_SYSTEM.get(system);
}
