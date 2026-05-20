# web-audit-feed-per-row-hash — surface the hash chain on every audit row (Etherscan parity)

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-audit-feed-per-row-hash`.
**Priority**: P1 — wave-1 trust+safety gap. The "forensic-grade audit chain
on every action" marketing claim is invisible in the feed; demo
buyer-persona expects Etherscan-style hash on every row.
**Sizing**: ~250-350 LOC — ActivityRow extension + new `<HashCell>` primitive
+ audit type extension + sample-data deterministic hashes + tests + a small
SQL surface change so the gateway envelope carries hash on every row.

---

## Why this brief exists

From wave-1 trust+safety audit:

> Audit feed renders no hash per row (`web/components/skycot/ActivityRow.tsx`).
> The "forensic hash chain" claim is invisible until the buyer drills
> into a single action. Etherscan — SKYCOT's chosen metaphor — shows
> hash on every row. The drill-in (/audit/action/<id>) is excellent;
> the feed isn't.

Substrate that already exists:
- `audit_record.self_hash` and `audit_record.parent_hash` columns are
  populated for every row in the chain (see `crates/argo-audit/src/sink_chain.rs`).
- `argo-audit::export::ExportedRow` carries `self_hash: String` (hex-encoded).
- `/audit/action/<id>` page renders the full hash chain correctly.

What's missing:
- Gateway feed endpoint envelope doesn't include hash per row.
- `WorkerActivity` web type doesn't have a hash field.
- `ActivityRow` doesn't render hash.

---

## Pre-flight verification

```bash
# 1. Confirm hash chain is populated.
sqlite3 ~/.argo/store.db "SELECT id, hex(self_hash), hex(parent_hash) FROM audit_record LIMIT 3"

# 2. Confirm ActivityRow + WorkerActivity shape.
rg "interface WorkerActivity\b" web/lib/api/worker-activity.ts -A 10

# 3. Confirm audit feed gateway endpoint.
rg "/api/v1/audit\|fn audit_response" crates/argo-cli/src/handlers/gateway/ -l

# 4. Confirm /audit page does NOT yet render hash per row.
rg "self_hash\|chainHash\|0x" web/components/skycot/ActivityRow.tsx
```

---

## Phase 1 — Gateway: surface `chain_hash` in the audit feed envelope

**File**: `crates/argo-cli/src/handlers/gateway/routes_audit.rs` (or wherever
the audit-list endpoint lives — verify with `rg "/api/v1/audit " crates/`).

Add `self_hash` (already in the row) to the per-row JSON:

```diff
  json!({
      "id": row.id,
      "kind": row.kind_label,
      "actor": row.actor,
      "created_at": row.created_at_micros,
      "seq": row.seq,
+     "chain_hash": hex::encode(&row.self_hash),
+     "parent_hash": hex::encode(&row.parent_hash),
      "payload_summary": row.payload_summary,
      // ... existing fields ...
  })
```

`hex::encode` is already available via the `hex` workspace dep (used by
`argo-audit/src/export.rs`).

---

## Phase 2 — Web type extension

**File**: `web/lib/api/worker-activity.ts` (or sibling type def)

```diff
  export interface WorkerActivity {
    id: string;
    worker: { id: string; name: string; avatarUrl?: string };
    verb: string;
    timestamp: string;
    status: WorkerActivityStatus;
+   chainHash?: string;        // hex-encoded self_hash from audit_record.
+   parentHash?: string;       // hex-encoded parent_hash — optional, used in drill-in.
  }
```

**File**: `web/lib/api/audit.ts` — same field additions on `AuditRow` if
the type differs.

**File**: `web/lib/sample-audit.ts` — deterministic hashes:

```ts
import { sha256Hex } from "@/lib/utils/hash";   // or implement inline

for (const row of SAMPLE_AUDIT) {
  // Deterministic hash from (worker.id | verb | timestamp) so demo reproduces.
  row.chainHash = sha256Hex(`${row.worker.id}|${row.verb}|${row.timestamp}`);
}
```

If `sha256Hex` doesn't exist, add a tiny helper in `web/lib/utils/hash.ts`:

```ts
export async function sha256Hex(input: string): Promise<string> {
  const buf = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(input));
  return Array.from(new Uint8Array(buf)).map(b => b.toString(16).padStart(2, "0")).join("");
}

// Synchronous alternative for static sample data — non-cryptographic but
// deterministic (FNV-1a 32-bit) so the demo's sample rows don't need async:
export function fnv1aHex(input: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    h ^= input.charCodeAt(i);
    h = (h + ((h << 1) + (h << 4) + (h << 7) + (h << 8) + (h << 24))) >>> 0;
  }
  return "0x" + h.toString(16).padStart(8, "0") + "fake0000fake0000fake0000fake";
  // 32-bit fake hash padded to look like a full 64-char SHA-256 in dev.
}
```

(Sample data uses the synchronous FNV variant to keep seed module
top-level-synchronous. Prod uses the real `self_hash` from the gateway.)

---

## Phase 3 — `<HashCell>` primitive

**File**: `web/components/ui/HashCell.tsx` (new) — ~80 LOC.

```tsx
"use client";

import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { cn } from "@/lib/utils";
import { useToast } from "@/lib/ui/use-toast";

export interface HashCellProps {
  /** Full hex hash (with or without 0x prefix). */
  hash: string;
  /** Number of leading + trailing chars to keep around the ellipsis. */
  truncateTo?: { lead: number; tail: number };
  className?: string;
  /** When true, prefix with "0x". */
  withPrefix?: boolean;
}

export function HashCell({
  hash,
  truncateTo = { lead: 6, tail: 4 },
  className,
  withPrefix = true,
}: HashCellProps) {
  const [copied, setCopied] = useState(false);
  const { toast } = useToast();
  const normalized = hash.startsWith("0x") ? hash.slice(2) : hash;
  const display = `${withPrefix ? "0x" : ""}${normalized.slice(0, truncateTo.lead)}…${normalized.slice(-truncateTo.tail)}`;
  const fullDisplay = `${withPrefix ? "0x" : ""}${normalized}`;

  return (
    <button
      type="button"
      aria-label={`Audit chain hash ${fullDisplay}, click to copy`}
      title={`${fullDisplay}\nClick to copy`}
      onClick={async (e) => {
        e.preventDefault();
        e.stopPropagation();           // don't trigger parent <Link> navigation
        try {
          await navigator.clipboard.writeText(fullDisplay);
          setCopied(true);
          toast?.({ title: "Hash copied", durationMs: 1500 });
          setTimeout(() => setCopied(false), 1500);
        } catch {
          toast?.({ title: "Copy failed", variant: "destructive" });
        }
      }}
      className={cn(
        "inline-flex items-center gap-1 font-mono text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors",
        className,
      )}
      data-testid="hash-cell"
    >
      <span>{display}</span>
      {copied ? <Check size={11} aria-hidden /> : <Copy size={11} aria-hidden className="opacity-0 group-hover:opacity-100 transition-opacity" />}
    </button>
  );
}
```

---

## Phase 4 — Wire `<HashCell>` into `<ActivityRow>`

**File**: `web/components/skycot/ActivityRow.tsx`

The current grid template is `grid-cols-[auto_auto_1fr_auto_auto_auto]` — 6
columns. Add a new column for the hash, right-aligned, between the
status pill and the chevron:

```diff
  <Link
    href={href}
    role="listitem"
    ...
    className={cn(
-     "group grid grid-cols-[auto_auto_1fr_auto_auto_auto] items-center gap-3 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2.5",
+     "group grid grid-cols-[auto_auto_1fr_auto_auto_auto_auto] items-center gap-3 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2.5",
      className,
    )}
  >
    {/* col 1: avatar */}
    {/* col 2: worker name */}
    {/* col 3: verb (1fr) */}
    {/* col 4: status pill */}
    {/* col 5: timestamp */}
+   {/* col 6: hash cell (NEW) */}
+   {activity.chainHash && (
+     <HashCell hash={activity.chainHash} className="ml-auto" />
+   )}
    {/* col 7: chevron arrow */}
  </Link>
```

The `activity.chainHash &&` guard handles sample rows during the rollout
period when not every row has a hash yet — graceful degradation.

---

## Phase 5 — Tests

**File**: `web/components/ui/HashCell.test.tsx` (new):
1. `renders_truncated_hash_with_default_lead_and_tail`
2. `respects_custom_truncate_options`
3. `click_copies_full_hash_to_clipboard`
4. `prevents_parent_link_navigation_on_click`
5. `aria_label_includes_full_hash`
6. `keyboard_activation_via_enter_copies`

**File**: `web/components/skycot/ActivityRow.test.tsx` (extend):
7. `renders_hash_cell_when_chainhash_present`
8. `does_not_render_hash_cell_when_chainhash_absent`
9. `clicking_hash_does_not_navigate_to_drill_in`

---

## Phase 6 — Visual checks against Etherscan parity

Etherscan reference (open in browser, screenshot for reference):
`https://etherscan.io/txs` — observe:
- Hash column is right-aligned-ish (or column 3 of N).
- Monospace font, neutral gray.
- Hover reveals full hash + copy icon.
- Click = copy, NOT navigate.

Our implementation matches the click-to-copy + monospace + hover-reveal
patterns. **Don't add an external link icon** — clicking the hash in our
UI copies; clicking elsewhere on the row navigates to drill-in.

---

## may-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit.rs` — add `chain_hash` + `parent_hash` to envelope.
- `web/lib/api/worker-activity.ts` — add `chainHash` / `parentHash` to `WorkerActivity`.
- `web/lib/api/audit.ts` — same on `AuditRow` (if a separate type).
- `web/lib/sample-audit.ts` — populate deterministic hashes on seed rows.
- `web/lib/utils/hash.ts` (new) — `fnv1aHex` + `sha256Hex` helpers.
- `web/components/ui/HashCell.tsx` (new) + `HashCell.test.tsx`.
- `web/components/skycot/ActivityRow.tsx` — add the hash column + use `<HashCell>`.
- `web/components/skycot/ActivityRow.test.tsx` — extend with hash assertions.

## must-NOT-touch

- `web/app/(app)/audit/action/[id]/page.tsx` — drill-in already shows the chain; out of scope.
- `crates/argo-audit/src/sink_chain.rs` — hash-chain compute is IMMUTABLE.
- Approvals page — separate concern.
- The Link href / navigation logic — only intercept the hash cell click, not the row.

---

## Acceptance criteria

- [ ] Every audit row in `/audit` renders a truncated hash like `0xabc123…f9e8`.
- [ ] Clicking the hash copies the FULL value (64 hex chars + `0x`) to clipboard.
- [ ] Clicking the hash does NOT navigate to drill-in (verified by E2E spec).
- [ ] Hover tooltip shows full hash + "Click to copy".
- [ ] `aria-label` includes the full hash for screen readers.
- [ ] Sample-data hashes are deterministic: same row → same hash across reloads.
- [ ] Gateway envelope includes `chain_hash` + `parent_hash` for every row.
- [ ] When `chainHash` is absent on a row (graceful degradation), the column slot is empty — not broken.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] Etherscan-parity rubric: re-audit moves from ~4 to 9.
- [ ] Demo MP4 segment: 2s pause on `/audit`, click any hash, "Copied" toast pops — verifies the viral microinteraction.

---

## Gotchas

1. **`e.stopPropagation()` is required** — without it, the `<Link>` wrapper
   navigates to drill-in on every hash click. The brief's HashCell does
   this; preserve it.
2. **`navigator.clipboard` requires HTTPS or localhost** — works fine in
   dev (localhost) + prod (HTTPS). HTTP-on-LAN demos fail silently. The
   toast catches the failure and shows "Copy failed."
3. **The hash column squeezes the grid** on narrow viewports — at < 800px,
   hide the hash column with `hidden md:inline-flex` on the HashCell.
   Mobile users see no hash; tablet+ does.
4. **Deterministic seed hashes** matter for the demo MP4: re-recording
   the screen capture should show the same hashes as the prior take, so
   the "look at this hash" callout doesn't shift. The `fnv1aHex` helper
   keeps it deterministic.
5. **Truncation defaults `{ lead: 6, tail: 4 }`** = "0xabc123…f9e8" = 11
   visible chars. Etherscan uses 6+4 too. Don't go shorter — it stops
   being recognizable as a hash.
6. **Monospace font**: the existing `font-mono` Tailwind class maps to
   the project's chosen mono (JetBrains Mono per brand guide). Don't
   override with a different mono — brand consistency.

---

## References

- Audit: `docs/skycot/audit/trust-safety/2026-05-20-wave-1-trust-safety-audit.md`
- Hash chain substrate: `crates/argo-audit/src/sink_chain.rs`
- Hex encode pattern: `crates/argo-audit/src/export.rs` (`hex::encode(&self_hash)`)
- Drill-in (out of scope but reference): `web/app/(app)/audit/action/[id]/page.tsx`
- Viral microinteraction note: "the action-id copy-on-hover microinteraction in the audit feed is a viral sleeper, worth a 2s pause in the demo MP4." (wave-5 viral-moments audit)
