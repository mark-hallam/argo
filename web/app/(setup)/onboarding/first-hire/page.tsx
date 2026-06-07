"use client";

import { Loader2, Sparkles } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import {
  FIRST_HIRE_VERIFY_FAILED_MESSAGE,
  useFirstHire,
  useConfirmFirstHire,
} from "@/lib/onboarding/use-first-hire";
import { clearOnboardingIdempotencyKey } from "@/lib/onboarding/onboarding-idempotency-key";
import { exampleForVertical } from "@/lib/onboarding/example-prompts";
import { isDemoMode } from "@/lib/skycot/demo-mode";
import { FirstHireErrorBanner } from "@/components/skycot/onboarding/FirstHireErrorBanner";
import { FirstHireReviewPanel } from "./FirstHireReviewPanel";
import { Toaster } from "@/components/ui/sonner";
import { WizardSkeleton } from "@/components/auth/WizardSkeleton";
import { scrubOnboardingErrorMessage } from "../_scrub-error";
import { OnboardingShell } from "../_OnboardingShell";
import {
  ONBOARDING_MAX_LENGTHS,
  firstHireDescriptionSchema,
  firstHireNameSchema,
  parseField,
} from "../schemas";

// Hold the success toast on screen briefly before navigating so the
// buyer registers the "✓ Hired" acknowledgment (F14). Long enough to
// read, short enough not to stall the camera. The (setup) route group
// has no global Toaster (only (app)/layout.tsx mounts one), so this
// page mounts its own — same pattern as the workspace step.
const SUCCESS_TOAST_HOLD_MS = 600;

const NAME_PLACEHOLDER_BY_VERTICAL: Record<string, string> = {
  "marketing-intern": "Maya",
  "gtm-engineer": "Marcus",
  "sales-sdr": "Priya",
  "customer-support": "Jamal",
  "finance-ar": "Nori",
};

export default function OnboardingFirstHirePage() {
  // Gate on persist rehydration — both name and prompt come from the
  // draft store via selectors, so without the gate the textarea would
  // mount with the empty `firstHirePrompt` default before persist
  // resolves and re-renders with the saved value (F8 flash).
  const hasHydrated = useOnboardingDraft((s) => s.hasHydrated);
  if (!hasHydrated) {
    return <WizardSkeleton step={5} />;
  }
  return <FirstHireFormBody />;
}

function FirstHireFormBody() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const setFirstHireNameInStore = useOnboardingDraft((s) => s.setFirstHireName);
  const setFirstHirePromptInStore = useOnboardingDraft((s) => s.setFirstHirePrompt);
  const reset = useOnboardingDraft((s) => s.reset);
  // Read both inputs straight from the draft store so step-back
  // navigation and refresh both restore what the user has typed. The
  // store is the single source of truth — no local mirror to drift out
  // of sync.
  const name = useOnboardingDraft((s) => s.firstHireName);
  const prompt = useOnboardingDraft((s) => s.firstHirePrompt);
  // Step 4 stores the picked verticalId in the draft store; we resolve
  // it to a placeholder so the example matches what the operator just
  // chose. Falls back to the marketing-intern copy when the operator
  // landed here directly (deep link / wizard skip) without picking a
  // template. Keeps Genesis's archetype dispatch aligned with the
  // template the operator selected rather than the placeholder text.
  const pickedVerticalId = useOnboardingDraft((s) => s.verticalId);
  const examplePrompt = exampleForVertical(pickedVerticalId);
  const namePlaceholder = useMemo(
    () =>
      (pickedVerticalId && NAME_PLACEHOLDER_BY_VERTICAL[pickedVerticalId]) ||
      "Maya",
    [pickedVerticalId],
  );
  const firstHire = useFirstHire();
  const confirmHire = useConfirmFirstHire();
  const [error, setError] = useState("");
  const [showHelpLink, setShowHelpLink] = useState(false);
  // Derive the needs_review screen from the mutation's OWN cached data
  // instead of a separate `useState`. Holding it in local state meant a
  // remount during the ~10-40s rubric wait (React Strict Mode in dev, HMR,
  // or any real future remount) lost the `setReview` from the first
  // mutateAsync — the "needs a 2nd click" quirk. Reading `firstHire.data`
  // makes that loss structurally impossible: React Query owns the result.
  // The confirm path never re-holds (a second 422 throws), so a confirmed
  // hire leaves `confirmHire.data.kind === "hired"` and this stays null.
  const review =
    firstHire.data?.kind === "needs_review" &&
    confirmHire.data?.kind !== "hired"
      ? firstHire.data.review
      : null;

  useEffect(() => {
    setStep(5);
  }, [setStep]);

  // Shared success path for BOTH the initial 201 (no-gate) hire AND the
  // post-review confirm. Acknowledges, resets the draft, drops the
  // just-hired flag, holds the toast, then redirects (demo → roster,
  // non-demo → /onboarding/done celebration).
  const succeedHired = useCallback(
    async (workerId: string, workerName: string) => {
      // Acknowledge the hire before navigating away. Without this the
      // page swaps to the celebration/roster with no confirmation that
      // the click landed — the buyer is left guessing whether it worked.
      toast.success(`✓ Hired ${workerName}. Taking you to their page…`);
      reset();
      // Drop the idempotency token alongside the draft reset so the NEXT
      // first hire mints a fresh one rather than re-using this worker's.
      clearOnboardingIdempotencyKey();
      // sessionStorage flag drives the one-shot "Set their schedule"
      // callout on the worker overview page; per-tab + cleared on first
      // read, so a reload doesn't re-surface it. Carries through both
      // the demo-mode roster redirect and the /onboarding/done
      // celebration's auto-advance.
      try {
        sessionStorage.setItem("skycot:just-hired", workerId);
      } catch {
        /* private mode / storage disabled — silently degrade */
      }
      // Hire-complete redirect:
      // - In demo mode: land on /workers?just-hired=1&new=<id> so the
      //   camera sees the roster grow (new card highlighted) — the
      //   recording's own emotional beat. The /onboarding/done
      //   celebration would land between the wizard and the roster and
      //   blunt that punchline. URL pin in
      //   `onboarding-demo-mode-first-hire-lands-on-worker.spec.ts`
      //   still matches. See brief
      //   `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker`.
      // - In non-demo mode: route through `/onboarding/done` so the
      //   F1 "meet your first teammate" celebration lands before the
      //   buyer reaches the worker profile. The done page reads
      //   `?worker=<id>&name=<workerName>` from the URL (the draft is
      //   reset above so the URL is the only carrier) and auto-advances
      //   to `/workers/<id>` after 6s.
      // Brief hold so the success toast is visible before the route
      // change unmounts this page.
      await new Promise((resolve) => setTimeout(resolve, SUCCESS_TOAST_HOLD_MS));
      if (isDemoMode()) {
        router.push(`/workers?just-hired=1&new=${encodeURIComponent(workerId)}`);
      } else {
        const nameParam = encodeURIComponent(workerName);
        router.push(
          `/onboarding/done?worker=${encodeURIComponent(workerId)}&name=${nameParam}`,
        );
      }
    },
    [reset, router],
  );

  // Translate a thrown hire/confirm error into the honest form banner copy.
  function surfaceHireError(e: unknown) {
    // Wave-11 fix (2026-05-21): prior copy claimed "we'll create
    // your worker in the background — try again or skip to your
    // team" but there's no background job and no Skip button. Be
    // honest: the call failed; user retries.
    if (e instanceof Error && e.message === FIRST_HIRE_VERIFY_FAILED_MESSAGE) {
      setError(e.message);
      setShowHelpLink(true);
      return;
    }
    // Scrub internal protocol detail before it reaches the form.
    const scrubbed = scrubOnboardingErrorMessage(e);
    const suffix = scrubbed.wasProtocolLeak
      ? ""
      : " Try again, or come back once the gateway is up.";
    setError(`${scrubbed.message}${suffix}`);
  }

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    // Re-entry guard (FIX 2): the disabled submit button does NOT stop a
    // native form Enter-submit, so a fast double-Enter could fire two hires
    // before the first settled. Bail if a hire is already in flight; the
    // same-key idempotency token is the second line of defence on the wire.
    if (firstHire.isPending) return;
    setError("");
    setShowHelpLink(false);
    // Validate name + description through the shared Zod schemas: bounds the
    // length, rejects whitespace-only, and strips control / zero-width chars
    // before the value becomes the worker's display name everywhere.
    const nameResult = parseField(firstHireNameSchema, name);
    if (!nameResult.ok) {
      setError(nameResult.error);
      return;
    }
    const descResult = parseField(firstHireDescriptionSchema, prompt);
    if (!descResult.ok) {
      setError(descResult.error);
      return;
    }
    const trimmedName = nameResult.value;
    const description = descResult.value;
    const { tenantId, verticalId } = useOnboardingDraft.getState();
    if (!verticalId) {
      setError("Pick a template first.");
      return;
    }
    setFirstHireNameInStore(trimmedName);
    setFirstHirePromptInStore(description);
    try {
      const outcome = await firstHire.mutateAsync({
        name: trimmedName,
        description,
        tenantId,
        verticalId,
      });
      if (outcome.kind === "needs_review") {
        // Gateway derived the truthful contract and is holding the hire for
        // human review (DEC-025 #1). The review screen is now DERIVED from
        // `firstHire.data` (set by the mutation itself), so there's nothing
        // to stash here — just stop and let the render switch to the panel.
        // Form values + draft stay put (no reset, no redirect).
        return;
      }
      await succeedHired(outcome.workerId, outcome.workerName);
    } catch (e) {
      surfaceHireError(e);
    }
  }

  // Fired by RoleContractReview ONLY after the operator ticks confirm +
  // presses the button. Echoes the gateway-returned digest at the confirm
  // call; never auto-confirms.
  async function handleConfirm() {
    if (!review) return;
    setError("");
    setShowHelpLink(false);
    try {
      const outcome = await confirmHire.mutateAsync(review);
      // Confirm always resolves to `hired` (a re-hold throws), but narrow
      // defensively so a future variant can't fall through silently.
      if (outcome.kind === "hired") {
        await succeedHired(outcome.workerId, outcome.workerName);
      }
    } catch (e) {
      surfaceHireError(e);
    }
  }

  function handleStartOver() {
    // Reset the mutation so the DERIVED `review` (read off `firstHire.data`)
    // falls back to null and the form re-renders. `confirmHire.reset()` too,
    // so a prior confirm error/data can't keep the panel pinned. Form values
    // stay in the draft store, so "← Start over" returns to a populated form.
    firstHire.reset();
    confirmHire.reset();
    setError("");
    setShowHelpLink(false);
  }

  const submitting = firstHire.isPending;
  const canSubmit =
    name.trim().length > 0 && prompt.trim().length > 0 && !submitting;

  // Review screen takes over the wizard surface when the gateway held the
  // hire for human review. The form values stay in the draft store so
  // "← Start over" returns to a populated form.
  if (review) {
    return (
      <OnboardingShell
        step={5}
        completedSteps={4}
        stepLabel="FIRST HIRE"
        eyebrow="STEP 05 · REVIEW & CONFIRM"
        heroSegments={[
          { text: "Review what " },
          { text: `${review.name} can do.`, accent: true },
        ]}
        sub={
          <>
            This is the <strong>truthful capability contract</strong> we
            derived — confirm it to hire {review.name}.
          </>
        }
      >
        <FirstHireReviewPanel
          review={review}
          onConfirm={handleConfirm}
          onStartOver={handleStartOver}
          isHiring={confirmHire.isPending}
          error={error}
        />
        <Toaster />
      </OnboardingShell>
    );
  }

  return (
    <OnboardingShell
      step={5}
      completedSteps={4}
      stepLabel="FIRST HIRE"
      eyebrow="STEP 05 · YOUR TEAMMATE"
      heroSegments={[
        { text: "Describe your " },
        { text: "first teammate.", accent: true },
      ]}
      sub={
        <>
          Their <strong>role</strong> and{" "}
          <strong>what they&apos;ll do</strong> — plain English. You can set
          their schedule after they&apos;re hired.
        </>
      }
    >
      <form
        className="ob-form-card"
        onSubmit={onSubmit}
        data-testid="onboarding-first-hire-form"
        noValidate
        style={{ maxWidth: 640 }}
      >
        <label className="ob-field">
          <span className="ob-field-label">Their name</span>
          <input
            className="ob-field-input"
            data-testid="first-hire-name"
            type="text"
            value={name}
            onChange={(e) => setFirstHireNameInStore(e.target.value)}
            placeholder={namePlaceholder}
            aria-label="First hire name"
            autoComplete="off"
            maxLength={ONBOARDING_MAX_LENGTHS.firstHireName}
            autoFocus
            required
          />
        </label>
        <label className="ob-field">
          <span className="ob-field-label">Tell us about them</span>
          <textarea
            className="ob-field-textarea"
            data-testid="first-hire-prompt"
            value={prompt}
            onChange={(e) => setFirstHirePromptInStore(e.target.value)}
            placeholder={examplePrompt}
            aria-label="First hire description"
            maxLength={ONBOARDING_MAX_LENGTHS.firstHireDescription}
            rows={5}
          />
          <p className="ob-field-hint">
            Example: <em>{examplePrompt}</em>
          </p>
        </label>
        {error ? (
          <>
            <FirstHireErrorBanner message={error} showHelpLink={showHelpLink} />
            {/* Escape hatch (F14): a failed hire shouldn't trap the buyer
                on this step. They can retry (resubmit), step Back, or
                bail straight to their team. */}
            <p className="ob-field-hint" data-testid="first-hire-skip-to-team-row">
              Or{" "}
              <Link
                href="/team"
                className="underline"
                data-testid="first-hire-skip-to-team"
              >
                skip to your team →
              </Link>
            </p>
          </>
        ) : null}
        <div className="ob-cta-row is-split">
          <button
            type="button"
            className="ob-cta-back"
            onClick={() => router.push("/onboarding/pick-vertical")}
            data-testid="onboarding-first-hire-back"
          >
            ← Back
          </button>
          <button
            type="submit"
            className="ob-cta"
            disabled={!canSubmit}
            aria-busy={submitting || undefined}
            data-testid="onboarding-first-hire-submit"
          >
            {submitting ? (
              <Loader2 className="size-4 animate-spin" aria-hidden />
            ) : (
              <Sparkles className="size-4" aria-hidden />
            )}
            Hire them
            <span className="ob-cta-kbd" aria-hidden>
              ↵
            </span>
          </button>
        </div>
      </form>
      <Toaster />
    </OnboardingShell>
  );
}
