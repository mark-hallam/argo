"use client";

import { useEffect, useId, useState } from "react";
import { Clock, X, Zap, ChevronDown } from "lucide-react";
import { Button } from "@/components/ui/Button";
import type { WorkerDuty } from "@/lib/hire/draft-types";
import { CRON_PRESETS, parseCron, type CronParseResult } from "@/lib/cron/parse";
import { cn } from "@/lib/utils";

/**
 * One editable duty row inside the structured Adjust drawer.
 * Cron strings are validated via `parseCron`; invalid expressions
 * render a red border + inline error + notify the parent through
 * `onValidationChange`.
 */
export interface DutyEditorRowProps {
  duty: WorkerDuty;
  index: number;
  onChange: (index: number, next: WorkerDuty) => void;
  onRemove: (index: number) => void;
  onValidationChange: (index: number, valid: boolean) => void;
}

export function DutyEditorRow({
  duty, index, onChange, onRemove, onValidationChange,
}: DutyEditorRowProps) {
  const labelId = useId();
  const scheduleId = useId();
  const Icon = duty.schedule.kind === "cron" ? Clock : Zap;
  const [presetsOpen, setPresetsOpen] = useState(false);

  const cronResult: CronParseResult =
    duty.schedule.kind === "cron" ? parseCron(duty.schedule.spec) : { ok: true };
  const cronInvalid = duty.schedule.kind === "cron" && !cronResult.ok;
  const cronReason = !cronResult.ok ? cronResult.reason : undefined;

  useEffect(() => {
    onValidationChange(index, !cronInvalid);
  }, [index, cronInvalid, onValidationChange]);

  const updateLabel = (next: string) =>
    onChange(index, { ...duty, label: next });
  const updateSpec = (next: string) =>
    onChange(index, { ...duty, schedule: { ...duty.schedule, spec: next } });
  const updateKind = (kind: "cron" | "event") => {
    if (kind === duty.schedule.kind) return;
    onChange(index, {
      ...duty,
      schedule: { kind, spec: kind === "cron" ? "0 9 * * *" : "message.inbound" },
    });
  };
  const toggleEnabled = (next: boolean) =>
    onChange(index, { ...duty, defaultEnabled: next });

  const kindBtn = (kind: "cron" | "event", icon: typeof Clock, label: string) => (
    <button
      type="button"
      data-testid={`hire-step4-duty-editor-${index}-kind-${kind}`}
      onClick={() => updateKind(kind)}
      aria-pressed={duty.schedule.kind === kind}
      className={cn(
        kind === "cron" ? "rounded-l-[var(--radius-sm)]" : "rounded-r-[var(--radius-sm)]",
        "px-2 py-1",
        duty.schedule.kind === kind
          ? "bg-[var(--primary)] text-[var(--primary-foreground)]"
          : "bg-transparent text-[var(--muted-foreground)] hover:bg-[var(--accent)]",
      )}
    >
      {icon === Clock ? <Clock className="size-3 inline" aria-hidden />
        : <Zap className="size-3 inline" aria-hidden />} {label}
    </button>
  );

  return (
    <div
      data-testid={`hire-step4-duty-editor-${index}`}
      className={cn(
        "space-y-2 rounded-[var(--radius-md)] border border-[var(--border)] p-3",
        duty.defaultEnabled ? "bg-[var(--card)]" : "bg-[var(--muted)]/30",
      )}
    >
      <div className="flex flex-wrap items-center gap-2">
        <input id={labelId} type="checkbox"
          data-testid={`hire-step4-duty-editor-${index}-toggle`}
          checked={duty.defaultEnabled}
          onChange={(e) => toggleEnabled(e.target.checked)}
          aria-label={`Enable duty: ${duty.label || "untitled"}`}
          className="size-4 shrink-0 accent-[var(--primary)]" />
        <input type="text"
          data-testid={`hire-step4-duty-editor-${index}-label`}
          value={duty.label}
          onChange={(e) => updateLabel(e.target.value)}
          placeholder="What should this duty do?"
          aria-label={`Edit label for duty ${index + 1}`}
          className={cn(
            "min-w-[12rem] flex-1 rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--background)] px-2 py-1 text-sm",
            "outline-none focus:ring-2 focus:ring-[var(--ring)]",
          )} />
        <Button type="button" variant="ghost" size="sm"
          data-testid={`hire-step4-duty-editor-${index}-remove`}
          onClick={() => onRemove(index)}
          aria-label={`Remove duty ${index + 1}`}>
          <X className="size-3.5" aria-hidden />
        </Button>
      </div>
      <div className="flex flex-wrap items-center gap-2 pl-7">
        <div role="group" aria-label="Schedule kind"
          className="inline-flex rounded-[var(--radius-sm)] border border-[var(--border)] text-xs">
          {kindBtn("cron", Clock, "Cron")}
          {kindBtn("event", Zap, "Event")}
        </div>
        <input id={scheduleId} type="text"
          data-testid={`hire-step4-duty-editor-${index}-spec`}
          value={duty.schedule.spec}
          onChange={(e) => updateSpec(e.target.value)}
          aria-label={`Schedule expression for duty ${index + 1}`}
          aria-invalid={cronInvalid || undefined}
          placeholder={duty.schedule.kind === "cron" ? "0 9 * * *" : "message.inbound"}
          className={cn(
            "min-w-[10rem] flex-1 rounded-[var(--radius-sm)] border bg-[var(--background)] px-2 py-1 font-mono text-xs",
            "outline-none focus:ring-2 focus:ring-[var(--ring)]",
            cronInvalid ? "border-[var(--destructive)]" : "border-[var(--border)]",
          )} />
        {duty.schedule.kind === "cron" ? (
          <div className="relative">
            <Button type="button" variant="ghost" size="sm"
              data-testid={`hire-step4-duty-editor-${index}-presets`}
              onClick={() => setPresetsOpen((o) => !o)}
              aria-haspopup="menu" aria-expanded={presetsOpen}>
              Presets<ChevronDown className="size-3" aria-hidden />
            </Button>
            {presetsOpen ? (
              <ul role="menu"
                data-testid={`hire-step4-duty-editor-${index}-presets-menu`}
                className="absolute right-0 z-10 mt-1 w-40 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] py-1 text-xs shadow-md">
                {CRON_PRESETS.map((preset) => (
                  <li key={preset.id}>
                    <button type="button" role="menuitem"
                      data-testid={`hire-step4-duty-editor-${index}-preset-${preset.id}`}
                      onClick={() => { updateSpec(preset.spec); setPresetsOpen(false); }}
                      className="block w-full px-2 py-1 text-left hover:bg-[var(--accent)]">
                      {preset.label}
                    </button>
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
        ) : null}
      </div>
      {cronInvalid ? (
        <p data-testid={`hire-step4-duty-editor-${index}-error`} role="alert"
          className="pl-7 text-xs text-[var(--destructive)]">
          {cronReason}
        </p>
      ) : null}
      <p className="pl-7 text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">
        <Icon className="inline size-3" aria-hidden />{" "}
        {duty.schedule.kind === "cron" ? "Cron schedule" : "Event trigger"}
      </p>
    </div>
  );
}
