# Vertical demo scripts

These scripts provide deterministic `--mock-mode` learning-loop demos for the
five v1 enterprise vertical packs. Each script validates the local pack, links
it through `argo packs dev-link`, walks the first-task Skill IR graph, then
records a mock learned-skill candidate and promotion artifact.

Run from the repository root with a built `argo` binary on `PATH`, or set
`ARGO_BIN` to the binary path:

```bash
ARGO_BIN=/path/to/argo verticals/recruiter/demo/learning_loop.sh --mock-mode
```

| Vertical | Script | Demo path |
|---|---|---|
| Recruiter | `verticals/recruiter/demo/learning_loop.sh` | Screen a resume batch, learn a resume-evidence gate, promote it. |
| AP Specialist | `verticals/ap-specialist/demo/learning_loop.sh` | Process invoice intake, learn low-confidence exception routing, promote it. |
| Vendor Manager | `verticals/vendor-manager/demo/learning_loop.sh` | Score supplier proposals, learn security-risk weighting, promote it. |
| Support Tier 1 | `verticals/support-tier1/demo/learning_loop.sh` | Triage support tickets, learn SLA-sensitive escalation, promote it. |
| Deal Desk | `verticals/deal-desk/demo/learning_loop.sh` | Validate a quote, learn payment-term exception checks, promote it. |

CI smoke can run all five with:

```bash
for script in \
  verticals/recruiter/demo/learning_loop.sh \
  verticals/ap-specialist/demo/learning_loop.sh \
  verticals/vendor-manager/demo/learning_loop.sh \
  verticals/support-tier1/demo/learning_loop.sh \
  verticals/deal-desk/demo/learning_loop.sh
do
  "$script" --mock-mode
done
```
