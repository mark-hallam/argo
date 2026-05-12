"use client";

import { useEffect, useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { AlertDialog } from "@/components/ui/AlertDialog";
import {
  AUTONOMY_LEVELS,
  AUTONOMY_LABELS,
  type AutonomyConfig,
  type AutonomyLevel,
} from "@/lib/api/worker-settings";
import { cn } from "@/lib/utils";

/**
 * Autonomy section — three-option radio. Switching from Balanced or
 * Aggressive down to Conservative triggers a confirm dialog that
 * surfaces the trust-ratchet reset (the worker re-asks for any
 * action she had been doing autonomously).
 */

const RATCHET_RESET_LEVEL: AutonomyLevel = "conservative";

export interface AutonomySectionProps {
  value: AutonomyConfig;
  workerName: string;
  onSave: (next: AutonomyConfig) => void;
}

export function AutonomySection({ value, workerName, onSave }: AutonomySectionProps) {
  const [level, setLevel] = useState<AutonomyLevel>(value.level);
  const [pendingConfirm, setPendingConfirm] = useState<AutonomyLevel | null>(null);

  useEffect(() => {
    setLevel(value.level);
  }, [value.level]);

  const isDirty = level !== value.level;

  const handleSave = () => {
    if (!isDirty) return;
    if (level === RATCHET_RESET_LEVEL && value.level !== RATCHET_RESET_LEVEL) {
      setPendingConfirm(level);
      return;
    }
    onSave({ level });
  };

  return (
    <Card data-testid="card-settings-autonomy">
      <CardHeader>
        <CardTitle>Autonomy level</CardTitle>
        <p className="text-sm text-[var(--muted-foreground)]">
          How often {workerName} pauses to ask for approval before acting.
        </p>
      </CardHeader>
      <CardBody className="pt-1">
        <ul role="radiogroup" aria-label="Autonomy level" className="space-y-2">
          {AUTONOMY_LEVELS.map((opt) => (
            <li key={opt}>
              <label
                data-testid={`autonomy-option-${opt}`}
                title={
                  opt === RATCHET_RESET_LEVEL
                    ? `Conservative resets the trust ratchet — ${workerName} asks again for any action she's been doing autonomously.`
                    : undefined
                }
                className={cn(
                  "flex items-center gap-3 rounded-[var(--radius-md)] border px-3 py-2 cursor-pointer",
                  level === opt
                    ? "border-[var(--primary)]/60 bg-[var(--accent)]/20"
                    : "border-[var(--border)]",
                )}
              >
                <input
                  type="radio"
                  name="autonomy-level"
                  checked={level === opt}
                  onChange={() => setLevel(opt)}
                  data-testid={`autonomy-radio-${opt}`}
                  className="size-3.5 accent-[var(--primary)]"
                />
                <span className="text-sm text-[var(--foreground)]">{AUTONOMY_LABELS[opt]}</span>
              </label>
            </li>
          ))}
        </ul>
        <div className="mt-4 flex justify-end gap-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={!isDirty}
            onClick={() => setLevel(value.level)}
            data-testid="autonomy-cancel"
          >
            Cancel
          </Button>
          <Button
            variant="primary"
            size="sm"
            disabled={!isDirty}
            onClick={handleSave}
            data-testid="autonomy-save"
          >
            Save
          </Button>
        </div>
      </CardBody>
      <AlertDialog
        open={pendingConfirm !== null}
        onOpenChange={(open) => {
          if (!open) setPendingConfirm(null);
        }}
        title="Reset trust ratchet?"
        description={`Switching to Conservative resets the trust ratchet — ${workerName} asks again for any action she's been doing autonomously.`}
        confirmLabel="Reset & save"
        cancelLabel="Cancel"
        confirmVariant="destructive"
        onConfirm={() => {
          if (pendingConfirm) {
            onSave({ level: pendingConfirm });
          }
          setPendingConfirm(null);
        }}
        onCancel={() => setPendingConfirm(null)}
      />
    </Card>
  );
}
