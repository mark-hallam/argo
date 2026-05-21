import type { Metadata } from "next";
import type { ReactNode } from "react";

export async function generateMetadata({
  params,
}: {
  params: Promise<{ skillId: string }>;
}): Promise<Metadata> {
  const { skillId } = await params;
  return { title: skillId };
}

export default function SkillDetailLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
