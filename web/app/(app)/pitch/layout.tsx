import type { Metadata } from "next";
import type { ReactNode } from "react";

// `/pitch` is the operator-facing rehearsal surface for the Pitch
// Agent (Portia, per DEC-016). The page itself is a `"use client"`
// component — it calls the pitch API and manages chat state — so it
// cannot export `metadata` directly. This sibling layout supplies the
// static per-route metadata so the browser tab reads
// "Pitch rehearsal · SKYCOT" instead of falling back to the (app)
// group default (a camera-readable error during demo recording).
export const metadata: Metadata = {
  title: "Pitch rehearsal",
  description: "Rehearse with Portia, the truth-bounded Pitch Agent.",
  openGraph: {
    title: "Pitch rehearsal — SKYCOT",
    description: "Rehearse with Portia, the truth-bounded Pitch Agent.",
  },
};

export default function PitchLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
