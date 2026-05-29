"use client";

import { usePathname } from "next/navigation";
import { findActiveTab } from "./SettingsTabs";

export function SettingsHeading() {
  const pathname = usePathname() ?? "/settings";
  const label = findActiveTab(pathname)?.label ?? "Workspace";
  return (
    <h1 className="sr-only" data-testid="settings-sr-only-heading">
      Settings: {label}
    </h1>
  );
}
