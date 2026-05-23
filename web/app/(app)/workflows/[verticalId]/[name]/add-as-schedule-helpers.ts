export interface CadenceOption {
  id: string;
  label: string;
  detail: string;
  /** 5-field cron (minute hour dom month dow). */
  cron: string;
}

export const CADENCE_PRESETS: readonly CadenceOption[] = [
  {
    id: "every-15-min",
    label: "Every 15 minutes",
    detail: "fires four times an hour, every hour",
    cron: "*/15 * * * *",
  },
  {
    id: "hourly",
    label: "Hourly on the hour",
    detail: "fires at :00 of every hour",
    cron: "0 * * * *",
  },
  {
    id: "daily-9am",
    label: "Daily at 9:00 AM",
    detail: "every day of the week",
    cron: "0 9 * * *",
  },
  {
    id: "weekdays-9am",
    label: "Weekdays at 9:00 AM",
    detail: "Monday through Friday",
    cron: "0 9 * * 1-5",
  },
  {
    id: "weekly-monday-9am",
    label: "Weekly Monday at 9:00 AM",
    detail: "start of every week",
    cron: "0 9 * * 1",
  },
] as const;

export const CUSTOM_CADENCE_ID = "custom" as const;

export const DEFAULT_CADENCE_ID = "daily-9am";

export type DrawerStep = 1 | 2 | 3;

export function resolveCadence(cadenceId: string, customCron: string): string | null {
  if (cadenceId === CUSTOM_CADENCE_ID) {
    const trimmed = customCron.trim();
    const parts = trimmed.split(/\s+/);
    if (parts.length !== 5) return null;
    return trimmed;
  }
  return CADENCE_PRESETS.find((c) => c.id === cadenceId)?.cron ?? null;
}

export function describeCadence(cadenceId: string, customCron: string): string {
  if (cadenceId === CUSTOM_CADENCE_ID) {
    const trimmed = customCron.trim();
    return trimmed ? `Custom cron · ${trimmed}` : "Custom cron";
  }
  return (
    CADENCE_PRESETS.find((c) => c.id === cadenceId)?.label ?? "Daily at 9:00 AM"
  );
}

export function formatNextFire(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}
