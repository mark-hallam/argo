/**
 * Canonical ChannelKind union for the SPA — single source of truth for
 * every Record<ChannelKind, …> callsite (filter chips, badge styles,
 * channel-strip cards, etc.).
 *
 * The list matches the operator-facing notify-channels catalog at
 * `lib/notify-channels/registry.ts`. Discord is included because PR
 * #2222 wired Discord interaction-signature verification on the
 * gateway side — the SPA's narrower union was silently coercing
 * Discord values to the unknown-channel fallback.
 *
 * Older modules re-export `ChannelKind` from here so existing
 * `import { ChannelKind } from "@/lib/sample-approvals"` callsites
 * keep compiling without churn.
 */
export type ChannelKind =
  | "slack"
  | "discord"
  | "email"
  | "sms"
  | "imessage"
  | "whatsapp"
  | "voice"
  | "push"
  | "web";

export const CHANNEL_KINDS: readonly ChannelKind[] = [
  "slack",
  "discord",
  "email",
  "sms",
  "imessage",
  "whatsapp",
  "voice",
  "push",
  "web",
] as const;
