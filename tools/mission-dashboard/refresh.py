#!/usr/bin/env python3
"""Mission dashboard generator.

Reads (read-only):
  - docs/build-plan/MISSION_BACKLOG.md   (canonical, ranked above the rest)
  - docs/build-plan/BUILD_STATUS.md      (live ledger; cross-ref for [BRIEF: x])
  - docs/build-plan/BUILD_PLAN_V11.md    (headline framing)
  - docs/skycot/MISSION.md               (six load-bearing claims)
  - git log                              (recent activity + burn-up history)

Writes:
  - mission-dashboard/dashboard.html     (self-contained, embedded JSON)

Refresh = re-run this script. The dashboard's "refresh" button copies the
command to your clipboard.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from collections import defaultdict
from datetime import datetime, timedelta, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent

# Resolve the canonical Argo checkout. Resolution order:
#   1. ARGO_DASHBOARD_ROOT env var (explicit override)
#   2. Auto-detect: assume this script lives at <repo>/tools/mission-dashboard/
#      and walk up two levels — works when committed inside the repo.
#   3. Hardcoded fallback ~/code/argo (operator's canonical
#      checkout, kept for the ~/.argo/dashboard/ deployment)
#   4. Last resort: `git rev-parse --show-toplevel` from HERE
# A valid root is a directory that contains both .git/ and the BUILD_STATUS
# ledger. Anything else is rejected so we don't silently render against a
# stranger repo.

def _is_argo_root(p: Path) -> bool:
    return p.is_dir() and (p / ".git").exists() and (p / "docs/build-plan/BUILD_STATUS.md").is_file()


def repo_root() -> Path:
    env = os.environ.get("ARGO_DASHBOARD_ROOT")
    if env:
        ep = Path(env).expanduser().resolve()
        if _is_argo_root(ep):
            return ep
    auto = HERE.parent.parent
    if _is_argo_root(auto):
        return auto
    fallback = Path("~/code/argo")
    if _is_argo_root(fallback):
        return fallback
    out = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], cwd=HERE)
    return Path(out.decode().strip())


# ─── Source readers ─────────────────────────────────────────────────────────

ROOT = repo_root()
BACKLOG = ROOT / "docs/build-plan/MISSION_BACKLOG.md"
LEDGER = ROOT / "docs/build-plan/BUILD_STATUS.md"
STREAMS_DIR = ROOT / "docs/build-plan/streams"
PLAN = ROOT / "docs/build-plan/BUILD_PLAN_V11.md"
ROADMAP = ROOT / "docs/build-plan/PRODUCT_ROADMAP.md"
MISSION = ROOT / "docs/skycot/MISSION.md"


def read(p: Path) -> str:
    return p.read_text(encoding="utf-8")


def _auto_pull_once() -> str:
    """Try to fast-forward ROOT to origin/main. Returns a one-line status
    suitable for logging. Never raises — silent failure is the safe default
    for a background timer that runs unattended."""
    try:
        head = subprocess.run(
            ["git", "symbolic-ref", "--short", "HEAD"],
            cwd=ROOT, capture_output=True, text=True, check=False, timeout=10,
        )
        if head.returncode != 0 or head.stdout.strip() != "main":
            return f"skip: HEAD is {head.stdout.strip() or '<detached>'}, not main"
        subprocess.run(
            ["git", "fetch", "origin", "main"],
            cwd=ROOT, capture_output=True, check=False, timeout=60,
        )
        # `git pull --ff-only` aborts cleanly (non-zero rc, no working-tree
        # mutation) if there are local commits or conflicting uncommitted
        # changes — exactly the conservative behavior we want.
        r = subprocess.run(
            ["git", "pull", "--ff-only", "origin", "main"],
            cwd=ROOT, capture_output=True, text=True, check=False, timeout=60,
        )
        if r.returncode == 0:
            first = (r.stdout or "").splitlines()[:1]
            return "pulled: " + (first[0] if first else "ok")
        msg = ((r.stderr or r.stdout or "").splitlines() or ["unknown"])[0]
        return f"skip: ff refused ({msg.strip()[:120]})"
    except Exception as exc:
        return f"skip: {type(exc).__name__}: {exc}"


def _auto_pull_loop(interval_seconds: int) -> None:
    import time
    while True:
        try:
            ts = datetime.now().strftime("%H:%M:%S")
            sys.stderr.write(f"[{ts}] auto-pull: {_auto_pull_once()}\n")
            sys.stderr.flush()
        except Exception:
            pass
        time.sleep(interval_seconds)


# ─── BUILD_STATUS ledger parser ─────────────────────────────────────────────

LEDGER_STATUSES = {"MERGED", "TODO", "CLAIMED", "IN_REVIEW", "BLOCKED", "DEFERRED-v1.x", "REVERTED"}


# Recognises the ledger's native phase divider rows, e.g.
#   | **▼ PHASE A — stop bleeding credibility (highest priority)** | — | … |
# Subsequent rows belong to that phase until the next divider. This is the
# conductor's live grouping — primary lens, ahead of MISSION_BACKLOG.
PHASE_DIVIDER_RE = re.compile(
    r"\*\*▼\s*PHASE\s+([A-F])\s*[—–-]\s*([^*]+?)\*\*"
)


def parse_ledger(text: str) -> tuple[dict[str, dict], dict[str, str]]:
    """Return (briefs, phase_titles).

    briefs: brief_id → {owner, status, pr, last_update, phase}
    phase_titles: phase_letter → title text extracted from the divider row

    `phase` is the letter A-F from the most recent `▼ PHASE X` divider row
    above this row, or None for rows above the first divider. Phase titles
    are auto-extracted so the dashboard never goes out of sync with renames
    the conductor makes in BUILD_STATUS.md."""
    out: dict[str, dict] = {}
    phase_titles: dict[str, str] = {}
    cur_phase: str | None = None
    for line in text.splitlines():
        if not line.startswith("|"):
            continue
        m = PHASE_DIVIDER_RE.search(line)
        if m:
            cur_phase = m.group(1)
            title = m.group(2).strip().rstrip("*").strip()
            # Strip only administrative-priority hints, keep informative parens
            # like "(self-improvement)" intact.
            title_clean = re.sub(r"\s*\((?:highest|lowest|low|high)\s+priority\)\s*$", "", title, flags=re.I).strip()
            phase_titles[cur_phase] = title_clean[:1].upper() + title_clean[1:] if title_clean else ""
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 5:
            continue
        brief, owner, status, pr, last = cells[0], cells[1], cells[2], cells[3], cells[4]
        if brief.startswith("---") or brief.lower() == "brief":
            continue
        brief = brief.strip("*")
        if not brief or brief == "—":
            continue
        # Phantom sub-divider rows like `**▼ MVP v1.0**` inside a phase: the
        # outer ▼ PHASE divider is caught above by PHASE_DIVIDER_RE, but these
        # inline group headers slip through and would otherwise inflate the
        # phase brief count with status="—" rows.
        if brief.lstrip().startswith("▼"):
            continue
        out[brief] = {
            "owner": owner,
            "status": status,
            "pr": pr,
            "last_update": last,
            "phase": cur_phase,
        }
    return out, phase_titles


# Note on Phase F: BUILD_STATUS.md titles it "scale-out" but in practice it's
# the historical catchall — most pre-Tick-198 v0.x work (A-executor-trait,
# B-computer-use-executor, ... H-auto-approval, plus architectural splits)
# lives there alongside genuine scale-out items. That's why F reads ~98% done.

LEDGER_COMPLETION_WEIGHT = {
    "MERGED": 1.0,
    "IN_REVIEW": 0.7,
    "CLAIMED": 0.4,
    "TODO": 0.1,
    "BLOCKED": 0.0,
    "REVERTED": 0.0,  # merged then backed out — counts against progress
    "DEFERRED-v1.x": None,  # excluded from denominator
    "DEFERRED-operator": None,  # excluded from denominator (operator-deferred)
}


def compute_ledger_phase_stats(ledger: dict[str, dict], phase_titles: dict[str, str]) -> dict:
    """Compute live-ledger phase progress directly from BUILD_STATUS.md.

    This is the dashboard's PRIMARY lens — what the conductor sees.
    MISSION_BACKLOG-derived stats remain available as a secondary lens for
    strategic-completeness questions.

    `phase_titles` is the auto-extracted divider-row map from `parse_ledger`."""
    by_phase: dict[str, dict] = {}
    # Walk only the phases the ledger actually contains, plus A–F for stable
    # column order even if a phase is empty.
    phase_letters = sorted(set("ABCDEF") | set(phase_titles.keys()))
    for letter in phase_letters:
        by_phase[letter] = {
            "letter": letter,
            "title": phase_titles.get(letter, f"Phase {letter}"),
            "n": 0,
            "counts": defaultdict(int),
            "blocked_briefs": [],
            "in_review_briefs": [],
            "claimed_briefs": [],
            "todo_briefs": [],
            "weight_num": 0.0,
            "weight_den": 0,
        }

    unphased: list[str] = []
    for brief, row in ledger.items():
        ph = row.get("phase")
        st = row.get("status", "")
        if ph not in by_phase:
            unphased.append(brief)
            continue
        b = by_phase[ph]
        b["n"] += 1
        b["counts"][st] += 1
        # Bucket by status so the drawer can list each group on click.
        if st == "BLOCKED":
            b["blocked_briefs"].append(brief)
        elif st == "IN_REVIEW":
            b["in_review_briefs"].append(brief)
        elif st == "CLAIMED":
            b["claimed_briefs"].append(brief)
        elif st == "TODO":
            b["todo_briefs"].append(brief)
        w = LEDGER_COMPLETION_WEIGHT.get(st)
        if w is None:
            continue
        b["weight_num"] += w
        b["weight_den"] += 1

    for b in by_phase.values():
        b["pct"] = (b["weight_num"] / b["weight_den"] * 100.0) if b["weight_den"] else 0.0
        b["counts"] = dict(b["counts"])
        del b["weight_num"]
        del b["weight_den"]

    # totals across all phased rows
    total_phased = sum(b["n"] for b in by_phase.values())
    total_done = sum(b["counts"].get("MERGED", 0) for b in by_phase.values())
    total_blocked = sum(b["counts"].get("BLOCKED", 0) for b in by_phase.values())
    weighted_num = 0.0
    weighted_den = 0
    for row in ledger.values():
        if row.get("phase") not in by_phase:
            continue
        w = LEDGER_COMPLETION_WEIGHT.get(row.get("status", ""))
        if w is None:
            continue
        weighted_num += w
        weighted_den += 1
    pct_total = (weighted_num / weighted_den * 100.0) if weighted_den else 0.0

    return {
        "by_phase": by_phase,
        "total_phased": total_phased,
        "total_done": total_done,
        "total_blocked": total_blocked,
        "pct_total": pct_total,
        "unphased_count": len(unphased),
    }


# ─── Tier parser (reads docs/build-plan/streams/*.md) ──────────────────────

# Tier markers live in the brief stream files, not in BUILD_STATUS.md. The
# operator's 2026-05-10 strategic-priority directive defines four tiers:
#   M — Mission MVP critical path (v1.0 ship gate)
#   A — tool-ship Tier A (must-have tools)
#   B — tool-ship Tier B (nice-to-have tools)
#   P — Platform / split-prep / cleanup (deferred)
# Pattern in stream files: `**Tier M — ...**` or `**Tier M (...) — ...**`
TIER_MARKER_RE = re.compile(r"\*\*\s*Tier\s+([MABP])\b", re.I)


def parse_tier_map(ledger: dict[str, dict] | None = None) -> dict[str, str]:
    """Walk docs/build-plan/streams/*.md and return {brief_id: 'M'|'A'|'B'|'P'}.

    Briefs without an explicit tier marker are absent from the map. This is
    one filesystem walk + one small read per stream file (only the first ~50
    lines are searched — the marker reliably appears near the top).

    Secondary source: if `ledger` is provided, also scan each row's
    `last_update` cell for `**Tier [MABP]` — many briefs (especially
    recently authored mission-* ones) carry the tier marker in the
    BUILD_STATUS row note but not in the stream-file head.
    """
    out: dict[str, str] = {}
    if STREAMS_DIR.is_dir():
        for p in STREAMS_DIR.iterdir():
            if not p.is_file() or not p.name.endswith(".md"):
                continue
            brief = p.name[:-3]  # strip .md
            try:
                with p.open("r", encoding="utf-8", errors="ignore") as f:
                    head = "".join(next(f, "") for _ in range(50))
            except OSError:
                continue
            m = TIER_MARKER_RE.search(head)
            if m:
                out[brief] = m.group(1).upper()
    if ledger:
        for brief, row in ledger.items():
            if brief in out:
                continue
            note = row.get("last_update", "") or ""
            m = TIER_MARKER_RE.search(note)
            if m:
                out[brief] = m.group(1).upper()
    return out


def compute_tier_stats(ledger: dict[str, dict], tier_map: dict[str, str]) -> dict:
    """Group ledger rows by tier (M / A / B / P / untiered) with status
    breakdowns. Tier M is the v1.0 ship-gate; surfaced prominently in the UI."""
    tiers: dict[str, dict] = {
        t: {"n": 0, "counts": defaultdict(int), "rows": []}
        for t in ("M", "A", "B", "P")
    }
    for brief, row in ledger.items():
        tier = tier_map.get(brief)
        if tier is None:
            continue
        bucket = tiers[tier]
        bucket["n"] += 1
        bucket["counts"][row.get("status", "")] += 1
        bucket["rows"].append({
            "brief": brief,
            "status": row.get("status", ""),
            "owner": row.get("owner", ""),
            "phase": row.get("phase"),
            "pr": row.get("pr", ""),
        })
    # Sort rows: MERGED first, then IN_REVIEW, CLAIMED, TODO, BLOCKED, DEFERRED
    status_rank = {"MERGED": 0, "IN_REVIEW": 1, "CLAIMED": 2, "TODO": 3, "BLOCKED": 4, "DEFERRED-v1.x": 5}
    for bucket in tiers.values():
        bucket["counts"] = dict(bucket["counts"])
        bucket["rows"].sort(key=lambda r: (status_rank.get(r["status"], 99), r["brief"]))
        merged = bucket["counts"].get("MERGED", 0)
        bucket["pct"] = (merged / bucket["n"] * 100.0) if bucket["n"] else 0.0
    return {
        "by_tier": tiers,
        "tier_titles": {
            "M": "v1.0 ship gate (Mission MVP critical path)",
            "A": "Tool-ship Tier A (must-have)",
            "B": "Tool-ship Tier B (nice-to-have)",
            "P": "Platform / split-prep / cleanup",
        },
    }


# ─── PRODUCT_ROADMAP tracks parser ──────────────────────────────────────────

# Tracks A–G live in `docs/build-plan/PRODUCT_ROADMAP.md` (operative post-MVP
# plan, authored 2026-05-11). The seven-tracks table is the authoritative
# state-of-work summary; each row is:
#   | <Letter> | <Scope> | <Who> | **<STATUS>** [<date>] [— <note>] | <Closes> |
# Where STATUS ∈ {DONE, IN PROGRESS, QUEUED, NOT STARTED}.
ROADMAP_ROW_RE = re.compile(
    r"^\|\s*([A-Z])\s*\|([^|]+)\|([^|]+)\|\s*\*\*([A-Z ]+?)\*\*([^|]*)\|([^|]+)\|\s*$"
)

ROADMAP_STATUS_WEIGHT = {
    "DONE": 1.0,
    "IN PROGRESS": 0.5,
    "QUEUED": 0.1,
    "NOT STARTED": 0.0,
}


def _parse_closes_claims(closes: str) -> list[int]:
    """Extract mission-claim numbers (1-6) from a Track's `closes` cell.

    Handles three shapes seen in PRODUCT_ROADMAP.md:
      "mission claim 4"                        → [4]
      "mission claim 5 (Windows leg)"          → [5]
      "mission claims 1, 2, 3, 6"              → [1, 2, 3, 6]
      "mission claims 1-6 backend completeness"→ [1, 2, 3, 4, 5, 6]
    Cells that don't mention claims (e.g. "design brief for B/C/D/E")
    return []."""
    if not closes or not re.search(r"claim", closes, re.I):
        return []
    nums: set[int] = set()
    for m in re.finditer(r"\b([1-6])\s*[–\-]\s*([1-6])\b", closes):
        a, b = int(m.group(1)), int(m.group(2))
        if a <= b:
            nums.update(range(a, b + 1))
    for m in re.finditer(r"\b([1-6])\b", closes):
        nums.add(int(m.group(1)))
    return sorted(nums)


# Letter-prefix brief filename: K1-..., E0-..., E1-a-..., G3-..., J18-..., etc.
# Captures the leading single-letter track tag.
TRACK_BRIEF_FILE_RE = re.compile(r"^([A-Z])\d+(?:-|$)")


def _discover_track_briefs() -> dict[str, list[str]]:
    """Scan streams/ for letter-prefix brief filenames and group by letter.

    Returns {letter -> [brief_id, ...]}. Letters surface in the roadmap
    panel even when PRODUCT_ROADMAP.md hasn't documented them yet — e.g.
    Track K (deployment-path-directive-v3, 5 briefs authored 2026-05-12)
    appears as a DRAFT card before its row lands in the roadmap table.
    """
    if not STREAMS_DIR.is_dir():
        return {}
    by_letter: dict[str, list[str]] = defaultdict(list)
    for p in sorted(STREAMS_DIR.iterdir()):
        if not p.is_file() or p.suffix != ".md":
            continue
        m = TRACK_BRIEF_FILE_RE.match(p.name)
        if not m:
            continue
        by_letter[m.group(1)].append(p.stem)
    return dict(by_letter)


def parse_product_roadmap() -> dict:
    """Return {tracks, pct, by_status, available}.

    Tracks come from the "## The seven tracks" table at the top of
    PRODUCT_ROADMAP.md. Returns available=False if the file is missing
    (old worktrees) so the renderer hides the panel cleanly.

    Discovered letters in streams/ that are NOT in the roadmap table
    are merged in as DRAFT entries so newly-authored tracks (e.g. K
    from deployment-path-directive-v3) surface immediately rather than
    blocking on PRODUCT_ROADMAP.md updates. Every track card carries
    a `brief_count` derived from the streams/ scan.
    """
    discovered = _discover_track_briefs()

    if not ROADMAP.is_file():
        # Roadmap missing (old worktrees) — but if streams/ has letter-
        # prefix briefs, surface them as DRAFT so the panel isn't empty.
        if not discovered:
            return {"tracks": [], "pct": 0.0, "by_status": {}, "available": False, "tracks_by_claim": {}}
        tracks = _draft_tracks_from_letters(sorted(discovered.keys()), discovered)
        return {
            "tracks": tracks,
            "pct": 0.0,
            "by_status": {"DRAFT": len(tracks)},
            "available": True,
            "tracks_by_claim": {},
        }

    text = ROADMAP.read_text(encoding="utf-8")
    tracks: list[dict] = []
    in_section = False
    for line in text.splitlines():
        if line.startswith("## "):
            in_section = "seven tracks" in line.lower()
            continue
        if not in_section:
            continue
        m = ROADMAP_ROW_RE.match(line)
        if not m:
            continue
        letter, scope, who, status, status_tail, closes = m.groups()
        closes_clean = closes.strip()
        tracks.append({
            "letter": letter,
            "scope": scope.strip(),
            "who": who.strip(),
            "status": status.strip(),
            "note": status_tail.strip().lstrip("—").strip(),
            "closes": closes_clean,
            "closes_claims": _parse_closes_claims(closes_clean),
        })

    # Stamp brief_count on each documented track.
    for t in tracks:
        t["brief_count"] = len(discovered.get(t["letter"], []))

    # Merge discovered-but-undocumented letters as DRAFT entries.
    documented = {t["letter"] for t in tracks}
    undocumented = [l for l in sorted(discovered.keys()) if l not in documented]
    tracks.extend(_draft_tracks_from_letters(undocumented, discovered))

    by_status: dict[str, int] = defaultdict(int)
    wn, wd = 0.0, 0
    for t in tracks:
        by_status[t["status"]] += 1
        w = ROADMAP_STATUS_WEIGHT.get(t["status"])
        if w is None:
            continue
        wn += w
        wd += 1
    pct = (wn / wd * 100.0) if wd else 0.0
    # Inverted map: claim_number → [track_letter, ...] for the Mission-claim
    # row badge. DRAFT tracks contribute too if they ever pick up a closes value.
    tracks_by_claim: dict[int, list[str]] = defaultdict(list)
    for t in tracks:
        for cid in t.get("closes_claims", []) or []:
            tracks_by_claim[cid].append(t["letter"])
    return {
        "tracks": tracks,
        "pct": pct,
        "by_status": dict(by_status),
        "available": True,
        "tracks_by_claim": {str(k): v for k, v in sorted(tracks_by_claim.items())},
    }


def _draft_tracks_from_letters(letters: list[str], discovered: dict[str, list[str]]) -> list[dict]:
    """Build DRAFT track cards for letters present in streams/ but absent
    from PRODUCT_ROADMAP.md's seven-tracks table. Scope is derived from the
    common prefix of the discovered brief ids so operators can see what the
    track is at a glance without opening any file.
    """
    out: list[dict] = []
    for letter in letters:
        brief_ids = discovered.get(letter, [])
        scope = _scope_hint_from_briefs(brief_ids) or f"Track {letter} (discovered)"
        out.append({
            "letter": letter,
            "scope": scope,
            "who": "swarm",
            "status": "DRAFT",
            "note": f"{len(brief_ids)} briefs authored under streams/; row not yet in PRODUCT_ROADMAP.md",
            "closes": "—",
            "closes_claims": [],
            "brief_count": len(brief_ids),
        })
    return out


def _scope_hint_from_briefs(brief_ids: list[str]) -> str:
    """Derive a one-line scope hint from a list of brief ids by stripping
    the leading letter-digit prefix and finding the common word prefix.
    Example: ["K1-deploy-docker-compose-stack", "K2-deploy-k8s-manifests"]
    -> "deploy". Returns "" if no useful common prefix.
    """
    if not brief_ids:
        return ""
    # Strip leading "L<digits>(-letter)?-" -> remaining slug
    stripped: list[list[str]] = []
    pref_re = re.compile(r"^[A-Z]\d+(?:-[a-z])?-")
    for bid in brief_ids:
        rest = pref_re.sub("", bid)
        parts = rest.split("-")
        if parts:
            stripped.append(parts)
    if not stripped:
        return ""
    # Find longest common word prefix.
    common: list[str] = []
    for i in range(min(len(s) for s in stripped)):
        word = stripped[0][i]
        if all(s[i] == word for s in stripped):
            common.append(word)
        else:
            break
    if not common:
        # Fall back to the first brief's first word, if any.
        return stripped[0][0] if stripped[0] else ""
    return " ".join(common)


# ─── Conductor advisories (read-only tracker) ───────────────────────────────

# Conductor advisories live under docs/build-plan/conductor-advisory-*.md.
# Each carries a `> **STATUS: <STATE> <date>**` banner near the top — usually
# ACTIONED + ISO date. The dashboard surfaces filename + status + date so
# operators can spot un-actioned advisories at a glance.
ADVISORY_STATUS_RE = re.compile(
    r"\*\*STATUS:\s*([A-Z][A-Z _-]*?)\s+(\d{4}-\d{2}-\d{2})\*\*",
)


def parse_conductor_advisories() -> list[dict]:
    """Return list of {filename, title, status, date} sorted by filename.

    Looks at the first ~40 lines of each conductor-advisory-*.md under
    docs/build-plan/ for the STATUS banner. Files missing the banner still
    appear with status="UNKNOWN" so the operator knows the advisory exists
    but hasn't been parsed."""
    advisories: list[dict] = []
    plan_dir = ROOT / "docs/build-plan"
    if not plan_dir.is_dir():
        return advisories
    for p in sorted(plan_dir.glob("conductor-advisory-*.md")):
        try:
            with p.open("r", encoding="utf-8", errors="ignore") as f:
                head = "".join(next(f, "") for _ in range(40))
        except OSError:
            continue
        title = ""
        for line in head.splitlines():
            if line.startswith("# "):
                title = line[2:].strip()
                break
        m = ADVISORY_STATUS_RE.search(head)
        if m:
            status, date = m.group(1).strip(), m.group(2)
        else:
            status, date = "UNKNOWN", ""
        advisories.append({
            "filename": p.name,
            "title": title or p.stem,
            "status": status,
            "date": date,
        })
    return advisories


# ─── Archive ledger (pre-2026Q2 merged baseline) ────────────────────────────

ARCHIVE_LEDGER = ROOT / "docs/build-plan/BUILD_STATUS_ARCHIVE_2026Q2.md"


def archive_merged_baseline() -> int:
    """Count MERGED rows in the archived ledger snapshot.

    The 2026-05-13 sweep moved 463 MERGED rows out of the live ledger into
    `BUILD_STATUS_ARCHIVE_2026Q2.md`. The burn-up curve previously summed
    MERGED counts from the live ledger only, so it cliffs at the sweep
    point. Reading the archive once and treating its MERGED count as a
    constant baseline preserves the "total briefs merged ever" semantics
    of the green line."""
    if not ARCHIVE_LEDGER.is_file():
        return 0
    try:
        text = ARCHIVE_LEDGER.read_text(encoding="utf-8")
    except OSError:
        return 0
    # Match table rows with status MERGED in the third cell, mirroring the
    # row-context-aware count used in burn_history.
    return len(re.findall(r"^\|[^|\n]+\|[^|\n]+\|\s*MERGED\s*\|", text, re.M))


# ─── Repo freshness (auto-pull staleness) ───────────────────────────────────

def repo_freshness() -> dict:
    """How current is ROOT vs origin/main? Used to surface a banner when
    auto-pull is stalling (non-main branch, ahead/behind, etc.).
    Reads only — does NOT fetch. The auto-pull loop does the fetch.
    """
    def _run(args: list[str]) -> str:
        try:
            out = subprocess.run(
                ["git", *args], cwd=ROOT, capture_output=True, text=True,
                check=False, timeout=10,
            )
            return out.stdout.strip()
        except Exception:
            return ""
    head_branch = _run(["symbolic-ref", "--short", "HEAD"]) or "<detached>"
    local_sha = _run(["rev-parse", "HEAD"])[:12]
    origin_sha = _run(["rev-parse", "origin/main"])[:12]
    counts = _run(["rev-list", "--left-right", "--count", "origin/main...HEAD"])
    behind, ahead = 0, 0
    if counts:
        parts = counts.split()
        if len(parts) == 2 and parts[0].isdigit() and parts[1].isdigit():
            behind, ahead = int(parts[0]), int(parts[1])
    last_commit_iso = _run(["log", "-1", "--format=%aI"])
    last_commit_age_hours = None
    if last_commit_iso:
        try:
            dt = datetime.fromisoformat(last_commit_iso.replace("Z", "+00:00"))
            last_commit_age_hours = round(
                (datetime.now(timezone.utc) - dt).total_seconds() / 3600.0, 2
            )
        except Exception:
            pass
    return {
        "head_branch": head_branch,
        "local_sha": local_sha,
        "origin_sha": origin_sha,
        "ahead": ahead,
        "behind": behind,
        "in_sync": (ahead == 0 and behind == 0 and head_branch == "main"),
        "last_commit_iso": last_commit_iso,
        "last_commit_age_hours": last_commit_age_hours,
    }


# ─── MISSION_BACKLOG parser ─────────────────────────────────────────────────

SECTION_RE = re.compile(r"^##\s+SECTION\s+(\d+)\s*[—–-]\s*(.+)$")
SUBSECTION_RE = re.compile(r"^###\s+(.+)$")
ITEM_START_RE = re.compile(r"^(\d+)\.\s+(.*)$")
TAG_BRIEF_RE = re.compile(r"\[BRIEF:\s*([a-z0-9][a-z0-9-]*)")
TAG_ANY_RE = re.compile(r"\[(NEW|BRIEF|SHIPPED|PARTIAL|DEFER)\b")


def parse_backlog(text: str) -> tuple[list[dict], dict[str, list[int]]]:
    """Return (items, phase_map).

    items: list of {n, section, subsection, body, tags, brief_id}
    phase_map: phase letter → list of item numbers
    """
    lines = text.splitlines()
    items: list[dict] = []

    # walk forward, track section / subsection
    section = None
    subsection = None
    in_phases = False

    cur: dict | None = None

    def flush():
        nonlocal cur
        if cur is None:
            return
        body = " ".join(cur["body_lines"]).strip()
        cur["body"] = body
        cur["tags"] = sorted(set(TAG_ANY_RE.findall(body)))
        m = TAG_BRIEF_RE.search(body)
        cur["brief_id"] = m.group(1) if m else None
        del cur["body_lines"]
        items.append(cur)
        cur = None

    for raw in lines:
        line = raw.rstrip()
        if line.startswith("## Sequencing"):
            in_phases = True
            flush()
            section = None
            subsection = None
            continue
        if in_phases:
            continue

        m = SECTION_RE.match(line)
        if m:
            flush()
            section = f"Section {m.group(1)} — {m.group(2).strip()}"
            subsection = None
            continue
        m = SUBSECTION_RE.match(line)
        if m:
            flush()
            subsection = m.group(1).strip()
            continue
        m = ITEM_START_RE.match(line)
        if m:
            flush()
            cur = {
                "n": int(m.group(1)),
                "section": section,
                "subsection": subsection,
                "body_lines": [m.group(2)],
            }
            continue
        if cur is not None and line.strip():
            cur["body_lines"].append(line.strip())

    flush()

    # parse Phase A-F sequencing
    phase_map: dict[str, list[int]] = {}
    in_phases = False
    cur_phase: str | None = None
    phase_body: list[str] = []

    def commit_phase():
        if cur_phase is None:
            return
        body = " ".join(phase_body)
        # find ranges and singletons
        nums: set[int] = set()
        for m in re.finditer(r"(\d+)\s*[–-]\s*(\d+)", body):
            a, b = int(m.group(1)), int(m.group(2))
            if 1 <= a <= 280 and 1 <= b <= 280 and b - a < 50:
                nums.update(range(a, b + 1))
        # singletons (find numbers preceded by "item"/"items" or in brackets)
        for m in re.finditer(r"[Ii]tems?\s+([\d, –-]+)", body):
            chunk = m.group(1)
            for tok in re.split(r"[,\s]+", chunk.strip()):
                if tok.isdigit():
                    n = int(tok)
                    if 1 <= n <= 280:
                        nums.add(n)
        phase_map[cur_phase] = sorted(nums)

    for raw in lines:
        line = raw.rstrip()
        if line.startswith("## Sequencing"):
            in_phases = True
            continue
        if not in_phases:
            continue
        m = re.match(r"^###\s+Phase\s+([A-F])\s*[—–-]\s*\"([^\"]+)\"", line)
        if m:
            commit_phase()
            cur_phase = m.group(1)
            phase_body = [m.group(2)]
            continue
        if cur_phase and line.strip().startswith("##"):
            commit_phase()
            cur_phase = None
            phase_body = []
            continue
        if cur_phase:
            phase_body.append(line)

    commit_phase()

    return items, phase_map


# ─── Effective status logic ─────────────────────────────────────────────────

# Priority: PARTIAL > BRIEF (look up) > SHIPPED > DEFER > NEW
# (PARTIAL beats BRIEF because some items are tagged like
#  `[PARTIAL — handoff-routing-rules #883 SHIPPED but ...]` and the
#  human meaning is "partial".)

EFFECTIVE_STATES = [
    "Done",         # SHIPPED, or BRIEF→MERGED
    "In review",    # BRIEF→IN_REVIEW
    "In flight",    # BRIEF→CLAIMED
    "Queued",       # BRIEF→TODO
    "Blocked",      # BRIEF→BLOCKED
    "Briefed",      # BRIEF tag but not in ledger
    "Partial",      # PARTIAL
    "Deferred",     # DEFER, or BRIEF→DEFERRED-v1.x
    "Not started",  # NEW or no tag
]

# weight contribution to % complete; Deferred excluded from denominator
# Partial weight is overrideable via --partial-weight CLI flag.
DEFAULT_PARTIAL_WEIGHT = 0.5
COMPLETION_WEIGHT = {
    "Done": 1.0,
    "In review": 0.7,
    "In flight": 0.4,
    "Queued": 0.1,
    "Blocked": 0.0,
    "Briefed": 0.1,
    "Partial": DEFAULT_PARTIAL_WEIGHT,
    "Deferred": None,  # excluded from denominator
    "Not started": 0.0,
}

STATE_COLOR = {
    "Done": "#059669",
    "In review": "#10b981",
    "In flight": "#2563eb",
    "Queued": "#9ca3af",
    "Blocked": "#dc2626",
    "Briefed": "#6366f1",
    "Partial": "#d97706",
    "Deferred": "#a3a3a3",
    "Not started": "#e5e7eb",
}


def effective_state(item: dict, ledger: dict[str, dict], pr_lookup: dict[str, str] | None = None) -> tuple[str, str | None, str | None]:
    """Compute effective state, plus how it was derived and any PR number.

    Returns (state, derivation, pr_number).
      derivation ∈ {"tag", "ledger", "git", "ledger+git"}
      pr_number  = the PR number if derived from git log, else None.

    The git-log lookup wins over a stale ledger: if BUILD_STATUS still says
    TODO/CLAIMED/IN_REVIEW/BLOCKED but a real PR with the brief id in its
    subject is in git history, count it as Done."""
    tags = item["tags"]
    body = item["body"]
    bid = item["brief_id"]
    pr_lookup = pr_lookup or {}

    if "PARTIAL" in tags:
        return ("Partial", "tag", None)
    if "DEFER" in tags:
        return ("Deferred", "tag", None)

    git_pr = pr_lookup.get(bid) if bid else None

    if bid:
        row = ledger.get(bid)
        ledger_status = row["status"] if row else None

        # PR-shipped wins over any non-MERGED ledger state
        if git_pr:
            if ledger_status == "MERGED":
                return ("Done", "ledger+git", git_pr)
            return ("Done", "git", git_pr)

        if ledger_status == "MERGED":
            return ("Done", "ledger", None)
        if ledger_status == "IN_REVIEW":
            return ("In review", "ledger", None)
        if ledger_status == "CLAIMED":
            return ("In flight", "ledger", None)
        if ledger_status == "TODO":
            return ("Queued", "ledger", None)
        if ledger_status == "BLOCKED":
            return ("Blocked", "ledger", None)
        if ledger_status == "DEFERRED-v1.x":
            return ("Deferred", "ledger", None)

        # brief named but not in ledger — could be inline-noted as SHIPPED
        if "SHIPPED" in body:
            return ("Done", "tag", None)
        return ("Briefed", "tag", None)

    if "SHIPPED" in tags:
        return ("Done", "tag", None)
    if "NEW" in tags:
        return ("Not started", "tag", None)
    return ("Not started", "tag", None)


def build_pr_lookup() -> dict[str, str]:
    """Scan git log; map brief_id → PR number for any brief that has clearly
    shipped. Two evidence patterns accepted:
      1. `status: <name> merged (#NNN)`         — bookkeeping commit
      2. any non-status commit subject containing <name> with `(#NNN)`
         — the real PR squash-merge commit"""
    out = subprocess.check_output(
        ["git", "log", "--all", "--pretty=format:%s"], cwd=ROOT,
    ).decode().splitlines()
    # collect (kind, brief, pr) candidates
    found: dict[str, tuple[str, str]] = {}  # brief → (kind, pr)
    # match brief ids: lowercase, dash-separated tokens
    brief_re = re.compile(r"\b([a-z][a-z0-9]*(?:-[a-z0-9]+){2,})\b")
    pr_re = re.compile(r"\(?#(\d+)\)?")
    for subj in out:
        prm = pr_re.search(subj)
        if not prm:
            continue
        pr = prm.group(1)
        is_status = subj.startswith("status:")
        # pattern 1: status: <brief> merged
        if is_status and "merged" in subj.lower():
            for m in brief_re.finditer(subj):
                bid = m.group(1)
                # prefer "PR" evidence over "status" evidence on later iter
                if bid not in found:
                    found[bid] = ("status", pr)
        # pattern 2: real PR — non-status, brief id appears in subject
        elif not is_status:
            for m in brief_re.finditer(subj):
                bid = m.group(1)
                found[bid] = ("PR", pr)  # PR evidence always wins
    return {bid: pr for bid, (_, pr) in found.items()}


# ─── Mission claim mapping ──────────────────────────────────────────────────

# The backlog uses subsection headers like "Claim 1: Real teammate" inside
# Section 3. Map those to canonical claim labels from MISSION.md.
CLAIM_LABELS = {
    1: "Teammate, not tool",
    2: "Delegation is the unit",
    3: "Platform builds workers (Genesis)",
    4: "Skills shared and learned",
    5: "Real-world execution (any PC task)",
    6: "Multi-channel approval gates",
}


def claim_for_item(item: dict) -> int | None:
    sub = item["subsection"] or ""
    m = re.match(r"Claim\s+(\d)", sub)
    if m:
        return int(m.group(1))
    return None


# ─── Git log readers ────────────────────────────────────────────────────────

def claim_timestamps_from_git(max_lookback: int = 5000) -> dict[str, datetime]:
    """Map brief_id → datetime of its most recent `status: claim <brief-id>`
    commit. Used by stale-CLAIM detection. Looks back up to `max_lookback`
    commits — comfortably more than a swarm's history depth on any active day.
    """
    out = subprocess.check_output(
        ["git", "log", f"-{max_lookback}", "--pretty=format:%aI\x01%s"],
        cwd=ROOT,
    ).decode()
    claim_re = re.compile(r"^status:\s+claim\s+([a-z][a-z0-9-]+)\b", re.I)
    last_claim: dict[str, datetime] = {}
    for line in out.splitlines():
        try:
            ts, subj = line.split("\x01", 1)
        except ValueError:
            continue
        m = claim_re.match(subj)
        if not m:
            continue
        bid = m.group(1)
        # newest commits come first; only set on first sighting
        if bid not in last_claim:
            last_claim[bid] = datetime.fromisoformat(ts.replace("Z", "+00:00"))
    return last_claim


def stale_claims(ledger: dict[str, dict], threshold_hours: float = 24.0) -> list[dict]:
    """List of {brief, owner, phase, claimed_at, age_hours} for rows with
    status==CLAIMED whose `status: claim` commit is older than threshold."""
    claim_times = claim_timestamps_from_git()
    now = datetime.now(timezone.utc)
    stale: list[dict] = []
    for brief, row in ledger.items():
        if row.get("status") != "CLAIMED":
            continue
        ts = claim_times.get(brief)
        if ts is None:
            # No claim commit found — treat as unknown rather than stale.
            continue
        age = (now - ts).total_seconds() / 3600.0
        if age >= threshold_hours:
            stale.append({
                "brief": brief,
                "owner": row.get("owner", ""),
                "phase": row.get("phase"),
                "claimed_at": ts.isoformat(timespec="minutes"),
                "age_hours": round(age, 1),
            })
    stale.sort(key=lambda r: -r["age_hours"])
    return stale


def owner_velocity(burn: list[dict], ledger: dict[str, dict]) -> dict:
    """Per-owner snapshot from the live ledger plus 24h merge counts from
    git log. Ledger is the authoritative source for current load; git log is
    the only way to attribute *recent* merges to owners.
    """
    # Current load by owner from ledger
    by_owner: dict[str, dict] = {}
    for row in ledger.values():
        owner = (row.get("owner") or "").strip().lower() or "?"
        b = by_owner.setdefault(owner, {"merged": 0, "in_review": 0, "claimed": 0, "todo": 0, "blocked": 0, "merged_24h": 0})
        st = row.get("status", "")
        if st == "MERGED": b["merged"] += 1
        elif st == "IN_REVIEW": b["in_review"] += 1
        elif st == "CLAIMED": b["claimed"] += 1
        elif st == "TODO": b["todo"] += 1
        elif st == "BLOCKED": b["blocked"] += 1

    # 24h merges by owner — scan git log for `status: <id> merged` lines
    cutoff = datetime.now(timezone.utc) - timedelta(hours=24)
    out = subprocess.check_output(
        ["git", "log", "-2000", "--pretty=format:%aI\x01%s"],
        cwd=ROOT,
    ).decode()
    merge_re = re.compile(r"^status:\s+([a-z][a-z0-9-]+)\s+merged\b", re.I)
    for line in out.splitlines():
        try:
            ts_str, subj = line.split("\x01", 1)
        except ValueError:
            continue
        ts = datetime.fromisoformat(ts_str)
        if ts < cutoff:
            break  # commits are newest-first; older than 24h → stop
        m = merge_re.match(subj)
        if not m:
            continue
        bid = m.group(1)
        owner = (ledger.get(bid, {}).get("owner") or "").strip().lower()
        # Fallback: infer from brief id prefix if ledger row is gone (rare)
        if not owner:
            if bid.startswith("claude-") or bid.startswith("mission-"):
                owner = "claude"
            elif bid.startswith("codex-"):
                owner = "codex"
            else:
                owner = "?"
        b = by_owner.setdefault(owner, {"merged": 0, "in_review": 0, "claimed": 0, "todo": 0, "blocked": 0, "merged_24h": 0})
        b["merged_24h"] += 1
    # Sort owners by 24h merge count then total merged
    ordered = sorted(by_owner.items(), key=lambda kv: (-kv[1]["merged_24h"], -kv[1]["merged"]))
    return {"by_owner": dict(ordered)}


def recent_commits(n: int = 50) -> list[dict]:
    out = subprocess.check_output(
        ["git", "log", f"-{n}", "--pretty=format:%h\x01%ai\x01%an\x01%s"],
        cwd=ROOT,
    ).decode()
    rows = []
    for line in out.splitlines():
        h, ad, an, sub = line.split("\x01", 3)
        # detect PR number in subject
        prm = re.search(r"#(\d+)", sub)
        rows.append({
            "sha": h,
            "date": ad,
            "author": an,
            "subject": sub,
            "pr": int(prm.group(1)) if prm else None,
            "is_merge": sub.startswith("status:") and "merged" in sub.lower(),
        })
    return rows


def velocity_stats(burn: list[dict]) -> dict:
    """From the burn history, compute MERGED items/hour over wall-clock
    windows ending NOW (not at the last commit). If the swarm has been
    idle, recent-window rates drop toward zero — which is the truth."""
    if len(burn) < 2:
        return {"v_1h": 0.0, "v_6h": 0.0, "v_24h": 0.0, "v_overall": 0.0,
                "hourly_bins": [], "span_hours": 0.0,
                "idle_hours": 0.0, "merged_now": burn[-1]["merged"] if burn else 0}

    def parse_iso(s: str) -> datetime:
        # Python 3.10's fromisoformat doesn't accept the trailing-Z UTC marker;
        # older BUILD_STATUS commits use that form.
        return datetime.fromisoformat(s.replace("Z", "+00:00"))

    pts = [(parse_iso(p["ts"]), p["merged"]) for p in burn]
    now = datetime.now(timezone.utc)
    now_ts = now.timestamp()
    latest_ts, latest_m = pts[-1]
    idle_hours = max(0.0, (now_ts - latest_ts.timestamp()) / 3600.0)

    def merges_in_window(window_hours: float) -> float:
        """Sum of MERGED-count deltas that fall in [now - window, now].
        Splits each segment proportionally if it straddles the window edge."""
        cutoff = now_ts - window_hours * 3600
        total = 0.0
        for i in range(1, len(pts)):
            t_prev, m_prev = pts[i - 1]
            t, m = pts[i]
            delta = m - m_prev
            if delta <= 0:
                continue
            seg_start = max(t_prev.timestamp(), cutoff)
            seg_end = min(t.timestamp(), now_ts)
            if seg_end <= seg_start:
                continue
            seg_len = t.timestamp() - t_prev.timestamp()
            if seg_len <= 0:
                total += delta
            else:
                total += delta * (seg_end - seg_start) / seg_len
        return total

    def rate_over(window_hours: float) -> float:
        # Numerator: actual merges in the wall-clock window.
        # Denominator: window size, but never larger than data span (else
        # we'd undercount when the data is shorter than the window).
        span_hours = (now_ts - pts[0][0].timestamp()) / 3600.0
        denom = min(window_hours, max(span_hours, 0.01))
        return merges_in_window(window_hours) / denom

    span_hours = (now_ts - pts[0][0].timestamp()) / 3600.0

    # hourly bins: hours-ago-from-NOW, last 48
    bins: dict[int, float] = defaultdict(float)
    for i in range(1, len(pts)):
        t_prev, m_prev = pts[i - 1]
        t, m = pts[i]
        delta = m - m_prev
        if delta <= 0:
            continue
        hours_ago = int((now_ts - t.timestamp()) / 3600.0)
        if 0 <= hours_ago < 48:
            bins[hours_ago] += delta
    hourly = []
    max_hours_back = min(48, int(span_hours) + 1)
    for h in range(max_hours_back, -1, -1):
        hourly.append({"hours_ago": h, "count": int(round(bins.get(h, 0)))})

    return {
        "v_1h": round(rate_over(1), 2),
        "v_6h": round(rate_over(6), 2),
        "v_24h": round(rate_over(24), 2),
        "v_overall": round(rate_over(max(0.1, span_hours)), 2),
        "hourly_bins": hourly,
        "span_hours": round(span_hours, 2),
        "idle_hours": round(idle_hours, 2),
        "merged_now": latest_m,
    }


def project_etas(stats: dict, velocity: dict) -> dict:
    """For each phase + total backlog, project hours-until-complete at the
    24h velocity. Phase ETAs come in two flavors: 'focused' (just this phase)
    and 'cumulative' (this phase plus all earlier phases that aren't done)."""
    rate = velocity.get("v_24h", 0.0) or velocity.get("v_overall", 0.0)
    out = {"rate_used": rate, "rate_window": "24h" if velocity.get("v_24h") else "overall"}

    # per-phase remaining (strict: counts only Done as complete)
    phase_order = ["A", "B", "C", "D", "E", "F"]
    cumulative_remaining = 0
    phases_eta = {}
    for p in phase_order:
        info = stats["by_phase"].get(p)
        if not info or not info["n"]:
            continue
        c = info["counts"]
        done_strict = c.get("Done", 0)
        partial = c.get("Partial", 0)
        deferred = c.get("Deferred", 0)
        remaining_strict = info["n"] - done_strict - deferred
        remaining_effective = remaining_strict - 0.5 * partial  # partial counted as half-done
        cumulative_remaining += remaining_strict
        phases_eta[p] = {
            "remaining": remaining_strict,
            "remaining_effective": round(remaining_effective, 1),
            "eta_focused_hours": (remaining_strict / rate) if rate > 0 else None,
            "eta_cumulative_hours": (cumulative_remaining / rate) if rate > 0 else None,
        }
    out["phases"] = phases_eta

    # ledger drain ETA (TODO+CLAIMED+IN_REVIEW)
    lc = stats["ledger_counts"]
    drain = lc.get("TODO", 0) + lc.get("CLAIMED", 0) + lc.get("IN_REVIEW", 0)
    out["ledger_drain"] = {
        "items": drain,
        "eta_hours": (drain / rate) if rate > 0 else None,
    }

    # backlog total ETA (sum of all not-done items)
    total_remaining = sum(1 for s, n in stats["by_state"].items() if s != "Done" and s != "Deferred" for _ in range(n))
    out["total_backlog"] = {
        "items": total_remaining,
        "eta_hours": (total_remaining / rate) if rate > 0 else None,
    }
    return out


def burn_history(target_points: int = 40, archive_baseline: int = 0) -> list[dict]:
    """Sample BUILD_STATUS.md commits (newest→oldest), down to ~target_points,
    and count statuses at each. Uses full ISO datetime so multi-commit days
    still produce a curve.

    `archive_baseline` is added to every sample's `merged` count. After the
    2026-05-13 archive sweep, the live ledger no longer contains 463 MERGED
    rows that previously sat in it; passing the archive's MERGED count keeps
    the green burn-up line continuous ("total briefs merged ever") instead
    of cliffing at the sweep commit."""
    out = subprocess.check_output(
        ["git", "log",
         "--pretty=format:%H|%aI",
         "--", "docs/build-plan/BUILD_STATUS.md"],
        cwd=ROOT,
    ).decode()
    rows = [tuple(line.split("|", 1)) for line in out.splitlines() if "|" in line]
    rows.reverse()  # oldest first
    if len(rows) > target_points:
        step = max(1, len(rows) // target_points)
        # always keep first + last
        sampled = rows[::step]
        if rows[-1] not in sampled:
            sampled.append(rows[-1])
        rows = sampled

    history = []
    for sha, ts in rows:
        try:
            blob = subprocess.check_output(
                ["git", "show", f"{sha}:docs/build-plan/BUILD_STATUS.md"],
                cwd=ROOT, stderr=subprocess.DEVNULL,
            ).decode("utf-8", errors="replace")
        except subprocess.CalledProcessError:
            continue
        # Count by table-row context, not bare string occurrences. Narrative
        # cells often mention OTHER PRs as MERGED/BLOCKED/etc, which inflates
        # raw counts ~50% above the true row count and skews run-rate + ETAs.
        # Match: `|<brief>|<owner>|<STATUS>|` — STATUS is the 3rd column.
        def _row_status_count(label: str) -> int:
            return len(re.findall(rf"^\|[^|\n]+\|[^|\n]+\|\s*{re.escape(label)}\s*\|", blob, re.M))
        merged = _row_status_count("MERGED")
        in_review = _row_status_count("IN_REVIEW")
        blocked = _row_status_count("BLOCKED")
        claimed = _row_status_count("CLAIMED")
        todo = _row_status_count("TODO")
        history.append({
            "ts": ts,
            "date": ts[:16].replace("T", " "),  # YYYY-MM-DD HH:MM
            "merged": merged + archive_baseline,
            "in_review": in_review,
            "blocked": blocked,
            "claimed": claimed,
            "todo": todo,
        })
    return history


# ─── Stats ──────────────────────────────────────────────────────────────────

def compute_stats(items: list[dict], ledger: dict[str, dict], phase_map: dict[str, list[int]]):
    by_state: dict[str, int] = defaultdict(int)
    for it in items:
        by_state[it["state"]] += 1

    # overall % complete (excluding Deferred)
    num = 0.0
    den = 0
    for it in items:
        w = COMPLETION_WEIGHT[it["state"]]
        if w is None:
            continue
        num += w
        den += 1
    pct = (num / den * 100.0) if den else 0.0

    # by section
    by_section: dict[str, dict] = {}
    for it in items:
        sec = it["section"] or "Unsectioned"
        b = by_section.setdefault(sec, {"counts": defaultdict(int), "n": 0, "weight_num": 0.0, "weight_den": 0})
        b["counts"][it["state"]] += 1
        b["n"] += 1
        w = COMPLETION_WEIGHT[it["state"]]
        if w is not None:
            b["weight_num"] += w
            b["weight_den"] += 1
    for sec, b in by_section.items():
        b["pct"] = (b["weight_num"] / b["weight_den"] * 100.0) if b["weight_den"] else 0.0
        b["counts"] = dict(b["counts"])

    # by phase
    by_phase: dict[str, dict] = {}
    for phase, nums in phase_map.items():
        included = [it for it in items if it["n"] in set(nums)]
        counts: dict[str, int] = defaultdict(int)
        wn, wd = 0.0, 0
        for it in included:
            counts[it["state"]] += 1
            w = COMPLETION_WEIGHT[it["state"]]
            if w is not None:
                wn += w; wd += 1
        by_phase[phase] = {
            "counts": dict(counts),
            "n": len(included),
            "pct": (wn / wd * 100.0) if wd else 0.0,
            "items": nums,
        }

    # by claim
    by_claim: dict[int, dict] = {}
    for cid in range(1, 7):
        by_claim[cid] = {"label": CLAIM_LABELS[cid], "counts": defaultdict(int), "n": 0, "wn": 0.0, "wd": 0, "items": []}
    for it in items:
        cid = claim_for_item(it)
        if cid is None:
            continue
        c = by_claim[cid]
        c["counts"][it["state"]] += 1
        c["n"] += 1
        w = COMPLETION_WEIGHT[it["state"]]
        if w is not None:
            c["wn"] += w; c["wd"] += 1
        c["items"].append(it["n"])
    for cid, c in by_claim.items():
        c["pct"] = (c["wn"] / c["wd"] * 100.0) if c["wd"] else 0.0
        c["counts"] = dict(c["counts"])
        del c["wn"]; del c["wd"]

    # ledger breakdown
    ledger_counts: dict[str, int] = defaultdict(int)
    for row in ledger.values():
        ledger_counts[row["status"]] += 1
    ledger_counts = dict(ledger_counts)

    return {
        "by_state": dict(by_state),
        "pct_complete": pct,
        "total": len(items),
        "deferred": by_state.get("Deferred", 0),
        "by_section": by_section,
        "by_phase": by_phase,
        "by_claim": by_claim,
        "ledger_counts": ledger_counts,
        "ledger_total": len(ledger),
    }


# ─── HTML render ────────────────────────────────────────────────────────────

def render_html(payload: dict) -> str:
    data_json = json.dumps(payload, indent=2, default=str)
    return HTML_TEMPLATE.replace("__DATA_JSON__", data_json)


HTML_TEMPLATE = r"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>SKYCOT mission dashboard</title>
<style>
  :root {
    --bg: #0f1115;
    --panel: #161a22;
    --panel-2: #1c2230;
    --hairline: #262d3d;
    --hairline-2: #323b51;
    --ink: #e6e9ef;
    --ink-2: #b8bfcc;
    --ink-3: #7a8294;
    --mute: #525a6b;
    --accent: #34d399;
    --accent-soft: rgba(52, 211, 153, 0.15);
    --warn: #f59e0b;
    --weak: #ef4444;
    --info: #60a5fa;
    --done: #059669;
    --in-review: #10b981;
    --in-flight: #2563eb;
    --queued: #9ca3af;
    --blocked: #dc2626;
    --briefed: #6366f1;
    --partial: #d97706;
    --deferred: #a3a3a3;
    --not-started: #6b7280;
  }
  * { box-sizing: border-box; }
  html, body {
    margin: 0; padding: 0; background: var(--bg); color: var(--ink);
    font: 14px/1.5 -apple-system, "Inter Tight", "Segoe UI", system-ui, sans-serif;
    letter-spacing: -0.005em;
  }
  .mono { font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace; font-size: 0.92em; }
  .wrap { max-width: 1400px; margin: 0 auto; padding: 24px 28px 64px; }

  header { display: flex; align-items: flex-end; justify-content: space-between; gap: 24px; padding-bottom: 16px; border-bottom: 1px solid var(--hairline); margin-bottom: 24px; }
  header .title { font-size: 22px; font-weight: 600; letter-spacing: -0.02em; color: var(--ink); }
  header .sub { color: var(--ink-3); font-size: 12px; margin-top: 4px; }
  header .right { display: flex; flex-direction: column; align-items: flex-end; gap: 6px; }
  header .when { color: var(--ink-3); font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; }
  header button {
    background: var(--accent-soft); color: var(--accent); border: 1px solid var(--accent);
    padding: 6px 14px; border-radius: 6px; font: inherit; font-size: 12px; cursor: pointer;
    transition: background 0.15s;
  }
  header button:hover { background: rgba(52, 211, 153, 0.25); }
  header button.copied { background: var(--accent); color: var(--bg); }

  .cards { display: grid; grid-template-columns: repeat(5, 1fr); gap: 12px; margin-bottom: 24px; }
  .card { background: var(--panel); border: 1px solid var(--hairline); border-radius: 10px; padding: 16px; }
  .card .label { color: var(--ink-3); text-transform: uppercase; letter-spacing: 0.08em; font-size: 10px; }
  .card .value { font-size: 32px; font-weight: 600; color: var(--ink); margin-top: 6px; letter-spacing: -0.02em; }
  .card .delta { color: var(--ink-3); font-size: 11px; margin-top: 4px; }
  .card.accent .value { color: var(--accent); }
  .card.warn .value { color: var(--warn); }
  .card.weak .value { color: var(--weak); }

  .grid { display: grid; gap: 16px; margin-bottom: 16px; }
  .grid.two { grid-template-columns: 1.4fr 1fr; }
  .grid.three { grid-template-columns: repeat(3, 1fr); }
  .panel { background: var(--panel); border: 1px solid var(--hairline); border-radius: 10px; padding: 18px; }
  .panel h2 { font-size: 13px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: var(--ink-2); margin: 0 0 14px; }
  .panel h2 .right { float: right; color: var(--ink-3); text-transform: none; letter-spacing: 0; font-weight: 400; font-size: 12px; }

  /* progress bars */
  .progress-row { display: grid; grid-template-columns: 220px 1fr 64px; gap: 12px; align-items: center; margin-bottom: 8px; }
  .progress-row .pname { color: var(--ink-2); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .progress-row .pname small { color: var(--ink-3); font-size: 10px; margin-left: 4px; }
  .progress-row .pbar { height: 18px; background: var(--panel-2); border-radius: 4px; overflow: hidden; display: flex; }
  .progress-row .pbar > span { display: block; height: 100%; transition: width 0.2s; }
  .progress-row .ppct { font-size: 12px; color: var(--ink-2); text-align: right; font-variant-numeric: tabular-nums; }

  /* legend */
  .legend { display: flex; flex-wrap: wrap; gap: 12px; font-size: 11px; color: var(--ink-3); margin-top: 8px; }
  .legend .sw { display: inline-block; width: 10px; height: 10px; border-radius: 2px; vertical-align: middle; margin-right: 4px; }

  /* donut */
  .donut-wrap { display: flex; align-items: center; gap: 24px; }
  .donut-wrap svg { flex: 0 0 180px; }
  .donut-wrap ul { list-style: none; padding: 0; margin: 0; flex: 1; }
  .donut-wrap li { display: flex; align-items: center; justify-content: space-between; padding: 5px 0; font-size: 12px; color: var(--ink-2); border-bottom: 1px solid var(--hairline); }
  .donut-wrap li:last-child { border: none; }
  .donut-wrap li .lab { display: flex; align-items: center; gap: 10px; }
  .donut-wrap li .lab .sw { display: inline-block; width: 14px; height: 14px; border-radius: 3px; border: 1px solid rgba(255,255,255,0.08); flex-shrink: 0; }
  .donut-wrap li .v { color: var(--ink); font-variant-numeric: tabular-nums; }

  /* burn-up */
  .burn { width: 100%; height: 240px; }
  .burn .axis { stroke: var(--hairline); stroke-width: 1; }
  .burn .grid-line { stroke: var(--hairline); stroke-dasharray: 2 4; stroke-width: 1; }
  .burn .label { fill: var(--ink-3); font-size: 10px; font-family: ui-monospace, monospace; }
  .burn path.merged { fill: none; stroke: var(--accent); stroke-width: 2; }
  .burn path.merged-fill { fill: var(--accent-soft); stroke: none; }
  .burn path.in-review { fill: none; stroke: var(--info); stroke-width: 1.5; stroke-dasharray: 4 3; }
  .burn path.blocked { fill: none; stroke: var(--weak); stroke-width: 1.5; stroke-dasharray: 4 3; }

  /* claim cards */
  .claim { background: var(--panel-2); border: 1px solid var(--hairline-2); border-radius: 8px; padding: 14px; }
  .claim .ch { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: 8px; }
  .claim .ch .num { color: var(--ink-3); font-size: 11px; font-family: ui-monospace, monospace; }
  .claim .ch .name { color: var(--ink); font-weight: 600; font-size: 13px; }
  .claim .ch .pct { color: var(--accent); font-weight: 600; font-size: 14px; font-variant-numeric: tabular-nums; }
  .claim .stripe { height: 8px; background: var(--bg); border-radius: 3px; overflow: hidden; display: flex; margin-bottom: 8px; }
  .claim .stripe > span { display: block; height: 100%; }
  .claim .meta { font-size: 11px; color: var(--ink-3); }

  /* phase cards (hero) */
  .phase-grid { grid-template-columns: repeat(6, 1fr); gap: 10px; }
  @media (max-width: 1100px) { .phase-grid { grid-template-columns: repeat(3, 1fr); } }
  @media (max-width: 700px) { .phase-grid { grid-template-columns: repeat(2, 1fr); } }
  .phase-card { background: var(--panel-2); border: 1px solid var(--hairline-2); border-radius: 8px; padding: 14px; position: relative; overflow: hidden; cursor: pointer; transition: border-color 0.15s, transform 0.15s; }
  .phase-card:hover { border-color: var(--accent); transform: translateY(-1px); }
  .phase-card.active { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .phase-card .ph-letter { font-size: 28px; font-weight: 700; line-height: 1; color: var(--ink); letter-spacing: -0.02em; font-family: ui-monospace, monospace; }
  .phase-card .ph-title { color: var(--ink-2); font-size: 11px; margin-top: 6px; line-height: 1.3; min-height: 2.6em; font-weight: 500; }
  .phase-card .ph-pct { color: var(--accent); font-weight: 600; font-size: 22px; margin-top: 10px; font-variant-numeric: tabular-nums; }
  .phase-card .ph-stripe { height: 6px; background: var(--bg); border-radius: 3px; overflow: hidden; display: flex; margin-top: 8px; }
  .phase-card .ph-stripe > span { display: block; height: 100%; }
  .phase-card .ph-meta { color: var(--ink-3); font-size: 10px; margin-top: 8px; line-height: 1.4; font-family: ui-monospace, monospace; }
  .phase-card .ph-eta { color: var(--ink-3); font-size: 10px; text-transform: uppercase; letter-spacing: 0.06em; margin-top: 4px; }

  /* compact claims list */
  #claims-compact .progress-row .pname { font-size: 11px; }
  #claims-compact .progress-row .pname small { display: block; color: var(--ink-3); margin-left: 0; margin-top: 1px; font-size: 10px; }
  #claims-compact .progress-row .pname .claim-tracks {
    display: inline-block; margin-left: 8px; padding: 1px 7px; border-radius: 9px;
    background: rgba(99, 102, 241, 0.15); color: var(--briefed); border: 1px solid rgba(99,102,241,0.4);
    font-size: 10px; font-weight: 600; letter-spacing: 0.02em;
    vertical-align: middle; font-family: ui-monospace, monospace;
  }

  /* conductor advisory tracker panel */
  .adv-list { list-style: none; padding: 0; margin: 0; font-family: ui-monospace, monospace; font-size: 11px; }
  .adv-list li { display: grid; grid-template-columns: 1fr auto auto; gap: 12px; padding: 7px 8px; border-bottom: 1px solid var(--hairline); align-items: center; border-radius: 4px; }
  .adv-list li:hover { background: rgba(255,255,255,0.03); }
  .adv-list li:last-child { border-bottom: none; }
  .adv-list .adv-file { color: var(--ink); }
  .adv-list .adv-file small { display: block; color: var(--ink-3); font-size: 10px; margin-top: 1px; font-family: inherit; }
  .adv-list .adv-status { padding: 2px 8px; border-radius: 10px; font-size: 10px; font-weight: 600; letter-spacing: 0.04em; white-space: nowrap; }
  .adv-list .adv-date { color: var(--ink-3); font-size: 10px; min-width: 72px; text-align: right; }

  /* run rate */
  .rate-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; margin-bottom: 14px; }
  .rate-cell { background: var(--panel-2); border: 1px solid var(--hairline-2); border-radius: 6px; padding: 10px 12px; }
  .rate-cell .lab { color: var(--ink-3); font-size: 10px; text-transform: uppercase; letter-spacing: 0.06em; }
  .rate-cell .v { color: var(--ink); font-size: 18px; font-weight: 600; margin-top: 4px; font-variant-numeric: tabular-nums; }
  .rate-cell .v small { color: var(--ink-3); font-size: 10px; font-weight: 400; margin-left: 3px; }
  .rate-cell.hot .v { color: var(--accent); }
  .hourly { width: 100%; height: 110px; }
  .hourly rect { fill: var(--accent); }
  .hourly rect.idle { fill: var(--hairline-2); }
  .hourly text { fill: var(--ink-3); font-size: 9px; font-family: ui-monospace, monospace; }

  /* ETA list */
  .eta-row { display: grid; grid-template-columns: 26px 1fr auto auto; gap: 10px; padding: 8px 0; align-items: center; border-bottom: 1px solid var(--hairline); }
  .eta-row:last-child { border: none; }
  .eta-row .pl { font-family: ui-monospace, monospace; font-size: 14px; font-weight: 600; color: var(--ink); text-align: center; }
  .eta-row .ph-name { color: var(--ink-2); font-size: 12px; }
  .eta-row .ph-name small { color: var(--ink-3); display: block; font-size: 10px; }
  .eta-row .focused { color: var(--accent); font-size: 13px; font-weight: 600; font-variant-numeric: tabular-nums; min-width: 80px; text-align: right; }
  .eta-row .cumul { color: var(--ink-3); font-size: 11px; font-variant-numeric: tabular-nums; min-width: 90px; text-align: right; }
  .eta-row.summary { border-top: 1px solid var(--hairline-2); margin-top: 6px; padding-top: 12px; border-bottom: none; }
  .eta-row.summary .pl { color: var(--ink-3); font-size: 11px; text-transform: uppercase; }
  .eta-row.summary .focused { color: var(--warn); }
  .eta-row.eta-complete .pl { color: var(--accent); }
  .eta-row.eta-complete .ph-name { color: var(--ink-3); }
  .eta-row.eta-complete .focused { color: var(--accent); }
  .eta-row.no-rate { color: var(--ink-3); font-style: italic; }

  /* phase card extra eta line */
  .phase-card .ph-eta-proj { color: var(--accent); font-size: 11px; margin-top: 6px; font-family: ui-monospace, monospace; }
  .phase-card.phase-blocked { border-color: var(--weak); }
  .phase-card.phase-blocked::before { content: "▲"; position: absolute; top: 8px; right: 10px; color: var(--weak); font-size: 11px; }
  /* phase detail drawer */
  .phase-drawer { display: none; margin-top: 16px; background: var(--panel-2); border: 1px solid var(--hairline-2); border-radius: 10px; padding: 16px 18px; max-height: 60vh; overflow-y: auto; }
  .phase-drawer.open { display: block; animation: drawerIn 0.18s ease-out; }
  @keyframes drawerIn { from { opacity: 0; transform: translateY(-4px); } to { opacity: 1; transform: translateY(0); } }
  .phase-drawer .dr-head { display: flex; flex-direction: column; gap: 4px; padding-bottom: 12px; border-bottom: 1px solid var(--hairline); margin-bottom: 14px; position: relative; }
  .phase-drawer .dr-title { font-size: 14px; color: var(--ink); }
  .phase-drawer .dr-sub { font-size: 11px; color: var(--ink-3); font-family: ui-monospace, monospace; }
  .phase-drawer .dr-close { position: absolute; top: 0; right: 0; background: transparent; border: 1px solid var(--hairline-2); color: var(--ink-3); width: 28px; height: 28px; border-radius: 6px; cursor: pointer; font-size: 16px; line-height: 1; }
  .phase-drawer .dr-close:hover { color: var(--ink); border-color: var(--ink-3); }
  .phase-drawer .dr-section { margin-bottom: 14px; }
  .phase-drawer .dr-section:last-child { margin-bottom: 0; }
  .phase-drawer .dr-sec-head { display: flex; align-items: center; gap: 8px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--ink-2); margin-bottom: 6px; }
  .phase-drawer .dr-sec-head .dr-sw { display: inline-block; width: 10px; height: 10px; border-radius: 2px; }
  .phase-drawer .dr-sec-head .dr-count { color: var(--ink-3); font-weight: normal; }
  .phase-drawer .dr-list { list-style: none; padding: 0; margin: 0; font-family: ui-monospace, monospace; font-size: 11px; }
  .phase-drawer .dr-list li { display: grid; grid-template-columns: 1fr auto auto; gap: 12px; padding: 4px 8px; border-radius: 4px; }
  .phase-drawer .dr-list li:hover { background: rgba(255,255,255,0.03); }
  .phase-drawer .dr-list .dr-brief { color: var(--ink); }
  .phase-drawer .dr-list .dr-owner { color: var(--ink-3); }
  .phase-drawer .dr-list .dr-pr { color: var(--accent); min-width: 56px; text-align: right; }
  .phase-drawer .dr-empty { color: var(--ink-3); font-size: 11px; padding: 8px 0; }
  /* per-owner velocity panel */
  .ov-row { display: grid; grid-template-columns: 80px 1fr 70px 1fr; gap: 12px; padding: 8px 0; align-items: center; border-bottom: 1px solid var(--hairline); font-size: 11px; }
  .ov-row:last-child { border-bottom: none; }
  .ov-row .ov-name { color: var(--ink); font-weight: 600; font-family: ui-monospace, monospace; }
  .ov-row .ov-bar { height: 8px; background: var(--bg); border-radius: 4px; overflow: hidden; }
  .ov-row .ov-bar > span { display: block; height: 100%; }
  .ov-row .ov-num { color: var(--accent); font-variant-numeric: tabular-nums; font-size: 14px; font-weight: 600; text-align: right; }
  .ov-row .ov-num small { color: var(--ink-3); font-size: 9px; font-weight: normal; margin-left: 2px; }
  .ov-row .ov-meta { color: var(--ink-3); font-family: ui-monospace, monospace; font-size: 10px; }
  /* stale claims panel */
  .sc-list { list-style: none; padding: 0; margin: 0; font-family: ui-monospace, monospace; font-size: 11px; }
  .sc-list li { display: grid; grid-template-columns: 1fr auto auto auto; gap: 12px; padding: 5px 0; border-bottom: 1px solid var(--hairline); align-items: center; }
  .sc-list li:last-child { border-bottom: none; }
  .sc-list .sc-brief { color: var(--ink); }
  .sc-list .sc-owner { color: var(--ink-3); }
  .sc-list .sc-phase { color: var(--ink-3); font-size: 10px; }
  .sc-list .sc-age { font-weight: 600; min-width: 40px; text-align: right; }
  /* Tier M v1.0 ship-gate panel */
  .tm-list { list-style: none; padding: 0; margin: 0; font-family: ui-monospace, monospace; font-size: 11px; }
  .tm-list li { display: grid; grid-template-columns: 100px 1fr auto auto auto; gap: 12px; padding: 7px 8px; border-bottom: 1px solid var(--hairline); align-items: center; border-radius: 4px; }
  .tm-list li:hover { background: rgba(255,255,255,0.03); }
  .tm-list li:last-child { border-bottom: none; }
  .tm-list .tm-pill { display: inline-block; padding: 2px 8px; border-radius: 10px; font-size: 9px; font-weight: 600; letter-spacing: 0.04em; white-space: nowrap; text-align: center; }
  .tm-list .tm-brief { color: var(--ink); }
  .tm-list .tm-phase { color: var(--ink-3); font-size: 10px; }
  .tm-list .tm-owner { color: var(--ink-3); font-size: 10px; min-width: 60px; text-align: right; }
  .tm-list .tm-pr { color: var(--accent); min-width: 56px; text-align: right; }
  .tm-list .tm-pr-empty { color: var(--mute); min-width: 56px; text-align: right; }
  .tm-tier-letter { display: inline-block; padding: 1px 6px; border-radius: 8px; font-size: 9px; font-weight: 600; letter-spacing: 0.04em; margin-right: 4px; }
  /* tier badge in live-ledger drill */
  #ledger-table .ltier { color: var(--ink-3); width: 30px; text-align: center; font-weight: 600; }
  /* backlog-gap panel */
  .bg-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-bottom: 14px; }
  .bg-stat { background: var(--panel-2); border: 1px solid var(--hairline-2); border-radius: 8px; padding: 14px; }
  .bg-stat .bg-num { font-size: 28px; font-weight: 700; color: var(--ink); font-variant-numeric: tabular-nums; line-height: 1; letter-spacing: -0.02em; }
  .bg-stat .bg-lab { color: var(--ink-3); font-size: 11px; margin-top: 6px; line-height: 1.3; }
  .bg-recent-head { color: var(--ink-2); font-size: 12px; margin: 4px 0 6px; }
  .bg-recent { list-style: none; padding: 0; margin: 0; font-family: ui-monospace, monospace; font-size: 11px; }
  .bg-recent li { display: flex; gap: 12px; padding: 4px 0; border-bottom: 1px solid var(--hairline); align-items: center; }
  .bg-recent li:last-child { border-bottom: none; }
  .bg-recent .bg-pr { color: var(--accent); min-width: 60px; }
  .bg-recent .bg-brief { color: var(--ink); flex: 1; }
  .bg-recent .bg-phase { color: var(--ink-3); font-size: 10px; }

  /* activity feed */
  .activity { max-height: 360px; overflow-y: auto; }
  .activity .row { display: grid; grid-template-columns: 84px 60px 1fr; gap: 10px; padding: 6px 0; font-size: 12px; border-bottom: 1px solid var(--hairline); }
  .activity .row:last-child { border: none; }
  .activity .row .d { color: var(--ink-3); font-family: ui-monospace, monospace; font-size: 11px; }
  .activity .row .h { color: var(--ink-3); font-family: ui-monospace, monospace; font-size: 11px; }
  .activity .row .s { color: var(--ink-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .activity .row .s.merge { color: var(--accent); }
  .activity .row .s a { color: var(--info); text-decoration: none; }

  /* drilldown table */
  .filters { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; align-items: center; }
  .filters input[type=text] { background: var(--panel-2); color: var(--ink); border: 1px solid var(--hairline-2); padding: 6px 10px; border-radius: 6px; font: inherit; font-size: 12px; min-width: 240px; }
  .filters select { background: var(--panel-2); color: var(--ink); border: 1px solid var(--hairline-2); padding: 6px 10px; border-radius: 6px; font: inherit; font-size: 12px; }
  .filters .count { color: var(--ink-3); font-size: 12px; margin-left: auto; }
  /* live-ledger drill */
  #ledger-table { width: 100%; border-collapse: collapse; font-size: 11px; font-family: ui-monospace, monospace; }
  #ledger-table th { text-align: left; color: var(--ink-3); font-weight: 500; font-size: 10px; text-transform: uppercase; letter-spacing: 0.06em; padding: 6px 8px; border-bottom: 1px solid var(--hairline-2); }
  #ledger-table td { padding: 5px 8px; border-bottom: 1px solid var(--hairline); }
  #ledger-table tr:hover td { background: rgba(255,255,255,0.03); }
  #ledger-table .lstate-pill { display: inline-block; padding: 2px 7px; border-radius: 10px; font-size: 9px; font-weight: 600; letter-spacing: 0.04em; white-space: nowrap; }
  #ledger-table .lphase { color: var(--ink-2); width: 50px; }
  #ledger-table .lbrief { color: var(--ink); }
  #ledger-table .lowner { color: var(--ink-3); width: 80px; }
  #ledger-table .lpr { color: var(--accent); }
  table { width: 100%; border-collapse: collapse; font-size: 12px; }
  table th { text-align: left; color: var(--ink-3); font-weight: 500; text-transform: uppercase; letter-spacing: 0.06em; font-size: 10px; padding: 8px 10px; border-bottom: 1px solid var(--hairline); position: sticky; top: 0; background: var(--panel); }
  table td { padding: 8px 10px; border-bottom: 1px solid var(--hairline); color: var(--ink-2); vertical-align: top; }
  table td.n { color: var(--ink-3); font-family: ui-monospace, monospace; font-size: 11px; width: 38px; }
  table td.body { line-height: 1.4; }
  table td.body code { background: var(--panel-2); padding: 1px 4px; border-radius: 3px; font-size: 11px; color: var(--ink-2); }
  table td.state { width: 110px; }
  table td.brief { width: 280px; font-family: ui-monospace, monospace; font-size: 11px; color: var(--ink-3); }
  table td.brief .bid { color: var(--ink-2); }
  table td.brief a { color: var(--info); text-decoration: none; }
  .pill { display: inline-block; padding: 2px 8px; border-radius: 10px; font-size: 11px; font-weight: 500; }
  .badge-pr { background: rgba(52,211,153,0.15); color: var(--accent); padding: 1px 6px; border-radius: 3px; font-size: 10px; border: 1px solid var(--accent); }
  .ledger-stale { color: var(--warn); font-size: 11px; margin-left: 8px; }

  .tablewrap { max-height: 600px; overflow-y: auto; border: 1px solid var(--hairline); border-radius: 8px; }
  details { margin-top: 8px; color: var(--ink-3); font-size: 11px; }
  details summary { cursor: pointer; }
  footer { color: var(--ink-3); font-size: 11px; text-align: center; margin-top: 32px; padding-top: 16px; border-top: 1px solid var(--hairline); }
  footer code { background: var(--panel); padding: 2px 6px; border-radius: 3px; }

  @media (max-width: 1100px) {
    .cards { grid-template-columns: repeat(2, 1fr); }
    .grid.two, .grid.three { grid-template-columns: 1fr; }
  }
</style>
</head>
<body>
<div class="wrap">

<header>
  <div>
    <div class="title">SKYCOT mission dashboard</div>
    <div class="sub">Primary source: docs/build-plan/BUILD_STATUS.md (live ledger, conductor-maintained) · MISSION_BACKLOG.md 280-item view shown as secondary lens<span id="stale-note" class="ledger-stale" style="display:none"></span></div>
    <div id="freshness-note" class="ledger-stale" style="display:none;margin-top:4px;"></div>
  </div>
  <div class="right">
    <span class="when">Generated <span id="gen-when"></span></span>
    <button id="refresh-btn" title="Refresh">Refresh</button>
    <span id="refresh-mode-note" style="color:var(--ink-3);font-size:10px;margin-top:2px;"></span>
  </div>
</header>

<div class="cards" id="cards"></div>

<div class="panel" style="margin-bottom:16px;">
  <h2>Phase progress · live ledger <span class="right">primary lens · BUILD_STATUS.md ▼ PHASE dividers (the conductor's truth) · click a phase to see its briefs</span></h2>
  <div class="grid phase-grid" id="ledger-phase-cards"></div>
  <div class="legend" id="ledger-state-legend" style="margin-top:14px;"></div>
  <div id="phase-detail-drawer" class="phase-drawer"></div>
  <div style="color:var(--ink-3);font-size:11px;margin-top:8px;">
    <span id="ledger-phase-summary"></span>
    <span style="margin-left:8px;color:var(--mute);">Phase F is a historical catchall (pre-Tick-198 v0.x foundation work + architectural splits) — high % there reflects archive, not forward progress.</span>
  </div>
</div>


<div class="panel" style="margin-bottom:16px;">
  <h2>Tier M · v1.0 ship gate <span class="right" id="tier-m-note">Mission MVP critical path · operator strategic priority 2026-05-10</span></h2>
  <div id="tier-m-roadmap"></div>
  <div id="tier-other" style="margin-top:14px;color:var(--ink-3);font-size:11px;"></div>
</div>


<div class="panel" id="roadmap-panel" style="margin-bottom:16px;display:none;">
  <h2>Product roadmap · tracks <span id="roadmap-letters">A–G</span> <span class="right" id="roadmap-note">post-MVP product-surface program · PRODUCT_ROADMAP.md (operative 2026-05-11)</span></h2>
  <div id="roadmap-tracks" class="grid" style="grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:10px;"></div>
  <div id="roadmap-legend" style="margin-top:14px;color:var(--ink-3);font-size:11px;"></div>
</div>

<div class="panel" id="advisory-panel" style="margin-bottom:16px;display:none;">
  <h2>Conductor advisories <span class="right" id="advisory-note">docs/build-plan/conductor-advisory-*.md · read-only tracker</span></h2>
  <ul class="adv-list" id="advisory-list"></ul>
</div>


<div class="grid two">
  <div class="panel">
    <h2>Run rate <span class="right" id="rate-window-note">items merged per hour, derived from BUILD_STATUS commit history</span></h2>
    <div class="rate-row" id="rate-row"></div>
    <svg class="hourly" id="hourly" viewBox="0 0 800 120" preserveAspectRatio="none"></svg>
    <div class="legend" style="margin-top:6px;">
      <span style="color:var(--ink-3)">Bars: items merged per hour, last 48h. Tall bar = busy hour. Empty = idle.</span>
    </div>
  </div>
  <div class="panel">
    <h2>ETA at current rate <span class="right" id="eta-note"></span></h2>
    <div id="eta-list"></div>
    <details>
      <summary>How is this calculated?</summary>
      ETAs assume the current 24-hour run-rate continues. "Focused" = items remaining in this phase ÷ rate (single-phase view). "Cumulative" = all earlier-phase remaining work + this phase, ÷ rate (sequenced view). Both ignore that some items aren't yet briefed and some need predecessor work.
    </details>
  </div>
</div>

<div class="grid two">
  <div class="panel">
    <h2>Burn-up <span class="right" id="burn-note">MERGED count in BUILD_STATUS.md over time</span></h2>
    <svg class="burn" id="burn" viewBox="0 0 800 240" preserveAspectRatio="none"></svg>
    <div class="legend">
      <span><span class="sw" style="background:var(--accent)"></span>Merged</span>
      <span><span class="sw" style="background:var(--info)"></span>In review</span>
      <span><span class="sw" style="background:var(--weak)"></span>Blocked</span>
      <span id="burn-archive-legend" style="color:var(--ink-3);display:none;"></span>
    </div>
  </div>
  <div class="panel">
    <h2>Live ledger state mix <span class="right">all <span id="ledger-total-h">—</span> BUILD_STATUS rows · primary lens</span></h2>
    <div class="donut-wrap">
      <svg id="donut" viewBox="0 0 200 200"></svg>
      <ul id="donut-legend"></ul>
    </div>
  </div>
</div>

<div class="panel" style="margin-bottom:16px;">
  <h2>Mission claims (six load-bearing) <span class="right">% complete weighted · scoped to the <span id="claims-scope-n">—</span> brief-tagged items in MISSION_BACKLOG only · partial signal</span></h2>
  <div id="claims-compact"></div>
</div>

<div class="grid two">
  <div class="panel">
    <h2>Per-owner velocity <span class="right">live ledger load + 24h merge count from git log</span></h2>
    <div id="owner-velocity"></div>
  </div>
  <div class="panel">
    <h2>Stale CLAIMs <span class="right">CLAIMED &gt;24h with no IN_REVIEW transition · candidates for sweep</span></h2>
    <div id="stale-claims"></div>
  </div>
</div>

<div class="panel" style="margin-bottom:16px;">
  <h2>Live work outside the 280-item backlog <span class="right">briefs the conductor is shipping that MISSION_BACKLOG never enumerated</span></h2>
  <div id="backlog-gap"></div>
</div>

<div class="panel" style="margin-bottom:16px;">
  <h2>Section progress <span class="right">9 sections of MISSION_BACKLOG.md · tertiary lens</span></h2>
  <div id="section-bars"></div>
</div>

<div class="panel" style="margin-bottom:16px;">
  <h2>Drill into ledger <span class="right">all <span id="ledger-drill-total"></span> live BUILD_STATUS rows · primary lens · search by brief id, filter by status / phase / owner</span></h2>
  <div class="filters">
    <input type="text" id="lq" placeholder="Search brief id…" />
    <select id="lstate"><option value="">All statuses</option></select>
    <select id="lphase"><option value="">All phases</option></select>
    <select id="ltier"><option value="">All tiers</option></select>
    <select id="lowner"><option value="">All owners</option></select>
    <span class="count" id="lcount"></span>
  </div>
  <div class="tablewrap">
    <table id="ledger-table">
      <thead>
        <tr><th>Status</th><th>Tier</th><th>Phase</th><th>Brief</th><th>Owner</th><th>PR</th></tr>
      </thead>
      <tbody></tbody>
    </table>
  </div>
</div>

<div class="grid two">
  <div class="panel">
    <h2>Recent activity <span class="right">last 50 commits</span></h2>
    <div class="activity" id="activity"></div>
  </div>
  <div class="panel">
    <h2>Drill into 280-item backlog <span class="right">secondary lens · MISSION_BACKLOG view</span></h2>
    <div class="filters">
      <input type="text" id="q" placeholder="Search title or brief id…" />
      <select id="fstate"><option value="">All states</option></select>
      <select id="fphase"><option value="">All phases</option></select>
      <span class="count" id="rowcount"></span>
    </div>
    <div class="tablewrap">
      <table id="items-table">
        <thead>
          <tr><th>#</th><th>State</th><th>Item</th><th>Brief</th></tr>
        </thead>
        <tbody></tbody>
      </table>
    </div>
  </div>
</div>

<footer>
  One-click refresh: <code>python3 <span id="script-path"></span> --serve --open</code><br>
  Static (one-shot): <code id="refresh-cmd">python3 <span id="script-path-2"></span></code> ·
  Sources (read-only): BUILD_STATUS.md (live ledger, primary) · MISSION_BACKLOG.md (280-item view) · git log ·
  Weights: Done 1.0 · In review 0.7 · In flight 0.4 · <span id="partial-w"></span> · Queued 0.1 · Blocked 0 · Not started 0 · Deferred excluded.
</footer>

</div>

<script>
const DATA = __DATA_JSON__;

// ── helpers ──
const $ = (s, r=document) => r.querySelector(s);
const $$ = (s, r=document) => Array.from(r.querySelectorAll(s));
const fmt = n => n.toLocaleString();
const stateOrder = ["Done","In review","In flight","Queued","Blocked","Briefed","Partial","Deferred","Not started"];
const stateColor = {
  "Done": "#059669", "In review": "#10b981", "In flight": "#2563eb",
  "Queued": "#9ca3af", "Blocked": "#dc2626", "Briefed": "#6366f1",
  "Partial": "#d97706", "Deferred": "#a3a3a3", "Not started": "#3f4655"
};
const ledgerColor = {
  "MERGED": "#059669", "IN_REVIEW": "#10b981", "CLAIMED": "#2563eb",
  "TODO": "#9ca3af", "BLOCKED": "#dc2626", "DEFERRED-v1.x": "#a3a3a3"
};

// ── shared constants (hoisted) ──
const PHASE_META = {
  A: { title: "Stop bleeding credibility", eta: "1 week" },
  B: { title: "Mission claim closure", eta: "3–4 weeks" },
  C: { title: "Production hardening", eta: "4–6 weeks" },
  D: { title: "Catch-up volume", eta: "parallel, ongoing" },
  E: { title: "Differentiate via self-improvement", eta: "2–3 months" },
  F: { title: "Scale-out", eta: "3–6 months" },
};

// ── header ──
document.getElementById("gen-when").textContent = DATA.generated_at_local + " (local)";
document.getElementById("partial-w").textContent = "Partial " + DATA.partial_weight.toFixed(2);
document.getElementById("script-path").textContent = DATA.script_abs_path;
document.getElementById("script-path-2").textContent = DATA.script_abs_path;
if (DATA.stale_ledger_count > 0) {
  const sn = document.getElementById("stale-note");
  sn.textContent = `⚠ ${DATA.stale_ledger_count} item${DATA.stale_ledger_count === 1 ? '' : 's'} done via PR but BUILD_STATUS.md still says TODO/CLAIMED — counted as Done anyway`;
  sn.style.display = "inline";
}

// Fill stale hardcoded heading counts from live data.
(() => {
  const lt = (DATA.stats && DATA.stats.ledger_total) || 0;
  const h = document.getElementById("ledger-total-h");
  if (h) h.textContent = lt;
  const cs = document.getElementById("claims-scope-n");
  if (cs) cs.textContent = DATA.backlog_brief_tagged_count || 0;
})();

// Repo-freshness banner — surfaces when auto-pull stalls (local ahead/behind
// origin/main, on a non-main branch, or last commit is very old).
(() => {
  const f = DATA.freshness;
  if (!f) return;
  const note = document.getElementById("freshness-note");
  if (!note) return;
  const parts = [];
  if (f.head_branch !== "main") {
    parts.push(`⚠ HEAD is <b>${f.head_branch}</b>, not main — auto-pull is skipping (data may lag origin/main)`);
  }
  if (f.behind > 0) {
    parts.push(`⚠ local is <b>${f.behind}</b> commit${f.behind === 1 ? '' : 's'} behind origin/main`);
  }
  if (f.ahead > 0) {
    parts.push(`local has <b>${f.ahead}</b> un-pushed commit${f.ahead === 1 ? '' : 's'} — auto-pull will refuse fast-forward`);
  }
  if (parts.length) {
    note.innerHTML = parts.join(" · ");
    note.style.display = "block";
  }
})();

// ── ETA helpers ──
function fmtHours(h) {
  if (h == null) return "—";
  if (!isFinite(h)) return "∞";
  if (h < 1) return Math.round(h * 60) + " min";
  if (h < 24) return h.toFixed(1) + " hr";
  if (h < 24 * 14) return (h / 24).toFixed(1) + " days";
  if (h < 24 * 60) return (h / 24 / 7).toFixed(1) + " weeks";
  return (h / 24 / 30).toFixed(1) + " months";
}
function etaLine(p) {
  const ph = (DATA.etas.phases || {})[p];
  if (!ph) return "";
  if (DATA.etas.rate_used <= 0 || ph.eta_focused_hours == null) {
    return `<div class="ph-eta-proj">eta: no rate yet</div>`;
  }
  return `<div class="ph-eta-proj">eta focused: ${fmtHours(ph.eta_focused_hours)}<br>eta seq: ${fmtHours(ph.eta_cumulative_hours)}</div>`;
}

const refreshBtn = document.getElementById("refresh-btn");
const refreshNote = document.getElementById("refresh-mode-note");
const isServed = location.protocol === "http:" || location.protocol === "https:";

// ── refresh state preservation ──
// Plain location.reload() blows away scroll position, open drawers, and
// every <input>/<select> filter value the user has set on the page. That
// makes the Refresh button feel destructive even though the regenerated
// page is identical in structure. We persist the user's view state to
// sessionStorage on click, restore it after the reload. Sub-50 LOC, no
// fetch/render-pipeline refactor required.
const STATE_KEY = "skycot-dashboard-state-v1";

function captureState() {
  const drawer = document.getElementById("phase-detail-drawer");
  const out = {
    scrollY: window.scrollY,
    filters: {},
    drawerPhase: drawer && drawer.classList.contains("open") ? drawer.dataset.openPhase : null,
  };
  // Every filter input/select that has an id — both 280-item drill and live-ledger drill
  ["q","fstate","fphase","lq","lstate","lphase","ltier","lowner"].forEach(id => {
    const el = document.getElementById(id);
    if (el) out.filters[id] = el.value;
  });
  return out;
}

function restoreState() {
  try {
    const raw = sessionStorage.getItem(STATE_KEY);
    if (!raw) return;
    sessionStorage.removeItem(STATE_KEY);
    const s = JSON.parse(raw);
    Object.entries(s.filters || {}).forEach(([id, val]) => {
      const el = document.getElementById(id);
      if (el && val) {
        el.value = val;
        el.dispatchEvent(new Event(el.tagName === "INPUT" ? "input" : "change"));
      }
    });
    if (s.drawerPhase) {
      const card = document.querySelector(`[data-ledger-phase="${s.drawerPhase}"]`);
      if (card) card.click();
    }
    if (typeof s.scrollY === "number") {
      // Defer scroll until after re-render settles
      window.requestAnimationFrame(() => window.scrollTo(0, s.scrollY));
    }
  } catch (e) {}
}
window.addEventListener("load", restoreState);

if (isServed) {
  refreshBtn.textContent = "Refresh";
  refreshNote.textContent = "served · click to regenerate (state preserved)";
  refreshBtn.addEventListener("click", () => {
    sessionStorage.setItem(STATE_KEY, JSON.stringify(captureState()));
    refreshBtn.textContent = "Refreshing…";
    refreshBtn.classList.add("copied");
    refreshBtn.disabled = true;
    location.reload();
  });
} else {
  // static file:// fallback: copy the absolute command
  refreshBtn.textContent = "Copy refresh command";
  refreshNote.textContent = "static file · for one-click, run with --serve";
  refreshBtn.title = "Static page — click to copy the command, then run it in a terminal";
  refreshBtn.addEventListener("click", () => {
    const cmd = "python3 " + DATA.script_abs_path;
    navigator.clipboard.writeText(cmd).then(() => {
      refreshBtn.textContent = "Copied — paste in terminal";
      refreshBtn.classList.add("copied");
      setTimeout(() => {
        refreshBtn.textContent = "Copy refresh command";
        refreshBtn.classList.remove("copied");
      }, 2400);
    });
  });
}

// ── headline cards (driven by BUILD_STATUS live ledger — primary lens) ──
const stats = DATA.stats;                  // MISSION_BACKLOG-derived (kept for secondary uses)
const lps = DATA.ledger_phase_stats || {}; // live BUILD_STATUS phases — primary
const lc = (DATA.stats && DATA.stats.ledger_counts) || {};
const v = DATA.velocity;
// "Focus now" = first phase (alphabetical) that still has open work (TODO/CLAIMED/IN_REVIEW > 0).
// Phase A is 100% MERGED today, so the headline auto-advances to Phase B.
const focusPhase = (() => {
  const order = ["A","B","C","D","E","F"];
  for (const p of order) {
    const info = (lps.by_phase || {})[p];
    if (!info || !info.n) continue;
    const c = info.counts || {};
    const open = (c.TODO||0) + (c.CLAIMED||0) + (c.IN_REVIEW||0);
    if (open > 0) return { p, info };
  }
  return { p: "A", info: (lps.by_phase || {}).A };
})();
const focusInfo = focusPhase.info || { pct: 0, n: 0, title: "" };
const focusBlocked = (focusInfo.counts && focusInfo.counts.BLOCKED) || 0;
const ledgerActive = (lc.IN_REVIEW || 0) + (lc.CLAIMED || 0);
const ledgerBlockedTotal = lps.total_blocked || (lc.BLOCKED || 0);
const cardsHTML = [
  { label: "% complete (weighted)",
    value: (lps.pct_total != null ? lps.pct_total : 0).toFixed(1) + "%",
    delta: (lps.total_done||0) + " merged of " + (lps.total_phased||0) + " phased ledger rows",
    cls: "accent" },
  { label: "Run rate (24h)",
    value: v.v_24h.toFixed(1),
    delta: "items/hr · 1h: " + v.v_1h.toFixed(1) + " · 6h: " + v.v_6h.toFixed(1),
    cls: v.v_24h > 0 ? "accent" : "" },
  { label: "Phase " + focusPhase.p + " — focus now",
    value: focusInfo.pct.toFixed(0) + "%",
    delta: (focusInfo.title || "") + " · " +
           ((focusInfo.counts && focusInfo.counts.TODO || 0) + (focusInfo.counts && focusInfo.counts.CLAIMED || 0) + (focusInfo.counts && focusInfo.counts.IN_REVIEW || 0)) + " open" +
           (focusBlocked > 0 ? " · " + focusBlocked + " blocked" : ""),
    cls: "warn" },
  { label: "In flight or review",
    value: fmt(ledgerActive),
    delta: (lc.CLAIMED || 0) + " claimed · " + (lc.IN_REVIEW || 0) + " in review" },
  { label: "Blocked (live ledger)",
    value: fmt(ledgerBlockedTotal),
    delta: ledgerBlockedTotal > 0
      ? Object.entries(lps.by_phase || {})
          .filter(([,info]) => (info.counts && info.counts.BLOCKED) > 0)
          .sort((a,b) => (b[1].counts.BLOCKED||0) - (a[1].counts.BLOCKED||0))
          .map(([p, info]) => `${p}: ${info.counts.BLOCKED}`)
          .join(" · ") || "needs unblock"
      : "all clear",
    cls: ledgerBlockedTotal > 0 ? "weak" : "" },
].map(c => `<div class="card ${c.cls||''}"><div class="label">${c.label}</div><div class="value">${c.value}</div><div class="delta">${c.delta}</div></div>`).join("");
$("#cards").innerHTML = cardsHTML;

// ── burn-up SVG ──
(() => {
  const svg = $("#burn");
  const W = 800, H = 240, P = { l: 40, r: 16, t: 16, b: 28 };
  const pts = DATA.burn_history;
  if (!pts.length) { svg.innerHTML = '<text x="400" y="120" text-anchor="middle" class="label">no history</text>'; return; }
  const xs = pts.map((_,i) => i);
  const maxY = Math.max(...pts.map(p => p.merged + p.in_review)) * 1.1 + 5;
  const x = i => P.l + (i / Math.max(1, pts.length-1)) * (W - P.l - P.r);
  const y = v => H - P.b - (v / maxY) * (H - P.t - P.b);

  const gridLines = [];
  for (let g = 0; g <= 4; g++) {
    const yv = (maxY * g/4);
    const yp = y(yv);
    gridLines.push(`<line class="grid-line" x1="${P.l}" x2="${W-P.r}" y1="${yp}" y2="${yp}"/>`);
    gridLines.push(`<text class="label" x="${P.l-6}" y="${yp+3}" text-anchor="end">${Math.round(yv)}</text>`);
  }

  const path = (key) => "M" + pts.map((p, i) => `${x(i)},${y(p[key])}`).join(" L");
  const fillPath = (key) => path(key) + ` L${x(pts.length-1)},${H-P.b} L${x(0)},${H-P.b} Z`;

  // x-axis labels: first, mid, last
  const xLabels = [];
  [0, Math.floor(pts.length/2), pts.length-1].forEach(i => {
    if (i < pts.length) xLabels.push(`<text class="label" x="${x(i)}" y="${H-P.b+16}" text-anchor="middle">${pts[i].date.slice(5)}</text>`);
  });

  svg.innerHTML = gridLines.join("") +
    `<path class="merged-fill" d="${fillPath('merged')}"/>` +
    `<path class="merged" d="${path('merged')}"/>` +
    `<path class="in-review" d="${path('in_review')}"/>` +
    `<path class="blocked" d="${path('blocked')}"/>` +
    xLabels.join("") +
    `<line class="axis" x1="${P.l}" x2="${W-P.r}" y1="${H-P.b}" y2="${H-P.b}"/>` +
    `<line class="axis" x1="${P.l}" x2="${P.l}" y1="${P.t}" y2="${H-P.b}"/>`;

  // Archive-aware burn-up: surface the +baseline so operators understand
  // why the green line starts above zero. Without this, a 463-row offset
  // looks like a parser bug.
  const baseline = DATA.burn_archive_baseline || 0;
  const legend = document.getElementById("burn-archive-legend");
  const note = document.getElementById("burn-note");
  if (baseline > 0 && legend) {
    legend.textContent = `+${baseline} pre-2026Q2 archive baseline`;
    legend.style.display = "inline";
    if (note) note.textContent = "total MERGED count = live ledger + archive baseline (continuous across 2026-05-13 sweep)";
  }
})();

// ── run rate panel ──
(() => {
  const v = DATA.velocity;
  const cells = [
    { lab: "Last 1 hour",  val: v.v_1h.toFixed(1),  hot: v.v_1h > v.v_24h },
    { lab: "Last 6 hours", val: v.v_6h.toFixed(1),  hot: v.v_6h > v.v_24h },
    { lab: "Last 24 hours",val: v.v_24h.toFixed(1), hot: false },
    { lab: "Overall",      val: v.v_overall.toFixed(1), hot: false },
  ];
  $("#rate-row").innerHTML = cells.map(c =>
    `<div class="rate-cell ${c.hot ? 'hot' : ''}"><div class="lab">${c.lab}</div><div class="v">${c.val}<small>/hr</small></div></div>`
  ).join("");
  // v.merged_now is the count of "MERGED" string occurrences at the latest
  // BUILD_STATUS commit — overcounts vs row-count because narrative cells
  // sometimes mention OTHER prs as merged. Show the live row count instead;
  // keep the burn-up history string-count semantics for backward continuity.
  const liveMerged = (DATA.stats && DATA.stats.ledger_counts && DATA.stats.ledger_counts.MERGED) || v.merged_now;
  let note = `Sample span: ${v.span_hours.toFixed(1)} hours · ${liveMerged} merged ledger rows`;
  if (v.idle_hours >= 0.5) {
    note += ` · ⏸ swarm idle ${v.idle_hours.toFixed(1)}h (last commit was ${v.idle_hours.toFixed(1)}h ago)`;
  }
  $("#rate-window-note").textContent = note;

  // hourly bars SVG
  const svg = $("#hourly");
  const bins = v.hourly_bins;
  if (!bins.length) { svg.innerHTML = '<text x="400" y="55" text-anchor="middle">no data</text>'; return; }
  const W = 800, H = 110, P = { l: 28, r: 8, t: 6, b: 18 };
  const maxC = Math.max(1, ...bins.map(b => b.count));
  const bw = (W - P.l - P.r) / bins.length;
  const x = i => P.l + i * bw;
  const yTop = c => H - P.b - (c / maxC) * (H - P.t - P.b);
  const rects = bins.map((b, i) =>
    `<rect class="${b.count === 0 ? 'idle' : ''}" x="${x(i)+1}" y="${b.count === 0 ? H-P.b-2 : yTop(b.count)}" width="${Math.max(1, bw-2)}" height="${b.count === 0 ? 2 : (H-P.b-yTop(b.count))}" />`
  ).join("");
  // x-labels: oldest, mid, newest
  const labels = [
    `<text x="${P.l}" y="${H-4}" text-anchor="start">${bins[0].hours_ago}h ago</text>`,
    `<text x="${W/2}" y="${H-4}" text-anchor="middle">${bins[Math.floor(bins.length/2)].hours_ago}h ago</text>`,
    `<text x="${W-P.r}" y="${H-4}" text-anchor="end">now</text>`,
    `<text x="${P.l-4}" y="${P.t+8}" text-anchor="end">${maxC}</text>`,
    `<text x="${P.l-4}" y="${H-P.b}" text-anchor="end">0</text>`,
  ].join("");
  svg.innerHTML = rects + labels;
})();

// ── ETA list (driven by live ledger phases — primary lens) ──
(() => {
  const e = DATA.etas;
  const lpsBP = (DATA.ledger_phase_stats || {}).by_phase || {};
  const rate = e.rate_used || 0;
  $("#eta-note").textContent = rate > 0 ? `using ${e.rate_window} rate of ${rate.toFixed(2)}/hr` : "rate too low to project";
  if (rate <= 0) {
    $("#eta-list").innerHTML = `<div class="eta-row no-rate">No run rate yet — need more BUILD_STATUS commit history before ETAs are meaningful.</div>`;
    return;
  }
  // Open-work weighting: TODO=1.0, CLAIMED=0.6, IN_REVIEW=0.3 (closer to merge → less remaining work).
  // BLOCKED is excluded from drain ETA — it won't merge until unblocked.
  const openWeighted = (c) => (c.TODO||0)*1.0 + (c.CLAIMED||0)*0.6 + (c.IN_REVIEW||0)*0.3;
  const openCount    = (c) => (c.TODO||0) + (c.CLAIMED||0) + (c.IN_REVIEW||0);
  let cumulative = 0;
  const phaseRows = Object.entries(lpsBP).map(([p, info]) => {
    if (!info.n) return '';
    const c = info.counts || {};
    const open = openCount(c);
    const openW = openWeighted(c);
    const blocked = c.BLOCKED || 0;
    const isComplete = open === 0 && blocked === 0;
    const stuck = open === 0 && blocked > 0;
    cumulative += openW;
    const focusedCell = isComplete
      ? `<span style="color:var(--accent);font-weight:600;">✓ complete</span>`
      : stuck
        ? `<span style="color:var(--weak);font-weight:600;">${blocked} blocked</span>`
        : fmtHours(openW / rate);
    const cumulCell = isComplete
      ? `<span style="color:var(--ink-3);">—</span>`
      : `seq ${fmtHours(cumulative / rate)}`;
    const detail = isComplete
      ? `${info.n} ledger rows · all merged`
      : `${open} open${blocked ? ` · ${blocked} blocked` : ''} (weighted ${openW.toFixed(1)})`;
    return `<div class="eta-row${isComplete ? ' eta-complete' : ''}">
      <div class="pl">${p}</div>
      <div class="ph-name">${info.title}<small>${detail}</small></div>
      <div class="focused">${focusedCell}</div>
      <div class="cumul">${cumulCell}</div>
    </div>`;
  }).join("");
  const drainRow = `<div class="eta-row summary">
    <div class="pl">drain</div>
    <div class="ph-name">Live ledger drain<small>TODO + CLAIMED + IN_REVIEW = ${e.ledger_drain.items} rows (excluding ${(DATA.ledger_phase_stats||{}).total_blocked||0} BLOCKED)</small></div>
    <div class="focused">${fmtHours(e.ledger_drain.eta_hours)}</div>
    <div class="cumul">to empty</div>
  </div>`;
  $("#eta-list").innerHTML = `<div style="display:grid;grid-template-columns:26px 1fr auto auto;gap:10px;padding-bottom:6px;color:var(--ink-3);font-size:10px;text-transform:uppercase;letter-spacing:0.06em;border-bottom:1px solid var(--hairline);"><div></div><div>Scope</div><div style="text-align:right;">Focused</div><div style="text-align:right;">Cumulative</div></div>` + phaseRows + drainRow;
})();

// ── donut (live ledger funnel — primary lens) ──
(() => {
  const svg = $("#donut");
  const lcounts = (DATA.stats && DATA.stats.ledger_counts) || {};
  const ledgerOrder = ["MERGED","IN_REVIEW","CLAIMED","TODO","BLOCKED","DEFERRED-v1.x"];
  const ledgerColor = {
    "MERGED": "#059669",
    "IN_REVIEW": "#10b981",
    "CLAIMED": "#2563eb",
    "TODO": "#9ca3af",
    "BLOCKED": "#dc2626",
    "DEFERRED-v1.x": "#a3a3a3",
  };
  // Locally rebind so the existing renderer below works against the live
  // ledger without touching its math.
  const stateColor = ledgerColor;
  const stateOrder = ledgerOrder;
  const counts = lcounts;
  const entries = stateOrder.map(s => [s, counts[s] || 0]).filter(([,n]) => n > 0);
  const total = entries.reduce((a,[,n]) => a+n, 0);
  const cx = 100, cy = 100, r = 70, ir = 44;
  let a0 = -Math.PI / 2;
  let paths = [];
  for (const [s, n] of entries) {
    const a1 = a0 + (n/total) * Math.PI * 2;
    const large = (a1 - a0) > Math.PI ? 1 : 0;
    const x0 = cx + r*Math.cos(a0), y0 = cy + r*Math.sin(a0);
    const x1 = cx + r*Math.cos(a1), y1 = cy + r*Math.sin(a1);
    const xi0 = cx + ir*Math.cos(a0), yi0 = cy + ir*Math.sin(a0);
    const xi1 = cx + ir*Math.cos(a1), yi1 = cy + ir*Math.sin(a1);
    paths.push(`<path d="M${x0},${y0} A${r},${r} 0 ${large} 1 ${x1},${y1} L${xi1},${yi1} A${ir},${ir} 0 ${large} 0 ${xi0},${yi0} Z" fill="${stateColor[s]}"/>`);
    a0 = a1;
  }
  paths.push(`<text x="100" y="98" text-anchor="middle" fill="var(--ink)" font-size="22" font-weight="600">${total}</text>`);
  paths.push(`<text x="100" y="116" text-anchor="middle" fill="var(--ink-3)" font-size="10" letter-spacing="1">rows</text>`);
  svg.innerHTML = paths.join("");

  $("#donut-legend").innerHTML = entries.map(([s,n]) =>
    `<li><span class="lab"><span class="sw" style="background:${stateColor[s]}"></span>${s}</span><span class="v">${n}</span></li>`
  ).join("");
})();

// ── stacked progress bar helper ──
function stackBar(counts, total, palette) {
  return Object.entries(counts)
    .sort(([a],[b]) => stateOrder.indexOf(a) - stateOrder.indexOf(b))
    .map(([s,n]) => `<span style="background:${palette[s]||'#444'};width:${(n/total)*100}%"></span>`).join("");
}

// (Strategic-completeness MISSION_BACKLOG phase cards intentionally removed —
// the live-ledger lens above is now the single phase view.)

// ── live-ledger phase cards (PRIMARY lens; BUILD_STATUS native phases) ──
(() => {
  const lps = DATA.ledger_phase_stats;
  if (!lps) return;
  // status colors mirror the live ledger funnel
  const ledgerColor = {
    "MERGED": "#059669",
    "IN_REVIEW": "#10b981",
    "CLAIMED": "#2563eb",
    "TODO": "#9ca3af",
    "BLOCKED": "#dc2626",
    "DEFERRED-v1.x": "#a3a3a3",
  };
  const ledgerOrder = ["MERGED","IN_REVIEW","CLAIMED","TODO","BLOCKED","DEFERRED-v1.x"];
  const stack = (counts, total) => ledgerOrder
    .filter(s => counts[s])
    .map(s => `<span style="background:${ledgerColor[s]};width:${(counts[s]/total)*100}%"></span>`).join("");

  const html = Object.entries(lps.by_phase).map(([p, info]) => {
    if (!info.n) return '';
    const c = info.counts;
    const blocked = c["BLOCKED"] || 0;
    const blockedClass = blocked > 0 ? 'phase-blocked' : '';
    return `<div class="phase-card ${blockedClass}" data-ledger-phase="${p}">
      <div class="ph-letter">${p}</div>
      <div class="ph-title">${info.title}</div>
      <div class="ph-pct">${info.pct.toFixed(0)}%</div>
      <div class="ph-stripe">${stack(c, info.n)}</div>
      <div class="ph-meta">
        ${c["MERGED"]||0} merged · ${(c["IN_REVIEW"]||0)+(c["CLAIMED"]||0)} active · ${c["TODO"]||0} todo
        ${blocked > 0 ? `· <span style="color:var(--weak);font-weight:600">${blocked} blocked</span>` : ''}
        ${(c["DEFERRED-v1.x"]||0) > 0 ? `· ${c["DEFERRED-v1.x"]} deferred` : ''}
      </div>
      <div class="ph-eta">${info.n} ledger rows</div>
    </div>`;
  }).join("");
  $("#ledger-phase-cards").innerHTML = html;

  $("#ledger-state-legend").innerHTML = ledgerOrder.map(s =>
    `<span><span class="sw" style="background:${ledgerColor[s]}"></span>${s}</span>`
  ).join("");

  $("#ledger-phase-summary").innerHTML =
    `<b>${lps.total_done}</b> merged of <b>${lps.total_phased}</b> phased rows ` +
    `(<b>${lps.pct_total.toFixed(1)}%</b> weighted) · ` +
    `<span style="color:var(--weak);font-weight:600">${lps.total_blocked} BLOCKED</span> across all phases`;

  // Click a phase card → open the inline drawer below the card grid.
  // Drawer shows phase header + sectioned brief lists (BLOCKED, IN_REVIEW,
  // CLAIMED, TODO). Click again to close; click another card to swap.
  const drawer = document.getElementById("phase-detail-drawer");
  function renderDrawer(p) {
    const info = lps.by_phase[p];
    if (!info) { drawer.classList.remove("open"); return; }
    const c = info.counts || {};
    const sections = [
      { key: "blocked_briefs", lab: "BLOCKED", color: "var(--weak)", count: c.BLOCKED || 0 },
      { key: "in_review_briefs", lab: "IN_REVIEW", color: "var(--in-review)", count: c.IN_REVIEW || 0 },
      { key: "claimed_briefs", lab: "CLAIMED", color: "var(--in-flight)", count: c.CLAIMED || 0 },
      { key: "todo_briefs", lab: "TODO", color: "var(--queued)", count: c.TODO || 0 },
    ];
    const ledger = DATA.ledger || {};
    const sectionHTML = sections.filter(s => s.count > 0).map(s => {
      const briefs = info[s.key] || [];
      const items = briefs.map(b => {
        const row = ledger[b] || {};
        const pr = (row.pr || "").trim();
        const prHTML = pr && pr !== "—" ? `<span class="dr-pr">${pr.replace(/^\(|\)$/g,'')}</span>` : '';
        return `<li><span class="dr-brief">${b}</span><span class="dr-owner">${row.owner||''}</span>${prHTML}</li>`;
      }).join("");
      return `<div class="dr-section">
        <div class="dr-sec-head"><span class="dr-sw" style="background:${s.color}"></span>${s.lab} <span class="dr-count">${s.count}</span></div>
        <ul class="dr-list">${items}</ul>
      </div>`;
    }).join("");
    drawer.innerHTML =
      `<div class="dr-head">
         <div class="dr-title"><b>Phase ${p}</b> — ${info.title}</div>
         <div class="dr-sub">${info.n} ledger rows · ${info.pct.toFixed(1)}% weighted complete · <b>${c.MERGED||0}</b> merged · <b>${c.IN_REVIEW||0}</b> in review · <b>${c.CLAIMED||0}</b> claimed · <b>${c.TODO||0}</b> todo · <b style="color:var(--weak);">${c.BLOCKED||0}</b> blocked</div>
         <button class="dr-close" aria-label="Close">×</button>
       </div>
       ${sectionHTML || '<div class="dr-empty">No open or blocked rows in this phase.</div>'}`;
    drawer.classList.add("open");
    drawer.dataset.openPhase = p;
    drawer.querySelector(".dr-close").addEventListener("click", () => {
      drawer.classList.remove("open");
      drawer.dataset.openPhase = "";
      $$('[data-ledger-phase]').forEach(c => c.classList.remove("active"));
    });
  }
  $$('[data-ledger-phase]').forEach(card => {
    card.addEventListener("click", () => {
      const p = card.dataset.ledgerPhase;
      const wasOpen = drawer.classList.contains("open") && drawer.dataset.openPhase === p;
      $$('[data-ledger-phase]').forEach(c => c.classList.remove("active"));
      if (wasOpen) {
        drawer.classList.remove("open");
        drawer.dataset.openPhase = "";
      } else {
        card.classList.add("active");
        renderDrawer(p);
        drawer.scrollIntoView({ behavior: "smooth", block: "nearest" });
      }
    });
  });
})();

// ── Tier M v1.0 ship-gate panel ──
(() => {
  const ts = DATA.tier_stats;
  if (!ts) return;
  const tierColor = {"M":"#f59e0b","A":"#10b981","B":"#34d399","P":"#9ca3af"};
  const ledgerStateColor = {
    "MERGED": "#059669",
    "IN_REVIEW": "#10b981",
    "CLAIMED": "#2563eb",
    "TODO": "#9ca3af",
    "BLOCKED": "#dc2626",
    "DEFERRED-v1.x": "#a3a3a3",
  };
  const m = ts.by_tier.M || {n:0, counts:{}, rows:[], pct:0};
  const counts = m.counts || {};
  $("#tier-m-note").innerHTML =
    `<b>${counts.MERGED||0}/${m.n}</b> MERGED · ${m.pct.toFixed(0)}% · ` +
    `${counts.IN_REVIEW||0} in review · ${counts.CLAIMED||0} claimed · ` +
    `<span style="color:var(--weak);font-weight:600">${counts.BLOCKED||0} blocked</span>`;
  const rowsHTML = (m.rows || []).map(r => {
    const pr = (r.pr||"").trim().replace(/^\(|\)$/g,'');
    const prCell = pr && pr !== "—" ? `<span class="tm-pr">${pr}</span>` : '<span class="tm-pr-empty">—</span>';
    const sc = ledgerStateColor[r.status] || "#777";
    return `<li>
      <span class="tm-pill" style="background:${sc}20;color:${sc};border:1px solid ${sc}50">${r.status}</span>
      <span class="tm-brief">${r.brief}</span>
      <span class="tm-phase">phase ${r.phase||'—'}</span>
      <span class="tm-owner">${r.owner||''}</span>
      ${prCell}
    </li>`;
  }).join("");
  $("#tier-m-roadmap").innerHTML = `<ul class="tm-list">${rowsHTML}</ul>`;
  // Compact summary for Tier A / B / P below
  const others = ["A","B","P"].map(t => {
    const b = ts.by_tier[t] || {n:0,counts:{},pct:0};
    if (!b.n) return '';
    return `<span style="margin-right:18px;">
      <span class="tm-tier-letter" style="background:${tierColor[t]}20;color:${tierColor[t]};border:1px solid ${tierColor[t]}50">Tier ${t}</span>
      ${b.counts.MERGED||0}/${b.n} merged (${b.pct.toFixed(0)}%) · ${ts.tier_titles[t]}
    </span>`;
  }).join("");
  $("#tier-other").innerHTML = others || '';
})();

// ── Product Roadmap · tracks A–G (+ discovered DRAFT tracks) ──
(() => {
  const r = DATA.product_roadmap;
  if (!r || !r.available || !r.tracks || !r.tracks.length) return;
  const panel = document.getElementById("roadmap-panel");
  panel.style.display = "block";
  const statusColor = {
    "DONE": "#059669",
    "IN PROGRESS": "#2563eb",
    "QUEUED": "#d97706",
    "NOT STARTED": "#6b7280",
    "DRAFT": "#9333ea",
  };
  const cards = r.tracks.map(t => {
    const c = statusColor[t.status] || "#6b7280";
    const scopeShort = (t.scope || "").split(/[—\-–]/)[0].trim().slice(0, 28);
    const noteShort = (t.note || "").replace(/\[[^\]]*\]\([^)]*\)/g, '').slice(0, 90);
    const briefCount = (typeof t.brief_count === "number") ? t.brief_count : null;
    const badge = briefCount !== null
      ? `<span class="ph-badge" title="${briefCount} brief file${briefCount === 1 ? '' : 's'} under streams/" style="position:absolute;top:6px;right:8px;font-size:10px;background:${c}1a;color:${c};border:1px solid ${c}40;border-radius:9px;padding:1px 7px;font-weight:600;letter-spacing:0.02em;">${briefCount}</span>`
      : "";
    return `<div class="phase-card" style="border-color:${c}40;position:relative;">
      ${badge}
      <div class="ph-letter" style="font-size:22px;">${t.letter}</div>
      <div class="ph-title" style="min-height:2.6em;">${escapeHtml(scopeShort)}</div>
      <div class="ph-pct" style="color:${c};font-size:14px;text-transform:uppercase;letter-spacing:0.04em;">${t.status}</div>
      <div class="ph-meta" style="margin-top:8px;font-family:inherit;">${(t.closes && t.closes !== "—") ? "closes: " + escapeHtml(t.closes) : escapeHtml(t.closes || "")}</div>
      ${noteShort ? `<div class="ph-eta" style="text-transform:none;letter-spacing:0;margin-top:6px;">${escapeHtml(noteShort)}…</div>` : ""}
    </div>`;
  }).join("");
  $("#roadmap-tracks").innerHTML = cards;
  const letters = r.tracks.map(t => t.letter);
  const letterRange = letters.length > 1 ? `${letters[0]}–${letters[letters.length-1]}` : letters[0];
  const rl = $("#roadmap-letters"); if (rl) rl.textContent = letterRange;
  const done = (r.by_status||{})["DONE"] || 0;
  const inProg = (r.by_status||{})["IN PROGRESS"] || 0;
  const queued = (r.by_status||{})["QUEUED"] || 0;
  const notStarted = (r.by_status||{})["NOT STARTED"] || 0;
  const draft = (r.by_status||{})["DRAFT"] || 0;
  const draftBit = draft ? ` · <span style="color:${statusColor.DRAFT}">${draft} draft</span>` : "";
  $("#roadmap-legend").innerHTML =
    `<b>${r.pct.toFixed(0)}%</b> weighted · ` +
    `<span style="color:${statusColor.DONE}">${done} done</span> · ` +
    `<span style="color:${statusColor['IN PROGRESS']}">${inProg} in progress</span> · ` +
    `<span style="color:${statusColor.QUEUED}">${queued} queued</span> · ` +
    `<span style="color:${statusColor['NOT STARTED']}">${notStarted} not started</span>` +
    draftBit;
})();

// ── conductor advisories tracker ──
(() => {
  const advisories = DATA.conductor_advisories || [];
  if (!advisories.length) return;
  const panel = document.getElementById("advisory-panel");
  const list = document.getElementById("advisory-list");
  if (!panel || !list) return;
  // ACTIONED → green; anything else (UNKNOWN, OPEN, DRAFT, ...) → warn.
  const statusColor = (s) => s === "ACTIONED" ? "var(--accent)" : "var(--warn)";
  const html = advisories.map(a => {
    const sc = statusColor(a.status);
    return `<li>
      <span class="adv-file">${escapeHtml(a.filename)}<small>${escapeHtml(a.title)}</small></span>
      <span class="adv-status" style="background:${sc}1a;color:${sc};border:1px solid ${sc}66;">${escapeHtml(a.status)}</span>
      <span class="adv-date">${escapeHtml(a.date || "—")}</span>
    </li>`;
  }).join("");
  list.innerHTML = html;
  const note = document.getElementById("advisory-note");
  if (note) {
    const actioned = advisories.filter(a => a.status === "ACTIONED").length;
    note.textContent = `${actioned}/${advisories.length} ACTIONED · docs/build-plan/conductor-advisory-*.md`;
  }
  panel.style.display = "block";
})();

// ── backlog-gap panel ──
(() => {
  const g = DATA.backlog_gap;
  if (!g) return;
  const el = $("#backlog-gap");
  if (!el) return;
  const recent = (g.outside_recent_merged || []).map(r =>
    `<li>
      <span class="bg-pr">${(r.pr||'').replace(/^\(|\)$/g,'')}</span>
      <span class="bg-brief">${r.brief}</span>
      <span class="bg-phase">phase ${r.phase || '—'}</span>
    </li>`
  ).join("");
  el.innerHTML =
    `<div class="bg-grid">
      <div class="bg-stat"><div class="bg-num">${g.ledger_unique_briefs}</div><div class="bg-lab">unique briefs in BUILD_STATUS</div></div>
      <div class="bg-stat"><div class="bg-num">${g.backlog_referenced_briefs}</div><div class="bg-lab">referenced from MISSION_BACKLOG</div></div>
      <div class="bg-stat" style="background:var(--accent-soft);"><div class="bg-num" style="color:var(--accent)">${g.outside_backlog_count}</div><div class="bg-lab">live ledger work the 280-item view hides</div></div>
    </div>
    <div class="bg-recent-head">Recent MERGED briefs outside the backlog (top ${(g.outside_recent_merged||[]).length}):</div>
    <ul class="bg-recent">${recent}</ul>`;
})();

// (Live-ledger horizontal bars panel removed — the donut above renders the
// same data with a center-total readout, so the bars were redundant.)

// ── per-owner velocity ──
(() => {
  const ov = DATA.owner_velocity || {by_owner: {}};
  // Filter to real owners: must look like an identifier, not "(in landing pr)" or other.
  const owners = Object.entries(ov.by_owner).filter(([k]) => /^[a-z][a-z0-9_-]*$/i.test(k));
  if (!owners.length) {
    $("#owner-velocity").innerHTML = `<div style="color:var(--ink-3);font-size:11px;">No owner data available.</div>`;
    return;
  }
  const max24h = Math.max(1, ...owners.map(([,c]) => c.merged_24h));
  const html = owners.map(([owner, c]) => {
    const active = c.in_review + c.claimed;
    const barW = (c.merged_24h / max24h) * 100;
    return `<div class="ov-row">
      <div class="ov-name">${owner}</div>
      <div class="ov-bar"><span style="background:var(--accent);width:${barW}%"></span></div>
      <div class="ov-num">${c.merged_24h}<small>/24h</small></div>
      <div class="ov-meta">${active} active <span style="color:var(--mute)">(${c.in_review} IR · ${c.claimed} CL)</span> · ${c.todo} todo · <span style="color:var(--weak)">${c.blocked} blocked</span></div>
    </div>`;
  }).join("");
  $("#owner-velocity").innerHTML = html;
})();

// ── stale CLAIMs ──
(() => {
  const stales = DATA.stale_claims || [];
  const target = $("#stale-claims");
  if (!stales.length) {
    target.innerHTML = `<div style="color:var(--accent);font-size:12px;padding:6px 0;">✓ No stale CLAIMs — every CLAIMED row was claimed within the last 24h.</div>`;
    return;
  }
  const rows = stales.slice(0, 20).map(s => {
    const ageColor = s.age_hours >= 72 ? 'var(--weak)' : (s.age_hours >= 48 ? 'var(--warn)' : 'var(--ink-2)');
    return `<li>
      <span class="sc-brief">${s.brief}</span>
      <span class="sc-owner">${s.owner||''}</span>
      <span class="sc-phase">phase ${s.phase||'—'}</span>
      <span class="sc-age" style="color:${ageColor}">${s.age_hours}h</span>
    </li>`;
  }).join("");
  const more = stales.length > 20 ? `<div style="color:var(--ink-3);font-size:10px;margin-top:6px;">…and ${stales.length - 20} more</div>` : '';
  target.innerHTML = `<ul class="sc-list">${rows}</ul>${more}`;
})();

// ── claims (compact list) ──
(() => {
  const claims = DATA.stats.by_claim;
  const tracksByClaim = (DATA.product_roadmap && DATA.product_roadmap.tracks_by_claim) || {};
  const html = Object.entries(claims).map(([cid, c]) => {
    const tracks = tracksByClaim[cid] || [];
    const tracksBadge = tracks.length
      ? `<span class="claim-tracks" title="Tracks in PRODUCT_ROADMAP.md that declare they close this claim">tracks: ${tracks.join(", ")}</span>`
      : "";
    return `<div class="progress-row">
      <div class="pname">Claim ${cid} — ${c.label}${tracksBadge}<small>${c.n} items · ${c.counts["Done"]||0} done</small></div>
      <div class="pbar">${stackBar(c.counts, c.n||1, stateColor)}</div>
      <div class="ppct">${c.pct.toFixed(0)}%</div>
    </div>`;
  }).join("");
  $("#claims-compact").innerHTML = html;
})();

// ── sections ──
(() => {
  const secs = DATA.stats.by_section;
  const html = Object.entries(secs).map(([name, info]) =>
    `<div class="progress-row">
      <div class="pname">${name}<small>${info.n} items</small></div>
      <div class="pbar">${stackBar(info.counts, info.n, stateColor)}</div>
      <div class="ppct">${info.pct.toFixed(0)}%</div>
    </div>`
  ).join("");
  $("#section-bars").innerHTML = html;
})();

// ── activity feed ──
(() => {
  const html = DATA.git.map(c => {
    const day = c.date.slice(0,10);
    const subj = escapeHtml(c.subject);
    const cls = c.is_merge ? "merge" : "";
    const prLink = c.pr ? ` <a href="https://github.com/anthropics/argo/pull/${c.pr}" target="_blank">#${c.pr}</a>` : '';
    const subjNoPr = subj.replace(/\(#\d+\)$/, '').trim();
    return `<div class="row"><div class="d">${day}</div><div class="h">${c.sha}</div><div class="s ${cls}">${subjNoPr}${prLink}</div></div>`;
  }).join("");
  $("#activity").innerHTML = html;
})();

// ── drilldown table ──
(() => {
  const items = DATA.items;
  const fstate = $("#fstate");
  const fphase = $("#fphase");
  stateOrder.forEach(s => { const o = document.createElement("option"); o.value=s; o.textContent=s; fstate.appendChild(o); });
  Object.keys(DATA.stats.by_phase).forEach(p => {
    if (DATA.stats.by_phase[p].n) {
      const o = document.createElement("option"); o.value=p; o.textContent="Phase "+p; fphase.appendChild(o);
    }
  });

  const phaseLookup = {};
  Object.entries(DATA.stats.by_phase).forEach(([p, info]) => {
    info.items.forEach(n => { phaseLookup[n] = (phaseLookup[n] || []).concat(p); });
  });

  function render() {
    const q = $("#q").value.toLowerCase();
    const sf = fstate.value;
    const pf = fphase.value;
    const tb = $("#items-table tbody");
    let n = 0;
    tb.innerHTML = items.filter(it => {
      if (sf && it.state !== sf) return false;
      if (pf && !(phaseLookup[it.n]||[]).includes(pf)) return false;
      if (q && !(it.body.toLowerCase().includes(q) || (it.brief_id||'').toLowerCase().includes(q))) return false;
      return true;
    }).map(it => {
      n++;
      const pillStyle = `background:${stateColor[it.state]}22;color:${stateColor[it.state]};border:1px solid ${stateColor[it.state]}55`;
      let briefBlob = '';
      if (it.brief_id) {
        const ledgerRow = DATA.ledger[it.brief_id];
        briefBlob = `<span class="bid">${it.brief_id}</span>`;
        if (ledgerRow) {
          const lp = ledgerRow.pr && ledgerRow.pr !== '—' ? ' ' + escapeHtml(ledgerRow.pr) : '';
          briefBlob += ` · <span style="color:${ledgerColor[ledgerRow.status]||'var(--ink-3)'}">${ledgerRow.status}${lp}</span>`;
        } else {
          briefBlob += ` <span style="color:var(--ink-3)">(not in ledger)</span>`;
        }
        if (it.derivation === 'git' && it.pr) {
          briefBlob += ` · <span class="badge-pr" title="BUILD_STATUS.md is stale; this PR has actually merged">via <a href="https://github.com/anthropics/argo/pull/${it.pr}" target="_blank">#${it.pr}</a> ✓</span>`;
        } else if (it.derivation === 'ledger+git' && it.pr) {
          briefBlob += ` · <a href="https://github.com/anthropics/argo/pull/${it.pr}" target="_blank">#${it.pr}</a>`;
        }
      }
      return `<tr>
        <td class="n">${it.n}</td>
        <td class="state"><span class="pill" style="${pillStyle}">${it.state}</span></td>
        <td class="body">${escapeHtml(stripTags(it.body))}</td>
        <td class="brief">${briefBlob}</td>
      </tr>`;
    }).join("");
    $("#rowcount").textContent = n + " of " + items.length;
  }

  $("#q").addEventListener("input", render);
  fstate.addEventListener("change", render);
  fphase.addEventListener("change", render);
  render();
})();

// ── live-ledger drill (PRIMARY lens · operates on the full 398-row ledger) ──
(() => {
  const ledger = DATA.ledger || {};
  const briefs = Object.keys(ledger).sort();
  const ledgerStateOrder = ["MERGED","IN_REVIEW","CLAIMED","TODO","BLOCKED","DEFERRED-v1.x"];
  const ledgerStateColor = {
    "MERGED":"#059669","IN_REVIEW":"#10b981","CLAIMED":"#2563eb",
    "TODO":"#9ca3af","BLOCKED":"#dc2626","DEFERRED-v1.x":"#a3a3a3",
  };
  const lstate = $("#lstate");
  const lphase = $("#lphase");
  const ltier = $("#ltier");
  const lowner = $("#lowner");
  if (!lstate) return;
  const tierMap = DATA.tier_map || {};
  const tierColor = {"M":"#f59e0b","A":"#10b981","B":"#34d399","P":"#9ca3af"};
  ledgerStateOrder.forEach(s => { const o=document.createElement("option"); o.value=s; o.textContent=s; lstate.appendChild(o); });
  // Phase options pulled from the live ledger phase stats so we don't hardcode A–F
  Object.keys(DATA.ledger_phase_stats?.by_phase || {}).forEach(p => {
    const o=document.createElement("option"); o.value=p; o.textContent="Phase "+p; lphase.appendChild(o);
  });
  // Tier options
  ["M","A","B","P"].forEach(t => {
    const o=document.createElement("option"); o.value=t; o.textContent="Tier "+t; ltier.appendChild(o);
  });
  // Owner options: real ones only (skip "(in landing PR)" etc.)
  const owners = [...new Set(Object.values(ledger).map(r => (r.owner||"").trim()).filter(o => /^[a-z][a-z0-9_-]*$/i.test(o)))].sort();
  owners.forEach(o => { const opt=document.createElement("option"); opt.value=o; opt.textContent=o; lowner.appendChild(opt); });
  $("#ledger-drill-total").textContent = briefs.length;

  function render() {
    const q = $("#lq").value.trim().toLowerCase();
    const sf = lstate.value;
    const pf = lphase.value;
    const tf = ltier.value;
    const of_ = lowner.value;
    const tb = $("#ledger-table tbody");
    const out = [];
    let n = 0;
    for (const b of briefs) {
      const r = ledger[b];
      const tier = tierMap[b] || "";
      if (q && !b.toLowerCase().includes(q)) continue;
      if (sf && r.status !== sf) continue;
      if (pf && r.phase !== pf) continue;
      if (tf && tier !== tf) continue;
      if (of_ && (r.owner||"").trim().toLowerCase() !== of_) continue;
      n++;
      const pr = (r.pr||"").trim().replace(/^\(|\)$/g,'');
      const prCell = pr && pr !== "—" ? `<span class="lpr">${escapeHtml(pr)}</span>` : '';
      const tierCell = tier
        ? `<span class="tm-tier-letter" style="background:${tierColor[tier]}20;color:${tierColor[tier]};border:1px solid ${tierColor[tier]}50">${tier}</span>`
        : '<span style="color:var(--mute)">—</span>';
      out.push(`<tr>
        <td><span class="lstate-pill" style="background:${ledgerStateColor[r.status]||'#444'}20;color:${ledgerStateColor[r.status]||'#aaa'};border:1px solid ${ledgerStateColor[r.status]||'#444'}40">${r.status}</span></td>
        <td class="ltier">${tierCell}</td>
        <td class="lphase">${r.phase||'—'}</td>
        <td class="lbrief">${escapeHtml(b)}</td>
        <td class="lowner">${escapeHtml(r.owner||'')}</td>
        <td>${prCell}</td>
      </tr>`);
    }
    tb.innerHTML = out.join("");
    $("#lcount").textContent = n + " of " + briefs.length;
  }
  $("#lq").addEventListener("input", render);
  lstate.addEventListener("change", render);
  lphase.addEventListener("change", render);
  ltier.addEventListener("change", render);
  lowner.addEventListener("change", render);
  render();
})();

function stripTags(s) {
  // remove the [...] tags from the end of body for cleaner display
  return s.replace(/\s*\[(NEW|BRIEF|SHIPPED|PARTIAL|DEFER)[^\]]*\]\s*$/, '').replace(/\s*\([Ff]olded[^)]*\)\s*$/, '').trim();
}
function escapeHtml(s) {
  return String(s).replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
}
</script>
</body>
</html>
"""


# ─── Main ───────────────────────────────────────────────────────────────────

def build_payload() -> dict:
    """Re-read all sources and return the payload dict the HTML embeds."""
    ledger, ledger_phase_titles = parse_ledger(read(LEDGER))
    items, phase_map = parse_backlog(read(BACKLOG))
    pr_lookup = build_pr_lookup()
    stale_ledger_count = 0
    for it in items:
        state, derivation, pr = effective_state(it, ledger, pr_lookup)
        it["state"] = state
        it["derivation"] = derivation
        it["pr"] = pr
        if derivation == "git":
            stale_ledger_count += 1
    stats = compute_stats(items, ledger, phase_map)
    ledger_phase_stats = compute_ledger_phase_stats(ledger, ledger_phase_titles)
    tier_map = parse_tier_map(ledger)
    tier_stats = compute_tier_stats(ledger, tier_map)
    product_roadmap = parse_product_roadmap()
    freshness = repo_freshness()
    backlog_brief_tagged_count = sum(1 for it in items if it.get("brief_id"))
    backlog_brief_ids = {it["brief_id"] for it in items if it.get("brief_id")}
    ledger_brief_ids = set(ledger.keys())
    outside_brief_ids = sorted(ledger_brief_ids - backlog_brief_ids)
    # Recent merged briefs that are outside the backlog — surfaces what the
    # 280-item view hides. Sort by PR number (highest first ≈ most recent).
    def _pr_int(b: str) -> int:
        pr = ledger.get(b, {}).get("pr", "") or ""
        m = re.match(r"#?(\d+)", pr.lstrip("#").lstrip("("))
        return int(m.group(1)) if m else -1
    outside_recent_merged = sorted(
        [b for b in outside_brief_ids if ledger.get(b, {}).get("status") == "MERGED"],
        key=_pr_int, reverse=True,
    )[:12]
    backlog_gap = {
        "ledger_unique_briefs": len(ledger_brief_ids),
        "backlog_referenced_briefs": len(backlog_brief_ids),
        "outside_backlog_count": len(outside_brief_ids),
        "outside_recent_merged": [
            {
                "brief": b,
                "status": ledger[b]["status"],
                "pr": ledger[b]["pr"],
                "phase": ledger[b].get("phase"),
            }
            for b in outside_recent_merged
        ],
    }
    git = recent_commits(50)
    archive_baseline = archive_merged_baseline()
    burn = burn_history(archive_baseline=archive_baseline)
    velocity = velocity_stats(burn)
    advisories = parse_conductor_advisories()
    etas = project_etas(stats, velocity)
    owners = owner_velocity(burn, ledger)
    stales = stale_claims(ledger, threshold_hours=24.0)
    return {
        "generated_at_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "generated_at_local": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "partial_weight": COMPLETION_WEIGHT["Partial"],
        "script_abs_path": str((HERE / "refresh.py").resolve()),
        "stale_ledger_count": stale_ledger_count,
        "items": items,
        "ledger": ledger,
        "pr_lookup": pr_lookup,
        "stats": stats,
        "ledger_phase_stats": ledger_phase_stats,
        "ledger_phase_titles": ledger_phase_titles,
        "tier_stats": tier_stats,
        "tier_map": tier_map,
        "product_roadmap": product_roadmap,
        "freshness": freshness,
        "backlog_brief_tagged_count": backlog_brief_tagged_count,
        "backlog_gap": backlog_gap,
        "owner_velocity": owners,
        "stale_claims": stales,
        "git": git,
        "burn_history": burn,
        "burn_archive_baseline": archive_baseline,
        "velocity": velocity,
        "etas": etas,
        "claim_labels": CLAIM_LABELS,
        "conductor_advisories": advisories,
    }


def serve(port: int, open_browser: bool, auto_pull_seconds: int = 900) -> int:
    """Run an http server that regenerates the dashboard on every request.
    One-click refresh = browser reload.

    If auto_pull_seconds > 0, a background thread runs `git fetch origin main`
    + `git pull --ff-only origin main` in ROOT every N seconds, so the
    dashboard self-heals to current main without manual `git pull`. Pull is
    skipped silently if HEAD isn't `main` or fast-forward isn't possible —
    your in-flight work is never disturbed.
    """
    from http.server import BaseHTTPRequestHandler, HTTPServer

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path.split("?", 1)[0] in ("/", "/dashboard.html", "/index.html"):
                try:
                    payload = build_payload()
                    body = render_html(payload).encode("utf-8")
                except Exception as exc:  # show the error in the browser
                    body = (f"<pre style='color:#dc2626;background:#0f1115;color:#e6e9ef;"
                            f"padding:24px;font:12px monospace'>"
                            f"refresh failed: {type(exc).__name__}: {exc}</pre>").encode()
                    self.send_response(500)
                    self.send_header("Content-Type", "text/html; charset=utf-8")
                    self.end_headers()
                    self.wfile.write(body)
                    return
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Cache-Control", "no-store, max-age=0")
                self.end_headers()
                self.wfile.write(body)
            else:
                self.send_response(404); self.end_headers()
        def log_message(self, fmt, *a):  # one-line, quieter than default
            sys.stderr.write(f"[{self.log_date_time_string()}] {self.address_string()} {fmt % a}\n")

    url = f"http://127.0.0.1:{port}/"
    httpd = HTTPServer(("127.0.0.1", port), Handler)
    print(f"mission dashboard serving at {url}")
    print(f"  press Ctrl+C to stop")
    print(f"  every page-load regenerates against current sources")
    print(f"  reading from: {ROOT}")
    if auto_pull_seconds > 0:
        import threading
        mins = auto_pull_seconds / 60
        print(f"  auto-pull: every {mins:g} min (origin/main → {ROOT}, ff-only)")
        threading.Thread(
            target=_auto_pull_loop, args=(auto_pull_seconds,), daemon=True,
        ).start()
    if open_browser:
        import webbrowser, threading
        threading.Timer(0.4, lambda: webbrowser.open(url)).start()
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nstopped.")
    return 0


def main() -> int:
    import argparse
    ap = argparse.ArgumentParser(description="Generate (or serve) the SKYCOT mission dashboard.")
    ap.add_argument("--partial-weight", type=float, default=DEFAULT_PARTIAL_WEIGHT,
                    help=f"Weight contribution of [PARTIAL] items toward % complete "
                         f"(0.0–1.0, default {DEFAULT_PARTIAL_WEIGHT}). "
                         "e.g. --partial-weight 0.25 for strict, 0.75 for generous.")
    ap.add_argument("--serve", action="store_true",
                    help="Run as an http server; every reload regenerates the page. "
                         "Lets the in-page Refresh button work as one-click.")
    ap.add_argument("--port", type=int, default=8765, help="Port for --serve (default 8765).")
    ap.add_argument("--auto-pull-seconds", type=int, default=900,
                    help="With --serve, run `git fetch + git pull --ff-only origin main` "
                         "in ROOT every N seconds. Default 900 (15 min). 0 disables.")
    ap.add_argument("--open", action="store_true",
                    help="Open the browser to the dashboard URL (with --serve) or to "
                         "the static file (without).")
    args = ap.parse_args()
    if not (0.0 <= args.partial_weight <= 1.0):
        print(f"error: --partial-weight must be 0.0..1.0, got {args.partial_weight}", file=sys.stderr)
        return 2
    COMPLETION_WEIGHT["Partial"] = args.partial_weight

    if args.serve:
        return serve(port=args.port, open_browser=args.open,
                     auto_pull_seconds=args.auto_pull_seconds)

    payload = build_payload()
    if len(payload["items"]) != 280:
        print(f"warn: parsed {len(payload['items'])} items (expected 280)", file=sys.stderr)

    out = HERE / "dashboard.html"
    out.write_text(render_html(payload), encoding="utf-8")
    print(f"wrote {out} ({out.stat().st_size:,} bytes)")
    print(f"open:  file://{out}")
    if args.open:
        import webbrowser
        webbrowser.open(f"file://{out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
