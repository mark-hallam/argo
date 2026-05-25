"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  AlertTriangle,
  Bell,
  BellRing,
  BookOpen,
  Building2,
  Download,
  Key,
  KeyRound,
  Plug,
  Receipt,
  ShieldCheck,
  Users,
  type LucideIcon,
} from "lucide-react";

interface SettingsTab {
  href: string;
  label: string;
  match: string[];
  icon: LucideIcon;
}

const SETTINGS_TABS: SettingsTab[] = [
  {
    href: "/settings",
    label: "Workspace",
    match: ["/settings", "/settings/workspace"],
    icon: Building2,
  },
  { href: "/settings/members", label: "Team", match: ["/settings/members"], icon: Users },
  {
    href: "/settings/notifications",
    label: "Notifications",
    match: ["/settings/notifications"],
    icon: Bell,
  },
  {
    href: "/settings/security",
    label: "Security",
    match: ["/settings/security"],
    icon: ShieldCheck,
  },
  { href: "/settings/providers", label: "Providers", match: ["/settings/providers"], icon: Key },
  { href: "/settings/api-keys", label: "API keys", match: ["/settings/api-keys"], icon: KeyRound },
  {
    href: "/settings/notify-channels",
    label: "Channels",
    match: ["/settings/notify-channels"],
    icon: BellRing,
  },
  {
    href: "/settings/knowledge",
    label: "Knowledge",
    match: ["/settings/knowledge"],
    icon: BookOpen,
  },
  {
    href: "/settings/connectors",
    label: "Connectors",
    match: ["/settings/connectors"],
    icon: Plug,
  },
  { href: "/settings/billing", label: "Billing", match: ["/settings/billing"], icon: Receipt },
  {
    href: "/settings/data-export",
    label: "Data export",
    match: ["/settings/data-export"],
    icon: Download,
  },
];

export function SettingsTabs() {
  const pathname = usePathname() ?? "/settings";

  return (
    <nav className="settings-tabs" aria-label="Settings sections" data-testid="settings-tabs">
      {SETTINGS_TABS.map((tab) => {
        const active = isActive(pathname, tab);
        const Icon = tab.icon;
        return (
          <Link
            key={tab.href}
            href={tab.href}
            className="settings-tab"
            aria-current={active ? "page" : undefined}
            data-testid={`settings-tab-${slugFor(tab.label)}`}
          >
            <Icon aria-hidden />
            {tab.label}
          </Link>
        );
      })}
      <Link
        href="/settings/security#danger-zone"
        className="settings-tab settings-danger-tab"
        data-testid="settings-danger-tab"
      >
        <AlertTriangle aria-hidden />
        Danger zone
      </Link>
    </nav>
  );
}

function isActive(pathname: string, tab: SettingsTab): boolean {
  return tab.match.some((prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`));
}

function slugFor(label: string): string {
  return label.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
}
