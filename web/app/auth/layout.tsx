import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Sign in",
  description:
    "Magic-link sign-in to your SKYCOT workspace. No passwords. Audit-first AI teammates.",
};

export default function AuthLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
