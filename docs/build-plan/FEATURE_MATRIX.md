# SKYCOT — Feature Matrix & Roadmap (source of truth)

**What this is:** the single, honest inventory of every SKYCOT capability — what's
**real and working**, what's **scaffold**, and what's **planned** (with phase/timeline).
It is the canonical tracker for features + roadmap; other docs link here.

**Last verified:** 2026-05-29 by a parallel **code-grounded audit** (5 agents over
core platform / product surfaces / integrations+channels / agents+verticals+voice /
roadmap); **updated 2026-05-30** for the Sprint-2 build-out (voice V1/V2, settings,
evals, governance readiness, training-data export, retrain-recent — each row cites
its commit). Every row is grounded in a crate, gateway route, SPA module, or doc —
**not** transcribed from roadmap narratives (which drift). Re-verify with `git log`
+ the cited source before trusting any single row. **Caveat carried by several
2026-05-30 rows:** the feature is committed + tested but the *running* gateway is
still the old binary, so it goes live only after the next gateway rebuild
(`scripts/restart-live-gateway.sh --force --no-prompt`).

> **Why the rigor:** this codebase is *more real* than its docs feared (genuine
> hash-chained audit, enforced RBAC, per-tenant MCP pooling, ~8,200 test fns) — but
> it also has scaffold dressed as features (orphan crates, mock-in-demo-live SPA
> surfaces). This matrix draws that line explicitly so we never confuse "code exists"
> with "it works for a user."

## Status legend

| Badge | Meaning |
|---|---|
| ✅ **Live** | Real, wired, verified working end-to-end |
| 🟡 **Partial** | Works with caveats (one direction, mock in some modes, missing sub-piece) |
| 🟧 **Scaffold** | Code exists but **unwired / unusable / stub** (not a working feature yet) |
| 🚧 **In-progress** | Actively being built (brief claimed / partially landed) |
| 🗓️ **Planned** | Designed / brief exists, not built — phase + timeline noted |
| 💡 **Backlog** | Named, not yet scoped |

## At-a-glance

- **The product is genuinely demo-ready.** All 6 mission claims have real backend +
  real demo surfaces; two AI teammates (Portia, Grant) do real LLM work, truth-bounded,
  approval-gated; audit is a real hash chain; ~30 MCP connectors are real.
- **Biggest honesty gaps (updated 2026-05-30):** (1) **live voice on Zoom/Teams calls = not built** (the L1 trait layer + L2 truth-bounded gateway voice-loop now ship against a *mock* provider — commits `4e891c0b6`/`5a98d63dc`; the **real OpenAI adapter is in-progress** per DEC-021, and there's still no meeting-join layer); (2) the cluster of mock-in-demo-live SPA pages is **shrinking** — settings/governance-readiness/retrain-recent/audit-export/retros are now **live** (gateway rebuilt + verified 2026-05-30, commit `ccfa89588`); evals is live-but-empty (`skill_reeval_history` not yet seeded — follow-up), leaving api-keys/provider-keys (SKIP-privacy), ghostwriter, skill-IR, elevated-view, worker schedules/escalation; (3) the **JSON-RPC plugin ABI** and **teammate-eval scorer** are still tested code with zero live consumers.

---

## 1 · Mission claims (the 6 — backend + demo-surface rollup)

| Claim | Backend | Demo surface | Notes |
|---|---|---|---|
| 1 — Teammate, not tool (persistent identity/memory) | ✅ Live | ✅ Live | 7 teammates persist across restart; worker-detail tabs render live |
| 2 — Delegation is the unit | ✅ Live | ✅ Live | `delegate_task` tool threads `current_handoff_id`; Grant owns a standing responsibility |
| 3 — Platform builds workers (Genesis) | ✅ Live | ✅ Live | Genesis SSE pipeline + `/genesis` page; **text-only** (multi-modal = Phase 2, planned) |
| 4 — Skills shared + learned | ✅ Live | ✅ Live | Layered-lookup wired; real per-skill adoption seeded; learning-loop distillation = 🟡 placeholder |
| 5 — Any-PC execution (computer-use) | ✅ Live (macOS) | ✅ Live | macOS-first per DEC-002; Linux/Windows/E2B = 🟧 deferred-v1.x |
| 6 — Multi-channel + multi-tenant + approvals | ✅ Live | ✅ Live | 9 of 11 channels wired both ways; tenant_id plumbed everywhere |

---

## 2 · Core platform

| Feature | Status | What's real / what's not | Source |
|---|---|---|---|
| Agent runtime / turn loop | ✅ Live | Real multi-round tool loop; wired into gateway `/sessions` + `/workers/<id>/run` | `argo-agent-runner/turn_run_loop.rs` |
| Provider routing (manifests + transports) | ✅ Live | TOML manifests (not Rust adapters); anthropic/bedrock/claude-code-cli/openai transports + retry/stream/cache/spend-gate | `argo-provider`; `manifests/providers/*.toml` |
| Model catalog / cost-tier routing | ✅ Live | `TieredRouter` classifier → simple/medium/complex, per-session cache; Sonnet floor (no Haiku) per DEC-013 | `argo-routing`; `turn_routing.rs` |
| Memory store | ✅ Live | append/supersede/revert/lock/versions, all tenant-scoped; layers, embeddings, shared-team memory | `argo-memory/store.rs` |
| Skills — library / layered-lookup | ✅ Live | `LayeredLookup::with_library` wired into the live turn; cross-tenant negative test | `argo-agent-runner/turn_helpers.rs:65` |
| Skill install / promote / refine | ✅ Live | `POST /skills` install + `/skills/<name>/promote`; refine route + Forge | `routes_post_writes_skills.rs`, `routes_skill_refine.rs` |
| Skill learning loop (CU trajectory → skill) | 🟡 Partial | Primitives real (`should_propose`, `SkillCandidate::from_cu_trajectory`); **Claude distillation pass not done** — ships a deterministic placeholder body | `argo-skill/learn.rs` |
| Audit — hash-chained ledger | ✅ Live | Real SHA-256 chain + verify/gap-detect/export/tamper-watchdog; ~247 emit callsites | `argo-audit/sink_chain.rs`; `/api/v1/audit` |
| Governance audit-readiness report | ✅ Live | `GET /api/v1/governance/readiness` — read-only assembly of the moat (audit chain-integrity + approval coverage + RBAC role presets + truth-boundary status), all values read from source (NO THEATER), tenant-scoped. Live (gateway rebuilt + verified 2026-05-30, commit `ccfa89588`) | `routes_governance.rs` (commit `764f725ff`) |
| RBAC / capabilities | ✅ Live | `CapabilityEngine` + JWT-derived role gate; header-bypass closed (403 in prod). Supersedes the stale "RBAC is copy" note | `argo-capability`; `gateway/role_gate.rs` |
| Truth-boundary (claim gating) | ✅ Live | Wired into live turn + Pitch/GTM agents; per-channel policy; Allow/Escalate/Forbid | `turn_helpers_truth_boundary.rs`; `policies/truth-boundary/*.toml` |
| Gateway (HTTP server) | 🟡 Partial | ~32 real `/api/v1` noun-groups w/ CORS/CSRF/JWT/rate-limit. Caveat: **hand-rolled HTTP/1.1 string-dispatcher**, not axum/hyper — works but parse-fragile | `argo-cli/handlers/gateway/` |
| MCP integration (client pool + tools) | ✅ Live | Per-tenant `McpClientPool` (LRU + idle-TTL); subprocess client; registry merges MCP + 15 builtin tools | `argo-mcp/pool.rs` |
| In-core tools (15) | ✅ Live | FileRead/Write/Patch, Terminal, WebSearch/Extract, Memory, DelegateTask, Vision, CodeExecute, SendMessage, SessionSearch, Todo, Clarify, Process | `argo-tool/tools/mod.rs` |
| Marketplace (OCI bundles + skill browse/install) | ✅ Live | OCI push/pull via CLI; `/marketplace/*` browse+install gateway surface | `argo-marketplace`; `gateway/marketplace.rs` |
| Eval harness | ✅ Live | Runner + compare/honesty/regression/self-healing; CLI `Eval` + per-vertical suites + `pitch-eval`; `argo eval export-training-data` (SFT-JSONL from decided approvals + committed retrains, commit `7e84ba7d6`); `argo eval generate-cases` (Step 1b — same judged history → argo-eval regression cases: rejected approvals → must-refuse/defer, retrains → corrected-behaviour; commit `99114bd5a`) | `argo-eval`; `eval_export_training_data.rs`; `case_gen.rs`; `eval_generate_cases.rs` |
| Teammate-eval (6-axis scoring) | 🟧 Scaffold | Real 6-axis scoring library but **no runner/CLI/gateway caller** — design-first (DEC-017) | `argo-eval/teammate.rs` (no consumer) |
| Forge (skill optimization) | ✅ Live | `spawn_forge_cron` at boot; CLI bench/optimize/promote/sweep | `argo-forge` |
| Replay engine | ✅ Live | Time-travel replay via CLI + export bundle | `argo-replay` |
| SIEM export / telemetry | ✅ Live | Datadog/Elastic sinks; span+metric taxonomy (prompt-cache hit counters) | `argo-siem`, `argo-telemetry` |
| Multi-tenancy | ✅ Live | `tenant_id` plumbed pervasively (~6,884 refs); memory/audit/MCP/skill tenant-scoped w/ negative tests | workspace-wide |
| Migrations (additive + slot-guarded) | ✅ Live | Schema v85 (`workspace_settings` added, commit `423274a25`); duplicate-slot validator + CI guard at boot | `argo-store/migrations/registry.rs`, `migrations/v85.rs` |
| JSON-RPC plugin ABI | 🟧 Scaffold | `argo-plugin` has a real tested subprocess protocol but **0 production consumers** — invariant exists as code, not a live extension path | `argo-plugin` (0 deps) |
| Single-binary architecture | ✅ Live | One `argo` binary bundles runtime/gateway/eval/forge/replay/marketplace; other bins are out-of-process MCP servers (by design) | `argo-cli` `[[bin]] argo` |

---

## 3 · Product surfaces (the SPA)

> **Data-source rule:** ✅ = hits the live gateway in demo-live (`isAuthBypassed() && !isDemoLive()` gate). 🟧 = short-circuits to **mock on `isAuthBypassed()` alone**, so it shows fixtures even in demo-live.

| Feature | Status | What's real / what's not | Source |
|---|---|---|---|
| /team roster | ✅ Live | Live `GET /api/v1/workers` in demo-live | `lib/api/team.ts` |
| /approvals inbox + KPIs + decide | ✅ Live | Live list + `POST .../decide` | `lib/api/approvals.ts` |
| /audit feed + action drill-in + replay | ✅ Live | All three live in demo-live | `lib/api/audit.ts`, `audit-detail.ts`, `replay.ts` |
| /pitch (Portia rehearsal) | ✅ Live | No mock path — always `POST /api/v1/pitch/ask` | `lib/api/pitch.ts` |
| /gtm (Grant console) | ✅ Live | No mock path — always `POST /api/v1/gtm/draft-followup` | `lib/api/gtm.ts` |
| /genesis (onboarding studio, SSE) | ✅ Live | Streaming SSE + classify/draft-worker + hire; **text-only** (multi-modal planned) | `lib/api/genesis.ts` |
| /cohorts | ✅ Live | Live in demo-live | `lib/api/cohorts.ts` |
| /skills marketplace + learned-this-month + adoption | ✅ Live | Live; real per-skill adoption (seeded worker_skill) | `lib/api/skills.ts` |
| /workflows list + detail + builder + save/publish + runs | ✅ Live | List + detail live in demo-live; detail (`fetchWorkflow`) gate-corrected this commit — it was bare `isAuthBypassed()` (served the seed spec on camera in Scene 5) and now uses `isAuthBypassed() && !isDemoLive()` so `GET /api/v1/verticals/<v>/workflows/<n>` fires (`workflows.ts:213`, demo-live vitest added). **Runs panel (`workflow-runs.ts`) still bare** — flagged WIRE #3 in overnight-build-out P1 worklist | `lib/api/workflows.ts`, `workflow-runs.ts` |
| /orchestration | ✅ Live | Live runs + cancel/reset/escalate + **team-run tree view** (lead-with-children, per-step status) — design plan 02 Phase 3, commit `59aa69080`; driven by real `[team:<id>]`-prefixed rows from `GET /api/v1/orchestration/runs` (verified live), falls back to flat queue when no team linkage. **Concurrent-team grouping now precise** (this commit; see `git log`): each run carries a title-encoded run-wide id (`[team:<id>]`), so two concurrent team runs render as two separate trees instead of collapsing into a shared "Delegated steps" node; legacy bare-`[team]` rows still group gracefully | `lib/api/orchestration.ts`; `components/skycot/orch/TeamRunTree.tsx` |
| /settings/knowledge (workplace wiki) | ✅ Live | Full CRUD live | `lib/api/workplace-knowledge.ts` |
| /settings/members, /security (SSO/SAML), /connectors | ✅ Live | Always-live (no mock path) | `workspace-members.ts`, `sso.ts`, `connectors.ts` |
| worker-detail: overview/activity/memory/skills/trust/settings | 🟡 Partial | overview/activity/memory/skills/settings live in demo-live (live-shape crashes fixed 2026-05-29); **Trust tab (`worker-trust.ts:166,222`) still bare `isAuthBypassed()`** — serves mock on camera, flagged WIRE #4 in overnight-build-out P1 worklist | `lib/api/worker-*.ts` |
| Auth — magic-link signup/verify/signin (+ OAuth) | ✅ Live | NextAuth + Google/GitHub/Microsoft + magic-link; demo-mint gated to non-prod | `lib/api/auth.ts` |
| Onboarding wizard (5-step) | 🟡 Partial | Renders; progress persists to localStorage; pick-vertical/connect-provider touch gateway; welcome/workspace steps client-only | `lib/onboarding/` |
| Multi-tenant theming / white-label | 🟡 Partial | Token infra + per-tenant CSS synthesis real; tenant-override **inert today** (awaits workspace-switcher) | `lib/theme/`, `lib/api/tenant.ts` |
| worker-detail: escalation | 🟡 Partial | Page renders from **sample trust-history**; live `GET .../escalations` is a TODO | `lib/api/worker-policies.ts` |
| worker-detail: schedules | 🟧 Scaffold | Renders from persona fixtures; no live schedules list wired | `lib/api/duties.ts` |
| /settings workspace + providers + notifications | ✅ Live | Backend shipped: `workspace_settings` table (migration v85) + GET/PATCH `/api/v1/{workspace,providers/policy,notifications/preferences}`; `settings.ts` un-mocked under demo-live. Live (gateway rebuilt + verified 2026-05-30, commit `ccfa89588`) | `lib/api/settings.ts`; `routes_workspace_settings.rs` (commit `423274a25`) |
| /settings api-keys + provider-keys | 🟧 Scaffold | Mock-on-bypass; sensitive (no secret exposure); endpoints absent | `lib/api/api-keys.ts`, `provider-keys.ts` |
| /evals dashboard | 🟡 Partial | Backend shipped: `/api/v1/evals` now a real store-backed query (worker's learned skills → their `skill_reeval_history`), tenant-scoped, real-empty when no history; `evals.ts` **NOT yet un-mocked** — the gate comment at `evals.ts:54` references the canonical `!isDemoLive()` guard but the short-circuit at `evals.ts:166` is still bare `isAuthBypassed()` (serves mock in demo-live); flagged WIRE #2 in overnight-build-out P1 worklist. ✅ code+tests (6 handler + vitest 233); endpoint live. **Data now seeded** — `argo_demo_seed::seed_skill_reeval_history` inserts a 3-run trend (one regression) per catalog skill keyed on the hero workers' learned skill ids; full-live after next gateway rebuild + reseed | `lib/api/evals.ts`; `routes_evals.rs` (commit `53933f011`); `argo-demo-seed/src/skill_ir_eval.rs` |
| Audit export (CSV/signed) | 🟡 Partial | Real export-preview under demo-live (NO-THEATER flip, commit `2d5bd6b06`); export action, not a render surface | `lib/api/audit-export.ts` |
| Elevated-view (break-glass) | 🟧 Scaffold | Mock-on-bypass | `lib/api/elevated-view.ts` |
| Skill-IR drill-in (view/save/export) | 🟡 Partial | **Data seeded + 404 fixed** — `argo_demo_seed::seed_skill_ir` persists a real `SkillGraph` per catalog skill via the canonical canvas writer (`save_edited_graph`), keyed on the same id `/skills` lists. The residual live 404 ("skill has no IR") was NOT an id mismatch (ids are byte-identical) but a **tenant-resolution divergence**: a header-less IR GET fell back to `auth.tenant_id` (nil `0000…` under dev-bypass) while `/api/v1/skills` falls back to the cwd workspace tenant where the seed wrote → different tenants → miss. Fix (commit `8a5fb0bd0`): `routes_skill_ir_helpers::resolve_tenant` now resolves the cwd workspace tenant on a header-less request, identical to the list endpoint; cross-tenant `as_tenant` gate preserved. Proven by 2 dispatcher tests (seed→catalog-id→GET found; header-less GET 200). Full-live after next gateway rebuild + reseed | `lib/api/skill-ir*.ts`; `crates/argo-cli/src/handlers/gateway/routes_skill_ir_helpers.rs`; `argo-demo-seed/src/skill_ir_eval.rs` |
| Retros panel | 🟡 Partial | Seeded retros un-hidden under demo-live (NO-THEATER flip, commit `2d5bd6b06`) | `lib/api/retros.ts` |
| Retrain "recent changes" panel | ✅ Live | Backend shipped: `GET /api/v1/workers/<id>/retrain/recent` + real retrain-history seed (commit `5cae58294`). Live (gateway rebuilt + verified 2026-05-30, commit `ccfa89588`) | `lib/api/retrain.ts`; `routes_retrain.rs`, `routes_retrain_helpers.rs` |
| Ghostwriter panel | 🟧 Scaffold | Own mock module; no live backend | `lib/api/ghostwriter.ts` |

---

## 4 · Integrations · channels · connectors · executors

| Feature | Status | What's real / what's not | Source |
|---|---|---|---|
| Slack (approvals) | ✅ Live | Both ways — Block Kit buttons out, HMAC-verified webhook in | `argo-channels/slack.rs` |
| Discord (approvals) | ✅ Live | Both ways — embeds+buttons out, Ed25519-verified in | `argo-channels/discord.rs` |
| SMS — Twilio (approvals) | ✅ Live | Both ways (High-risk gated); inbound reply route | `argo-channels/sms.rs` |
| Voice/telephony — Twilio DTMF (approvals) | ✅ Live | Both ways — TwiML out, signed DTMF callback in. **Touch-tone, not conversation** | `argo-channels/voice.rs` |
| iMessage — BlueBubbles (approvals) | ✅ Live | Both ways; real client crate | `argo-channels/imessage.rs` |
| Web inbox (approvals) | ✅ Live | Both ways + WS event stream | `argo-channels/webinbox.rs` |
| Push — Expo (approvals) | ✅ Live | Both ways; device registry + deep-links | `argo-channels/push.rs` |
| Microsoft Teams (approvals) | 🟡 Partial | **Outbound only** (Graph adaptive cards); inbound out of scope; no env-dispatch arm (deferred-v1.x) | `argo-channels/teams.rs` |
| Email — Postmark (approvals) | 🟡 Partial | Outbound real; **inbound is a stub** (no IMAP/SES) | `argo-channels/email.rs` |
| WhatsApp — Twilio | 🟡 Partial | Inbound real; `send()` exists but **no production outbound dispatch arm** (deferred-v1.x) | `argo-channels/whatsapp.rs` |
| Matrix | 🟧 Scaffold | Real send impl but neither outbound dispatch nor inbound route wired (deferred-v1.x) | `argo-channels/matrix.rs` |
| MCP connectors — ~30 real | ✅ Live | Subprocess binaries w/ real HTTP: slack, gmail, gcal, notion, hubspot, jira, linear, salesforce, stripe, zendesk, asana, github-issues, … (6 have live integration tests) | `crates/argo-mcp-connector-*` |
| MCP connectors — apollo/zoominfo/zoom | 🟧 Scaffold | Empty orphan dirs (no Cargo.toml); enterprise placeholders, deferred-v1.x | `crates/argo-mcp-connector-{apollo,zoominfo,zoom}` |
| MCP connectors — signal/telegram/mailgun/resend | 🟧 Scaffold | Empty orphan dirs; consumer stubs slated for deletion | (empty dirs) |
| Document executors (Office/MD/CSV/PDF + Google Workspace) | ✅ Live | Real local + MS Graph + Drive/Docs/Sheets/Slides writes | `argo-executor-document*`, `-gsuite`, `-csv` |
| Filesystem executor | ✅ Live | Real local sandbox executor | `argo-executor-filesystem` |
| Computer-use — macOS | ✅ Live | Real (screencapture + osascript + CU loop); wired; claim-5 smoke passes; macOS-first (DEC-002) | `argo-executor-computer-use-macos` |
| Computer-use — E2B (cloud) | 🟡 Partial | Real e2b.dev client but deferred-v1.x (not the demo path) | `argo-executor-computer-use/e2b.rs` |
| Computer-use — Linux / Windows | 🟧 Scaffold | Real impls compile but 0 tests, deferred-v1.x | `argo-executor-computer-use-{linux,windows}` |

---

## 5 · AI agents · verticals · Genesis · voice

| Feature | Status | What's real / what's not | Source |
|---|---|---|---|
| Pitch Agent (Portia) — text Q&A | ✅ Live | Canonical IDF lookup → live LLM fallback → truth-boundary swap → 2 audit rows; SPA page + client | `routes_pitch_ask*.rs`; `packs/pitch-agent/` |
| GTM Engineer (Grant) — qualify + draft | ✅ Live | Deterministic Hot/Warm/Cold qualification + live LLM draft + truth-boundary + writes approval; never auto-sends | `routes_gtm_draft_followup.rs`; `verticals/gtm-engineer/` |
| Genesis pipeline (charter → vertical) | ✅ Live | 12-archetype classifier + manifest/prompt/eval-seed gen + safety-judge + dry-run-first-24h; round-trips through `load_dir` | `argo-genesis` |
| Genesis SSE + classify + draft-worker | ✅ Live | All 3 routes wired; real SPA SSE consumer | `routes_genesis_stream.rs` |
| Genesis LLM refinement | 🟡 Partial | `refine.rs` real but opt-in; shipped path is deterministic archetype | `argo-genesis/refine.rs` |
| Other 30+ verticals (support, recruiter, finance-ar, sales-sdr, legal, …) | 🟧 Scaffold | **Template packs only** (workflow.toml + skills + eval seeds), loadable, but **no wired live endpoint / verified LLM execution**. Only Portia + Grant have endpoints | `verticals/*/`, `packs/*/` |
| Worker retrain flow | ✅ Live | `POST .../retrain/{classify,commit,undo,reverse-test}` + **`GET /retrain/recent`** (recent-changes history) all real + wired; real retrain-history seed (commit `5cae58294`) | `routes_retrain.rs`, `routes_retrain_helpers.rs` |
| `argo-voice-io` (L1 — STT/TTS traits + turn-taking) | 🟡 Partial | V1 redesign: clean `SpeechToText`/`TextToSpeech` async traits + key-free `MockStt`/`MockTts` + `voice_turn` composition seam (commit `4e891c0b6`). **No longer an orphan** — wired into the gateway via V2 below. Real vendor adapter = 🚧 (next row) | `crates/argo-voice-io` |
| Gateway voice routes (L2) — `POST /api/v1/voice/{tts,stt,pitch-turn}` | ✅ Live | V2: routes wired into the dispatcher; `pitch-turn` reuses the **real truth boundary** in-process (over-claim → defers, no `$` figure). Provider seam reads `ARGO_VOICE_PROVIDER` (Mock provider today; real OpenAI adapter = next row). Live (gateway rebuilt + verified 2026-05-30, commit `ccfa89588`) | `routes_voice_io.rs`, `routes_voice_io/provider.rs` (commit `5a98d63dc`) |
| Real voice vendor (OpenAI STT `/v1/audio/transcriptions` + TTS `/v1/audio/speech`) | 🟡 Partial | **Adapter SHIPPED** (DEC-021): `OpenAiStt`/`OpenAiTts` impl the L1 `SpeechToText`/`TextToSpeech` traits — real multipart whisper-1 transcription + tts-1 mp3 synthesis, Bearer `OPENAI_API_KEY` (read `Redacted`), single shared `reqwest::Client`. Vendor- **and** user-selectable: `Provider::OpenAi` arm + per-tenant `voice` workspace_settings group (`GET/PATCH /api/v1/voice/settings`, default `mock`) preferred over `ARGO_VOICE_PROVIDER`. No-fake: a chosen real provider with no key is a 503, never mock audio. Unit + trait + selector tests green; **live network path untested (no `OPENAI_API_KEY` present)** — key-gated round-trip test skips without one. Realtime socket = later latency upgrade | `routes_voice_io/openai.rs`, `routes_voice_io/provider.rs`, `routes_workspace_settings.rs`; DEC-021 |
| Browser push-to-talk on `/pitch` (L3) | 🗓️ Planned | Depends on V2 routes (now exist); mic capture + audio playback in the SPA | DEC-021; build-out tracker V3 |
| `argo-voice-tui` (local hands-free approvals) | ✅ Live | whisper.cpp + wake-word + intent/fuzzy + confirm; wired to CLI. **Voice for approvals, not agent speech** | `crates/argo-voice-tui` |
| Voice MCP server (`mcp-servers/voice`) | 🟡 Partial | openai/elevenlabs cloud backends real; whisper/piper "not wired"; **server unregistered** (not in runtime) | `mcp-servers/voice/` |
| **Live agent voice on Zoom/Teams/Meet calls** | 🗓️ Planned | **None in code.** `live-meeting.toml` describes intent; no join_meeting/WebRTC/Zoom-SDK executor. (Track P P-07, ~8wk, macOS-only) | `packs/pitch-agent/workflows/live-meeting.toml` (manifest only) |

---

## 6 · Roadmap — planned / in-progress / backlog (with phase + timeline)

| Area | Feature | Status | Phase / timeline | Source |
|---|---|---|---|---|
| Demo | v1.0 mission demo MP4 (6-scene, real backend) — ship gate | 🗓️ Planned | v1.0 ship gate — **operator-only**, last gate item | CLAUDE.md (d); DEC-007 |
| Voice | Pitch Agent live-meeting orchestrator (Zoom-on-macOS + system-audio bridge + CU screen-share) | 🚧 In-progress | Track P P-07; ~8 wk full swarm; macOS-only | BUILD_STATUS `pitch-meeting-orchestrator-zoom-channel` (BLOCKED); DEC-008 |
| Voice | Real STT/TTS provider plugin — **OpenAI for launch** (discrete whisper-1 + tts-1) behind the `argo-voice-io` trait | 🟡 Partial | Sprint 2 (DEC-021) — adapter shipped + vendor/user-selectable; live key-path untested; Realtime socket = later latency upgrade | DEC-021; `routes_voice_io/openai.rs`; build-out tracker V2; MISSION_BACKLOG #162 |
| Web | Pitch Agent public `/pitch` web-embed widget + WebSocket session API | 🗓️ Planned | Track P P-06 (DROPPED via drift-sweep; re-author if wanted) | BUILD_STATUS `pitch-skycot-web-embed-channel` |
| Genesis | Phase-2 multi-modal onboarding: voice-role input, doc/SOP upload → memory ingest, integration import (Slack/Notion/Drive/M365), access-controls UI, comprehension quiz | 🚧 In-progress / 🗓️ Planned | Genesis Phase 2 (P0); blocked on Phase-1 SPA chain | DEC-010; `mission-genesis-*` briefs (BLOCKED) |
| Platform | Retrain recent-changes log + `GET /retrain/recent` + seed | ✅ Done | shipped — see §5 worker-retrain | commit `5cae58294` |
| Platform | Settings + evals gateway backends (SPA pages were on mock) | ✅ Done | shipped — see §3 (settings, evals); api-keys/provider-keys still SKIP (privacy) | commits `423274a25` (settings), `53933f011` (evals) |
| Eval | Teammate-perf eval — historical-replay scenario source (real customer data) | 🗓️ Planned | Phase-2 seam; switch on with a design-partner | DEC-017; `TEAMMATE_PERFORMANCE_EVAL_HARNESS_DESIGN.md` |
| Compute | Non-macOS computer-use hardening (E2B / Linux-Wayland / Windows) | 🟧 Scaffold | DEFERRED-v1.x | DEC-002; `claude-defer-non-macos-computer-use` (#1642) |
| Connectors | Enterprise MCP hardening (Apollo/ZoomInfo/Zoom) | 🟧 Scaffold | DEFERRED-v1.x | DEC-001 |
| Channels | Teams/Matrix/iMessage auto-approval dispatch routing | 🟧 Scaffold | DEFERRED-v1.x | MISSION_BACKLOG #116/#125/#127 |
| Channels | N-of-M multi-step approval (signing) | 🗓️ Planned | v1.1 / governance v2.1 | MISSION_BACKLOG #118 |
| Channels | Quiet-hours / per-operator prefs / escalation-on-no-reply / bundle UI | 💡 Backlog | Phase D (ongoing) | MISSION_BACKLOG #120-130 |
| Capability | Sandboxed filesystem + code-execution (Python/JS) sandbox executors | 💡 Backlog | Phase v1.2 | BUILD_PLAN_V11 §v1.2 |
| Connectors | Connector volume catch-up (Box/OneDrive/Xero/Lever/WhatsApp-Cloud/…) | 💡 Backlog | Phase v1.2 / Phase D | BUILD_PLAN_V11 §v1.2 |
| Orchestration | Multi-worker orchestration finish — **Supervisor model** (parallel fan-out + dependency layers over the handoff chain), not visual DAG | 🟡 Partial | Engine shipped (`argo_orchestrator::Supervisor::run_team` — bounded-concurrency fan-out + `depends_on` topological layers + failure-drops-dependents; `POST /api/v1/orchestration/team-runs`; children persist as existing orchestration-run rows, tenant-scoped) — commit `de3c58e2e`. SPA team-tree view (design plan 02 Phase 3) SHIPPED — `/orchestration` renders team runs as a lead-with-children tree, commit `59aa69080`. **Concurrent-team grouping now precise** (this commit; see `git log`): `run_team` mints one run-wide `team_run_id` threaded through every `StepContext` + the `StepExecutor::synthesize` call, title-encoded as `[team:<id>]` (NO migration, NO schema change), so the SPA groups concurrent team runs into separate trees by id. Remaining: real LLM-backed `StepExecutor` (today a persisting executor records rows; honest seam, additive); rosters/skill-auction/checkpoint-resume later (v1.3) | DEC-022; design plan 02; `crates/argo-orchestrator/src/team_run.rs`; `web/components/skycot/orch/TeamRunTree.tsx`; BUILD_PLAN_V11 §v1.3 |
| Self-improve | Skill A/B + regression, training-data export, feedback→Dreams, cross-tenant trust graph | 💡 Backlog | Phase v2.0 (2-3 mo) | BUILD_PLAN_V11 §v2.0 |
| Governance | HIPAA BAA, gateway-edge SSO SAML/OIDC, SCIM, BYOK, data-residency, IP-allowlist | 💡 Backlog | Phase v2.1 / Phase C (4-6 wk) | BUILD_PLAN_V11 §v2.1 |
| Platform | Production perimeter (TLS termination, per-tenant rate-limit, per-request audit, health-check) | 💡 Backlog | Phase C (4-6 wk) | MISSION_BACKLOG #20-23 |
| Vertical | Demo-vertical depth + new "any role" verticals | 💡 Backlog | Phase v2.2 / Phase F (3-6 mo) | BUILD_PLAN_V11 §v2.2 |
| Platform | PostgreSQL / horizontal scale (ADR shipped, impl deferred) | 💡 Backlog | Phase F (3-6 mo) | MISSION_BACKLOG #29/#241 |

---

## 6a · Live voice build-out — the path to "Portia pitches on a call"

The capability the operator wants (Portia speaking live, incl. on Zoom/Teams)
is **partly built** as of 2026-05-30: the trait layer (L1) + the truth-bounded
gateway voice-loop (L2) are shipped against a mock provider; the **real OpenAI
adapter is in-progress** (DEC-021). Here is the scoped path, **in build order**.
Each layer is independently demoable; truth-boundary + audit (already real in
text) carry through every layer unchanged.

| Layer | What it unlocks | Status | Rough effort | Depends on | Approach |
|---|---|---|---|---|---|
| **L1 — Voice trait layer + provider seam** | Clean STT/TTS contracts + `voice_turn` seam (mock today, real vendor pluggable) | ✅ Live | done (`4e891c0b6`) | `argo-voice-io` | Redesigned `SpeechToText`/`TextToSpeech` async traits + key-free `MockStt`/`MockTts` + `voice_turn` composition. Real OpenAI adapter slots in behind this (DEC-021). |
| **L1b — Real voice provider (OpenAI)** | Portia can hear + speak (real audio in/out) | 🟡 Partial (adapter shipped; live key-path untested) | this sprint | L1 | OpenAI for launch (DEC-021): `OpenAiStt`/`OpenAiTts` impl the L1 traits — discrete `POST /v1/audio/transcriptions` (whisper-1, multipart) + `/v1/audio/speech` (tts-1, mp3) with Bearer `OPENAI_API_KEY`. Selected by `Provider::OpenAi` via the per-tenant `voice` workspace_settings group then `ARGO_VOICE_PROVIDER`; no-key ⇒ 503 (no fake). Unit/trait/selector tests green; live round-trip is key-gated (skips with no key). Realtime socket = later latency upgrade. (`routes_voice_io/openai.rs`) |
| **L2 — Agent voice-loop** | Live spoken Q&A, **still truth-bounded** | ✅ Live (mock) | done (`5a98d63dc`) | L1 + `/api/v1/pitch/ask` | `POST /api/v1/voice/{tts,stt,pitch-turn}`; `pitch-turn` reuses Portia's real pitch logic (canonical + live LLM + truth-boundary) in-process → over-claim defers. `argo-voice-io` is now wired (no longer an orphan). Becomes real-audio once L1b lands. |
| **L3a — Browser voice (fast-path demo)** | "Portia speaks" in a browser — **no meeting bot** | 🗓️ Planned | ~1 wk after L1b | L1b + L2 | Add mic capture + audio playback to `/pitch` (or the web-embed widget). **Fastest path to a live-VOICE investor demo** — recommended first milestone. |
| **L3b — Live meeting join (Zoom/Teams/Meet)** | Portia joins a real call as a participant | 🗓️ Planned | ~3–5 wk | L2 | **Fastest:** Recall.ai meeting-bot SaaS (joins the call, streams call audio out + injects Portia's TTS back). **Native:** Zoom Meeting SDK / MS Teams bot framework. **Or** Track-P P-07: Zoom-on-macOS via computer-use + a system-audio bridge (already partly designed). |

**Recommended sequence:** L1 → L2 → **L3a (browser voice — ship the demo)** →
L3b (meeting join, Recall.ai first). L1+L2+L3a gives a genuinely live, voice,
truth-bounded Portia demo in ~4–6 weeks without the heaviest piece (meeting
bot). Existing assets that de-risk this: the `argo-voice-io` abstraction
(manifests/session/turn-taking), the real openai/elevenlabs backends in
`mcp-servers/voice`, and Portia's already-real truth-bounded pitch endpoint.

## 6b · Competitive gaps (vs. 2025-26 AI-agent leaders)

From the verified competitive analysis ([`docs/skycot/research/2026-05-29-competitive-gap-analysis.md`](../skycot/research/2026-05-29-competitive-gap-analysis.md) — 26 sources, adversarially verified; 3 vendor claims killed). These are the features the field now ships that we lack — tracked here as roadmap items. **Don't over-correct:** our audit + truth-boundary + approval-first + hire-a-teammate framing are differentiators no competitor matched.

> **Ready-for-go implementation plans** for the top 3 gaps (voice, orchestration, self-improvement) live at [`docs/skycot/design/2026-05-29-gap-closure-plans/`](../skycot/design/2026-05-29-gap-closure-plans/README.md) — each flags the operator decision(s) needed, then drops into the swarm as briefs.

| Gap | Status | Priority | Who has it | SKYCOT today | Maps to |
|---|---|---|---|---|---|
| Dual agent-identity (delegated on-behalf-of vs. autonomous) + cross-runtime governance + behavioral threat-detection | 🟡 Partial | **Highest** | MS Agent 365, Salesforce | "Package existing moat first" move A shipped: `GET /api/v1/governance/readiness` (commit `764f725ff`). Still: no dual-identity / cross-runtime / threat-detection | [plan 04](../skycot/design/2026-05-29-gap-closure-plans/04-enterprise-governance-and-audit-readiness.md) — move A done; rest backlog |
| Multi-worker orchestration — parallel fan-out + dependency layers, handoff/routing/escalation, nested supervisors | 🗓️ Planned | High | Relevance, Dust, LangGraph, Beam, Ema | flat 1-to-1 delegation today; **Supervisor model locked** (DEC-022) for Sprint 2 | DEC-022; §6 orchestration row; design plan 02 |
| Self-improvement / feedback-mining loop (human-feedback training-data export + production-trace clustering) | 🟡 Partial | Med-High | CrewAI, LangSmith | **`argo eval export-training-data` ships SFT-JSONL export of decided approvals + committed retrains (2026-05-29, `7e84ba7d6`)**; **Step 1b `argo eval generate-cases` turns the same judged history into argo-eval regression cases (rejected→must-refuse/defer, retrains→corrected-behaviour) so judged-bad behaviour becomes a held-to test (2026-05-30, `99114bd5a`)**; trace-clustering (Step 2) still backlog | [plan 03](../skycot/design/2026-05-29-gap-closure-plans/03-self-improvement-training-export.md) |
| Live conversational voice (real STT/TTS provider behind the stub) | 🟡 Partial | High | ElevenLabs, Bland, Vapi, Retell | L1 trait layer + mocks (`4e891c0b6`) + L2 gateway voice routes, truth-bounded (`5a98d63dc`) live; **real OpenAI adapter shipped** (vendor- + user-selectable; live key-path untested) — DEC-021 | §6a L1–L2; DEC-021; `routes_voice_io/openai.rs` |
| Multi-modal Genesis onboarding (doc/SOP/wiki upload + comprehension quiz) | 🗓️ Planned | Medium | Ema, Relevance | text-only Genesis | §6 Genesis Phase 2 |
| Audit-readiness / compliance packaging — make our audit + truth-boundary legible to buyers | 🟡 Partial | Medium (defend) | (borrow Agent 365's framing for OUR moat) | `GET /api/v1/governance/readiness` packages chain-integrity + approval-coverage + truth-boundary into one buyer-legible report (commit `764f725ff`); UI surface still to come | `routes_governance.rs`; plan 04 |

> **Thinner than they look:** Relevance's self-improvement and Beam's governance-suite claims **failed** verification, and Bland's "1M concurrent calls" was refuted — so our relative position on self-improvement + governance-breadth is better than vendor pages suggest. **Genuinely ahead (defend + market): hash-chained audit, claim-level truth-boundary, 9-channel approval-first HITL, hire-a-teammate framing — no researched competitor matched these.**

## 7 · Key honesty callouts (read before pitching capabilities)

1. **No live voice on calls yet.** Portia/Grant are real in **text**. The voice
   trait layer (L1) + a truth-bounded gateway voice-loop (L2, `POST /api/v1/voice/*`)
   are live. The **real OpenAI STT/TTS adapter is now shipped** (DEC-021, discrete
   whisper-1 + tts-1, vendor- + user-selectable) — but its **live network path is
   untested** (no `OPENAI_API_KEY` present at build time; the key-gated round-trip
   test skips), and the default provider is still the mock until a tenant/operator
   selects `openai`. There is still **no meeting-join/WebRTC layer**, so spoken
   conversation on Zoom/Teams/Meet remains unbuilt. Order to a live-voice demo:
   verify OpenAI adapter with a key → browser push-to-talk (L3a) → meeting-bot
   (e.g. Recall.ai or Zoom SDK).
2. **30+ verticals are template packs, not running agents.** Only **Portia** (pitch)
   and **Grant** (gtm) have wired live endpoints + verified LLM execution.
3. **The mock-in-demo-live SPA cluster is shrinking but not gone.** Now **live**
   (gateway rebuilt + verified 2026-05-30, commit `ccfa89588`): settings
   workspace/providers/notifications, governance readiness, retrain-recent,
   audit-export preview, retros. Evals is **live-but-empty** (endpoint live;
   `skill_reeval_history` not yet seeded — follow-up). Still mock / scaffold:
   api-keys + provider-keys (SKIP — secret-exposure privacy rule), ghostwriter,
   skill-IR (needs IR seeded), elevated-view (needs SPA↔backend validation
   alignment), worker schedules/escalation.
4. **Two "core invariants" are scaffold:** the JSON-RPC plugin ABI and the teammate-eval
   6-axis scorer are tested code with **zero live consumers**.
5. **Architecture caveat:** the gateway is a **hand-rolled HTTP/1.1 string-dispatcher**
   (not axum/hyper) — works, but parse-fragile; worth knowing for reliability talks.
6. **Channels are approval-notification channels**, not general chat — they send
   approval prompts + receive approve/reject, across 9 wired transports.

---

## 8 · How this stays the source of truth

- **Location:** `docs/build-plan/FEATURE_MATRIX.md` (this file). Linked from `CLAUDE.md`
  (Plan documents), `docs/build-plan/RESUME.md`, and `docs/build-plan/PRODUCT_ROADMAP.md`.
- **Update discipline (load-bearing):** when a feature flips status (ships, gets wired,
  gets deferred), update its row **in the same commit/cycle that ships the feature** —
  and cite the commit SHA + crate/route/doc so the claim stays verifiable. Move the row
  between §2-5 (current) and §6 (roadmap) as appropriate. The mirror rule for operator
  decisions: every operator-locked call gets a `docs/skycot/decisions/` ADR in the same
  cycle. Build-agent prompts must carry both rules (see `docs/skycot/overnight-build-out.md`).
- **Periodic re-verify:** re-run the parallel code-grounded audit (§ method at top)
  before any investor/customer capability claim, or at each phase boundary. Trust
  `git log` + the cited source over any prose elsewhere.
- **Relationship to other docs:** `BUILD_STATUS.md` = live work ledger (what's being
  built now); `MISSION_BACKLOG.md` = raw un-shipped queue; `PRODUCT_ROADMAP.md` =
  product-surface program. **This matrix is the rollup** — the honest "what exists and
  what's coming" across all of them.
