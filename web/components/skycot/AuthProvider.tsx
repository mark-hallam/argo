"use client";

import { SessionProvider } from "next-auth/react";
import type { ReactNode } from "react";

/**
 * In demo-bypass mode there is no `__Host-session` cookie minted, so
 * NextAuth's polling `useSession()` loop hits `/api/auth/session`, gets
 * a config-error response, and surfaces a `ClientFetchError` in the
 * browser console on every navigation. Sustained polling also makes
 * the renderer janky after ~60 navigations.
 *
 * Fix: when either of the demo-bypass env-vars is set, mount the
 * provider with `session={null}` (skips the initial fetch),
 * `refetchInterval={0}` (no polling), and `refetchOnWindowFocus={false}`.
 * Consumers of `useSession()` still get a valid context returning
 * `{ data: null, status: "unauthenticated" }` — required because
 * NextAuth v5's `useSession()` throws when no provider is mounted.
 */
const DEMO_BYPASS =
  process.env.NEXT_PUBLIC_ARGO_DEMO_MODE === "1" ||
  process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1";

export function AuthProvider({ children }: { children: ReactNode }) {
  if (DEMO_BYPASS) {
    return (
      <SessionProvider
        session={null}
        refetchInterval={0}
        refetchOnWindowFocus={false}
      >
        {children}
      </SessionProvider>
    );
  }
  return <SessionProvider>{children}</SessionProvider>;
}
