"use client";

import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import {
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ChangeEvent,
} from "react";
import { Search, X } from "lucide-react";
import {
  RETRO_SECTION_KEYS,
  RETRO_SECTION_LABELS,
  summarizeMetricsLine,
  type Retro,
  type RetroSectionKey,
} from "@/lib/api/retros";
import { useRetrosIndex } from "@/lib/hooks/use-retros";

const URL_DEBOUNCE_MS = 150;

export default function RetroIndexPage() {
  return (
    <Suspense fallback={<RetroIndexFallback />}>
      <RetroIndexContent />
    </Suspense>
  );
}

function RetroIndexFallback() {
  return (
    <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8">
      <h1 className="text-xl font-semibold tracking-tight">Retros</h1>
      <p className="mt-2 text-sm text-[var(--muted-foreground)]">Loading…</p>
    </div>
  );
}

function RetroIndexContent() {
  const router = useRouter();
  const pathname = usePathname() ?? "/retro";
  const searchParams = useSearchParams();

  const q = searchParams?.get("q")?.trim() ?? "";
  const [inputQuery, setInputQuery] = useState(q);

  useEffect(() => {
    setInputQuery(q);
  }, [q]);

  const updateQ = useCallback(
    (next: string) => {
      const params = new URLSearchParams(searchParams?.toString() ?? "");
      if (next.length > 0) params.set("q", next);
      else params.delete("q");
      const qs = params.toString();
      router.replace(qs ? `${pathname}?${qs}` : pathname, { scroll: false });
    },
    [pathname, router, searchParams],
  );

  useEffect(() => {
    const trimmed = inputQuery.trim();
    if (trimmed === q) return;
    const t = setTimeout(() => updateQ(trimmed), URL_DEBOUNCE_MS);
    return () => clearTimeout(t);
  }, [inputQuery, q, updateQ]);

  const onChange = (e: ChangeEvent<HTMLInputElement>) => setInputQuery(e.target.value);
  const onClear = () => {
    setInputQuery("");
    updateQ("");
  };

  const retros = useRetrosIndex(q);
  const items = useMemo(() => retros.data ?? [], [retros.data]);

  return (
    <div className="mx-auto max-w-5xl px-4 sm:px-6 lg:px-8 py-8">
      <header className="space-y-2">
        <h1 className="text-xl font-semibold tracking-tight text-[var(--foreground)]">
          Retros
        </h1>
        <p
          className="text-sm text-[var(--muted-foreground)]"
          data-testid="retro-index-subtitle"
        >
          {items.length} {items.length === 1 ? "retro" : "retros"}
          {q ? ` matching "${q}"` : ""} across the tenant
        </p>
      </header>

      <div className="mt-6 relative">
        <Search
          aria-hidden
          className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-[var(--muted-foreground)]"
        />
        <input
          type="text"
          value={inputQuery}
          onChange={onChange}
          placeholder="Search retro themes…"
          aria-label="Search retro themes"
          data-testid="retro-index-search"
          className="w-full h-10 pl-9 pr-9 rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--card)]/60 text-sm placeholder:text-[var(--muted-foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)] focus:border-transparent transition-shadow"
        />
        {inputQuery.length > 0 ? (
          <button
            type="button"
            onClick={onClear}
            aria-label="Clear search"
            data-testid="retro-index-search-clear"
            className="absolute right-2 top-1/2 -translate-y-1/2 grid size-6 place-items-center rounded-full text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--muted)] transition-colors"
          >
            <X className="size-3.5" />
          </button>
        ) : null}
      </div>

      <div className="mt-6">
        {retros.isPending ? (
          <p
            data-testid="retro-index-loading"
            className="text-sm text-[var(--muted-foreground)]"
          >
            Loading retros…
          </p>
        ) : retros.isError ? (
          <p
            data-testid="retro-index-error"
            className="text-sm text-[var(--destructive)]"
          >
            Couldn&apos;t load retros — try again.
          </p>
        ) : items.length === 0 ? (
          <EmptyState q={q} />
        ) : (
          <ul className="space-y-3" data-testid="retro-index-list">
            {items.map((r) => (
              <li key={r.retroId}>
                <RetroCard retro={r} highlight={q} />
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

function EmptyState({ q }: { q: string }) {
  return (
    <div
      data-testid="retro-index-empty"
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-12 px-6 text-center space-y-3"
    >
      <h2 className="text-base font-semibold text-[var(--foreground)]">
        {q ? "No retros match your search" : "No retros yet"}
      </h2>
      <p className="text-sm text-[var(--muted-foreground)]">
        {q
          ? "Try a broader theme — sections are searched case-insensitively."
          : "Run a retro from a worker profile after a long-running duty wraps."}
      </p>
    </div>
  );
}

function RetroCard({ retro, highlight }: { retro: Retro; highlight: string }) {
  const window = formatWindow(retro.windowStart, retro.windowEnd);
  const preview = previewSection(retro, highlight);
  return (
    <Link
      href={`/workers/${retro.workerId}#retro-${retro.retroId}`}
      data-testid={`retro-card-${retro.retroId}`}
      className="block rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-4 transition-colors hover:border-[var(--ring)]"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          <p className="text-sm font-medium text-[var(--foreground)]">
            Worker {retro.workerId.slice(0, 8)}…
          </p>
          <p className="text-xs text-[var(--muted-foreground)]">{window}</p>
        </div>
        <FinalizedBadge finalized={retro.finalized} />
      </div>
      <p className="mt-3 text-xs text-[var(--muted-foreground)]">
        {summarizeMetricsLine(retro.metrics)}
      </p>
      {preview ? (
        <p
          className="mt-3 line-clamp-3 text-sm text-[var(--foreground)]"
          data-testid={`retro-card-preview-${retro.retroId}`}
        >
          <span className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
            {RETRO_SECTION_LABELS[preview.key]}
          </span>
          <br />
          {preview.text}
        </p>
      ) : null}
    </Link>
  );
}

function FinalizedBadge({ finalized }: { finalized: boolean }) {
  if (finalized) {
    return (
      <span
        data-testid="retro-card-finalized"
        className="rounded-full bg-[var(--muted)] px-2 py-0.5 text-xs text-[var(--muted-foreground)]"
      >
        Finalised
      </span>
    );
  }
  return (
    <span
      data-testid="retro-card-draft"
      className="rounded-full bg-[var(--primary)]/10 px-2 py-0.5 text-xs text-[var(--primary)]"
    >
      Draft
    </span>
  );
}

function previewSection(
  retro: Retro,
  highlight: string,
): { key: RetroSectionKey; text: string } | null {
  const needle = highlight.trim().toLowerCase();
  if (needle) {
    for (const key of RETRO_SECTION_KEYS) {
      const text = retro.sections[key];
      if (text && text.toLowerCase().includes(needle)) {
        return { key, text };
      }
    }
  }
  for (const key of RETRO_SECTION_KEYS) {
    const text = retro.sections[key];
    if (text) return { key, text };
  }
  return null;
}

function formatWindow(startUnix: number, endUnix: number): string {
  const start = new Date(startUnix * 1000);
  const end = new Date(endUnix * 1000);
  const fmt: Intl.DateTimeFormatOptions = { month: "short", day: "numeric" };
  return `${start.toLocaleDateString(undefined, fmt)} → ${end.toLocaleDateString(undefined, fmt)}`;
}
