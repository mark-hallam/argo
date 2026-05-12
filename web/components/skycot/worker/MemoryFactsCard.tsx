"use client";

import {
  useEffect, useMemo, useRef, useState, type KeyboardEvent,
} from "react";
import { Pencil, Plus, Trash2 } from "lucide-react";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { RedactedText } from "@/components/skycot/RedactedText";
import type { RedactionPolicy } from "@/lib/redaction/redact";
import type { WorkerFact } from "@/lib/api/worker-memory";
import { cn } from "@/lib/utils";

/**
 * Facts card for the Memory tab (E4-d). UI-only — mutations and the
 * Add-a-fact dialog are owned by the parent page so the optimistic
 * cache layer in `useWorkerMemoryActions` can run uninterrupted.
 *
 * Behaviour: case-insensitive substring search; ✎ → inline textarea
 * (Enter saves, Esc cancels); 🗑 → confirm dialog → delete; "+ Add a
 * fact" calls `onRequestAdd`. Fact text is wrapped in `<RedactedText>`
 * so PII sentinels resolve under the viewer's redaction policy.
 */

export interface MemoryFactsCardProps {
  workerName: string;
  facts: WorkerFact[];
  canEdit: boolean;
  onAdd: (text: string) => void | Promise<void>;
  onEdit: (factId: string, text: string) => void | Promise<void>;
  onDelete: (factId: string) => void | Promise<void>;
  onRequestAdd: () => void;
  error?: string | null;
  onRetry?: () => void;
  /** Resolved policy; omit to use `useRedactionPolicy()`. */
  redactionPolicy?: RedactionPolicy;
  className?: string;
}

export function MemoryFactsCard({
  workerName, facts, canEdit, onEdit, onDelete, onRequestAdd,
  error, onRetry, redactionPolicy, className,
}: MemoryFactsCardProps) {
  const [search, setSearch] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<WorkerFact | null>(null);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return facts;
    return facts.filter((it) => it.text.toLowerCase().includes(q));
  }, [facts, search]);

  return (
    <Card data-testid="card-memory-facts" className={className}>
      <CardHeader className="flex flex-row items-start justify-between gap-3">
        <div className="flex items-baseline gap-2">
          <CardTitle>Facts she knows</CardTitle>
          <span data-testid="facts-count" className="text-xs text-[var(--muted-foreground)]">
            {facts.length} {facts.length === 1 ? "fact" : "facts"}
          </span>
        </div>
        <input
          type="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search facts..."
          data-testid="facts-search"
          aria-label="Search facts"
          className="h-8 w-44 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)] px-2 text-sm text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
        />
      </CardHeader>
      <CardBody className="pt-1 space-y-2">
        {error ? (
          <ErrorBlock message={error} onRetry={onRetry} />
        ) : facts.length === 0 ? (
          <EmptyState workerName={workerName} />
        ) : filtered.length === 0 ? (
          <p data-testid="facts-empty-search" className="py-6 text-center text-sm text-[var(--muted-foreground)]">
            No facts match &ldquo;{search}&rdquo;.
          </p>
        ) : (
          <ul data-testid="facts-list" role="list" className="divide-y divide-[var(--border)]/60">
            {filtered.map((row) => (
              <FactRow
                key={row.factId}
                fact={row}
                canEdit={canEdit}
                editing={editingId === row.factId}
                onStartEdit={() => setEditingId(row.factId)}
                onCommitEdit={(next) => {
                  setEditingId(null);
                  if (next && next !== row.text) onEdit(row.factId, next);
                }}
                onCancelEdit={() => setEditingId(null)}
                onRequestDelete={() => setPendingDelete(row)}
                redactionPolicy={redactionPolicy}
              />
            ))}
          </ul>
        )}

        {canEdit ? (
          <div className="pt-2">
            <Button variant="ghost" size="sm" onClick={onRequestAdd} data-testid="facts-add">
              <Plus className="size-3.5" />
              Add a fact
            </Button>
          </div>
        ) : null}
      </CardBody>

      <AlertDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => { if (!open) setPendingDelete(null); }}
        title="Delete this fact?"
        description={pendingDelete ? (
          <span data-testid="facts-delete-preview">
            &ldquo;{truncate(pendingDelete.text, 140)}&rdquo;
          </span>
        ) : undefined}
        confirmLabel="Delete"
        cancelLabel="Keep"
        onConfirm={() => {
          if (pendingDelete) onDelete(pendingDelete.factId);
          setPendingDelete(null);
        }}
      />
    </Card>
  );
}

interface FactRowProps {
  fact: WorkerFact;
  canEdit: boolean;
  editing: boolean;
  onStartEdit: () => void;
  onCommitEdit: (next: string) => void;
  onCancelEdit: () => void;
  onRequestDelete: () => void;
  redactionPolicy?: RedactionPolicy;
}

function FactRow({
  fact, canEdit, editing, onStartEdit, onCommitEdit, onCancelEdit,
  onRequestDelete, redactionPolicy,
}: FactRowProps) {
  const [draft, setDraft] = useState(fact.text);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => { if (!editing) setDraft(fact.text); }, [editing, fact.text]);
  useEffect(() => {
    if (editing && textareaRef.current) {
      textareaRef.current.focus();
      const len = textareaRef.current.value.length;
      textareaRef.current.setSelectionRange(len, len);
    }
  }, [editing]);

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Escape") { e.preventDefault(); onCancelEdit(); }
    else if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); onCommitEdit(draft.trim()); }
  };

  return (
    <li data-testid={`fact-row-${fact.factId}`} className="py-2">
      {editing ? (
        <textarea
          ref={textareaRef}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={() => onCommitEdit(draft.trim())}
          onKeyDown={onKeyDown}
          rows={3}
          data-testid={`fact-editor-${fact.factId}`}
          aria-label="Edit fact"
          className="w-full resize-none rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)] px-3 py-2 text-sm leading-relaxed text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
        />
      ) : (
        <div className="flex items-start justify-between gap-3">
          <p
            data-testid={`fact-text-${fact.factId}`}
            title={new Date(fact.createdAt).toLocaleString()}
            className="text-sm leading-relaxed text-[var(--foreground)]/90 whitespace-pre-wrap"
          >
            <RedactedText policy={redactionPolicy}>{fact.text}</RedactedText>
          </p>
          {canEdit ? (
            <div className="shrink-0 flex items-center gap-1">
              <RowIconButton onClick={onStartEdit} testId={`fact-edit-${fact.factId}`} label="Edit fact" tone="default">
                <Pencil className="size-3.5" />
              </RowIconButton>
              <RowIconButton onClick={onRequestDelete} testId={`fact-delete-${fact.factId}`} label="Delete fact" tone="destructive">
                <Trash2 className="size-3.5" />
              </RowIconButton>
            </div>
          ) : null}
        </div>
      )}
    </li>
  );
}

function RowIconButton({
  onClick, testId, label, tone, children,
}: {
  onClick: () => void; testId: string; label: string;
  tone: "default" | "destructive"; children: React.ReactNode;
}) {
  return (
    <button type="button" onClick={onClick} data-testid={testId} aria-label={label}
      className={cn(
        "rounded-[var(--radius-sm)] p-1 text-[var(--muted-foreground)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
        tone === "destructive"
          ? "hover:bg-[var(--destructive)]/10 hover:text-[var(--destructive)]"
          : "hover:bg-[var(--accent)]/40 hover:text-[var(--foreground)]")}>
      {children}
    </button>
  );
}

function EmptyState({ workerName }: { workerName: string }) {
  return (
    <div
      data-testid="facts-empty"
      className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] py-8 px-6 text-center space-y-1"
    >
      <p className="text-sm font-medium text-[var(--foreground)]">
        {workerName} is still learning about your business.
      </p>
      <p className="text-sm text-[var(--muted-foreground)]">
        Add what she should know above, or let her pick it up as she works.
      </p>
    </div>
  );
}

function ErrorBlock({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="space-y-2 text-sm text-[var(--muted-foreground)]">
      <p>{message}</p>
      {onRetry ? (
        <button type="button" onClick={onRetry} className="text-[var(--primary)] underline-offset-4 hover:underline">
          Retry
        </button>
      ) : null}
    </div>
  );
}

function truncate(s: string, max: number): string {
  return s.length <= max ? s : `${s.slice(0, max - 1)}…`;
}

export function MemoryFactsCardSkeleton() {
  return (
    <Card data-testid="card-memory-facts-skeleton">
      <CardHeader>
        <div className="h-4 w-40 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-2">
        {[0, 1, 2, 3].map((i) => (
          <div key={i} className="flex items-center gap-3">
            <div className="flex-1 h-3 rounded bg-[var(--muted)]" />
            <div className="h-3 w-6 rounded bg-[var(--muted)]" />
            <div className="h-3 w-6 rounded bg-[var(--muted)]" />
          </div>
        ))}
      </CardBody>
    </Card>
  );
}
