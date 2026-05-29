import type { Metadata } from "next";
import type { ReactNode } from "react";

import { resolveWorkflowDisplayMetadata } from "@/lib/api/workflow-metadata";

// Server-side metadata for the workflow viewer. The sibling `page.tsx`
// is `"use client"` (react-flow canvas + tanstack query), so it cannot
// export `generateMetadata`; this layout stamps the per-workflow
// title + OG card before the client component hydrates. Without it,
// SSR returns the parent `/workflows` title ("Workflows · SKYCOT") and
// share-link previews show the catalog card instead of the specific
// workflow.
export async function generateMetadata({
  params,
}: {
  params: Promise<{ verticalId: string; name: string }>;
}): Promise<Metadata> {
  const { verticalId, name } = await params;
  const { title, description } = resolveWorkflowDisplayMetadata(verticalId, name);
  return {
    title,
    description,
    openGraph: {
      title: `${title} — SKYCOT workflow`,
      description,
    },
  };
}

export default function WorkflowDetailLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
