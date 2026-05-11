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

const statusRing = cva("absolute inset-0 rounded-full ring-2", {
  variants: {
    status: {
      idle: "ring-[var(--muted-foreground)]/30",
      working: "ring-[var(--info)]",
      waiting: "ring-[var(--warning)]",
      blocked: "ring-[var(--destructive)]",
      none: "ring-transparent",
    },
  },
  defaultVariants: { status: "none" },
});

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
      <RAvatar.Root ref={ref} className={cn(avatarVariants({ size }), className)} {...props}>
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
        {status ? <span aria-hidden className={statusRing({ status })} /> : null}
      </RAvatar.Root>
    );
  },
);
Avatar.displayName = "Avatar";
