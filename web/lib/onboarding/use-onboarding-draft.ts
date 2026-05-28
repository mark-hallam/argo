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
  reset: () => void;
}

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
  | "reset"
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
      reset: () => set({ ...INITIAL }),
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
      version: 1,
    },
  ),
);
