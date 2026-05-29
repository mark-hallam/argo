"use client";

import "./marketing-nav.css";

import Link from "next/link";
import { usePathname } from "next/navigation";
import * as RDialog from "@radix-ui/react-dialog";
import { Menu, X } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { ThemeToggle } from "@/components/skycot/ThemeToggle";

/**
 * Shared marketing top-nav used by /landing, /pricing, /customers,
 * /changelog, /docs, /compare, /privacy, /terms, /security.
 *
 * Desktop (≥900px): wordmark + horizontal link row + sign-in + CTA pill.
 * Mobile (<900px): wordmark + hamburger trigger; a right-side Radix
 * Dialog sheet carries the same links + sign-in + CTA. Radix gives
 * focus trap, click-outside-close, and Esc-to-close for free; this
 * component layers first-link autofocus on open per the brief.
 *
 * Active page is detected via `usePathname`. Pages whose hero is an
 * anchor target (e.g. /landing#how) can pass `current="/landing#how"`
 * to override the auto-detection.
 *
 * `extras` is rendered between the link list and the sign-in/CTA both
 * desktop and inside the sheet — used by /changelog for its RSS link.
 */

interface MarketingNavLink {
  href: string;
  label: string;
}

const NAV_LINKS: MarketingNavLink[] = [
  { href: "/landing#how", label: "How it works" },
  { href: "/landing#features", label: "Features" },
  { href: "/compare", label: "Compare" },
  { href: "/teammates", label: "Roster" },
  { href: "/pricing", label: "Pricing" },
  { href: "/docs", label: "Docs" },
  { href: "/changelog", label: "Changelog" },
];

/**
 * Stable, deterministic id for the mobile menu Dialog content. Radix
 * Dialog otherwise derives `aria-controls` (trigger) + `id` (content)
 * from an internal `useId()`; in this SSR setup that value drifts
 * between the server- and client-rendered tree, so React flags an
 * `aria-controls` hydration mismatch on `.mk-burger` and surfaces the
 * red "1 Issue" dev badge on every marketing page. Pinning both sides
 * to a constant removes the random suffix entirely. There is exactly
 * one MarketingNav per page, so a constant id cannot collide.
 */
const MOBILE_MENU_ID = "mk-mobile-menu";

export interface MarketingNavProps {
  /** Explicit active path; defaults to `usePathname()`. */
  current?: string;
  /** Right-side affordance rendered before sign-in (e.g. RSS link). */
  extras?: ReactNode;
}

function linkMatchesPath(href: string, pathname: string | null): boolean {
  if (!pathname) return false;
  const target = href.split("#")[0];
  if (target === "/landing") return pathname === "/landing";
  return pathname === target || pathname.startsWith(`${target}/`);
}

export function MarketingNav({ current, extras }: MarketingNavProps) {
  const pathname = usePathname();
  const active = current ?? pathname;
  const [open, setOpen] = useState(false);
  const firstLinkRef = useRef<HTMLAnchorElement>(null);

  useEffect(() => {
    function onResize() {
      if (window.innerWidth >= 900) setOpen(false);
    }
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return (
    <header className="mk-top">
      <Link href="/landing" className="mk-brand" aria-label="SKYCOT home">
        <div className="mk-brand-mark" />
        <div className="mk-brand-name">SKYCOT</div>
      </Link>

      <nav className="mk-nav" aria-label="Marketing">
        {NAV_LINKS.map((link) => {
          const isActive = linkMatchesPath(link.href, active);
          return (
            <Link
              key={link.href}
              href={link.href}
              aria-current={isActive ? "page" : undefined}
            >
              {link.label}
            </Link>
          );
        })}
      </nav>

      <div className="mk-top-right">
        {extras}
        <ThemeToggle className="mk-theme-toggle" />
        <Link href="/auth/signin" className="mk-signin">
          Sign in
        </Link>
        <Link href="/auth/signup" className="mk-cta-pill">
          Hire your first teammate
        </Link>
      </div>

      <RDialog.Root open={open} onOpenChange={setOpen}>
        <RDialog.Trigger asChild>
          <button
            type="button"
            className="mk-burger"
            aria-label="Open menu"
            aria-controls={MOBILE_MENU_ID}
            data-testid="marketing-nav-open"
          >
            <Menu size={20} />
          </button>
        </RDialog.Trigger>
        <RDialog.Portal>
          <RDialog.Overlay className="mk-sheet-overlay" />
          <RDialog.Content
            id={MOBILE_MENU_ID}
            className="mk-sheet"
            aria-describedby={undefined}
            onOpenAutoFocus={(event) => {
              event.preventDefault();
              firstLinkRef.current?.focus();
            }}
          >
            <div className="mk-sheet-head">
              <RDialog.Title className="mk-brand-name">SKYCOT</RDialog.Title>
              <RDialog.Close asChild>
                <button
                  type="button"
                  className="mk-sheet-close"
                  aria-label="Close menu"
                >
                  <X size={20} />
                </button>
              </RDialog.Close>
            </div>

            <nav className="mk-sheet-links" aria-label="Marketing">
              {NAV_LINKS.map((link, idx) => {
                const isActive = linkMatchesPath(link.href, active);
                return (
                  <Link
                    key={link.href}
                    ref={idx === 0 ? firstLinkRef : undefined}
                    href={link.href}
                    className="mk-sheet-link"
                    aria-current={isActive ? "page" : undefined}
                    onClick={() => setOpen(false)}
                  >
                    {link.label}
                  </Link>
                );
              })}
            </nav>

            {extras ? <div className="mk-sheet-extras">{extras}</div> : null}

            <div className="mk-sheet-theme">
              <ThemeToggle className="mk-theme-toggle" />
            </div>

            <div className="mk-sheet-foot">
              <Link
                href="/auth/signin"
                className="mk-sheet-signin"
                onClick={() => setOpen(false)}
              >
                Sign in
              </Link>
              <Link
                href="/auth/signup"
                className="mk-sheet-cta"
                onClick={() => setOpen(false)}
              >
                Hire your first teammate
              </Link>
            </div>
          </RDialog.Content>
        </RDialog.Portal>
      </RDialog.Root>
    </header>
  );
}
