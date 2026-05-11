import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef, type HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

const pillVariants = cva(
  "inline-flex items-center gap-1.5 font-medium border whitespace-nowrap",
  {
    variants: {
      tone: {
        success:
          "border-[var(--success)]/30 bg-[var(--success)]/10 text-[var(--success)]",
        warning:
          "border-[var(--warning)]/40 bg-[var(--warning)]/10 text-[var(--warning)]",
        error:
          "border-[var(--destructive)]/30 bg-[var(--destructive)]/10 text-[var(--destructive)]",
        info:
          "border-[var(--info)]/30 bg-[var(--info)]/10 text-[var(--info)]",
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

export const StatusPill = forwardRef<HTMLSpanElement, StatusPillProps>(
  ({ className, tone, size, dot = true, children, ...props }, ref) => (
    <span ref={ref} className={cn(pillVariants({ tone, size }), className)} {...props}>
      {dot ? (
        <span
          aria-hidden
          className="inline-block size-1.5 rounded-full bg-current"
        />
      ) : null}
      {children}
    </span>
  ),
);
StatusPill.displayName = "StatusPill";
