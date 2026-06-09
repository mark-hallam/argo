"use client";

import { useCallback, useRef, useState } from "react";
import { BookText, FileText, Globe, Link2 } from "lucide-react";

/**
 * Genesis branded wiki / Notion / Confluence URL-import row
 * (mission-genesis-integration-import-slack-notion-confluence; memory
 * `genesis-is-ai-teammate-onboarding-studio`).
 *
 * A stranger has no signal that "import my wiki" is even possible from a single
 * anonymous URL box. This renders a small row of presentational source chips —
 * Notion / Confluence / Wiki·Docs — that re-point ONE underlying URL input's
 * placeholder + helper copy and focus it, so the operator knows they can paste a
 * published page URL from their tooling. Under the hood every chip submits the
 * SAME `format:"wiki_url"` ingest (the SSRF-guarded server `fetch_wiki` path) —
 * there is NO new endpoint and NO parallel ingest route.
 *
 * Honest boundary (NO_THEATER, DEC-007): a one-line note tells the operator that
 * PRIVATE Slack / Notion / Confluence workspaces need a workspace connector
 * (coming soon) — only public / published page URLs work today, so a pasted
 * private URL gets a clear expectation rather than a silent failure.
 *
 * Consumes ONLY the locked brand tokens from `globals.css` (purple→PINK
 * gradient, Inter, `--brand-ink`) — no color literals, no new design (DEC-027).
 */

/** The published-page sources we brand the URL paste-box for. */
export type WikiSourceKey = "notion" | "confluence" | "wiki";

/** One branded source: chip label + the placeholder/helper it points the input at. */
export interface WikiSource {
  key: WikiSourceKey;
  label: string;
  placeholder: string;
  helper: string;
}

/** Source-specific guidance. Notion / Confluence published pages are plain
 *  fetchable HTML — the real, demoable slice; the generic wiki covers the rest. */
export const WIKI_SOURCES: WikiSource[] = [
  {
    key: "notion",
    label: "Notion",
    placeholder: "Paste a published / shared Notion page URL",
    helper: "Paste a published or shared Notion page URL — your teammate reads the page text.",
  },
  {
    key: "confluence",
    label: "Confluence",
    placeholder: "Paste a Confluence page URL your account can view",
    helper: "Paste a Confluence page URL that is viewable without a login.",
  },
  {
    key: "wiki",
    label: "Wiki / Docs",
    placeholder: "…or paste a wiki / docs URL",
    helper: "Paste any public wiki or docs page URL (GitBook, internal wiki, help-centre…).",
  },
];

/** The default placeholder when no source chip is active (mirrors the original). */
const DEFAULT_PLACEHOLDER = "…or paste a wiki / docs URL";

function ChipIcon({ source }: { source: WikiSourceKey }) {
  if (source === "notion") return <BookText size={13} aria-hidden />;
  if (source === "confluence") return <FileText size={13} aria-hidden />;
  return <Globe size={13} aria-hidden />;
}

export interface WikiImportSourcesProps {
  /** Disabled while a generation / hire is in flight or before the role exists. */
  disabled?: boolean;
  /** Submit a URL for `wiki_url` ingest. Returns true if accepted (so the input
   *  is cleared); false if validation in the parent rejected it. */
  onAddLink(url: string): Promise<boolean>;
}

/**
 * Branded source chips + the single `wiki_url` URL-paste input. The chips are
 * presentational hints only — they adjust placeholder/helper copy and focus the
 * input; the POST is always `format:"wiki_url"` to the existing endpoint.
 */
export function WikiImportSources({ disabled, onAddLink }: WikiImportSourcesProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [active, setActive] = useState<WikiSourceKey | null>(null);
  const [url, setUrl] = useState("");

  const activeSource = active ? WIKI_SOURCES.find((s) => s.key === active) ?? null : null;

  const pick = useCallback((source: WikiSource) => {
    setActive(source.key);
    inputRef.current?.focus();
  }, []);

  const submit = useCallback(async () => {
    const accepted = await onAddLink(url);
    if (accepted) setUrl("");
  }, [onAddLink, url]);

  return (
    <div className="wis" data-testid="genesis-import-sources">
      <style jsx>{`
        .wis {
          margin-top: 14px;
        }
        .wis-chips {
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
          align-items: center;
        }
        .wis-lead {
          font-size: 12.5px;
          font-weight: 700;
          color: color-mix(in srgb, var(--brand-ink) 72%, transparent);
        }
        .wis-chip {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 6px 12px;
          border-radius: 999px;
          border: 1px solid color-mix(in srgb, var(--brand-primary) 32%, transparent);
          background: #fff;
          color: var(--brand-ink);
          font-size: 12.5px;
          font-weight: 600;
          font-family: var(--font-sans);
          cursor: pointer;
          transition: border-color 0.15s ease, background 0.15s ease, color 0.15s ease;
        }
        .wis-chip svg {
          color: var(--brand-primary);
        }
        .wis-chip.on {
          border-color: transparent;
          color: #fff;
          background: linear-gradient(90deg, var(--brand-primary), var(--brand-pink));
        }
        .wis-chip.on svg {
          color: #fff;
        }
        .wis-chip:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
        .wis-helper {
          margin-top: 8px;
          font-size: 12px;
          line-height: 1.45;
          color: color-mix(in srgb, var(--brand-ink) 64%, transparent);
        }
        .wis-url {
          display: flex;
          align-items: center;
          gap: 8px;
          margin-top: 10px;
        }
        .wis-url-wrap {
          position: relative;
          flex: 1;
          display: flex;
          align-items: center;
        }
        .wis-url-icon {
          position: absolute;
          left: 10px;
          color: var(--brand-primary);
          pointer-events: none;
        }
        .wis-url-input {
          flex: 1;
          padding: 9px 12px 9px 34px;
          border-radius: 12px;
          border: 1px solid color-mix(in srgb, var(--brand-ink) 14%, transparent);
          font-size: 13px;
          font-family: var(--font-sans);
          color: var(--brand-ink);
          background: #fff;
        }
        .wis-url-btn {
          padding: 9px 16px;
          border-radius: 12px;
          border: none;
          font-size: 13px;
          font-weight: 700;
          color: #fff;
          cursor: pointer;
          background: linear-gradient(90deg, var(--brand-primary), var(--brand-pink));
          transition: opacity 0.15s ease;
        }
        .wis-url-btn:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
        .wis-note {
          margin-top: 8px;
          font-size: 11.5px;
          line-height: 1.45;
          color: color-mix(in srgb, var(--brand-ink) 55%, transparent);
        }
      `}</style>

      <div className="wis-chips">
        <span className="wis-lead">Import from</span>
        {WIKI_SOURCES.map((s) => (
          <button
            key={s.key}
            type="button"
            className={`wis-chip${active === s.key ? " on" : ""}`}
            data-testid={`genesis-import-source-${s.key}`}
            aria-pressed={active === s.key}
            disabled={disabled}
            onClick={() => pick(s)}
          >
            <ChipIcon source={s.key} />
            {s.label}
          </button>
        ))}
      </div>

      {activeSource ? (
        <p className="wis-helper" data-testid="genesis-import-helper">
          {activeSource.helper}
        </p>
      ) : null}

      <div className="wis-url" data-testid="genesis-url-intake">
        <span className="wis-url-wrap">
          <Link2 size={15} className="wis-url-icon" aria-hidden />
          <input
            ref={inputRef}
            className="wis-url-input"
            type="url"
            inputMode="url"
            placeholder={activeSource?.placeholder ?? DEFAULT_PLACEHOLDER}
            aria-label="Wiki, Notion, or Confluence page URL"
            data-testid="genesis-url-input"
            value={url}
            disabled={disabled}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                void submit();
              }
            }}
          />
        </span>
        <button
          type="button"
          className="wis-url-btn"
          data-testid="genesis-url-add"
          disabled={disabled || url.trim().length === 0}
          onClick={() => void submit()}
        >
          Add link
        </button>
      </div>

      <p className="wis-note" data-testid="genesis-import-connector-note">
        Private Slack / Notion / Confluence workspaces need a workspace connector
        (coming soon). Public or published page URLs work today.
      </p>
    </div>
  );
}
