"use client";

import { useId } from "react";
import type { WorkerVoiceProfile } from "@/lib/hire/draft-types";
import { cn } from "@/lib/utils";

/**
 * Voice section of the structured Adjust drawer.
 *
 * Sub-sections:
 *  - Tone: 3-button toggle group (warm / neutral / formal).
 *  - Formality: slider 0–100 with low/high labels. The slider value
 *    maps to the underlying `WorkerVoiceProfile["formality"]` enum
 *    in 3 buckets — see `bucketFormality()` for thresholds.
 *  - Signature phrases: multi-line textarea, one phrase per line.
 *    Filtered against the v1 wordlist on save (handled by the
 *    parent drawer, not here).
 */
export interface VoiceProfileEditorProps {
  voice: WorkerVoiceProfile;
  /** Free-text textarea content for signature phrases. */
  phrasesText: string;
  onVoiceChange: (next: WorkerVoiceProfile) => void;
  onPhrasesTextChange: (next: string) => void;
}

const TONES: ReadonlyArray<{ id: WorkerVoiceProfile["tone"]; label: string }> = [
  { id: "warm", label: "Warm" },
  { id: "neutral", label: "Neutral" },
  { id: "formal", label: "Formal" },
];

/**
 * Map a 0–100 slider value to the discrete `formality` enum. The
 * thresholds are linear thirds: 0–33 → low, 34–66 → medium, 67–100 →
 * high.
 */
export function bucketFormality(
  value: number,
): WorkerVoiceProfile["formality"] {
  if (value <= 33) return "low";
  if (value <= 66) return "medium";
  return "high";
}

/** Inverse mapping so the slider starts at the midpoint of each bucket. */
export function formalityToSlider(
  formality: WorkerVoiceProfile["formality"],
): number {
  if (formality === "low") return 17;
  if (formality === "high") return 83;
  return 50;
}

export function VoiceProfileEditor({
  voice,
  phrasesText,
  onVoiceChange,
  onPhrasesTextChange,
}: VoiceProfileEditorProps) {
  const sliderId = useId();
  const phrasesId = useId();
  const sliderValue = formalityToSlider(voice.formality);

  const setTone = (tone: WorkerVoiceProfile["tone"]) =>
    onVoiceChange({ ...voice, tone });

  const setSlider = (next: number) => {
    const formality = bucketFormality(next);
    if (formality === voice.formality) return;
    onVoiceChange({ ...voice, formality });
  };

  return (
    <section
      data-testid="hire-step4-voice-editor"
      className="space-y-4"
      aria-label="Voice profile"
    >
      <div className="space-y-2">
        <label className="block text-xs font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">
          Tone
        </label>
        <div
          role="radiogroup"
          aria-label="Voice tone"
          data-testid="hire-step4-voice-tone"
          className="inline-flex rounded-[var(--radius-md)] border border-[var(--border)] p-0.5"
        >
          {TONES.map((tone) => {
            const selected = tone.id === voice.tone;
            return (
              <button
                key={tone.id}
                type="button"
                role="radio"
                aria-checked={selected}
                data-testid={`hire-step4-voice-tone-${tone.id}`}
                onClick={() => setTone(tone.id)}
                className={cn(
                  "rounded-[var(--radius-sm)] px-3 py-1 text-sm",
                  selected
                    ? "bg-[var(--primary)] text-[var(--primary-foreground)]"
                    : "bg-transparent text-[var(--muted-foreground)] hover:bg-[var(--accent)]",
                )}
              >
                {tone.label}
              </button>
            );
          })}
        </div>
      </div>

      <div className="space-y-2">
        <label
          htmlFor={sliderId}
          className="block text-xs font-semibold uppercase tracking-wide text-[var(--muted-foreground)]"
        >
          Formality
        </label>
        <div className="flex items-center gap-2">
          <span className="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">
            low
          </span>
          <input
            id={sliderId}
            type="range"
            min={0}
            max={100}
            step={1}
            value={sliderValue}
            data-testid="hire-step4-voice-formality"
            onChange={(e) => setSlider(Number.parseInt(e.target.value, 10))}
            aria-valuetext={voice.formality}
            className="h-2 flex-1 accent-[var(--primary)]"
          />
          <span className="text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">
            high
          </span>
        </div>
      </div>

      <div className="space-y-2">
        <label
          htmlFor={phrasesId}
          className="block text-xs font-semibold uppercase tracking-wide text-[var(--muted-foreground)]"
        >
          Signature phrases
        </label>
        <textarea
          id={phrasesId}
          data-testid="hire-step4-voice-phrases"
          rows={4}
          value={phrasesText}
          onChange={(e) => onPhrasesTextChange(e.target.value)}
          placeholder={"One phrase per line\nE.g. Cheers!\nThanks heaps."}
          aria-label="Signature phrases (one per line)"
          className={cn(
            "block w-full resize-none rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] p-2 text-sm",
            "outline-none focus:ring-2 focus:ring-[var(--ring)]",
          )}
        />
        <p className="text-xs text-[var(--muted-foreground)]">
          Each line becomes a phrase your teammate uses. Phrases under 3
          characters or matching the blocklist are removed on save.
        </p>
      </div>
    </section>
  );
}
