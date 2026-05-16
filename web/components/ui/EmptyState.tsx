"use client";

import Link from "next/link";
import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentType, ReactNode, SVGProps } from "react";
import { buttonVariants } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

const wrapperVariants = cva(
  "rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] text-center text-[var(--foreground)]",
  {
    variants: {
      variant: {
        default: "px-6 py-14 sm:py-16 space-y-4",
        compact: "px-6 py-10 space-y-3",
        inline:
          "flex items-center justify-center gap-3 px-4 py-4 text-left space-y-0",
      },
    },
    defaultVariants: { variant: "default" },
  },
);

type IllustrationProps = SVGProps<SVGSVGElement>;

interface EmptyStateAction {
  label: string;
  href?: string;
  onClick?: () => void;
}

export interface EmptyStateProps
  extends Omit<VariantProps<typeof wrapperVariants>, "variant"> {
  headline: string;
  body?: ReactNode;
  illustration?: ComponentType<IllustrationProps>;
  cta?: EmptyStateAction;
  secondary?: EmptyStateAction;
  variant?: "default" | "compact" | "inline";
  ariaLabel?: string;
  className?: string;
  "data-testid"?: string;
}

export function EmptyState({
  headline,
  body,
  illustration: Illustration,
  cta,
  secondary,
  variant = "default",
  ariaLabel,
  className,
  ...rest
}: EmptyStateProps) {
  const testId = rest["data-testid"];
  if (variant === "inline") {
    return (
      <section
        aria-label={ariaLabel ?? headline}
        data-testid={testId}
        className={cn(wrapperVariants({ variant }), className)}
      >
        <h2 className="text-sm font-medium text-[var(--foreground)]">{headline}</h2>
        {body ? (
          <p className="text-sm text-[var(--muted-foreground)]">{body}</p>
        ) : null}
      </section>
    );
  }
  return (
    <section
      aria-label={ariaLabel ?? headline}
      data-testid={testId}
      className={cn(wrapperVariants({ variant }), className)}
    >
      {variant === "default" && Illustration ? (
        <Illustration
          aria-hidden
          className="mx-auto size-16 text-[var(--muted-foreground)]"
        />
      ) : null}
      <div className="space-y-1.5">
        <h2 className="text-base font-semibold tracking-tight text-[var(--foreground)]">
          {headline}
        </h2>
        {body ? (
          <p className="mx-auto max-w-md text-sm text-[var(--muted-foreground)]">
            {body}
          </p>
        ) : null}
      </div>
      {cta || secondary ? (
        <div className="flex flex-wrap items-center justify-center gap-2 pt-1">
          {cta ? <ActionButton action={cta} variant="primary" /> : null}
          {secondary ? <ActionButton action={secondary} variant="ghost" /> : null}
        </div>
      ) : null}
    </section>
  );
}

function ActionButton({
  action,
  variant,
}: {
  action: EmptyStateAction;
  variant: "primary" | "ghost";
}) {
  const className = buttonVariants({ variant, size: "sm" });
  if (action.href) {
    return (
      <Link href={action.href} className={className}>
        {action.label}
      </Link>
    );
  }
  return (
    <button type="button" onClick={action.onClick} className={className}>
      {action.label}
    </button>
  );
}
