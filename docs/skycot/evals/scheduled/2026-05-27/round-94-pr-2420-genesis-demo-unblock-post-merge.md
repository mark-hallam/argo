# Round 94 — PR #2420 genesis-orphan-discoverability + provider-key alias POST-MERGE

- **Date**: 2026-05-26T21:15Z (AEST 2026-05-27 07:15)
- **Round**: 94
- **Pillar / Claim**: Claim 3 (Platform-builds-workers) — Genesis Studio is the investor-demo headline; CLAIM 1 (Teammate-not-tool) — Onboarding navigation discoverability
- **Verdict**: 🟡 **CODE-PASS / DEMO-BLOCKED** (P0 — investor-demo recording still blocked until live gateway restarts)

## Setup

PR #2420 `fix(genesis): unblock investor demo — nav + clickable pillar + provider-key alias` merged 2026-05-26T20:50:59Z as commit `91215d129` (closes `fix-genesis-orphan-discoverability-and-provider-key-503` / UX-AUDIT-T2309MEL F1 P0).

Files touched (+140/-13 across 6 files):

- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream.rs` (+29/-)
- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_tests.rs` (+59/-)
- `scripts/demo-dry-run.sh` (+27 new)
- `web/app/(setup)/onboarding/welcome/page.tsx` (+15/-)
- `web/components/skycot/nav-items.tsx` (+3/-)
- `web/tests/nav-sidebar-order.test.ts` (+20/-)

Verified on-`origin/main` via source-read + **live gateway probe** at `http://127.0.0.1:7400/api/v1/genesis`.

## Observed

### A — Production code is correct

**Provider-key alias logic** (routes_genesis_stream.rs):

```
47: pub const PROVIDER_KEY_ENV: &str = "ARGO_GENESIS_PROVIDER_KEY";
51: const PROVIDER_KEY_ALIASES: &[&str] = &["ANTHROPIC_API_KEY", "OPENAI_API_KEY"];
…
239-246: fn provider_key_present() -> bool {
            let non_empty = |key: &str| std::env::var(key)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);
            non_empty(PROVIDER_KEY_ENV) ||
                PROVIDER_KEY_ALIASES.iter().any(|k| non_empty(k))
         }
```

Either the canonical env var OR any alias satisfies the gate.

**503 error message updated** (routes_genesis_stream.rs:85-94):

```
let aliases = PROVIDER_KEY_ALIASES.join(" or ");
… &format!(
    "Set {PROVIDER_KEY_ENV} (or {aliases}) to enable POST /api/v1/genesis (LLM-driver cost guardrail)"
)
```

**Tests (routes_genesis_stream_tests.rs)**:

- `fails_closed_without_provider_key` (line 121) **hardened** — now `EnvGuard::unset(PROVIDER_KEY_ENV + all PROVIDER_KEY_ALIASES)` so a stray `ANTHROPIC_API_KEY` in the test runner's parent env cannot mask the fail-closed contract. Anti-theater discipline.
- `anthropic_alias_satisfies_provider_key_gate` (line 144) sets `ANTHROPIC_API_KEY="sk-ant-stub-key-for-alias-test"`, unsets `PROVIDER_KEY_ENV`, asserts `HTTP/1.1 200 OK` AND `!raw.contains("genesis_provider_key_missing")` — proves alias actually satisfies gate.

### B — Discoverability is wired correctly

**NAV_ITEMS** (web/components/skycot/nav-items.tsx:22-31):

```
{ href: "/", label: "Approvals", icon: <Inbox /> },
{ href: "/team", label: "Team", icon: <Users /> },
{ href: "/workers", label: "Workers", icon: <Workflow /> },
{ href: "/genesis", label: "Genesis", icon: <Wand2 /> },   ← NEW
{ href: "/cohorts", label: "Cohorts", icon: <Layers /> },
…
```

Position is between Workers and Cohorts — matches the "people-cluster" intent the PR body cites.

**Welcome pillar tile** (web/app/(setup)/onboarding/welcome/page.tsx:43-62):

```
<li
  className="ob-pillar"
  data-testid="onboarding-pillar-genesis-studio"
  role="link"
  tabIndex={0}
  style={{ cursor: "pointer" }}
  onClick={() => router.push("/genesis")}
  onKeyDown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      router.push("/genesis");
    }
  }}
>
```

Proper keyboard accessibility (Enter + Space + tabIndex) — not just a `<div onClick>` anti-pattern.

### C — Live gateway is STALE ⚠️ — demo recording still blocked

Live probe:

```bash
$ curl -sS -X POST http://127.0.0.1:7400/api/v1/genesis \
    -H "Content-Type: application/json" \
    -d '{"role_description":"smoke-test"}'
{"error":{"code":"genesis_provider_key_missing","message":"Set ARGO_GENESIS_PROVIDER_KEY to enable POST /api/v1/genesis (LLM-driver cost guardrail)"}}
```

The returned message is the **pre-PR-#2420** string. The new message would say:

```
"Set ARGO_GENESIS_PROVIDER_KEY (or ANTHROPIC_API_KEY or OPENAI_API_KEY) to enable POST /api/v1/genesis (LLM-driver cost guardrail)"
```

Gateway clock:
- `/health` reports `uptime_seconds=60028` at 21:14:31Z
- Gateway boot time: 2026-05-26T04:34:03Z (~16.7 hours ago)
- PR #2420 merge time: 2026-05-26T20:50:59Z
- **Gateway started ~16.3 hours BEFORE the PR merged.**

The live process is running pre-#2420 binary. Schema unchanged (v83 → v83), so `scripts/check-gateway-freshness.sh` reports FRESH — **false-negative for handler-only code changes**.

## Anti-theater check

Dispositive on **multiple independent axes** for the merged code:

1. `PROVIDER_KEY_ALIASES` is a `&[&str]` constant — easy to grep and audit; both aliases declared verbatim
2. `provider_key_present()` short-circuits canonical first, then iterates aliases — covers both old + new paths
3. `fails_closed_without_provider_key` test was deliberately hardened (unset alias keys too) — prevents env-leak masking
4. `anthropic_alias_satisfies_provider_key_gate` test has both positive `200 OK` AND negative `!contains("genesis_provider_key_missing")` assertions — catches partial regressions where status flips but error code leaks through
5. Nav-sidebar-order test (touched in PR) pins position so future nav additions can't silently displace Genesis

**98th anti-theater verification this week.**

## Gap / follow-up brief candidates

### 🟡 **P0 — Live gateway needs restart to carry PR #2420** (operator-actionable)

The investor-demo unblock is **code-correct but runtime-stale**. Until the operator runs `make restart-live-gateway` (which has been the canonical recipe since #2283), POST /api/v1/genesis will still return the old 503 with the old error message, and the demo recording will hit the original block.

This is operator-actionable, not agent-actionable (gateway restart requires the operator's local environment). Conductor should surface in next handoff.

### 🟢 **P1 NEW — `ops-gateway-freshness-check-also-detects-binary-staleness`** (~60 LOC)

The current `scripts/check-gateway-freshness.sh` only compares migration schema versions. Handler-only Rust changes (like #2420, #2429, #2422, and several others this week) ship to main without bumping schema → freshness check stays GREEN while live runtime silently serves stale code.

Recipe:
- Embed a build-time `git rev-parse HEAD` into the gateway binary (already common; argo-cli may already have this via `built` crate or env macro)
- Add `git_sha` (or `build_sha`) to `/health` JSON response
- `check-gateway-freshness.sh` reads + compares against `git rev-parse origin/main`
- If different → emit STATUS: STALE-BINARY (drift=N commits ahead)

Cuts the false-negative class entirely. Would have caught this exact #2420 case at the next conductor tick (the binary-sha drift would be `91215d1...` on main vs the boot-sha from 16.7h ago).

### 🟢 **P3 NEW — `ops-conductor-post-merge-suggest-gateway-restart-on-handler-change`** (~30 LOC bash)

After conductor merges any PR that touches `crates/argo-cli/src/handlers/**` and the live gateway uptime > 1h, conductor should append a `🚨 gateway-restart-needed` line to its tick summary. Cheaper than the freshness-script-extension above but only signals — doesn't detect post-merge drift from non-conductor pushes.

## Closure

**Code POST-MERGE PASS. Runtime POST-MERGE BLOCKED on gateway restart.** Both the alias gate + the discoverability nav/pillar landed cleanly with anti-theater test coverage. The investor-demo unblock is not delivered until the live gateway runtime carries the fix — this is an operator-side restart action, not an agent action.

PR #2420 closes UX-AUDIT-T2309MEL F1 in the codebase. The recording-readiness blocker class is wider than this brief, but the brief itself is done.
