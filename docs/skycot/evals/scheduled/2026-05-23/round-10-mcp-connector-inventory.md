# Round 10 — MCP connector inventory: real-vs-stub triage

**Date**: 2026-05-23
**Round**: 10
**Mission claim**: 5 (Any task a human can do on a PC — MCP backend)
**Surface**: 29 connector crates in workspace + 7 empty leftover dirs

## Verdict: 🟡 PARTIAL — 27 real implementations look healthy, 3 hygiene findings worth a quick brief

- **🟢 substrate**: 29 workspace-member connector crates with `Cargo.toml`, all compile, all carry at least one test file.
- **🟡 P3** — slack listed TWICE in workspace `Cargo.toml` (lines 24 + 55).
- **🟡 P3** — 7 empty leftover directories with no `Cargo.toml`, no source files, not workspace members. Filesystem clutter + ambiguity about operator intent for the 3 "DEFERRED-v1.x" enterprise stubs (apollo / zoom / zoominfo).
- **🟡 P2** — `argo-mcp-connector-notion` is dramatically under-developed vs peers (360 LOC, 2 tests vs cohort median ~700 LOC, 9 tests). Real but lean enough to call out.

This round did NOT do per-connector OAuth scoping verification — that's the canonical brief shape from the gap manifest (`eval-mcp-connector-real-vs-stub-inventory`) and the right next-session focus.

## Setup

```bash
# Per-connector size + signal sweep
for c in crates/argo-mcp-connector-*; do
  # files / LOC / OAuth+token signals / dispatch fns / TODOs / test files / unit tests / real URLs
done

# Workspace membership audit
grep -E '"crates/argo-mcp-connector-' Cargo.toml

# Sample compile-check
cargo check -p argo-mcp-connector-{slack,notion,stripe,discord}
```

## Observed

### Total inventory

| Bucket | Count | List |
|---|---|---|
| **Empty leftover dirs** (no `Cargo.toml`, not workspace members) | 7 | apollo, mailgun, resend, signal, telegram, zoom, zoominfo |
| **Real connector crates** (workspace + Cargo.toml + source + tests) | 29 | airtable, asana, calendly, clickup, discord, figma, fireflies, gcal, github-issues, gmail, google-meet, hubspot, imessage, jira, linear, mailchimp, monday, notion, onedrive, pagerduty, postmark, salesforce, sendgrid, shared, slack, stripe, stripe-events, trello, typeform, zendesk |
| **Substrate** (utility, not a connector) | 1 | argo-mcp-connector-shared (retry-with-backoff) |
| **Effective connectors** | 28 | (above minus `shared`) |

### Size cohort — sorted by LOC, real implementations only

```
1175  stripe          (3 oauth signals, 14 tests)
1047  onedrive        (3 oauth signals,  9 tests)
 951  clickup         (1 oauth signal,  10 tests)
 856  github-issues   (1 oauth signal,   9 tests)
 845  salesforce      (3 oauth signals,  9 tests)
 811  monday          (1 oauth signal,  10 tests)
 811  pagerduty       (0 oauth signals,  9 tests)
 779  jira            (0 oauth signals,  9 tests, 4 real URLs)
 771  airtable        (1 oauth signal,   9 tests)
 766  linear          (1 oauth signal,  10 tests)
 760  fireflies       (1 oauth signal,  10 tests)
 755  gmail           (3 oauth signals,  9 tests, 3 real URLs)
 746  trello          (0 oauth signals, 10 tests)
 713  zendesk         (0 oauth signals,  8 tests)
 703  asana           (1 oauth signal,   8 tests)
 700  postmark        (0 oauth signals,  9 tests)
 694  sendgrid        (1 oauth signal,   7 tests)
 694  typeform        (1 oauth signal,   8 tests)
 676  gcal            (4 oauth signals,  4 tests, 5 real URLs)
 663  discord         (0 oauth signals,  7 tests)  ← bot-token auth, not OAuth
 663  mailchimp       (0 oauth signals,  8 tests)
 661  calendly        (1 oauth signal,   7 tests, 3 real URLs)
 651  stripe-events   (1 oauth signal,   9 tests)
 608  google-meet     (2 oauth signals,  7 tests)
 602  figma           (0 oauth signals,  8 tests)
 509  imessage        (0 oauth signals,  7 tests)  ← local AppleScript, not OAuth
 480  hubspot         (1 oauth signal,   8 tests)
 411  slack           (1 oauth signal,   3 tests)  ← bot-token, structurally lean
 360  notion          (1 oauth signal,   2 tests)  ← outlier: see Finding C
 476  shared          (substrate — retry-with-backoff)
```

Sample compile-check across 4 (slack / notion / stripe / discord) — all clean, no warnings beyond pre-existing workspace noise. Implementations are wired, not dead.

### Finding A (🟡 P3) — slack listed twice in workspace `Cargo.toml`

```
22:  "crates/argo-mcp",
23:  "crates/argo-mcp-connector-shared",
24:  "crates/argo-mcp-connector-slack",       ← first listing
25:  "crates/argo-cli",
...
54:  "crates/argo-genesis",
55:  "crates/argo-mcp-connector-slack",       ← duplicate, 31 lines later
56:  "crates/argo-mcp-connector-gcal",
```

Cargo silently dedupes, so this isn't a build break, but it's a confusing source-of-truth bug — a future hand cleaning up the workspace list will trip over which entry is canonical. Remove the line-24 entry (the line-55 listing groups with the other connectors and matches the surrounding alphabetical-ish ordering).

### Finding B (🟡 P3) — 7 empty leftover directories

The four "consumer" connectors the operator memory said were "being deleted" — signal, telegram, mailgun, resend — are NOT just stub-shells. Their directories on disk are completely empty: no `Cargo.toml`, no `src/`, nothing. They are also NOT workspace members.

Same shape for the three "enterprise DEFERRED-v1.x stubs" — apollo, zoom, zoominfo. The operator memory at `project_v1_strategic_decisions_2026_05_14.md` says these should "stay as DEFERRED-v1.x placeholders." Today they are also empty leftover directories with no workspace membership.

This is a process drift between the operator-intent doc and on-disk state. The fix is one of:
1. `rmdir` the 7 empty dirs (matches actual state — placeholder intent has been silently dropped).
2. Re-create `Cargo.toml` + `lib.rs` shell stubs for apollo / zoom / zoominfo and add them back to the workspace, to make the "DEFERRED-v1.x placeholder" intent real.

Either is fine; doing neither leaves the contradiction in the tree.

### Finding C (🟡 P2) — `argo-mcp-connector-notion` is dramatically under-developed

Notion is 360 LOC across 4 source files with only 2 unit tests. Median peer is ~700 LOC, 9 tests. The other under-cohort entries have explanations:

- **slack** (411 LOC, 3 tests) — lean by design; uses bot-token auth, one chat-post tool, one channel-list tool. The brief was always scoped tight.
- **hubspot** (480 LOC, 8 tests) — reasonable.

Notion is a high-leverage connector (workplace knowledge, doc edits) and has a real implementation but at less than half the surface of peers. Two unit tests likely don't cover the per-tenant credential scoping path. Worth a closer read in a follow-up round, OR a brief to bring it up to peer parity if `notion` is in the v1 demo surface.

## Anti-theater check

The LOC + test counts come from `wc -l` + `grep -c '#\[(tokio::)?test\]'` on actual source files — not from a manifest, not from a registry, not from documentation. The compile-check on 4 sample connectors proves they are not zero-cost stub modules that the workspace happens to list.

I did NOT verify per-connector OAuth scoping (the canonical brief from the gap manifest) — that requires reading each connector's `client.rs` for `with_token(tenant_id, ...)` patterns, which is per-crate work and the right size for a dedicated round. Today's verdict is structural ("crates exist + compile + have tests"), not semantic ("each connector keys credentials by tenant").

## Operational

- Live gateway at :7400 healthy — `curl -sf http://127.0.0.1:7400/health` returned 200 (body empty).

## Gap / follow-up brief candidates

1. **`claude-workspace-cargo-dedupe-slack-entry`** (P3 hygiene, ~5 min) — Delete the line-24 duplicate `"crates/argo-mcp-connector-slack"` entry in `Cargo.toml`.

2. **`claude-mcp-connector-empty-dir-cleanup`** (P3 hygiene, ~15 min) — Either `rmdir` the 7 empty `crates/argo-mcp-connector-{apollo,mailgun,resend,signal,telegram,zoom,zoominfo}/` directories OR re-stub `apollo / zoom / zoominfo` as DEFERRED-v1.x workspace members per `project_v1_strategic_decisions_2026_05_14.md` intent. Operator-decisional. **Recommend**: delete all 7 (operator intent has clearly moved on; reviving stubs adds noise without value).

3. **`eval-mcp-connector-notion-peer-parity-check`** (P2, ~30 min) — Read `crates/argo-mcp-connector-notion/src/client.rs` to determine whether the 360-LOC / 2-test surface covers (a) per-tenant credential keying, (b) the workplace-knowledge editor flows that depend on it, (c) standard MCP error mapping. If gaps surface, author a `mission-mcp-connector-notion-fill-in` follow-up.

4. **`eval-mcp-connector-credential-scoping-sweep`** (P1, ~60-90 min, FUTURE EVAL ROUND) — The canonical brief shape from the round-8 gap manifest. For each of the 27 real connectors, verify the client-construction path takes a `tenant_id` (or equivalent per-tenant credential scope), the request layer does not silently fall back to a shared/global token, and there is at least one negative test that cross-tenant calls cannot reuse another tenant's auth.

## Manifest row update

| Crate | Old verdict | New verdict |
|---|---|---|
| 27 MCP connectors | 🔴 NOT AUDITED individually | 🟡 round 10 — structural inventory PASS, per-connector credential scoping deferred to follow-up round |
| `argo-mcp-image-gen` | (not in manifest) | (still not audited — image-gen sidecar; queued) |
