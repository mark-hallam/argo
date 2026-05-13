/**
 * HubSpot connector generator (v1).
 *
 * Detects HubSpot CRM URLs of the form
 * `https://app.hubspot.com/contacts/<portal>/<object>/<id>/...`
 * or `https://app.hubspot.com/contacts/<portal>/record/<object>/<id>`.
 * v1 covers the four primary CRM objects (contact, company, deal,
 * ticket) — enough to validate the manifest pattern before
 * generalising in v1.5.
 */
import type { GeneratorMatch, ConnectorGenerator } from "../types";

const URL_RE =
  /https:\/\/app\.hubspot\.com\/contacts\/\d{4,12}\/(?:record\/(?:0-1|0-2|0-3|0-5)\/\d{1,18}|(?:contact|company|deal|ticket)\/\d{1,18})/g;

export const hubspotGenerator: ConnectorGenerator = {
  system: "hubspot",
  systemLabel: "HubSpot",
  icon: "users",
  match(text: string): GeneratorMatch[] {
    const out: GeneratorMatch[] = [];
    for (const m of text.matchAll(URL_RE)) {
      const raw = m[0];
      const start = m.index ?? 0;
      out.push({
        start,
        end: start + raw.length,
        raw,
        deepLink: { url: raw, label: "View in HubSpot" },
      });
    }
    return out;
  },
  verifyOrigin(url: string): boolean {
    return url.startsWith("https://app.hubspot.com/");
  },
};
