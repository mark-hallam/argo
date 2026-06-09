"use client";

import { useRouter } from "next/navigation";

/**
 * Genesis "describe your own teammate" entry card for onboarding
 * step-04 (`/onboarding/pick-vertical`).
 *
 * Rendered ABOVE the canned-template grid as the headline hire path:
 * a stranger describes the teammate they want in plain English and
 * watches Genesis build it (the describe-and-watch moment that
 * distinguishes SKYCOT from a static template picker). The canned
 * template cards stay below as the fast-lane.
 *
 * Style: reuses the wizard's `.wiz-editorial --ob-*` tokens (they
 * cascade from the shell root) plus a single purple→pink gradient
 * border — the one brand-loud accent in the otherwise-restrained F1
 * wizard (design lock: `project_onboarding_design_locked` F1).
 * `<style jsx>` requires `"use client"`, which this component already
 * declares for `useRouter`.
 */
export function GenesisDescribeCta() {
  const router = useRouter();
  return (
    <button
      type="button"
      className="genesis-describe-cta"
      data-testid="onboarding-genesis-describe-cta"
      // TODO(mission-genesis): carry a `?return=/onboarding/pick-vertical`
      // hint once `/genesis` consumes a return param. It does not read
      // one today (verified 2026-06-09), so route plain to avoid an
      // unconsumed query string.
      onClick={() => router.push("/genesis")}
    >
      <span className="genesis-describe-cta__icon" aria-hidden>
        ✦
      </span>
      <span className="genesis-describe-cta__body">
        <span className="genesis-describe-cta__title">
          Describe your own teammate
        </span>
        <span className="genesis-describe-cta__sub">
          Tell us what you need in plain English &mdash; AI will build it, live.
        </span>
      </span>
      <span className="genesis-describe-cta__arrow" aria-hidden>
        →
      </span>
      <style jsx>{`
        .genesis-describe-cta {
          position: relative;
          width: 100%;
          display: flex;
          align-items: center;
          gap: 16px;
          padding: 20px 22px;
          border-radius: 16px;
          /* The single brand-loud accent: a purple→pink gradient
             border over a solid fill, via the double-background
             padding-box/border-box trick so the 2px edge reads as
             gradient without tinting the card body. */
          border: 2px solid transparent;
          background:
            linear-gradient(var(--ob-bg, #ffffff), var(--ob-bg, #ffffff))
              padding-box,
            var(--ob-gradient, linear-gradient(135deg, #8b5cf6, #ec4899))
              border-box;
          text-align: left;
          font-family: "Inter", sans-serif;
          color: var(--ob-ink, #1e1b4b);
          cursor: pointer;
          transition:
            transform 0.15s,
            box-shadow 0.15s;
        }
        .genesis-describe-cta:hover {
          transform: translateY(-2px);
          box-shadow: 0 12px 30px -10px rgba(139, 92, 246, 0.3);
        }
        .genesis-describe-cta:focus-visible {
          outline: none;
          box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.25);
        }
        .genesis-describe-cta__icon {
          flex-shrink: 0;
          width: 40px;
          height: 40px;
          border-radius: 12px;
          display: grid;
          place-items: center;
          font-size: 20px;
          color: #ffffff;
          background: var(
            --ob-gradient,
            linear-gradient(135deg, #8b5cf6, #ec4899)
          );
        }
        .genesis-describe-cta__body {
          display: flex;
          flex-direction: column;
          gap: 3px;
          flex: 1;
          min-width: 0;
        }
        .genesis-describe-cta__title {
          font-size: 17px;
          font-weight: 700;
          letter-spacing: -0.015em;
        }
        .genesis-describe-cta__sub {
          font-size: 13.5px;
          color: var(--ob-ink-2, #3f3f46);
          line-height: 1.5;
        }
        .genesis-describe-cta__arrow {
          flex-shrink: 0;
          font-size: 18px;
          color: var(--ob-purple, #8b5cf6);
        }
        @media (max-width: 480px) {
          .genesis-describe-cta {
            gap: 12px;
            padding: 16px 18px;
          }
        }
      `}</style>
    </button>
  );
}
