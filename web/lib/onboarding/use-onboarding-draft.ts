"use client";

import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

export type OnboardingStep = 1 | 2 | 3 | 4 | 5;

export type SelectedPlan = "free" | "pro";

export interface OnboardingDraftState {
  step: OnboardingStep;
  workspaceName: string;
  companyName: string;
  providerKeyConfigured: boolean;
  providerSkipped: boolean;
  verticalId: string | null;
  /**
   * Display name + skill count captured at pick-vertical time so
   * step 5 ("first hire") can render contextualised copy
   * ("Hire your first Marketplace Seller — 2 skills ready"). Both
   * are populated by `setVertical` from the gateway catalogue row;
   * fall back to `verticalId` if the catalogue is offline.
   */
  verticalDisplayName: string | null;
  verticalSkillCount: number | null;
  firstHireName: string;
  firstHirePrompt: string;
  tenantId: string | null;
  workspaceId: string | null;
  /**
   * Plan the visitor picked on `/pricing` before signup. Persisted so
   * the onboarding welcome step can route a Pro selector straight to
   * `/billing/subscribe?plan=pro` after the magic-link round-trip.
   * `null` means the visitor never came through a tier CTA.
   */
  selectedPlan: SelectedPlan | null;
  /**
   * Flipped to `true` by `onRehydrateStorage` once zustand-persist has
   * finished reading localStorage. Wizard pages gate their form on
   * this so the field inputs never paint with the un-hydrated empty
   * default before the saved draft restores. Excluded from
   * `partialize` — derives from the rehydrate lifecycle, not from
   * stored state.
   */
  hasHydrated: boolean;
  setStep: (step: OnboardingStep) => void;
  setWorkspace: (workspaceName: string, companyName: string) => void;
  setProvision: (tenantId: string, workspaceId: string) => void;
  setProvider: (configured: boolean, skipped: boolean) => void;
  setVertical: (
    verticalId: string,
    displayName?: string | null,
    skillCount?: number | null,
  ) => void;
  setFirstHireName: (name: string) => void;
  setFirstHirePrompt: (prompt: string) => void;
  setSelectedPlan: (plan: SelectedPlan | null) => void;
  setHasHydrated: (hasHydrated: boolean) => void;
  reset: () => void;
}

// `hasHydrated` is deliberately omitted from `INITIAL` so the value
// passed to `reset()` (below) can flip it to `true` — resetting the
// draft mid-session must not roll the gate back to the skeleton.
const INITIAL: Omit<
  OnboardingDraftState,
  | "setStep"
  | "setWorkspace"
  | "setProvision"
  | "setProvider"
  | "setVertical"
  | "setFirstHireName"
  | "setFirstHirePrompt"
  | "setSelectedPlan"
  | "setHasHydrated"
  | "reset"
  | "hasHydrated"
> = {
  step: 1,
  workspaceName: "",
  companyName: "",
  providerKeyConfigured: false,
  providerSkipped: false,
  verticalId: null,
  verticalDisplayName: null,
  verticalSkillCount: null,
  firstHireName: "",
  firstHirePrompt: "",
  tenantId: null,
  workspaceId: null,
  selectedPlan: null,
};

export const ONBOARDING_DRAFT_STORAGE_KEY = "skycot:onboarding-draft-v1";

export const useOnboardingDraft = create<OnboardingDraftState>()(
  persist(
    (set) => ({
      ...INITIAL,
      hasHydrated: false,
      setStep: (step) => set({ step }),
      setWorkspace: (workspaceName, companyName) => set({ workspaceName, companyName }),
      setProvision: (tenantId, workspaceId) => set({ tenantId, workspaceId }),
      setProvider: (providerKeyConfigured, providerSkipped) =>
        set({ providerKeyConfigured, providerSkipped }),
      setVertical: (verticalId, displayName = null, skillCount = null) =>
        set({
          verticalId,
          verticalDisplayName: displayName,
          verticalSkillCount: skillCount,
        }),
      setFirstHireName: (firstHireName) => set({ firstHireName }),
      setFirstHirePrompt: (firstHirePrompt) => set({ firstHirePrompt }),
      setSelectedPlan: (selectedPlan) => set({ selectedPlan }),
      setHasHydrated: (hasHydrated) => set({ hasHydrated }),
      // Reset clears persisted fields but keeps `hasHydrated: true` so
      // the wizard does NOT re-enter the skeleton gate (vitest paths
      // call `reset()` to start each test from a clean draft).
      reset: () => set({ ...INITIAL, hasHydrated: true }),
    }),
    {
      name: ONBOARDING_DRAFT_STORAGE_KEY,
      storage: createJSONStorage(() => ({
        getItem: (key) =>
          typeof window === "undefined" ? null : window.localStorage.getItem(key),
        setItem: (key, value) => {
          if (typeof window !== "undefined") window.localStorage.setItem(key, value);
        },
        removeItem: (key) => {
          if (typeof window !== "undefined") window.localStorage.removeItem(key);
        },
      })),
      partialize: (s) => ({
        step: s.step,
        workspaceName: s.workspaceName,
        companyName: s.companyName,
        providerKeyConfigured: s.providerKeyConfigured,
        providerSkipped: s.providerSkipped,
        verticalId: s.verticalId,
        verticalDisplayName: s.verticalDisplayName,
        verticalSkillCount: s.verticalSkillCount,
        firstHireName: s.firstHireName,
        firstHirePrompt: s.firstHirePrompt,
        tenantId: s.tenantId,
        workspaceId: s.workspaceId,
        selectedPlan: s.selectedPlan,
      }),
      // `onRehydrateStorage` returns a callback that zustand-persist
      // fires after attempting to restore from localStorage — whether
      // a draft was found or not. Flipping `hasHydrated` here lifts
      // the skeleton gate on the wizard pages so the field inputs
      // either show the restored value or render the empty form
      // without an intervening empty-then-full flash.
      onRehydrateStorage: () => (state) => {
        state?.setHasHydrated(true);
      },
      version: 1,
    },
  ),
);
