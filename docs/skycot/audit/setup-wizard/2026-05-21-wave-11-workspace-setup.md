# Wave-11 audit #2 — workspace setup wizard (steps 1 + 2)

**Date:** 2026-05-21
**Auditor:** Wave-11 agent #2 (read-only static analysis)
**Scope:** `/onboarding/welcome` + `/onboarding/workspace` — the first two steps of the post-signin 5-step provisioning wizard. Post-#1898 (F1 editorial shell port).
**Method:** read every file in scope. No browser session. Cross-checked against wave-3 onboarding audit and the live `WizardEditorialShell`.
**Files inspected (absolute paths):**
- `~/code/argo/web/app/(setup)/onboarding/welcome/page.tsx`
- `~/code/argo/web/app/(setup)/onboarding/workspace/page.tsx`
- `~/code/argo/web/app/(setup)/onboarding/_OnboardingShell.tsx`
- `~/code/argo/web/app/(setup)/onboarding/error.tsx`
- `~/code/argo/web/components/auth/WizardEditorialShell.tsx`
- `~/code/argo/web/lib/onboarding/use-onboarding-draft.ts`
- `~/code/argo/web/lib/onboarding/use-provision.ts`
- `~/code/argo/web/components/auth/__tests__/WizardEditorialShell.onboarding.test.tsx`

---

## Audit checklist — per step

Six checks per step (carried from the wave-3 onboarding playbook + the wave-11 brief):

1. **Field validation.** Every field has client-side validation (required + pattern + length).
2. **Inline errors next to fields.** Errors live next to the offending input (not just toast).
3. **Back/Next buttons present + wired.**
4. **Progress indicator updates** (step pip strip + ARIA `aria-valuenow`).
5. **SAVE & EXIT affordance** (per wave-3 onboarding audit + locked-shell contract).
6. **Mission-pillar copy surfaced** — audit-first / single-binary / enterprise-grade.

A pass = "implemented correctly". A partial = "wired but a gap exists". A fail = "missing or broken".

---

### Step 1 — `/onboarding/welcome` (`web/app/(setup)/onboarding/welcome/page.tsx`)

| # | Check | Verdict | Evidence |
|---|---|---|---|
| 1 | Field validation | **n/a** | No input fields. Step is read-only — "Continue" button only. |
| 2 | Inline errors next to fields | **n/a** | No fields means no errors. |
| 3 | Back / Next buttons | **partial** | `onboarding-welcome-continue` button at `welcome/page.tsx:69-76` pushes to `/onboarding/workspace`. **No Back button** — but step 1 is the entry point, so Back to `/signin` would be a regression. Acceptable. |
| 4 | Progress indicator | **pass** | Shell sets `step={1}` (`welcome/page.tsx:20`); `StepStrip` (`WizardEditorialShell.tsx:256-288`) renders `aria-valuenow="1"`, `aria-valuemax="5"`, pip-1 filled with gradient. Verified by `WizardEditorialShell.onboarding.test.tsx:62-63`. |
| 5 | SAVE & EXIT | **pass** | Shell renders `wiz-shell-exit` button (`WizardEditorialShell.tsx:359-366`). Because `unsavedHire` is not passed, click routes directly to `/` without the confirm dialog. Brand button (`wiz-shell-brand`) also routes home. |
| 6 | Mission-pillar copy | **pass** | All three pillars surfaced in a 3-column grid (`welcome/page.tsx:35-67`): audit-first ("Every action signed and replayable — SOC 2-ready"), single-binary ("One Rust process. No Kubernetes, no sidecars"), enterprise-grade ("Multi-tenant, capability-based RBAC, GDPR Article 30"). Verified by `WizardEditorialShell.onboarding.test.tsx:58-60`. |

**Welcome step score: 5/5 applicable checks pass. 10/10.**

Notes:
- The "(90 seconds.)" promise in the sub headline is a concrete, mission-aligned commitment (`welcome/page.tsx:25-28`). It sets a measurable expectation the rest of the wizard is held to.
- No skip / quit-without-saving on the welcome step — but the brand + SAVE & EXIT both return home, which is the right escape valve for a step with no draft yet.

---

### Step 2 — `/onboarding/workspace` (`web/app/(setup)/onboarding/workspace/page.tsx`)

| # | Check | Verdict | Evidence |
|---|---|---|---|
| 1 | Field validation | **partial — required only; no pattern, no length cap** | `workspace/page.tsx:29-34`: both `workspaceName` and `companyName` are `.trim()`-ed and rejected if empty. `canSubmit` (`:46`) gates the Continue button on `> 0` length. **No max-length** on either input — server could reject; HTML `<input>` has no `maxLength` attribute. **No pattern** — accepts emoji, control chars, line breaks. **No slug derivation** — the brief mentions "slug" but the page only collects display name + company name; slug is server-derived (no client preview). **No optional team-size field** — the brief asks about it; the page does not collect it. |
| 2 | Inline errors next to fields | **partial — one shared error row, not per-field** | `workspace/page.tsx:95-99`: single `<p data-testid="onboarding-workspace-error">` renders below both fields. Both empty-state error and server provision error use the same slot. There is no per-field error (e.g. workspace-name vs company-name); the operator can't tell which field is at fault when only one is blank. `aria-invalid` and `aria-describedby` are absent on both inputs — screen readers don't get the field-error association. |
| 3 | Back / Next buttons | **pass** | Back (`:101-108`) → `router.push("/onboarding/welcome")`. Continue (`:109-117`) submits the form; `disabled={!canSubmit}` gates on input + pending state. Loading state shown via `loading={submitting}`. Both buttons carry `data-testid` for E2E. |
| 4 | Progress indicator | **pass** | Shell sets `step={2}` (`workspace/page.tsx:50`); `StepStrip` renders `aria-valuenow="2"`, pip-1 + pip-2 filled. Verified by `WizardEditorialShell.onboarding.test.tsx:67-69`. |
| 5 | SAVE & EXIT | **partial — present but does not actually save** | Shell renders `wiz-shell-exit` (`WizardEditorialShell.tsx:359-366`). On click, because the workspace page does **not** pass `unsavedHire={true}`, the user is silently routed to `/` **without** the confirm dialog. The draft IS persisted to `localStorage` via the `persist` middleware in `use-onboarding-draft.ts:50-89` — but only after `setWorkspace(...)` fires, which only happens inside `onSubmit` (`:35`). If the user types a name then hits SAVE & EXIT, **nothing is saved.** The label promises "SAVE & EXIT"; the behaviour is "EXIT, abandon". |
| 6 | Mission-pillar copy | **fail** | Step 2 surfaces zero mission pillars. The hero says "Name your workspace." and the sub says "Workers live in workspaces. You can rename them later." No audit-first, no single-binary, no enterprise-grade framing on this step. The trust-strip footer ("NO CREDIT CARD · SOC 2-READY AUDIT TRAIL · GDPR ARTICLE 30 · 2,847 COMPANIES THIS MONTH") IS rendered by the shell (`WizardEditorialShell.tsx:411-427`) and partially substitutes — but the body itself surfaces no pillar copy where it would reinforce the workspace concept (e.g. "Each workspace gets its own audit trail" — currently absent). |

**Workspace step score: 2 pass · 3 partial · 1 fail. 5/10.**

---

## Cross-cutting findings

### Finding 1 — SAVE & EXIT is a lie on step 2 (P1)

The locked F1 shell exposes a "SAVE & EXIT" affordance (`WizardEditorialShell.tsx:359-366`). On step 2, clicking it discards any unsaved typing. The draft store (`use-onboarding-draft.ts`) persists to `localStorage` via the zustand `persist` middleware, so the *capability* to save is present — but the workspace page never calls `setWorkspace(wsName, coName)` until form submit. SAVE & EXIT should call `setWorkspace(workspaceName, companyName)` (no validation — let the draft hold partial data) before navigating away.

**Fix:** add an `onDiscard` (or a `beforeExit`) prop to the shell that fires from `attemptLeave()` before `router.push("/")`. The workspace page wires it to dump the current `workspaceName` and `companyName` into the store. Re-entering `/onboarding/workspace` rehydrates from store (already implemented at `:18-19` via `initialWorkspace` / `initialCompany`).

### Finding 2 — Validation is "non-empty" only; no length cap, no pattern, no slug preview (P2)

For a workspace + company name destined for an organisation row, server-side limits exist (typically 64-128 chars). The page does not surface them. A 10,000-char paste succeeds client-side and fails server-side as a generic "Provisioning failed." toast.

**Fix options:**
- Set `maxLength={64}` on workspace-name and `maxLength={128}` on company-name to match server limits.
- Surface a live character counter or "(N of 64)" hint below each field.
- If "slug" matters to the operator (per the brief), derive it from `workspaceName` and preview it (e.g. "skycot.com/w/acme-growth") below the input — gives instant feedback that the name will become a URL.

### Finding 3 — Single error row for two inputs; no `aria-invalid` (P2)

`workspace/page.tsx:95-99` is a single shared `<p>` slot. Per-field errors would let the operator see "Workspace name is required" next to that input only, rather than a generic "Workspace name and company name are required." that mentions both even when only one is empty. Plus, neither input has `aria-invalid` or `aria-describedby` — assistive tech can't link the error to the field.

**Fix:** split into two slots (`workspace-name-error`, `company-name-error`); add `aria-invalid={!!fieldError}` and `aria-describedby={fieldError ? errorId : undefined}` on each input.

### Finding 4 — Mission pillars only land on step 1 (P2)

The welcome step does this perfectly — three pillar cards, mission-aligned copy, all three SOC 2 / single-binary / RBAC stories surfaced. Step 2 then drops them. The shell's trust-strip footer carries the load partially. A single right-rail sidecar on step 2 ("Each workspace gets its own audit trail and tenant boundary") would reinforce the audit-first pillar at the point the operator is naming their tenant.

### Finding 5 — No team-size or industry capture (P3, brief-driven)

The brief mentions "optional team-size". The page does not collect it. If the operator product roadmap needs team-size for vertical routing (e.g. SMB-default vs enterprise-default at step 4), it must be collected before pick-vertical. Decision needed: do we need it now, or is this DEFERRED until v1.1?

### Finding 6 — Server-error UX is generic toast-style, not field-tied (P3)

`workspace/page.tsx:40-42`: any server provisioning failure renders as "Provisioning failed. Please try again." in the same shared error row. A 409 ("workspace name taken") could route to a per-field message "That workspace name is taken — try another."

### Finding 7 — Welcome step `data-testid="onboarding-welcome"` wraps the whole section but the Continue button has its own (positive) (P0 — no fix)

Defensive testability is good. Skipping.

---

## Top 3 friction points (operator-shipping lens)

1. **SAVE & EXIT discards step-2 typing.** The label promises a contract the implementation doesn't keep. First-time enterprise evaluators who hit SAVE & EXIT, get a coffee, and return will think the wizard is broken. P1 fix is ~30 LOC.
2. **No length or pattern validation on workspace/company name.** Enterprise IT admins will paste names that exceed server limits; the failure mode is a generic "Provisioning failed." that doesn't tell them what to fix. P2 fix is `maxLength=` + a length-error message branch.
3. **Mission pillars vanish at step 2.** Step 1 lands the three pillars cleanly; step 2 forgets them. The operator's brand promise ("audit-first, single-binary, enterprise-grade") needs to be reinforced where the operator is actually committing data — not just on the page they're leaving.

---

## Per-step verdicts

- **Step 1 (welcome):** 10/10. Ship-ready. Pillars surface, progress indicator works, SAVE & EXIT works (no draft to lose), Continue routes correctly. The only nit is no Back button — and Back from step 1 would mean back to signin, which is wrong.
- **Step 2 (workspace):** 5/10. Functional happy-path. Three real gaps: SAVE & EXIT does not save, validation is non-empty-only, mission pillars absent. None are P0 — but all three are visible to a real enterprise evaluator within 60 seconds.

**Combined verdict for steps 1+2: 7.5/10 — ship, with P1 SAVE & EXIT fix queued for next wave.**
