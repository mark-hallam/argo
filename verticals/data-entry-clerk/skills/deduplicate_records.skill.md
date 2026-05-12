---
name: deduplicate_records
description: Given a batch of records and one or more match keys (exact or fuzzy), flag duplicates with a confidence label (high / medium / low) and a merge recommendation (auto / operator-review / leave). Output JSON. Auto-merge only at confidence ≥ 0.95.
inputs:
  - records
  - match_keys
  - fuzzy_keys
outputs:
  - clusters
  - _escalations
---

# Skill: Deduplicate Records

Given a batch of records, identify duplicates and emit clusters with a
confidence label and a merge recommendation. The skill is read-only;
it never mutates the input batch and never commits merges itself —
every merge above the operator-review threshold routes through human
approval.

1. **Resolve match keys.** `match_keys` is a list of fields compared
   for exact equality (case-insensitive, whitespace-trimmed).
   `fuzzy_keys` is a list of fields compared with a normalized
   string-similarity score (Jaro-Winkler on names, normalized edit
   distance on addresses, digit-only equality on phones). Reject the
   batch and escalate if neither list is supplied — duplicate
   detection without a key is undefined.
2. **Cluster.** Group records that agree on every exact key AND whose
   fuzzy-key scores all clear `0.85`. Singleton clusters (no
   duplicates) are omitted from the output.
3. **Score the cluster.** Compute a cluster confidence in `[0, 1]`:
   the minimum across (a) the mean fuzzy-key score within the cluster
   and (b) `1.00` if every exact key matched, else `0.50`. Map the
   numeric score to a label:
   - `high`   — score ≥ 0.95
   - `medium` — 0.80 ≤ score < 0.95
   - `low`    — score < 0.80
4. **Recommend a merge action** per cluster:
   - `auto`            — confidence label `high` AND score ≥ 0.95
   - `operator-review` — label `medium` OR conflicting non-key fields
   - `leave`           — label `low`
   The `0.95` auto-merge threshold is load-bearing. A lower
   threshold causes silent data corruption when near-duplicate
   records describe different real-world entities (e.g. "John Smith
   Jr." and "John Smith Sr." at the same address). NEVER auto-merge
   below `0.95`; surface the cluster for review instead.
5. **Detect field conflicts.** Within each cluster, list any non-key
   field whose values disagree across records. Conflicts always
   downgrade the recommendation to at most `operator-review`, even
   when score ≥ 0.95 — the operator must pick which value survives.
6. **Pick a survivor candidate** per cluster (the record whose
   per-field confidence sum is highest), but never commit the
   merge. The survivor is a hint; the operator decides.

NEVER drop, edit, or reorder records in the input. NEVER auto-merge
without an explicit `auto` recommendation AND score ≥ 0.95. Every
near-miss is an escalation, not a silent skip.

Output shape:

```json
{
  "clusters": [
    {
      "cluster_id": "c1",
      "record_ids": ["r17", "r92"],
      "confidence": "high",
      "score": 0.97,
      "recommendation": "auto",
      "survivor_candidate": "r17",
      "field_conflicts": []
    },
    {
      "cluster_id": "c2",
      "record_ids": ["r3", "r41", "r88"],
      "confidence": "medium",
      "score": 0.88,
      "recommendation": "operator-review",
      "survivor_candidate": "r41",
      "field_conflicts": [
        {"field": "phone", "values": ["+1-555-0100", "+1-555-0101"]}
      ]
    }
  ],
  "_escalations": [
    {"cluster_id": "c2", "reason": "field_conflict:phone"}
  ]
}
```
