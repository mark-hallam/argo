#!/usr/bin/env bash
# check-manager-package.sh — validate bounded manager-worker packages.
#
# Usage: scripts/check-manager-package.sh <brief-id>

set -uo pipefail

BRIEF_ID="${1:-}"
if [[ -z "$BRIEF_ID" ]]; then
  echo "usage: $0 <brief-id>" >&2
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LEDGER="$REPO_ROOT/docs/build-plan/BUILD_STATUS.md"
BRIEF="$REPO_ROOT/docs/build-plan/streams/$BRIEF_ID.md"
PACKAGE="$REPO_ROOT/docs/build-plan/work-packages/$BRIEF_ID.md"

python3 - "$BRIEF_ID" "$LEDGER" "$BRIEF" "$PACKAGE" <<'PY'
import fnmatch
import re
import sys
from pathlib import Path

brief_id, ledger_path, brief_path, package_path = sys.argv[1:5]
ledger = Path(ledger_path)
brief = Path(brief_path)
package = Path(package_path)
errors = []
warnings = []

def fail(msg):
    errors.append(msg)

def warn(msg):
    warnings.append(msg)

if not ledger.exists():
    fail(f"ledger missing: {ledger}")
if not brief.exists():
    fail(f"brief missing: {brief}")
if not package.exists():
    fail(f"manager package missing: {package}")

if errors:
    for e in errors:
        print(f"FAIL: {e}", file=sys.stderr)
    sys.exit(1)

ledger_text = ledger.read_text()
brief_text = brief.read_text()
package_text = package.read_text()

if not re.search(rf"^\| {re.escape(brief_id)} \|", ledger_text, re.M):
    fail(f"parent row not found in BUILD_STATUS.md: {brief_id}")

declared = re.search(r"^parent_brief:\s*([^\s]+)\s*$", package_text, re.M)
if not declared:
    fail("package missing parent_brief")
elif declared.group(1) != brief_id:
    fail(f"parent_brief mismatch: {declared.group(1)} != {brief_id}")

depth = re.search(r"^max_depth:\s*(\d+)\s*$", package_text, re.M)
if not depth:
    fail("package missing max_depth")
elif depth.group(1) != "1":
    fail(f"max_depth must be 1, got {depth.group(1)}")

budget = re.search(r"^spawn_budget:\s*(\d+)\s*$", package_text, re.M)
child_count = 0
child_ids = set()
in_children = False
for line in package_text.splitlines():
    if re.match(r"^children:\s*$", line):
        in_children = True
        continue
    if in_children and re.match(r"^\S", line) and not line.startswith("children:"):
        in_children = False
    child_match = re.match(r"^\s{2}- id:\s*([A-Za-z0-9_.-]+)\s*$", line)
    if in_children and child_match:
        child_count += 1
        child_ids.add(child_match.group(1))
if not budget:
    fail("package missing spawn_budget")
    spawn_budget = 0
else:
    spawn_budget = int(budget.group(1))
    if child_count > spawn_budget:
        fail(f"child count {child_count} exceeds spawn_budget {spawn_budget}")

mode = re.search(r"^\s{2}mode:\s*([A-Za-z0-9_.-]+)\s*$", package_text, re.M)
if mode and mode.group(1) not in {"automatic_if_supported", "manual"}:
    fail(f"unsupported spawn_policy.mode: {mode.group(1)}")

parallel = re.search(r"^\s{2}max_parallel_children:\s*(\d+)\s*$", package_text, re.M)
if parallel and int(parallel.group(1)) > spawn_budget:
    fail(
        f"max_parallel_children {parallel.group(1)} exceeds "
        f"spawn_budget {spawn_budget}"
    )

def extract_parent_may_touch(text):
    out = []
    in_section = False
    for line in text.splitlines():
        if line.startswith("## Files you may touch"):
            in_section = True
            continue
        if in_section and line.startswith("## "):
            break
        if not in_section:
            continue
        match = re.search(r"`([^`]+)`", line)
        if match:
            value = match.group(1).strip()
            value = value.replace("NEW: ", "").strip()
            if not value.startswith("<"):
                out.append(value)
    return out

def extract_child_allowed(text):
    values = []
    in_children = False
    collecting = False
    allowed_indent = 0
    for line in text.splitlines():
        if re.match(r"^children:\s*$", line):
            in_children = True
            continue
        if in_children and re.match(r"^\S", line) and not line.startswith("children:"):
            in_children = False
        if not in_children:
            continue
        if re.match(r"^\s+allowed_files:\s*$", line):
            collecting = True
            allowed_indent = len(line) - len(line.lstrip())
            continue
        if collecting:
            indent = len(line) - len(line.lstrip())
            if line.strip().startswith("- "):
                value = line.strip()[2:].strip().strip('"').strip("'")
                if value and not value.startswith("<"):
                    values.append(value)
                continue
            if line.strip() and indent <= allowed_indent:
                collecting = False
    return values

parents = extract_parent_may_touch(brief_text)
children = extract_child_allowed(package_text)
if not parents:
    warn("no parent may-touch entries parsed; subset check skipped")

def allowed(child, parent):
    child = child.strip()
    parent = parent.strip()
    if parent.endswith("/**"):
        return child.startswith(parent[:-3].rstrip("/") + "/")
    if parent.endswith("/"):
        return child.startswith(parent)
    if "*" in parent or "?" in parent or "[" in parent:
        return fnmatch.fnmatch(child, parent)
    return child == parent or child.startswith(parent.rstrip("/") + "/")

for child in children:
    if child == "docs/build-plan/BUILD_STATUS.md":
        fail("child allowed_files includes BUILD_STATUS.md")
    if parents and not any(allowed(child, parent) for parent in parents):
        fail(f"child file outside parent may-touch: {child}")

spawn_requests = []
in_spawn = False
current = None
for line in package_text.splitlines():
    if re.match(r"^spawn_requests:\s*$", line):
        in_spawn = True
        continue
    if in_spawn and re.match(r"^\S", line) and not line.startswith("spawn_requests:"):
        in_spawn = False
    if not in_spawn:
        continue
    new_req = re.match(r"^\s{2}- child_id:\s*([A-Za-z0-9_.-]+)\s*$", line)
    if new_req:
        current = {"child_id": new_req.group(1)}
        spawn_requests.append(current)
        continue
    field = re.match(r"^\s{4}([A-Za-z_]+):\s*([A-Za-z0-9_.-]+|null)\s*$", line)
    if current is not None and field:
        current[field.group(1)] = field.group(2)

if len(spawn_requests) > spawn_budget:
    fail(f"spawn request count {len(spawn_requests)} exceeds spawn_budget {spawn_budget}")

valid_statuses = {"pending", "spawned", "done", "failed", "manual_required"}
for req in spawn_requests:
    child_id = req["child_id"]
    if child_id not in child_ids:
        fail(f"spawn request references unknown child_id: {child_id}")
    if req.get("role") not in {None, "subworker"}:
        fail(f"spawn request for {child_id} has non-subworker role: {req.get('role')}")
    status = req.get("status", "pending")
    if status not in valid_statuses:
        fail(f"spawn request for {child_id} has invalid status: {status}")

for w in warnings:
    print(f"WARN: {w}", file=sys.stderr)
if errors:
    for e in errors:
        print(f"FAIL: {e}", file=sys.stderr)
    sys.exit(1)

print(
    f"manager package OK: {brief_id} ({child_count} child task(s), "
    f"{len(children)} child allowed file entrie(s), "
    f"{len(spawn_requests)} spawn request(s))"
)
PY
