import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef, type HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

const pillVariants = cva(
  "inline-flex items-center gap-1 font-medium border whitespace-nowrap",
  {
    variants: {
      // A11Y-1: label TEXT uses the darker `*-text` token (WCAG AA 4.5:1).
      // The tint background + border keep the saturated status colour, and
      // the dot is forced back to the saturated colour below so the badge
      // still reads as its status hue.
      tone: {
        success:
          "border-[var(--success)]/30 bg-[var(--success)]/10 text-[var(--success-text)]",
        warning:
          "border-[var(--warning)]/40 bg-[var(--warning)]/10 text-[var(--warning-text)]",
        error:
          "border-[var(--destructive)]/30 bg-[var(--destructive)]/10 text-[var(--error-text)]",
        info:
          "border-[var(--info)]/30 bg-[var(--info)]/10 text-[var(--info-text)]",
        muted:
          "border-[var(--border)] bg-[var(--muted)] text-[var(--muted-foreground)]",
      },
      size: {
        sm: "text-[11px] leading-none px-2 py-0.5 rounded-[var(--radius-sm)]",
        md: "text-xs leading-none px-2.5 py-1 rounded-[var(--radius-sm)]",
      },
    },
    defaultVariants: { tone: "muted", size: "md" },
  },
);

export interface StatusPillProps
  extends HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof pillVariants> {
  /** Render the leading dot indicator. */
  dot?: boolean;
}

/**
 * Dot colour per tone. The label TEXT is now the darker `*-text` token for
 * contrast (A11Y-1), but the dot stays the SATURATED status colour so the
 * badge still reads as its status hue. `muted` keeps `currentColor`.
 */
const dotToneClass: Record<NonNullable<StatusPillProps["tone"]>, string> = {
  success: "bg-[var(--success)]",
  warning: "bg-[var(--warning)]",
  error: "bg-[var(--destructive)]",
  info: "bg-[var(--info)]",
  muted: "bg-current",
};

export const StatusPill = forwardRef<HTMLSpanElement, StatusPillProps>(
  ({ className, tone, size, dot = true, children, ...props }, ref) => (
    <span ref={ref} className={cn(pillVariants({ tone, size }), className)} {...props}>
      {dot ? (
        <span
          aria-hidden
          className={cn(
            "inline-block size-1.5 rounded-full",
            dotToneClass[tone ?? "muted"],
          )}
        />
      ) : null}
      {children}
    </span>
  ),
);
StatusPill.displayName = "StatusPill";
