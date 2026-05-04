# GTM Engineer launch microsite

A polished static page that turns the proven GTM Engineer learning
loop into a shareable launch artifact — designed for demo videos,
website embedding, and social sharing.

> SKYCOT builds AI employees that learn the job.
>
> The AI GTM Engineer analyzes a weak campaign, detects a pattern,
> proposes a learned skill, gets approval, and uses the skill on
> the next run.

This is **not** the production dashboard. It is a launch surface:
five sections, one Approve button, one animated transition,
receipts.

## Files

```
demo/launch-gtm-engineer/
├── index.html          structure (5 sections + meta)
├── style.css           dark-themed design system (mobile-responsive)
├── app.js              fetch data/demo.json + bind + Approve animation
├── data/demo.json      story payload — overwritten by capture_demo.sh
├── capture_demo.sh     runs the demo + writes data/demo.json
└── README.md           this file
```

No backend. No build step. No external paid services. Open
`index.html` in any modern browser.

## Open the demo

```bash
# Easiest — just open the file
open demo/launch-gtm-engineer/index.html

# OR serve it locally if your browser blocks file:// fetches
# (Chrome's strictest profile, some sandboxed browsers, etc.)
python3 -m http.server -d demo/launch-gtm-engineer 8000
# → http://localhost:8000/
```

The shipped `data/demo.json` is a ready-to-display sample. The page
loads it, fills every `[data-bind]` element, and wires the
**Approve skill** button to its lifecycle-pulse animation. Click
the button to watch `pending_review` → `active` and the page
auto-scrolls into the After section.

## Capture deterministic data (no API key)

```bash
cargo build --bin argo
ARGO_BIN=$(pwd)/target/debug/argo \
  bash demo/launch-gtm-engineer/capture_demo.sh
```

`capture_demo.sh` runs the existing
`demo/gtm_engineer_learning_loop.sh` against the mock provider,
extracts the session/candidate/skill UUIDs + the scored-eval
result, merges them into the static presentation template, and
writes the merged JSON atomically to `data/demo.json`. Re-open
`index.html` and the receipts in the Proof section now reflect
the run you just made.

Deterministic mode requires nothing beyond `argo`, `bash`, `jq`,
and `awk`. No API key. Runs in ~2 seconds.

## Capture live data (real LLM)

```bash
export ANTHROPIC_API_KEY=...
LIVE_LLM=1 ARGO_BIN=$(pwd)/target/debug/argo \
  bash demo/launch-gtm-engineer/capture_demo.sh
# OR equivalently:
ARGO_BIN=$(pwd)/target/debug/argo \
  bash demo/launch-gtm-engineer/capture_demo.sh --live
```

In live mode, `capture_demo.sh` invokes the demo script with
`--live`, which boots `argo gateway start` and drives three real
turns through `anthropic/claude-sonnet-4-6` (analyze → detect
pattern → propose skill, with a JSON-shape contract on the third
turn). The propose-step output becomes the
`learned_skill_candidate` memory entry that the rest of the
lifecycle promotes + activates.

The capture script never echoes, stores, writes, or logs
`ANTHROPIC_API_KEY` — it inherits it via the process env and
hands it untouched to the gateway subprocess. If the env var is
missing when `--live` is requested, the script fails immediately
with `FAIL: --live / LIVE_LLM=1 requires ANTHROPIC_API_KEY in
env` before any state is written.

## What counts as success

After running `capture_demo.sh`, refreshing `index.html` should
show:

| Section | Indicator |
|---|---|
| Hero | mode pill reads `deterministic` or `live`, captured-at date is today |
| Before | weak-message body + the campaign metrics read **100 emails sent · 2 replies · 0 positive** |
| Learning Moment | candidate fields populated; **Approve skill** button visible |
| After | active-skill pill matches the candidate's skill name |
| Proof | all 7 receipt rows show real UUIDs + `10/10 passed pass_rate=1.0` |

Click **Approve skill** in the Learning Moment section to see the
`pending_review → active` animation, then auto-scroll into After.

## Recording the launch video

A 60-90 second demo video pattern that hits every story beat:

1. **0:00-0:08** — Hero. Pause on the headline. Read the
   subheadline aloud.
2. **0:08-0:25** — Click "Watch the GTM Engineer learn" → Before.
   Voiceover: "The AI inherited a generic outreach campaign —
   100 emails sent, 2 replies, 0 positive. The agent diagnoses
   why."
3. **0:25-0:50** — Scroll into Learning Moment. Voiceover:
   "Three reinforcing observations across distinct cohorts. The
   agent emits a learned-skill candidate — but it never
   auto-activates. Operator review is mandatory."
4. **0:50-1:05** — Click **Approve skill**. Watch the lifecycle
   badge pulse `pending_review → active`. Auto-scroll into After.
5. **1:05-1:25** — After section. Voiceover: "Same prospect, next
   send — anchored in a verified signal, audit-chain framing,
   single CTA. The agent now consults the skill it just learned."
6. **1:25-1:40** — Scroll into Proof. Voiceover: "Every step is
   a real CLI call. Real session, real memory writes, real skill
   row in `lifecycle_state=active`. Re-running the scored eval
   confirms the platform stayed green throughout."

For best capture quality:
- Resize browser to ~1280×800 for 16:10 video framing.
- Use a screen recorder that captures the page at 60fps so the
  Approve-button spinner + lifecycle pulse animations look smooth.
- Capture with the deterministic JSON first (snappy demo).
  Re-record once with `--live` if you want a "yes this is real"
  cut for skeptical viewers.

## Conventions

- Static-only: no React, no node_modules, no build step.
- Each file under 500 LOC (CI-checked workspace-wide).
- `capture_demo.sh` writes `data/demo.json` atomically (tmp +
  mv) so a partial failure never leaves a half-written file.
- `data/demo.json` ships a sensible sample so `index.html`
  works without a capture step. Tests validate the shape of
  whatever JSON is committed.
- The Approve button's animation is a story-mode visual — the
  real `argo skill review approve` happens inside
  `capture_demo.sh`'s child run. The button is the operator
  beat the launch video pivots on.
