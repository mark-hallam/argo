"use client";

import * as Popover from "@radix-ui/react-popover";
import { ChevronDown, LogOut } from "lucide-react";
import { useRouter } from "next/navigation";
import { signOut as nextAuthSignOut } from "next-auth/react";
import { useState } from "react";
import { Avatar } from "@/components/ui/Avatar";
import { signOut as gatewaySignOut } from "@/lib/api/auth";
import { useSkycotSession } from "@/lib/skycot/use-session";
import { useToast } from "@/lib/ui/use-toast";

/**
 * Sidebar bottom-left identity pill.
 *
 * Reads the current visitor's email + workspace from `useSkycotSession`
 * (which honors `NEXT_PUBLIC_SKYCOT_DEMO_MODE` for v1.0 demo replay) so
 * a fresh signup no longer sees "Sam Harlow · Acmeco · Owner" baked
 * into the chrome. The popover hosts the sign-out action; click flows
 * through the gateway sign-out endpoint to revoke the JWT jti before
 * dropping the NextAuth session client-side.
 */
export function UserPill() {
  const session = useSkycotSession();
  const [open, setOpen] = useState(false);
  const [signingOut, setSigningOut] = useState(false);
  const router = useRouter();
  const toast = useToast();

  async function handleSignOut() {
    if (signingOut) return;
    setSigningOut(true);
    const result = await gatewaySignOut();
    if (!result.ok) {
      setSigningOut(false);
      setOpen(false);
      toast.error("Couldn't sign out — please refresh and try again.");
      return;
    }
    // Gateway has cleared `__Host-session` via Set-Cookie and recorded
    // the JWT's jti in `revoked_tokens`. Drop the NextAuth session
    // client-side; `redirect: false` keeps router state consistent.
    await nextAuthSignOut({ redirect: false });
    toast.success("Signed out");
    router.replace("/auth/signin");
  }

  const avatarAlt = session.displayName;
  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger asChild>
        <button
          type="button"
          data-testid="appshell-user-menu-trigger"
          aria-label="Open account menu"
          className="mt-1 flex items-center gap-2.5 rounded-[var(--radius-md)] px-3 py-2 hover:bg-[var(--accent)] transition-colors duration-[var(--duration-fast)]"
        >
          <Avatar alt={avatarAlt} size="sm" />
          <span className="flex-1 flex flex-col items-start leading-tight min-w-0">
            <span
              data-testid="appshell-user-name"
              className="text-sm font-medium text-[var(--foreground)] truncate max-w-[140px]"
            >
              {session.displayName}
            </span>
            <span
              data-testid="appshell-user-workspace"
              className="text-[11px] text-[var(--muted-foreground)] truncate max-w-[140px]"
            >
              {session.workspaceLabel}
            </span>
          </span>
          <ChevronDown className="size-4 text-[var(--muted-foreground)]" />
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          align="end"
          side="top"
          sideOffset={6}
          className="z-50 min-w-[200px] rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] p-1 shadow-lg"
        >
          {session.email ? (
            <div
              data-testid="appshell-user-email"
              className="px-3 py-2 text-[11px] text-[var(--muted-foreground)] truncate"
            >
              {session.email}
            </div>
          ) : null}
          <button
            type="button"
            data-testid="appshell-sign-out"
            disabled={signingOut}
            onClick={handleSignOut}
            className="flex w-full items-center gap-2 rounded-[var(--radius-sm)] px-3 py-2 text-left text-sm text-[var(--foreground)] hover:bg-[var(--accent)] disabled:opacity-60 disabled:cursor-not-allowed transition-colors duration-[var(--duration-fast)]"
          >
            <LogOut className="size-4 text-[var(--muted-foreground)]" />
            <span>{signingOut ? "Signing out…" : "Sign out"}</span>
          </button>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
