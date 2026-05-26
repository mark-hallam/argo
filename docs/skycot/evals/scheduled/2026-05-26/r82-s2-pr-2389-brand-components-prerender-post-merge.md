# R82 · S2 · PR #2389 brand-components prerender fix — POST-MERGE

**Date**: 2026-05-26 (round 82)
**Pillar**: Pillar VI — commercial usability (brand-components showcase page is the visual reference surface)
**Verdict**: 🟢 **PASS · P3** (minimal-correct 2-LOC fix; matches established `styled-jsx`/event-handler client-boundary trap pattern)

## Setup

PR #2389 — `fix(web): make brand components showcase a client page` — MERGED 2026-05-26T06:42:05Z (commit `8057f8e96`). +2 / −0 across 1 file: `web/app/(app)/brand/components/page.tsx`.

Commands:

```
cd /tmp/eval-r82-wt
git show 8057f8e96 -- 'web/app/(app)/brand/components/page.tsx'
head -20 'web/app/(app)/brand/components/page.tsx'
rg -n "onClick|onChange|onSubmit|useState|useEffect" 'web/app/(app)/brand/components/page.tsx'
rg -n "use client" web/components/ui/{ApprovalCard,AuditTimeline,Button}.tsx
```

## Observed

The diff is literally two lines at the top of the page file:

```diff
+"use client";
+
 import { ... } from "lucide-react";
```

The page imports many `"use client"` components (`Button`, `ApprovalCard`, `AuditTimeline`, etc., all verified to have `"use client"` at line 1) and additionally passes inline event handlers from a Server Component context:

```
56:            <SignupForm onSubmit={() => {}} />
59:            <SignupForm onSubmit={() => {}} state="loading" />
62:            <SignupForm onSubmit={() => {}} state="success" />
```

This is the known Next.js 15+ prerender trap: passing a function value (`() => {}` inline) from a Server Component to a Client Component fails at build/prerender time because functions are not serialisable across the RSC boundary. Adding `"use client"` to the page promotes the whole subtree to client-rendering, eliminating the boundary crossing.

Per memory `feedback_styled_jsx_requires_use_client` (2026-05-20 wave-2 trap), pages with similar boundary issues (styled-jsx blocks) return HTTP 500 without `"use client"`. The fix here is the same shape and same minimum change.

## Anti-theater check

Cannot probe live SPA — Next.js dev server is not running on :3000 (only the Rust gateway is alive on :7400, which returns 404 for SPA paths). Per scheduled-task hard rule "Skip browser scenarios if Chrome MCP isn't connected and Playwright isn't available", verification is source-level only.

Source-code evidence that the fix is real and not theater:
1. **The change is in production code**, not in a test. Cannot be "rewritten green".
2. **The trap pattern is established and documented** in operator memory + the wave-2 perf-trap incident notes. The `onSubmit={() => {}}` pattern + nested `"use client"` children is a Next.js-defined failure mode, not a project-invented constraint.
3. **Page renders without test infrastructure changes** — no new mocks, no new fixtures. The only change is the boundary annotation.

## Gap / follow-up

**Potential brief candidate (P3, low priority):** A repo-wide lint/regex to flag any `web/app/**/*page.tsx` that (a) lacks `"use client"` and (b) contains inline arrow-function props (`={() => …}`) being passed to imported components. This would catch the same trap on future pages before merge. Likely 30-50 LOC test + scan helper, similar in shape to the `#71717a` directory-scan guard at `web/__tests__/a11y-contrast.test.ts:113-130`.

Brief shape:
- **Name**: `web-prerender-trap-server-component-with-inline-handler-guard`
- **Scope**: new test `web/__tests__/a11y-prerender-trap.test.ts` (~40 LOC) that walks `web/app/**/page.tsx`, checks for inline-handler prop literals when `"use client"` is absent, and fails with a remediation hint.
- **Priority**: P3 (preventive). Defer until v1.0 demo ships unless another instance recurs.

Not recommending now — the operator-locked overnight Genesis+Track-P merge-aggressive window (per `feedback_overnight_genesis_track_p_merge_authorization_2026_05_25` memory) closes 2026-05-26T22:00Z AEST, and this is a non-critical-path P3.
