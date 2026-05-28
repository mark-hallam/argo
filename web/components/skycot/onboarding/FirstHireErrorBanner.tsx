"use client";

import Link from "next/link";
import { AlertTriangle } from "lucide-react";

interface FirstHireErrorBannerProps {
  message: string;
  showHelpLink?: boolean;
}

export function FirstHireErrorBanner({
  message,
  showHelpLink = false,
}: FirstHireErrorBannerProps) {
  return (
    <div
      className="ob-form-error"
      data-testid="onboarding-first-hire-error"
      role="alert"
    >
      <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden />
      <span>
        {message}
        {showHelpLink ? (
          <>
            {" "}
            <Link
              href="/help"
              className="underline"
              data-testid="onboarding-first-hire-error-help-link"
            >
              Help
            </Link>
          </>
        ) : null}
      </span>
    </div>
  );
}
