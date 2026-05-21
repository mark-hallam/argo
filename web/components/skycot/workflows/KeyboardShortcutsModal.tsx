"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { Keyboard, X } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Builder cheatsheet — modal listing the keyboard shortcuts wired by
 * `useBuilderKeyboard`. Triggered by `?` (no modifier) on the builder
 * page; also openable from the floating affordance bottom-right of the
 * canvas. Keys render as `<kbd>` chips so the surface stays brand-clean
 * while feeling like a power-user tool.
 */

interface ShortcutRow {
  label: string;
  keys: string[];
}

const ROWS: ShortcutRow[] = [
  { label: "Undo", keys: ["Mod", "Z"] },
  { label: "Redo", keys: ["Mod", "Shift", "Z"] },
  { label: "Redo (Windows)", keys: ["Ctrl", "Y"] },
  { label: "Fit workflow to view", keys: ["Mod", "0"] },
  { label: "Zoom in", keys: ["Mod", "="] },
  { label: "Zoom out", keys: ["Mod", "-"] },
  { label: "Select all steps", keys: ["Mod", "A"] },
  { label: "Duplicate selected", keys: ["Mod", "D"] },
  { label: "Delete selected", keys: ["Backspace"] },
  { label: "Show this menu", keys: ["?"] },
];

const isMacUA = (): boolean => {
  if (typeof navigator === "undefined") return false;
  return /Mac|iPhone|iPod|iPad/.test(navigator.platform);
};

const renderKey = (key: string, mac: boolean): string => {
  if (key === "Mod") return mac ? "⌘" : "Ctrl";
  if (key === "Shift") return mac ? "⇧" : "Shift";
  return key;
};

export interface KeyboardShortcutsModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function KeyboardShortcutsModal({ open, onOpenChange }: KeyboardShortcutsModalProps) {
  const mac = isMacUA();
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className={cn(
            "fixed inset-0 z-40 bg-black/40 backdrop-blur-sm",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
        />
        <DialogPrimitive.Content
          aria-modal="true"
          data-testid="builder-shortcuts-modal"
          className={cn(
            "fixed inset-0 z-50 bg-[var(--background)] sm:inset-auto sm:left-1/2 sm:top-1/2",
            "sm:w-[92vw] sm:max-w-md sm:-translate-x-1/2 sm:-translate-y-1/2",
            "sm:rounded-[var(--radius-lg)] sm:border sm:border-[var(--border)] sm:shadow-lg",
            "flex flex-col sm:max-h-[85vh] focus:outline-none",
          )}
        >
          <header className="flex items-start justify-between gap-4 border-b border-[var(--border)] px-6 py-5">
            <div className="flex items-center gap-2 min-w-0">
              <span
                aria-hidden
                className="grid size-8 place-items-center rounded-[var(--radius-md)] bg-[var(--muted)] text-[var(--muted-foreground)]"
              >
                <Keyboard size={16} />
              </span>
              <div className="min-w-0">
                <DialogPrimitive.Title
                  className="text-base font-semibold text-[var(--foreground)]"
                  data-testid="builder-shortcuts-title"
                >
                  Keyboard shortcuts
                </DialogPrimitive.Title>
                <DialogPrimitive.Description className="mt-0.5 text-xs text-[var(--muted-foreground)]">
                  Press <kbd className="rounded border border-[var(--border)] bg-[var(--muted)] px-1.5 py-0.5 font-mono text-[10px]">?</kbd> to reopen this menu.
                </DialogPrimitive.Description>
              </div>
            </div>
            <DialogPrimitive.Close
              aria-label="Close"
              data-testid="builder-shortcuts-close"
              className={cn(
                "grid size-8 place-items-center rounded-[var(--radius-md)]",
                "text-[var(--muted-foreground)] hover:bg-[var(--muted)] hover:text-[var(--foreground)]",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
              )}
            >
              <X aria-hidden size={18} />
            </DialogPrimitive.Close>
          </header>

          <ul className="flex flex-col divide-y divide-[var(--border)] px-6 py-2">
            {ROWS.map((row) => (
              <li
                key={row.label}
                className="flex items-center justify-between gap-4 py-2 text-sm"
              >
                <span className="text-[var(--foreground)]">{row.label}</span>
                <span className="flex items-center gap-1">
                  {row.keys.map((k, idx) => (
                    <kbd
                      key={`${row.label}-${idx}`}
                      className="rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--muted)] px-2 py-0.5 font-mono text-xs text-[var(--foreground)]"
                    >
                      {renderKey(k, mac)}
                    </kbd>
                  ))}
                </span>
              </li>
            ))}
          </ul>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
