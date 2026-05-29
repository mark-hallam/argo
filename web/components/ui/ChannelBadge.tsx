import {
  Bot,
  Globe,
  Hash,
  Mail,
  MessageCircle,
  MessageSquare,
  MonitorSmartphone,
  Phone,
  Plug,
  Radio,
  Slack,
  Smartphone,
  type LucideIcon,
} from "lucide-react";
import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/utils";
import type { ChannelKind } from "@/lib/types/channels";

export type { ChannelKind } from "@/lib/types/channels";

const channelBadge = cva(
  "inline-flex items-center rounded-[var(--radius-sm)] border font-medium",
  {
    variants: {
      kind: {
        slack: "border-[var(--info)]/30 bg-[var(--info)]/10 text-[var(--info)]",
        discord:
          "border-[#5865F2]/30 bg-[#5865F2]/10 text-[#5865F2]",
        email:
          "border-[var(--border)] bg-[var(--muted)] text-[var(--muted-foreground)]",
        sms: "border-[var(--success)]/30 bg-[var(--success)]/10 text-[var(--success)]",
        imessage:
          "border-[var(--info)]/30 bg-[var(--info)]/10 text-[var(--info)]",
        whatsapp:
          "border-[var(--success)]/30 bg-[var(--success)]/10 text-[var(--success)]",
        voice:
          "border-[var(--warning)]/30 bg-[var(--warning)]/10 text-[var(--warning)]",
        push:
          "border-[var(--border)] bg-[var(--muted)] text-[var(--muted-foreground)]",
        web:
          "border-[var(--border)] bg-[var(--muted)] text-[var(--muted-foreground)]",
      },
      size: {
        default: "gap-1 px-2 py-0.5 text-xs",
        compact: "size-[18px] justify-center p-0",
      },
    },
    defaultVariants: { kind: "email", size: "default" },
  },
);

const ICONS: Record<ChannelKind, LucideIcon> = {
  slack: Slack,
  discord: Hash,
  email: Mail,
  sms: MessageSquare,
  imessage: MessageCircle,
  whatsapp: MessageCircle,
  voice: Phone,
  push: Smartphone,
  web: Globe,
};

const CHANNEL_LABELS: Record<ChannelKind, string> = {
  slack: "Slack",
  discord: "Discord",
  email: "Email",
  sms: "SMS",
  imessage: "iMessage",
  whatsapp: "WhatsApp",
  voice: "Voice",
  push: "Push",
  web: "Web inbox",
};

// Audit rows also carry non-messaging *action surfaces* (an agent acting
// on its own, an MCP tool call, a computer-use session). These are NOT
// messaging channels you hire/notify on, so they intentionally stay out
// of the `ChannelKind` taxonomy — but `/audit` still renders a badge for
// them, so give them a sensible icon + label here.
const EXTRA_SURFACE_ICONS: Record<string, LucideIcon> = {
  agent: Bot,
  mcp: Plug,
  "computer-use": MonitorSmartphone,
};
const EXTRA_SURFACE_LABELS: Record<string, string> = {
  agent: "Agent",
  mcp: "MCP",
  "computer-use": "Computer use",
};
// Last-resort icon for any value neither a known channel nor a known
// extra surface — guarantees a badge never renders `<undefined />` and
// crashes the page (the bug this guard closes: gateway-emitted channel
// values outside the SPA's union took down the whole /audit feed).
const FALLBACK_ICON: LucideIcon = Radio;

const KNOWN_CHANNELS = new Set<string>(Object.keys(ICONS));

/** Title-case an unknown surface token for display (e.g. "in-app" → "In app"). */
function humanizeSurface(token: string): string {
  return token
    .replace(/[-_]+/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

export interface ChannelBadgeProps
  extends Omit<HTMLAttributes<HTMLSpanElement>, "title" | "kind">,
    Omit<VariantProps<typeof channelBadge>, "kind"> {
  /**
   * Channel or audit action-surface. A known {@link ChannelKind} gets
   * its branded icon + colour; anything else (e.g. `agent` / `mcp` /
   * `computer-use` from the audit feed, or a future gateway value)
   * renders a sensible fallback icon + label rather than crashing.
   */
  kind: ChannelKind | (string & {});
  label?: ReactNode;
  /** Icon-only 18px tile for dense rows. Channel name moves to
   *  `title` + `aria-label` for assistive tech. */
  compact?: boolean;
}

export const ChannelBadge = forwardRef<HTMLSpanElement, ChannelBadgeProps>(
  ({ className, kind, label, compact, ...props }, ref) => {
    // `kind` may be a known messaging channel, a known audit surface, or
    // an unexpected gateway value. Resolve icon + label defensively so a
    // single unknown value can never render `<undefined />` and take the
    // whole page down via the route error boundary.
    const safeKind = KNOWN_CHANNELS.has(kind) ? (kind as ChannelKind) : undefined;
    const Icon =
      (safeKind && ICONS[safeKind]) ?? EXTRA_SURFACE_ICONS[kind] ?? FALLBACK_ICON;
    const name =
      (safeKind && CHANNEL_LABELS[safeKind]) ??
      EXTRA_SURFACE_LABELS[kind] ??
      humanizeSurface(kind);
    const displayLabel = label ?? name;
    const size = compact ? "compact" : "default";
    if (compact) {
      return (
        <span
          ref={ref}
          data-testid="channel-badge"
          data-kind={kind}
          data-compact="true"
          aria-label={name}
          title={name}
          className={cn(channelBadge({ kind: safeKind, size }), className)}
          {...props}
        >
          <Icon aria-hidden className="size-3" />
        </span>
      );
    }
    return (
      <span
        ref={ref}
        data-testid="channel-badge"
        data-kind={kind}
        className={cn(channelBadge({ kind: safeKind, size }), className)}
        {...props}
      >
        <Icon aria-hidden className="size-3" />
        <span className="capitalize">{displayLabel}</span>
      </span>
    );
  },
);
ChannelBadge.displayName = "ChannelBadge";
