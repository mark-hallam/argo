/**
 * Operator-facing notification-channel catalog — the 11 approval
 * channels shipped in `crates/argo-channels/src/`. Source of truth is
 * the 2026-05-23 audit
 * (`docs/skycot/evals/scheduled/2026-05-23/eval-channels-v1-config-vs-code-inventory.md`)
 * and pin-test `crates/argo-channels/tests/channels_v1_inventory_pin_test.rs`.
 *
 * This file is a SIBLING of `web/lib/connectors/registry.ts`, NOT a
 * replacement. The two catalogs are disjoint:
 *
 * - `web/lib/connectors/registry.ts` → work-tool MCP connectors
 *   (Gmail, Calendar, Slack-work-tool, etc.) used by the hire wizard.
 * - `web/lib/notify-channels/registry.ts` (this file) → approval
 *   notification channels (Slack-approvals, Email, iMessage, etc.)
 *   wired through `dispatch_notify`.
 *
 * The audit-verdict bucket is the v1 static fallback when the gateway
 * `/api/doctor/channels` endpoint is unavailable; a later brief
 * replaces it with live `argo doctor channels` probe data. Until then,
 * `verdict` is what the page renders.
 */
import type { LucideIcon } from "lucide-react";
import {
  Inbox,
  Mail,
  MessageCircle,
  MessageSquare,
  Mic,
  Phone,
  Smartphone,
} from "lucide-react";

/**
 * Audit verdict from the 2026-05-23 channels-config-vs-code inventory.
 *
 * - `live`     → 🟢 LIVE-in-v1.0. Promised in a tier card AND
 *                impl-wired-routed end-to-end.
 * - `shipped`  → 🟡 SHIPPED-BUT-NOT-CONFIGURED-IN-V1.0. Real impl +
 *                env-var configurable, just not in a tier promise.
 * - `deferred` → 🔴 STUB-OR-NOT-WIRED. Outbound dispatcher refuses
 *                this channel (or critical link missing) per
 *                `worker_actions_notify_env.rs:30` `DEFERRED-v1.x`.
 */
export type ChannelVerdict = "live" | "shipped" | "deferred";

/** Live probe status from `argo doctor channels` JSON output. */
export type ProbeStatus = "OK" | "MISSING" | "WARN" | "ERROR" | "SKIP" | "UNKNOWN";

export interface ChannelEnvVar {
  /** Environment variable name (e.g. `SLACK_APPROVALS_BOT_TOKEN`). */
  name: string;
  /** One-line description of what the operator should paste here. */
  hint: string;
  /** Whether this env var must be set for the channel to work. */
  required: boolean;
}

export interface NotifyChannelEntry {
  /** Channel id pinned by `channels_v1_inventory_pin_test.rs`. */
  id: string;
  /** Operator-facing display name. */
  name: string;
  /** One-line blurb shown beneath the name on the card. */
  description: string;
  /** Audit verdict from the 2026-05-23 inventory. */
  verdict: ChannelVerdict;
  /**
   * Whether this channel appears in the v1.0 Pro-tier "Multi-channel
   * (Slack / iMessage / Email)" promise. Pro = {slack, imessage,
   * email}; webinbox is in the Free tier promise; everything else is
   * 🟡 shipped-but-not-promised.
   */
  proTier: boolean;
  /** Env vars the operator pastes into `~/.argo/cred.toml` or shell. */
  envVars: readonly ChannelEnvVar[];
  /** Lucide icon. Brief restricts to 7 icons; map sensibly. */
  Icon: LucideIcon;
  /** Background tint class for the icon tile. */
  tintClass: string;
  /**
   * Per-channel note. iMessage carries the sales-promise gap warning
   * (Pro card promises iMessage but `dispatch_notify` refuses it
   * until `mission-imessage-wire-pro-tier-outbound-dispatch` lands).
   */
  note?: string;
}

export const NOTIFY_CHANNELS: readonly NotifyChannelEntry[] = [
  {
    id: "email",
    name: "Email",
    description: "Postmark-backed approval notifications + Postmark inbound webhook.",
    verdict: "live",
    proTier: true,
    envVars: [
      { name: "POSTMARK_SERVER_TOKEN", hint: "Postmark server API token.", required: true },
      { name: "POSTMARK_FROM_ADDRESS", hint: "Verified sender address.", required: true },
      { name: "ARGO_CHANNEL_HMAC_SECRET", hint: "Shared secret for signed reply tokens.", required: true },
    ],
    Icon: Mail,
    tintClass: "channel-tint-rose",
  },
  {
    id: "slack",
    name: "Slack",
    description: "chat.postMessage approvals + HMAC-verified interaction callbacks.",
    verdict: "live",
    proTier: true,
    envVars: [
      { name: "SLACK_APPROVALS_BOT_TOKEN", hint: "Slack bot token (xoxb-…).", required: true },
      { name: "SLACK_APPROVALS_SIGNING_SECRET", hint: "App signing secret for HMAC.", required: true },
      { name: "SLACK_APPROVALS_TEST_CHANNEL", hint: "Channel id for `argo doctor channels` smoke test.", required: false },
    ],
    Icon: MessageSquare,
    tintClass: "channel-tint-violet",
  },
  {
    id: "imessage",
    name: "iMessage",
    description: "Blue Bubbles bridge to the Apple Messages stack.",
    verdict: "deferred",
    proTier: true,
    envVars: [
      { name: "BLUE_BUBBLES_URL", hint: "Blue Bubbles server base URL.", required: true },
      { name: "BLUE_BUBBLES_PASSWORD", hint: "Blue Bubbles server password.", required: true },
      { name: "BLUE_BUBBLES_WEBHOOK_SECRET", hint: "Shared secret for the inbound webhook.", required: true },
    ],
    Icon: MessageCircle,
    tintClass: "channel-tint-emerald",
    note:
      "Pro tier promises iMessage but `dispatch_notify` currently refuses outbound until " +
      "`mission-imessage-wire-pro-tier-outbound-dispatch` lands. Configure now; outbound " +
      "lights up the day that brief merges. Inbound webhook is already wired.",
  },
  {
    id: "webinbox",
    name: "Web inbox",
    description: "Local JSONL queue surfaced by the gateway's `/inbox` SSE route. Default fallback.",
    verdict: "live",
    proTier: false,
    envVars: [
      { name: "ARGO_WEBINBOX_PATH", hint: "Path to JSONL queue. Defaults to a per-tenant tmp file.", required: false },
    ],
    Icon: Inbox,
    tintClass: "channel-tint-indigo",
  },
  {
    id: "discord",
    name: "Discord",
    description: "Bot webhook + button-decision route. Shipped, not in a v1 tier card.",
    verdict: "shipped",
    proTier: false,
    envVars: [
      { name: "DISCORD_BOT_TOKEN", hint: "Discord bot token.", required: true },
      { name: "DISCORD_APPROVALS_CHANNEL_ID", hint: "Channel id approvals post into.", required: true },
      { name: "DISCORD_PUBLIC_KEY", hint: "App public key for interaction-signature verification.", required: true },
    ],
    Icon: MessageSquare,
    tintClass: "channel-tint-violet",
  },
  {
    id: "push",
    name: "Push (Expo)",
    description: "Expo Push notification + button-decision route for the SKYCOT mobile app.",
    verdict: "shipped",
    proTier: false,
    envVars: [
      { name: "EXPO_ACCESS_TOKEN", hint: "Expo access token for the push project.", required: true },
      { name: "EXPO_PUSH_ENDPOINT", hint: "Expo push endpoint (default https://exp.host/--/api/v2/push/send).", required: false },
    ],
    Icon: Smartphone,
    tintClass: "channel-tint-amber",
  },
  {
    id: "sms",
    name: "SMS (Twilio)",
    description: "Twilio Messages API outbound. Inbound webhook is owned by sibling brief.",
    verdict: "shipped",
    proTier: false,
    envVars: [
      { name: "TWILIO_ACCOUNT_SID", hint: "Twilio account SID (AC…).", required: true },
      { name: "TWILIO_AUTH_TOKEN", hint: "Twilio auth token.", required: true },
      { name: "TWILIO_FROM_NUMBER", hint: "Verified Twilio number messages send from.", required: true },
    ],
    Icon: MessageCircle,
    tintClass: "channel-tint-sky",
  },
  {
    id: "voice",
    name: "Voice (Twilio)",
    description: "Twilio Voice + DTMF (keypad) inbound webhook with Twilio-HMAC verification.",
    verdict: "shipped",
    proTier: false,
    envVars: [
      { name: "TWILIO_ACCOUNT_SID", hint: "Twilio account SID. Shared with the SMS channel.", required: true },
      { name: "TWILIO_AUTH_TOKEN", hint: "Twilio auth token. Shared with the SMS channel.", required: true },
      { name: "TWILIO_VOICE_FROM_NUMBER", hint: "Voice-capable Twilio number (falls back to TWILIO_FROM_NUMBER).", required: true },
      { name: "ARGO_TWILIO_VOICE_DECISION_URL", hint: "Public URL Twilio POSTs the DTMF decision to.", required: true },
    ],
    Icon: Phone,
    tintClass: "channel-tint-sky",
  },
  {
    id: "whatsapp",
    name: "WhatsApp",
    description: "Twilio WhatsApp + keyword-parser inbound webhook. Outbound currently deferred.",
    verdict: "deferred",
    proTier: false,
    envVars: [
      { name: "TWILIO_ACCOUNT_SID", hint: "Twilio account SID.", required: true },
      { name: "TWILIO_AUTH_TOKEN", hint: "Twilio auth token.", required: true },
      { name: "TWILIO_WHATSAPP_FROM_NUMBER", hint: "WhatsApp-enabled Twilio number (whatsapp:+…).", required: true },
    ],
    Icon: MessageCircle,
    tintClass: "channel-tint-emerald",
    note: "Inbound webhook is wired but `dispatch_notify` refuses outbound (DEFERRED-v1.x).",
  },
  {
    id: "teams",
    name: "Microsoft Teams",
    description: "Microsoft Graph + Bot Framework activity with OAuth client-credentials.",
    verdict: "deferred",
    proTier: false,
    envVars: [
      { name: "TEAMS_TENANT_ID", hint: "Azure AD tenant id.", required: true },
      { name: "TEAMS_CLIENT_ID", hint: "App registration client id.", required: true },
      { name: "TEAMS_CLIENT_SECRET", hint: "App secret. Configure in [channels.teams] then point client_secret_env here.", required: true },
    ],
    Icon: Mic,
    tintClass: "channel-tint-indigo",
    note: "DEFERRED-v1.x: outbound dispatcher refuses Teams; inbound route is also missing.",
  },
  {
    id: "matrix",
    name: "Matrix",
    description: "Matrix Client-Server API. Decentralized chat for self-hosted SKYCOT deployments.",
    verdict: "deferred",
    proTier: false,
    envVars: [
      { name: "MATRIX_HOMESERVER", hint: "Homeserver base URL (e.g. https://matrix.org).", required: true },
      { name: "MATRIX_ACCESS_TOKEN", hint: "Long-lived access token. Point [channels.matrix].access_token_env here.", required: true },
      { name: "MATRIX_ROOM_ID", hint: "Room id approvals post into.", required: true },
    ],
    Icon: MessageSquare,
    tintClass: "channel-tint-amber",
    note: "DEFERRED-v1.x: outbound refused; no inbound webhook.",
  },
] as const;

/** Lookup helper — `undefined` if no channel matches the id. */
export function findChannelById(id: string): NotifyChannelEntry | undefined {
  return NOTIFY_CHANNELS.find((c) => c.id === id);
}

/**
 * Static fallback verdict → probe status mapping. Used when the
 * gateway `/api/doctor/channels` is 404 / unavailable so the page
 * still surfaces a meaningful pill instead of "Unknown".
 */
export function fallbackStatusFor(verdict: ChannelVerdict): ProbeStatus {
  switch (verdict) {
    case "live":
      return "OK";
    case "shipped":
      return "MISSING";
    case "deferred":
      return "WARN";
  }
}
