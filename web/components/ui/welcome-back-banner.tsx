"use client";

import { useEffect } from "react";
import { toast } from "sonner";
import { WELCOME_BACK_FLAG_KEY } from "@/lib/auth/return-to";

/**
 * Self-mounting bridge that consumes the [WELCOME_BACK_FLAG_KEY]
 * sessionStorage flag set by the verify page (after a session-
 * expired re-auth) and fires a 4-second sonner toast on the
 * destination route.
 *
 * The banner is intentionally a one-shot toast on top of the
 * already-global sonner Toaster (mounted in `(app)/layout.tsx`)
 * rather than a parallel banner element — the brief calls for a
 * "small flash banner" and sonner is the project's existing flash
 * surface, so this avoids inventing a second one.
 */
export function WelcomeBackBanner() {
  useEffect(() => {
    let flag: string | null = null;
    try {
      flag = window.sessionStorage.getItem(WELCOME_BACK_FLAG_KEY);
      if (flag !== null) window.sessionStorage.removeItem(WELCOME_BACK_FLAG_KEY);
    } catch {
      return;
    }
    if (flag !== "1") return;
    toast("Welcome back — picking up where you left off.", { duration: 4000 });
  }, []);

  return null;
}
