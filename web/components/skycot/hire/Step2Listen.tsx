"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { QuestionCard } from "@/components/skycot/hire/QuestionCard";
import { EXAMPLE_ROLES, type ExampleRole } from "@/lib/hire/examples";
import { STUB_BANNER_MESSAGE } from "@/lib/hire/genesis-stub";
import {
  isAnswerFilled,
  type Question,
  type QuestionAnswer,
} from "@/lib/hire/genesis-types";
import { useGenesisQuestions } from "@/lib/hire/use-genesis-questions";

/**
 * Hire flow — Step 2 (Listen).
 *
 * Fires the Genesis-lite classifier against the user's Step 1
 * problem statement and renders 1–3 clarifying question cards.
 * Loading state is a skeleton; fallback (stub) renders a banner;
 * post-retry error renders a skip-to-template panel populated from
 * the same `EXAMPLE_ROLES` set as Step 1.
 *
 * The view is mostly stateless — `answers` is hoisted locally and
 * the parent receives every change via `onChange` so the owning page
 * can wire HireShell's Continue button (`continueDisabled` +
 * `onContinue`) without lifting the genesis hook itself.
 *
 * PII echo protection: the user's raw Step 1 input is intentionally
 * not rendered back to them here — only the derived questions are.
 */
export interface Step2ListenProps {
  /** The Step 1 problem statement; drives the Genesis call. */
  problem: string;
  /**
   * Pre-existing answers from the draft store (e.g. user navigated
   * back to Step 2 from Step 3). Keyed by `question_id`.
   */
  initialAnswers?: Record<string, QuestionAnswer>;
  /**
   * Fires on every answer change. The parent should snapshot
   * `answers` so it can persist on Continue, and read `canContinue`
   * to enable/disable HireShell's Continue button.
   */
  onChange: (snapshot: {
    answers: Record<string, QuestionAnswer>;
    canContinue: boolean;
    source: "gateway" | "stub" | undefined;
  }) => void;
  /**
   * Called when the user picks a fallback-panel example tile after
   * Genesis fails 3×. The parent should update the draft's `problem`
   * to the picked example's description and reload step 2 (the
   * Genesis hook then re-fires with the new input).
   */
  onSkipToTemplate: (example: ExampleRole) => void;
}

function computeCanContinue(
  questions: Question[],
  answers: Record<string, QuestionAnswer>,
): boolean {
  if (questions.length === 0) return false;
  return questions.every((q) => {
    if (q.required === false) return true;
    return isAnswerFilled(answers[q.id]);
  });
}

export function Step2Listen({
  problem,
  initialAnswers,
  onChange,
  onSkipToTemplate,
}: Step2ListenProps) {
  const { questions, status, source } = useGenesisQuestions(problem);
  const [answers, setAnswers] = useState<Record<string, QuestionAnswer>>(
    () => initialAnswers ?? {},
  );

  // Notify the parent on every relevant change. Stash the callback in
  // a ref so identity churn on the parent's closure doesn't fire spurious
  // notifications, and only re-emit when the snapshot's content changes.
  const onChangeRef = useRef(onChange);
  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);
  const canContinue = computeCanContinue(questions, answers);
  useEffect(() => {
    onChangeRef.current({ answers, canContinue, source });
  }, [answers, canContinue, source]);

  const handleAnswerChange = (next: QuestionAnswer) => {
    setAnswers((prev) => ({ ...prev, [next.question_id]: next }));
  };

  const examples = useMemo(() => EXAMPLE_ROLES, []);

  return (
    <section
      className="mx-auto max-w-2xl space-y-6"
      data-testid="hire-step2-listen"
    >
      <header className="space-y-2">
        <h1 className="text-3xl font-semibold tracking-tight text-[var(--foreground)]">
          A few quick questions
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          So I can shape this teammate around your setup.
        </p>
      </header>

      {status === "loading" ? <LoadingSkeleton /> : null}

      {status === "error" ? (
        <SkipToTemplatePanel examples={examples} onPick={onSkipToTemplate} />
      ) : null}

      {(status === "ready" || status === "fallback") && questions.length > 0 ? (
        <div className="space-y-3" data-testid="hire-step2-questions">
          {status === "fallback" ? (
            <p
              role="status"
              data-testid="hire-step2-fallback-banner"
              className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--accent)] px-3 py-2 text-xs text-[var(--accent-foreground)]"
            >
              {STUB_BANNER_MESSAGE}
            </p>
          ) : null}
          {questions.map((question, idx) => (
            <QuestionCard
              key={question.id}
              question={question}
              index={idx + 1}
              answer={answers[question.id]}
              onChange={handleAnswerChange}
            />
          ))}
        </div>
      ) : null}
    </section>
  );
}

function LoadingSkeleton() {
  return (
    <div className="space-y-3" data-testid="hire-step2-loading">
      {[0, 1, 2].map((i) => (
        <Card key={i} className="animate-pulse" aria-hidden>
          <CardHeader>
            <div className="h-4 w-2/3 rounded bg-[var(--muted)]" />
          </CardHeader>
          <CardBody>
            <div className="space-y-2">
              <div className="h-10 rounded bg-[var(--muted)]" />
              <div className="h-10 rounded bg-[var(--muted)]" />
            </div>
          </CardBody>
        </Card>
      ))}
      <p className="sr-only" role="status">
        Generating clarifying questions…
      </p>
    </div>
  );
}

interface SkipToTemplatePanelProps {
  examples: readonly ExampleRole[];
  onPick: (example: ExampleRole) => void;
}

function SkipToTemplatePanel({ examples, onPick }: SkipToTemplatePanelProps) {
  return (
    <Card data-testid="hire-step2-skip-to-template">
      <CardHeader>
        <CardTitle>
          Couldn&apos;t figure out the right questions for your setup. Want to
          start with a similar template?
        </CardTitle>
      </CardHeader>
      <CardBody>
        <div className="grid gap-2 sm:grid-cols-2">
          {examples.map((example) => (
            <Button
              key={example.title}
              type="button"
              variant="secondary"
              className="h-auto justify-start whitespace-normal px-3 py-2 text-left"
              data-testid={`hire-step2-template-${slugify(example.title)}`}
              onClick={() => onPick(example)}
            >
              <span className="block w-full">
                <span className="block text-sm font-medium">{example.title}</span>
                <span className="mt-1 block text-xs text-[var(--muted-foreground)]">
                  {example.description}
                </span>
              </span>
            </Button>
          ))}
        </div>
      </CardBody>
    </Card>
  );
}

function slugify(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
}
