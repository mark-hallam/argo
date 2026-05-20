"use client";

import { useState } from "react";
import { CircleSlash, KeyRound, Plus, Trash2 } from "lucide-react";
import {
  useApiKeys,
  useCreateApiKey,
  useRevokeApiKey,
} from "@/lib/hooks/use-api-keys";
import type { ApiKeyRow, CreateApiKeyResponse } from "@/lib/api/api-keys";
import { ApiKeysStyles } from "./styles";
import { CreateModal } from "./create-modal";
import { RevealPanel, RevokeDialog } from "./reveal-panel";

/**
 * `/settings/api-keys` — long-lived bearer tokens for service accounts,
 * CI, and integrations.
 *
 * Brand-restrained register: Inter typography, gradient only on the
 * primary "Create API key" pill. Reveal-once flow guards the full token;
 * once dismissed only the masked prefix is ever shown again.
 */

function relativeTime(iso: string | null, fallback = "Never"): string {
  if (!iso) return fallback;
  const then = new Date(iso).getTime();
  const now = Date.now();
  const diff = Math.max(0, now - then);
  const m = 60_000;
  const h = 60 * m;
  const d = 24 * h;
  if (diff < m) return "just now";
  if (diff < h) return `${Math.floor(diff / m)} min ago`;
  if (diff < d) return `${Math.floor(diff / h)} h ago`;
  if (diff < 30 * d) return `${Math.floor(diff / d)} d ago`;
  return new Date(iso).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function expiryLabel(iso: string | null): {
  label: string;
  tone: "neutral" | "warn" | "stale";
} {
  if (!iso) return { label: "Never", tone: "neutral" };
  const then = new Date(iso).getTime();
  const now = Date.now();
  if (then < now) return { label: "Expired", tone: "stale" };
  const diffDays = Math.ceil((then - now) / (24 * 60 * 60_000));
  const date = new Date(iso).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
  if (diffDays <= 14) return { label: `${date} (${diffDays}d)`, tone: "warn" };
  return { label: date, tone: "neutral" };
}

export default function ApiKeysPage() {
  const query = useApiKeys();
  const create = useCreateApiKey();
  const revoke = useRevokeApiKey();

  const [showCreate, setShowCreate] = useState(false);
  const [reveal, setReveal] = useState<CreateApiKeyResponse | null>(null);
  const [confirmCloseReveal, setConfirmCloseReveal] = useState(false);
  const [confirmRevoke, setConfirmRevoke] = useState<ApiKeyRow | null>(null);

  const keys = query.data?.keys ?? [];
  const isEmpty = !query.isLoading && keys.length === 0;

  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&display=swap"
        rel="stylesheet"
      />
      <ApiKeysStyles />

      <div className="api-keys-stage">
        <div className="ak-header">
          <div>
            <div className="ak-eyebrow">⚙ WORKSPACE SETTINGS</div>
            <h1 className="ak-title">API keys</h1>
            <p className="ak-sub">
              Long-lived tokens for service accounts, CI, and integrations.
            </p>
          </div>
          <button type="button" className="ak-create-btn" onClick={() => setShowCreate(true)}>
            <Plus size={15} /> Create API key
          </button>
        </div>

        {isEmpty ? (
          <EmptyState onCreate={() => setShowCreate(true)} />
        ) : (
          <KeysTable
            keys={keys}
            isLoading={query.isLoading}
            onRevoke={(k) => setConfirmRevoke(k)}
          />
        )}
      </div>

      {showCreate ? (
        <CreateModal
          onCancel={() => setShowCreate(false)}
          onSubmit={(req) =>
            create.mutate(req, {
              onSuccess: (resp) => {
                setShowCreate(false);
                setReveal(resp);
              },
            })
          }
          isSubmitting={create.isPending}
          submitError={create.error?.message ?? null}
        />
      ) : null}

      {reveal ? (
        <RevealPanel
          response={reveal}
          confirmingClose={confirmCloseReveal}
          onRequestClose={() => setConfirmCloseReveal(true)}
          onCancelClose={() => setConfirmCloseReveal(false)}
          onConfirmClose={() => {
            setConfirmCloseReveal(false);
            setReveal(null);
          }}
        />
      ) : null}

      {confirmRevoke ? (
        <RevokeDialog
          row={confirmRevoke}
          isPending={revoke.isPending}
          onCancel={() => setConfirmRevoke(null)}
          onConfirm={() => {
            revoke.mutate(confirmRevoke.id, {
              onSettled: () => setConfirmRevoke(null),
            });
          }}
        />
      ) : null}
    </>
  );
}

function KeysTable({
  keys,
  isLoading,
  onRevoke,
}: {
  keys: ApiKeyRow[];
  isLoading: boolean;
  onRevoke: (k: ApiKeyRow) => void;
}) {
  if (isLoading) {
    return (
      <div className="ak-list" role="status" aria-live="polite">
        <div className="ak-empty-blurb">Loading keys…</div>
      </div>
    );
  }
  return (
    <div className="ak-list">
      <div className="ak-list-head">
        <span>Name</span>
        <span>Prefix</span>
        <span>Scopes</span>
        <span>Created</span>
        <span>Last used</span>
        <span>Expires</span>
        <span />
      </div>
      {keys.map((k) => (
        <KeyRow key={k.id} row={k} onRevoke={onRevoke} />
      ))}
    </div>
  );
}

function KeyRow({ row, onRevoke }: { row: ApiKeyRow; onRevoke: (k: ApiKeyRow) => void }) {
  const expiry = expiryLabel(row.expires_at);
  const cohortLabel = row.all_cohorts
    ? "All cohorts"
    : row.cohort_ids.length === 0
      ? "No cohorts"
      : `${row.cohort_ids.length} cohort${row.cohort_ids.length === 1 ? "" : "s"}`;
  return (
    <div className={`ak-row ${row.revoked ? "ak-row-revoked" : ""}`}>
      <div className="ak-row-name">
        <span className="ak-row-name-text">{row.name}</span>
        {row.revoked ? (
          <span className="ak-pill ak-pill-revoked">
            <CircleSlash size={11} /> Revoked
          </span>
        ) : null}
      </div>
      <code className="ak-row-prefix">{row.prefix}</code>
      <div className="ak-row-scopes">
        <span className="ak-pill ak-pill-cohort">{cohortLabel}</span>
        {row.scopes.slice(0, 2).map((s) => (
          <span key={s} className="ak-pill ak-pill-scope">
            {s}
          </span>
        ))}
        {row.scopes.length > 2 ? (
          <span className="ak-pill ak-pill-scope">+{row.scopes.length - 2}</span>
        ) : null}
      </div>
      <div className="ak-row-time">{relativeTime(row.created_at)}</div>
      <div className="ak-row-time">{relativeTime(row.last_used_at, "Never")}</div>
      <div className={`ak-row-time ak-row-expires-${expiry.tone}`}>{expiry.label}</div>
      <div className="ak-row-actions">
        {row.revoked ? null : (
          <button
            type="button"
            className="ak-revoke-btn"
            aria-label={`Revoke ${row.name}`}
            onClick={() => onRevoke(row)}
          >
            <Trash2 size={14} /> Revoke
          </button>
        )}
      </div>
    </div>
  );
}

function EmptyState({ onCreate }: { onCreate: () => void }) {
  return (
    <div className="ak-empty">
      <div className="ak-empty-icon">
        <KeyRound size={32} />
      </div>
      <h2 className="ak-empty-title">No API keys yet.</h2>
      <p className="ak-empty-blurb">
        Mint one to wire SKYCOT into your CI pipeline, data warehouse sync, or any integration that
        needs to call the API without a human session.
      </p>
      <button type="button" className="ak-create-btn" onClick={onCreate}>
        <Plus size={15} /> Create your first key
      </button>
    </div>
  );
}
