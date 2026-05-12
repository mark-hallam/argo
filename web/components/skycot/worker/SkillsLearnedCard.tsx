"use client";

import { useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { SkillRow } from "@/components/skycot/worker/SkillRow";
import type { LearnedPattern } from "@/lib/api/worker-skills";

/**
 * Learned-pattern section — novel patterns the worker has used
 * successfully enough that they're candidates for promotion to the
 * Library. Promote opens a 3-output preview dialog; Discard hides
 * the pattern (operator-view can resurface). Both actions confirm
 * before mutating.
 */

export interface SkillsLearnedCardProps {
  workerName: string;
  patterns: LearnedPattern[];
  onPromote: (patternId: string) => void;
  onDiscard: (patternId: string) => void;
}

export function SkillsLearnedCard({
  workerName,
  patterns,
  onPromote,
  onDiscard,
}: SkillsLearnedCardProps) {
  const [pendingPromote, setPendingPromote] = useState<LearnedPattern | null>(null);
  const [pendingDiscard, setPendingDiscard] = useState<LearnedPattern | null>(null);

  return (
    <>
      <Card data-testid="card-skills-learned">
        <CardHeader>
          <CardTitle>Learned</CardTitle>
        </CardHeader>
        <CardBody className="pt-1">
          {patterns.length === 0 ? (
            <p
              data-testid="skills-learned-empty"
              className="text-sm text-[var(--muted-foreground)]"
            >
              Nothing new yet. {workerName} will suggest patterns here
              once she&apos;s used them successfully a few times.
            </p>
          ) : (
            <ul role="list" className="divide-y divide-[var(--border)]/60">
              {patterns.map((pattern) => (
                <SkillRow
                  key={pattern.id}
                  id={pattern.id}
                  title={pattern.label}
                  meta={
                    <span
                      data-testid={`learned-counts-${pattern.id}`}
                      className="text-xs text-[var(--muted-foreground)]"
                    >
                      {pattern.successCount} successes · {pattern.failCount} fails
                    </span>
                  }
                  actions={
                    <>
                      <Button
                        variant="secondary"
                        size="sm"
                        data-testid={`learned-promote-${pattern.id}`}
                        onClick={() => setPendingPromote(pattern)}
                      >
                        Promote
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        data-testid={`learned-discard-${pattern.id}`}
                        onClick={() => setPendingDiscard(pattern)}
                      >
                        Discard
                      </Button>
                    </>
                  }
                />
              ))}
            </ul>
          )}
        </CardBody>
      </Card>

      <PromoteDialog
        pattern={pendingPromote}
        onClose={() => setPendingPromote(null)}
        onConfirm={(id) => {
          onPromote(id);
          setPendingPromote(null);
        }}
      />

      <AlertDialog
        open={pendingDiscard !== null}
        onOpenChange={(open) => {
          if (!open) setPendingDiscard(null);
        }}
        title="Discard this pattern?"
        description={
          pendingDiscard
            ? `${workerName} will stop suggesting "${pendingDiscard.label}". You can re-surface it later from the operator view.`
            : ""
        }
        confirmLabel="Discard"
        cancelLabel="Keep"
        confirmVariant="destructive"
        onConfirm={() => {
          if (pendingDiscard) onDiscard(pendingDiscard.id);
          setPendingDiscard(null);
        }}
        onCancel={() => setPendingDiscard(null)}
      />
    </>
  );
}

function PromoteDialog({
  pattern,
  onClose,
  onConfirm,
}: {
  pattern: LearnedPattern | null;
  onClose: () => void;
  onConfirm: (id: string) => void;
}) {
  return (
    <AlertDialog
      open={pattern !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      title="Promote this pattern?"
      description={
        pattern ? (
          <span data-testid="promote-preview" className="block">
            <span className="block">
              Pattern: <strong>{pattern.label}</strong>
            </span>
            <span className="mt-2 block">3 example outputs:</span>
            {pattern.sampleOutputs.map((sample, i) => (
              <span
                key={i}
                data-testid={`promote-sample-${i}`}
                className="mt-1 block text-xs text-[var(--muted-foreground)]"
              >
                {i + 1}. {sample}
              </span>
            ))}
          </span>
        ) : (
          ""
        )
      }
      confirmLabel="Promote"
      cancelLabel="Cancel"
      confirmVariant="primary"
      onConfirm={() => {
        if (pattern) onConfirm(pattern.id);
      }}
      onCancel={onClose}
    />
  );
}

export function SkillsLearnedCardSkeleton() {
  return (
    <Card data-testid="card-skills-learned-skeleton">
      <CardHeader>
        <div className="h-4 w-20 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-3">
        {[0, 1].map((i) => (
          <div key={i} className="flex items-center gap-3">
            <div className="size-4 rounded bg-[var(--muted)]" />
            <div className="flex-1 h-3 rounded bg-[var(--muted)]" />
            <div className="h-8 w-20 rounded bg-[var(--muted)]" />
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
