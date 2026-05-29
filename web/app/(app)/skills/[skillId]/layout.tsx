import type { Metadata } from "next";
import type { ReactNode } from "react";

import { resolveSkillDisplayMetadata } from "@/lib/api/skills";

// Server-side metadata for the skill detail surface. The sibling
// `page.tsx` is `"use client"`, so the title + OG cards have to come
// from this layout. Previous version returned only `{ title: skillId }`,
// which meant share-link previews fell through to the parent
// `/skills/layout.tsx` ("Skill marketplace — SKYCOT") instead of the
// skill-specific card.
export async function generateMetadata({
  params,
}: {
  params: Promise<{ skillId: string }>;
}): Promise<Metadata> {
  const { skillId } = await params;
  const { title, description } = resolveSkillDisplayMetadata(skillId);
  return {
    title,
    description,
    openGraph: {
      title: `${title} — SKYCOT skill`,
      description,
    },
  };
}

export default function SkillDetailLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
