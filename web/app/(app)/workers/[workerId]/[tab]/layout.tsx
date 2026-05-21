import type { Metadata } from "next";
import type { ReactNode } from "react";

const TAB_TITLES: Record<string, string> = {
  overview: "Overview",
  activity: "Activity",
  memory: "Memory",
  skills: "Skills",
  trust: "Trust",
  escalation: "Escalation",
  settings: "Settings",
};

export async function generateMetadata({
  params,
}: {
  params: Promise<{ workerId: string; tab: string }>;
}): Promise<Metadata> {
  const { workerId, tab } = await params;
  const label = TAB_TITLES[tab] ?? tab;
  return { title: `${workerId} · ${label}` };
}

export default function WorkerTabLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
