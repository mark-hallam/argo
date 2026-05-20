"use client";

import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { CommandPalette } from "./CommandPalette";

/**
 * Global keyboard listener + open-state controller for the SKYCOT cross-app
 * command palette. Mount once in `app/layout.tsx` so ⌘K (Mac) / Ctrl+K
 * (Windows + Linux) works on every page.
 *
 * The modal itself (`CommandPalette`) is only rendered when `open === true`,
 * so closed-state DOM cost is zero.
 *
 * The context is exposed so callers can also open the palette programmatically
 * (e.g. the `⌘K` hint pill on the /skills hero will be wired to call
 * `openPalette()` in a follow-up brief).
 */

interface CommandPaletteContextValue {
  open: boolean;
  openPalette: () => void;
  closePalette: () => void;
  togglePalette: () => void;
}

const CommandPaletteContext = createContext<CommandPaletteContextValue | undefined>(undefined);

export function useCommandPalette(): CommandPaletteContextValue {
  const ctx = useContext(CommandPaletteContext);
  if (!ctx) {
    // Safe fallback rather than throwing — keeps callers from crashing the page
    // if they render outside the provider (e.g. in a Storybook isolation test).
    return {
      open: false,
      openPalette: () => {},
      closePalette: () => {},
      togglePalette: () => {},
    };
  }
  return ctx;
}

interface CommandPaletteProviderProps {
  children: React.ReactNode;
}

export function CommandPaletteProvider({ children }: CommandPaletteProviderProps) {
  const [open, setOpen] = useState(false);

  const openPalette = useCallback(() => setOpen(true), []);
  const closePalette = useCallback(() => setOpen(false), []);
  const togglePalette = useCallback(() => setOpen((prev) => !prev), []);

  // Global ⌘K / Ctrl+K listener. SSR-safe: guarded by `typeof window`.
  useEffect(() => {
    if (typeof window === "undefined") return;
    const handler = (event: KeyboardEvent) => {
      const isCmdK =
        event.key.toLowerCase() === "k" && (event.metaKey || event.ctrlKey) && !event.altKey;
      if (isCmdK) {
        event.preventDefault();
        setOpen((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // Prevent background scroll while the palette is open.
  useEffect(() => {
    if (typeof document === "undefined") return;
    if (open) {
      const previousOverflow = document.body.style.overflow;
      document.body.style.overflow = "hidden";
      return () => {
        document.body.style.overflow = previousOverflow;
      };
    }
    return;
  }, [open]);

  const value = useMemo<CommandPaletteContextValue>(
    () => ({ open, openPalette, closePalette, togglePalette }),
    [open, openPalette, closePalette, togglePalette],
  );

  return (
    <CommandPaletteContext.Provider value={value}>
      {children}
      {open ? <CommandPalette open={open} onClose={closePalette} /> : null}
    </CommandPaletteContext.Provider>
  );
}
