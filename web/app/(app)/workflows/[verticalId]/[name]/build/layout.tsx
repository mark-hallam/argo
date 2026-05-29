import type { Metadata } from "next";
import type { ReactNode } from "react";

import { resolveWorkflowDisplayMetadata } from "@/lib/api/workflow-metadata";

// Server-side metadata for the workflow builder canvas. The sibling
// `page.tsx` is `"use client"` (react-flow + builder state hooks); the
// parent `[name]/layout.tsx` already stamps the viewer title, so this
// nested layout overrides it with the "Editing …" prefix that signals
// the route is the editor, not the read-only viewer.
export async function generateMetadata({
  params,
}: {
  params: Promise<{ verticalId: string; name: string }>;
}): Promise<Metadata> {
  const { verticalId, name } = await params;
  const { title, description } = resolveWorkflowDisplayMetadata(verticalId, name);
  return {
    title: `Editing ${title}`,
    description,
    openGraph: {
      title: `Editing ${title} — SKYCOT workflow builder`,
      description,
    },
  };
}

export default function WorkflowBuilderLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
