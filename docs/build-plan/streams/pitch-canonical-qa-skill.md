# pitch-canonical-qa-skill

**Owner**: claude.
**Authored against**: origin/main `73ac6e06c` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh pitch-canonical-qa-skill`.
**Blocked by**: `pitch-truth-boundary-capability-schema` (P-01) MUST be MERGED — this skill produces `declared_classes` consumed by the truth-boundary evaluator.

🔴 **P0 v1.0 ship-blocker · Track P (Pitch Agent) · brief P-04 of 8**

A new skill in the skill library — `canonical-qa` — that matches incoming questions against a pre-warmed knowledge base of canonical Q&A pairs and returns a fixed answer (with `declared_classes` so the truth-boundary gate from P-02 has structured input). This is the hot path: sub-300ms voice response, no LLM round-trip, deterministic answers for the top ~100 anticipated investor questions.

Cold-path (generative + capability gate) responses still flow through the normal agent runner. This skill is the optimization that makes the Pitch Agent feel snappy, not the foundation of correctness.

## Why this brief exists

The v1.0 demo storyboard requires sub-300ms responses to objection-drill questions ("How are you different from Lindy?"). Round-tripping through an LLM at the demo's quality bar is ~1-3s — too slow for the conversational feel. The hot-path canonical-qa skill closes that latency gap.

Operator memory `project_pitch_agent_v1_demo.md` locks the design: "Hot path: pre-warmed canonical Q&A pairs (top ~100 anticipated investor questions), vector-similarity match, ~100-200ms voice response, no LLM round-trip."

A canonical hit also short-circuits truth-boundary classification — the canonical entry already declares its claim classes, so the gate runs in O(1) instead of running a classifier.

## Source of truth

- **Storyboard**: [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html) — Scene 2b (Lindy comparison, sub-300ms response, "Hot path · canonical answer" pill)
- **Memory**: `project_pitch_agent_v1_demo.md` — locks the hot/cold architecture
- **P-01 brief**: `pitch-truth-boundary-capability-schema.md` — defines `ClaimCandidate { declared_classes }` which this skill populates
- **Skill SDK**: `crates/argo-skill-sdk/` (existing) — the contract this skill follows
- **Existing skill library wire-up**: `crates/argo-agent-runner/src/turn_helpers.rs:65` — `LayeredLookup::with_library` per memory `project_v1_strategic_decisions_2026_05_14`

## Scope

### 1. The skill itself

The skill lives in the skill library (not in a Rust crate body). Skill artifact format follows existing conventions — see `crates/argo-skill/` for the loader.

Skill manifest:

```toml
# skill-library/canonical-qa/manifest.toml
id = "canonical-qa"
version = "0.1.0"
description = "Hot-path matcher: maps incoming questions to pre-warmed canonical answers with declared claim classes."
shared = true
input_schema = "schemas/input.schema.json"
output_schema = "schemas/output.schema.json"
```

### 2. The Q&A index

Each tenant has its own canonical Q&A corpus. Stored as TOML, one file per topic for editability:

```
skill-library/canonical-qa/corpora/
└── skycot/
    ├── competitive-positioning.toml
    ├── pricing-and-business-model.toml
    ├── architecture-and-platform.toml
    ├── team-and-traction.toml
    └── product-roadmap.toml
```

Schema per file:

```toml
[[entries]]
id = "lindy-comparison"
prompts = [
  "How is Skycot different from Lindy?",
  "How do you compare to Lindy?",
  "What's the difference between you and Lindy?",
  "Is this just Lindy?",
]
answer = """Lindy is a workflow tool — if-this-then-that with AI bolted on. \
Skycot is the workforce platform — where your AI teammates live, learn, \
and report up to you. Different category, different buyer, different ceiling."""
declared_claim_classes = ["competitive-positioning"]
audience_tags = ["investor", "buyer", "press"]
last_reviewed = "2026-05-24"
review_owner = "founder@example.com"

[[entries]]
id = "valuation-deferral"
prompts = ["What's your valuation?", "What are you valued at?", "What's the round at?"]
answer = "I'm not authorized to share valuation publicly. Mark, do you want to take it?"
declared_claim_classes = ["valuation"]  # P-01 truth-boundary will see this and Forbid
audience_tags = ["investor", "press"]
```

A canonical entry that names a forbidden claim class produces the refusal naturally — the truth-boundary gate doesn't need to *replace* the answer because the canonical entry IS the refusal. Defense in depth: if the entry is stale or someone edits the answer past policy, the gate (P-02) still catches it.

### 3. The matcher

Sentence-embedding similarity (cosine), with a cutoff threshold. Stays in-process to preserve latency budget — no external embedding API call per turn.

```rust
pub struct CanonicalQaIndex {
    tenant_id: TenantId,
    entries: Vec<CanonicalEntry>,
    embeddings: Vec<Embedding>,  // pre-computed at load time
    embedding_dim: usize,
    similarity_threshold: f32,    // default 0.78
}

pub struct CanonicalEntry {
    pub id: String,
    pub prompts: Vec<String>,
    pub answer: String,
    pub declared_claim_classes: Vec<String>,
    pub audience_tags: Vec<String>,
    pub last_reviewed: DateTime<Utc>,
}

impl CanonicalQaIndex {
    pub fn load(corpus_dir: &Path, tenant_id: TenantId) -> Result<Self>;
    pub fn lookup(&self, query: &str, audience: &str) -> Option<CanonicalMatch>;
}

pub struct CanonicalMatch {
    pub entry_id: String,
    pub answer: String,
    pub declared_claim_classes: Vec<String>,
    pub similarity: f32,
}
```

**Embedding model:** small local model (e.g. `all-MiniLM-L6-v2` via `ort` or `candle` — pick one in implementation; flag the choice in the PR). 384-dim is plenty. Loading the model adds ~50MB to the runtime binary footprint — acceptable for the v1 demo path; can be made optional via a Cargo feature in a follow-up.

If a local embedder proves too heavy, fall back to lexical (BM25 over the `prompts` list) — that ships ~80% of the quality at zero binary weight. Acceptable for the v1 demo; flag the choice in the PR.

### 4. Skill execution

The skill SDK contract takes a JSON input, returns a JSON output. Input = `{ query, audience_tag, tenant_id }`. Output = `{ matched: bool, answer, declared_claim_classes, entry_id, similarity }` — or `{ matched: false }` if no entry exceeds the threshold (the agent falls back to the cold path).

`audience_tag` lets a single corpus serve multiple personas — investor questions vs. customer questions vs. press questions can pull different answers.

### 5. Corpus authoring tool

Add `argo-cli canonical-qa lint <corpus-dir>` that:

- Validates every TOML file parses
- Validates every entry has ≥3 prompts (defense against thin matches)
- Validates every `declared_claim_class` is a known class in the tenant's truth-boundary policy (cross-checks P-01)
- Validates `last_reviewed` is within 90 days (corpus rot is real)
- Validates no two entries have overlapping prompt sets (warns)

The lint is part of the brief because: a corpus that drifts is worse than no corpus.

### 6. Tests

`crates/argo-skill/src/canonical_qa_tests.rs`:

- Lookup matches the right entry for a paraphrase
- Lookup returns `None` below the similarity threshold
- Audience tag filters — investor question doesn't match customer-only entry
- Cross-tenant: tenant A's corpus does not match tenant B's queries
- Corpus loader rejects malformed entries
- Corpus loader rejects entries with `declared_claim_classes` not present in the tenant's truth-boundary policy
- Embedding fallback (lexical) returns sensible matches if the local model isn't available

Aim for ≥10 tests.

## Files you may touch

- `crates/argo-skill/src/canonical_qa.rs` (new — index, lookup, loader)
- `crates/argo-skill/src/canonical_qa_tests.rs` (new)
- `crates/argo-skill/src/lib.rs` (re-export)
- `crates/argo-skill/Cargo.toml` (add embedding deps behind a feature flag)
- `skill-library/canonical-qa/manifest.toml` (new)
- `skill-library/canonical-qa/corpora/skycot/*.toml` (new — start with ~30 entries spanning the 5 corpus files; remaining entries land in follow-up)
- `crates/argo-cli/src/handlers/canonical_qa_lint.rs` (new — wire to `argo canonical-qa lint`)

## Files you must NOT touch

- `crates/argo-capability/` — P-01 owns it; this skill *uses* the claim-class names defined in policies but does not modify them
- The skill SDK contract itself — author the skill to the existing contract; don't extend the contract
- The agent runner — this skill is invoked by the existing skill-library lookup wired at `turn_helpers.rs:65`

## Acceptance criteria

- [ ] `argo-skill` builds with the new module + an optional feature flag for the embedding model
- [ ] ≥10 tests pass including cross-tenant negative + audience-filter + corpus-loader rejection cases
- [ ] At least 30 canonical entries authored across 5 corpus files for the `skycot` tenant
- [ ] `argo canonical-qa lint skill-library/canonical-qa/corpora/skycot/` reports zero errors
- [ ] Skill manifest at `skill-library/canonical-qa/manifest.toml` is loadable by the existing skill-library loader
- [ ] Lookup latency p95 ≤ 50ms on a 100-entry corpus (CI benchmark via `criterion`)
- [ ] No file >500 LOC; no fn >80 LOC; no `unwrap`/`expect` outside `#[cfg(test)]`
- [ ] Rustdoc on every public type/fn with at least one example

## Verification

```bash
cargo build -p argo-skill --features canonical-qa-embed
cargo test -p argo-skill canonical_qa
cargo run -p argo-cli -- canonical-qa lint skill-library/canonical-qa/corpora/skycot/
cargo bench -p argo-skill canonical_qa_lookup
bash scripts/check-brief-freshness.sh pitch-canonical-qa-skill
```

Smoke (manual): seed the skill in a session and query "How do you compare to Lindy?" — confirm <100ms response with the canonical-positioning answer.

## Cross-references

- **Blocked by**: `pitch-truth-boundary-capability-schema` (P-01) — claim-class names referenced in corpora
- **Consumed by**: `pitch-agent-worker-template` (P-05) — manifest lists this skill in the bundle; `pitch-truth-boundary-runtime-enforcement` (P-02) — provenance shows `ProvenanceSource::CanonicalQa` on hits
- **Tested by**: `pitch-eval-suite-investor-qa` (P-08) — the 100+ Q&As include both canonical-hit and canonical-miss cases
- **Memory**: `project_pitch_agent_v1_demo.md` — locks the hot/cold architecture
- **Pre-flight grep** 2026-05-24: no existing `CanonicalQaIndex`/`CanonicalEntry`/`CanonicalMatch` surfaces
