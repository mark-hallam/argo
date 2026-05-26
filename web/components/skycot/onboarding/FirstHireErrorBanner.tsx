"use client";

import { AlertTriangle } from "lucide-react";

interface FirstHireErrorBannerProps {
  message: string;
}

export function FirstHireErrorBanner({ message }: FirstHireErrorBannerProps) {
  return (
    <div
      className="ob-form-error"
      data-testid="onboarding-first-hire-error"
      role="alert"
    >
      <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden />
      <span>{message}</span>
    </div>
  );
}
