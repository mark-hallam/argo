"use client";

import Link from "next/link";
import { useEffect } from "react";
import { ErrorShell } from "@/components/error/ErrorShell";
import { buttonVariants } from "@/components/ui/Button";

export default function OnboardingError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    if (typeof console !== "undefined") {
      console.error("[skycot:onboarding] onboarding wizard error:", error);
    }
  }, [error]);

  return (
    <ErrorShell
      code="500"
      variant="server-error"
      title="Onboarding hit a snag."
      description="Something went wrong while setting up your workspace. Retrying the step usually fixes it; nothing you've configured so far has been lost."
      actions={
        <>
          <button
            type="button"
            onClick={reset}
            className={buttonVariants({ variant: "primary", size: "md" })}
          >
            Try this step again
          </button>
          <Link
            href="/onboarding/welcome"
            className={buttonVariants({ variant: "secondary", size: "md" })}
          >
            Restart onboarding
          </Link>
        </>
      }
    />
  );
}
