# Argo Roadmap

The canonical phase plan lives in `docs/plan/argo-build-plan.md` §27.

## Current phase

**Phase 0 — Skeleton + initial Phase 1 progress.**

## Status

| Phase | Status | Gate |
|---|---|---|
| 0 — Skeleton | ✅ in progress | `cargo test --workspace` green; CI green on Linux/macOS/Windows |
| 1 — Walking skeleton + budget framework | ⏳ partially started | End-to-end test (user → file_read → answer); cold boot < 200 ms |
| 2 — Provider breadth + cost router | ⏳ | ≥ 85% pass on `core` and `routing-correctness` |
| 3 — Tools, memory, skills, judge | ⏳ | ≥ 85% pass on `core` + `memory` + `judge-disagreement` |
| 4 — Plugins + MCP | ⏳ | Plugin chaos test green; off-the-shelf MCP servers work |
| 4.5 — ACP + bundled MCP servers | ⏳ | VS Code/Zed/JetBrains drive Argo via ACP |
| 5 — Delegation + sandboxes | ⏳ | Budget tree property tests; sibling-merge tests |
| 6 — Gateway + multi-agent + RBAC | ⏳ | 24-hour soak; ≥ 1,000 concurrent sessions |
| 6.5 — Workspace marketplace | ⏳ | Push → fresh machine pull → byte-identical |
| 7 — Polish + replay + perf + docs | ⏳ | All NFRs green |
| 7.5 — Argo Forge | ⏳ | Optimize ref skill ≥ 70% improvement, ≤ 8 min, ≤ $5 |

## Next gate

End of Phase 0 → cargo test green; CI green on three OSes; LOC budget enforced.
