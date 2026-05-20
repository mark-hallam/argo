# Wave-13 NL-Workflow-Editing Deep Audit — 2026-05-21

**Scope.** The natural-language editing surface at
`/workflows/<verticalId>/<name>/build`. This is the SKYCOT competitive
moat — n8n / UIpath have no NL editing; Replit Agent has no visual
canvas. Audit the full UX as it stands on `origin/main` today, what
the open PR #1882 (`claude-web-workflow-builder-ai-prompt-wired`)
adds, and whether the demo voiceover can talk about this honestly.

**Files audited.**
- `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — main, 1120 LOC
- `~/code/argo/web/app/api/workflows/propose/route.ts` — propose route, 153 LOC
- `~/code/argo/skills/library/propose_workflow_from_plain_english.skill.md` — skill manifest
- `~/code/argo/web/components/skycot/ProposeWorkflowChat.tsx` — sibling that already does this correctly (`/workers/<id>/responsibilities/new`)
- PR branch `origin/claude-web-workflow-builder-ai-prompt-wired` (PR #1882, 1230 +/ 33 -)
- `streams/web-workflow-builder-ai-prompt-wired.md` — brief
- `docs/skycot/audit/action-fires/2026-05-21-wave-6-action-fires-correctly.md` — prior wave

---

## 1. Prompt input — UX check (current main)

| Property | State on main | Verdict |
|---|---|---|
| Element | Single-line `<input type="text">` (line 967) | **GAP — should be `<textarea>` with autosize**. Workflow descriptions ("every Friday gather the customer churn report, cluster by reason code, draft a summary, post in #revenue, page Sam if churn > 5%") run 30-80 words. A single-line input chops them at the right edge. Sibling `ProposeWorkflowChat` correctly uses `<textarea>`. |
| Placeholder example | "Describe a change in plain English — e.g. 'Add a step that emails the seller before posting'" | OK — concrete enough |
| Multi-line | Not supported (Enter submits, no Shift+Enter newline) | **GAP** — paragraph-long prompts impossible |
| Autosize | n/a (single-line input) | **GAP** |
| Disabled while proposing | NO on main; YES on PR #1882 branch | minor — fixes in PR |
| `aria-label` | Missing on main; added in PR #1882 (`aria-label="Describe a change in plain English"`) | PR #1882 fixes |

## 2. Send / Propose button

| Property | State on main | Verdict |
|---|---|---|
| Disabled when input empty | YES (`!promptText.trim() || proposing`) | OK |
| Loading state | "Proposing…" label while `proposing` is true | OK |
| Loading state during real network | n/a on main (stub setTimeout); PR #1882 wires real fetch | PR #1882 OK |
| Keyboard submit | Enter key triggers `handleProposeChange()` | OK |
| Test id | `data-testid="builder-propose-submit"` on PR #1882; missing on main | minor |

## 3. Suggestion chips ("Add a step" / "Remove gate")

| Property | State on main | Verdict |
|---|---|---|
| Chip 1 label | "Add a step" → fills `"Add a step that emails the seller before posting"` | OK — readable label, populates input |
| Chip 2 label | "Remove gate" → fills `"Convert this to fully autonomous (no approval needed)"` | OK |
| Behavior | `setPromptText(...)` only — does **not** auto-submit | **MIXED**. Operator must then click Propose. PR #1882 changes this to auto-submit via `handleSuggestion` (fills input + calls `submitProposal` immediately). That matches what a user expects from a "try this" chip but is debatable UX (some users want to edit before submitting). Brief asked for auto-submit; PR delivers it. |
| Number of chips | 2 | **GAP** — at least 4-5 would be valuable for a "differentiator" surface. Suggestion: "Add an approval before step N", "Move step X after step Y", "Add a Slack notify on failure", "Branch on error". |
| Truncation | Chip text fits on one line at 1280px viewport | OK |
| Disabled while proposing | NO on main; YES on PR #1882 | minor |

## 4. Toast behavior post wave-6 softening

**Wave-6 softening verified.** On main (lines 371-392), the handler is now:

- Step 1: `toast.loading("Drafting: \"<prompt>\"", { description: "Live AI workflow editing — wired in the next deploy." })`
- Step 2 (after 1.4s): `toast.message("Preview build — draft not persisted", { id: dismissId, description: "Live wire ships in PR #1882. Canvas changes will land then." })`

This is **substantially honest** — no longer claims "1 step change proposed. Approve to apply to the canvas" (the prior lie wave-6 caught). It tells the operator the wire is the next deploy and the canvas didn't change.

**However, two soft-lies remain.**

1. **"Live wire ships in PR #1882"** — leaks an internal brief/PR identifier in operator-facing copy. A buyer who sees the toast learns this is unfinished SKYCOT-internal scaffolding. Should read: "Live AI workflow editing is in the next deploy. This preview builds from a stored draft."
2. The toast **fires regardless of network connectivity, gateway state, or input quality**. It always says the same thing. A buyer who pastes garbage gets the same "draft not persisted" response as a buyer with a perfect prompt — which betrays the stub. The PR branch fixes this by actually streaming and rendering a diff.

Similarly, **`handleTestRun`** (line 394-402) and **`handleSaveAndPublish`** (line 404-412) softened to "Real run streams from the gateway in v1.x" and "Publish to tenant ships in v1.x. Draft is in-memory only here" — both honest, but **the Save+Publish copy still leaks "v1.x" jargon** that a buyer wouldn't parse.

## 5. /api/workflows/propose endpoint — what it actually returns today

The Next.js route at `web/app/api/workflows/propose/route.ts` is **real and useful**:

1. **Validates input** — JSON body, requires `description` (≤ 4000 chars) and `workerId`; returns 400 with a typed `{ error: { code, message } }` if missing.
2. **Authenticates** — `await auth()` reads the NextAuth session; if there's a `gatewayToken`, attempts to stream from `POST {GATEWAY}/api/v1/skills/propose_workflow_from_plain_english/invoke/stream` with `Bearer <token>`.
3. **Streams from gateway when reachable** — passes the upstream `ReadableStream` straight through with `X-Workflow-Source: gateway`.
4. **Falls back to mock TOML in dev** — if `NODE_ENV != production` or gateway unreachable, generates a deterministic TOML stub (`mockToml`) from the description: a header `[workflow] name = "<snake_case_of_first_line>"` + two `[[step]]`s (`describe_intent` + `operator_review`), chunk-streamed at 28ms per 48-char chunk to mimic streaming UX.
5. **Returns 502 in production if gateway unreachable** — `gateway_unreachable` error.

**Verdict on the stub quality post-wave-6 softening.** The **route stub is solid** — it has session auth, real validation, real streaming, real fallback. The bug is that the **builder page does not call it**. Both stubs are technically honest, but they live in two different files at two different levels of completeness. The route is ready for the live wire; the page is not — yet.

**Critical:** The gateway-side skill `propose_workflow_from_plain_english` has a `.skill.md` manifest (`skills/library/propose_workflow_from_plain_english.skill.md`) but **no gateway invoke route**. `grep -rn "skills/.*invoke/stream" crates/` returns zero results — the route the proxy POSTs to (`/api/v1/skills/{skill}/invoke/stream`) is not implemented in `argo-cli/src/handlers/gateway/`. Skills currently expose `/ir`, `/quality`, `/show`, `/post-writes` and dispatch-post routes, but no `/invoke/stream` for ad-hoc skill execution. So even after PR #1882 merges, **the in-prod path against a real gateway will 404 → fall through to 502** unless dev mode is on. This is a **second ship-blocker behind PR #1882**.

## 6. PR #1882 status

**State:** OPEN, IN_REVIEW. Authored 2026-05-20, last touched 2026-05-20. `mergeable: UNKNOWN`. Branch is `claude-web-workflow-builder-ai-prompt-wired`, head `79c8db03`.

**Checks: 14 of 15 FAILURE, 1 SKIPPED** (coverage). The failing checks include `fmt`, `clippy`, `loc-budget`, `test (ubuntu-latest/macos-latest/windows-latest)`, `e2e-onboarding`, plus 7 Rust-side gates. This is **not** the brief's substantive fault — the additions are 1230 web-only lines (+33 deletions), but CI is gating on Rust workspace state at the time of run (likely staleness vs. main, since the brief touched no Rust files). The ledger row reads "REBASED post-#1885 2026-05-20 — PR now MERGEABLE" so the rebase was done, but CI hasn't re-run cleanly. **Next action: re-trigger CI.**

**Canvas-update logic — IS the branch ready?**

YES, fully implemented:

- `parseProposalToml.ts` (131 LOC) — minimal tolerant TOML parser that handles partial mid-stream input + a `diffProposalSteps()` that returns `{ added, changed, removed }` sets keyed by step name.
- `page.tsx` (+769, -33) — `WorkflowBuilderInner` (now exported for tests) holds `currentSteps` state, a `ProposalStatus` finite state machine (`idle | streaming | ready | editing | submitting | error`), and:
  - `submitProposal(description)` — fetches `/api/workflows/propose`, reads the stream with `ReadableStreamDefaultReader`, accumulates into `proposalToml`, updates state to `ready` on completion.
  - `ProposalPanel` (new component, ~270 LOC) — 2-column panel showing streamed TOML on the left + diff summary + per-step diff list + Approve / Reject / Edit / Retry CTAs on the right.
  - Canvas re-renders with `diffNodesToFlow` overlay — added nodes get a green badge, changed nodes get amber, removed nodes get red, with edges colored to match.
  - `handleApprove` — POSTs `application/toml` body to `/api/v1/verticals/<v>/workflows/<workflow_name>`, swaps `currentSteps` to the proposed list, flashes a success "Saved <name>. Canvas updated." toast (real, not lie).
  - `handleReject` — aborts the in-flight fetch + resets state.
  - `handleEdit` — drops the streamed TOML into a textarea for manual tweaks before approving.
  - `handleRetry` — re-runs `submitProposal(promptText)` on the original prompt.

**The wire is complete.** What's missing is (a) green CI and (b) the gateway-side skill invoke route.

## 7. Side panel for an existing node — Edit copy + Save propagation

**Click flow.** Click any canvas node → `setSelectedNode(node)` opens a 360px right-side panel with the step's name, description, INPUTS code block, OUTPUTS code block, APPROVAL GATE banner (if any), and a primary CTA.

**CTA label is dynamic.** Line 1099: `{selectedIsTrigger ? "Configure trigger" : "Edit step"}`.

**Critical bug — "Edit step" is silent-broken for non-trigger nodes.** Line 1080: `onClick={() => setTriggerConfigOpen(true)}` — but the drawer that opens is gated by `selectedIsTrigger && triggerConfigOpen` (line 1103). So:

- Click a **trigger node** → "Configure trigger" button → `TriggerConfigDrawer` opens. **OK.**
- Click a **skill / approval / tool_call / notify / decision / data node** (the majority) → "Edit step" button → state flips, nothing visible happens. **SILENT-BROKEN.** The seller-approval step, the BANT-score skill, the Marketplace tool_call — none can be edited.

This is **worse than wave-6 thought**. Wave-6's matrix recorded the row as "**LOUDLY-BROKEN** — no onClick" but actually it has an onClick that just leads to a void. Post-wave-6, the on-click was added (so it's no longer LOUDLY-broken) but the underlying drawer was only built for triggers, so it became SILENT-broken instead.

**Save propagation.** n/a — no save path exists for non-trigger steps. For triggers, `TriggerConfigDrawer.onSave` just closes the drawer; `triggerConfigs` is stored in client state only with no gateway POST.

---

## 250-word summary

**NL editing UX score: 6/10 on main, 8/10 with PR #1882 merged + green.** The prompt bar, suggestion chips, and the propose route stub are competent. The two missing capabilities — multi-line `<textarea>` and a non-trigger step editor — are the difference between "demo-pretty" and "buyer-actually-uses-this."

**Voiceover honesty (post-wave-6 softening): mostly safe.** The Propose / Test-run / Save+Publish toasts no longer lie about persistence. The remaining risks: (a) the toast leaks internal identifiers ("PR #1882", "v1.x") that look unfinished on camera; (b) the toast is identical regardless of input quality. Acceptable for the demo as long as the voiceover doesn't say "watch the canvas update" — it won't until #1882 lands.

**Top 3 NL gaps for the live wire:**
1. **Gateway-side skill `/invoke/stream` route is missing.** PR #1882 calls `POST /api/v1/skills/propose_workflow_from_plain_english/invoke/stream` but `grep crates/` for `invoke/stream` returns zero hits. Even after the web merge, production calls fall through to the 502 path. Author a follow-up brief: `gateway-skill-invoke-stream-route`.
2. **`<input>` should be `<textarea>` with autosize.** Workflow descriptions are paragraphs, not titles. Sibling `ProposeWorkflowChat` already does this correctly.
3. **"Edit step" button is silent-broken for 6 of 7 step kinds.** Only trigger nodes open a drawer; everything else is a void onClick. Either wire a generic step editor or change the label to "Configure" + only show on triggers.

**Verdict on Propose stub quality post-wave-6 softening: CREDIBLE.** Honest in spirit, leaks two internal identifiers, fires identical regardless of input — fine for the demo voiceover, not fine for buyer hands-on. PR #1882 closes the gap.
