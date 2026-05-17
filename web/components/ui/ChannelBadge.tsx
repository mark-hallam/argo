import { Mail, MessageSquare, Phone, Slack } from "lucide-react";
import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export type ChannelKind = "slack" | "email" | "sms" | "voice";

const channelBadge = cva(
  "inline-flex items-center gap-1 rounded-[var(--radius-sm)] border px-2 py-0.5 text-xs font-medium",
  {
    variants: {
      kind: {
        slack: "border-[var(--info)]/30 bg-[var(--info)]/10 text-[var(--info)]",
        email:
          "border-[var(--border)] bg-[var(--muted)] text-[var(--muted-foreground)]",
        sms: "border-[var(--success)]/30 bg-[var(--success)]/10 text-[var(--success)]",
        voice:
          "border-[var(--warning)]/30 bg-[var(--warning)]/10 text-[var(--warning)]",
      },
    },
    defaultVariants: { kind: "email" },
  },
);

const ICONS = {
  slack: Slack,
  email: Mail,
  sms: MessageSquare,
  voice: Phone,
} as const;

export interface ChannelBadgeProps
  extends HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof channelBadge> {
  kind: ChannelKind;
  label?: ReactNode;
}

export const ChannelBadge = forwardRef<HTMLSpanElement, ChannelBadgeProps>(
  ({ className, kind, label, ...props }, ref) => {
    const Icon = ICONS[kind];
    const displayLabel = label ?? kind;
    return (
      <span
        ref={ref}
        data-testid="channel-badge"
        data-kind={kind}
        className={cn(channelBadge({ kind }), className)}
        {...props}
      >
        <Icon aria-hidden className="size-3" />
        <span className="capitalize">{displayLabel}</span>
      </span>
    );
  },
);
ChannelBadge.displayName = "ChannelBadge";
