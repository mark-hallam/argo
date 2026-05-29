# Gap-closure plans — ready-for-go implementation designs (2026-05-29)

Three implementation-ready plans for the highest-value competitive gaps found in
the [2026-05-29 competitive gap analysis](../../research/2026-05-29-competitive-gap-analysis.md).
Each is written so the operator can make the (few) decisions it flags, then it
drops straight into the swarm as briefs.

Authored overnight 2026-05-29 in preference to half-building these big swings
blind — each needs an operator design call (or, for voice, an external API key),
and "land scaffold now, wire later" is the exact drift `DRIFT_PREVENTION.md`
warns against.

| # | Plan | Gap | The decision you need to make | Effort (first useful slice) |
|---|---|---|---|---|
| [01](01-live-voice.md) | **Live voice** (Portia speaks/listens) | #4 + your keenest interest | Which speech vendor (ElevenLabs+Deepgram recommended) — needs an API key | L1+L2+L3a ≈ 4–6 wk → "pitches live in the browser" |
| [02](02-multi-worker-orchestration.md) | **Multi-worker orchestration** | #2 + our core thesis | Supervisor vs DAG model (Supervisor recommended); drop "auction" | Engine (fan-out + deps) = MEDIUM |
| [03](03-self-improvement-training-export.md) | **Self-improvement / training export** | #3 (thinner than it looks) | Export format (SFT JSONL recommended) | Step 1 export — ✅ **SHIPPED 2026-05-29** (`argo eval export-training-data`) |
| [04](04-enterprise-governance-and-audit-readiness.md) | **Enterprise governance + audit-readiness** | #1 (**highest priority**) | Report vs. dashboard for move A (report-first recommended) | Move A (package existing moat) = **LOW**, highest credibility-per-effort |

**If you want one BUILT next (not just planned):** #03 Step 1 — it's bounded,
uses 100% real data, needs no external key, and pairs with our audit moat.

**If you want the biggest wow:** #01 voice L1→L3a (browser) — but it's blocked
on choosing a vendor + providing a key.

**Most architectural (get the model call right first):** #02.
