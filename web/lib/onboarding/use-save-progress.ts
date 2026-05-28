"use client";

import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";

/**
 * Onboarding "Exit setup" affordance support.
 *
 * The wizard auto-persists every step to localStorage via the zustand
 * `useOnboardingDraft` store, so a literal "save" call isn't required —
 * the draft is already on disk by the time the operator clicks the
 * top-bar exit button. This hook therefore exists for two reasons:
 *
 * 1. It documents the "save" semantics (the draft is durably persisted
 *    across browser sessions; the operator can resume on next visit by
 *    revisiting `/onboarding/welcome`).
 * 2. It computes the destination the wizard should navigate to once the
 *    operator confirms the exit. Per the brief
 *    (fix-onboarding-save-and-exit-button-safety, UX-audit F10/P2):
 *    - if the workspace has been provisioned (i.e. the gateway returned
 *      a `tenantId`/`workspaceId` during step 2), route to `/workers` —
 *      the operator presumably wants to inspect or list workers next.
 *    - otherwise route to `/landing`, the public-facing home, so the
 *      operator isn't dropped into a protected route they can't browse.
 *
 * This is intentionally a thin selector + a derived flag. The wizard
 * shell calls `saveProgress()` immediately before navigating; in the
 * current implementation that's a no-op (zustand-persist already
 * flushed). Keeping the function in the hook contract leaves room for
 * future gateway-side flush logic without churning the call sites.
 */
export interface SaveProgressResult {
  /** Where the wizard should navigate once exit is confirmed. */
  destination: string;
  /**
   * Whether the destination is a protected route (`/workers`) reached
   * via the provisioned tenant, vs the public landing page. Exposed so
   * the confirm dialog copy can mention the destination explicitly.
   */
  isProvisioned: boolean;
  /**
   * Persist the in-progress draft. No-op today (zustand-persist already
   * mirrors every set into localStorage), kept as an explicit verb so
   * future call sites can rely on the contract.
   */
  saveProgress: () => void;
}

export function useOnboardingSaveProgress(): SaveProgressResult {
  const workspaceId = useOnboardingDraft((s) => s.workspaceId);
  const tenantId = useOnboardingDraft((s) => s.tenantId);
  const isProvisioned = Boolean(workspaceId && tenantId);
  return {
    destination: isProvisioned ? "/workers" : "/landing",
    isProvisioned,
    saveProgress: () => {
      // Intentional no-op — see hook docstring.
    },
  };
}
