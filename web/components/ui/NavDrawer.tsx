"use client";

import * as RDialog from "@radix-ui/react-dialog";
import { Menu, X } from "lucide-react";
import { forwardRef, useEffect, useState, type ReactNode } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

/**
 * Responsive navigation. On mobile (< md) it's a bottom-sheet
 * triggered by a hamburger; on desktop it's a fixed left rail.
 * Same component, two layouts — the surface chooses by viewport,
 * not by prop.
 */
export interface NavDrawerProps {
  /** Navigation content (link list, footer). */
  children: ReactNode;
  /** Brand/logo slot at the top of the drawer. */
  brand?: ReactNode;
  /** Class applied to the desktop rail container. */
  className?: string;
}

export function NavDrawer({ children, brand, className }: NavDrawerProps) {
  const [open, setOpen] = useState(false);

  // Close on resize-to-desktop to avoid stale-open state.
  useEffect(() => {
    function onResize() {
      if (window.innerWidth >= 768) setOpen(false);
    }
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  return (
    <>
      {/* Mobile trigger */}
      <div className="md:hidden sticky top-0 z-40 flex items-center justify-between border-b border-[var(--border)] bg-[var(--background)] px-4 py-3">
        <div className="text-sm font-semibold">{brand}</div>
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setOpen(true)}
          aria-label="Open navigation"
          data-testid="navdrawer-open"
        >
          <Menu />
        </Button>
      </div>

      {/* Mobile sheet */}
      <RDialog.Root open={open} onOpenChange={setOpen}>
        <RDialog.Portal>
          <RDialog.Overlay
            className="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm md:hidden data-[state=open]:animate-in data-[state=closed]:animate-out"
          />
          <RDialog.Content
            className={cn(
              "fixed inset-x-0 bottom-0 z-50 max-h-[80vh] overflow-auto rounded-t-[var(--radius-lg)]",
              "border-t border-[var(--border)] bg-[var(--background)] p-4 md:hidden",
              "data-[state=open]:animate-in data-[state=closed]:animate-out",
            )}
            aria-describedby={undefined}
          >
            <div className="flex items-center justify-between pb-2">
              <RDialog.Title className="text-sm font-semibold">{brand}</RDialog.Title>
              <RDialog.Close asChild>
                <Button variant="ghost" size="icon" aria-label="Close navigation">
                  <X />
                </Button>
              </RDialog.Close>
            </div>
            <div className="pt-2">{children}</div>
          </RDialog.Content>
        </RDialog.Portal>
      </RDialog.Root>

      {/* Desktop rail */}
      <aside
        className={cn(
          "hidden md:flex md:flex-col md:gap-1 md:p-3",
          "md:w-64 md:border-r md:border-[var(--border)] md:bg-[var(--card)]/40",
          "md:backdrop-blur-sm md:fixed md:inset-y-0 md:left-0 md:z-30",
          className,
        )}
      >
        {brand ? (
          <div className="flex items-center gap-2 px-3 py-3 mb-2 border-b border-[var(--border)]">
            {brand}
          </div>
        ) : null}
        <nav className="flex-1 flex flex-col gap-0.5">{children}</nav>
      </aside>
    </>
  );
}

export const NavItem = forwardRef<
  HTMLAnchorElement,
  React.AnchorHTMLAttributes<HTMLAnchorElement> & { active?: boolean; badge?: React.ReactNode }
>(({ className, active, badge, children, ...props }, ref) => (
  <a
    ref={ref}
    data-active={active || undefined}
    className={cn(
      "group flex items-center gap-3 rounded-[var(--radius-md)] px-3 py-2 text-sm text-[var(--muted-foreground)]",
      "transition-colors duration-[var(--duration-fast)]",
      "hover:bg-[var(--accent)] hover:text-[var(--foreground)]",
      "[&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:opacity-70",
      "data-[active=true]:bg-[var(--primary)]/12 data-[active=true]:text-[var(--foreground)] data-[active=true]:font-medium",
      "data-[active=true]:[&_svg]:opacity-100 data-[active=true]:[&_svg]:text-[var(--primary)]",
      className,
    )}
    {...props}
  >
    <span className="flex flex-1 items-center gap-3">{children}</span>
    {badge}
  </a>
));
NavItem.displayName = "NavItem";
