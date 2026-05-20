import type { Metadata } from "next";
import type { ReactNode } from "react";
import { AppShell } from "@/components/skycot/AppShell";
import { HelpWidget } from "@/components/ui/HelpWidget";
import { RouteTransition } from "@/lib/motion/primitives";
import { MotionRoot } from "@/lib/motion/provider";
import { Toaster } from "@/components/ui/sonner";

/**
 * Default per-tab `<title>` for the (app) group. Individual routes
 * override via their own `layout.tsx` metadata. Without this, every
 * authed page falls back to the root layout's "SKYCOT — Hire your
 * first AI teammate" — a marketing title leaking onto the product
 * tabs on camera.
 */
export const metadata: Metadata = {
  title: {
    default: "Approvals",
    template: "%s · SKYCOT",
  },
};

export default function AppGroupLayout({ children }: { children: ReactNode }) {
  return (
    <MotionRoot>
      <AppShell>
        <RouteTransition>{children}</RouteTransition>
        <HelpWidget />
        <Toaster />
      </AppShell>
    </MotionRoot>
  );
}
