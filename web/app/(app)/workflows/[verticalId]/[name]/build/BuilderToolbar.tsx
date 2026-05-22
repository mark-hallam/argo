"use client";
import { ArrowLeft, Loader2, Play, Rocket, RotateCcw, Save, Send, Sparkles } from "lucide-react";
import Link from "next/link";
import { useCallback } from "react";

import { ConcurrentEditBanner } from "@/components/skycot/workflows/ConcurrentEditBanner";
import {
  clearSaveDraftConflict,
  useSaveDraftConflict,
} from "@/lib/hooks/use-workflow-save-publish";

import type { WorkflowSpec } from "./types";

type TestRunPhase = "idle" | "running" | "complete" | "failed";

interface Props {
  workflow: WorkflowSpec;
  verticalId: string;
  name: string;
  dirty: boolean;
  saving?: boolean;
  publishing?: boolean;
  /** Numeric version of the most-recently-saved draft; null until first save in this session. */
  savedDraftVersion?: number | null;
  /** When true, the saved draft matches the current canvas — Publish is safe. */
  canPublish?: boolean;
  promptText: string;
  proposing: boolean;
  onPromptTextChange: (next: string) => void;
  onProposeChange: () => void;
  /** Pick a suggestion chip: fill the prompt AND submit immediately. */
  onPickSuggestion: (next: string) => void;
  onTestRun: () => void;
  onSaveAndPublish: () => void;
  onPublish: () => void;
  testRunPhase?: TestRunPhase;
}

export function BuilderToolbar({
  workflow,
  verticalId,
  name,
  dirty,
  saving = false,
  publishing = false,
  savedDraftVersion = null,
  canPublish = false,
  promptText,
  proposing,
  onPromptTextChange,
  onProposeChange,
  onPickSuggestion,
  onTestRun,
  onSaveAndPublish,
  onPublish,
  testRunPhase = "idle",
}: Props) {
  const isRunning = testRunPhase === "running";
  const hasRun = testRunPhase === "complete" || testRunPhase === "failed";
  const publishDisabled = !canPublish || publishing || saving;
  // Tooltip nudges the user toward the right next action when the
  // button is disabled — without this the disabled state is silent.
  const publishHint = savedDraftVersion === null
    ? "Save a draft first"
    : !canPublish
      ? "Save your latest changes before publishing"
      : `Publish v${savedDraftVersion} to head`;
  // Concurrent-edit conflict state is published by the saveDraft API
  // client into a module-level store; we subscribe here so the banner
  // can render without the parent page having to thread props. The
  // banner sits below the toolbar and above the unsaved-changes
  // banner — the user's dirty edits are preserved while they decide
  // whether to refresh.
  const lastConflict = useSaveDraftConflict();
  const handleRefresh = useCallback(() => {
    clearSaveDraftConflict();
    if (typeof window !== "undefined") {
      window.location.reload();
    }
  }, []);
  const handleShowDiff = useCallback(() => {
    if (typeof window === "undefined") return;
    // v1 stub: open the read-only viewer of the current head in a new
    // tab. A real inline diff against the user's dirty draft is a v1.x
    // follow-up.
    const url = `/workflows/${encodeURIComponent(verticalId)}/${encodeURIComponent(name)}`;
    window.open(url, "_blank", "noopener");
  }, [verticalId, name]);
  const handleDismiss = useCallback(() => {
    clearSaveDraftConflict();
  }, []);
  return (
    <>
      <header className="builder-top">
        <Link href="/workflows" className="builder-back">
          <ArrowLeft size={12} /> ALL WORKFLOWS
        </Link>
        <span className="builder-vertical-tag">{workflow.vertical}</span>
        <h1 className="builder-title">{workflow.name}</h1>
        <span className="builder-version">version {workflow.version}</span>
        <div className="builder-actions">
          <Link href={`/workflows/${verticalId}/${name}`} className="builder-btn">
            View timeline
          </Link>
          <button
            type="button"
            className="builder-btn"
            onClick={onTestRun}
            disabled={isRunning}
            data-testid="builder-test-run"
            data-run-phase={testRunPhase}
          >
            {isRunning ? (
              <>
                <Loader2 size={13} className="builder-spin" /> Running…
              </>
            ) : hasRun ? (
              <>
                <RotateCcw size={13} /> Run again
              </>
            ) : (
              <>
                <Play size={13} /> Test run
              </>
            )}
          </button>
          <button
            type="button"
            className="builder-btn builder-btn-primary"
            onClick={onSaveAndPublish}
            disabled={!dirty || saving}
            data-testid="builder-save"
            data-dirty={dirty ? "true" : "false"}
            data-saving={saving ? "true" : "false"}
          >
            {saving ? (
              <>
                <Loader2 size={13} className="builder-spin" /> Saving…
              </>
            ) : (
              <>
                <Save size={13} /> Save as draft
              </>
            )}
          </button>
          <button
            type="button"
            className="builder-btn builder-btn-primary"
            onClick={onPublish}
            disabled={publishDisabled}
            title={publishHint}
            data-testid="builder-publish"
            data-can-publish={canPublish ? "true" : "false"}
            data-publishing={publishing ? "true" : "false"}
          >
            {publishing ? (
              <>
                <Loader2 size={13} className="builder-spin" /> Publishing…
              </>
            ) : (
              <>
                <Rocket size={13} /> Publish
              </>
            )}
          </button>
        </div>
      </header>

      {lastConflict ? (
        <ConcurrentEditBanner
          currentVersion={lastConflict.current_version}
          onRefresh={handleRefresh}
          onShowDiff={handleShowDiff}
          onDismiss={handleDismiss}
        />
      ) : null}

      {/* AI prompt bar — the SKYCOT differentiator */}
      <div className="builder-prompt-bar">
        <div className="builder-prompt-icon">
          <Sparkles size={16} />
        </div>
        <input
          type="text"
          className="builder-prompt-input"
          placeholder="Describe a change in plain English — e.g. 'Add a step that emails the seller before posting'"
          value={promptText}
          onChange={(e) => onPromptTextChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onProposeChange();
          }}
        />
        <button
          type="button"
          className="builder-prompt-send"
          disabled={!promptText.trim() || proposing}
          onClick={onProposeChange}
        >
          {proposing ? "Proposing…" : (<><Send size={12} /> Propose</>)}
        </button>
        <button
          type="button"
          className="builder-prompt-suggest"
          disabled={proposing}
          onClick={() =>
            onPickSuggestion("Add a step that emails the seller before posting")
          }
        >
          Add a step
        </button>
        <button
          type="button"
          className="builder-prompt-suggest"
          disabled={proposing}
          onClick={() =>
            onPickSuggestion("Convert this to fully autonomous (no approval needed)")
          }
        >
          Remove gate
        </button>
      </div>
    </>
  );
}
