import type { ReactNode } from "react";
import { AppShell } from "@/components/skycot/AppShell";
import { HelpWidget } from "@/components/ui/HelpWidget";
import { RouteTransition } from "@/lib/motion/primitives";
import { MotionRoot } from "@/lib/motion/provider";
import { Toaster } from "@/components/ui/sonner";

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
