import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { Avatar } from "@/components/ui/Avatar";
import { cn } from "@/lib/utils";

export interface TestimonialCardProps extends Omit<HTMLAttributes<HTMLElement>, "role"> {
  quote: ReactNode;
  name: string;
  role?: ReactNode;
  company?: ReactNode;
  avatarSrc?: string;
  avatarFallback?: string;
}

export const TestimonialCard = forwardRef<HTMLElement, TestimonialCardProps>(
  ({ className, quote, name, role, company, avatarSrc, avatarFallback, ...props }, ref) => (
    <figure
      ref={ref}
      data-testid="testimonial-card"
      className={cn(
        "flex flex-col gap-4 p-6 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] text-[var(--card-foreground)] shadow-sm",
        className,
      )}
      {...props}
    >
      <blockquote className="text-base leading-relaxed text-[var(--foreground)]">
        <span aria-hidden className="text-[var(--brand-primary)]">&ldquo;</span>
        {quote}
        <span aria-hidden className="text-[var(--brand-primary)]">&rdquo;</span>
      </blockquote>
      <figcaption className="flex items-center gap-3">
        <Avatar size="sm" alt={name} src={avatarSrc} fallback={avatarFallback} />
        <div className="flex flex-col">
          <span className="text-sm font-semibold text-[var(--foreground)]">{name}</span>
          {(role || company) ? (
            <span className="text-xs text-[var(--muted-foreground)]">
              {role}
              {role && company ? <span aria-hidden> &middot; </span> : null}
              {company}
            </span>
          ) : null}
        </div>
      </figcaption>
    </figure>
  ),
);
TestimonialCard.displayName = "TestimonialCard";
