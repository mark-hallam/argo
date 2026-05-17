import { Check } from "lucide-react";
import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface PricingCardProps extends HTMLAttributes<HTMLDivElement> {
  planName: ReactNode;
  price: ReactNode;
  cadence?: ReactNode;
  description?: ReactNode;
  features: ReactNode[];
  cta: ReactNode;
  featured?: boolean;
}

export const PricingCard = forwardRef<HTMLDivElement, PricingCardProps>(
  ({ className, planName, price, cadence, description, features, cta, featured, ...props }, ref) => (
    <div
      ref={ref}
      data-testid="pricing-card"
      data-featured={featured || undefined}
      className={cn(
        "flex flex-col gap-5 p-6 rounded-[var(--radius-lg)] border bg-[var(--card)] text-[var(--card-foreground)]",
        featured
          ? "border-[var(--brand-primary)] ring-2 ring-[var(--brand-primary)]/30 shadow-md"
          : "border-[var(--border)] shadow-sm",
        className,
      )}
      {...props}
    >
      <div className="flex flex-col gap-1">
        <h3 className="text-sm font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
          {planName}
        </h3>
        <div className="flex items-baseline gap-1">
          <span className="text-4xl font-bold tracking-tight text-[var(--foreground)]">{price}</span>
          {cadence ? (
            <span className="text-sm text-[var(--muted-foreground)]">{cadence}</span>
          ) : null}
        </div>
        {description ? (
          <p className="text-sm text-[var(--muted-foreground)]">{description}</p>
        ) : null}
      </div>
      <ul className="flex flex-col gap-2 flex-1">
        {features.map((feat, i) => (
          <li key={i} className="flex items-start gap-2 text-sm text-[var(--foreground)]">
            <Check
              aria-hidden
              className="size-4 mt-0.5 shrink-0 text-[var(--brand-primary)]"
            />
            <span>{feat}</span>
          </li>
        ))}
      </ul>
      <div>{cta}</div>
    </div>
  ),
);
PricingCard.displayName = "PricingCard";
