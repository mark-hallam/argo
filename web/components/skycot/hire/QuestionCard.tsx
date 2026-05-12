"use client";

import { useId } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import type { Question, QuestionAnswer } from "@/lib/hire/genesis-types";

/**
 * Renders a single Genesis-lite clarifying question with the answer
 * surface dictated by `question.answer_type`:
 *
 *   single-select → radio group
 *   multi-select  → checkbox group
 *   free-text     → textarea (3-row min)
 *   yes-no        → 2-button toggle
 *
 * The component is fully controlled — the parent owns the `answer`
 * value and reacts to `onChange`. The numbered heading comes from the
 * `index` prop so the parent decides display order without the card
 * having to know its position.
 */
export interface QuestionCardProps {
  question: Question;
  /** 1-based card position; renders as "1. <text>". */
  index: number;
  answer: QuestionAnswer | undefined;
  onChange: (next: QuestionAnswer) => void;
}

const TEXTAREA_CLASS = [
  "w-full resize-y rounded-[var(--radius-md)]",
  "border border-[var(--border)] bg-[var(--background)]",
  "px-3 py-2 text-sm text-[var(--foreground)]",
  "placeholder:text-[var(--muted-foreground)]",
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
].join(" ");

export function QuestionCard({ question, index, answer, onChange }: QuestionCardProps) {
  const groupId = useId();

  return (
    <Card data-testid={`hire-step2-question-${question.id}`}>
      <CardHeader>
        <CardTitle className="text-base text-[var(--foreground)]">
          <span className="mr-2 text-[var(--muted-foreground)]">{index}.</span>
          {question.text}
        </CardTitle>
      </CardHeader>
      <CardBody>
        {question.answer_type === "single-select" ? (
          <SingleSelectAnswer
            question={question}
            answer={answer}
            groupId={groupId}
            onChange={onChange}
          />
        ) : null}
        {question.answer_type === "multi-select" ? (
          <MultiSelectAnswer question={question} answer={answer} onChange={onChange} />
        ) : null}
        {question.answer_type === "free-text" ? (
          <textarea
            data-testid={`hire-step2-answer-${question.id}`}
            rows={3}
            className={TEXTAREA_CLASS}
            value={answer?.type === "free-text" ? answer.value : ""}
            onChange={(event) =>
              onChange({
                question_id: question.id,
                type: "free-text",
                value: event.target.value,
              })
            }
          />
        ) : null}
        {question.answer_type === "yes-no" ? (
          <YesNoAnswer question={question} answer={answer} onChange={onChange} />
        ) : null}
      </CardBody>
    </Card>
  );
}

interface VariantProps {
  question: Question;
  answer: QuestionAnswer | undefined;
  onChange: (next: QuestionAnswer) => void;
}

function SingleSelectAnswer({
  question,
  answer,
  groupId,
  onChange,
}: VariantProps & { groupId: string }) {
  const value = answer?.type === "single-select" ? answer.value : undefined;
  const options = question.options ?? [];
  return (
    <div role="radiogroup" aria-label={question.text} className="space-y-2">
      {options.map((option) => {
        const id = `${groupId}-${option}`;
        const checked = value === option;
        return (
          <label
            key={option}
            htmlFor={id}
            className={cn(
              "flex items-center gap-2 rounded-[var(--radius-md)] border px-3 py-2 cursor-pointer",
              "border-[var(--border)] bg-[var(--background)] text-sm text-[var(--foreground)]",
              "hover:bg-[var(--accent)] hover:text-[var(--accent-foreground)]",
              checked && "border-[var(--ring)] bg-[var(--accent)] text-[var(--accent-foreground)]",
            )}
          >
            <input
              id={id}
              type="radio"
              name={groupId}
              value={option}
              checked={checked}
              data-testid={`hire-step2-answer-${question.id}-${option}`}
              onChange={() =>
                onChange({
                  question_id: question.id,
                  type: "single-select",
                  value: option,
                })
              }
              className="size-4 accent-[var(--primary)]"
            />
            <span>{option}</span>
          </label>
        );
      })}
    </div>
  );
}

function MultiSelectAnswer({ question, answer, onChange }: VariantProps) {
  const selected = new Set(answer?.type === "multi-select" ? answer.values : []);
  const options = question.options ?? [];
  return (
    <div role="group" aria-label={question.text} className="space-y-2">
      {options.map((option) => {
        const checked = selected.has(option);
        return (
          <label
            key={option}
            className={cn(
              "flex items-center gap-2 rounded-[var(--radius-md)] border px-3 py-2 cursor-pointer",
              "border-[var(--border)] bg-[var(--background)] text-sm text-[var(--foreground)]",
              "hover:bg-[var(--accent)] hover:text-[var(--accent-foreground)]",
              checked && "border-[var(--ring)] bg-[var(--accent)] text-[var(--accent-foreground)]",
            )}
          >
            <input
              type="checkbox"
              value={option}
              checked={checked}
              data-testid={`hire-step2-answer-${question.id}-${option}`}
              onChange={(event) => {
                const next = new Set(selected);
                if (event.target.checked) next.add(option);
                else next.delete(option);
                onChange({
                  question_id: question.id,
                  type: "multi-select",
                  values: Array.from(next),
                });
              }}
              className="size-4 accent-[var(--primary)]"
            />
            <span>{option}</span>
          </label>
        );
      })}
    </div>
  );
}

function YesNoAnswer({ question, answer, onChange }: VariantProps) {
  const value = answer?.type === "yes-no" ? answer.value : undefined;
  return (
    <div role="group" aria-label={question.text} className="flex gap-2">
      <Button
        type="button"
        variant={value === true ? "primary" : "secondary"}
        data-testid={`hire-step2-answer-${question.id}-yes`}
        onClick={() => onChange({ question_id: question.id, type: "yes-no", value: true })}
      >
        Yes
      </Button>
      <Button
        type="button"
        variant={value === false ? "primary" : "secondary"}
        data-testid={`hire-step2-answer-${question.id}-no`}
        onClick={() => onChange({ question_id: question.id, type: "yes-no", value: false })}
      >
        No
      </Button>
    </div>
  );
}
