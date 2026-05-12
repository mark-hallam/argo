"use client";

import { Fragment, type ReactNode } from "react";
import { useRedactionPolicy } from "@/lib/hooks/use-redaction-policy";
import {
  redactPII,
  POLICY_SHOW_ALL,
  type RedactionPolicy,
} from "@/lib/redaction/redact";
import { SENTINEL, type RedactionCategory } from "@/lib/redaction/patterns";

const SENTINEL_VALUES = Object.values(SENTINEL) as string[];
const CATEGORY_BY_SENTINEL: Record<string, RedactionCategory> = (
  Object.entries(SENTINEL) as [RedactionCategory, string][]
).reduce<Record<string, RedactionCategory>>((acc, [cat, sentinel]) => {
  acc[sentinel] = cat;
  return acc;
}, {});

/**
 * Regex that captures any sentinel string as a delimiter. Used with
 * `String.split` so the matched sentinels stay in the result array
 * and can be rendered as tooltipped spans while non-sentinel text
 * passes through as-is.
 */
const SENTINEL_SPLIT_RE = new RegExp(
  `(${SENTINEL_VALUES.map((s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})`,
  "g",
);

export const REDACTED_TOOLTIP_COPY =
  "Masked — your role doesn't include unredacted view";

export interface RedactedTextProps {
  /** Raw string. Wrapper redacts it according to the supplied policy. */
  children: string;
  /**
   * The policy to apply. When omitted the component resolves it via
   * `useRedactionPolicy()`. Supplying the prop lets callers (page-
   * level surfaces, tests) read the policy once at the top and
   * push it down — avoiding N hook calls and side-stepping the
   * SessionProvider requirement inside test harnesses.
   */
  policy?: RedactionPolicy;
  /** Optional className applied to the wrapping span. */
  className?: string;
}

/**
 * Wraps a plain-text node and substitutes PII sentinels per the
 * supplied (or resolved) `RedactionPolicy`. Each sentinel renders
 * as a `<span>` with a `title` tooltip (`REDACTED_TOOLTIP_COPY`)
 * and a `data-redacted` attribute so tests + auditors can target
 * masked spans without re-parsing prose.
 *
 * The wrapping `<span>` is inline so the parent's text styling
 * (font, leading, colour) carries through into the redacted
 * output.
 *
 * @example
 *   <RedactedText policy={policy}>{approval.detail}</RedactedText>
 */
export function RedactedText(props: RedactedTextProps) {
  if (props.policy) {
    return <StaticRedactedText {...props} policy={props.policy} />;
  }
  return <DynamicRedactedText {...props} />;
}

function DynamicRedactedText(props: Omit<RedactedTextProps, "policy">) {
  const policy = useRedactionPolicy();
  return <StaticRedactedText {...props} policy={policy} />;
}

function StaticRedactedText({
  children,
  policy,
  className,
}: RedactedTextProps & { policy: RedactionPolicy }) {
  const { redacted } = redactPII(children, policy);
  const parts = redacted.split(SENTINEL_SPLIT_RE);

  return (
    <span className={className} data-testid="redacted-text">
      {parts.map((part, idx) => renderPart(part, idx))}
    </span>
  );
}

function renderPart(part: string, idx: number): ReactNode {
  const category = CATEGORY_BY_SENTINEL[part];
  if (category) {
    return (
      <span
        key={idx}
        data-redacted={category}
        title={REDACTED_TOOLTIP_COPY}
        className="rounded-sm bg-[var(--muted)] px-1 py-[1px] font-mono text-[0.85em] text-[var(--muted-foreground)]"
      >
        {part}
      </span>
    );
  }
  return <Fragment key={idx}>{part}</Fragment>;
}

/**
 * Inline (no-wrapper) form for callers that already control the
 * parent element. Returns a `ReactNode` instead of a span.
 *
 * @example
 *   <p>{redactInline(approval.detail, policy)}</p>
 */
export function redactInline(
  text: string,
  policy: RedactionPolicy,
): ReactNode {
  const { redacted } = redactPII(text, policy);
  return redacted.split(SENTINEL_SPLIT_RE).map((part, idx) => renderPart(part, idx));
}

/** Re-export for callers that want the "no redaction" default explicitly. */
export { POLICY_SHOW_ALL };
