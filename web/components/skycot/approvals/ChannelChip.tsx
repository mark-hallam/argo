import {
  Bell,
  Globe,
  Hash,
  Linkedin,
  Mail,
  MessageCircle,
  MessageSquare,
  Mic,
  Smartphone,
  Twitter,
  type LucideIcon,
} from "lucide-react";
import type { ChannelKind } from "@/lib/sample-approvals";
import { cn } from "@/lib/utils";

/**
 * Claim 6 (multi-channel + multi-tenant) row-level proof on `/approvals`.
 * One chip per approval row carrying `channel`; pairs with the
 * `<ChannelStripCard>` on worker overview (#2391) so the same channel
 * vocabulary lands at both inbox and worker scopes.
 *
 * `x` + `linkedin` are publishing surfaces the gateway's
 * marketing-draft route emits on a queued Marcus draft. Every lookup
 * below resolves through a `?? <fallback>` so a channel value outside
 * the union (a future gateway channel, a typo) renders a sensible
 * default chip instead of returning `undefined` and crashing the
 * whole inbox via the route error boundary — defense-in-depth, not
 * just these two.
 */

const FALLBACK_ICON: LucideIcon = Globe;
const FALLBACK_DOT = "var(--muted-foreground)";

const LABEL: Record<ChannelKind, string> = {
  slack: "Slack",
  discord: "Discord",
  email: "Email",
  imessage: "iMessage",
  sms: "SMS",
  push: "Push",
  web: "Web",
  voice: "Voice",
  whatsapp: "WhatsApp",
  x: "X",
  linkedin: "LinkedIn",
};

const ICON: Record<ChannelKind, LucideIcon> = {
  slack: MessageSquare,
  discord: Hash,
  email: Mail,
  imessage: MessageCircle,
  sms: Smartphone,
  push: Bell,
  web: Globe,
  voice: Mic,
  whatsapp: MessageCircle,
  x: Twitter,
  linkedin: Linkedin,
};

const DOT_COLOR: Record<ChannelKind, string> = {
  slack: "var(--info)",
  discord: "var(--primary)",
  email: "var(--muted-foreground)",
  imessage: "var(--success)",
  sms: "var(--info)",
  push: "var(--primary)",
  web: "var(--muted-foreground)",
  voice: "var(--warning)",
  whatsapp: "var(--success)",
  x: "var(--foreground)",
  linkedin: "var(--info)",
};

/** Title-case an unknown channel token for display (e.g. "in-app" → "In app"). */
function humanizeChannel(token: string): string {
  return token.replace(/[-_]+/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

export interface ChannelChipProps {
  /**
   * A known {@link ChannelKind} gets its branded icon + colour;
   * anything else (a future gateway channel, an unexpected value)
   * renders a sensible fallback chip rather than crashing. The prop
   * is widened to `string` so an off-union value coming off an
   * `any`-typed gateway envelope can never produce `<undefined />`.
   */
  channel: ChannelKind | (string & {});
  size?: "sm" | "md";
  className?: string;
}

export function ChannelChip({ channel, size = "sm", className }: ChannelChipProps) {
  // Resolve icon / label / dot defensively: an unmapped channel must
  // render a default chip, never return `undefined` from the icon map
  // (that renders `<undefined />` and takes the whole inbox down).
  const Icon = ICON[channel as ChannelKind] ?? FALLBACK_ICON;
  const label = LABEL[channel as ChannelKind] ?? humanizeChannel(channel);
  const dot = DOT_COLOR[channel as ChannelKind] ?? FALLBACK_DOT;
  return (
    <span
      data-testid={`channel-chip-${channel}`}
      data-channel={channel}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-[var(--radius-sm)] border",
        "border-[var(--border)] bg-[var(--muted)]/40 text-[var(--muted-foreground)]",
        "font-mono leading-none",
        size === "sm" ? "px-1.5 py-0.5 text-[10px]" : "px-2 py-1 text-[11px]",
        className,
      )}
    >
      <span
        aria-hidden
        className="size-1.5 rounded-full"
        style={{ backgroundColor: dot }}
      />
      <Icon aria-hidden className={cn(size === "sm" ? "size-3" : "size-3.5")} />
      <span>{label}</span>
    </span>
  );
}
