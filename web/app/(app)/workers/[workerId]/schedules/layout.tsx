import type { Metadata } from "next";
import type { ReactNode } from "react";
import { findSamplePersona } from "@/lib/sample-personas";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ workerId: string }>;
}): Promise<Metadata> {
  const { workerId } = await params;
  const persona = findSamplePersona(workerId);
  if (persona) {
    return { title: `${persona.name} · Schedules` };
  }
  return { title: "Worker · Schedules" };
}

export default function WorkerSchedulesLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
