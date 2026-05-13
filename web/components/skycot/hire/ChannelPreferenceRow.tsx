"use client";

import {
  Bell,
  Mail,
  MessageCircle,
  MessageSquare,
  Phone,
  Smartphone,
  type LucideIcon,
} from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Multi-select channel toggle for Step 5 (Connect). Each chip toggles
 * inclusion; the radio dot in the chip designates the primary. Fully
 * controlled — parent owns `selected[]` + `primary` and mutates them.
 *
 * v1 surfaces 7 channels per the brief. The architectural target
 * (`mission-enterprise-channel-routing-policy`) replaces the
 * primary/fallback list with per-channel-kind routing rules; the
 * `Step5Connect` payload sent to the gateway stays
 * `{ primary, fallback[] }` so the routing-policy brief layers on top
 * without rewriting this UI.
 */

export type ChannelKind =
  | "slack"
  | "imessage"
  | "email"
  | "sms"
  | "push"
  | "whatsapp"
  | "voice";

export interface ChannelMeta {
  kind: ChannelKind;
  label: string;
  Icon: LucideIcon;
}

export const CHANNEL_KINDS: readonly ChannelMeta[] = [
  { kind: "slack", label: "Slack", Icon: MessageSquare },
  { kind: "imessage", label: "iMessage", Icon: MessageCircle },
  { kind: "email", label: "Email", Icon: Mail },
  { kind: "sms", label: "SMS", Icon: Smartphone },
  { kind: "push", label: "Push", Icon: Bell },
  { kind: "whatsapp", label: "WhatsApp", Icon: MessageCircle },
  { kind: "voice", label: "Voice", Icon: Phone },
] as const;

export interface ChannelPreferenceRowProps {
  selected: ChannelKind[];
  primary: ChannelKind | null;
  onChange: (next: { selected: ChannelKind[]; primary: ChannelKind | null }) => void;
}

export function ChannelPreferenceRow({
  selected,
  primary,
  onChange,
}: ChannelPreferenceRowProps) {
  function toggle(kind: ChannelKind) {
    const isOn = selected.includes(kind);
    if (isOn) {
      const next = selected.filter((k) => k !== kind);
      const nextPrimary = primary === kind ? (next[0] ?? null) : primary;
      onChange({ selected: next, primary: nextPrimary });
    } else {
      const next = [...selected, kind];
      const nextPrimary = primary ?? kind;
      onChange({ selected: next, primary: nextPrimary });
    }
  }

  function makePrimary(kind: ChannelKind) {
    const next = selected.includes(kind) ? selected : [...selected, kind];
    onChange({ selected: next, primary: kind });
  }

  return (
    <div
      data-testid="hire-step5-channels"
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-4 space-y-3"
    >
      <div className="space-y-1">
        <h3 className="text-sm font-semibold text-[var(--foreground)]">
          How should they reach you for approvals?
        </h3>
        <p className="text-xs text-[var(--muted-foreground)]">
          Pick one or more. The starred one is the primary.
        </p>
      </div>
      <div
        role="group"
        aria-label="Channel preferences"
        className="flex flex-wrap gap-2"
      >
        {CHANNEL_KINDS.map(({ kind, label, Icon }) => {
          const on = selected.includes(kind);
          const isPrimary = primary === kind;
          return (
            <div
              key={kind}
              className={cn(
                "inline-flex items-center gap-1 rounded-full border px-2.5 py-1",
                on
                  ? "border-[var(--primary)] bg-[var(--primary)]/10 text-[var(--foreground)]"
                  : "border-[var(--border)] bg-[var(--background)] text-[var(--muted-foreground)]",
              )}
              data-testid={`hire-step5-channel-${kind}`}
              data-on={on ? "true" : "false"}
              data-primary={isPrimary ? "true" : "false"}
            >
              <button
                type="button"
                onClick={() => toggle(kind)}
                aria-pressed={on}
                aria-label={`Toggle ${label}`}
                data-testid={`hire-step5-channel-${kind}-toggle`}
                className="inline-flex items-center gap-1.5 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] rounded-full"
              >
                <Icon className="size-3.5" aria-hidden />
                {label}
              </button>
              <button
                type="button"
                onClick={() => makePrimary(kind)}
                aria-label={`Make ${label} primary`}
                aria-pressed={isPrimary}
                disabled={!on}
                data-testid={`hire-step5-channel-${kind}-primary`}
                className={cn(
                  "ml-1 grid size-4 place-items-center rounded-full border text-[10px] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                  isPrimary
                    ? "border-[var(--primary)] bg-[var(--primary)] text-[var(--primary-foreground)]"
                    : "border-[var(--border)] bg-transparent text-[var(--muted-foreground)] disabled:opacity-40",
                )}
              >
                {isPrimary ? "★" : "☆"}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}

/**
 * Auto-detect the default primary channel for a freshly-rendered Step 5.
 * v1 reads from a cookie/session hint if available; otherwise falls
 * back to Slack per the brief. The hint string is set by the auth
 * provider after sign-in.
 */
export function detectDefaultPrimary(hint: string | null | undefined): ChannelKind {
  switch (hint) {
    case "slack":
    case "imessage":
    case "email":
    case "sms":
    case "push":
    case "whatsapp":
    case "voice":
      return hint;
    default:
      return "slack";
  }
}
