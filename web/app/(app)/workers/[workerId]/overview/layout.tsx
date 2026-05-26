import type { Metadata } from "next";
import type { ReactNode } from "react";
import { findSamplePersona } from "@/lib/sample-personas";

// A UUID-shaped id in the browser tab title was the demo-recording
// tell — `stub-018f67d0-... · Overview` is the headline finding in
// `fix-hire-demo-mode-stub-uuid-worker-roster-invisibility`. The
// metadata runs server-side and can't read the session-local demo
// roster, so we resolve the bundled persona name when we recognise
// the id and fall back to a clean "Worker · Overview" otherwise (a
// brand-new demo hire's id is a real UUIDv7 we won't have seen yet;
// the client-side card sets `document.title` from there).
export async function generateMetadata({
  params,
}: {
  params: Promise<{ workerId: string }>;
}): Promise<Metadata> {
  const { workerId } = await params;
  const persona = findSamplePersona(workerId);
  if (persona) {
    return { title: `${persona.name} · Overview` };
  }
  return { title: "Worker · Overview" };
}

export default function WorkerOverviewLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
