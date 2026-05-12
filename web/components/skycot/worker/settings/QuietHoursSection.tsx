"use client";

import { useEffect, useMemo, useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import type { QuietHoursConfig } from "@/lib/api/worker-settings";

/**
 * Quiet-hours section — single tz-aware time range. v1 uses the
 * workspace's detected timezone (`Intl.DateTimeFormat()`); enterprise
 * promotes this to per-channel-kind quiet hours via the Track H
 * brief `mission-enterprise-per-channel-quiet-hours`.
 */

const TIMEZONE_OPTIONS = [
  "America/Los_Angeles",
  "America/Denver",
  "America/Chicago",
  "America/New_York",
  "Europe/London",
  "Europe/Berlin",
  "Asia/Singapore",
  "Asia/Tokyo",
  "Australia/Sydney",
  "UTC",
];

export interface QuietHoursSectionProps {
  value: QuietHoursConfig;
  workerName: string;
  onSave: (next: QuietHoursConfig) => void;
}

export function QuietHoursSection({ value, workerName, onSave }: QuietHoursSectionProps) {
  const [start, setStart] = useState(value.start);
  const [end, setEnd] = useState(value.end);
  const [timezone, setTimezone] = useState(value.timezone);

  useEffect(() => {
    setStart(value.start);
    setEnd(value.end);
    setTimezone(value.timezone);
  }, [value.start, value.end, value.timezone]);

  const tzOptions = useMemo(() => {
    const set = new Set(TIMEZONE_OPTIONS);
    set.add(value.timezone);
    return Array.from(set).sort();
  }, [value.timezone]);

  const isDirty =
    start !== value.start || end !== value.end || timezone !== value.timezone;

  return (
    <Card data-testid="card-settings-quiet-hours">
      <CardHeader>
        <CardTitle>Quiet hours</CardTitle>
        <p
          data-testid="quiet-hours-summary"
          className="text-sm text-[var(--muted-foreground)]"
        >
          {workerName} won&apos;t be reachable between {start} and {end} ({timezone}).
          Outside this window she can ping you on your primary channel.
        </p>
      </CardHeader>
      <CardBody className="pt-1">
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <label className="flex flex-col gap-1 text-xs font-medium text-[var(--muted-foreground)]">
            Start
            <input
              type="time"
              value={start}
              onChange={(e) => setStart(e.target.value)}
              data-testid="quiet-hours-start"
              className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2 py-1.5 text-sm text-[var(--foreground)]"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs font-medium text-[var(--muted-foreground)]">
            End
            <input
              type="time"
              value={end}
              onChange={(e) => setEnd(e.target.value)}
              data-testid="quiet-hours-end"
              className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2 py-1.5 text-sm text-[var(--foreground)]"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs font-medium text-[var(--muted-foreground)]">
            Timezone
            <select
              value={timezone}
              onChange={(e) => setTimezone(e.target.value)}
              data-testid="quiet-hours-tz"
              className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2 py-1.5 text-sm text-[var(--foreground)]"
            >
              {tzOptions.map((tz) => (
                <option key={tz} value={tz}>
                  {tz}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="mt-4 flex justify-end gap-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={!isDirty}
            onClick={() => {
              setStart(value.start);
              setEnd(value.end);
              setTimezone(value.timezone);
            }}
            data-testid="quiet-hours-cancel"
          >
            Cancel
          </Button>
          <Button
            variant="primary"
            size="sm"
            disabled={!isDirty}
            onClick={() => onSave({ start, end, timezone })}
            data-testid="quiet-hours-save"
          >
            Save
          </Button>
        </div>
      </CardBody>
    </Card>
  );
}
