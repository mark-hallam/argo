/**
 * v1 connector catalog — hardcoded per the E2-g brief. The architectural
 * target (`mission-enterprise-connector-allowlist`) swaps this for a
 * workspace-managed allowlist surfaced by the gateway. The shape here is
 * what that endpoint will return, so the swap is a `useConnectorCatalog`
 * hook replacing the static import — no consumer changes required.
 *
 * Vertical-affinity strings line up with the slugs used by
 * `WorkerDraft.suggestedVertical` (see `lib/hire/draft-types.ts`) and the
 * `id` values on `EXAMPLE_ROLES` (see `lib/hire/examples.ts`).
 */
import type { LucideIcon } from "lucide-react";
import {
  Calendar,
  CreditCard,
  Github,
  ListChecks,
  Mail,
  MessageCircle,
  MessageSquare,
  ShoppingBag,
  Store,
  Tag,
} from "lucide-react";

export type ConnectorAuthType = "oauth" | "cu";

export interface ConnectorMeta {
  /** Stable id; matches the gateway connector slug. */
  id: string;
  /** Display name (Brand-cased). */
  name: string;
  /** One-line description shown beneath the name. */
  description: string;
  /** OAuth vs computer-use session capture flow. */
  authType: ConnectorAuthType;
  /** Lucide icon component — substitute for the brand logo asset in v1. */
  Icon: LucideIcon;
  /** Background tint class for the icon tile. */
  tintClass: string;
  /**
   * Vertical slugs where this connector is high-affinity. `relevance()`
   * promotes these for the matching vertical; non-affinity connectors
   * surface only if marked `universal`.
   */
  verticalAffinity: readonly string[];
  /**
   * Always-relevant connectors (e.g. Gmail) — surface for every vertical
   * after the high-affinity ones, until the 6-row cap is hit.
   */
  universal?: boolean;
}

export const CONNECTORS: readonly ConnectorMeta[] = [
  {
    id: "gmail",
    name: "Gmail",
    description: "Read and send email from your account.",
    authType: "oauth",
    Icon: Mail,
    tintClass: "bg-rose-500/10 text-rose-500",
    verticalAffinity: ["recruiter", "sales-sdr", "support-tier1", "tax-prep-intake"],
    universal: true,
  },
  {
    id: "google-calendar",
    name: "Google Calendar",
    description: "Book meetings and read your schedule.",
    authType: "oauth",
    Icon: Calendar,
    tintClass: "bg-sky-500/10 text-sky-500",
    verticalAffinity: ["recruiter", "sales-sdr"],
    universal: true,
  },
  {
    id: "slack",
    name: "Slack",
    description: "Post updates and reply in your workspace.",
    authType: "oauth",
    Icon: MessageSquare,
    tintClass: "bg-violet-500/10 text-violet-500",
    verticalAffinity: ["support-tier1", "sales-sdr"],
  },
  {
    id: "facebook-marketplace",
    name: "Facebook Marketplace",
    description: "Reply to listings and manage messages.",
    authType: "cu",
    Icon: Store,
    tintClass: "bg-blue-500/10 text-blue-500",
    verticalAffinity: ["marketplace-seller"],
  },
  {
    id: "gumtree",
    name: "Gumtree",
    description: "Sign in once on this device to manage listings.",
    authType: "cu",
    Icon: Tag,
    tintClass: "bg-emerald-500/10 text-emerald-500",
    verticalAffinity: ["marketplace-seller"],
  },
  {
    id: "ebay",
    name: "eBay",
    description: "Reply to buyers and manage listings.",
    authType: "oauth",
    Icon: ShoppingBag,
    tintClass: "bg-amber-500/10 text-amber-500",
    verticalAffinity: ["marketplace-seller"],
  },
  {
    id: "stripe",
    name: "Stripe",
    description: "Check payments, invoices, and customer details.",
    authType: "oauth",
    Icon: CreditCard,
    tintClass: "bg-indigo-500/10 text-indigo-500",
    verticalAffinity: ["sales-sdr"],
  },
  {
    id: "intercom",
    name: "Intercom",
    description: "Triage tickets and reply on your behalf.",
    authType: "oauth",
    Icon: MessageCircle,
    tintClass: "bg-blue-400/10 text-blue-400",
    verticalAffinity: ["support-tier1"],
  },
  {
    id: "linear",
    name: "Linear",
    description: "Open issues and move tickets along.",
    authType: "oauth",
    Icon: ListChecks,
    tintClass: "bg-purple-500/10 text-purple-500",
    verticalAffinity: ["support-tier1"],
  },
  {
    id: "github",
    name: "GitHub",
    description: "Read PRs, file issues, and comment.",
    authType: "oauth",
    Icon: Github,
    tintClass: "bg-zinc-500/10 text-zinc-500",
    verticalAffinity: [],
  },
] as const;

/** Lookup helper — `undefined` if no connector matches the id. */
export function findConnectorById(id: string): ConnectorMeta | undefined {
  return CONNECTORS.find((c) => c.id === id);
}
