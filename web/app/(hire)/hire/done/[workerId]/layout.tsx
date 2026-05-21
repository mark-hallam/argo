import type { Metadata } from "next";
import type { ReactNode } from "react";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ workerId: string }>;
}): Promise<Metadata> {
  const { workerId } = await params;
  return { title: `Welcome to ${workerId}` };
}

export default function HireDoneLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
