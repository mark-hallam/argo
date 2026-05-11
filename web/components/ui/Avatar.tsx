"use client";

import * as RAvatar from "@radix-ui/react-avatar";
import { cva, type VariantProps } from "class-variance-authority";
import { forwardRef } from "react";
import { cn } from "@/lib/utils";

const avatarVariants = cva(
  "relative inline-flex shrink-0 overflow-hidden rounded-full bg-[var(--muted)] text-[var(--muted-foreground)]",
  {
    variants: {
      size: {
        xs: "size-6 text-[10px]",
        sm: "size-8 text-xs",
        md: "size-10 text-sm",
        lg: "size-14 text-base",
      },
    },
    defaultVariants: { size: "md" },
  },
);

const statusRing = cva(
  "absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full ring-2 ring-[var(--card)]",
  {
    variants: {
      status: {
        idle: "bg-[var(--muted-foreground)]/40",
        working: "bg-[var(--info)]",
        waiting: "bg-[var(--warning)]",
        blocked: "bg-[var(--destructive)]",
        none: "hidden",
      },
    },
    defaultVariants: { status: "none" },
  },
);

export type WorkerStatus = "idle" | "working" | "waiting" | "blocked";

export interface AvatarProps
  extends Omit<React.ComponentPropsWithoutRef<typeof RAvatar.Root>, "children">,
    VariantProps<typeof avatarVariants> {
  src?: string;
  alt: string;
  fallback?: string;
  status?: WorkerStatus;
}

export const Avatar = forwardRef<HTMLSpanElement, AvatarProps>(
  ({ className, size, src, alt, fallback, status, ...props }, ref) => {
    const initials =
      fallback ??
      alt
        .split(/\s+/)
        .map((p) => p[0])
        .filter(Boolean)
        .slice(0, 2)
        .join("")
        .toUpperCase();

    return (
      <span className={cn("relative inline-block", className)}>
        <RAvatar.Root className={cn(avatarVariants({ size }))} ref={ref} {...props}>
          {src ? (
            <RAvatar.Image
              src={src}
              alt={alt}
              className="aspect-square h-full w-full object-cover"
            />
          ) : null}
          <RAvatar.Fallback
            delayMs={src ? 200 : 0}
            className="flex h-full w-full items-center justify-center font-medium"
          >
            {initials}
          </RAvatar.Fallback>
        </RAvatar.Root>
        {status ? <span aria-hidden className={statusRing({ status })} /> : null}
      </span>
    );
  },
);
Avatar.displayName = "Avatar";
