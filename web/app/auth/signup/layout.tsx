import type { Metadata } from "next";
import type { ReactNode } from "react";
import { SKYCOT_TAGLINE } from "@/lib/marketing-copy";

export const metadata: Metadata = {
  title: SKYCOT_TAGLINE,
  description:
    "Start with one teammate that lives in Slack, learns from your feedback, and asks before they act. Four minutes from job description to first hire.",
};

export default function SignUpLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
