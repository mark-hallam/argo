"use client";

import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

export type OnboardingStep = 1 | 2 | 3 | 4 | 5;

export interface OnboardingDraftState {
  step: OnboardingStep;
  workspaceName: string;
  companyName: string;
  providerKeyConfigured: boolean;
  providerSkipped: boolean;
  verticalId: string | null;
  firstHirePrompt: string;
  tenantId: string | null;
  workspaceId: string | null;
  setStep: (step: OnboardingStep) => void;
  setWorkspace: (workspaceName: string, companyName: string) => void;
  setProvision: (tenantId: string, workspaceId: string) => void;
  setProvider: (configured: boolean, skipped: boolean) => void;
  setVertical: (verticalId: string) => void;
  setFirstHirePrompt: (prompt: string) => void;
  reset: () => void;
}

const INITIAL: Omit<
  OnboardingDraftState,
  | "setStep"
  | "setWorkspace"
  | "setProvision"
  | "setProvider"
  | "setVertical"
  | "setFirstHirePrompt"
  | "reset"
> = {
  step: 1,
  workspaceName: "",
  companyName: "",
  providerKeyConfigured: false,
  providerSkipped: false,
  verticalId: null,
  firstHirePrompt: "",
  tenantId: null,
  workspaceId: null,
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
      setVertical: (verticalId) => set({ verticalId }),
      setFirstHirePrompt: (firstHirePrompt) => set({ firstHirePrompt }),
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
        firstHirePrompt: s.firstHirePrompt,
        tenantId: s.tenantId,
        workspaceId: s.workspaceId,
      }),
      version: 1,
    },
  ),
);
