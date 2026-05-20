import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Sign in",
  description:
    "Magic-link sign in to your SKYCOT workspace. No passwords — audit-first AI teammates.",
};

export default function SignInLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
