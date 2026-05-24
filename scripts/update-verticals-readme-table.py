#!/usr/bin/env python3
"""Refresh or check the loadable-vertical inventory in verticals/README.md."""

from __future__ import annotations

import argparse
import difflib
import json
import os
import subprocess
import sys
from pathlib import Path
from urllib.parse import quote


SECTION_HEADING = "## Verticals shipped today\n"
NEXT_SECTION = "\nWhen authoring a NEW vertical:\n"
START_MARKER = "<!-- verticals-readme-table:start -->"
END_MARKER = "<!-- verticals-readme-table:end -->"


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def vertical_list_command(root: Path) -> list[str]:
    argo_bin = os.environ.get("ARGO_BIN")
    if argo_bin:
        return [argo_bin, "vertical", "list", "--source", "workspace", "--json"]

    candidates: list[Path] = []
    target_dir = os.environ.get("CARGO_TARGET_DIR")
    if target_dir:
        candidates.append(Path(target_dir) / "debug" / "argo")
    candidates.append(root / "target" / "debug" / "argo")
    for candidate in candidates:
        if candidate.is_file():
            return [
                str(candidate),
                "vertical",
                "list",
                "--source",
                "workspace",
                "--json",
            ]

    return [
        "cargo",
        "run",
        "-q",
        "-p",
        "argo-cli",
        "--",
        "vertical",
        "list",
        "--source",
        "workspace",
        "--json",
    ]


def run_vertical_list(root: Path) -> dict:
    env = os.environ.copy()
    env["ARGO_VERTICALS_DIR"] = str(root / "verticals")
    cmd = vertical_list_command(root)
    proc = subprocess.run(
        cmd,
        cwd=root,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        if proc.stdout:
            print(proc.stdout, file=sys.stderr, end="")
        if proc.stderr:
            print(proc.stderr, file=sys.stderr, end="")
        fail("argo vertical list failed")
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        if proc.stdout:
            print(proc.stdout, file=sys.stderr, end="")
        if proc.stderr:
            print(proc.stderr, file=sys.stderr, end="")
        fail(f"argo vertical list did not emit JSON: {exc}")
    if data.get("errors"):
        for err in data["errors"]:
            print(f"{err.get('dir')}: {err.get('error')}", file=sys.stderr)
        fail("vertical loader reported errors")
    if not data.get("ok"):
        fail("vertical loader returned ok=false")
    return data


def vertical_root(root: Path, row: dict) -> Path:
    raw = Path(row["root"])
    if not raw.is_absolute():
        raw = root / raw
    return raw.resolve()


def vertical_dir(root: Path, row: dict) -> str:
    verticals_root = (root / "verticals").resolve()
    try:
        return vertical_root(root, row).relative_to(verticals_root).as_posix()
    except ValueError:
        fail(f"loaded vertical root is outside verticals/: {row['root']}")


def immediate_dirs_without_manifest(root: Path, loaded_dirs: set[str]) -> list[str]:
    out: list[str] = []
    for path in sorted((root / "verticals").iterdir()):
        if not path.is_dir():
            continue
        rel = path.relative_to(root / "verticals").as_posix()
        if rel not in loaded_dirs and not (path / "vertical.toml").is_file():
            out.append(rel)
    return out


def code(value: str) -> str:
    return f"`{value}`"


def link(label: str, href: str) -> str:
    return f"[{label}]({href})"


def table_block(root: Path, data: dict) -> str:
    rows = sorted(data["loaded"], key=lambda row: row["id"])
    loaded_dirs = {vertical_dir(root, row) for row in rows}
    excluded = immediate_dirs_without_manifest(root, loaded_dirs)
    excluded_text = (
        "none"
        if not excluded
        else ", ".join(code(f"{directory}/") for directory in excluded)
    )

    lines = [
        f"The workspace currently contains **{len(rows)} loadable vertical templates**.",
        "",
        f"Excluded top-level directories without `vertical.toml`: {excluded_text}.",
        "",
        "| Manifest id | Directory | Version | Worker v3 actions | Persona source |",
        "|---|---|---|---:|---|",
    ]

    for row in rows:
        directory = vertical_dir(root, row)
        manifest_href = "./" + quote(f"{directory}/vertical.toml", safe="/")
        manifest_id = link(row["id"], manifest_href)
        actions = str(row["action_count"])
        persona = row["persona"]
        persona_cell = "none" if persona == "-" else code(persona)
        lines.append(
            "| "
            + " | ".join(
                [
                    manifest_id,
                    code(f"{directory}/"),
                    code(row["version"]),
                    actions,
                    persona_cell,
                ]
            )
            + " |"
        )
    return "\n".join(lines)


def generated_section(root: Path, data: dict) -> str:
    return "\n".join(
        [
            "The table below is generated from `argo vertical list --source workspace --json`.",
            "That CLI path calls `argo_vertical::load_dir`, so the row set matches runtime discovery.",
            "Regenerate with `scripts/update-verticals-readme-table.py --write`; check with `scripts/update-verticals-readme-table.py --check`.",
            "",
            START_MARKER,
            table_block(root, data),
            END_MARKER,
            "",
            "The set covers a deliberate spread across library-tier skills, technical review, front-office coordination, GTM/sales, finance, healthcare, legal, recruiting, ops, procurement, QA, real estate, research, and data-entry work. These templates run through the same runtime, sharing zero domain-specific code in the platform crates -- that is the abstraction working.",
            "",
        ]
    )


def replace_section(readme: str, section: str) -> str:
    if SECTION_HEADING not in readme:
        fail(f"missing section heading: {SECTION_HEADING.strip()}")
    if NEXT_SECTION not in readme:
        fail(f"missing next-section anchor: {NEXT_SECTION.strip()}")
    prefix, rest = readme.split(SECTION_HEADING, 1)
    _old_body, suffix = rest.split(NEXT_SECTION, 1)
    return prefix + SECTION_HEADING + "\n" + section + NEXT_SECTION + suffix


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Refresh or check the generated vertical inventory table."
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true", help="rewrite verticals/README.md")
    mode.add_argument("--check", action="store_true", help="fail if the table is stale")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = repo_root()
    readme_path = root / "verticals" / "README.md"
    current = readme_path.read_text()
    expected = replace_section(current, generated_section(root, run_vertical_list(root)))

    if args.write:
        if expected != current:
            readme_path.write_text(expected)
            print("updated verticals/README.md")
        else:
            print("verticals/README.md already up to date")
        return 0

    if expected != current:
        diff = difflib.unified_diff(
            current.splitlines(),
            expected.splitlines(),
            fromfile="verticals/README.md",
            tofile="generated",
            lineterm="",
        )
        print("\n".join(diff), file=sys.stderr)
        fail("verticals/README.md generated table is stale")

    print("ok: verticals/README.md generated table is current")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
