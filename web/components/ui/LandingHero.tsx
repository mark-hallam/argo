import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface LandingHeroProps extends HTMLAttributes<HTMLElement> {
  eyebrow?: ReactNode;
  headline: ReactNode;
  subcopy?: ReactNode;
  primaryCta: ReactNode;
  secondaryCta?: ReactNode;
  align?: "center" | "left";
}

export const LandingHero = forwardRef<HTMLElement, LandingHeroProps>(
  ({ className, eyebrow, headline, subcopy, primaryCta, secondaryCta, align = "center", ...props }, ref) => (
    <section
      ref={ref}
      data-testid="landing-hero"
      className={cn(
        "w-full px-6 py-16 sm:py-24",
        align === "center" ? "text-center" : "text-left",
        className,
      )}
      {...props}
    >
      <div className={cn("mx-auto max-w-3xl", align === "center" ? "items-center" : "items-start", "flex flex-col gap-5")}>
        {eyebrow ? (
          <p className="text-sm font-medium tracking-wide text-[var(--brand-primary)] uppercase">
            {eyebrow}
          </p>
        ) : null}
        <h1 className="text-4xl sm:text-5xl md:text-6xl font-bold tracking-tight text-[var(--foreground)] leading-tight">
          {headline}
        </h1>
        {subcopy ? (
          <p className="max-w-2xl text-base sm:text-lg text-[var(--muted-foreground)] leading-relaxed">
            {subcopy}
          </p>
        ) : null}
        <div
          className={cn(
            "mt-2 flex flex-col sm:flex-row gap-3",
            align === "center" ? "justify-center" : "justify-start",
          )}
        >
          {primaryCta}
          {secondaryCta}
        </div>
      </div>
    </section>
  ),
);
LandingHero.displayName = "LandingHero";
