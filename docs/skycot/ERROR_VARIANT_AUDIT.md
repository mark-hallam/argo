# `ArgoError` variant-usage audit

**Generated**: 2026-05-09 by `scripts/check-error-variant-usage.sh` against
`crates/argo-types/src/error.rs` canonical contract.

This audit lists call sites where the chosen `ArgoError` variant likely
does not match the canonical contract (see the module-level docs on
`ArgoError`). It is the input for the follow-up brief
`claude-error-types-rewrite-callsites`, which will mechanically apply the
recommendations and flip the CI guard from warning to fail.

## Methodology

The audit is a **heuristic** scan — `scripts/check-error-variant-usage.sh`
greps the workspace for variant-vs-message pairs that are statistically
inconsistent with the contract:

- `Provider(format!("read …"))` etc. → I/O on local files, not a
  downstream service. Recommended: `Io`.
- `Provider(format!("invalid …"))` etc. → caller-supplied bad input.
  Recommended: `Schema`.
- `Provider(format!("uuid …"))`, `decode …`, `timestamp …` → internal
  data corruption / impossible-state branches. Recommended: `Invariant`.

False positives are possible. The rewrite brief judges each site
individually. The script's `error.rs` self-references are filtered.

## Summary

Headline counts (2026-05-09 snapshot — run the script for live totals):

| Recommended variant | Sites |
|---|---|
| `Io` (local FS / process I/O) | 315 |
| `Schema` (caller-supplied bad input) | 76 |
| `Schema` (explicit 'missing field' phrasing) | 0 |
| `Invariant` (internal data corruption / unreachable) | 15 |
| **Total** | **406** |

Per-site table is a snapshot; counts drift. The rewrite brief re-runs
the script and judges each then-current site.

## Notes for the rewrite brief

- The `From<std::io::Error>` impl already exists; most
  `Provider(format!("read {p}: {e}"))` rewrites collapse to `?`.
- sqlx `Provider(format!("audit insert: {e}"))` sites are not flagged:
  no dedicated `Storage` variant exists. The rewrite brief decides
  whether to add one or fold into `Io`.
- A few hits are in `*tests*.rs` exercising the gateway-mapping path
  and may be left as `Provider`. Inspect each before rewriting.

## Per-site detail

### Recommend `ArgoError::Io` — local FS / process I/O

_Rationale: These call sites wrap local I/O failures (`std::io`, `std::fs`, `std::env::current_dir`, etc.) in `ArgoError::Provider`, but `Provider` is reserved for downstream-service failures. Use `ArgoError::Io` (or rely on the `From<std::io::Error>` impl)._

Site count: **315**

| Site (path:line) | Excerpt |
|---|---|
| `crates/argo-skill/src/ab_test.rs:193` | `ArgoError::Provider(format!("create ab run: {e}")` |
| `crates/argo-marketplace/src/bundle.rs:71` | `ArgoError::Provider(format!("write tar {}: {e}", out_tar.display()` |
| `crates/argo-marketplace/src/bundle.rs:117` | `ArgoError::Provider(format!("write tar {}: {e}", out_tar.display()` |
| `crates/argo-marketplace/src/bundle.rs:157` | `ArgoError::Provider(format!("read tar entries: {e}")` |
| `crates/argo-marketplace/src/bundle.rs:176` | `ArgoError::Provider(format!("read tar body: {e}")` |
| `crates/argo-marketplace/src/bundle.rs:179` | `ArgoError::Provider(format!("write {}: {e}", abs.display()` |
| `crates/argo-marketplace/src/bundle.rs:208` | `ArgoError::Provider(format!("read {}: {e}", p.display()` |
| `crates/argo-marketplace/src/bundle.rs:229` | `ArgoError::Provider(format!("read_dir {}: {e}", dir.display()` |
| `crates/argo-marketplace/src/bundle.rs:264` | `ArgoError::Provider(format!("read {}: {err}", abs.display()` |
| `crates/argo-marketplace/src/oci_distribution.rs:211` | `ArgoError::Provider(format!("read tar entries: {e}")` |
| `crates/argo-marketplace/src/oci_distribution.rs:225` | `ArgoError::Provider(format!("read tar body: {e}")` |
| `crates/argo-agent-runner/src/transport.rs:91` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-delegate/src/workspace.rs:169` | `ArgoError::Provider(format!("mkdir {}: {e}", parent.display()` |
| `crates/argo-delegate/src/workspace.rs:181` | `ArgoError::Provider(format!("mkdir {}: {e}", parent.display()` |
| `crates/argo-delegate/src/workspace.rs:184` | `ArgoError::Provider(format!("copy {} → {}: {e}", src.display()` |
| `crates/argo-delegate/src/workspace.rs:192` | `ArgoError::Provider(format!("read {}: {e}", p.display()` |
| `crates/argo-delegate/src/workspace.rs:207` | `ArgoError::Provider(format!("read_dir {}: {e}", dir.display()` |
| `crates/argo-identity/src/roster.rs:381` | `ArgoError::Provider(format!("remove roster member: {e}")` |
| `crates/argo-store/src/worker_budget_write.rs:115` | `ArgoError::Provider(format!("read worker policy: {e}")` |
| `crates/argo-store/src/lib.rs:262` | `ArgoError::Provider(format!("create idx_session_worker: {e}")` |
| `crates/argo-config/src/lib.rs:446` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-config/src/cred.rs:67` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-config/src/cred.rs:153` | `ArgoError::Provider(format!("create {}: {e}", parent.display()` |
| `crates/argo-config/src/cred.rs:167` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/home.rs:43` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/main.rs:242` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_onboard.rs:75` | `ArgoError::Provider(format!("read {}: {e}", persona_spec.display()` |
| `crates/argo-cli/src/handlers/worker_onboard.rs:231` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_charter.rs:167` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_anomalies.rs:26` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/work_jobs.rs:46` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/work_jobs.rs:248` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/work_jobs.rs:302` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/cred_import.rs:23` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session.rs:373` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session.rs:475` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/agents.rs:31` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/agents.rs:43` | `ArgoError::Provider(format!("read {}: {e}", toml_path.display()` |
| `crates/argo-cli/src/handlers/agents.rs:199` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/agents.rs:212` | `ArgoError::Provider(format!("read {}: {e}", toml_path.display()` |
| `crates/argo-cli/src/handlers/agents.rs:239` | `ArgoError::Provider(format!("write {}: {e}", toml_path.display()` |
| `crates/argo-cli/src/handlers/budget_watch.rs:69` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_channels.rs:219` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/handoff_policy.rs:135` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/handoff_policy.rs:228` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/audit_replay.rs:35` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/forge_subject.rs:17` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/forge_by_skill.rs:10` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/action_history.rs:279` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/messages_by_day.rs:10` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/handoff_auctions_cancel.rs:10` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/skill_create.rs:20` | `ArgoError::Provider(format!("read {path}: {e}")` |
| `crates/argo-cli/src/handlers/skill_create.rs:49` | `ArgoError::Provider(format!("read stdin: {e}")` |
| `crates/argo-cli/src/handlers/cred_list.rs:19` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_capability_gaps.rs:24` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/budget.rs:130` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_export.rs:22` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_export.rs:34` | `ArgoError::Provider(format!("create {path}: {e}")` |
| `crates/argo-cli/src/handlers/cron.rs:15` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/cron.rs:127` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/cron.rs:145` | `ArgoError::Provider(format!("read_dir {}: {e}", dir.display()` |
| `crates/argo-cli/src/handlers/cron.rs:153` | `ArgoError::Provider(format!("read {}: {e}", p.display()` |
| `crates/argo-cli/src/handlers/cron.rs:206` | `ArgoError::Provider(format!("read_dir {}: {e}", dir.display()` |
| `crates/argo-cli/src/handlers/cron.rs:303` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/cron.rs:314` | `ArgoError::Provider(format!("remove {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/audit_by_kind.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/onboard_stages.rs:87` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/onboard_stages.rs:136` | `ArgoError::Provider(format!("write argo.toml: {e}")` |
| `crates/argo-cli/src/handlers/onboard_stages.rs:140` | `ArgoError::Provider(format!("mkdir {}: {e}", p.display()` |
| `crates/argo-cli/src/handlers/skill_import.rs:112` | `ArgoError::Provider(format!("read skill artifact entries: {e}")` |
| `crates/argo-cli/src/handlers/skill_import.rs:115` | `ArgoError::Provider(format!("read skill artifact entry: {e}")` |
| `crates/argo-cli/src/handlers/skill_import.rs:127` | `ArgoError::Provider(format!("read {path}: {e}")` |
| `crates/argo-cli/src/handlers/skill_import.rs:178` | `ArgoError::Provider(format!("read {path}: {e}")` |
| `crates/argo-cli/src/handlers/health.rs:150` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/forge_by_day.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_turns.rs:20` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_doctor.rs:81` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/route.rs:58` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/costs_recent.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/tools.rs:25` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/tools.rs:99` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_messages_rollups.rs:17` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/resume.rs:13` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/tenant.rs:171` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/audit.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_transitions.rs:146` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_transitions.rs:164` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/steer.rs:21` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/costs_by_session.rs:12` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/handoff_rules_list.rs:122` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/tenant_archive.rs:39` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/audit_tool_errors.rs:13` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/eval_run.rs:277` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/recover_cache.rs:26` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/memory_dream_now.rs:24` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/skill_runs.rs:31` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/subtask_list.rs:19` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/forge_enqueue.rs:18` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/audit_by_day_by_kind.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/messages_by_session.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/action_replay.rs:37` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/action_replay.rs:335` | `ArgoError::Provider(format!("create replay dir {}: {e}", parent.display()` |
| `crates/argo-cli/src/handlers/action_replay.rs:339` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/audit_by_session.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/memory.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/memory.rs:474` | `ArgoError::Provider(format!("write {}: {e}", tmp.display()` |
| `crates/argo-cli/src/handlers/memory.rs:485` | `ArgoError::Provider(format!("read {}: {e}", tmp.display()` |
| `crates/argo-cli/src/handlers/skill_review.rs:132` | `ArgoError::Provider(format!("read operator feedback: {e}")` |
| `crates/argo-cli/src/handlers/plan_generate.rs:74` | `ArgoError::Provider(format!("mkdir {}: {e}", parent.display()` |
| `crates/argo-cli/src/handlers/plan_generate.rs:77` | `ArgoError::Provider(format!("write {path}: {e}")` |
| `crates/argo-cli/src/handlers/plan_generate.rs:91` | `ArgoError::Provider(format!("write {}: {e}", sidecar.display()` |
| `crates/argo-cli/src/handlers/session_diff.rs:16` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_costs.rs:77` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/skill_export.rs:65` | `ArgoError::Provider(format!("create {}: {e}", parent.display()` |
| `crates/argo-cli/src/handlers/skill_export.rs:109` | `ArgoError::Provider(format!("write {output}: {e}")` |
| `crates/argo-cli/src/handlers/skill_export.rs:212` | `ArgoError::Provider(format!("create {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/skill_export.rs:258` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/plan_template_list.rs:13` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/messages_by_role.rs:10` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/provider.rs:23` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/provider.rs:29` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/provider.rs:108` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/provider.rs:118` | `ArgoError::Provider(format!("read_dir {}: {e}", dir.display()` |
| `crates/argo-cli/src/handlers/provider.rs:134` | `ArgoError::Provider(format!("read {}: {e}", p.display()` |
| `crates/argo-cli/src/handlers/provider.rs:165` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/provider.rs:174` | `ArgoError::Provider(format!("read_dir {}: {e}", manifests_dir.display()` |
| `crates/argo-cli/src/handlers/provider.rs:182` | `ArgoError::Provider(format!("read {}: {e}", p.display()` |
| `crates/argo-cli/src/handlers/provider.rs:246` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/provider.rs:257` | `ArgoError::Provider(format!("read_dir {}: {e}", dir.display()` |
| `crates/argo-cli/src/handlers/provider.rs:327` | `ArgoError::Provider(format!("read {}: {e}", p.display()` |
| `crates/argo-cli/src/handlers/provider.rs:341` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/vertical_archive.rs:37` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/vertical_archive.rs:48` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:100` | `ArgoError::Provider(format!("create {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:109` | `ArgoError::Provider(format!("remove {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:116` | `ArgoError::Provider(format!("create {}: {e}", parent.display()` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:117` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:122` | `ArgoError::Provider(format!("create {}: {e}", ctx.home.display()` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:136` | `ArgoError::Provider(format!("read {}: {e}", dir.display()` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:137` | `ArgoError::Provider(format!("read vertical entry: {e}")` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:147` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:158` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:170` | `ArgoError::Provider(format!("create {}: {e}", ctx.home.display()` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:213` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_skills.rs:84` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_approval.rs:23` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/budget_by_skill.rs:117` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/skill_quality_recompute.rs:31` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/plan_template_show.rs:10` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/plan_template_promote.rs:12` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/forge_by_status.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/onboard.rs:18` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/session_messages.rs:25` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/audit_count.rs:17` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/audit_count.rs:59` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/vertical_run.rs:32` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/subtask_show.rs:9` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/mcp_registry.rs:168` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/alerts.rs:181` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/workspace_marketplace.rs:29` | `ArgoError::Provider(format!("read {}: {e}", out_tar.display()` |
| `crates/argo-cli/src/handlers/workspace_marketplace.rs:56` | `ArgoError::Provider(format!("write {}: {e}", out_tar.display()` |
| `crates/argo-cli/src/handlers/workspace_marketplace.rs:100` | `ArgoError::Provider(format!("mkdir {}: {e}", dest.display()` |
| `crates/argo-cli/src/handlers/subtask_tree.rs:13` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/audit_report.rs:104` | `ArgoError::Provider(format!("write {path}: {e}")` |
| `crates/argo-cli/src/handlers/forge.rs:116` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_items.rs:177` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_items.rs:258` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/skill.rs:20` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/skill.rs:237` | `ArgoError::Provider(format!("read {source}: {e}")` |
| `crates/argo-cli/src/handlers/session_bisect.rs:23` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/org.rs:106` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/org.rs:214` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/cred_export.rs:14` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/skill_quality_list.rs:24` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/trajectory_export.rs:104` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/trajectory_export.rs:111` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/trajectory_export.rs:219` | `ArgoError::Provider(format!("read {}: {e}", action_json_path.display()` |
| `crates/argo-cli/src/handlers/session_watch.rs:16` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/costs_anomalies.rs:19` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/trace.rs:147` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_executor.rs:471` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/session_by_day.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/db.rs:9` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/messages_recent.rs:16` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/backup_create.rs:53` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/classify_rules.rs:8` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/work_steps.rs:114` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/work_steps.rs:238` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_show.rs:91` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/compliance_export.rs:89` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/compliance_export.rs:131` | `ArgoError::Provider(format!("write {path}: {e}")` |
| `crates/argo-cli/src/handlers/audit_by_vertical.rs:15` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/roster_list.rs:126` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/doctor.rs:136` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_transcript.rs:21` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/skill_deps.rs:14` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/session_by_provider.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/handoff_auctions_show.rs:10` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/session_clone_branch.rs:131` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_clone_branch.rs:157` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/eval_list.rs:95` | `ArgoError::Provider(format!("read eval run {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/eval_list.rs:200` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/eval_list.rs:275` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/vertical_workflow.rs:128` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/vertical_workflow.rs:427` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/skill_search.rs:84` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/recover.rs:23` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/recover.rs:114` | `ArgoError::Provider(format!("read cache: {e}")` |
| `crates/argo-cli/src/handlers/recover.rs:145` | `ArgoError::Provider(format!("write cache: {e}")` |
| `crates/argo-cli/src/handlers/worker_budget.rs:69` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/skill_export_all.rs:16` | `ArgoError::Provider(format!("create {}: {e}", dir.display()` |
| `crates/argo-cli/src/handlers/forge_optimize.rs:102` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/plan_show.rs:16` | `ArgoError::Provider(format!("read {}: {e}", yaml_path.display()` |
| `crates/argo-cli/src/handlers/plan_show.rs:48` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/roster_create.rs:65` | `ArgoError::Provider(format!("read {label} '{}': {e}", path.display()` |
| `crates/argo-cli/src/handlers/workspace.rs:46` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/workspace.rs:263` | `ArgoError::Provider(format!("write argo.toml: {e}")` |
| `crates/argo-cli/src/handlers/workspace.rs:267` | `ArgoError::Provider(format!("mkdir {}: {e}", p.display()` |
| `crates/argo-cli/src/handlers/workspace.rs:273` | `ArgoError::Provider(format!("write {}: {e}", p.display()` |
| `crates/argo-cli/src/handlers/workflow.rs:101` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/workflow.rs:223` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/audit_by_day.rs:11` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/skill_ab.rs:168` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/skill_distill_now.rs:274` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/costs_by_day.rs:12` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/handoff_auctions_list.rs:21` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/activity_since.rs:82` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/classify_list.rs:23` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/tenant_quota_set.rs:72` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/costs_by_provider.rs:12` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/feedback_common.rs:12` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_list.rs:152` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_list.rs:163` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_query.rs:146` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/approval_policy.rs:144` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/approval_policy.rs:147` | `ArgoError::Provider(format!("create {}: {e}", dir.display()` |
| `crates/argo-cli/src/handlers/approval_policy.rs:157` | `ArgoError::Provider(format!("write {}: {e}", dest.display()` |
| `crates/argo-cli/src/handlers/approval_policy.rs:179` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/approval_policy.rs:207` | `ArgoError::Provider(format!("read {}: {e}", local.display()` |
| `crates/argo-cli/src/handlers/approval_policy.rs:270` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/approval_policy.rs:277` | `ArgoError::Provider(format!("read_dir {}: {e}", dir.display()` |
| `crates/argo-cli/src/handlers/approval_policy.rs:281` | `ArgoError::Provider(format!("read entry under {}: {e}", dir.display()` |
| `crates/argo-cli/src/handlers/worker_archive.rs:213` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/cred_clear.rs:23` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/skill_quality.rs:27` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/costs.rs:12` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/costs.rs:94` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/costs.rs:111` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/costs.rs:126` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/costs.rs:141` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/costs.rs:327` | `ArgoError::Provider(format!("create {p}: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_dispatch.rs:181` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_dispatch.rs:300` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_dispatch.rs:304` | `ArgoError::Provider(format!("create .skycot dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_dispatch.rs:446` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/judge.rs:25` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/judge.rs:68` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_replay.rs:41` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_kpi.rs:192` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/session_list.rs:26` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_list.rs:235` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/backup_restore.rs:64` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/backup_restore.rs:68` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/duty_list.rs:110` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/costs_by_model.rs:12` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_update.rs:65` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/gdpr_delete.rs:29` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/gdpr_delete.rs:71` | `ArgoError::Provider(format!("read confirmation: {e}")` |
| `crates/argo-cli/src/handlers/gdpr_delete.rs:149` | `ArgoError::Provider(format!("write {path}: {e}")` |
| `crates/argo-cli/src/handlers/new.rs:24` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/audit_by_tool.rs:13` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_by_model.rs:10` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_batch.rs:97` | `ArgoError::Provider(format!("read stdin: {e}")` |
| `crates/argo-cli/src/handlers/plugin.rs:9` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/skill_quality_trend.rs:20` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/cred_set.rs:19` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/cred_set.rs:31` | `ArgoError::Provider(format!("read credential from stdin: {e}")` |
| `crates/argo-cli/src/handlers/session_audit_rollups.rs:19` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_audit_rollups.rs:90` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/session_audit_rollups.rs:161` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/tui.rs:10` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/classify_set.rs:20` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/work.rs:79` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/work.rs:144` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/work.rs:336` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/work_artifacts/mod.rs:98` | `ArgoError::Provider(format!("create output dir: {e}")` |
| `crates/argo-cli/src/handlers/work_artifacts/mod.rs:396` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/vertical_eval_judge_backends.rs:63` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_create.rs:135` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_create.rs:377` | `ArgoError::Provider(format!("write {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/session_search.rs:20` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/handoff_rules_add.rs:44` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |
| `crates/argo-cli/src/handlers/model.rs:27` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/model.rs:34` | `ArgoError::Provider(format!("read_dir {}: {e}", dir.display()` |
| `crates/argo-cli/src/handlers/model.rs:116` | `ArgoError::Provider(format!("read {}: {e}", p.display()` |
| `crates/argo-cli/src/handlers/worker_portrait.rs:53` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/session_show.rs:15` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/forge_daemon.rs:36` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/messages_by_day_by_role.rs:10` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/policy_handlers.rs:29` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/policy_handlers.rs:174` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/mcp.rs:47` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/audit_export.rs:59` | `ArgoError::Provider(format!("write {path}: {e}")` |
| `crates/argo-cli/src/handlers/audit_export.rs:64` | `ArgoError::Provider(format!("write stdout: {e}")` |
| `crates/argo-cli/src/handlers/gateway/boot.rs:50` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/gateway/boot.rs:82` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/gateway/boot.rs:122` | `ArgoError::Provider(format!("write {}: {e}", pidfile.display()` |
| `crates/argo-cli/src/handlers/gateway/mod.rs:76` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/backup_list.rs:16` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/chat.rs:28` | `ArgoError::Provider(format!("current_dir: {e}")` |
| `crates/argo-cli/src/handlers/worker_resume.rs:19` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:140` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs:177` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-cli/src/handlers/setup.rs:20` | `ArgoError::Provider(format!("cwd: {e}")` |
| `crates/argo-eval/src/task.rs:87` | `ArgoError::Provider(format!("read {}: {e}", path.display()` |

### Recommend `ArgoError::Schema` — caller-supplied bad input (invalid / missing / bad / unknown / unparseable)

_Rationale: Phrases like 'invalid', 'missing', 'bad', 'unknown', or 'unparseable' indicate caller-input validation. Map to HTTP 400 via `ArgoError::Schema`, not 502 via `ArgoError::Provider`._

Site count: **76**

| Site (path:line) | Excerpt |
|---|---|
| `crates/argo-provider/src/discovery.rs:40` | `ArgoError::Provider(format!("invalid exclude_pattern {s:?}: {e}")` |
| `crates/argo-replay/src/source.rs:78` | `ArgoError::Provider(format!("unknown session {}", session.0)` |
| `crates/argo-agent-runner/src/helpers.rs:197` | `ArgoError::Provider(format!("unknown role '{other}'")` |
| `crates/argo-config/src/cred.rs:151` | `ArgoError::Provider(format!("invalid credentials path {}", path.display()` |
| `crates/argo-cli/src/handlers/worker_onboard.rs:261` | `ArgoError::Provider(format!("missing required {label}")` |
| `crates/argo-cli/src/handlers/worker_charter.rs:158` | `ArgoError::Provider(format!("invalid tenant id '{tenant}': {e}")` |
| `crates/argo-cli/src/handlers/worker_charter.rs:163` | `ArgoError::Provider(format!("invalid worker id '{worker_id}': {e}")` |
| `crates/argo-cli/src/handlers/session.rs:270` | `ArgoError::Provider(format!("invalid worker id: {e}")` |
| `crates/argo-cli/src/handlers/session.rs:276` | `ArgoError::Provider(format!("invalid session id '{}': {e}", session.id)` |
| `crates/argo-cli/src/handlers/session.rs:339` | `ArgoError::Provider(format!("invalid worker id '{w}': {e}")` |
| `crates/argo-cli/src/handlers/session.rs:385` | `ArgoError::Provider(format!("invalid tenant id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/session.rs:394` | `ArgoError::Provider(format!("invalid handoff status '{other}'")` |
| `crates/argo-cli/src/handlers/session.rs:483` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/worker_channels.rs:210` | `ArgoError::Provider(format!("invalid tenant id '{tenant}': {e}")` |
| `crates/argo-cli/src/handlers/worker_channels.rs:215` | `ArgoError::Provider(format!("invalid worker id '{worker_id}': {e}")` |
| `crates/argo-cli/src/handlers/action_history.rs:113` | `ArgoError::Provider(format!("invalid duration '{raw}'")` |
| `crates/argo-cli/src/handlers/action_history.rs:118` | `ArgoError::Provider(format!("invalid duration '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/session_capability_gaps.rs:32` | `ArgoError::Provider(format!("invalid session id '{session_id_str}': {e}")` |
| `crates/argo-cli/src/handlers/session_export.rs:29` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/session_turns.rs:26` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/compliance_soc2.rs:339` | `ArgoError::Provider(format!("invalid {flag} '{raw}': {e} (expected RFC3339)` |
| `crates/argo-cli/src/handlers/route.rs:65` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/forge_admin.rs:213` | `ArgoError::Provider(format!("invalid forge run id '{raw}'")` |
| `crates/argo-cli/src/handlers/session_messages_rollups.rs:15` | `ArgoError::Provider(format!("invalid session id '{session}': {e}")` |
| `crates/argo-cli/src/handlers/audit.rs:29` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/audit.rs:135` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/audit.rs:396` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/recover_cache.rs:224` | `ArgoError::Provider(format!("bad duration '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/skill_runs.rs:84` | `ArgoError::Provider(format!("invalid duration '{raw}'")` |
| `crates/argo-cli/src/handlers/skill_runs.rs:89` | `ArgoError::Provider(format!("invalid duration '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/memory.rs:520` | `ArgoError::Provider(format!("invalid uuid '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/skill_review.rs:364` | `ArgoError::Provider(format!("invalid skill id '{skill_id_str}': {e}")` |
| `crates/argo-cli/src/handlers/session_diff.rs:22` | `ArgoError::Provider(format!("invalid session id '{raw1}': {e}")` |
| `crates/argo-cli/src/handlers/session_diff.rs:24` | `ArgoError::Provider(format!("invalid session id '{raw2}': {e}")` |
| `crates/argo-cli/src/handlers/worker_costs.rs:210` | `ArgoError::Provider(format!("invalid --tenant '{tenant}': {e}")` |
| `crates/argo-cli/src/handlers/skill_export.rs:165` | `ArgoError::Provider(format!("invalid skill id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:66` | `ArgoError::Provider(format!("unknown fix category {other}")` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:76` | `ArgoError::Provider(format!("unknown safe fix {id}")` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:84` | `ArgoError::Provider(format!("unknown confirmation fix {id}")` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:196` | `ArgoError::Provider(format!("unknown doctor fix issue '{raw}'")` |
| `crates/argo-cli/src/handlers/doctor_fix.rs:200` | `ArgoError::Provider(format!("unknown doctor fix issue '{id}'")` |
| `crates/argo-cli/src/handlers/session_approval.rs:33` | `ArgoError::Provider(format!("invalid session id '{s}': {e}")` |
| `crates/argo-cli/src/handlers/session_messages.rs:31` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/audit_count.rs:28` | `ArgoError::Provider(format!("invalid session id '{s}': {e}")` |
| `crates/argo-cli/src/handlers/audit_report.rs:354` | `ArgoError::Provider(format!("invalid {flag} date '{raw}'")` |
| `crates/argo-cli/src/handlers/skill_review_candidates.rs:119` | `ArgoError::Provider(format!("invalid memory id '{memory_id_str}': {e}")` |
| `crates/argo-cli/src/handlers/session_bisect.rs:29` | `ArgoError::Provider(format!("invalid session id '{raw_id}': {e}")` |
| `crates/argo-cli/src/handlers/org.rs:209` | `ArgoError::Provider(format!("invalid tenant id '{s}': {e}")` |
| `crates/argo-cli/src/handlers/trajectory_export.rs:373` | `ArgoError::Provider(format!("invalid {flag} '{raw}'")` |
| `crates/argo-cli/src/handlers/trace.rs:157` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/mcp_tools.rs:156` | `ArgoError::Provider(format!("invalid --filter regex '{p}': {e}")` |
| `crates/argo-cli/src/handlers/session_clone_branch.rs:141` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/persona_edit.rs:349` | `ArgoError::Provider(format!("invalid tenant id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/recover.rs:31` | `ArgoError::Provider(format!("invalid session id '{session_id}': {e}")` |
| `crates/argo-cli/src/handlers/recover.rs:52` | `ArgoError::Provider(format!("invalid llm response: {e}")` |
| `crates/argo-cli/src/handlers/audit_query.rs:91` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/approval_export.rs:244` | `ArgoError::Provider(format!("invalid timestamp '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/approval_export.rs:257` | `ArgoError::Provider(format!("invalid duration '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/activity_since.rs:256` | `ArgoError::Provider(format!("invalid tenant id '{trimmed}': {e}")` |
| `crates/argo-cli/src/handlers/worker_archive.rs:204` | `ArgoError::Provider(format!("invalid tenant id '{tenant}': {e}")` |
| `crates/argo-cli/src/handlers/worker_archive.rs:209` | `ArgoError::Provider(format!("invalid worker id '{worker_id}': {e}")` |
| `crates/argo-cli/src/handlers/doctor_watch.rs:74` | `ArgoError::Provider(format!("invalid --interval '{raw}': duration too large")` |
| `crates/argo-cli/src/handlers/judge.rs:31` | `ArgoError::Provider(format!("invalid judge record id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/judge.rs:75` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/session_replay.rs:48` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/session_audit_rollups.rs:17` | `ArgoError::Provider(format!("invalid session id '{session}': {e}")` |
| `crates/argo-cli/src/handlers/session_audit_rollups.rs:88` | `ArgoError::Provider(format!("invalid session id '{session}': {e}")` |
| `crates/argo-cli/src/handlers/session_audit_rollups.rs:159` | `ArgoError::Provider(format!("invalid session id '{session}': {e}")` |
| `crates/argo-cli/src/handlers/session_search.rs:40` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/session_search.rs:74` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/audit_export.rs:96` | `ArgoError::Provider(format!("invalid session id '{raw}': {e}")` |
| `crates/argo-cli/src/handlers/audit_export.rs:314` | `ArgoError::Provider(format!("invalid {flag} date '{raw}'")` |
| `crates/argo-cli/src/handlers/gateway/boot.rs:322` | `ArgoError::Provider(format!("invalid PID in {}: {e}", pidfile.display()` |
| `crates/argo-cli/src/handlers/worker_resume.rs:16` | `ArgoError::Provider(format!("invalid tenant id '{tenant}': {e}")` |
| `crates/argo-cli/src/handlers/worker_resume.rs:18` | `ArgoError::Provider(format!("invalid worker id '{worker_id}': {e}")` |
| `crates/argo-cli/src/handlers/duty_pause.rs:12` | `ArgoError::Provider(format!("invalid --until '{raw}': {e}")` |

### Recommend `ArgoError::Schema` — explicit 'missing field' phrasing

_Rationale: The brief calls this pattern out specifically: `missing field` is by definition a schema-validation failure on caller input._

Site count: **0**

_(no callsites currently match this pattern; kept as a forward-looking guard)_

### Recommend `ArgoError::Invariant` — internal data corruption (uuid / decode / timestamp / 'script exhausted' / unreachable)

_Rationale: These flag persisted data that should be parseable but isn't (corrupt uuid blob, out-of-range timestamp), or test/synthetic 'unreachable' branches. Use `ArgoError::Invariant` so the gateway maps to HTTP 500 and operators triage as a bug._

Site count: **15**

| Site (path:line) | Excerpt |
|---|---|
| `crates/argo-skill/src/quality.rs:244` | `ArgoError::Provider(format!("decode skill id '{skill_id_str}': {e}")` |
| `crates/argo-skill/src/quality.rs:246` | `ArgoError::Provider(format!("decode quality inputs: {e}")` |
| `crates/argo-skill/src/quality.rs:248` | `ArgoError::Provider(format!("decode computed_at: {computed_at_micros}")` |
| `crates/argo-skill/src/quality.rs:250` | `ArgoError::Provider(format!("decode score '{score}': {e}")` |
| `crates/argo-orchestrator/src/routing.rs:420` | `ArgoError::Provider(format!("decode rule id '{id_str}': {e}")` |
| `crates/argo-orchestrator/src/routing.rs:422` | `ArgoError::Provider(format!("decode rule condition: {e}")` |
| `crates/argo-orchestrator/src/routing.rs:424` | `ArgoError::Provider(format!("decode rule target: {e}")` |
| `crates/argo-identity/src/roster.rs:399` | `ArgoError::Provider(format!("decode worker id '{raw}': {e}")` |
| `crates/argo-identity/src/roster.rs:443` | `ArgoError::Provider(format!("decode roster id '{id_str}': {e}")` |
| `crates/argo-identity/src/roster.rs:447` | `ArgoError::Provider(format!("decode tenant id '{tenant_str}': {e}")` |
| `crates/argo-identity/src/roster.rs:449` | `ArgoError::Provider(format!("decode roster policy: {e}")` |
| `crates/argo-identity/src/roster.rs:451` | `ArgoError::Provider(format!("decode roster channels: {e}")` |
| `crates/argo-cli/src/handlers/recover_cache.rs:90` | `ArgoError::Provider(format!("decode cached suggestions: {e}")` |
| `crates/argo-cli/src/handlers/recover.rs:119` | `ArgoError::Provider(format!("decode cached suggestions: {e}")` |
| `crates/argo-cli/src/handlers/gateway/routes_orchestrator.rs:310` | `ArgoError::Provider(format!("uuid blob: {e}")` |

