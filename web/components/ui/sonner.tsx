"use client";

import { useTheme } from "next-themes";
import {
  AlertTriangle,
  CheckCircle,
  Info,
  Loader2,
  XCircle,
} from "lucide-react";
import { Toaster as SonnerToaster, type ToasterProps } from "sonner";

/**
 * Shadcn-style Sonner wrapper. Wires Sonner's `theme` prop to
 * next-themes so toasts switch with the rest of the app, pins
 * iconography to the project's Lucide set, and maps Sonner's rich-
 * color slots to brand CSS vars (`--success`, `--destructive`,
 * `--warning`, `--brand-primary`). Mount once near the root.
 */
export function Toaster(props: ToasterProps) {
  const { resolvedTheme } = useTheme();
  const theme = resolvedTheme === "dark" ? "dark" : "light";
  const style = {
    "--normal-bg": "var(--card)",
    "--normal-text": "var(--card-foreground)",
    "--normal-border": "var(--border)",
    "--success-bg": "var(--success)",
    "--success-text": "var(--success-foreground)",
    "--success-border": "var(--success)",
    "--error-bg": "var(--destructive)",
    "--error-text": "var(--destructive-foreground)",
    "--error-border": "var(--destructive)",
    "--warning-bg": "var(--warning)",
    "--warning-text": "var(--warning-foreground)",
    "--warning-border": "var(--warning)",
    "--info-bg": "var(--brand-primary)",
    "--info-text": "var(--brand-primary-foreground)",
    "--info-border": "var(--brand-primary)",
  } as React.CSSProperties;
  return (
    <SonnerToaster
      theme={theme}
      richColors
      closeButton
      visibleToasts={3}
      icons={{
        success: <CheckCircle className="size-4" aria-hidden />,
        error: <XCircle className="size-4" aria-hidden />,
        info: <Info className="size-4" aria-hidden />,
        warning: <AlertTriangle className="size-4" aria-hidden />,
        loading: <Loader2 className="size-4 animate-spin" aria-hidden />,
      }}
      toastOptions={{
        classNames: {
          toast:
            "group rounded-[var(--radius-md)] border shadow-lg text-sm " +
            "bg-[var(--card)] text-[var(--card-foreground)] border-[var(--border)]",
          title: "text-sm font-semibold leading-tight",
          description: "text-xs text-current/80 mt-0.5",
        },
      }}
      style={style}
      {...props}
    />
  );
}
