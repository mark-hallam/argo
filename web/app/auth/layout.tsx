import type { ReactNode } from "react";

/**
 * Auth layout — intentionally exposes no `metadata` so per-page
 * `metadata` exports (e.g. `/auth/signup`, `/auth/verify`,
 * `/auth/check-email`) win the `<title>` slot. Previously this
 * layout hardcoded `title: "Sign in"` which leaked the signin
 * tab title onto signup / verify / check-email pages.
 *
 * The light-only theme for `/auth/*` is enforced by next-themes'
 * `forcedTheme="light"` (see `components/skycot/ThemeProvider.tsx`),
 * NOT by an inline MutationObserver here. The previous force-light
 * script ran `new MutationObserver(apply)` where `apply()` rewrote
 * `<html>`'s class — under `prefers-color-scheme: dark` it fought
 * next-themes in an infinite class-toggle ping-pong that pegged the
 * renderer and froze the page (every fresh dark-mode visitor hit it).
 * `forcedTheme` makes next-themes resolve light on these routes
 * itself, so nothing applies `dark` and there is nothing to fight.
 */
export default function AuthLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
