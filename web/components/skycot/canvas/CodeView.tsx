"use client";

import { useMemo, useState } from "react";
import { AlertCircle, Code2, RotateCcw, Wand2 } from "lucide-react";
import { SkillCanvas } from "@/components/skycot/canvas/SkillCanvas";
import {
  sdkSourceToSkillGraph,
  skillGraphToSdkSource,
  stableSkillGraphHash,
} from "@/lib/api/skill-code";
import type { SkillGraph } from "@/lib/types/skill-ir";

export interface CodeViewProps {
  graph: SkillGraph;
  skillId: string;
  version: string;
}

export function CodeView({ graph, skillId, version }: CodeViewProps) {
  const initialSource = useMemo(() => skillGraphToSdkSource(graph), [graph]);
  const [source, setSource] = useState(initialSource);
  const [parsedGraph, setParsedGraph] = useState<SkillGraph>(graph);
  const [error, setError] = useState<string | null>(null);

  const canvasHash = stableSkillGraphHash(graph);
  const parsedHash = stableSkillGraphHash(parsedGraph);
  const hashesMatch = canvasHash === parsedHash;

  function parseSource() {
    try {
      const next = sdkSourceToSkillGraph(source);
      setParsedGraph(next);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unable to parse SDK source.");
    }
  }

  function resetSource() {
    setSource(initialSource);
    setParsedGraph(graph);
    setError(null);
  }

  return (
    <section data-testid="skill-code-view" className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <h1 className="text-xl font-semibold text-[var(--foreground)]">Code mode</h1>
          <p className="text-sm text-[var(--muted-foreground)]">
            {skillId} · {version}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            data-testid="code-regenerate"
            onClick={resetSource}
            className="inline-flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2 text-sm font-medium"
          >
            <RotateCcw className="size-4" aria-hidden="true" />
            Regenerate
          </button>
          <button
            type="button"
            data-testid="code-parse"
            onClick={parseSource}
            className="inline-flex items-center gap-2 rounded-[var(--radius-md)] bg-[var(--primary)] px-3 py-2 text-sm font-medium text-[var(--primary-foreground)]"
          >
            <Wand2 className="size-4" aria-hidden="true" />
            Parse to canvas
          </button>
        </div>
      </div>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(360px,0.8fr)]">
        <div className="overflow-hidden rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)]">
          <div className="flex items-center gap-2 border-b border-[var(--border)] px-3 py-2 text-sm font-medium">
            <Code2 className="size-4" aria-hidden="true" />
            @skycot/skill-sdk
          </div>
          <textarea
            data-testid="skill-code-editor"
            value={source}
            spellCheck={false}
            onChange={(event) => setSource(event.target.value)}
            className="min-h-[520px] w-full resize-y bg-[var(--background)] p-3 font-mono text-xs leading-relaxed text-[var(--foreground)] outline-none"
          />
        </div>

        <div className="space-y-3">
          <div
            data-testid="roundtrip-status"
            data-match={hashesMatch || undefined}
            className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-3 text-xs text-[var(--muted-foreground)]"
          >
            <div>Canvas hash: {canvasHash}</div>
            <div>Code hash: {parsedHash}</div>
            <div className={hashesMatch ? "text-green-600 dark:text-green-400" : "text-[var(--destructive)]"}>
              {hashesMatch ? "Round-trip preserves IR" : "Round-trip changed IR"}
            </div>
          </div>

          {error ? (
            <div
              data-testid="skill-code-error"
              className="flex items-start gap-2 rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 p-3 text-sm text-[var(--destructive)]"
            >
              <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
              <span>{error}</span>
            </div>
          ) : null}

          <SkillCanvas graph={parsedGraph} skipFlowRender className="h-[520px] min-h-[420px]" />
        </div>
      </div>
    </section>
  );
}
