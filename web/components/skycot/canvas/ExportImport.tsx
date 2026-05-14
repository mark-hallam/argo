"use client";

import { AlertCircle, CheckCircle2, Download, FileJson, Upload } from "lucide-react";
import { useRef, useState } from "react";
import {
  exportSkillIr,
  importSkillIr,
  parseSkillIrBundle,
  type ImportSkillIrResult,
  type SkillIrBundle,
} from "@/lib/api/skill-ir-export";
import type { SkillGraph } from "@/lib/types/skill-ir";
import { DiffView } from "./DiffView";

export interface ExportImportProps {
  skillId: string;
  graph: SkillGraph;
  version: string;
  irHash?: string;
  onAdopt?: (result: ImportSkillIrResult) => void;
  exportBundle?: typeof exportSkillIr;
  importBundle?: typeof importSkillIr;
}

export function ExportImport({
  skillId,
  graph,
  version,
  irHash,
  onAdopt,
  exportBundle = exportSkillIr,
  importBundle = importSkillIr,
}: ExportImportProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [preview, setPreview] = useState<SkillIrBundle | null>(null);
  const [busy, setBusy] = useState<"export" | "import" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [adopted, setAdopted] = useState<ImportSkillIrResult | null>(null);

  async function onExport() {
    setBusy("export");
    setError(null);
    try {
      const bundle = await exportBundle(skillId, version, graph, irHash);
      downloadBundle(bundle);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Export failed");
    } finally {
      setBusy(null);
    }
  }

  async function onFile(file: File | undefined) {
    if (!file) return;
    setError(null);
    setAdopted(null);
    try {
      const parsed = JSON.parse(await file.text()) as unknown;
      setPreview(parseSkillIrBundle(parsed));
    } catch (err) {
      setPreview(null);
      setError(err instanceof Error ? err.message : "Import file is invalid");
    } finally {
      if (inputRef.current) inputRef.current.value = "";
    }
  }

  async function onAdoptPreview() {
    if (!preview) return;
    setBusy("import");
    setError(null);
    try {
      const result = await importBundle(skillId, preview);
      setAdopted(result);
      onAdopt?.(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Import failed");
    } finally {
      setBusy(null);
    }
  }

  return (
    <section
      data-testid="skill-ir-export-import"
      className="space-y-3 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-3"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <FileJson className="size-4 text-[var(--muted-foreground)]" aria-hidden="true" />
          <div className="min-w-0">
            <h2 className="truncate text-sm font-semibold text-[var(--foreground)]">
              Portable IR
            </h2>
            <p className="truncate text-xs text-[var(--muted-foreground)]">
              {version} {irHash ? `· ${irHash.slice(0, 10)}` : ""}
            </p>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            data-testid="export-skill-ir"
            onClick={onExport}
            disabled={busy !== null}
            className={buttonClass}
          >
            <Download className="size-4" aria-hidden="true" />
            {busy === "export" ? "Exporting" : "Export"}
          </button>
          <button
            type="button"
            data-testid="choose-skill-ir-import"
            onClick={() => inputRef.current?.click()}
            disabled={busy !== null}
            className={buttonClass}
          >
            <Upload className="size-4" aria-hidden="true" />
            Import
          </button>
          <input
            ref={inputRef}
            data-testid="skill-ir-import-file"
            type="file"
            accept="application/json,.json,.skill.json"
            className="hidden"
            onChange={(event) => onFile(event.target.files?.[0])}
          />
        </div>
      </div>

      {error ? (
        <p
          data-testid="skill-ir-import-error"
          className="flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-3 py-2 text-sm text-[var(--destructive)]"
        >
          <AlertCircle className="size-4" aria-hidden="true" />
          {error}
        </p>
      ) : null}

      {adopted ? (
        <p
          data-testid="skill-ir-import-success"
          className="flex items-center gap-2 rounded-[var(--radius-md)] border border-green-500/30 bg-green-500/5 px-3 py-2 text-sm text-green-700 dark:text-green-300"
        >
          <CheckCircle2 className="size-4" aria-hidden="true" />
          Imported as {adopted.version}
        </p>
      ) : null}

      {preview ? (
        <div data-testid="skill-ir-import-preview" className="space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-xs text-[var(--muted-foreground)]">
              {preview.version} · {preview.irHash.slice(0, 12)}
            </p>
            <button
              type="button"
              data-testid="adopt-skill-ir-import"
              onClick={onAdoptPreview}
              disabled={busy !== null}
              className={primaryButtonClass}
            >
              <CheckCircle2 className="size-4" aria-hidden="true" />
              {busy === "import" ? "Importing" : "Adopt"}
            </button>
          </div>
          <DiffView
            leftGraph={graph}
            rightGraph={preview.graph}
            leftVersion={version}
            rightVersion={preview.version}
          />
        </div>
      ) : null}
    </section>
  );
}

function downloadBundle(bundle: SkillIrBundle) {
  const text = JSON.stringify(bundle, null, 2);
  const blob = new Blob([text], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `${bundle.sourceSkillId}-${bundle.version}.skill.json`;
  anchor.click();
  URL.revokeObjectURL(url);
}

const buttonClass =
  "inline-flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40 disabled:cursor-not-allowed disabled:opacity-50";

const primaryButtonClass =
  "inline-flex items-center gap-2 rounded-[var(--radius-md)] bg-[var(--primary)] px-3 py-2 text-sm font-medium text-[var(--primary-foreground)] hover:bg-[var(--primary)]/90 disabled:cursor-not-allowed disabled:opacity-50";
