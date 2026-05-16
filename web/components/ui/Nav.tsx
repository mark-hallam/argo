"use client";

import { Menu, X } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

export interface NavLink {
  label: string;
  href: string;
}

export interface NavProps {
  brand: ReactNode;
  links: NavLink[];
  cta?: ReactNode;
  sticky?: boolean;
  className?: string;
}

export function Nav({ brand, links, cta, sticky = true, className }: NavProps) {
  const [scrolled, setScrolled] = useState(false);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!sticky) return;
    function onScroll() {
      setScrolled(window.scrollY > 8);
    }
    window.addEventListener("scroll", onScroll, { passive: true });
    onScroll();
    return () => window.removeEventListener("scroll", onScroll);
  }, [sticky]);

  return (
    <header
      data-testid="site-nav"
      data-scrolled={scrolled || undefined}
      className={cn(
        "w-full z-40 transition-colors duration-[var(--motion-fast)]",
        sticky && "sticky top-0",
        scrolled
          ? "bg-[var(--background)]/85 backdrop-blur border-b border-[var(--border)]"
          : "bg-transparent",
        className,
      )}
    >
      <div className="mx-auto max-w-6xl px-6 h-14 flex items-center justify-between gap-4">
        <div className="text-base font-semibold text-[var(--foreground)]">{brand}</div>
        <nav className="hidden md:flex items-center gap-6" aria-label="Primary">
          {links.map((l) => (
            <a
              key={l.href}
              href={l.href}
              className="text-sm text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors duration-[var(--motion-fast)]"
            >
              {l.label}
            </a>
          ))}
        </nav>
        <div className="hidden md:block">{cta}</div>
        <Button
          variant="ghost"
          size="icon"
          className="md:hidden"
          onClick={() => setOpen((v) => !v)}
          aria-label={open ? "Close menu" : "Open menu"}
          aria-expanded={open}
          data-testid="nav-burger"
        >
          {open ? <X /> : <Menu />}
        </Button>
      </div>
      {open ? (
        <div
          id="nav-mobile-panel"
          className="md:hidden border-t border-[var(--border)] bg-[var(--background)] px-6 py-4 flex flex-col gap-2"
        >
          {links.map((l) => (
            <a
              key={l.href}
              href={l.href}
              onClick={() => setOpen(false)}
              className="text-sm py-1 text-[var(--foreground)]"
            >
              {l.label}
            </a>
          ))}
          {cta ? <div className="mt-2">{cta}</div> : null}
        </div>
      ) : null}
    </header>
  );
}
