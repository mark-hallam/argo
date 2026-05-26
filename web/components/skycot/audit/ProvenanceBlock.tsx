"use client";

import { useState } from "react";
import { Check, Copy } from "lucide-react";
import {
  chainAnchorFromProvenance,
  provenanceClipboardPayload,
  type AuditDetail,
  type ProvenanceBlockData,
} from "@/lib/types/audit-detail";

/**
 * Provenance card — renders the canonical model/prompt/skill/memory/policy
 * rows plus the optional audit-chain anchor returned by the gateway. A
 * "Copy hash chain" button serializes the canonical JSON payload to the
 * clipboard.
 *
 * Hash-prefix length is locked at 8 chars (4-byte hex). The full
 * value lives in the underlying clipboard payload — auditors and the
 * eventual E5-f verifier consume the full string.
 */

const SHORT_HASH_LEN = 8;
const CHAIN_ANCHOR_LEN = 12;

export interface ProvenanceBlockProps {
  /** The provenance subtree from `AuditDetail.provenance`. */
  data: ProvenanceBlockData;
  /** The full detail — used for the canonical clipboard chain payload. */
  detail: AuditDetail;
}

export function ProvenanceBlock({ data, detail }: ProvenanceBlockProps) {
  const rows = buildRows(data);
  return (
    <section
      data-testid="audit-provenance-card"
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 space-y-3"
    >
      <header>
        <h2 className="text-sm font-semibold text-[var(--foreground)]">
          Provenance & Chain
        </h2>
        <p className="text-xs text-[var(--muted-foreground)]">
          Hashes pinpoint the exact code, prompt, skill, memory, policy,
          and chain anchor that produced this action.
        </p>
      </header>

      <dl className="space-y-1.5">
        {rows.map((row) => (
          <ProvenanceRow key={row.key} row={row} />
        ))}
      </dl>

      <div className="border-t border-[var(--border)] pt-3">
        <CopyHashChainButton detail={detail} />
      </div>
    </section>
  );
}

interface ProvenanceRow {
  key: string;
  label: string;
  /** The canonical identifier (e.g. `claude-opus-4-7`). */
  id?: string;
  /** The version or full hash. */
  full: string;
  display?: string;
}

function buildRows(data: ProvenanceBlockData): ProvenanceRow[] {
  const chainAnchor = chainAnchorFromProvenance(data);
  return [
    { key: "model", label: "Model", id: data.model.id, full: data.model.version },
    { key: "prompt", label: "Prompt template", id: data.promptTemplate.id, full: data.promptTemplate.hash },
    { key: "skill", label: "Skill version", id: data.skill.id, full: data.skill.version },
    { key: "memory", label: "Memory snapshot", id: data.memorySnapshot.id, full: data.memorySnapshot.id },
    { key: "policy", label: "Policy version", id: data.policy.id, full: `${data.policy.version} · ${data.policy.hash}` },
    ...(chainAnchor
      ? [{
          key: "chain-anchor",
          label: "Chain anchor",
          full: chainAnchor,
          display: formatChainAnchor(chainAnchor),
        }]
      : []),
  ];
}

function ProvenanceRow({ row }: { row: ProvenanceRow }) {
  return (
    <div
      data-testid={`audit-provenance-row-${row.key}`}
      className="grid grid-cols-[120px_1fr_auto] items-center gap-3 rounded-[var(--radius-sm)] px-2 py-1 hover:bg-[var(--muted)]/30"
    >
      <dt className="text-xs uppercase tracking-wider text-[var(--muted-foreground)]">
        {row.label}
      </dt>
      <dd className="font-mono text-[12px] text-[var(--foreground)] truncate">
        {row.id ? (
          <>
            <span data-testid={`audit-provenance-id-${row.key}`}>{row.id}</span>
            <span aria-hidden className="mx-2 text-[var(--muted-foreground)]">·</span>
          </>
        ) : null}
        <span
          data-testid={`audit-provenance-hash-${row.key}`}
          title={`${row.full} — Click to copy`}
          className="text-[var(--muted-foreground)]"
        >
          {row.display ?? shortHash(row.full)}
        </span>
      </dd>
      <CopyButton
        value={row.full}
        testid={`audit-provenance-copy-${row.key}`}
        ariaLabel={`Copy ${row.label.toLowerCase()} value`}
      />
    </div>
  );
}

function CopyHashChainButton({ detail }: { detail: AuditDetail }) {
  const [copied, setCopied] = useState(false);
  const payload = provenanceClipboardPayload(detail);
  return (
    <button
      type="button"
      data-testid="audit-provenance-copy-chain"
      onClick={() => {
        const json = JSON.stringify(payload, null, 2);
        void copyToClipboard(json).then((ok) => {
          if (!ok) return;
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1500);
        });
      }}
      className="inline-flex w-full items-center justify-center gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
    >
      {copied ? <Check aria-hidden className="size-4 text-[var(--success)]" /> : <Copy aria-hidden className="size-4" />}
      {copied ? "Copied hash chain" : "Copy hash chain"}
    </button>
  );
}

function CopyButton({ value, testid, ariaLabel }: { value: string; testid: string; ariaLabel: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      data-testid={testid}
      aria-label={ariaLabel}
      onClick={() => {
        void copyToClipboard(value).then((ok) => {
          if (!ok) return;
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1500);
        });
      }}
      className="inline-flex size-7 items-center justify-center rounded-[var(--radius-sm)] text-[var(--muted-foreground)] hover:bg-[var(--muted)]/40 hover:text-[var(--foreground)]"
    >
      {copied ? <Check className="size-3.5 text-[var(--success)]" /> : <Copy className="size-3.5" />}
    </button>
  );
}

function shortHash(value: string): string {
  if (!value) return "";
  const hex = value.replace(/[^0-9a-fA-F]/g, "");
  const head = (hex.length >= SHORT_HASH_LEN ? hex.slice(0, SHORT_HASH_LEN) : value.slice(0, SHORT_HASH_LEN));
  return `${head}…`;
}

function formatChainAnchor(value: string): string {
  const body = value.startsWith("0x") ? value.slice(2) : value;
  if (body.length <= CHAIN_ANCHOR_LEN) return `0x${body}`;
  return `0x${body.slice(0, CHAIN_ANCHOR_LEN)}…`;
}

async function copyToClipboard(value: string): Promise<boolean> {
  try {
    if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(value);
      return true;
    }
  } catch {
    /* fall through */
  }
  return false;
}
