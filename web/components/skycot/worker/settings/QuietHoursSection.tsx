"use client";

import { useEffect, useMemo } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { useZodForm } from "@/lib/forms/use-zod-form";
import { useToast } from "@/lib/ui/use-toast";
import type { QuietHoursConfig } from "@/lib/api/worker-settings";
import { quietHoursFormSchema } from "./schemas";

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
  onSave: (next: QuietHoursConfig) => Promise<void>;
  isSaving?: boolean;
}

export function QuietHoursSection({
  value,
  workerName,
  onSave,
  isSaving = false,
}: QuietHoursSectionProps) {
  const toast = useToast();
  const form = useZodForm(quietHoursFormSchema, {
    defaultValues: { start: value.start, end: value.end, timezone: value.timezone },
  });
  const { register, handleSubmit, reset, formState, watch } = form;

  useEffect(() => {
    reset({ start: value.start, end: value.end, timezone: value.timezone });
  }, [value.start, value.end, value.timezone, reset]);

  const start = watch("start");
  const end = watch("end");
  const timezone = watch("timezone");

  const tzOptions = useMemo(() => {
    const set = new Set(TIMEZONE_OPTIONS);
    set.add(value.timezone);
    return Array.from(set).sort();
  }, [value.timezone]);

  const submit = handleSubmit(async (values) => {
    try {
      await onSave(values);
      toast.success("Quiet hours saved");
      reset(values);
    } catch (err) {
      toast.error("Couldn't save quiet hours", {
        description: err instanceof Error ? err.message : undefined,
      });
    }
  });

  const submitDisabled = !formState.isDirty || formState.isSubmitting || isSaving;

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
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
          noValidate
        >
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <label className="flex flex-col gap-1 text-xs font-medium text-[var(--muted-foreground)]">
              Start
              <input
                type="time"
                data-testid="quiet-hours-start"
                aria-invalid={Boolean(formState.errors.start)}
                className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2 py-2 text-sm text-[var(--foreground)]"
                {...register("start")}
              />
              {formState.errors.start ? (
                <span
                  role="alert"
                  data-testid="quiet-hours-error-start"
                  className="text-[11px] text-[var(--destructive)]"
                >
                  {formState.errors.start.message}
                </span>
              ) : null}
            </label>
            <label className="flex flex-col gap-1 text-xs font-medium text-[var(--muted-foreground)]">
              End
              <input
                type="time"
                data-testid="quiet-hours-end"
                aria-invalid={Boolean(formState.errors.end)}
                className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2 py-2 text-sm text-[var(--foreground)]"
                {...register("end")}
              />
              {formState.errors.end ? (
                <span
                  role="alert"
                  data-testid="quiet-hours-error-end"
                  className="text-[11px] text-[var(--destructive)]"
                >
                  {formState.errors.end.message}
                </span>
              ) : null}
            </label>
            <label className="flex flex-col gap-1 text-xs font-medium text-[var(--muted-foreground)]">
              Timezone
              <select
                data-testid="quiet-hours-tz"
                aria-invalid={Boolean(formState.errors.timezone)}
                className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-2 py-2 text-sm text-[var(--foreground)]"
                {...register("timezone")}
              >
                {tzOptions.map((tz) => (
                  <option key={tz} value={tz}>
                    {tz}
                  </option>
                ))}
              </select>
              {formState.errors.timezone ? (
                <span
                  role="alert"
                  data-testid="quiet-hours-error-timezone"
                  className="text-[11px] text-[var(--destructive)]"
                >
                  {formState.errors.timezone.message}
                </span>
              ) : null}
            </label>
          </div>
          <div className="mt-4 flex justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={!formState.isDirty || formState.isSubmitting || isSaving}
              onClick={() =>
                reset({ start: value.start, end: value.end, timezone: value.timezone })
              }
              data-testid="quiet-hours-cancel"
            >
              Cancel
            </Button>
            <Button
              type="submit"
              variant="primary"
              size="sm"
              disabled={submitDisabled}
              loading={formState.isSubmitting || isSaving}
              data-testid="quiet-hours-save"
            >
              Save
            </Button>
          </div>
        </form>
      </CardBody>
    </Card>
  );
}
