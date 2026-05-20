import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Skill marketplace",
  description:
    "Every skill your AI team will ever need. Drop in pre-built skills and your teammates start using them the same shift.",
  openGraph: {
    title: "Skill marketplace — SKYCOT",
    description: "Pre-built skills your AI teammates can use today.",
  },
};

export default function SkillsLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
