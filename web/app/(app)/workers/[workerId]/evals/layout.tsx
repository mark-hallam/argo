import type { Metadata } from "next";
import type { ReactNode } from "react";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ workerId: string }>;
}): Promise<Metadata> {
  const { workerId } = await params;
  return { title: `${workerId} · Evals` };
}

export default function WorkerEvalsLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
