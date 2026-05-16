import type { ReactNode } from "react";
import { AppShell } from "@/components/skycot/AppShell";
import { HelpWidget } from "@/components/ui/HelpWidget";

export default function AppGroupLayout({ children }: { children: ReactNode }) {
  return (
    <AppShell>
      {children}
      <HelpWidget />
    </AppShell>
  );
}
