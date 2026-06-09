#!/usr/bin/env python3
"""north-star-metrics.py — DEC-042 Stage 0 verifier.

Computes the DEC-042 §6 north-star metrics for the continuous self-improvement
loop from REAL sources (git log, gh, the drift script, the BUILD_STATUS ledger)
and emits them as JSON. Metrics that are not yet instrumented are emitted as
`null` with an explicit `_status: "not-yet-instrumented"` note — never a
fabricated number (DEC-029 NO THEATER).

This script IS the Stage-0 checkable verifier: "metrics emit." Run it and the
build-loop metrics that can be computed today come out with real values; the
rest are honestly flagged as pending instrumentation in later stages.

Usage:
    python3 scripts/north-star-metrics.py                 # print JSON to stdout
    python3 scripts/north-star-metrics.py --write-heartbeat \\
        docs/skycot/agents/heartbeats/conductor.json      # inject into heartbeat

Exit status: 0 when at least one build-loop metric computed from a real source;
non-zero if every real-source probe failed (so CI can treat that as "verifier
did not emit").
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LEDGER = REPO / "docs" / "build-plan" / "BUILD_STATUS.md"
DRIFT = REPO / "scripts" / "find-stale-claim-drift.py"
CAPTURE = REPO / "scripts" / "capture-recurring-nits.py"

# Metrics named in DEC-042 §6 that have no real source wired yet. Listed here so
# the emitted payload is self-documenting about what Stage 1+ still has to build.
UNINSTRUMENTED = {
    "build_loop": {
        "pct_done_claims_pass_cold_reverify": "B-2/B-4: needs a cold re-verify ledger (the NO-THEATER metric)",
        "mean_time_to_green_ci": "needs per-PR CI-start→green timestamps",
        "defect_escape_rate": "needs a main-branch defect tracker (bugs that reached main)",
    },
    "product_loop": {
        "teammate_task_success_rate": "needs eval-harness graded-run rollup",
        "skills_created_per_week": "P-1: real skill distillation not yet emitting",
        "skills_adopted_per_week": "P-1: skill-adoption telemetry not yet wired",
        "approval_gate_catch_rate": "needs held-vs-passed sensitive-action tally",
        "audit_chain_integrity": "needs a chain-verify probe (hash-chain walk)",
        "time_to_first_useful_action": "needs new-hire→first-action timestamps",
    },
}


def _run(cmd: list[str], timeout: int = 30) -> str | None:
    """Run a command, return stripped stdout, or None on any failure.

    None is the honest 'could not measure' signal — callers must not substitute
    a fabricated value for it.
    """
    try:
        out = subprocess.run(
            cmd, cwd=REPO, capture_output=True, text=True, timeout=timeout
        )
    except (subprocess.SubprocessError, OSError):
        return None
    if out.returncode != 0 and not out.stdout.strip():
        return None
    return out.stdout.strip()


def merged_prs(since: str) -> int | None:
    """Count distinct (#NNNN) squash-merge subjects on origin/main since `since`."""
    raw = _run(["git", "log", f"--since={since}", "--oneline", "origin/main"])
    if raw is None:
        return None
    return len(set(re.findall(r"\(#(\d+)\)", raw)))


def open_pr_count() -> int | None:
    """Open-PR count via gh (None if gh is unavailable/unauthenticated)."""
    raw = _run(["gh", "pr", "list", "--state", "open", "--json", "number",
                "--jq", ". | length"], timeout=45)
    if raw is None or not raw.isdigit():
        return None
    return int(raw)


def drift_rows() -> int | None:
    """Stale-claim drift-row count via the canonical drift script."""
    raw = _run(["python3", str(DRIFT), "--quiet"])
    if raw is None:
        return None
    m = re.search(r"Found (\d+) drift", raw)
    return int(m.group(1)) if m else None


def recurring_nit_count() -> int | None:
    """Count of uncaptured recurring nits (DEC-042 B-3 capturer, --quiet).

    Best-effort: returns None if the capturer is missing or errors — never a
    fabricated value (DEC-029 NO THEATER). The value trends DOWN as nits get
    captured, which is the compounding signal B-3 exists to produce.
    """
    if not CAPTURE.exists():
        return None
    raw = _run(["python3", str(CAPTURE), "--quiet"])
    if raw is None or not raw.strip().isdigit():
        return None
    return int(raw.strip())


def ledger_queue() -> dict[str, int] | None:
    """Status histogram of the BUILD_STATUS ledger table rows."""
    if not LEDGER.exists():
        return None
    counts: dict[str, int] = {}
    row = re.compile(r"^\|[^|]*\|[^|]*\|\s*([A-Z_]+)\s*\|")
    for line in LEDGER.read_text().splitlines():
        m = row.match(line)
        if m:
            counts[m.group(1)] = counts.get(m.group(1), 0) + 1
    return {
        "todo": counts.get("TODO", 0),
        "claimed": counts.get("CLAIMED", 0),
        "in_review": counts.get("IN_REVIEW", 0),
        "blocked": counts.get("BLOCKED", 0),
    }


def collect() -> dict:
    """Assemble the full north-star payload from real sources."""
    m7 = merged_prs("7 days ago")
    queue = ledger_queue()
    build = {
        "merged_prs_per_day_7d": round(m7 / 7.0, 2) if m7 is not None else None,
        "merged_prs_last_7d": m7,
        "merged_prs_last_24h": merged_prs("1 day ago"),
        "open_prs": open_pr_count(),
        "drift_rows": drift_rows(),
        "queue_depth": queue,
        # B-3 (DEC-042 §3): real, computed from the recurring-nit capturer.
        "recurring_nit_count": recurring_nit_count(),
    }
    for key, reason in UNINSTRUMENTED["build_loop"].items():
        build[key] = None
    product = {key: None for key in UNINSTRUMENTED["product_loop"]}

    real_probes = [build["merged_prs_last_7d"], build["drift_rows"], queue]
    emitted = any(p is not None for p in real_probes)

    return {
        "_schema": "dec042-north-star-v1",
        "_doc": "docs/skycot/decisions/DEC-042-continuous-self-improvement-loop.md#6",
        "generated_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "build_loop": build,
        "product_loop": product,
        "_uninstrumented": UNINSTRUMENTED,
        "_emitted": emitted,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description="Emit DEC-042 north-star metrics.")
    ap.add_argument("--write-heartbeat", metavar="PATH",
                    help="inject/update north_star_metrics in this heartbeat JSON")
    args = ap.parse_args()

    payload = collect()
    print(json.dumps(payload, indent=2))

    if args.write_heartbeat:
        hb_path = Path(args.write_heartbeat)
        try:
            hb = json.loads(hb_path.read_text())
        except (OSError, json.JSONDecodeError) as exc:
            print(f"[north-star] could not read heartbeat: {exc}", file=sys.stderr)
            return 2
        hb["north_star_metrics"] = payload
        hb_path.write_text(json.dumps(hb, indent=2) + "\n")
        print(f"[north-star] wrote north_star_metrics into {hb_path}", file=sys.stderr)

    # Verifier contract: emit non-zero only if NO real-source metric came through.
    return 0 if payload["_emitted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
