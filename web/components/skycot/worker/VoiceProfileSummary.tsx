"use client";

import { Pencil } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import type { VoiceProfile, VoiceTone } from "@/lib/api/worker-memory";

/**
 * Voice profile summary — tone, formality, and a short list of
 * signature phrases the worker leans on. The `[✎ Adjust]` button is
 * a placeholder until E4-g lands the retrain dialog; the parent
 * page wires it to a toast announcing the deferred surface.
 */

export interface VoiceProfileSummaryProps {
  voice: VoiceProfile;
  onAdjust: () => void;
  className?: string;
}

const TONE_LABEL: Record<VoiceTone, string> = {
  warm: "Warm",
  neutral: "Neutral",
  formal: "Formal",
};

function formalityLabel(value: number): string {
  if (value < 25) return "casual";
  if (value < 50) return "conversational";
  if (value < 75) return "professional";
  return "formal";
}

export function VoiceProfileSummary({
  voice,
  onAdjust,
  className,
}: VoiceProfileSummaryProps) {
  return (
    <Card data-testid="card-voice-profile" className={className}>
      <CardHeader className="flex flex-row items-start justify-between gap-2">
        <CardTitle>Voice profile</CardTitle>
        <Button
          variant="ghost"
          size="sm"
          onClick={onAdjust}
          data-testid="voice-adjust"
          aria-label="Adjust voice profile"
        >
          <Pencil className="size-3.5" />
          Adjust
        </Button>
      </CardHeader>
      <CardBody className="pt-1 space-y-3">
        <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 text-sm">
          <dt className="text-[var(--muted-foreground)]">Tone</dt>
          <dd data-testid="voice-tone" className="text-[var(--foreground)]">
            {TONE_LABEL[voice.tone]}
          </dd>
          <dt className="text-[var(--muted-foreground)]">Formality</dt>
          <dd data-testid="voice-formality" className="text-[var(--foreground)]">
            {voice.formality} / 100 ({formalityLabel(voice.formality)})
          </dd>
        </dl>

        <div className="space-y-1">
          <p className="text-xs font-medium text-[var(--muted-foreground)]">
            Signature phrases
          </p>
          {voice.signaturePhrases.length === 0 ? (
            <p
              data-testid="voice-phrases-empty"
              className="text-sm text-[var(--muted-foreground)]"
            >
              No signature phrases yet — she&apos;ll pick them up as she works.
            </p>
          ) : (
            <ul
              data-testid="voice-phrases"
              role="list"
              className="flex flex-wrap gap-1.5"
            >
              {voice.signaturePhrases.map((phrase) => (
                <li
                  key={phrase}
                  className="rounded-full border border-[var(--border)] bg-[var(--muted)]/40 px-2 py-0.5 text-xs text-[var(--foreground)]"
                >
                  &ldquo;{phrase}&rdquo;
                </li>
              ))}
            </ul>
          )}
        </div>
      </CardBody>
    </Card>
  );
}

export function VoiceProfileSummarySkeleton() {
  return (
    <Card data-testid="card-voice-profile-skeleton">
      <CardHeader>
        <div className="h-4 w-32 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-2">
        <div className="h-3 w-40 rounded bg-[var(--muted)]" />
        <div className="h-3 w-48 rounded bg-[var(--muted)]" />
        <div className="flex gap-1.5">
          <div className="h-5 w-24 rounded-full bg-[var(--muted)]" />
          <div className="h-5 w-32 rounded-full bg-[var(--muted)]" />
        </div>
      </CardBody>
    </Card>
  );
}
