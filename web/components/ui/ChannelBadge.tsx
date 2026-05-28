import {
  Globe,
  Hash,
  Mail,
  MessageCircle,
  MessageSquare,
  Phone,
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

export interface ChannelBadgeProps
  extends Omit<HTMLAttributes<HTMLSpanElement>, "title">,
    VariantProps<typeof channelBadge> {
  kind: ChannelKind;
  label?: ReactNode;
  /** Icon-only 18px tile for dense rows. Channel name moves to
   *  `title` + `aria-label` for assistive tech. */
  compact?: boolean;
}

export const ChannelBadge = forwardRef<HTMLSpanElement, ChannelBadgeProps>(
  ({ className, kind, label, compact, ...props }, ref) => {
    const Icon = ICONS[kind];
    const name = CHANNEL_LABELS[kind];
    const displayLabel = label ?? kind;
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
          className={cn(channelBadge({ kind, size }), className)}
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
        className={cn(channelBadge({ kind, size }), className)}
        {...props}
      >
        <Icon aria-hidden className="size-3" />
        <span className="capitalize">{displayLabel}</span>
      </span>
    );
  },
);
ChannelBadge.displayName = "ChannelBadge";
