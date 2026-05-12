/**
 * Hardcoded example role-descriptions surfaced in the Step 1
 * "Need some examples?" popover. v1 ships a fixed set across all
 * tenants; the enterprise target (operator-configurable example set
 * per workspace) closes via `mission-enterprise-hire-examples-
 * configurable`. See `streams/E2-b-hire-flow-step1-land.md`.
 *
 * Copy is locked by the brief — do not drift wording without a
 * follow-on copy review.
 */
export interface ExampleRole {
  /** Short label shown as the popover row title. */
  title: string;
  /** First-person description that fills the textarea on click. */
  description: string;
}

export const EXAMPLE_ROLES: readonly ExampleRole[] = [
  {
    title: "Marketplace seller",
    description:
      "I sell hockey gear on Facebook Marketplace and Gumtree. I get 30 messages a day, most are 'is this still available'. I'd love someone to handle that.",
  },
  {
    title: "Recruiter — candidate screening",
    description:
      "I get 50+ applications a week for engineering roles. I need someone to do first-pass screening, schedule interviews, and keep candidates warm.",
  },
  {
    title: "Customer support — Tier 1",
    description:
      "We get a flood of tier-1 SaaS support tickets. Most are answerable from our docs. I want someone to handle those and escalate the rest.",
  },
  {
    title: "Sales SDR — outbound",
    description:
      "I research prospects and send personalized outbound emails. I'd love help with the research + drafting so I can focus on the calls that close.",
  },
  {
    title: "Tax-prep document intake",
    description:
      "Clients send me piles of W-2s, 1099s, and receipts. I need someone to extract the line items, flag missing docs, and queue them for my review.",
  },
] as const;

/**
 * The 3 rotating textarea placeholder strings. One is picked at random
 * on mount and stays stable for the session (avoids disorienting
 * re-rolls on re-render). Copy locked to wireframe-hire-flow.md.
 */
export const PLACEHOLDER_PROMPTS: readonly string[] = [
  "e.g. 'I keep getting calendar invites I have to triage…'",
  "e.g. 'My inbox is full of \"is this in stock\" questions…'",
  "e.g. 'I need someone to qualify inbound leads…'",
] as const;

/** Minimum textarea length before Continue activates. */
export const STEP1_MIN_CHARS = 15;
