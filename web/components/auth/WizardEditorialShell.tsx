"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState, type ReactNode } from "react";
import { AlertDialog } from "@/components/ui/AlertDialog";

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
.wiz-editorial {
  --purple: #8b5cf6; --purple-deep: #6d28d9;
  --pink: #ec4899; --coral: #ff6b6b; --yellow: #fcd34d;
  --teal: #14b8a6; --cyan: #0891b2;
  --ink: #1e1b4b; --ink-2: #3f3f46; --ink-3: #52525b; --ink-4: #a1a1aa;
  --paper-2: #f4f4f4; --bg: #fafaff;
  --line: #ece9f5; --line-2: #d1d1d1;
  --gradient: linear-gradient(135deg, var(--purple), var(--pink));
  --gradient-3: linear-gradient(135deg, var(--purple), var(--pink), var(--coral));
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
  font-family: "Audiowide", sans-serif;
  font-size: 16px; letter-spacing: 0.08em; color: var(--ink);
}
.wiz-steps {
  display: flex; align-items: center; gap: 10px;
  font-family: "JetBrains Mono", monospace;
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
.wiz-pip-on {
  background: var(--gradient);
  box-shadow: 0 0 8px rgba(139, 92, 246, 0.45);
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
  cursor: pointer; font-family: "JetBrains Mono", monospace;
  letter-spacing: 0.06em;
}
.wiz-exit:hover { color: var(--ink); }
.wiz-stage {
  max-width: 840px; margin: 0 auto;
  padding: 96px 48px 80px; text-align: center;
}
.wiz-eyebrow {
  display: inline-flex; align-items: center; gap: 6px;
  font-family: "JetBrains Mono", monospace;
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
  font-family: "Audiowide", "Inter", sans-serif;
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
  font-family: "JetBrains Mono", monospace;
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
  font-family: "JetBrains Mono", monospace;
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

function StepStrip({ step, stepLabel }: { step: WizardStep; stepLabel: string }) {
  const displayStep = step === 6 ? TOTAL_STEPS : step;
  const stepNumber = step === 6 ? "✓" : String(displayStep).padStart(2, "0");
  return (
    <div
      className="wiz-steps"
      role="progressbar"
      aria-valuemin={1}
      aria-valuemax={TOTAL_STEPS}
      aria-valuenow={displayStep}
      aria-label={`Hire wizard — step ${displayStep} of ${TOTAL_STEPS}`}
    >
      <span>STEP</span>
      <span className="wiz-steps-pos">{stepNumber}</span>
      <span>OF</span>
      <span>0{TOTAL_STEPS}</span>
      <div className="wiz-pips" aria-hidden>
        {Array.from({ length: TOTAL_STEPS }).map((_, idx) => {
          const filled = idx + 1 <= displayStep;
          return (
            <span
              key={idx}
              className={filled ? "wiz-pip wiz-pip-on" : "wiz-pip"}
              data-testid={`wiz-pip-${idx + 1}`}
              data-state={filled ? "on" : "off"}
            />
          );
        })}
      </div>
      <span className="wiz-steps-label">{stepLabel}</span>
    </div>
  );
}

export function WizardEditorialShell({
  step,
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
  const router = useRouter();
  const [confirmOpen, setConfirmOpen] = useState(false);

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
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{WIZARD_STYLES}</style>
      <div className="wiz-editorial">
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
          <StepStrip step={step} stepLabel={stepLabel} />
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
            <strong>2,847</strong>COMPANIES THIS MONTH
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
