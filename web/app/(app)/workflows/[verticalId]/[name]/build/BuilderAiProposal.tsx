"use client";
import {
  AlertTriangle,
  Check,
  Loader2,
  Pencil,
  Sparkles,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { diffSteps, parseProposalToml, type ProposalDiff } from "./propose-toml";
import type { WorkflowStep } from "./types";

export interface BuilderAiProposalProps {
  prompt: string;
  verticalId: string;
  workerId: string;
  currentSteps: WorkflowStep[];
  onApprove: (steps: WorkflowStep[]) => void;
  onCancel: () => void;
  /** Test seam. */
  fetchImpl?: typeof fetch;
  /** Test seam. */
  proposeEndpoint?: string;
}

type Status =
  | { kind: "streaming" }
  | { kind: "ready" }
  | { kind: "editing" }
  | { kind: "error"; message: string };

async function* readChunks(
  body: ReadableStream<Uint8Array>,
): AsyncGenerator<string, void, unknown> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  while (true) {
    const { value, done } = await reader.read();
    if (done) return;
    yield decoder.decode(value, { stream: true });
  }
}

export function BuilderAiProposal({
  prompt,
  verticalId,
  workerId,
  currentSteps,
  onApprove,
  onCancel,
  fetchImpl,
  proposeEndpoint = "/api/workflows/propose",
}: BuilderAiProposalProps) {
  const doFetch = fetchImpl ?? fetch;
  const [streamedToml, setStreamedToml] = useState("");
  const [editorToml, setEditorToml] = useState("");
  const [status, setStatus] = useState<Status>({ kind: "streaming" });
  const [attempt, setAttempt] = useState(0);
  const abortRef = useRef<AbortController | null>(null);

  const runStream = useCallback(async () => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    setStreamedToml("");
    setStatus({ kind: "streaming" });
    try {
      const res = await doFetch(proposeEndpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ workerId, verticalId, description: prompt }),
        signal: controller.signal,
      });
      if (!res.ok) {
        const detail = await res.text().catch(() => "");
        throw new Error(detail || `Proposal failed (${res.status}).`);
      }
      if (!res.body) {
        const fallback = await res.text();
        setStreamedToml(fallback);
        setStatus({ kind: "ready" });
        return;
      }
      let acc = "";
      for await (const chunk of readChunks(res.body)) {
        if (controller.signal.aborted) return;
        acc += chunk;
        setStreamedToml(acc);
      }
      setStatus({ kind: "ready" });
    } catch (err) {
      if (controller.signal.aborted) return;
      const message = err instanceof Error ? err.message : "Streaming failed.";
      setStatus({ kind: "error", message });
    }
  }, [doFetch, proposeEndpoint, workerId, verticalId, prompt]);

  useEffect(() => {
    void runStream();
    return () => abortRef.current?.abort();
  }, [runStream, attempt]);

  const tomlForDiff = status.kind === "editing" ? editorToml : streamedToml;
  const parsed = useMemo(() => parseProposalToml(tomlForDiff), [tomlForDiff]);
  const diff: ProposalDiff = useMemo(
    () => diffSteps(currentSteps, parsed.steps),
    [currentSteps, parsed.steps],
  );

  const onEdit = useCallback(() => {
    setEditorToml(streamedToml);
    setStatus({ kind: "editing" });
  }, [streamedToml]);

  const onLeaveEditing = useCallback(() => {
    setStreamedToml(editorToml);
    setStatus({ kind: "ready" });
  }, [editorToml]);

  const onApproveClick = useCallback(() => {
    if (parsed.steps.length === 0) return;
    onApprove(parsed.steps);
  }, [parsed.steps, onApprove]);

  const onRetry = useCallback(() => setAttempt((a) => a + 1), []);

  const isStreaming = status.kind === "streaming";
  const hasError = status.kind === "error";
  const hasProposal = parsed.steps.length > 0;
  const isEditing = status.kind === "editing";

  return (
    <aside
      className="builder-proposal"
      role="dialog"
      aria-label="AI workflow proposal"
      data-testid="builder-ai-proposal"
    >
      <header className="builder-proposal-head">
        <div className="builder-proposal-icon">
          <Sparkles size={14} />
        </div>
        <div className="builder-proposal-titlewrap">
          <span className="builder-proposal-eyebrow">PROPOSED CHANGE</span>
          <h2 className="builder-proposal-title">
            {parsed.workflowName ?? "Drafting…"}
          </h2>
        </div>
        <button
          type="button"
          className="builder-panel-close"
          aria-label="Close proposal"
          onClick={onCancel}
          data-testid="builder-ai-proposal-close"
        >
          <X size={16} />
        </button>
      </header>

      <div className="builder-proposal-prompt">
        <span className="builder-proposal-prompt-label">YOU ASKED FOR</span>
        <p className="builder-proposal-prompt-body">{prompt}</p>
      </div>

      {hasError ? (
        <div
          role="alert"
          className="builder-proposal-error"
          data-testid="builder-ai-proposal-error"
        >
          <AlertTriangle size={14} />
          <div>
            <strong>Couldn't reach the assistant.</strong>
            <p>{status.kind === "error" ? status.message : ""}</p>
          </div>
          <button
            type="button"
            className="builder-prompt-send"
            onClick={onRetry}
            data-testid="builder-ai-proposal-retry"
          >
            Try again
          </button>
        </div>
      ) : null}

      <section className="builder-proposal-diff" aria-label="Change summary">
        <span className="builder-proposal-section-label">CHANGES</span>
        {hasProposal ? (
          <ul className="builder-proposal-diff-list" data-testid="builder-ai-proposal-diff">
            {diff.added.map((s) => (
              <li key={`add-${s.id}`} className="builder-proposal-diff-item builder-proposal-diff-add">
                <span className="builder-proposal-diff-marker">+</span>
                <span className="builder-proposal-diff-name">{s.name}</span>
                <span className="builder-proposal-diff-kind">{s.kind}</span>
              </li>
            ))}
            {diff.changed.map(({ before, after }) => (
              <li
                key={`chg-${after.id}`}
                className="builder-proposal-diff-item builder-proposal-diff-change"
              >
                <span className="builder-proposal-diff-marker">~</span>
                <span className="builder-proposal-diff-name">{after.name}</span>
                <span className="builder-proposal-diff-kind">
                  {before.kind} → {after.kind}
                </span>
              </li>
            ))}
            {diff.removed.map((s) => (
              <li
                key={`rm-${s.id}`}
                className="builder-proposal-diff-item builder-proposal-diff-remove"
              >
                <span className="builder-proposal-diff-marker">−</span>
                <span className="builder-proposal-diff-name">{s.name}</span>
                <span className="builder-proposal-diff-kind">{s.kind}</span>
              </li>
            ))}
            {diff.unchangedCount > 0 ? (
              <li className="builder-proposal-diff-item builder-proposal-diff-same">
                <span className="builder-proposal-diff-marker">·</span>
                <span className="builder-proposal-diff-name">
                  {diff.unchangedCount} step{diff.unchangedCount === 1 ? "" : "s"} unchanged
                </span>
              </li>
            ) : null}
          </ul>
        ) : isStreaming ? (
          <p className="builder-proposal-empty">
            <Loader2 size={12} className="builder-proposal-spin" /> Streaming response…
          </p>
        ) : (
          <p className="builder-proposal-empty">No structured steps parsed yet.</p>
        )}
      </section>

      <section className="builder-proposal-toml" aria-label="Proposed TOML">
        <span className="builder-proposal-section-label">
          {isStreaming ? "STREAMING…" : "TOML"}
        </span>
        {isEditing ? (
          <textarea
            aria-label="Edit proposed TOML"
            data-testid="builder-ai-proposal-editor"
            className="builder-proposal-editor"
            value={editorToml}
            onChange={(e) => setEditorToml(e.target.value)}
            rows={14}
          />
        ) : (
          <pre className="builder-proposal-pre" data-testid="builder-ai-proposal-toml">
            <code>{streamedToml || (isStreaming ? "…" : "")}</code>
          </pre>
        )}
      </section>

      <footer className="builder-proposal-actions">
        <button
          type="button"
          className="builder-btn"
          onClick={onCancel}
          data-testid="builder-ai-proposal-reject"
        >
          Reject
        </button>
        {isEditing ? (
          <button
            type="button"
            className="builder-btn"
            onClick={onLeaveEditing}
            data-testid="builder-ai-proposal-done-edit"
          >
            Done editing
          </button>
        ) : (
          <button
            type="button"
            className="builder-btn"
            onClick={onEdit}
            disabled={isStreaming || hasError || streamedToml.length === 0}
            data-testid="builder-ai-proposal-edit"
          >
            <Pencil size={12} /> Edit
          </button>
        )}
        <button
          type="button"
          className="builder-btn builder-btn-primary"
          onClick={onApproveClick}
          disabled={isStreaming || hasError || !hasProposal}
          data-testid="builder-ai-proposal-approve"
        >
          <Check size={13} /> Approve
        </button>
      </footer>
    </aside>
  );
}
