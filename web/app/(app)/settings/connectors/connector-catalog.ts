/**
 * Operator-facing connector catalog — the 29 REAL MCP connectors that
 * ship in v1.0 per the 2026-05-23 inventory
 * (`docs/skycot/evals/scheduled/2026-05-23/eval-mcp-connector-real-vs-stub-inventory.md`).
 *
 * This list is the source of truth for `/settings/connectors`. Each row
 * lines up 1:1 with a `crates/argo-mcp-connector-<id>/` crate. When a
 * new connector ships, add a row here, drop the logo SVG at
 * `web/public/logos/<id>.svg` (svgl.app per operator memory), and the
 * card renders automatically.
 *
 * `authType` reflects how an operator hands the gateway credentials:
 * - `oauth`  → 3-legged OAuth (popup → gateway `/connect/<id>`).
 * - `apikey` → labeled credential form (API token / bearer).
 *
 * Logos: 17 of 29 connectors have logos in `static/library` on the
 * upstream svgl.app library; the other 12 are not present there. Cards
 * fall back to a Lucide icon when `web/public/logos/<id>.svg` is
 * absent — see `connector-card.tsx`. When svgl.app adds the missing
 * 12 (or when an operator drops a custom SVG), no code change is
 * required.
 */
import type { LucideIcon } from "lucide-react";
import {
  AppWindow,
  Bell,
  BookOpen,
  Calendar,
  CalendarClock,
  Cloud,
  CreditCard,
  FileText,
  Github,
  Headphones,
  KanbanSquare,
  Layers,
  Linkedin,
  ListChecks,
  Mail,
  MailCheck,
  Megaphone,
  MessageCircle,
  MessageSquare,
  Mic,
  PenLine,
  Receipt,
  Send,
  ShoppingBag,
  Trello,
  Video,
} from "lucide-react";

export type ConnectorAuthType = "oauth" | "apikey";

export interface ConnectorEntry {
  /** Slug matches `crates/argo-mcp-connector-<id>/`. Stable. */
  id: string;
  /** Operator-facing display name. */
  name: string;
  /** One-line description shown beneath the name. */
  description: string;
  /** OAuth (popup) vs API-key (credential form). */
  authType: ConnectorAuthType;
  /**
   * Lucide fallback rendered when `web/public/logos/<id>.svg` is
   * missing. The fallback keeps the grid visually complete while the
   * brand logo for that connector is being sourced.
   */
  fallbackIcon: LucideIcon;
}

export const CONNECTOR_CATALOG: readonly ConnectorEntry[] = [
  {
    id: "airtable",
    name: "Airtable",
    description: "Read and update records in your Airtable bases.",
    authType: "apikey",
    fallbackIcon: Layers,
  },
  {
    id: "asana",
    name: "Asana",
    description: "Create tasks, projects, and move work along.",
    authType: "oauth",
    fallbackIcon: ListChecks,
  },
  {
    id: "calendly",
    name: "Calendly",
    description: "Read availability and surface booking links.",
    authType: "apikey",
    fallbackIcon: CalendarClock,
  },
  {
    id: "clickup",
    name: "ClickUp",
    description: "Create tasks, update statuses, and move work along.",
    authType: "apikey",
    fallbackIcon: KanbanSquare,
  },
  {
    id: "discord",
    name: "Discord",
    description: "Post updates and reply in your server channels.",
    authType: "oauth",
    fallbackIcon: MessageCircle,
  },
  {
    id: "figma",
    name: "Figma",
    description: "Read files, comments, and design tokens.",
    authType: "oauth",
    fallbackIcon: PenLine,
  },
  {
    id: "fireflies",
    name: "Fireflies",
    description: "Read meeting transcripts and action items.",
    authType: "apikey",
    fallbackIcon: Mic,
  },
  {
    id: "gcal",
    name: "Google Calendar",
    description: "Book meetings and read your schedule.",
    authType: "oauth",
    fallbackIcon: Calendar,
  },
  {
    id: "github-issues",
    name: "GitHub Issues",
    description: "Open, comment on, and close issues across repos.",
    authType: "oauth",
    fallbackIcon: Github,
  },
  {
    id: "gmail",
    name: "Gmail",
    description: "Read and send email from your Google account.",
    authType: "oauth",
    fallbackIcon: Mail,
  },
  {
    id: "google-meet",
    name: "Google Meet",
    description: "Start and schedule Meet sessions for your team.",
    authType: "oauth",
    fallbackIcon: Video,
  },
  {
    id: "hubspot",
    name: "HubSpot",
    description: "Sync contacts, deals, and engagements.",
    authType: "oauth",
    fallbackIcon: Megaphone,
  },
  {
    id: "imessage",
    name: "iMessage",
    description: "Send and receive iMessage via a Blue Bubbles relay.",
    authType: "apikey",
    fallbackIcon: MessageSquare,
  },
  {
    id: "jira",
    name: "Jira",
    description: "Create issues, transition workflows, link tickets.",
    authType: "oauth",
    fallbackIcon: AppWindow,
  },
  {
    id: "linear",
    name: "Linear",
    description: "Open issues and move tickets along.",
    authType: "oauth",
    fallbackIcon: Linkedin,
  },
  {
    id: "mailchimp",
    name: "Mailchimp",
    description: "Manage audiences, send campaigns, read reports.",
    authType: "apikey",
    fallbackIcon: MailCheck,
  },
  {
    id: "monday",
    name: "Monday",
    description: "Read boards, update items, run automations.",
    authType: "oauth",
    fallbackIcon: KanbanSquare,
  },
  {
    id: "notion",
    name: "Notion",
    description: "Read pages, update databases, capture notes.",
    authType: "apikey",
    fallbackIcon: BookOpen,
  },
  {
    id: "onedrive",
    name: "OneDrive",
    description: "Read and update files in your Microsoft OneDrive.",
    authType: "oauth",
    fallbackIcon: Cloud,
  },
  {
    id: "pagerduty",
    name: "PagerDuty",
    description: "Create, acknowledge, and resolve incidents.",
    authType: "apikey",
    fallbackIcon: Bell,
  },
  {
    id: "postmark",
    name: "Postmark",
    description: "Send transactional email and read delivery stats.",
    authType: "apikey",
    fallbackIcon: Send,
  },
  {
    id: "salesforce",
    name: "Salesforce",
    description: "Sync accounts, opportunities, and contacts.",
    authType: "oauth",
    fallbackIcon: ShoppingBag,
  },
  {
    id: "sendgrid",
    name: "SendGrid",
    description: "Send transactional email and read delivery stats.",
    authType: "apikey",
    fallbackIcon: Send,
  },
  {
    id: "slack",
    name: "Slack",
    description: "Post updates and reply in your workspace.",
    authType: "oauth",
    fallbackIcon: MessageSquare,
  },
  {
    id: "stripe",
    name: "Stripe",
    description: "Check payments, invoices, and customer details.",
    authType: "apikey",
    fallbackIcon: CreditCard,
  },
  {
    id: "stripe-events",
    name: "Stripe Events",
    description: "Read-only webhook event log paired with Stripe.",
    authType: "apikey",
    fallbackIcon: Receipt,
  },
  {
    id: "trello",
    name: "Trello",
    description: "Move cards, comment on lists, manage boards.",
    authType: "oauth",
    fallbackIcon: Trello,
  },
  {
    id: "typeform",
    name: "Typeform",
    description: "Read form responses and create follow-up tasks.",
    authType: "apikey",
    fallbackIcon: FileText,
  },
  {
    id: "zendesk",
    name: "Zendesk",
    description: "Triage tickets and reply on behalf of agents.",
    authType: "apikey",
    fallbackIcon: Headphones,
  },
] as const;

export function findConnectorById(id: string): ConnectorEntry | undefined {
  return CONNECTOR_CATALOG.find((c) => c.id === id);
}
