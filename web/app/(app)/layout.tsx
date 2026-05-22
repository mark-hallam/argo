import type { Metadata } from "next";
import type { ReactNode } from "react";
import { AppShell } from "@/components/skycot/AppShell";
import { HelpWidget } from "@/components/ui/HelpWidget";
import { MemberExpiryBanner } from "@/components/skycot/members/MemberExpiryBanner";
import { RouteTransition } from "@/lib/motion/primitives";
import { MotionRoot } from "@/lib/motion/provider";
import { Toaster } from "@/components/ui/sonner";
import { WelcomeBackBanner } from "@/components/ui/welcome-back-banner";

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
        <MemberExpiryBanner />
        <RouteTransition>{children}</RouteTransition>
        <HelpWidget />
        <Toaster />
        <WelcomeBackBanner />
      </AppShell>
    </MotionRoot>
  );
}
