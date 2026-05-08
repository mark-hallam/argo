#!/usr/bin/env bash
# Enforce workspace dependency pinning for crate manifests.
#
# Rule: if a dependency name exists in root [workspace.dependencies],
# each crates/*/Cargo.toml entry for that dependency must inherit it
# with `workspace = true` instead of spelling a literal version.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

python3 - "$REPO_ROOT" <<'PY'
from __future__ import annotations

import pathlib
import sys

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover - local Python fallback
    try:
        import tomli as tomllib
    except ModuleNotFoundError:
        print(
            "FAIL: Python needs tomllib (3.11+) or tomli to parse Cargo.toml",
            file=sys.stderr,
        )
        sys.exit(2)


ROOT = pathlib.Path(sys.argv[1])
TABLES = ("dependencies", "dev-dependencies", "build-dependencies")


def load_toml(path: pathlib.Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


root_manifest = load_toml(ROOT / "Cargo.toml")
workspace_deps = set(root_manifest.get("workspace", {}).get("dependencies", {}))
if not workspace_deps:
    print("FAIL: root Cargo.toml has no [workspace.dependencies]", file=sys.stderr)
    sys.exit(2)

violations: list[str] = []
for manifest in sorted((ROOT / "crates").glob("*/Cargo.toml")):
    data = load_toml(manifest)
    crate = data.get("package", {}).get("name", manifest.parent.name)
    rel = manifest.relative_to(ROOT)
    for table in TABLES:
        for dep, spec in data.get(table, {}).items():
            if dep not in workspace_deps:
                continue
            if isinstance(spec, dict) and spec.get("workspace") is True:
                continue
            violations.append(
                f"crate={crate} manifest={rel} table={table} "
                f"dep={dep} violation=literal-version-in-presence-of-workspace-pin"
            )

if violations:
    print("Workspace dependency pinning violations:")
    for violation in violations:
        print(violation)
    print("\nUse `<dep>.workspace = true` for deps pinned in root Cargo.toml.")
    sys.exit(1)

print(f"OK: {len(workspace_deps)} workspace dependency pins enforced across crates/*")
PY
