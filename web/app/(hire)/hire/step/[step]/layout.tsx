import type { Metadata } from "next";
import type { ReactNode } from "react";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ step: string }>;
}): Promise<Metadata> {
  const { step } = await params;
  return { title: `Hire · step ${step}` };
}

export default function HireStepLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
