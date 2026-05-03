# GTM Engineer learning-loop demo

This walkthrough drives the end-to-end loop the GTM Engineer
vertical exists to enable:

```
role  →  work  →  gap/pattern  →  learned-skill candidate
      →  review/promote  →  active skill  →  improved second run
```

Every step is exercised by an existing argo CLI surface against
the shipped GTM Engineer vertical. Run it with:

```bash
cargo build --bin argo
ARGO_BIN=$(pwd)/target/debug/argo bash demo/gtm_engineer_learning_loop.sh
```

The script tears down its own ARGO_HOME on exit, so it leaves
no state behind.

## What the demo proves

1. The GTM Engineer vertical bundles 8 skills out of the box
   and a session bound to it inherits all of them automatically.
2. Operator-recorded campaign outcomes accumulate as
   `campaign_results` memory entries inside the vertical's
   namespace (Rule 7 isolation).
3. Three reinforcing observations across distinct campaigns
   are sufficient — and required — to surface a pattern. One
   observation never qualifies (`gtm_no_skill_from_weak_evidence`
   eval fixture locks this contract).
4. `propose_gtm_skill` writes a `learned_skill_candidate`
   entry. Candidates are immutable and never auto-activate.
5. `argo skill review list` aggregates pending skills AND
   memory-backed candidates so the operator sees both sources
   in one queue.
6. `argo skill review promote <memory_id>` materialises a
   candidate as a `pending_review` skill row. The skill is
   gated until explicitly approved.
7. `argo skill review approve <skill_id>` flips lifecycle to
   `active`. The skill body is then visible to future agent
   runs via `argo skill list`.
8. The scored eval suite stays green throughout — promoting a
   new skill never regresses the existing 10-fixture contract.

## What the demo does NOT prove

The "agent does work" steps (`analyze_campaign`,
`detect_gtm_pattern`, `propose_gtm_skill`) are simulated by
direct memory writes that use the exact JSON shapes those
skills emit. The mock provider does not invoke an LLM, so the
demo cannot show the agent autonomously deriving the pattern
from raw campaign data — only the lifecycle from "candidate
exists in memory" through "active skill in the catalog."

To exercise the LLM-driven path end-to-end, set
`ANTHROPIC_API_KEY` and re-run with
`--provider anthropic --model claude-sonnet-4-6` in step 1.
The operator-side surface (review/promote/approve) is
provider-agnostic and stays identical.

## Step-by-step

### STEP 1 — role binding

```bash
argo new \
  --provider mock --model mock \
  --vertical gtm-engineer \
  --title "demo: GTM learning loop"
```

Output (excerpt):

```
created session 019def69-81a1-7713-8c24-9963cdbd1032
  provider:  mock
  model:     mock
  vertical:  gtm-engineer
  skills:    8 attached from vertical bundle
```

The session is bound to `gtm-engineer`. All 8 bundled skills
(`define_icp`, `find_target_accounts`, `research_company`,
`research_person`, `generate_outreach`, `analyze_campaign`,
`detect_gtm_pattern`, `propose_gtm_skill`) attach automatically
via the vertical manifest.

### STEP 2 — work: 3 campaigns

```bash
for i in 1 2 3; do
  argo memory append memory "$(cat demo/data/campaign_${i}_results.json)" \
    --vertical gtm-engineer --session "$SID"
done
```

Each entry is a `campaign_results` schema row with sends,
reply rates, and an observation. Demo data has the variant
("agent-learning + auditability") outperforming the control
("Save 20 hours a week") by 3-4x on positive-reply rate
across 3 distinct cohorts.

In production, the agent's `analyze_campaign` skill would
emit these from operator-supplied raw outcomes.

### STEP 3 — gap/pattern detection

```bash
argo memory append memory "$(cat demo/data/detected_pattern.json)" \
  --vertical gtm-engineer --session "$SID"
```

A `patterns` row tagged `propose_skill` (the
`detect_gtm_pattern` skill spec's contract: tag is one of
`watch | test | propose_skill`). The pattern row cites
all 3 campaign IDs as evidence and includes a counterexample
(non-technical founders did NOT exhibit the same lift) so the
downstream skill will scope itself correctly.

### STEP 4 — propose_gtm_skill writes a candidate

```bash
argo memory append memory "$(cat demo/data/skill_candidate.json)" \
  --vertical gtm-engineer --session "$SID"
```

The candidate has the canonical fields the
`propose_gtm_skill` skill spec requires: `skill_name`,
`problem`, `reusable_pattern`, `evidence`, `expected_reuse`,
`risk`. The proposal is written to
`memory.learned_skill_candidates[]`, which is append-only —
superseded proposals get a new row rather than overwriting.

### STEP 5 — review + promote

```bash
argo skill review list --json
```

The candidate now shows under `.memory_candidates[]`. Then:

```bash
argo skill review promote <candidate_memory_id> --json
```

This materialises the candidate as a real `pending_review`
skill row with `lifecycle_state=pending_review` and
`vertical_id=gtm-engineer`. The skill is NOT active yet — it
shows under `.pending[]` in the same review-list surface.

### STEP 6 — approve → active

```bash
argo skill review approve <new_skill_id> --json
```

Returns `{"to": "active", ...}`. The skill is now in the
active catalog and visible via `argo skill list --json`.

### STEP 7 — improved second run

```bash
argo vertical eval gtm-engineer \
  --score \
  --actuals verticals/gtm-engineer/eval/known_good_actuals.jsonl \
  --json
```

Output: `pass_rate=1.0` on 10/10 scored fixtures. The platform
stayed green through the entire promote → activate cycle —
adding a new active skill did not regress the existing
contract.

The promoted skill body is now installed. The next
`analyze_campaign` turn that matches the trigger conditions
(technical founder + agent-platform GTM hypothesis) will
consult `technical_founder_learning_moat_outreach` instead of
re-deriving the framing from scratch.

## File layout

| Path | Purpose |
|---|---|
| `demo/gtm_engineer_learning_loop.sh` | Driver script. Idempotent, self-cleaning. |
| `demo/data/campaign_{1,2,3}_results.json` | The 3 campaign outcomes. |
| `demo/data/detected_pattern.json` | The pattern row that 3 observations would trigger. |
| `demo/data/skill_candidate.json` | The candidate `propose_gtm_skill` would emit. |
| `verticals/gtm-engineer/eval/known_good_actuals.jsonl` | Re-scored in step 7. |

## CI lock

`crates/argo-cli/tests/demo_gtm_engineer_loop.rs` shells out
to the script on every test run, so any breakage in the
underlying CLI surfaces (memory append, skill review, scored
eval) shows up immediately rather than next time the demo is
manually run.

## Why this loop matters

The defining capability of a vertical-bound agent (vs a
prompt-only assistant) is that it accumulates and promotes
learned skills inside its own namespace. Without an end-to-end
demo, the loop is theoretical — and platform changes break it
silently. This script + CI lock turns the loop into a contract.
