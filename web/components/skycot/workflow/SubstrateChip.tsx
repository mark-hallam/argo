import {
  Activity,
  Cloud,
  Cpu,
  Globe2,
  Monitor,
  Terminal,
  type LucideIcon,
} from "lucide-react";
import type { WorkflowSubstrate } from "@/lib/api/workflows";

export type SubstrateStatus = "idle" | "running" | "completed" | "failed";

export interface SubstrateChipProps {
  substrate: WorkflowSubstrate;
  status: SubstrateStatus;
  compact?: boolean;
  staticPulse?: boolean;
}

const SUBSTRATE_META: Record<
  WorkflowSubstrate,
  { label: string; Icon: LucideIcon; bg: string; fg: string; dot: string }
> = {
  macos: { label: "macOS", Icon: Monitor, bg: "#eef2ff", fg: "#3730a3", dot: "#6366f1" },
  linux: { label: "Linux", Icon: Terminal, bg: "#ecfdf5", fg: "#047857", dot: "#10b981" },
  windows: { label: "Windows", Icon: Monitor, bg: "#eff6ff", fg: "#1d4ed8", dot: "#3b82f6" },
  e2b: { label: "E2B", Icon: Cloud, bg: "#f0fdfa", fg: "#0f766e", dot: "#14b8a6" },
  browser: { label: "Browser", Icon: Globe2, bg: "#fdf4ff", fg: "#a21caf", dot: "#d946ef" },
  headless: { label: "Headless", Icon: Cpu, bg: "#f4f4f5", fg: "#3f3f46", dot: "#52525b" },
};

const STATUS_LABEL: Record<SubstrateStatus, string> = {
  idle: "idle",
  running: "running",
  completed: "completed",
  failed: "failed",
};

export function SubstrateChip({
  substrate,
  status,
  compact = false,
  staticPulse = false,
}: SubstrateChipProps) {
  const meta = SUBSTRATE_META[substrate];
  const Icon = meta.Icon;
  const live = status === "running";
  return (
    <>
      {live && !staticPulse ? (
        <style>
          {`@keyframes substrate-chip-live-pulse {
          0%, 100% { transform: scale(1); opacity: 1; }
          50% { transform: scale(1.45); opacity: 0.55; }
        }`}
        </style>
      ) : null}
      <span
        data-testid="substrate-chip"
        data-substrate={substrate}
        data-status={status}
        aria-label={`${meta.label} substrate ${STATUS_LABEL[status]}`}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: compact ? 5 : 7,
          width: "fit-content",
          minHeight: compact ? 22 : 26,
          padding: compact ? "2px 8px" : "3px 10px",
          borderRadius: 999,
          background: meta.bg,
          color: meta.fg,
          fontFamily: "var(--font-mono)",
          fontSize: compact ? 9 : 10,
          fontWeight: 800,
          letterSpacing: 0,
          lineHeight: 1,
          textTransform: "uppercase",
          whiteSpace: "nowrap",
        }}
      >
        <Icon size={compact ? 11 : 13} aria-hidden />
        <span>{meta.label}</span>
        <span
          aria-hidden
          style={{
            width: compact ? 5 : 6,
            height: compact ? 5 : 6,
            borderRadius: 999,
            background: status === "failed" ? "#ef4444" : meta.dot,
            boxShadow: live ? `0 0 0 4px ${meta.dot}24` : undefined,
            animation: live && !staticPulse
              ? "substrate-chip-live-pulse 1.15s var(--motion-ease) infinite"
              : undefined,
          }}
        />
        <span>{STATUS_LABEL[status]}</span>
        {live ? <Activity size={compact ? 10 : 12} aria-hidden /> : null}
      </span>
    </>
  );
}
