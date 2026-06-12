// Pricing-page copy, Option D (DEC-053). Tier-card numbers, the feature
// matrix, and the FAQ are all DERIVED from tiers.ts (which reads the
// canonical v1-tiers.toml) — no hardcoded prices/allowances live here.
//
// Claim register: docs/skycot/marketing/PRICING.md (update in the same PR
// whenever this copy changes).
import {
  FREE,
  PRO,
  actions,
  overageExample,
  usd,
  OVERAGE_MULTIPLIER_LABEL,
} from "./tiers";

export type Tier = "free" | "pro" | "enterprise";

export interface TierCard {
  tier: Tier;
  title: string;
  price: string;
  period: string;
  blurb: string;
  features: string[];
  cta: string;
  /** Static signed-out target; Pro overrides this when signed in. */
  signedOutHref: string;
  signedInHref: string;
  /** Default featured (visually highlighted) state. */
  featured?: boolean;
}

export const TIERS: TierCard[] = [
  {
    tier: "free",
    title: "Free",
    price: usd(FREE.price_monthly_usd),
    period: "forever",
    blurb: "Try SKYCOT on our keys. No credit card.",
    features: [
      `${FREE.max_workers} teammate`,
      `${actions(FREE.max_actions_per_month)} actions / month`,
      "Community support",
      "Mock provider only",
      "Single channel (web inbox)",
    ],
    cta: "Choose Free",
    signedOutHref: "/auth/signup?plan=free",
    signedInHref: "/onboarding/welcome",
  },
  {
    tier: "pro",
    title: "Pro",
    price: usd(PRO.price_monthly_usd),
    period: "per teammate / month",
    blurb: `${actions(PRO.included_actions)} actions included per teammate, then metered overage with a cap you set.`,
    features: [
      `Up to ${PRO.max_workers} teammates`,
      `${actions(PRO.included_actions)} actions / teammate included`,
      `Overage at token cost ${OVERAGE_MULTIPLIER_LABEL}, never past your cap`,
      `Self-set spend cap (default ${usd(PRO.default_monthly_cap_usd)})`,
      "Pause a teammate, stop its billing",
      "Bring your own provider keys (OpenAI / Anthropic / Bedrock)",
      "Multi-channel (Slack / iMessage / Email)",
      "Approval flows + trust ratchet",
      "Marketplace skills",
    ],
    cta: "Choose Pro",
    signedOutHref: "/auth/signup?plan=pro",
    signedInHref: "/billing/subscribe?plan=pro",
    featured: true,
  },
  {
    tier: "enterprise",
    title: "Enterprise",
    price: "Custom",
    period: "talk to us",
    blurb: "Bring your own keys, a flat platform fee, SSO + SLA.",
    features: [
      "Unlimited teammates",
      "Bring-your-own keys — token cost billed to your provider account",
      "Flat negotiated platform fee",
      "Dedicated support + SLA",
      "SSO / SAML",
      "Audit export (S3 / GCS / Azure)",
      "Custom verticals + private marketplace",
      "On-prem / single-binary deploy",
    ],
    cta: "Talk to sales",
    signedOutHref: "mailto:hello@skycot.com?subject=Skycot%20Enterprise%20inquiry",
    signedInHref: "mailto:hello@skycot.com?subject=Skycot%20Enterprise%20inquiry",
  },
];

export interface MatrixRow {
  label: string;
  free: boolean;
  pro: boolean;
  enterprise: boolean;
}

export const MATRIX: MatrixRow[] = [
  { label: "SOC 2-ready audit trail", free: true, pro: true, enterprise: true },
  { label: "GDPR workflow tools", free: true, pro: true, enterprise: true },
  { label: "HIPAA-aligned redaction", free: false, pro: true, enterprise: true },
  { label: "ISO 27001 controls supported", free: false, pro: true, enterprise: true },
  { label: "Hash-chain + immutable log (regulator-ready)", free: false, pro: false, enterprise: true },
  { label: "Encryption-at-rest (SQLCipher)", free: true, pro: true, enterprise: true },
  { label: "Per-action role permissions", free: true, pro: true, enterprise: true },
  { label: "Forensic audit chain", free: true, pro: true, enterprise: true },
  { label: "Sub-workflows + delegation", free: false, pro: true, enterprise: true },
  { label: "Metered overage + self-set spend cap", free: false, pro: true, enterprise: false },
  { label: "Bring-your-own provider keys", free: false, pro: true, enterprise: true },
  { label: "Custom verticals + prompt packs", free: false, pro: false, enterprise: true },
  { label: "SSO / SAML", free: false, pro: false, enterprise: true },
];

export interface FaqItem {
  q: string;
  a: string;
}

export const FAQ: FaqItem[] = [
  {
    q: "How does Pro billing work?",
    a: `Pro is ${usd(PRO.price_monthly_usd)} per named teammate per month, and each teammate includes ${actions(PRO.included_actions)} actions. You only pay for teammates that are actively working — pause one and its billing stops. Enterprise is a flat negotiated rate.`,
  },
  {
    q: "What happens if a teammate goes over its action allowance?",
    a: overageExample(),
  },
  {
    q: "What is the spend cap?",
    a: `Every Pro workspace has a hard monthly spend cap you control (default ${usd(PRO.default_monthly_cap_usd)}). When period-to-date spend reaches the cap, SKYCOT stops running paid actions rather than billing you past it. Usage stops at your cap — no surprise bills.`,
  },
  {
    q: "Can I bring my own AI model keys?",
    a: "Yes. On Pro you can add your OpenAI, Anthropic, Bedrock, or self-hosted keys in Settings → Providers and pay the model provider directly for inference. Enterprise is bring-your-own-keys by default — your token costs bill straight to your own provider account, so SKYCOT carries zero token cost.",
  },
  {
    q: "Is data shared between tenants?",
    a: "No. Each customer workspace is fully isolated — every audit entry, skill run, integration call, and memory key carries the workspace stamp, and our automated checks block any code that could leak across the boundary. Tenant isolation is the load-bearing security guarantee.",
  },
  {
    q: "Can I self-host SKYCOT?",
    a: "Yes, on Enterprise. SKYCOT ships as a single binary with SQLite + SQLCipher, runs on a Mac mini or any Linux server, and includes a Docker image. The same binary you run locally runs in production — no separate self-host build.",
  },
  {
    q: "How is this different from Zapier / n8n?",
    a: "Zapier and n8n call them 'zaps' and 'workflows'. SKYCOT teammates have names, traits, and signature behaviors — they show up in Slack like a colleague, ask before they act, and earn trust over time. See /compare for the side-by-side breakdown.",
  },
];
