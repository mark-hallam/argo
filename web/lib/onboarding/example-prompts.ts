/**
 * Placeholder + hint copy for the first-hire textarea on step 5, keyed
 * by the vertical id chosen in step 4. Without this lookup the
 * placeholder is a single hardcoded "marketing intern named Maya"
 * string, which contradicts the picked template (e.g. operator picks
 * "GTM Engineer", types the placeholder verbatim, Genesis dispatches on
 * archetype words in the prompt — "marketing intern" — and builds a
 * Marketing Intern plan).
 *
 * Each entry intentionally names a person and a role so Genesis's
 * name-parser + archetype-dispatch land on the same vertical the
 * operator selected. Vertical ids must stay in sync with the cards in
 * `web/app/(setup)/onboarding/pick-vertical/page.tsx` and the gateway
 * catalogue rows surfaced by `useVerticals()`. Unknown ids fall back to
 * the marketing-intern copy so the textarea is never empty.
 *
 * The placeholders deliberately avoid cadence words ("weekly", "daily")
 * — see `fix-first-hire-cta-copy-promises-weekly-schedule-not-parsed`.
 * Phase 1 of the wizard only posts `{description, vertical_id}`; cadence
 * is set later on the worker detail page's Schedules card.
 */
export const EXAMPLE_PROMPTS: Record<string, string> = {
  "gtm-engineer":
    "Mariah is a GTM engineer who researches inbound leads and drafts day-zero outreach for approval.",
  "marketing-intern":
    "A marketing intern named Maya who drafts blog posts and asks before publishing.",
  "marketplace-seller":
    "Marco is a marketplace seller who drafts listings and replies to buyers, escalating offers over $500.",
  recruiter:
    "Rachel is a recruiter who screens inbound resumes and drafts outreach to shortlisted candidates.",
  "sales-sdr":
    "Sam is an SDR who researches prospects, drafts cold outreach, and books qualified meetings.",
  "customer-support":
    "Jamal triages tier-one tickets, drafts reply templates, and escalates refunds over $500.",
  "finance-ar":
    "Nori handles invoicing, sends dunning emails, and reconciles payments end of month.",
};

export const DEFAULT_EXAMPLE_PROMPT = EXAMPLE_PROMPTS["marketing-intern"]!;

export function exampleForVertical(verticalId: string | null | undefined): string {
  if (!verticalId) return DEFAULT_EXAMPLE_PROMPT;
  return EXAMPLE_PROMPTS[verticalId] ?? DEFAULT_EXAMPLE_PROMPT;
}
