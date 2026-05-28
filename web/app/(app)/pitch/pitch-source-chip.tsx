"use client";

import { Sparkles, Zap } from "lucide-react";
import type { PitchAskResponse, PitchSource } from "@/lib/api/pitch";

/**
 * Helpers extracted from `page.tsx` so the page module stays under
 * the LOC budget. Pure rendering — no state, no fetching. The
 * stylesheet that drives the `pitch-pill is-source-*` classes lives
 * inline in `page.tsx`.
 */

export function sourcePill(source: PitchSource | undefined) {
  if (!source || source === "canonical") {
    return { Icon: Zap, label: "canonical", className: "pitch-pill is-source-canonical" };
  }
  if (source === "llm_blocked_canonical_refused") {
    return {
      Icon: Sparkles,
      label: "llm → blocked",
      className: "pitch-pill is-source-llm-blocked",
    };
  }
  return { Icon: Sparkles, label: "llm", className: "pitch-pill is-source-llm" };
}

export function tokensLabel(tokens: PitchAskResponse["tokens"]): string | null {
  if (!tokens) return null;
  const input = tokens.input ?? null;
  const output = tokens.output ?? null;
  if (input === null && output === null) return null;
  const parts: string[] = [];
  if (input !== null) parts.push(`in ${input}`);
  if (output !== null) parts.push(`out ${output}`);
  return parts.join(" · ");
}
