"use client";

import { Fragment, useMemo, useState } from "react";
import Link from "next/link";
import { ArrowRight, ImageIcon, X } from "lucide-react";

import { RedactedText } from "@/components/skycot/RedactedText";
import { CrossSystemLink } from "@/components/skycot/audit/CrossSystemLink";
import { detect, segmentText } from "@/lib/cross-system/detect";
import { getGenerator } from "@/lib/cross-system/registry";
import type { CrossSystemMatch } from "@/lib/cross-system/types";
import type { AuditOutput, ScreenshotFrame } from "@/lib/types/audit-detail";
import type { RedactionPolicy } from "@/lib/redaction/redact";

/**
 * Output card — the final artifact the action produced. Rendering
 * varies by `output.kind`:
 *
 *  - `text`: rendered as `<pre>` with inline cross-system references
 *    wrapped via `<CrossSystemLink>` (E5-h).
 *  - `json`: rendered as collapsed JSON with `<details>` per level.
 *    ≤ 4 levels deep auto-expanded.
 *  - `computer-use`: text body shows the action log inline-linked;
 *    screenshot sequence rendered as thumbnails below.
 *
 * Cross-system detection covers Stripe / Slack / GitHub / Linear /
 * Salesforce / HubSpot / Zendesk / Jira in v1 — see
 * `lib/cross-system/registry.ts`.
 */

const AUTO_EXPAND_DEPTH = 4;

export interface ActionOutputCardProps {
  output: AuditOutput;
  policy: RedactionPolicy;
}

export function ActionOutputCard({ output, policy }: ActionOutputCardProps) {
  return (
    <section
      data-testid="audit-output-card"
      data-output-kind={output.kind}
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 space-y-3"
    >
      <header className="flex items-center justify-between">
        <h2 className="text-sm font-semibold text-[var(--foreground)]">Output</h2>
        <span className="text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
          {KIND_LABEL[output.kind]}
        </span>
      </header>

      {output.kind === "text" ? <TextBody body={String(output.body)} policy={policy} /> : null}
      {output.kind === "json" ? <JsonBody body={output.body} /> : null}
      {output.kind === "computer-use" ? (
        <ComputerUseBody output={output} policy={policy} />
      ) : null}

      {output.delivery ? (
        <div
          data-testid="audit-output-delivery"
          className="grid grid-cols-[auto_1fr] items-baseline gap-x-3 gap-y-1 border-t border-[var(--border)] pt-3 text-xs text-[var(--muted-foreground)]"
        >
          <span>Sent</span>
          <span className="font-mono text-[var(--foreground)]">{output.delivery.sentAt}</span>
          {output.delivery.receivedAt ? (
            <>
              <span>Received</span>
              <span className="font-mono text-[var(--foreground)]">{output.delivery.receivedAt}</span>
            </>
          ) : null}
          {output.delivery.downstreamId ? (
            <>
              <span>Downstream id</span>
              <span className="font-mono text-[var(--foreground)]">{output.delivery.downstreamId}</span>
            </>
          ) : null}
        </div>
      ) : null}

      {output.followUp ? (
        <div className="border-t border-[var(--border)] pt-3">
          <p className="mb-1 text-[11px] uppercase tracking-wider text-[var(--muted-foreground)]">
            Follow-up
          </p>
          <Link
            href={`/audit/action/${encodeURIComponent(output.followUp.id)}`}
            data-testid="audit-output-followup"
            data-followup-id={output.followUp.id}
            className="inline-flex items-center gap-2 text-sm text-[var(--primary)] underline-offset-4 hover:underline"
          >
            <span>
              <RedactedText policy={policy}>{output.followUp.verb}</RedactedText>
            </span>
            <ArrowRight aria-hidden className="size-4" />
          </Link>
        </div>
      ) : null}
    </section>
  );
}

const KIND_LABEL: Record<AuditOutput["kind"], string> = {
  text: "Text",
  json: "JSON",
  "computer-use": "Computer use",
};

function TextBody({ body, policy }: { body: string; policy: RedactionPolicy }) {
  const matches = useMemo(() => detect(body), [body]);
  const segments = useMemo(() => segmentText(body, matches), [body, matches]);
  const hasMatches = matches.length > 0;

  return (
    <pre
      data-testid="audit-output-body-text"
      {...(hasMatches ? { "data-cross-links": "true" } : {})}
      className="max-h-96 overflow-auto rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/20 p-3 text-sm font-mono whitespace-pre-wrap break-words text-[var(--foreground)]"
    >
      {hasMatches ? (
        <span data-testid="audit-output-cross-links">
          {segments.map((seg, idx) =>
            seg.kind === "text" ? (
              <Fragment key={idx}>
                <RedactedText policy={policy}>{seg.value}</RedactedText>
              </Fragment>
            ) : (
              <CrossSystemMatchAnchor key={idx} match={seg.match} />
            ),
          )}
        </span>
      ) : (
        <RedactedText policy={policy}>{body}</RedactedText>
      )}
    </pre>
  );
}

function CrossSystemMatchAnchor({ match }: { match: CrossSystemMatch }) {
  const gen = getGenerator(match.system);
  const label = gen?.systemLabel ?? match.system;
  return (
    <CrossSystemLink
      system={match.system}
      systemLabel={label}
      href={match.deepLink.url}
    >
      {match.raw}
    </CrossSystemLink>
  );
}

function JsonBody({ body }: { body: AuditOutput["body"] }) {
  return (
    <div
      data-testid="audit-output-body-json"
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/20 p-3 text-sm font-mono text-[var(--foreground)] overflow-auto max-h-96"
    >
      <JsonNode value={body} depth={0} />
    </div>
  );
}

function JsonNode({ value, depth }: { value: unknown; depth: number }) {
  if (value === null || typeof value !== "object") {
    return <span className="text-[var(--primary)]">{JSON.stringify(value)}</span>;
  }
  if (Array.isArray(value)) {
    if (value.length === 0) return <span>[]</span>;
    return (
      <details open={depth < AUTO_EXPAND_DEPTH} className="ml-2">
        <summary className="cursor-pointer text-[var(--muted-foreground)]">
          Array ({value.length})
        </summary>
        <ul className="ml-3 border-l border-[var(--border)] pl-3 space-y-0.5">
          {value.map((item, idx) => (
            <li key={idx}>
              <span className="text-[var(--muted-foreground)]">{idx}:</span>{" "}
              <JsonNode value={item} depth={depth + 1} />
            </li>
          ))}
        </ul>
      </details>
    );
  }
  const entries = Object.entries(value as Record<string, unknown>);
  if (entries.length === 0) return <span>{`{}`}</span>;
  return (
    <details open={depth < AUTO_EXPAND_DEPTH} className="ml-2">
      <summary className="cursor-pointer text-[var(--muted-foreground)]">
        {`{ ${entries.length} field${entries.length === 1 ? "" : "s"} }`}
      </summary>
      <ul className="ml-3 border-l border-[var(--border)] pl-3 space-y-0.5">
        {entries.map(([k, v]) => (
          <li key={k}>
            <span className="text-[var(--foreground)]">{k}:</span>{" "}
            <JsonNode value={v} depth={depth + 1} />
          </li>
        ))}
      </ul>
    </details>
  );
}

function ComputerUseBody({ output, policy }: { output: AuditOutput; policy: RedactionPolicy }) {
  const screenshots = output.screenshots ?? [];
  return (
    <div className="space-y-3">
      <TextBody body={String(output.body)} policy={policy} />
      {screenshots.length > 0 ? (
        <ScreenshotTrace frames={screenshots} />
      ) : (
        <p data-testid="audit-output-no-screenshots" className="text-xs text-[var(--muted-foreground)]">
          No screenshot trace captured.
        </p>
      )}
    </div>
  );
}

function ScreenshotTrace({ frames }: { frames: ScreenshotFrame[] }) {
  const [open, setOpen] = useState(false);
  const [lightbox, setLightbox] = useState<ScreenshotFrame | null>(null);
  return (
    <details
      data-testid="audit-output-screenshots"
      open={open}
      onToggle={(e) => setOpen((e.target as HTMLDetailsElement).open)}
      className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/10 p-3"
    >
      <summary className="cursor-pointer text-sm font-medium text-[var(--foreground)] inline-flex items-center gap-2">
        <ImageIcon aria-hidden className="size-4" /> Screenshot trace ({frames.length})
      </summary>
      <ol className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-3">
        {frames.map((frame, idx) => (
          <li key={`${frame.url}-${idx}`}>
            <button
              type="button"
              data-testid={`audit-output-screenshot-${idx}`}
              onClick={() => setLightbox(frame)}
              className="block w-full rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--card)] p-2 text-left hover:border-[var(--primary)]/40"
            >
              <img
                src={frame.url}
                alt={frame.description ?? `Frame ${idx + 1}`}
                className="aspect-video w-full rounded-[var(--radius-sm)] object-cover bg-[var(--muted)]"
                loading="lazy"
              />
              <p className="mt-1 text-[11px] text-[var(--muted-foreground)]">
                {idx + 1}. {frame.description ?? "frame"}
              </p>
              <p className="text-[10px] font-mono text-[var(--muted-foreground)]">
                {frame.timestamp}
              </p>
            </button>
          </li>
        ))}
      </ol>
      {lightbox ? <Lightbox frame={lightbox} onClose={() => setLightbox(null)} /> : null}
    </details>
  );
}

function Lightbox({ frame, onClose }: { frame: ScreenshotFrame; onClose: () => void }) {
  return (
    <div
      data-testid="audit-output-lightbox"
      role="dialog"
      aria-label="Screenshot lightbox"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-6"
      onClick={onClose}
    >
      <button
        type="button"
        aria-label="Close lightbox"
        onClick={onClose}
        className="absolute top-4 right-4 inline-flex size-10 items-center justify-center rounded-full bg-[var(--card)]/90 text-[var(--foreground)] hover:bg-[var(--card)]"
      >
        <X aria-hidden className="size-5" />
      </button>
      <img src={frame.url} alt={frame.description ?? "screenshot"} className="max-h-full max-w-full rounded-[var(--radius-md)]" />
    </div>
  );
}
