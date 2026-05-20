import type { ReactNode } from "react";

/**
 * Auth layout — intentionally exposes no `metadata` so per-page
 * `metadata` exports (e.g. `/auth/signup`, `/auth/verify`,
 * `/auth/check-email`) win the `<title>` slot. Previously this
 * layout hardcoded `title: "Sign in"` which leaked the signin
 * tab title onto signup / verify / check-email pages.
 */
export default function AuthLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
