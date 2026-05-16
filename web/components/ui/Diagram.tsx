import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface DiagramProps extends HTMLAttributes<HTMLElement> {
  caption?: ReactNode;
  ariaLabel: string;
  children: ReactNode;
  maxWidth?: number;
}

export const Diagram = forwardRef<HTMLElement, DiagramProps>(
  ({ className, caption, ariaLabel, children, maxWidth = 720, ...props }, ref) => (
    <figure
      ref={ref}
      data-testid="diagram"
      className={cn(
        "mx-auto w-full flex flex-col gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-4",
        className,
      )}
      style={{ maxWidth }}
      {...props}
    >
      <div
        role="img"
        aria-label={ariaLabel}
        className="w-full overflow-hidden text-[var(--foreground)] [&_svg]:w-full [&_svg]:h-auto"
      >
        {children}
      </div>
      {caption ? (
        <figcaption className="text-xs text-[var(--muted-foreground)] text-center">
          {caption}
        </figcaption>
      ) : null}
    </figure>
  ),
);
Diagram.displayName = "Diagram";
