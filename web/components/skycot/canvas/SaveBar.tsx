"use client";

import { Save } from "lucide-react";

export interface SaveBarProps {
  dirty: boolean;
  saving: boolean;
  error?: string | null;
  versions: string[];
  abScheduled?: boolean;
  onSave: () => void;
}

export function SaveBar({
  dirty,
  saving,
  error,
  versions,
  abScheduled,
  onSave,
}: SaveBarProps) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--border)] bg-[var(--background)] p-3">
      <div className="min-w-0 text-xs text-[var(--muted-foreground)]">
        <span data-testid="edit-history">
          Last versions: {versions.slice(0, 5).join(", ") || "none"}
        </span>
        {abScheduled ? (
          <span data-testid="ab-scheduled" className="ml-3 text-green-600 dark:text-green-400">
            A/B eval scheduled
          </span>
        ) : null}
        {error ? (
          <div data-testid="save-error" className="mt-1 text-[var(--destructive)]">
            {error}
          </div>
        ) : null}
      </div>
      <button
        type="button"
        data-testid="save-ir"
        onClick={onSave}
        disabled={saving || !dirty}
        className="inline-flex items-center gap-2 rounded-[var(--radius-md)] bg-[var(--primary)] px-3 py-2 text-sm font-medium text-[var(--primary-foreground)] disabled:cursor-not-allowed disabled:opacity-50"
      >
        <Save className="size-4" aria-hidden="true" />
        {saving ? "Saving" : "Save"}
      </button>
    </div>
  );
}
