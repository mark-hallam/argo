"use client";

import { useEffect, useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import {
  CHANNEL_KINDS,
  CHANNEL_LABELS,
  type ChannelKind,
  type ChannelsConfig,
} from "@/lib/api/worker-settings";
import { cn } from "@/lib/utils";

/**
 * Channels section — multi-select toggle list with a single
 * primary radio. Mirrors the hire-flow Step-5 channel picker so a
 * delegator's mental model is consistent across the two surfaces.
 *
 * Local state drives the form; Save fires `onSave` with the
 * snapshot. Cancel reverts to the cached value. If the user picks
 * a primary, that channel is automatically toggled on.
 */

export interface ChannelsSectionProps {
  value: ChannelsConfig;
  workerName: string;
  onSave: (next: ChannelsConfig) => void;
}

export function ChannelsSection({ value, workerName, onSave }: ChannelsSectionProps) {
  const [enabled, setEnabled] = useState<ChannelKind[]>(value.enabled);
  const [primary, setPrimary] = useState<ChannelKind>(value.primary);

  useEffect(() => {
    setEnabled(value.enabled);
    setPrimary(value.primary);
  }, [value.enabled, value.primary]);

  const isDirty =
    !sameSet(enabled, value.enabled) || primary !== value.primary;

  const toggleChannel = (kind: ChannelKind) => {
    setEnabled((prev) => {
      if (prev.includes(kind)) {
        if (kind === primary) return prev;
        return prev.filter((k) => k !== kind);
      }
      return [...prev, kind];
    });
  };

  const onPrimary = (kind: ChannelKind) => {
    setPrimary(kind);
    setEnabled((prev) => (prev.includes(kind) ? prev : [...prev, kind]));
  };

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
                    "flex items-center gap-1.5 text-xs",
                    on ? "text-[var(--muted-foreground)]" : "text-[var(--muted-foreground)]/50",
                  )}
                >
                  <input
                    type="radio"
                    name="channels-primary"
                    checked={isPrimary}
                    onChange={() => onPrimary(kind)}
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
        <div className="mt-4 flex justify-end gap-2">
          <Button
            variant="ghost"
            size="sm"
            disabled={!isDirty}
            onClick={() => {
              setEnabled(value.enabled);
              setPrimary(value.primary);
            }}
            data-testid="channels-cancel"
          >
            Cancel
          </Button>
          <Button
            variant="primary"
            size="sm"
            disabled={!isDirty}
            onClick={() => onSave({ enabled, primary })}
            data-testid="channels-save"
          >
            Save
          </Button>
        </div>
      </CardBody>
    </Card>
  );
}

function sameSet(a: ChannelKind[], b: ChannelKind[]): boolean {
  if (a.length !== b.length) return false;
  const aSet = new Set(a);
  for (const k of b) if (!aSet.has(k)) return false;
  return true;
}
