import {
  Bell,
  Globe,
  Hash,
  Mail,
  MessageCircle,
  MessageSquare,
  Mic,
  Smartphone,
  type LucideIcon,
} from "lucide-react";
import type { ChannelKind } from "@/lib/sample-approvals";
import { cn } from "@/lib/utils";

/**
 * Claim 6 (multi-channel + multi-tenant) row-level proof on `/approvals`.
 * One chip per approval row carrying `channel`; pairs with the
 * `<ChannelStripCard>` on worker overview (#2391) so the same channel
 * vocabulary lands at both inbox and worker scopes.
 */

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
};

export interface ChannelChipProps {
  channel: ChannelKind;
  size?: "sm" | "md";
  className?: string;
}

export function ChannelChip({ channel, size = "sm", className }: ChannelChipProps) {
  const Icon = ICON[channel];
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
        style={{ backgroundColor: DOT_COLOR[channel] }}
      />
      <Icon aria-hidden className={cn(size === "sm" ? "size-3" : "size-3.5")} />
      <span>{LABEL[channel]}</span>
    </span>
  );
}
