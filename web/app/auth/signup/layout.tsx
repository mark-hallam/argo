import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Hire your first AI teammate",
  description:
    "Start with one teammate that lives in Slack, learns from your feedback, and asks before they act. Four minutes from job description to first hire.",
};

export default function SignUpLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
