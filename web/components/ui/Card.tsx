import { forwardRef, type HTMLAttributes } from "react";
import { cn } from "@/lib/utils";

const baseCard =
  "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] text-[var(--card-foreground)] shadow-sm";

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  /** Hoverable cards get an interactive cursor + lift on hover. */
  hoverable?: boolean;
  /** Selected = current-item state; renders ring without elevation change. */
  selected?: boolean;
}

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ className, hoverable, selected, ...props }, ref) => (
    <div
      ref={ref}
      data-selected={selected || undefined}
      className={cn(
        baseCard,
        hoverable &&
          "cursor-pointer transition-shadow duration-[var(--duration-fast)] hover:shadow-md",
        selected && "ring-2 ring-[var(--ring)] ring-offset-2 ring-offset-[var(--background)]",
        className,
      )}
      {...props}
    />
  ),
);
Card.displayName = "Card";

export const CardHeader = forwardRef<HTMLDivElement, HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("flex flex-col gap-1.5 px-5 pt-5", className)} {...props} />
  ),
);
CardHeader.displayName = "CardHeader";

export const CardTitle = forwardRef<HTMLHeadingElement, HTMLAttributes<HTMLHeadingElement>>(
  ({ className, ...props }, ref) => (
    <h3 ref={ref} className={cn("text-base font-semibold leading-tight tracking-tight", className)} {...props} />
  ),
);
CardTitle.displayName = "CardTitle";

export const CardDescription = forwardRef<HTMLParagraphElement, HTMLAttributes<HTMLParagraphElement>>(
  ({ className, ...props }, ref) => (
    <p ref={ref} className={cn("text-sm text-[var(--muted-foreground)]", className)} {...props} />
  ),
);
CardDescription.displayName = "CardDescription";

export const CardBody = forwardRef<HTMLDivElement, HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("px-5 py-4", className)} {...props} />
  ),
);
CardBody.displayName = "CardBody";

export const CardFooter = forwardRef<HTMLDivElement, HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn("flex items-center gap-2 px-5 pb-5 pt-2", className)}
      {...props}
    />
  ),
);
CardFooter.displayName = "CardFooter";
