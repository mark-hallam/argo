"use client";

import {
  Bell,
  Mail,
  MessageCircle,
  MessageSquare,
  Phone,
  Plus,
  Radio,
  Smartphone,
  Trash2,
} from "lucide-react";
import type { ComponentType } from "react";
import { cn } from "@/lib/utils";

export type ApprovalRouteChannelKind =
  | "slack"
  | "sms"
  | "email"
  | "push"
  | "web_inbox"
  | "voice"
  | "chat"
  | "sse";

export interface ApprovalRouteNode {
  id: string;
  channel: {
    kind: ApprovalRouteChannelKind;
    target: string;
  };
  timeoutMs: number;
  fallback?: ApprovalRouteNode;
}

export interface ApprovalRoutingPlan {
  id: string;
  label: string;
  root: ApprovalRouteNode;
}

const CHANNELS: Array<{
  kind: ApprovalRouteChannelKind;
  label: string;
  icon: ComponentType<{ className?: string; "aria-hidden"?: boolean }>;
}> = [
  { kind: "slack", label: "Slack", icon: MessageSquare },
  { kind: "sms", label: "SMS", icon: Smartphone },
  { kind: "email", label: "Email", icon: Mail },
  { kind: "push", label: "Push", icon: Bell },
  { kind: "web_inbox", label: "Inbox", icon: MessageCircle },
  { kind: "voice", label: "Voice", icon: Phone },
  { kind: "chat", label: "Chat", icon: MessageCircle },
  { kind: "sse", label: "SSE", icon: Radio },
];

export function ApprovalRoutingEditor({
  plan,
  onChange,
}: {
  plan: ApprovalRoutingPlan;
  onChange?: (plan: ApprovalRoutingPlan) => void;
}) {
  const updateRoot = (root: ApprovalRouteNode) => onChange?.({ ...plan, root });
  return (
    <section
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-4"
      data-testid="approval-routing-editor"
    >
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold text-[var(--foreground)]">{plan.label}</h3>
          <p className="text-xs text-[var(--muted-foreground)]">{plan.id}</p>
        </div>
        <div className="text-xs font-medium text-[var(--muted-foreground)]">
          {countNodes(plan.root)} channels
        </div>
      </div>
      <RouteNodeEditor node={plan.root} depth={0} onChange={updateRoot} />
    </section>
  );
}

function RouteNodeEditor({
  node,
  depth,
  onChange,
  onRemove,
}: {
  node: ApprovalRouteNode;
  depth: number;
  onChange: (node: ApprovalRouteNode) => void;
  onRemove?: () => void;
}) {
  const selected = CHANNELS.find((channel) => channel.kind === node.channel.kind) ?? CHANNELS[0];
  const Icon = selected.icon;
  const addFallback = () => {
    onChange({
      ...node,
      fallback: {
        id: `${node.id}-fallback`,
        channel: { kind: "email", target: "ops@example.com" },
        timeoutMs: 4 * 60 * 60 * 1000,
      },
    });
  };
  return (
    <div className={cn("relative", depth > 0 && "ml-6 border-l border-[var(--border)] pl-4")}>
      <div className="grid gap-3 rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--background)] p-3 md:grid-cols-[minmax(130px,180px)_1fr_130px_auto]">
        <label className="grid gap-1 text-xs font-medium text-[var(--muted-foreground)]">
          Channel
          <span className="relative">
            <Icon className="pointer-events-none absolute left-2 top-1/2 size-4 -translate-y-1/2 text-[var(--muted-foreground)]" aria-hidden />
            <select
              value={node.channel.kind}
              onChange={(event) =>
                onChange({
                  ...node,
                  channel: {
                    ...node.channel,
                    kind: event.target.value as ApprovalRouteChannelKind,
                  },
                })
              }
              className="h-9 w-full rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)] pl-8 pr-2 text-sm text-[var(--foreground)]"
            >
              {CHANNELS.map((channel) => (
                <option key={channel.kind} value={channel.kind}>
                  {channel.label}
                </option>
              ))}
            </select>
          </span>
        </label>
        <label className="grid gap-1 text-xs font-medium text-[var(--muted-foreground)]">
          Target
          <input
            value={node.channel.target}
            onChange={(event) =>
              onChange({ ...node, channel: { ...node.channel, target: event.target.value } })
            }
            className="h-9 rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)] px-3 text-sm text-[var(--foreground)]"
          />
        </label>
        <label className="grid gap-1 text-xs font-medium text-[var(--muted-foreground)]">
          Timeout
          <input
            type="number"
            min={1}
            value={msToMinutes(node.timeoutMs)}
            onChange={(event) =>
              onChange({ ...node, timeoutMs: minutesToMs(Number(event.target.value)) })
            }
            className="h-9 rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)] px-3 text-sm text-[var(--foreground)]"
          />
        </label>
        <div className="flex items-end gap-1">
          {!node.fallback ? (
            <IconButton label="Add fallback channel" icon={Plus} onClick={addFallback} />
          ) : null}
          {depth > 0 && onRemove ? (
            <IconButton label="Remove channel" icon={Trash2} onClick={onRemove} />
          ) : null}
        </div>
      </div>
      {node.fallback ? (
        <div className="mt-3">
          <RouteNodeEditor
            node={node.fallback}
            depth={depth + 1}
            onChange={(fallback) => onChange({ ...node, fallback })}
            onRemove={() => onChange({ ...node, fallback: undefined })}
          />
        </div>
      ) : null}
    </div>
  );
}

function IconButton({
  label,
  icon: Icon,
  onClick,
}: {
  label: string;
  icon: ComponentType<{ className?: string; "aria-hidden"?: boolean }>;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className="inline-flex size-9 items-center justify-center rounded-[var(--radius-sm)] border border-[var(--border)] text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--foreground)]"
    >
      <Icon className="size-4" aria-hidden />
    </button>
  );
}

function countNodes(node: ApprovalRouteNode): number {
  return 1 + (node.fallback ? countNodes(node.fallback) : 0);
}

function msToMinutes(ms: number): number {
  return Math.max(1, Math.round(ms / 60000));
}

function minutesToMs(minutes: number): number {
  return Math.max(1, Math.round(minutes || 1)) * 60000;
}
