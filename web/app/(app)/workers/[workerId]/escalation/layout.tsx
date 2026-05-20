import type { Metadata, ResolvingMetadata } from "next";
import type { ReactNode } from "react";

/**
 * Metadata-only layout for the escalation tab. Lets us declare a
 * dynamic `<title>` per worker without forcing the page itself
 * into a server component (the chart needs client-side hover).
 */
export async function generateMetadata(
  { params }: { params: Promise<{ workerId: string }> },
  _parent: ResolvingMetadata,
): Promise<Metadata> {
  const { workerId } = await params;
  return {
    title: `Escalation · ${workerId} · SKYCOT`,
    description:
      "Escalation rules, when-to-escalate matrix, and the 90-day trust-grade history for this worker.",
  };
}

export default function EscalationLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
