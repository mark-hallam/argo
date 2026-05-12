import { ArrowDownRight, ArrowUpRight, Minus } from "lucide-react";
import { cn } from "@/lib/utils";
import { toneTextClass, type ChartTone } from "./utils";

/**
 * Compact delta indicator: arrow + value + unit. The caller passes
 * the explicit `direction` (so an external trend calculator stays
 * authoritative) and `directionMeansGood` to map direction onto
 * semantic tone:
 *   - up + directionMeansGood=true   → success
 *   - up + directionMeansGood=false  → destructive
 *   - down + directionMeansGood=true → destructive
 *   - down + directionMeansGood=false → success
 *   - flat                            → muted (always)
 *
 * `directionMeansGood` defaults to `true`.
 */

export type DeltaDirection = "up" | "down" | "flat";

export interface DeltaChipProps {
  value: number;
  unit?: string;
  direction: DeltaDirection;
  directionMeansGood?: boolean;
  ariaLabel?: string;
  className?: string;
}

function toneFor(
  direction: DeltaDirection,
  directionMeansGood: boolean,
): ChartTone {
  if (direction === "flat") return "muted";
  const good =
    (direction === "up" && directionMeansGood) ||
    (direction === "down" && !directionMeansGood);
  return good ? "success" : "destructive";
}

function iconFor(direction: DeltaDirection) {
  switch (direction) {
    case "up":
      return ArrowUpRight;
    case "down":
      return ArrowDownRight;
    case "flat":
      return Minus;
  }
}

export function DeltaChip({
  value,
  unit,
  direction,
  directionMeansGood = true,
  ariaLabel,
  className,
}: DeltaChipProps) {
  const tone = toneFor(direction, directionMeansGood);
  const Icon = iconFor(direction);
  const sign = value > 0 ? "+" : "";
  const display = `${sign}${value}${unit ?? ""}`;
  const label =
    ariaLabel ?? `${display}, trending ${direction}`;
  return (
    <span
      role="img"
      aria-label={label}
      data-testid="delta-chip"
      data-direction={direction}
      data-tone={tone}
      className={cn(
        "inline-flex items-center gap-1 text-xs font-medium tabular-nums",
        toneTextClass(tone),
        className,
      )}
    >
      <Icon className="size-3.5" aria-hidden />
      <span>{display}</span>
    </span>
  );
}
