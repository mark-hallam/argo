"use client";

import { useMemo } from "react";

import {
  scheduleConfigSchema,
  SCHEDULE_MODES,
  type ScheduleConfig,
  type ScheduleMode,
} from "./types";
import {
  TextField,
  NumberField,
  FieldShell,
  PillButton,
  triggerStyles,
} from "./shared";

interface Props {
  config: ScheduleConfig;
  onChange: (next: ScheduleConfig) => void;
}

const MODE_LABEL: Record<ScheduleMode, string> = {
  minute: "Every minute",
  hour: "Every hour",
  day: "Every day",
  week: "Every week",
  month: "Every month",
  custom: "Custom cron",
};

const DAY_LABELS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

// Subset of IANA timezones — keeps the picker manageable for v1.
// Full IANA list is fetched from the workspace settings in a follow-up
// brief once that endpoint exists.
const COMMON_TIMEZONES = [
  "UTC",
  "Australia/Melbourne",
  "Australia/Sydney",
  "Australia/Brisbane",
  "Australia/Perth",
  "America/New_York",
  "America/Los_Angeles",
  "America/Chicago",
  "Europe/London",
  "Europe/Berlin",
  "Asia/Singapore",
  "Asia/Tokyo",
  "Asia/Kolkata",
];

export default function ScheduleConfigForm({ config, onChange }: Props) {
  const parsed = scheduleConfigSchema.safeParse(config);
  const errors = parsed.success ? {} : flattenErrors(parsed.error.issues);

  function patch(p: Partial<ScheduleConfig>) {
    onChange({ ...config, ...p });
  }

  function setMode(mode: ScheduleMode) {
    const next: ScheduleConfig = { ...config, mode };
    next.cronExpression = buildCron(next);
    onChange(next);
  }

  function toggleDay(d: number) {
    const days = config.daysOfWeek ?? [];
    const nextDays = days.includes(d) ? days.filter((x) => x !== d) : [...days, d].sort();
    const next: ScheduleConfig = { ...config, daysOfWeek: nextDays };
    next.cronExpression = buildCron(next);
    onChange(next);
  }

  const previewRuns = useMemo(() => nextNRuns(config, 5), [config]);
  const summary = useMemo(() => explainSchedule(config), [config]);

  return (
    <div data-testid="schedule-config-form">
      <FieldShell label="Frequency">
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          {SCHEDULE_MODES.map((m) => (
            <PillButton key={m} active={config.mode === m} onClick={() => setMode(m)}>
              {MODE_LABEL[m]}
            </PillButton>
          ))}
        </div>
      </FieldShell>

      {(config.mode === "day" || config.mode === "week" || config.mode === "month") ? (
        <div style={{ display: "flex", gap: 12 }}>
          <div style={{ flex: 1 }}>
            <NumberField
              id="schedule-hour"
              label="Hour (0–23)"
              value={config.hour ?? 0}
              min={0}
              max={23}
              onChange={(v) => {
                const next: ScheduleConfig = { ...config, hour: v };
                next.cronExpression = buildCron(next);
                onChange(next);
              }}
            />
          </div>
          <div style={{ flex: 1 }}>
            <NumberField
              id="schedule-minute"
              label="Minute (0–59)"
              value={config.minute ?? 0}
              min={0}
              max={59}
              onChange={(v) => {
                const next: ScheduleConfig = { ...config, minute: v };
                next.cronExpression = buildCron(next);
                onChange(next);
              }}
            />
          </div>
        </div>
      ) : null}

      {config.mode === "hour" ? (
        <NumberField
          id="schedule-minute"
          label="Minute of the hour"
          value={config.minute ?? 0}
          min={0}
          max={59}
          onChange={(v) => {
            const next: ScheduleConfig = { ...config, minute: v };
            next.cronExpression = buildCron(next);
            onChange(next);
          }}
        />
      ) : null}

      {config.mode === "week" ? (
        <FieldShell label="Days of week" error={errors.daysOfWeek}>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }} data-testid="schedule-days">
            {DAY_LABELS.map((d, i) => (
              <PillButton key={d} active={(config.daysOfWeek ?? []).includes(i)} onClick={() => toggleDay(i)}>
                {d}
              </PillButton>
            ))}
          </div>
        </FieldShell>
      ) : null}

      {config.mode === "month" ? (
        <NumberField
          id="schedule-dayofmonth"
          label="Day of month"
          value={config.dayOfMonth ?? 1}
          min={1}
          max={31}
          onChange={(v) => {
            const next: ScheduleConfig = { ...config, dayOfMonth: v };
            next.cronExpression = buildCron(next);
            onChange(next);
          }}
        />
      ) : null}

      {config.mode === "custom" ? (
        <TextField
          id="schedule-cron"
          label="Cron expression"
          value={config.cronExpression ?? ""}
          onChange={(v) => patch({ cronExpression: v })}
          placeholder="0 9 * * 1-5"
          error={errors.cronExpression ?? cronSyntaxWarning(config.cronExpression ?? "")}
          hint="Five fields: minute hour day month day-of-week."
        />
      ) : null}

      <FieldShell label="Timezone" hint="Defaults to your workspace timezone.">
        <select
          value={config.timezone}
          onChange={(e) => patch({ timezone: e.target.value })}
          data-testid="schedule-timezone"
          style={triggerStyles.input}
        >
          {COMMON_TIMEZONES.map((tz) => (
            <option key={tz} value={tz}>{tz}</option>
          ))}
        </select>
      </FieldShell>

      <FieldShell label="Cron explained">
        <div
          data-testid="schedule-summary"
          style={{
            padding: "10px 12px",
            borderRadius: 8,
            background: "rgba(139,92,246,0.08)",
            border: "1px solid rgba(139,92,246,0.2)",
            color: "#5b21b6",
            fontSize: 13,
            fontFamily: "Inter, sans-serif",
          }}
        >
          {summary}
        </div>
      </FieldShell>

      <FieldShell label="Next 5 runs">
        <ul data-testid="schedule-next-runs" style={{ listStyle: "none", padding: 0, margin: 0 }}>
          {previewRuns.map((r) => (
            <li
              key={r}
              style={{
                padding: "6px 10px",
                borderBottom: "1px solid #ece9f5",
                fontFamily: "JetBrains Mono, monospace",
                fontSize: 12,
                color: "#3f3f46",
              }}
            >
              {r}
            </li>
          ))}
        </ul>
      </FieldShell>
    </div>
  );
}

// ---------- cron helpers (deterministic, no extra deps) -----------

function pad2(n: number): string {
  return n < 10 ? `0${n}` : `${n}`;
}

export function buildCron(cfg: ScheduleConfig): string {
  switch (cfg.mode) {
    case "minute":
      return "* * * * *";
    case "hour":
      return `${cfg.minute ?? 0} * * * *`;
    case "day":
      return `${cfg.minute ?? 0} ${cfg.hour ?? 0} * * *`;
    case "week": {
      const days = (cfg.daysOfWeek && cfg.daysOfWeek.length > 0 ? cfg.daysOfWeek : [1]).join(",");
      return `${cfg.minute ?? 0} ${cfg.hour ?? 0} * * ${days}`;
    }
    case "month":
      return `${cfg.minute ?? 0} ${cfg.hour ?? 0} ${cfg.dayOfMonth ?? 1} * *`;
    case "custom":
      return cfg.cronExpression ?? "* * * * *";
  }
}

export function explainSchedule(cfg: ScheduleConfig): string {
  const tz = cfg.timezone || "UTC";
  switch (cfg.mode) {
    case "minute":
      return `Runs every minute in ${tz}.`;
    case "hour":
      return `Runs at minute ${cfg.minute ?? 0} of every hour in ${tz}.`;
    case "day":
      return `Runs at ${pad2(cfg.hour ?? 0)}:${pad2(cfg.minute ?? 0)} every day in ${tz}.`;
    case "week": {
      const days = (cfg.daysOfWeek && cfg.daysOfWeek.length > 0 ? cfg.daysOfWeek : [])
        .map((d) => DAY_LABELS[d])
        .join(", ");
      return `Runs at ${pad2(cfg.hour ?? 0)}:${pad2(cfg.minute ?? 0)} on ${days || "(no days)"} in ${tz}.`;
    }
    case "month":
      return `Runs at ${pad2(cfg.hour ?? 0)}:${pad2(cfg.minute ?? 0)} on day ${cfg.dayOfMonth ?? 1} of each month in ${tz}.`;
    case "custom":
      return `Runs on cron \`${cfg.cronExpression ?? ""}\` in ${tz}.`;
  }
}

// Lightweight next-runs preview — accurate for the predefined modes,
// best-effort for custom cron. A full cron evaluator lives on the
// gateway side; this is just UX confidence.
export function nextNRuns(cfg: ScheduleConfig, count: number, from: Date = new Date()): string[] {
  const out: string[] = [];
  switch (cfg.mode) {
    case "minute": {
      const d = new Date(from);
      d.setSeconds(0, 0);
      d.setMinutes(d.getMinutes() + 1);
      for (let i = 0; i < count; i++) {
        out.push(d.toISOString().replace(/\.\d{3}Z$/, "Z"));
        d.setMinutes(d.getMinutes() + 1);
      }
      return out;
    }
    case "hour": {
      const d = new Date(from);
      d.setSeconds(0, 0);
      d.setMinutes(cfg.minute ?? 0);
      if (d <= from) d.setHours(d.getHours() + 1);
      for (let i = 0; i < count; i++) {
        out.push(d.toISOString().replace(/\.\d{3}Z$/, "Z"));
        d.setHours(d.getHours() + 1);
      }
      return out;
    }
    case "day": {
      const d = new Date(from);
      d.setSeconds(0, 0);
      d.setHours(cfg.hour ?? 0, cfg.minute ?? 0);
      if (d <= from) d.setDate(d.getDate() + 1);
      for (let i = 0; i < count; i++) {
        out.push(d.toISOString().replace(/\.\d{3}Z$/, "Z"));
        d.setDate(d.getDate() + 1);
      }
      return out;
    }
    case "week": {
      const days = (cfg.daysOfWeek && cfg.daysOfWeek.length > 0 ? cfg.daysOfWeek : [1]).slice().sort();
      const d = new Date(from);
      d.setSeconds(0, 0);
      d.setHours(cfg.hour ?? 0, cfg.minute ?? 0);
      let safety = 0;
      while (out.length < count && safety++ < 366) {
        if (days.includes(d.getDay()) && d > from) {
          out.push(d.toISOString().replace(/\.\d{3}Z$/, "Z"));
        }
        d.setDate(d.getDate() + 1);
      }
      return out;
    }
    case "month": {
      const d = new Date(from);
      d.setSeconds(0, 0);
      d.setHours(cfg.hour ?? 0, cfg.minute ?? 0);
      d.setDate(cfg.dayOfMonth ?? 1);
      if (d <= from) d.setMonth(d.getMonth() + 1);
      for (let i = 0; i < count; i++) {
        out.push(d.toISOString().replace(/\.\d{3}Z$/, "Z"));
        d.setMonth(d.getMonth() + 1);
      }
      return out;
    }
    case "custom": {
      // Without a full cron parser we surface a placeholder rather
      // than guessing. The gateway-side scheduler is authoritative.
      return Array.from({ length: count }, () => "preview unavailable for custom cron");
    }
  }
}

function cronSyntaxWarning(expr: string): string | undefined {
  if (!expr.trim()) return undefined;
  const fields = expr.trim().split(/\s+/);
  if (fields.length !== 5) return "Cron must have exactly 5 fields.";
  return undefined;
}

function flattenErrors(issues: Array<{ path: (string | number)[]; message: string }>): Record<string, string> {
  const out: Record<string, string> = {};
  for (const issue of issues) {
    const key = issue.path.join(".");
    if (!out[key]) out[key] = issue.message;
  }
  return out;
}
