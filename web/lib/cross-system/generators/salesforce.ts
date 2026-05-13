/**
 * Salesforce connector generator (v1).
 *
 * Detects Lightning URLs of the form
 * `https://<instance>.lightning.force.com/lightning/r/<Object>/<Id>/view`
 * for the small set of object types operators most commonly link
 * out to (Opportunity, Account, Contact, Lead, Case). Origin is
 * any `*.lightning.force.com` host — Salesforce instances are
 * customer-namespaced and we cannot pre-enumerate them.
 *
 * Bare-ID detection (`SF-<n>` placeholders) is not part of v1; the
 * brief gates it behind operator-configured prefix mapping.
 */
import type { GeneratorMatch, ConnectorGenerator } from "../types";

const URL_RE =
  /https:\/\/[a-z0-9-]{1,63}\.lightning\.force\.com\/lightning\/r\/(?:Opportunity|Account|Contact|Lead|Case)\/[A-Za-z0-9]{15,18}\/view/g;

export const salesforceGenerator: ConnectorGenerator = {
  system: "salesforce",
  systemLabel: "Salesforce",
  icon: "cloud",
  match(text: string): GeneratorMatch[] {
    const out: GeneratorMatch[] = [];
    for (const m of text.matchAll(URL_RE)) {
      const raw = m[0];
      const start = m.index ?? 0;
      out.push({
        start,
        end: start + raw.length,
        raw,
        deepLink: { url: raw, label: "View in Salesforce" },
      });
    }
    return out;
  },
  verifyOrigin(url: string): boolean {
    return /^https:\/\/[a-z0-9-]{1,63}\.lightning\.force\.com\//.test(url);
  },
};
