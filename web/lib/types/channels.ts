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
 *
 * `x` (X/Twitter) + `linkedin` are publishing surfaces, not
 * notify-channels: the gateway's marketing-draft route
 * (`routes_marketing_draft_response.rs`) emits `channel: "x" |
 * "linkedin"` on the approval envelope for a queued Marcus draft.
 * They belong in this union so `/approvals` can render a channel
 * chip for those rows — a narrower union silently coerced them to
 * `undefined` in the chip's icon map and crashed the whole inbox.
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
  | "web"
  | "x"
  | "linkedin";

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
  "x",
  "linkedin",
] as const;
