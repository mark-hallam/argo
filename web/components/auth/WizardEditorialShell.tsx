"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useState, type ReactNode } from "react";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { DemoModeBanner } from "@/components/skycot/DemoModeBanner";

/**
 * Locked F1 editorial-center wizard shell (2026-05-19 design lock).
 *
 * Companion to `AuthEditorialShell`. Wraps the `/hire` wizard steps in
 * the same brand language: Audiowide hero typography, purple→pink→coral
 * gradient on emphasized words + CTA, warm radial-blob page bg, A-style
 * top bar (brand · step strip · Help / SAVE & EXIT), centred editorial
 * stage with eyebrow + hero + sub + body, and a trust-strip footer.
 *
 * Owned by the 5-step `/hire` wizard + the `/hire/done/<workerId>`
 * celebration. Each page passes per-step hero copy + a body slot; the
 * shell owns the chrome, the exit-confirm dialog, and the controlled
 * back/continue footer.
 */

const TOTAL_STEPS = 5;

export type WizardStep = 1 | 2 | 3 | 4 | 5 | 6;

interface HeroSpan {
  text: string;
  /** When true the segment renders in the purple→pink→coral gradient. */
  accent?: boolean;
}

export interface WizardEditorialShellProps {
  step: WizardStep;
  /**
   * How many prior steps the user has completed. Defaults to `step - 1`
   * (i.e. the active step itself is not yet complete). Pages can pass
   * a higher value during the brief "submit success" window to light
   * the final pip before navigating away.
   */
  completedSteps?: number;
  /** Short uppercase label shown in the step strip (e.g. "WELCOME"). */
  stepLabel: string;
  /** Eyebrow pill text (e.g. "WELCOME, MARK"). */
  eyebrow?: string;
  /** Hero headline as plain/accent segments so one or more words can carry the gradient. */
  heroSegments: HeroSpan[];
  sub?: ReactNode;
  children?: ReactNode;
  /** True if the user has draft data — exit attempts route through the dialog. */
  unsavedHire?: boolean;
  continueDisabled?: boolean;
  /** Override the default Back handler (`router.back()` if absent). */
  onBack?: () => void;
  /** Continue handler — provide to enable the CTA. */
  onContinue?: () => void;
  /** Called when the user confirms leaving an in-progress flow. */
  onDiscard?: () => void;
  backLabel?: string;
  continueLabel?: string;
  /** Optional ReactNode rendered below the CTA row (small link, etc.). */
  belowCta?: ReactNode;
  /** Hide the back/continue footer row (auto-advancing or terminal steps). */
  hideFooter?: boolean;
}

const WIZARD_STYLES = `
/* In dark color-scheme the document body bg is near-black
   (rgb(2,7,11)) and any overscroll, scroll-rubber-band, or
   bottom-of-page area below the shell shows it as a stripe under
   the cream/lavender wizard. The wizard is always-light by design
   (it hardcodes paper + soft gradients regardless of theme), so we
   pin html + body to the wizard's paper bg and force color-scheme
   light wherever the wizard is mounted. Scoped via :has() so other
   routes keep their dark-mode chrome. */
html:has(.wiz-editorial),
body:has(.wiz-editorial) {
  background-color: #fafaff;
  color-scheme: light;
}
.wiz-editorial {
  --purple: #8b5cf6; --purple-deep: #6d28d9;
  --pink: #ec4899; --coral: #ff6b6b; --yellow: #fcd34d;
  --teal: #14b8a6; --cyan: #0891b2;
  --ink: #1e1b4b; --ink-2: #3f3f46; --ink-3: #52525b; --ink-4: #a1a1aa;
  --paper-2: #f4f4f4; --bg: #fafaff;
  --line: #ece9f5; --line-2: #d1d1d1;
  --gradient: linear-gradient(135deg, var(--purple), var(--pink));
  --gradient-3: linear-gradient(135deg, var(--purple), var(--pink), var(--coral));
  /* WCAG 2.1 AA — lock shadcn semantic tokens to F1 light values so
     body content (Step1Land et al.) using text-[var(--foreground)]
     stays dark-ink when the user has dark theme set globally. The
     wizard bg is hardcoded light, so a globally-flipping --foreground
     produces white-on-white text (audit F9, 2026-05-26-2309).
     #1e1b4b on #fafaff ≈ 15.4:1; #52525b on #fafaff ≈ 7.0:1. */
  --foreground: #1e1b4b;
  --muted-foreground: #52525b;
  --background: #fafaff;
  --border: #ece9f5;
  font-family: "Inter", sans-serif;
  background: var(--bg); color: var(--ink);
  font-size: 16px; line-height: 1.55;
  -webkit-font-smoothing: antialiased; letter-spacing: -0.005em;
  min-height: 100vh; position: relative;
}
.wiz-editorial::before, .wiz-editorial::after {
  content: ""; position: fixed; pointer-events: none; z-index: 0;
}
.wiz-editorial::before {
  top: -20%; right: -10%; width: 700px; height: 700px;
  background: radial-gradient(circle, rgba(139, 92, 246, 0.15) 0%, transparent 70%);
}
.wiz-editorial::after {
  bottom: -30%; left: -10%; width: 800px; height: 800px;
  background: radial-gradient(circle, rgba(236, 72, 153, 0.1) 0%, transparent 70%);
}
.wiz-top, .wiz-stage, .wiz-foot { position: relative; z-index: 1; }
.wiz-top {
  padding: 22px 48px;
  display: grid; grid-template-columns: 1fr auto 1fr;
  align-items: center;
  border-bottom: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
  position: sticky; top: 0; z-index: 10;
}
.wiz-brand {
  display: flex; align-items: center; gap: 12px; justify-self: start;
  background: none; border: none; padding: 0; cursor: pointer;
  font: inherit; color: inherit;
}
.wiz-brand-mark {
  width: 28px; height: 28px; border-radius: 7px;
  background: var(--gradient);
  box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.3);
}
.wiz-brand-name {
  font-family: var(--font-audiowide);
  font-size: 16px; letter-spacing: 0.08em; color: var(--ink);
}
.wiz-steps {
  display: flex; align-items: center; gap: 10px;
  font-family: var(--font-mono);
  font-size: 11px; color: var(--ink-3);
  letter-spacing: 0.18em; text-transform: uppercase; font-weight: 600;
}
.wiz-steps-pos { color: var(--ink); font-weight: 700; }
.wiz-steps-label { color: var(--ink-2); font-weight: 700; }
.wiz-pips { display: flex; gap: 4px; margin: 0 12px; }
.wiz-pip {
  width: 24px; height: 3px; border-radius: 2px;
  background: var(--paper-2);
}
.wiz-pip-done {
  background: var(--gradient);
  box-shadow: 0 0 8px rgba(139, 92, 246, 0.45);
}
.wiz-pip-active {
  background: linear-gradient(90deg, var(--purple) 0%, var(--purple) 50%, var(--paper-2) 50%, var(--paper-2) 100%);
  outline: 1px solid var(--purple);
  outline-offset: 0;
}
.wiz-top-right {
  justify-self: end;
  display: flex; align-items: center; gap: 16px;
}
.wiz-help-btn {
  padding: 6px 12px; border-radius: 7px;
  border: 1px solid var(--line); background: var(--bg);
  color: var(--ink-2);
  font-size: 12px; font-weight: 500;
  cursor: pointer; font-family: "Inter", sans-serif;
  text-decoration: none;
}
.wiz-exit {
  font-size: 13px; color: var(--ink-3);
  background: none; border: none; padding: 0;
  cursor: pointer; font-family: var(--font-mono);
  letter-spacing: 0.06em;
}
.wiz-exit:hover { color: var(--ink); }
.wiz-stage {
  max-width: 840px; margin: 0 auto;
  padding: 96px 48px 80px; text-align: center;
}
.wiz-eyebrow {
  display: inline-flex; align-items: center; gap: 6px;
  font-family: var(--font-mono);
  font-size: 11px; font-weight: 700;
  letter-spacing: 0.2em; text-transform: uppercase;
  color: var(--ink-3); margin-bottom: 28px;
  padding: 6px 16px; border-radius: 999px;
  border: 1px solid var(--line); background: var(--bg);
}
.wiz-eyebrow-dot {
  width: 6px; height: 6px; border-radius: 50%;
  background: var(--teal); box-shadow: 0 0 6px var(--teal);
}
.wiz-hero {
  font-family: var(--font-audiowide);
  font-size: 76px; line-height: 1.0;
  font-weight: 400; letter-spacing: 0;
  margin: 0 auto 24px; color: var(--ink);
  max-width: 900px;
}
.wiz-hero-accent {
  background: var(--gradient-3);
  -webkit-background-clip: text; background-clip: text;
  color: transparent; font-weight: 400;
}
.wiz-sub {
  font-size: 19px; font-weight: 300; color: var(--ink-2);
  max-width: 640px; margin: 0 auto 48px;
  line-height: 1.5; letter-spacing: -0.01em;
}
.wiz-sub strong { color: var(--ink); font-weight: 600; }
.wiz-body { text-align: left; margin: 0 auto; max-width: 720px; }
.wiz-cta-row {
  margin-top: 40px;
  display: flex; gap: 16px;
  align-items: center; justify-content: center; flex-wrap: wrap;
}
.wiz-cta {
  display: inline-flex; align-items: center; gap: 12px;
  padding: 18px 36px; border-radius: 14px;
  font-family: "Inter", sans-serif;
  font-size: 16px; font-weight: 700; letter-spacing: -0.005em;
  background: var(--gradient); color: white;
  border: none; cursor: pointer;
  box-shadow: 0 8px 32px -8px rgba(139, 92, 246, 0.45),
    inset 0 1px 0 rgba(255, 255, 255, 0.25);
  transition: transform 0.15s, box-shadow 0.15s, opacity 0.15s;
}
.wiz-cta:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 12px 40px -8px rgba(139, 92, 246, 0.6),
    inset 0 1px 0 rgba(255, 255, 255, 0.3);
}
.wiz-cta:disabled { opacity: 0.5; cursor: not-allowed; box-shadow: none; }
.wiz-cta-back {
  background: transparent; color: var(--ink-2);
  border: 1px solid var(--line); box-shadow: none;
  padding: 18px 28px;
}
.wiz-cta-back:hover:not(:disabled) {
  transform: translateY(-1px);
  border-color: var(--line-2);
  background: rgba(255, 255, 255, 0.5);
  box-shadow: none;
}
.wiz-cta-kbd {
  font-family: var(--font-mono);
  font-size: 12px; padding: 3px 8px; border-radius: 5px;
  background: rgba(255, 255, 255, 0.22); letter-spacing: 0.04em;
}
.wiz-below-cta {
  margin-top: 16px; font-size: 13px;
  color: var(--ink-3); text-align: center;
}
.wiz-below-cta a {
  color: var(--ink-2); text-decoration: none;
  border-bottom: 1px solid var(--line-2);
}
.wiz-below-cta a:hover { color: var(--purple); border-color: var(--purple); }
.wiz-foot {
  margin-top: 64px; padding: 22px 32px;
  border-top: 1px solid var(--line);
  display: flex; gap: 36px;
  justify-content: center; align-items: center; flex-wrap: wrap;
  font-size: 11px; color: var(--ink-3);
}
.wiz-foot-item {
  display: inline-flex; align-items: center; gap: 8px;
  font-family: var(--font-mono);
  letter-spacing: 0.06em; text-transform: uppercase; font-weight: 600;
}
.wiz-foot-item strong { color: var(--ink); font-weight: 700; }
.wiz-foot-dot { color: var(--teal); }
@media (max-width: 800px) {
  .wiz-top { padding: 16px 20px; grid-template-columns: auto 1fr auto; gap: 12px; }
  .wiz-steps { font-size: 10px; gap: 6px; }
  .wiz-pips { margin: 0 6px; }
  .wiz-pip { width: 16px; }
  .wiz-stage { padding: 48px 20px 60px; }
  .wiz-hero { font-size: 44px; }
  .wiz-sub { font-size: 16px; }
  .wiz-foot { gap: 18px; }
}
`;

function StepStrip({
  step,
  stepLabel,
  completedSteps,
}: {
  step: WizardStep;
  stepLabel: string;
  completedSteps: number;
}) {
  const displayStep = step === 6 ? TOTAL_STEPS : step;
  const stepNumber = step === 6 ? "✓" : String(displayStep).padStart(2, "0");
  const totalLabel = String(TOTAL_STEPS).padStart(2, "0");
  // The visible "STEP NN OF NN · LABEL" runs in a single accessible-name
  // node (aria-label below) so screen readers say "Step 5 of 5 · First hire"
  // in one go, not "step, zero, five, of, zero, five" as separate static
  // text nodes (UX-audit F15, P3 a11y nit).
  const accessibleLabel = `Step ${displayStep} of ${TOTAL_STEPS} · ${stepLabel}`;
  return (
    <div
      className="wiz-steps"
      role="progressbar"
      aria-valuemin={1}
      aria-valuemax={TOTAL_STEPS}
      aria-valuenow={displayStep}
      aria-label={accessibleLabel}
    >
      <span aria-hidden>
        STEP <span className="wiz-steps-pos">{stepNumber}</span> OF {totalLabel}
      </span>
      <div className="wiz-pips" aria-hidden>
        {Array.from({ length: TOTAL_STEPS }).map((_, idx) => {
          const pipIndex = idx + 1;
          let pipState: "completed" | "active" | "pending";
          if (pipIndex <= completedSteps) {
            pipState = "completed";
          } else if (pipIndex === displayStep) {
            pipState = "active";
          } else {
            pipState = "pending";
          }
          const className =
            pipState === "completed"
              ? "wiz-pip wiz-pip-done"
              : pipState === "active"
                ? "wiz-pip wiz-pip-active"
                : "wiz-pip";
          return (
            <span
              key={idx}
              className={className}
              data-testid={`wiz-pip-${pipIndex}`}
              data-state={pipState}
              aria-current={pipState === "active" ? "step" : undefined}
            />
          );
        })}
      </div>
      <span className="wiz-steps-label" aria-hidden>{stepLabel}</span>
    </div>
  );
}

export function WizardEditorialShell({
  step,
  completedSteps,
  stepLabel,
  eyebrow,
  heroSegments,
  sub,
  children,
  unsavedHire = false,
  continueDisabled = false,
  onBack,
  onContinue,
  onDiscard,
  backLabel = "Back",
  continueLabel = "Continue",
  belowCta,
  hideFooter = false,
}: WizardEditorialShellProps) {
  // Default: prior steps are completed, the current step is "active" (not
  // yet done). UX-audit F9 (P1) — without this, all 5 pips lit on
  // arrival at step 5 and the progress strip read as decorative.
  const displayStep = step === 6 ? TOTAL_STEPS : step;
  const resolvedCompletedSteps = Math.max(
    0,
    Math.min(TOTAL_STEPS, completedSteps ?? displayStep - 1),
  );
  const router = useRouter();
  const pathname = usePathname();
  const [confirmOpen, setConfirmOpen] = useState(false);

  // F13: scroll resets on every onboarding-step route change. Without
  // this, a step that scrolled mid-form (e.g. workspace's autoFocus
  // scrollIntoView) leaves the next step landing mid-page with the
  // Audiowide hero behind the sticky bar.
  useEffect(() => {
    if (typeof window === "undefined") return;
    window.scrollTo(0, 0);
  }, [pathname]);

  function attemptLeave() {
    if (unsavedHire) {
      setConfirmOpen(true);
      return;
    }
    router.push("/");
  }

  function confirmDiscard() {
    onDiscard?.();
    router.push("/");
  }

  function handleBack() {
    if (onBack) {
      onBack();
      return;
    }
    router.back();
  }

  const continueDisabledFinal = continueDisabled || !onContinue;

  return (
    <>
      <style jsx global>{WIZARD_STYLES}</style>
      <div className="wiz-editorial">
        <DemoModeBanner />
        <header className="wiz-top">
          <button
            type="button"
            className="wiz-brand"
            onClick={attemptLeave}
            data-testid="wiz-shell-brand"
            aria-label="Return home"
          >
            <span className="wiz-brand-mark" aria-hidden />
            <span className="wiz-brand-name">SKYCOT</span>
          </button>
          <StepStrip
            step={step}
            stepLabel={stepLabel}
            completedSteps={resolvedCompletedSteps}
          />
          <div className="wiz-top-right">
            <Link href="/help" className="wiz-help-btn" data-testid="wiz-shell-help">
              Help
            </Link>
            <button
              type="button"
              className="wiz-exit"
              onClick={attemptLeave}
              data-testid="wiz-shell-exit"
            >
              SAVE &amp; EXIT
            </button>
          </div>
        </header>
        <main id="main" tabIndex={-1} className="wiz-stage">
          {eyebrow ? (
            <div className="wiz-eyebrow" data-testid="wiz-shell-eyebrow">
              <span className="wiz-eyebrow-dot" aria-hidden />
              {eyebrow}
            </div>
          ) : null}
          <h1 className="wiz-hero" data-testid="wiz-shell-hero">
            {heroSegments.map((seg, idx) => (
              <span key={idx} className={seg.accent ? "wiz-hero-accent" : undefined}>
                {seg.text}
              </span>
            ))}
          </h1>
          {sub ? <p className="wiz-sub">{sub}</p> : null}
          {children ? <div className="wiz-body">{children}</div> : null}
          {hideFooter ? null : (
            <div className="wiz-cta-row" data-testid="wiz-shell-cta-row">
              {onBack ? (
                <button
                  type="button"
                  className="wiz-cta wiz-cta-back"
                  onClick={handleBack}
                  data-testid="wiz-shell-back"
                >
                  {backLabel}
                </button>
              ) : null}
              <button
                type="button"
                className="wiz-cta"
                onClick={() => onContinue?.()}
                disabled={continueDisabledFinal}
                data-testid="wiz-shell-continue"
              >
                {continueLabel}
                <span className="wiz-cta-kbd" aria-hidden>↵</span>
              </button>
            </div>
          )}
          {belowCta ? <div className="wiz-below-cta">{belowCta}</div> : null}
        </main>
        <footer className="wiz-foot" data-testid="wiz-shell-trust">
          <span className="wiz-foot-item">
            <span className="wiz-foot-dot" aria-hidden>●</span>
            NO CREDIT CARD
          </span>
          <span className="wiz-foot-item">
            <span className="wiz-foot-dot" aria-hidden>●</span>
            SOC 2-READY AUDIT TRAIL
          </span>
          <span className="wiz-foot-item">
            <span className="wiz-foot-dot" aria-hidden>●</span>
            GDPR ARTICLE 30
          </span>
          <span className="wiz-foot-item">
            <strong>Founding 100</strong> · Early access cohort
          </span>
        </footer>
      </div>
      <AlertDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title="Discard your in-progress teammate?"
        description="Your draft will be cleared and you'll go back to the approvals inbox."
        confirmLabel="Discard and leave"
        cancelLabel="Keep working"
        onConfirm={confirmDiscard}
      />
    </>
  );
}
