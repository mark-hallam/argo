"use client";

import { AppShell } from "@/components/skycot/AppShell";
import { buttonVariants } from "@/components/ui/Button";

export default function NotFound() {
  return (
    <AppShell>
      <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-20">
        <div className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-16 px-6 text-center space-y-4">
          <h1 className="text-xl font-semibold text-[var(--foreground)]">Page not found</h1>
          <p className="text-sm text-[var(--muted-foreground)]">
            That route doesn&apos;t exist or you don&apos;t have access.
          </p>
          <div className="pt-2">
            <a href="/" className={buttonVariants({ variant: "primary", size: "md" })}>
              Back to approvals
            </a>
          </div>
        </div>
      </div>
    </AppShell>
  );
}
