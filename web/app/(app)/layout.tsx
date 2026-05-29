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
 * override via their own `layout.tsx` metadata. The default is
 * deliberately neutral ("SKYCOT") so any page lacking its own
 * metadata reads brand-correct rather than leaking a sibling route's
 * label (the original "Approvals" default leaked onto /genesis,
 * /brand, /retro, /billing and 4 others — visible in the tab bar
 * during demo recording). Lint pin lives at
 * `scripts/check-route-metadata.sh`.
 */
export const metadata: Metadata = {
  title: {
    default: "SKYCOT",
    template: "%s · SKYCOT",
  },
};

export default function AppGroupLayout({ children }: { children: ReactNode }) {
  return (
    <MotionRoot>
      <AppShell>
        <HideNextDevOverlay />
        <MemberExpiryBanner />
        <RouteTransition>{children}</RouteTransition>
        <HelpWidget />
        <Toaster />
        <WelcomeBackBanner />
      </AppShell>
    </MotionRoot>
  );
}

function HideNextDevOverlay() {
  if (process.env.NEXT_PUBLIC_HIDE_DEV_OVERLAY !== "1") return null;
  return (
    <style>
      {`
        [data-nextjs-toast] {
          display: none !important;
        }
      `}
    </style>
  );
}
