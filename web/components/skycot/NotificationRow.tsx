"use client";

import {
  AlertOctagon,
  AlertTriangle,
  Bell,
  CheckCircle2,
  DollarSign,
  FileText,
  KeyRound,
  PlayCircle,
  RefreshCw,
  Sparkles,
  TrendingDown,
  X,
  Zap,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import type { NotificationItem, NotificationKind } from "@/lib/sample-notifications";

type IconTone = "info" | "success" | "warning" | "destructive";

interface KindMeta {
  Icon: LucideIcon;
  tone: IconTone;
}

const KIND_META: Record<NotificationKind, KindMeta> = {
  "budget-approaching": { Icon: DollarSign, tone: "warning" },
  "budget-exceeded": { Icon: AlertTriangle, tone: "destructive" },
  "skill-promoted": { Icon: Sparkles, tone: "success" },
  "skill-drift-detected": { Icon: TrendingDown, tone: "warning" },
  "oauth-needs-reauth": { Icon: KeyRound, tone: "warning" },
  "duty-output-ready": { Icon: FileText, tone: "info" },
  "autonomy-promotion-suggested": { Icon: Zap, tone: "info" },
  incident: { Icon: AlertOctagon, tone: "destructive" },
  // Live gateway kinds (seeded by argo-demo-seed/seed_notifications.rs):
  "approval-pending": { Icon: CheckCircle2, tone: "warning" },
  "workflow-run-complete": { Icon: PlayCircle, tone: "success" },
  "skill-reeval": { Icon: RefreshCw, tone: "info" },
};

/**
 * Fallback meta for any `kind` the gateway emits that isn't in
 * `KIND_META`. The gateway's notification `kind` is a free-form string,
 * so the SPA must degrade gracefully on an unknown kind rather than
 * crash — an unguarded `KIND_META[item.kind]` destructure previously
 * took the whole bell-popover (and its host page) down with a
 * `TypeError` whenever the live data carried a kind the union missed.
 */
const FALLBACK_META: KindMeta = { Icon: Bell, tone: "info" };

const TONE_CLASSES: Record<IconTone, string> = {
  info: "text-[var(--primary)] bg-[var(--primary)]/10",
  success: "text-[var(--success)] bg-[var(--success)]/10",
  warning: "text-[var(--warning,var(--primary))] bg-[var(--warning,var(--primary))]/10",
  destructive: "text-[var(--destructive)] bg-[var(--destructive)]/10",
};

function toRelative(iso: string): string {
  const diff = Math.round((Date.now() - new Date(iso).getTime()) / 1000);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.round(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.round(diff / 3600)}h ago`;
  return `${Math.round(diff / 86400)}d ago`;
}

function RelativeTime({ iso }: { iso: string }) {
  const [label, setLabel] = useState<string | null>(null);
  useEffect(() => {
    setLabel(toRelative(iso));
    const tick = setInterval(() => setLabel(toRelative(iso)), 30_000);
    return () => clearInterval(tick);
  }, [iso]);
  if (label == null) return null;
  return (
    <time dateTime={iso} title={iso} className="tabular-nums">
      {label}
    </time>
  );
}

export interface NotificationRowProps {
  item: NotificationItem;
  onDismiss: (id: string) => void;
  /** Handles CTAs whose `action` is `"callback"`; routes by `kind`. */
  onCtaCallback?: (item: NotificationItem) => void;
}

export function NotificationRow({ item, onDismiss, onCtaCallback }: NotificationRowProps) {
  const { Icon, tone } = KIND_META[item.kind] ?? FALLBACK_META;
  const unread = !item.read;

  function handleCta() {
    if (!item.cta) return;
    if (item.cta.action === "callback") onCtaCallback?.(item);
    if (item.cta.href && typeof window !== "undefined") {
      window.location.assign(item.cta.href);
    }
    if (item.cta.action === "dismiss" || item.cta.action === "callback") {
      onDismiss(item.id);
    }
  }

  return (
    <div
      data-testid={`notification-row-${item.id}`}
      data-unread={unread || undefined}
      className={cn(
        "group relative flex items-start gap-3 px-4 py-3 transition-colors",
        unread ? "bg-[var(--muted)]/40 hover:bg-[var(--muted)]/55" : "hover:bg-[var(--muted)]/30",
      )}
    >
      {unread ? (
        <span aria-hidden className="absolute left-0 top-0 h-full w-[3px] bg-[var(--primary)]" />
      ) : null}
      <span
        aria-hidden
        className={cn(
          "mt-0.5 grid size-8 shrink-0 place-items-center rounded-full",
          TONE_CLASSES[tone],
        )}
      >
        <Icon className="size-4" />
      </span>
      <div className="min-w-0 flex-1 space-y-1">
        <p
          className={cn(
            "text-[13px] leading-snug",
            unread ? "font-medium text-[var(--foreground)]" : "text-[var(--muted-foreground)]",
          )}
        >
          {item.title}
        </p>
        <div className="flex items-center gap-2 text-[11px] text-[var(--muted-foreground)]">
          <RelativeTime iso={item.createdAt} />
          {item.cta ? (
            <>
              <span aria-hidden>·</span>
              <Button
                variant="link"
                size="sm"
                className="h-auto p-0 text-[11px]"
                onClick={handleCta}
                data-testid={`notification-cta-${item.id}`}
              >
                {item.cta.label}
              </Button>
            </>
          ) : null}
        </div>
      </div>
      <Button
        variant="ghost"
        size="icon"
        aria-label="Dismiss"
        className="size-7 shrink-0 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
        onClick={() => onDismiss(item.id)}
        data-testid={`notification-dismiss-${item.id}`}
      >
        <X />
      </Button>
    </div>
  );
}
