"use client";
import { ArrowLeft, Loader2, Play, RotateCcw, Send, Sparkles } from "lucide-react";
import Link from "next/link";

import type { WorkflowSpec } from "./types";

type TestRunPhase = "idle" | "running" | "complete" | "failed";

interface Props {
  workflow: WorkflowSpec;
  verticalId: string;
  name: string;
  dirty: boolean;
  promptText: string;
  proposing: boolean;
  onPromptTextChange: (next: string) => void;
  onProposeChange: () => void;
  /** Pick a suggestion chip: fill the prompt AND submit immediately. */
  onPickSuggestion: (next: string) => void;
  onTestRun: () => void;
  onSaveAndPublish: () => void;
  testRunPhase?: TestRunPhase;
}

export function BuilderToolbar({
  workflow,
  verticalId,
  name,
  dirty,
  promptText,
  proposing,
  onPromptTextChange,
  onProposeChange,
  onPickSuggestion,
  onTestRun,
  onSaveAndPublish,
  testRunPhase = "idle",
}: Props) {
  const isRunning = testRunPhase === "running";
  const hasRun = testRunPhase === "complete" || testRunPhase === "failed";
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
            disabled={!dirty}
            data-testid="builder-save"
            data-dirty={dirty ? "true" : "false"}
          >
            <Send size={13} /> Save + publish
          </button>
        </div>
      </header>

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
