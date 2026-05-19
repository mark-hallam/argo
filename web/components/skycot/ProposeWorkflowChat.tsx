"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Loader2, Send } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { WorkflowTomlPreview } from "./WorkflowTomlPreview";
import { cn } from "@/lib/utils";

/**
 * ProposeWorkflowChat — chat surface that turns plain English into a
 * proposed workflow TOML. Streams the assistant response from
 * `/api/workflows/propose`, lets the operator approve / edit /
 * cancel. Approve posts to the gateway workflow-edit API (P1 #6);
 * Edit drops to a textarea pre-filled with the streamed TOML.
 */

export interface ProposeWorkflowChatProps {
  workerId: string;
  verticalId: string;
  /** Where to POST the approved workflow. P1 #6 contract. */
  approveEndpoint: string;
  /** Where to send the user back on cancel / after persistence. */
  onCancel: () => void;
  onApproved: (workflowName: string) => void;
  /** Override the proposal endpoint (test seam). */
  proposeEndpoint?: string;
  /** Override fetch (test seam). */
  fetchImpl?: typeof fetch;
}

type Status =
  | { kind: "idle" }
  | { kind: "streaming" }
  | { kind: "ready" }
  | { kind: "editing" }
  | { kind: "submitting" }
  | { kind: "error"; message: string };

const MAX_INPUT_CHARS = 4000;
const NAME_RE = /^\s*name\s*=\s*"([^"\\]+)"/m;

function extractWorkflowName(toml: string): string | null {
  const headerSplit = toml.split(/\[\s*workflow\s*\]/);
  if (headerSplit.length < 2) return null;
  const match = NAME_RE.exec(headerSplit[1]!);
  return match ? match[1]!.trim() : null;
}

async function* streamLines(
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

export function ProposeWorkflowChat({
  workerId,
  verticalId,
  approveEndpoint,
  onCancel,
  onApproved,
  proposeEndpoint = "/api/workflows/propose",
  fetchImpl,
}: ProposeWorkflowChatProps) {
  const doFetch = fetchImpl ?? fetch;
  const [input, setInput] = useState("");
  const [streamedToml, setStreamedToml] = useState("");
  const [editorToml, setEditorToml] = useState("");
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const inputRef = useRef<HTMLTextAreaElement | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    inputRef.current?.focus();
    return () => abortRef.current?.abort();
  }, []);

  const onSubmit = useCallback(async () => {
    const description = input.trim();
    if (description.length === 0) return;
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    setStreamedToml("");
    setStatus({ kind: "streaming" });
    try {
      const res = await doFetch(proposeEndpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ workerId, verticalId, description }),
        signal: controller.signal,
      });
      if (!res.ok) {
        const detail = await res.text().catch(() => "");
        throw new Error(detail || `Proposal request failed (${res.status}).`);
      }
      if (!res.body) {
        const fallback = await res.text();
        setStreamedToml(fallback);
        setStatus({ kind: "ready" });
        return;
      }
      let acc = "";
      for await (const chunk of streamLines(res.body)) {
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
  }, [input, workerId, verticalId, proposeEndpoint, doFetch]);

  const onApprove = useCallback(async () => {
    const toml = status.kind === "editing" ? editorToml : streamedToml;
    const workflowName = extractWorkflowName(toml);
    if (!workflowName) {
      setStatus({
        kind: "error",
        message: "Couldn't read [workflow].name from the proposal.",
      });
      return;
    }
    setStatus({ kind: "submitting" });
    try {
      const res = await doFetch(`${approveEndpoint}/${encodeURIComponent(workflowName)}`, {
        method: "POST",
        headers: { "Content-Type": "application/toml" },
        body: toml,
      });
      if (!res.ok) {
        const detail = await res.text().catch(() => "");
        throw new Error(detail || `Approve failed (${res.status}).`);
      }
      onApproved(workflowName);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Couldn't save workflow.";
      setStatus({ kind: "error", message });
    }
  }, [status, editorToml, streamedToml, approveEndpoint, onApproved, doFetch]);

  const onEdit = useCallback(() => {
    setEditorToml(streamedToml);
    setStatus({ kind: "editing" });
  }, [streamedToml]);

  const onLeaveEditing = useCallback(() => {
    setStreamedToml(editorToml);
    setStatus({ kind: "ready" });
  }, [editorToml]);

  const isBusy = status.kind === "streaming" || status.kind === "submitting";
  const inputDisabled = isBusy;
  const showActions =
    status.kind === "ready" ||
    status.kind === "editing" ||
    status.kind === "submitting" ||
    status.kind === "error";
  const isSubmitting = status.kind === "submitting";
  const isEditing = status.kind === "editing";
  const tomlForPreview = isEditing ? editorToml : streamedToml;

  return (
    <section
      className="flex flex-col gap-4"
      aria-label="Propose workflow from plain English"
    >
      <div className="flex flex-col gap-2">
        <label
          htmlFor="propose-workflow-input"
          className="text-sm font-medium text-[color:var(--foreground)]"
        >
          Describe the responsibility in plain English
        </label>
        <textarea
          id="propose-workflow-input"
          ref={inputRef}
          value={input}
          onChange={(e) => setInput(e.target.value.slice(0, MAX_INPUT_CHARS))}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              void onSubmit();
            }
          }}
          rows={4}
          maxLength={MAX_INPUT_CHARS}
          placeholder="every Friday at 3pm, take this week's themes, draft the all-hands update, run it past head of product, post to Notion"
          aria-label="Describe the responsibility in plain English"
          disabled={inputDisabled}
          className={cn(
            "w-full resize-y rounded-[var(--radius-md)] border border-[var(--border)]",
            "bg-[var(--background)] px-3 py-2 text-sm leading-relaxed",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
            "disabled:opacity-60",
          )}
        />
        <div className="flex items-center justify-between gap-2">
          <span className="text-xs text-[color:var(--muted-foreground)]">
            {input.length}/{MAX_INPUT_CHARS} · ⌘↵ to propose
          </span>
          <Button
            type="button"
            onClick={() => void onSubmit()}
            disabled={inputDisabled || input.trim().length === 0}
            data-testid="propose-workflow-submit"
          >
            {status.kind === "streaming" ? (
              <Loader2 className="animate-spin" aria-hidden />
            ) : (
              <Send aria-hidden />
            )}
            <span>Propose</span>
          </Button>
        </div>
      </div>

      <div
        aria-live="polite"
        aria-busy={status.kind === "streaming"}
        className="flex flex-col gap-3"
        data-testid="propose-workflow-output"
      >
        {streamedToml.length > 0 && status.kind !== "editing" ? (
          <WorkflowTomlPreview
            toml={streamedToml}
            isStreaming={status.kind === "streaming"}
          />
        ) : null}
        {status.kind === "editing" ? (
          <div className="flex flex-col gap-2">
            <label
              htmlFor="propose-workflow-editor"
              className="text-sm font-medium text-[color:var(--foreground)]"
            >
              Edit the proposed TOML
            </label>
            <textarea
              id="propose-workflow-editor"
              value={editorToml}
              onChange={(e) => setEditorToml(e.target.value)}
              rows={16}
              aria-label="Edit the proposed TOML"
              className={cn(
                "w-full resize-y rounded-[var(--radius-md)] border border-[var(--border)]",
                "bg-[var(--muted)] px-3 py-2 text-sm font-mono leading-relaxed",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
              )}
            />
          </div>
        ) : null}
        {status.kind === "error" ? (
          <p
            role="alert"
            className="rounded-[var(--radius-md)] border border-[color:var(--destructive)]/40 bg-[color:var(--destructive)]/10 px-3 py-2 text-sm text-[color:var(--destructive)]"
            data-testid="propose-workflow-error"
          >
            {status.message}
          </p>
        ) : null}
      </div>

      {showActions ? (
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button
            type="button"
            variant="ghost"
            onClick={onCancel}
            disabled={isSubmitting}
            data-testid="propose-workflow-cancel"
          >
            Cancel
          </Button>
          {isEditing ? (
            <Button
              type="button"
              variant="secondary"
              onClick={onLeaveEditing}
              data-testid="propose-workflow-done-editing"
            >
              Done editing
            </Button>
          ) : (
            <Button
              type="button"
              variant="secondary"
              onClick={onEdit}
              disabled={isSubmitting || streamedToml.length === 0}
              data-testid="propose-workflow-edit"
            >
              Edit
            </Button>
          )}
          <Button
            type="button"
            onClick={() => void onApprove()}
            disabled={isSubmitting || tomlForPreview.trim().length === 0}
            aria-busy={isSubmitting}
            data-testid="propose-workflow-approve"
          >
            {isSubmitting ? <Loader2 className="animate-spin" aria-hidden /> : null}
            <span>{isSubmitting ? "Saving…" : "Approve & save"}</span>
          </Button>
        </div>
      ) : null}
    </section>
  );
}
