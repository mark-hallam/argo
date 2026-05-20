"use client";

import { useEffect } from "react";
import { Controller } from "react-hook-form";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { useZodForm } from "@/lib/forms/use-zod-form";
import { useToast } from "@/lib/ui/use-toast";
import {
  CHANNEL_KINDS,
  CHANNEL_LABELS,
  type ChannelKind,
  type ChannelsConfig,
} from "@/lib/api/worker-settings";
import { channelsFormSchema, type ChannelsFormValues } from "./schemas";
import { cn } from "@/lib/utils";

/**
 * Channels section — multi-select toggle list with a single
 * primary radio. RHF + Zod gates Save behind a non-empty selection
 * and the invariant that the primary channel must also be enabled.
 */

export interface ChannelsSectionProps {
  value: ChannelsConfig;
  workerName: string;
  onSave: (next: ChannelsConfig) => Promise<void>;
  isSaving?: boolean;
}

export function ChannelsSection({
  value,
  workerName,
  onSave,
  isSaving = false,
}: ChannelsSectionProps) {
  const toast = useToast();
  const form = useZodForm(channelsFormSchema, {
    defaultValues: { enabled: value.enabled, primary: value.primary },
  });
  const { control, handleSubmit, reset, formState, setValue, watch } = form;

  useEffect(() => {
    reset({ enabled: value.enabled, primary: value.primary });
  }, [value.enabled, value.primary, reset]);

  const enabled = watch("enabled");
  const primary = watch("primary");

  const toggleChannel = (kind: ChannelKind) => {
    if (kind === primary) return;
    const next = enabled.includes(kind)
      ? enabled.filter((k) => k !== kind)
      : [...enabled, kind];
    setValue("enabled", next, { shouldDirty: true, shouldValidate: true });
  };

  const setPrimary = (kind: ChannelKind) => {
    setValue("primary", kind, { shouldDirty: true, shouldValidate: true });
    if (!enabled.includes(kind)) {
      setValue("enabled", [...enabled, kind], {
        shouldDirty: true,
        shouldValidate: true,
      });
    }
  };

  const submit = handleSubmit(async (values) => {
    try {
      await onSave({ enabled: values.enabled, primary: values.primary });
      toast.success("Channels saved");
      reset(values);
    } catch (err) {
      toast.error("Couldn't save channels", {
        description: err instanceof Error ? err.message : undefined,
      });
    }
  });

  const enabledError = formState.errors.enabled?.message;
  const primaryError = formState.errors.primary?.message;
  const submitDisabled = !formState.isDirty || formState.isSubmitting || isSaving;

  return (
    <Card data-testid="card-settings-channels">
      <CardHeader>
        <CardTitle>Channels</CardTitle>
        <p className="text-sm text-[var(--muted-foreground)]">
          Where {workerName} reaches you. The primary channel is the default for
          new notifications.
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
          <Controller
            control={control}
            name="enabled"
            render={() => (
              <ul role="list" className="divide-y divide-[var(--border)]/60">
                {CHANNEL_KINDS.map((kind) => {
                  const on = enabled.includes(kind);
                  const isPrimary = primary === kind;
                  return (
                    <li
                      key={kind}
                      data-testid={`channel-row-${kind}`}
                      className="flex items-center gap-3 py-3"
                    >
                      <input
                        type="checkbox"
                        checked={on}
                        onChange={() => toggleChannel(kind)}
                        disabled={isPrimary}
                        data-testid={`channel-toggle-${kind}`}
                        aria-label={`Toggle ${CHANNEL_LABELS[kind]}`}
                        className="size-4 accent-[var(--primary)]"
                      />
                      <span className="flex-1 text-sm text-[var(--foreground)]">
                        {CHANNEL_LABELS[kind]}
                      </span>
                      <label
                        className={cn(
                          "flex items-center gap-1 text-xs",
                          on
                            ? "text-[var(--muted-foreground)]"
                            : "text-[var(--muted-foreground)]/50",
                        )}
                      >
                        <input
                          type="radio"
                          name="channels-primary"
                          checked={isPrimary}
                          onChange={() => setPrimary(kind)}
                          data-testid={`channel-primary-${kind}`}
                          aria-label={`Make ${CHANNEL_LABELS[kind]} primary`}
                          className="size-3.5 accent-[var(--primary)]"
                        />
                        Primary
                      </label>
                    </li>
                  );
                })}
              </ul>
            )}
          />
          {enabledError ? (
            <p
              role="alert"
              data-testid="channels-error-enabled"
              className="mt-2 text-xs text-[var(--destructive)]"
            >
              {enabledError}
            </p>
          ) : null}
          {primaryError ? (
            <p
              role="alert"
              data-testid="channels-error-primary"
              className="mt-1 text-xs text-[var(--destructive)]"
            >
              {primaryError}
            </p>
          ) : null}
          <div className="mt-4 flex justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={!formState.isDirty || formState.isSubmitting || isSaving}
              onClick={() => reset({ enabled: value.enabled, primary: value.primary })}
              data-testid="channels-cancel"
            >
              Cancel
            </Button>
            <Button
              type="submit"
              variant="primary"
              size="sm"
              disabled={submitDisabled}
              loading={formState.isSubmitting || isSaving}
              data-testid="channels-save"
            >
              Save
            </Button>
          </div>
        </form>
      </CardBody>
    </Card>
  );
}
