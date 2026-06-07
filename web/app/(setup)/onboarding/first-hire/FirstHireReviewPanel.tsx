"use client";

import {
  gateBodyToVerticalReady,
  type FirstHireReview,
} from "@/lib/onboarding/use-first-hire";
import { RoleContractReview } from "@/components/skycot/genesis/RoleContractReview";
import { FirstHireErrorBanner } from "@/components/skycot/onboarding/FirstHireErrorBanner";

/**
 * The onboarding "review before you hire" screen — the broken twin of the
 * `/genesis` studio's review step, now fixed (DEC-025 #1,
 * human-review-before-activation).
 *
 * When the gateway holds the first hire in `needs_review` (HTTP 422), the
 * wizard renders this panel INSTEAD of the form. It REUSES the exact
 * `RoleContractReview` component the studio page uses — adapting the
 * `needs_review` payload into the `GenesisVerticalReadyEvent` shape that
 * component consumes via `gateBodyToVerticalReady`. The operator must tick
 * the built-in confirm checkbox + press the button before `onConfirm` fires;
 * a `blocked` gate renders the component's non-hireable blocked note (the
 * button stays disabled), so a blocked teammate can never be confirmed.
 */
export interface FirstHireReviewPanelProps {
  review: FirstHireReview;
  /** Fires only after the operator confirms in `RoleContractReview`. */
  onConfirm(): void;
  /** Clears the review and returns to the form. */
  onStartOver(): void;
  /** Whether the confirm hire mutation is in-flight. */
  isHiring: boolean;
  /** Confirm-step error copy, when the confirm call failed. */
  error?: string;
}

export function FirstHireReviewPanel({
  review,
  onConfirm,
  onStartOver,
  isHiring,
  error,
}: FirstHireReviewPanelProps) {
  return (
    <div data-testid="onboarding-first-hire-review">
      <RoleContractReview
        ready={gateBodyToVerticalReady(review)}
        onConfirmHire={onConfirm}
        isHiring={isHiring}
      />
      {error ? (
        <div style={{ maxWidth: 920, margin: "16px auto 0", padding: "0 24px" }}>
          <FirstHireErrorBanner message={error} />
        </div>
      ) : null}
      <div style={{ maxWidth: 920, margin: "12px auto 0", padding: "0 24px" }}>
        <button
          type="button"
          className="ob-cta-back"
          onClick={onStartOver}
          disabled={isHiring}
          data-testid="onboarding-first-hire-review-start-over"
        >
          ← Start over
        </button>
      </div>
    </div>
  );
}
