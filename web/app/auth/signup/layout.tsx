import type { Metadata } from "next";
import type { ReactNode } from "react";
import { SKYCOT_TAGLINE } from "@/lib/marketing-copy";

export const metadata: Metadata = {
  title: SKYCOT_TAGLINE,
  description:
    "Start with one teammate that lives in Slack, learns from your feedback, and asks before they act. A guided interview so your teammate starts right the first time.",
};

export default function SignUpLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
