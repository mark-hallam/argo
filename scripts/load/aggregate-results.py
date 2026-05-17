#!/usr/bin/env python3
"""Aggregate one or more `oha -j` outputs + ps samples into a single
results blob for scripts/load/gateway-soak.sh.

Pure-stdlib. Called as a subprocess from the bash harness, never
imported. Acceptance shape per
docs/build-plan/streams/codex-gateway-load-soak-baseline.md:

  {profile, duration_s, requests_total, p50_ms, p95_ms, p99_ms,
   rss_kb_start, rss_kb_end, cpu_pct_avg, db_bytes_delta,
   audit_rows_delta}
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def _ms(seconds: float | None) -> float:
    return round(float(seconds) * 1000.0, 2) if seconds is not None else 0.0


def _load_oha_blobs(run_dir: Path, tags: list[str]) -> list[dict]:
    blobs = []
    for tag in tags:
        path = run_dir / f"{tag}.json"
        if not path.exists():
            continue
        try:
            blobs.append(json.loads(path.read_text()))
        except json.JSONDecodeError:
            # oha occasionally truncates on signal; skip and keep going.
            continue
    return blobs


def _rollup_latency(blobs: list[dict]) -> dict:
    # `oha -j` reports per-run percentiles. Without raw samples we
    # cannot recompute exact cross-run percentiles, so we take the
    # worst (max) of each percentile across runs as the conservative
    # answer — surfacing the slowest bucket, which is what an
    # operator looking at "did we breach p95?" cares about.
    p50 = p95 = p99 = 0.0
    total = 0
    success = 0
    rps = 0.0
    for b in blobs:
        lat = b.get("latencyPercentiles") or {}
        p50 = max(p50, _ms(lat.get("p50")))
        p95 = max(p95, _ms(lat.get("p95")))
        p99 = max(p99, _ms(lat.get("p99")))
        summary = b.get("summary") or {}
        total += int(summary.get("total", 0) or 0)
        success += int(summary.get("successful", summary.get("total", 0) or 0) or 0)
        rps += float(summary.get("requestsPerSec", 0.0) or 0.0)
    return {
        "p50_ms": p50,
        "p95_ms": p95,
        "p99_ms": p99,
        "requests_total": total,
        "requests_successful": success,
        "achieved_rps": round(rps, 2),
    }


def _rollup_samples(path: Path) -> dict:
    """Walk the ps samples.tsv (ts\trss_kb\tcpu_pct) into a summary."""
    rss_start = rss_end = 0
    rss_peak = 0
    cpu_sum = 0.0
    n = 0
    if path.exists():
        for line in path.read_text().splitlines():
            parts = line.split("\t")
            if len(parts) != 3:
                continue
            try:
                _, rss_kb, cpu_pct = parts
                rss_i = int(rss_kb)
                cpu_f = float(cpu_pct)
            except ValueError:
                continue
            if n == 0:
                rss_start = rss_i
            rss_end = rss_i
            rss_peak = max(rss_peak, rss_i)
            cpu_sum += cpu_f
            n += 1
    rss_growth_pct = 0.0
    if rss_start > 0:
        rss_growth_pct = round((rss_end - rss_start) / rss_start * 100.0, 2)
    cpu_avg = round(cpu_sum / n, 2) if n > 0 else 0.0
    return {
        "rss_kb_start": rss_start,
        "rss_kb_end": rss_end,
        "rss_kb_peak": rss_peak,
        "rss_growth_pct": rss_growth_pct,
        "cpu_pct_avg": cpu_avg,
        "samples": n,
    }


def _verdict(metrics: dict, thresholds: dict) -> tuple[str, list[str]]:
    """Return ('ok'|'warn'|'fail', [reason lines])."""
    notes = []
    worst = "ok"

    def bump(level):
        nonlocal worst
        order = {"ok": 0, "warn": 1, "fail": 2}
        if order[level] > order[worst]:
            worst = level

    p95 = metrics["p95_ms"]
    if p95 > thresholds["p95_warn_ms"]:
        notes.append(f"p95 {p95}ms > {thresholds['p95_warn_ms']}ms FAIL")
        bump("fail")
    elif p95 > thresholds["p95_ok_ms"]:
        notes.append(f"p95 {p95}ms > {thresholds['p95_ok_ms']}ms WARN")
        bump("warn")

    rss_pct = metrics["rss_growth_pct"]
    if rss_pct > thresholds["rss_growth_warn_pct"]:
        notes.append(
            f"RSS grew {rss_pct}% > {thresholds['rss_growth_warn_pct']}% FAIL (memleak suspect)"
        )
        bump("fail")
    elif rss_pct > thresholds["rss_growth_ok_pct"]:
        notes.append(
            f"RSS grew {rss_pct}% > {thresholds['rss_growth_ok_pct']}% WARN"
        )
        bump("warn")

    rows = metrics["audit_rows_delta"]
    bytes_d = metrics["db_bytes_delta"]
    if rows > 0:
        per_row = bytes_d / rows
        if per_row > thresholds["db_bytes_per_row_ok"]:
            notes.append(
                f"DB grew {per_row:.0f}B/audit-row > {thresholds['db_bytes_per_row_ok']}B WARN"
            )
            bump("warn")

    if not notes:
        notes.append("all thresholds OK")
    return worst, notes


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--profile", required=True)
    ap.add_argument("--soak-min", type=int, default=0)
    ap.add_argument("--start", required=True)
    ap.add_argument("--end", required=True)
    ap.add_argument("--duration-s", type=int, required=True)
    ap.add_argument("--base-url", required=True)
    ap.add_argument("--samples", required=True)
    ap.add_argument("--db-bytes-start", type=int, default=0)
    ap.add_argument("--db-bytes-end", type=int, default=0)
    ap.add_argument("--audit-rows-start", type=int, default=0)
    ap.add_argument("--audit-rows-end", type=int, default=0)
    ap.add_argument("--p95-ok-ms", type=int, required=True)
    ap.add_argument("--p95-warn-ms", type=int, required=True)
    ap.add_argument("--rss-growth-ok-pct", type=int, required=True)
    ap.add_argument("--rss-growth-warn-pct", type=int, required=True)
    ap.add_argument("--db-bytes-per-row-ok", type=int, required=True)
    ap.add_argument("--run-dir", required=True)
    ap.add_argument(
        "--tags", default="",
        help="space-separated list of oha tags written by the harness",
    )
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    run_dir = Path(args.run_dir)
    tags = [t for t in args.tags.split() if t]
    blobs = _load_oha_blobs(run_dir, tags)
    latency = _rollup_latency(blobs)
    samples = _rollup_samples(Path(args.samples))

    metrics = {
        "profile": args.profile,
        "soak_min": args.soak_min,
        "start": args.start,
        "end": args.end,
        "duration_s": args.duration_s,
        "base_url": args.base_url,
        "p50_ms": latency["p50_ms"],
        "p95_ms": latency["p95_ms"],
        "p99_ms": latency["p99_ms"],
        "requests_total": latency["requests_total"],
        "requests_successful": latency["requests_successful"],
        "achieved_rps": latency["achieved_rps"],
        "rss_kb_start": samples["rss_kb_start"],
        "rss_kb_end": samples["rss_kb_end"],
        "rss_kb_peak": samples["rss_kb_peak"],
        "rss_growth_pct": samples["rss_growth_pct"],
        "cpu_pct_avg": samples["cpu_pct_avg"],
        "ps_samples": samples["samples"],
        "db_bytes_start": args.db_bytes_start,
        "db_bytes_end": args.db_bytes_end,
        "db_bytes_delta": args.db_bytes_end - args.db_bytes_start,
        "audit_rows_start": args.audit_rows_start,
        "audit_rows_end": args.audit_rows_end,
        "audit_rows_delta": args.audit_rows_end - args.audit_rows_start,
        "tags": tags,
    }

    thresholds = {
        "p95_ok_ms": args.p95_ok_ms,
        "p95_warn_ms": args.p95_warn_ms,
        "rss_growth_ok_pct": args.rss_growth_ok_pct,
        "rss_growth_warn_pct": args.rss_growth_warn_pct,
        "db_bytes_per_row_ok": args.db_bytes_per_row_ok,
    }
    verdict, notes = _verdict(metrics, thresholds)

    out = {
        **metrics,
        "thresholds": thresholds,
        "verdict": verdict,
        "verdict_notes": notes,
    }
    Path(args.out).write_text(json.dumps(out, indent=2) + "\n")

    # Non-zero only on FAIL — WARN is informational and shouldn't fail
    # a CI invocation; the JSON carries the band so operators can
    # alert on warn separately.
    return 1 if verdict == "fail" else 0


if __name__ == "__main__":
    sys.exit(main())
