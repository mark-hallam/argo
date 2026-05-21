# mission-knowledge-startup-seed — seed 5 starter wiki docs on `/onboarding/provision`

**Owner**: codex or claude (mostly mechanical — codex preferred).
**Depends on**: `mission-workplace-knowledge-substrate` MERGED (uses `doc::create`) + `mission-workplace-knowledge-audit-kinds` MERGED (emit `DocCreated` on each seed).
**Authored against**: origin/main `c613d922` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-knowledge-startup-seed`.
**Priority**: P2 — Wave A brief 6 of 6. Polish but real: a new workspace without seed docs has an empty wiki, which is technically correct but operationally useless.
**Sizing**: ~300 LOC across handler extension + 5 markdown files + idempotency tests.

## Why this brief exists

Read design doc §5 brief 6 (the spec) and §9 step 1 (demo scenario — "5 seeded wiki docs appear" is the first thing the operator sees after provision).

Without this brief, a fresh tenant after `/onboarding/provision` has an empty wiki. The operator has to know what categories exist, what slugs to use, what reasonable starting content looks like. With this brief, they see 5 docs prefilled with sensible defaults, edit each to taste, and are ready in 5 minutes.

The 5 seeds are:
1. `brand-voice` — voice, tone, words to avoid.
2. `business-rules` — non-negotiable operational rules.
3. `escalation-policy` — when workers should escalate to humans.
4. `customer-tone` — how to talk to customers across channels.
5. `never-do-this` — explicit list of things workers must not do.

Each gets content an operator would actually use as a starting point, not Lorem Ipsum.

## Approach (concrete)

### Phase 1 — Seed content (5 markdown files)

1. **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/brand_voice.md` (~60 lines):
   ```markdown
   # Brand voice

   This is the voice every worker in this workspace must speak in.

   ## Tone
   - Confident, warm, never salesy.
   - We speak like a knowledgeable colleague, not a marketer.
   - Sentences are short. We avoid jargon unless the customer used it first.

   ## Words we never use
   - "cheap" → say "cost-effective" or "good value" instead.
   - "guaranteed" → say "we expect" or "in practice we see".
   - "instant" → say "fast" or "in under a minute".
   - "amazing" / "incredible" → describe the actual outcome instead.

   ## Phrases we always use
   - Sign off with "Speak soon — the team".
   - Open emails with a short context-aware acknowledgment, not "Hi!".

   ## Emojis
   - Avoid emojis except 👍 for confirmations and 🛠️ for in-progress work.

   ---
   **Operator: edit this doc to match YOUR brand. Every worker reads it every turn.**
   ```

2. **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/business_rules.md` (~50 lines):
   ```markdown
   # Business rules

   Non-negotiable operating rules. Workers MUST follow these.

   ## Approvals
   - Any commitment over $5,000 needs human approval before reply.
   - Any contract amendment needs human approval, no exceptions.
   - Any refund over $500 needs human approval.

   ## Working hours
   - Workers respond Mon-Fri 09:00-18:00 in customer's local timezone.
   - Outside hours, queue the response — do NOT send.

   ## Channels
   - Sales replies go via email. Support replies via support channel.
   - Internal chat (Slack/Teams) is for coordination only — never share customer data there.

   ## Forbidden actions
   - Never quote competitor pricing.
   - Never make legal promises (warranty, SLA, indemnity).
   - Never share another customer's information, even anonymized.

   ---
   **Operator: edit these rules to match YOUR operating boundaries.**
   ```

3. **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/escalation_policy.md` (~40 lines):
   ```markdown
   # Escalation policy

   When a worker should hand off to a human.

   ## Escalate IMMEDIATELY (do not reply, page on-call)
   - Legal threats, regulator inquiries, press requests.
   - Security incidents (breach, unauthorized access, data leak claim).
   - Customer says "cancel everything" or "I want a refund of the whole contract".
   - Anyone hostile, abusive, or threatening.

   ## Escalate SOON (reply with acknowledgment + flag for human)
   - Pricing discussions over $5K commitment.
   - Contract amendments.
   - Multi-channel customer (same person on multiple channels with conflicting requests).

   ## Handle autonomously
   - Standard product questions.
   - Onboarding setup help.
   - First-tier troubleshooting.

   ---
   **Operator: tune these thresholds to YOUR risk tolerance.**
   ```

4. **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/customer_tone.md` (~40 lines):
   ```markdown
   # Customer tone

   How we talk TO customers. Layer on top of brand-voice.md.

   ## First contact
   - Open with their NAME if known. Otherwise "Hi there".
   - Mirror their formality level.
   - Acknowledge what they said before responding.

   ## Difficult conversations
   - Lead with empathy. "I hear you — that's frustrating" beats "I'll look into it."
   - Own the problem ("we'll fix this"), not "the system did X".
   - Offer a concrete next step. No open-ended "we'll get back to you".

   ## Closing
   - Confirm what's next: action, owner, timeline.
   - Sign off per brand-voice.md.

   ---
   **Operator: customize for YOUR customer relationship style.**
   ```

5. **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/never_do_this.md` (~30 lines):
   ```markdown
   # Never do this

   Hard prohibitions. Every worker reads these every turn.

   - Never send a customer's data to another customer.
   - Never quote pricing without checking the current price sheet.
   - Never agree to a contract clause not in our standard template without explicit human approval.
   - Never recommend a competitor's product.
   - Never make a promise about timing, performance, or features without checking the canonical roadmap doc.
   - Never apologize for something we didn't do wrong; never deflect when we did.
   - Never reply outside business hours (queue the message instead).
   - Never share internal credentials, API keys, or system details with customers.

   ---
   **Operator: add YOUR specific never-dos. The vaguer this list is, the more workers will guess.**
   ```

### Phase 2 — Seed loader

6. **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/mod.rs` (~80 LOC):
   ```rust
   //! Workplace-knowledge seed docs shipped on `/onboarding/provision`.
   //!
   //! Content is embedded via `include_str!` so it ships in the binary;
   //! operators edit the docs via the gateway after provision.

   use argo_store::SessionStore;
   use argo_types::{ArgoResult, TenantId};
   use argo_workplace_knowledge::doc::{self, Category};

   pub struct SeedDoc {
       pub slug: &'static str,
       pub title: &'static str,
       pub category: Category,
       pub body_markdown: &'static str,
   }

   pub fn all_seeds() -> Vec<SeedDoc> {
       vec![
           SeedDoc {
               slug: "brand-voice",
               title: "Brand voice",
               category: Category::Brand,
               body_markdown: include_str!("brand_voice.md"),
           },
           SeedDoc {
               slug: "business-rules",
               title: "Business rules",
               category: Category::BusinessRules,
               body_markdown: include_str!("business_rules.md"),
           },
           SeedDoc {
               slug: "escalation-policy",
               title: "Escalation policy",
               category: Category::Compliance,
               body_markdown: include_str!("escalation_policy.md"),
           },
           SeedDoc {
               slug: "customer-tone",
               title: "Customer tone",
               category: Category::Brand,
               body_markdown: include_str!("customer_tone.md"),
           },
           SeedDoc {
               slug: "never-do-this",
               title: "Never do this",
               category: Category::BusinessRules,
               body_markdown: include_str!("never_do_this.md"),
           },
       ]
   }

   /// Idempotent seed: walks the 5 starter docs and inserts each only if
   /// no doc with that slug exists for the tenant.
   ///
   /// Returns the number of docs that were newly inserted (0..=5).
   /// Re-provisioning the same workspace is a no-op (returns 0).
   pub async fn seed_workplace_knowledge(
       store: &SessionStore,
       tenant_id: &TenantId,
       created_by: &str,  // typically "system:onboarding-seed"
   ) -> ArgoResult<u32> {
       let mut inserted = 0u32;
       for seed in all_seeds() {
           if doc::get(store.pool(), tenant_id, seed.slug).await?.is_none() {
               doc::create(
                   store.pool(),
                   tenant_id,
                   seed.slug,
                   seed.title,
                   seed.category,
                   seed.body_markdown,
                   created_by,
               ).await?;
               inserted += 1;
           }
       }
       Ok(inserted)
   }
   ```

### Phase 3 — Wire-in on provision

7. **MODIFY** `crates/argo-cli/src/handlers/gateway/onboarding_provision.rs` — at the end of the successful-provision path (after tenant + workspace creation, just before returning success), call:
   ```rust
   let seeded = onboarding_seeds::seed_workplace_knowledge(
       store,
       &tenant_id,
       "system:onboarding-seed",
   ).await?;
   tracing::info!(target: "argo::onboarding::seed", tenant = %tenant_id, count = seeded, "workplace knowledge seeded");
   ```
   Then for each newly-inserted doc, emit `AuditKind::WorkplaceKnowledgeDocCreated` (the wrapper from brief 2, or directly via `AuditSink`).

8. **MODIFY** `crates/argo-cli/src/handlers/gateway/mod.rs` — register the new `onboarding_seeds` module:
   ```rust
   mod onboarding_seeds;
   ```

### Phase 4 — Tests

9. **MODIFY** `crates/argo-cli/src/handlers/gateway/onboarding_provision_tests.rs` — extend with:
   - `provision_seeds_5_workplace_docs` — fresh tenant, call provision, query the wiki table, assert 5 rows.
   - `provision_is_idempotent_when_docs_already_exist` — call provision twice; second call returns success without re-creating docs; final doc count is still 5.
   - `seed_emits_5_audit_rows_with_doc_created_kind` — assert exactly 5 `WorkplaceKnowledgeDocCreated` rows in `audit_record`.
   - `seed_body_matches_disk_file_byte_for_byte` — read `brand_voice.md` from disk, compare to the inserted doc's `body_markdown` field. (Guards against accidental edits during the seed-encoding path.)

## Files you may touch (verified to exist on `c613d922`)

- **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/mod.rs`.
- **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/brand_voice.md`.
- **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/business_rules.md`.
- **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/escalation_policy.md`.
- **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/customer_tone.md`.
- **NEW** `crates/argo-cli/src/handlers/gateway/onboarding_seeds/never_do_this.md`.
- **MODIFY** `crates/argo-cli/src/handlers/gateway/onboarding_provision.rs` — add the seed call + audit emission. Keep file < 500 LOC.
- **MODIFY** `crates/argo-cli/src/handlers/gateway/mod.rs` — register the new module.
- **MODIFY** `crates/argo-cli/src/handlers/gateway/onboarding_provision_tests.rs` — 4 new tests.
- **MODIFY** `crates/argo-cli/Cargo.toml` — add `argo-workplace-knowledge = { path = "../argo-workplace-knowledge" }` if brief 3 hasn't already added it.

## Files you must NOT touch

- `crates/argo-workplace-knowledge/*` — substrate is brief 1's.
- `crates/argo-audit/src/event.rs` — variants are brief 2's.
- Anything under `web/` — this brief is gateway-only.
- The 5 `.md` files are static content embedded at compile time. If you find yourself dynamically templating them, STOP — that's scope creep.

## Pre-flight verification

```bash
# Confirm the 3 dependencies are on main.
ls crates/argo-workplace-knowledge/src/lib.rs
grep -n 'WorkplaceKnowledgeDocCreated' crates/argo-audit/src/event.rs
ls crates/argo-cli/src/handlers/gateway/onboarding_provision.rs

# Confirm the existing provision-success path.
rg 'fn handle_provision|provision_workspace|tenant created' crates/argo-cli/src/handlers/gateway/onboarding_provision.rs | head -5

# Confirm include_str! pattern is used elsewhere (sibling reference).
rg 'include_str!\("' crates/argo-cli/src/handlers/ | head -3
```

## Acceptance criteria

- [ ] All 5 .md files exist on disk under `crates/argo-cli/src/handlers/gateway/onboarding_seeds/`.
- [ ] All 5 docs are inserted on a fresh `/onboarding/provision` call.
- [ ] Re-provisioning the same workspace is a no-op (idempotent — second call inserts 0 new docs).
- [ ] Each insertion emits a `WorkplaceKnowledgeDocCreated` audit row.
- [ ] The 4 new tests in `onboarding_provision_tests.rs` pass.
- [ ] `cargo test -p argo-cli onboarding_provision_tests` green.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] Every modified file < 500 LOC.
- [ ] The .md files compile via `include_str!` — confirm by running the seed test (it will fail at compile time if any path is wrong).
- [ ] No `unwrap` / `expect` outside `#[cfg(test)]`.
- [ ] BUILD_STATUS.md row flipped to IN_REVIEW with PR number on push.

## Branch + PR

- Branch: `codex-mission-knowledge-startup-seed`
- PR title: `argo-cli(onboarding): seed 5 workplace-knowledge docs on provision (mission-knowledge-startup-seed)`

## Gotchas

- **Content quality**: the 5 .md files MUST be sensible operator-facing starting content, not Lorem Ipsum (operator-locked per design doc §9 demo scenario). The samples above are starting points — if the worker thinks any of them are weak, they can adjust (this is the one place this brief has design latitude). Match the operator's voice from the demo scenario.
- **Idempotency via `doc::get`-then-`create` check**: this is per-slug, not per-tenant. So if the operator deletes `brand-voice` and re-runs provision, that one doc gets re-inserted. That's the correct behavior — design doc §9 demo implies the operator EDITS the seeds, never expects them re-inserted on re-provision.
- **Audit rows on seed**: 5 audit rows per provision. With `created_by="system:onboarding-seed"`, operators can filter `/audit` by actor to see the initial seed event. That's a real audit trail, not a special-case bypass.
- **No special `provision_seed_id` table**: this brief uses the existing `workplace_knowledge_doc` table. There's no separate provenance — the `created_by` field is sufficient.
- **`include_str!` paths are compile-time**: the .md files MUST be in the exact subdir `onboarding_seeds/` next to `mod.rs`. Renaming the subdir or moving a file will break the build immediately.

## References

- Design doc: `docs/skycot/design/workplace-onboarding-architecture.md` §5 brief 6, §9 demo scenario step 1.
- Sibling briefs in Wave A:
  - **brief 1** (`mission-workplace-knowledge-substrate`) — provides `argo_workplace_knowledge::doc::{create, get}`.
  - **brief 2** (`mission-workplace-knowledge-audit-kinds`) — provides `AuditKind::WorkplaceKnowledgeDocCreated`.
  - **brief 5** (`mission-knowledge-context-injection`) — reads the seeded docs and injects them into every worker turn from boot.
- Reference patterns:
  - `crates/argo-cli/src/handlers/gateway/onboarding_provision.rs` — the existing provision handler this brief extends.
  - `include_str!` usage elsewhere in the codebase (grep above).
