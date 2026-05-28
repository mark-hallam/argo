"use client";

import { Suspense, useEffect, useRef, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { OnboardingShell } from "../_OnboardingShell";

/**
 * Step 6 — the celebration beat the wizard was missing. After
 * `/onboarding/first-hire` succeeds we no longer drop the operator
 * straight onto `/workers/<id>` (which reads as "I'm just browsing a
 * worker page"). Instead we route through this page so the punchline
 * lands: **"Meet your first teammate. <Name> is ready."**
 *
 * Per `fix-onboarding-add-done-page-with-meet-your-teammate-handoff`:
 *  - Worker name + id are carried on `?worker=<id>&name=<workerName>`
 *    (the onboarding draft is `reset()` before redirect, so the URL
 *    is the only carrier).
 *  - Primary CTA jumps to `/workers/<id>` immediately.
 *  - A 6-second timeout auto-advances if the operator freezes.
 *  - A `sessionStorage` breadcrumb prevents the loop from re-firing
 *    if the user navigates back to this URL in the same tab.
 */

const AUTO_ADVANCE_MS = 6000;
const SESSION_SEEN_PREFIX = "skycot:onboarding-done-seen:";

export default function OnboardingDonePage() {
  return (
    <Suspense fallback={<OnboardingDoneFallback />}>
      <OnboardingDoneContent />
    </Suspense>
  );
}

function OnboardingDoneFallback() {
  return (
    <OnboardingShell
      step={6}
      stepLabel="DONE"
      eyebrow="MEET YOUR FIRST TEAMMATE"
      heroSegments={[{ text: "Your teammate is ready.", accent: true }]}
    >
      <div />
    </OnboardingShell>
  );
}

function OnboardingDoneContent() {
  const router = useRouter();
  const params = useSearchParams();
  const workerId = params?.get("worker") ?? "";
  const workerName = (params?.get("name") ?? "").trim();
  const displayName = workerName.length > 0 ? workerName : "Your first teammate";
  const overviewPath = workerId ? `/workers/${workerId}` : "/team";
  const teamPath = "/team";
  const timeoutRef = useRef<number | null>(null);
  const [autoAdvancing, setAutoAdvancing] = useState(true);

  function clearAutoAdvance() {
    if (timeoutRef.current !== null && typeof window !== "undefined") {
      window.clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setAutoAdvancing(false);
  }

  useEffect(() => {
    if (typeof window === "undefined") return;
    // If we've already shown this celebration for this worker in this
    // tab, skip the beat — the operator is probably hitting back and
    // we don't want to re-trigger the timer loop indefinitely.
    if (workerId) {
      const key = `${SESSION_SEEN_PREFIX}${workerId}`;
      const seen = window.sessionStorage.getItem(key);
      if (seen === "1") {
        router.replace(overviewPath);
        return;
      }
      window.sessionStorage.setItem(key, "1");
    }
    timeoutRef.current = window.setTimeout(() => {
      timeoutRef.current = null;
      router.replace(overviewPath);
    }, AUTO_ADVANCE_MS);
    return () => {
      if (timeoutRef.current !== null) {
        window.clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
    };
  }, [workerId, overviewPath, router]);

  return (
    <OnboardingShell
      step={6}
      stepLabel="DONE"
      eyebrow="MEET YOUR FIRST TEAMMATE"
      heroSegments={[
        { text: displayName, accent: true },
        { text: " is ready." },
      ]}
      sub={
        <>
          She&rsquo;ll Slack-message you in a few seconds with her first
          questions. <strong>Your draft is saved here</strong> while she settles
          in.
        </>
      }
    >
      <div
        className="ob-form-card"
        data-testid="onboarding-done-card"
        style={{ maxWidth: 640, textAlign: "center" }}
      >
        <div
          className="ob-cta-row is-split"
          style={{ justifyContent: "center" }}
        >
          <button
            type="button"
            className="ob-cta-back"
            onClick={() => {
              clearAutoAdvance();
              router.push(teamPath);
            }}
            data-testid="onboarding-done-view-team"
          >
            View team →
          </button>
          <button
            type="button"
            className="ob-cta"
            onClick={() => {
              clearAutoAdvance();
              router.replace(overviewPath);
            }}
            data-testid="onboarding-done-meet-cta"
          >
            Meet {displayName} →
          </button>
        </div>
        <p
          className="ob-field-hint"
          data-testid="onboarding-done-auto-advance"
          data-state={autoAdvancing ? "armed" : "cancelled"}
          style={{ marginTop: 16 }}
        >
          {autoAdvancing
            ? "Taking you to their profile in a few seconds…"
            : "Take your time."}
        </p>
      </div>
    </OnboardingShell>
  );
}
