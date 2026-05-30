"use client";

/**
 * Chat-bubble rendering for the `/pitch` rehearsal surface.
 *
 * Extracted from `page.tsx` to keep that file lean: the page owns input +
 * state, this module owns how an operator question and an agent answer
 * (typed OR spoken) render, including the truth-boundary disposition +
 * source pills. Spoken answers reuse the exact same bubble — the page maps
 * a voice `PitchTurnResponse` into the `PitchAskResponse`-shaped `meta`
 * via `voiceMeta` below.
 */

import { ShieldAlert, ShieldCheck, ShieldQuestion } from "lucide-react";
import type { PitchAskResponse, PitchDisposition } from "@/lib/api/pitch";
import type { PitchTurnResponse } from "@/lib/api/voice";
import { sourcePill, tokensLabel } from "./pitch-source-chip";

export interface OperatorMessage {
  id: string;
  role: "operator";
  text: string;
}

export interface AgentMessage {
  id: string;
  role: "agent";
  text: string;
  meta?: PitchAskResponse;
}

export type Message = OperatorMessage | AgentMessage;

/**
 * Reshape a spoken-turn response into the `PitchAskResponse` the agent
 * bubble renders, so a spoken answer shows the same disposition + source
 * pills as a typed one. The truth boundary is computed server-side; this
 * only maps field names.
 */
export function voiceMeta(turn: PitchTurnResponse): PitchAskResponse {
  const tb = turn.truth_boundary;
  const disposition = (tb?.disposition as PitchDisposition) ?? "Allow";
  return {
    matched: tb?.source === "canonical",
    source: (tb?.source as PitchAskResponse["source"]) ?? undefined,
    answer: turn.answer_text,
    disposition,
    block_reason: tb?.block_reason ?? null,
    channel: "WebEmbed",
    latency_ms: 0,
  };
}

function dispositionPill(disposition: PitchDisposition) {
  if (disposition === "Block") {
    return { Icon: ShieldAlert, label: "Blocked", className: "pitch-pill is-block" };
  }
  if (disposition === "Escalate") {
    return { Icon: ShieldQuestion, label: "Escalated", className: "pitch-pill is-escalate" };
  }
  return { Icon: ShieldCheck, label: "Allowed", className: "pitch-pill is-allow" };
}

export function MessageBubble({ message }: { message: Message }) {
  if (message.role === "operator") {
    return (
      <div className="pitch-msg is-operator" data-testid="pitch-msg-operator">
        <p className="pitch-msg-text">{message.text}</p>
      </div>
    );
  }
  const meta = message.meta;
  const pill = meta ? dispositionPill(meta.disposition) : null;
  const source = meta ? sourcePill(meta.source) : null;
  const tokens = meta ? tokensLabel(meta.tokens) : null;
  const isSwapped = meta?.source === "llm_blocked_canonical_refused";
  return (
    <div className="pitch-msg is-agent" data-testid="pitch-msg-agent">
      <p className="pitch-msg-text">{message.text}</p>
      {isSwapped ? (
        <p className="pitch-swap-note" data-testid="pitch-swap-note">
          (canonical refusal substituted for the model&apos;s draft)
        </p>
      ) : null}
      {meta ? (
        <div className="pitch-meta">
          {pill ? (
            <span className={pill.className} data-testid="pitch-pill-disposition">
              <pill.Icon /> {pill.label}
            </span>
          ) : null}
          {source ? (
            <span
              className={source.className}
              data-testid="pitch-pill-source"
              title={meta?.model ? `${source.label} · ${meta.model}` : source.label}
            >
              <source.Icon /> {source.label}
            </span>
          ) : null}
          {meta.entry_id ? (
            <span className="pitch-pill is-entry" data-testid="pitch-pill-entry">
              {meta.entry_id}
            </span>
          ) : null}
          {meta.model ? (
            <span className="pitch-pill is-model" data-testid="pitch-pill-model">
              {meta.model}
            </span>
          ) : null}
          {tokens ? (
            <span className="pitch-pill is-tokens" data-testid="pitch-pill-tokens">
              {tokens}
            </span>
          ) : null}
          <span className="pitch-pill is-latency" data-testid="pitch-pill-latency">
            {meta.latency_ms}ms · {meta.channel}
          </span>
        </div>
      ) : null}
      {meta?.draft && isSwapped ? (
        <div className="pitch-draft-block" data-testid="pitch-draft">
          <span className="pitch-draft-label">Model draft (audited, not shipped)</span>
          {meta.draft}
        </div>
      ) : null}
    </div>
  );
}
