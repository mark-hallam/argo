# Memory and skills

Memory and skills are the two persistence surfaces a worker uses
across turns. Memory captures what the worker has seen; skills
capture what the worker has learned to do.

## Memory layers

Argo memory is layered:

1. **Worker** — per-worker scratchpad. Memories that only matter
   to one worker.
2. **Shared team** — tenant-scoped, author-stamped. Memories that
   belong to a team of workers (e.g. all support workers in a
   tenant).
3. **Global** — vertical-scoped, available to every worker
   instance of a vertical.

Layered lookup composes results — worker layer first, then team,
then global. Cross-tenant isolation is enforced by the store; you
cannot accidentally read another tenant's memory.

### Inspecting memory

```bash
argo memory list --file memory --worker <id>
argo memory shared-team list --tenant <id>
argo memory grep <query>
```

The full CLI is in [Reference → CLI](../reference/cli.md).

### Vector search

Memory entries are embedded with Voyage-3 and stored in
`sqlite-vec`. Lookup is k-NN cosine similarity, default `k = 8`.
Embeddings are recomputed lazily on read if the source row's
`embedding_version` is older than the active version.

## Skills

A skill is a learned procedure. The platform proposes skills from
successful trajectories (CU + executor traces) via the
SuccessMetric pipeline:

- A trajectory hits the proposal threshold
  (`confidence ≥ 0.8`, `trajectory_length ≤ 25`,
  `duration ≤ 600 s`).
- The proposed skill enters the
  [skill review](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/skycot/RUNBOOK.md)
  queue.
- A reviewer (human or judge) approves, rejects, or asks for
  refinement.
- Approved skills enter the active library.

### Skill quality scoring

After a skill ships, Argo scores it on each invocation
(success rate + p99 latency + retry count). The score is a 0-100
quality rolling average.

```bash
argo skill quality <name>
argo skill quality-list --quality-min 60
argo skill quality-trend <name>
```

Skills below a quality threshold are demoted; promoted skills can
A/B-test a new variant against the active version.

### Skill marketplace

Skills can be exported as `.skill.tar.gz` artifacts and shared
across tenants:

```bash
argo skill export <name> --output <name>.skill.tar.gz
argo skill import <name>.skill.tar.gz
```

Or via OCI:

```bash
argo skill publish <name> --tag v1
argo skill pull <oci-ref>
```

## Forge: automated skill optimization

`argo forge` runs a skill against eval seeds with various variants,
benchmarks each, and proposes a promotion. Auto-promotion is
gated by both a confidence threshold (default 0.85) and a judge
verdict; promotion fails CLOSED if the judge degrades.

```bash
argo forge optimize --skill <name> --examples seeds.yaml
argo forge bench --skill <name> --provider <id>
```

## Where this is documented

- [`docs/skycot/RUNBOOK.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/skycot/RUNBOOK.md)
  — operator-level skill review + memory curation.
- The skill review CLI lives at
  `argo skill review {list,show,approve,reject}` (see
  [CLI reference](../reference/cli.md)).
