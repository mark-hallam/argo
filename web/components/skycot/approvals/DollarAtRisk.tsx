import { cn } from "@/lib/utils";

/**
 * Per-card $ at risk readout. Renders nothing (an em-dash placeholder)
 * when no monetary impact is present — design-lock explicitly forbids
 * rendering "$0" for non-money actions (a misleading signal).
 */

export interface DollarAtRiskProps {
  /** Amount in major currency units, e.g. dollars. Omit / pass null for non-monetary actions. */
  amount?: number | null;
  /** ISO 4217 currency code. Defaults to USD. */
  currency?: string;
  /** Locale for the number format. Defaults to "en-US". */
  locale?: string;
  className?: string;
}

function format(amount: number, currency: string, locale: string) {
  try {
    return new Intl.NumberFormat(locale, {
      style: "currency",
      currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount);
  } catch {
    return `$${amount.toLocaleString(locale)}`;
  }
}

export function DollarAtRisk({
  amount,
  currency = "USD",
  locale = "en-US",
  className,
}: DollarAtRiskProps) {
  if (amount == null || amount === 0) {
    return (
      <span
        data-testid="dollar-at-risk-empty"
        aria-hidden
        className={cn(
          "font-mono text-[12px] text-[var(--muted-foreground)]/60",
          className,
        )}
      >
        —
      </span>
    );
  }

  return (
    <span
      data-testid="dollar-at-risk"
      data-amount={amount}
      className={cn(
        "font-mono text-[13px] font-semibold tabular-nums text-[var(--foreground)]",
        className,
      )}
      title={`${format(amount, currency, locale)} at risk if this is approved`}
    >
      {format(amount, currency, locale)}
    </span>
  );
}
