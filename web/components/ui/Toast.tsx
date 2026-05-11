"use client";

import * as RToast from "@radix-ui/react-toast";
import { cva, type VariantProps } from "class-variance-authority";
import { X } from "lucide-react";
import { forwardRef, type ReactNode } from "react";
import { cn } from "@/lib/utils";

const toastVariants = cva(
  "group pointer-events-auto relative flex w-full items-start gap-3 overflow-hidden rounded-[var(--radius-md)] border p-4 pr-6 shadow-lg " +
    "data-[state=open]:animate-in data-[state=closed]:animate-out " +
    "data-[swipe=move]:translate-x-[var(--radix-toast-swipe-move-x)] data-[swipe=end]:translate-x-[var(--radix-toast-swipe-end-x)]",
  {
    variants: {
      tone: {
        default: "border-[var(--border)] bg-[var(--card)] text-[var(--card-foreground)]",
        success: "border-[var(--success)]/30 bg-[var(--success)]/10 text-[var(--success-foreground)]",
        warning: "border-[var(--warning)]/30 bg-[var(--warning)]/10 text-[var(--warning-foreground)]",
        error: "border-[var(--destructive)]/30 bg-[var(--destructive)]/10 text-[var(--destructive-foreground)]",
      },
    },
    defaultVariants: { tone: "default" },
  },
);

export const ToastProvider = RToast.Provider;

export const ToastViewport = forwardRef<
  React.ElementRef<typeof RToast.Viewport>,
  React.ComponentPropsWithoutRef<typeof RToast.Viewport>
>(({ className, ...props }, ref) => (
  <RToast.Viewport
    ref={ref}
    className={cn(
      "fixed bottom-4 right-4 z-50 flex max-h-screen w-full max-w-sm flex-col gap-2 outline-none",
      className,
    )}
    {...props}
  />
));
ToastViewport.displayName = "ToastViewport";

export interface ToastProps
  extends Omit<React.ComponentPropsWithoutRef<typeof RToast.Root>, "title">,
    VariantProps<typeof toastVariants> {
  title?: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
}

export const Toast = forwardRef<React.ElementRef<typeof RToast.Root>, ToastProps>(
  ({ className, tone, title, description, action, ...props }, ref) => (
    <RToast.Root ref={ref} className={cn(toastVariants({ tone }), className)} {...props}>
      <div className="flex-1 space-y-1">
        {title ? (
          <RToast.Title className="text-sm font-semibold leading-tight">{title}</RToast.Title>
        ) : null}
        {description ? (
          <RToast.Description className="text-xs text-current/80">{description}</RToast.Description>
        ) : null}
      </div>
      {action ? <RToast.Action altText="action">{action}</RToast.Action> : null}
      <RToast.Close
        className="absolute right-2 top-2 rounded-[var(--radius-sm)] p-1 text-current/60 hover:text-current"
        aria-label="Dismiss"
      >
        <X className="size-4" />
      </RToast.Close>
    </RToast.Root>
  ),
);
Toast.displayName = "Toast";
