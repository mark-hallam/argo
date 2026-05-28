/**
 * Operator-facing catalog for the **inbound** half of the approval
 * channels — webhook URLs the operator must paste into the upstream
 * console (Twilio, Discord) so reply messages reach the gateway.
 *
 * Distinct from `web/lib/notify-channels/registry.ts` (which catalogs
 * the outbound side — "where SKYCOT sends approval requests"). The
 * two surfaces sit side-by-side under `/settings/notify-channels` +
 * `/settings/inbound-channels`.
 *
 * The webhook paths here MUST stay in sync with the route predicates
 * in `crates/argo-cli/src/handlers/gateway/routes_channels_*.rs`:
 *
 * - `routes_channels_sms.rs:is_sms_webhook` → `/twilio/sms/decision`
 * - `routes_channels_discord.rs:is_discord_decision` → `/channels/discord/decision`
 * - `routes_channels_voice.rs:is_voice_webhook` → `/twilio/voice/decision`
 */
import type { LucideIcon } from "lucide-react";
import { MessageCircle, MessageSquare, Phone } from "lucide-react";

export type InboundKind = "sms" | "discord" | "voice";

export interface InboundChannelEntry {
  /** Channel id consumed by `GET /api/v1/inbound-channels/{kind}/recent`. */
  id: InboundKind;
  /** Operator-facing name. */
  name: string;
  /** One-line blurb. */
  description: string;
  /** Path the gateway listens on for this kind. */
  webhookPath: string;
  /** Console copy — which upstream console the operator pastes this into. */
  upstreamConsole: string;
  /** Env vars the operator must configure for the route to authenticate. */
  envVars: readonly string[];
  /** Inline copy explaining how to send a test message. */
  testHowTo: string;
  /** Lucide icon. */
  Icon: LucideIcon;
  /** Background tint class shared with the outbound catalog. */
  tintClass: string;
}

export const INBOUND_CHANNELS: readonly InboundChannelEntry[] = [
  {
    id: "sms",
    name: "SMS (Twilio)",
    description:
      "Twilio Messaging webhook. Operators reply YES / NO with the approval token; the route verifies the Twilio HMAC then resolves the approval.",
    webhookPath: "/twilio/sms/decision",
    upstreamConsole: "Twilio Console → Phone Numbers → Active number → Messaging webhook",
    envVars: ["TWILIO_AUTH_TOKEN", "ARGO_CHANNEL_HMAC_SECRET"],
    testHowTo:
      "Send an SMS reply containing `yes <approval-token>` to the configured Twilio number, OR use the Twilio Console’s “Send Webhook test request” button against this URL.",
    Icon: MessageCircle,
    tintClass: "channel-tint-sky",
  },
  {
    id: "discord",
    name: "Discord",
    description:
      "Discord Application Interactions endpoint. Button clicks on approval messages POST a signed interaction here; the route verifies the Ed25519 signature.",
    webhookPath: "/channels/discord/decision",
    upstreamConsole:
      "Discord Developer Portal → Application → General Information → Interactions Endpoint URL",
    envVars: ["DISCORD_PUBLIC_KEY", "ARGO_CHANNEL_HMAC_SECRET"],
    testHowTo:
      "Save the Interactions Endpoint URL in the Discord Developer Portal — Discord pings the URL with a signed PING and refuses to save unless the signature verifies.",
    Icon: MessageSquare,
    tintClass: "channel-tint-violet",
  },
  {
    id: "voice",
    name: "Voice DTMF (Twilio)",
    description:
      "Twilio Voice webhook for DTMF (keypad) approval replies. Press 1 for approve, 2 for reject; the route verifies the Twilio HMAC then resolves the approval.",
    webhookPath: "/twilio/voice/decision",
    upstreamConsole: "Twilio Console → Phone Numbers → Active number → Voice & Fax → A call comes in",
    envVars: ["TWILIO_AUTH_TOKEN", "ARGO_TWILIO_VOICE_DECISION_URL", "ARGO_CHANNEL_HMAC_SECRET"],
    testHowTo:
      "Place a test call to the Twilio number; Twilio POSTs DTMF results to this URL. The Twilio Voice debugger replays a known-good DTMF event when verifying configuration.",
    Icon: Phone,
    tintClass: "channel-tint-sky",
  },
] as const;

export function findInboundChannelById(id: string): InboundChannelEntry | undefined {
  return INBOUND_CHANNELS.find((c) => c.id === (id as InboundKind));
}
