import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface KeyFactProps extends HTMLAttributes<HTMLDivElement> {
  value: ReactNode;
  label: ReactNode;
  context?: ReactNode;
  align?: "left" | "center";
}

export const KeyFact = forwardRef<HTMLDivElement, KeyFactProps>(
  ({ className, value, label, context, align = "left", ...props }, ref) => (
    <div
      ref={ref}
      data-testid="key-fact"
      className={cn(
        "flex flex-col gap-1",
        align === "center" && "items-center text-center",
        className,
      )}
      {...props}
    >
      <span className="text-4xl sm:text-5xl font-bold tracking-tight text-[var(--foreground)] tabular-nums">
        {value}
      </span>
      <span className="text-sm font-medium text-[var(--foreground)]">{label}</span>
      {context ? (
        <span className="text-xs text-[var(--muted-foreground)] leading-relaxed">
          {context}
        </span>
      ) : null}
    </div>
  ),
);
KeyFact.displayName = "KeyFact";
