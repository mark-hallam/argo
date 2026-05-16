import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface FeatureSectionProps extends HTMLAttributes<HTMLElement> {
  icon?: ReactNode;
  eyebrow?: ReactNode;
  headline: ReactNode;
  body?: ReactNode;
  media?: ReactNode;
  reverse?: boolean;
}

export const FeatureSection = forwardRef<HTMLElement, FeatureSectionProps>(
  ({ className, icon, eyebrow, headline, body, media, reverse, ...props }, ref) => (
    <section
      ref={ref}
      data-testid="feature-section"
      data-reverse={reverse || undefined}
      className={cn(
        "w-full px-6 py-12 grid gap-8 md:grid-cols-2 md:items-center",
        className,
      )}
      {...props}
    >
      <div className={cn("flex flex-col gap-3", reverse && "md:order-2")}>
        {icon ? (
          <span
            aria-hidden
            className="inline-flex items-center justify-center size-10 rounded-[var(--radius-md)] bg-[var(--brand-primary)]/10 text-[var(--brand-primary)]"
          >
            {icon}
          </span>
        ) : null}
        {eyebrow ? (
          <p className="text-xs font-medium uppercase tracking-wide text-[var(--brand-primary)]">
            {eyebrow}
          </p>
        ) : null}
        <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight text-[var(--foreground)] leading-tight">
          {headline}
        </h2>
        {body ? (
          <p className="text-base text-[var(--muted-foreground)] leading-relaxed">{body}</p>
        ) : null}
      </div>
      {media ? (
        <div className={cn(reverse && "md:order-1")}>{media}</div>
      ) : null}
    </section>
  ),
);
FeatureSection.displayName = "FeatureSection";
