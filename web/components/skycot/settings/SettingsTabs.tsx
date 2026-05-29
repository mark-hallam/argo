"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  Bell,
  BellRing,
  BookOpen,
  Building2,
  ChevronLeft,
  ChevronRight,
  Download,
  Inbox,
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
    href: "/settings/inbound-channels",
    label: "Inbound",
    match: ["/settings/inbound-channels"],
    icon: Inbox,
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

const SCROLL_STEP_PX = 220;

export function SettingsTabs() {
  const pathname = usePathname() ?? "/settings";
  const navRef = useRef<HTMLElement | null>(null);
  const [overflowLeft, setOverflowLeft] = useState(false);
  const [overflowRight, setOverflowRight] = useState(false);

  const recompute = useCallback(() => {
    const el = navRef.current;
    if (!el) return;
    const overflow = el.scrollWidth - el.clientWidth;
    setOverflowLeft(el.scrollLeft > 1);
    setOverflowRight(overflow > 1 && el.scrollLeft < overflow - 1);
  }, []);

  useEffect(() => {
    const el = navRef.current;
    if (!el) return;
    recompute();
    const onScroll = () => recompute();
    el.addEventListener("scroll", onScroll, { passive: true });
    let ro: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined") {
      ro = new ResizeObserver(() => recompute());
      ro.observe(el);
    }
    window.addEventListener("resize", recompute);
    return () => {
      el.removeEventListener("scroll", onScroll);
      ro?.disconnect();
      window.removeEventListener("resize", recompute);
    };
  }, [recompute]);

  const scrollByStep = (delta: number) => {
    const el = navRef.current;
    if (!el) return;
    el.scrollBy({ left: delta, behavior: "smooth" });
  };

  const wrapperClass = [
    "settings-tabs-overflow",
    overflowLeft ? "is-left-overflow" : "",
    overflowRight ? "is-right-overflow" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <>
      <style dangerouslySetInnerHTML={{ __html: SETTINGS_TABS_OVERFLOW_STYLES }} />
      <div
        className={wrapperClass}
        data-testid="settings-tabs-overflow"
        data-overflow-left={overflowLeft ? "true" : "false"}
        data-overflow-right={overflowRight ? "true" : "false"}
      >
        <button
          type="button"
          className="settings-tabs-scroll-button settings-tabs-scroll-left"
          aria-label="Scroll settings tabs left"
          aria-hidden={!overflowLeft}
          tabIndex={overflowLeft ? 0 : -1}
          onClick={() => scrollByStep(-SCROLL_STEP_PX)}
          data-testid="settings-tabs-scroll-left"
        >
          <ChevronLeft aria-hidden />
        </button>
        <nav
          ref={navRef}
          className="settings-tabs"
          aria-label="Settings sections"
          data-testid="settings-tabs"
        >
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
        <button
          type="button"
          className="settings-tabs-scroll-button settings-tabs-scroll-right"
          aria-label="Scroll settings tabs right"
          aria-hidden={!overflowRight}
          tabIndex={overflowRight ? 0 : -1}
          onClick={() => scrollByStep(SCROLL_STEP_PX)}
          data-testid="settings-tabs-scroll-right"
        >
          <ChevronRight aria-hidden />
        </button>
      </div>
    </>
  );
}

function isActive(pathname: string, tab: SettingsTab): boolean {
  return tab.match.some((prefix) => {
    if (prefix === "/settings") return pathname === "/settings";
    return pathname === prefix || pathname.startsWith(`${prefix}/`);
  });
}

function slugFor(label: string): string {
  return label.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
}

const SETTINGS_TABS_OVERFLOW_STYLES = `
.settings-tabs-overflow {
  position: relative;
}
.settings-tabs-overflow > .settings-tabs {
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.settings-tabs-overflow > .settings-tabs::-webkit-scrollbar {
  display: none;
}
.settings-tabs-overflow::before,
.settings-tabs-overflow::after {
  content: "";
  position: absolute;
  top: 7px;
  bottom: 29px;
  width: 44px;
  pointer-events: none;
  opacity: 0;
  transition: opacity 140ms ease;
  z-index: 1;
}
.settings-tabs-overflow::before {
  left: 7px;
  background: linear-gradient(
    90deg,
    var(--settings-card, #ffffff) 0%,
    color-mix(in srgb, var(--settings-card, #ffffff) 0%, transparent) 100%
  );
  border-radius: 7px 0 0 7px;
}
.settings-tabs-overflow::after {
  right: 7px;
  background: linear-gradient(
    270deg,
    var(--settings-card, #ffffff) 0%,
    color-mix(in srgb, var(--settings-card, #ffffff) 0%, transparent) 100%
  );
  border-radius: 0 7px 7px 0;
}
.settings-tabs-overflow.is-left-overflow::before {
  opacity: 1;
}
.settings-tabs-overflow.is-right-overflow::after {
  opacity: 1;
}
.settings-tabs-scroll-button {
  position: absolute;
  top: 50%;
  transform: translateY(calc(-50% - 11px));
  z-index: 2;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 999px;
  border: 1px solid var(--settings-border, #e5e7eb);
  background: var(--settings-card, #ffffff);
  color: var(--settings-ink, #1f2937);
  cursor: pointer;
  box-shadow: 0 6px 14px -10px rgba(15, 23, 42, 0.4);
  opacity: 0;
  pointer-events: none;
  transition: opacity 140ms ease, background 140ms ease;
  padding: 0;
}
.settings-tabs-scroll-button svg {
  width: 14px;
  height: 14px;
}
.settings-tabs-scroll-button:hover {
  background: var(--settings-soft, color-mix(in srgb, #f4f4f5 68%, transparent));
}
.settings-tabs-scroll-button:focus-visible {
  outline: 2px solid var(--ring, #6366f1);
  outline-offset: 2px;
  opacity: 1;
  pointer-events: auto;
}
.settings-tabs-scroll-left {
  left: -2px;
}
.settings-tabs-scroll-right {
  right: -2px;
}
.settings-tabs-overflow.is-left-overflow .settings-tabs-scroll-left,
.settings-tabs-overflow.is-right-overflow .settings-tabs-scroll-right {
  opacity: 1;
  pointer-events: auto;
}
`;
