# V1.4 — RUNBOOK troubleshooting expansion

**Owner**: claude. **Depends on**: nothing (RUNBOOK is the
canonical operator runbook today).

## Why

`docs/skycot/RUNBOOK.md § troubleshooting` covers the
happy-path failures (worker hung, channel auth expired). The
five most common operator-reported confusions during the
SKYCOT swarm sprint are not yet documented:

1. **Schema version mismatch** — vertical TOML written
   against an older schema, executor refuses to load.
2. **Port 7090 already bound** — gateway fails to start on
   default port (commonly: another `argo` process, or
   another tool squatting on the port).
3. **`ANTHROPIC_API_KEY` missing** — clear-text symptom
   ("model call failed: unauthenticated") that doesn't yet
   point at the env-var fix.
4. **Channel not configured** — task references a channel id
   that has no binding; current error is opaque.
5. **Executor kind unknown** — vertical references an
   executor variant that's been renamed/removed; error
   surfaces as a generic "no executor for kind X".

Each is a 30-second fix once you know what to look at. Each
is a 30-minute slog when you don't. RUNBOOK is the right home.

## Scope

Narrow extension of `docs/skycot/RUNBOOK.md § troubleshooting`
adding one subsection per error above. Each subsection has:

- **Symptom** — exact error text or log line the operator
  will see (verbatim, copy-pasteable for grep).
- **Cause** — one-paragraph explanation in operator language,
  no Rust internals.
- **Fix** — numbered steps, copy-paste commands, expected
  outcome.
- **Related** — cross-link to relevant doc (CLI reference,
  channels guide, etc.).

Symptoms are pulled from real current output — verify each
by triggering the failure locally before writing it down.

Mission paragraph + 6 claims block in MISSION.md is
**immutable** and out of scope.

## Files you may touch
- `docs/skycot/RUNBOOK.md` (narrow extension — append to
  troubleshooting section, do not refactor existing prose)

## Files you must NOT touch
- `docs/skycot/MISSION.md` (immutable mission paragraph)
- Any `crates/**` Rust source (don't change error text to
  match the doc; document what the binary actually emits)
- `book/**` (mdBook ingests RUNBOOK by reference; no direct
  edit needed there)
- Other `docs/**` files

## Acceptance criteria
- [ ] All five subsections present with Symptom / Cause /
      Fix / Related blocks
- [ ] Every "Symptom" string matches verbatim what the
      current binary emits (verify each — `argo run` with the
      env var unset, etc.)
- [ ] Every "Fix" command is paste-runnable
- [ ] No edits outside the troubleshooting section of
      RUNBOOK.md (PR diff confirms)
- [ ] Existing troubleshooting subsections untouched
- [ ] Cross-links to CLI reference / channels / etc. resolve
      in the rendered mdBook output (`mdbook test` passes)
- [ ] Total line addition ≤ ~150 lines (concise, scannable)

## Branch + PR
- Branch: `claude-troubleshooting-guide-expand`
- PR title: `docs(runbook): expand troubleshooting (schema/port/key/channel/executor) (claude-troubleshooting-guide-expand)`

## Notes

Verify the error strings *before* writing each section.
Reproducing each failure takes a couple of minutes:

- Schema mismatch: hand-edit a TOML to bump
  `schema_version` to a value the binary doesn't know.
- Port bind: run `argo gateway` twice in two terminals.
- API key: `unset ANTHROPIC_API_KEY; argo run ...`.
- Channel not configured: write a task referencing a
  channel id with no binding.
- Executor unknown: edit a vertical TOML to set
  `executor.kind = "nonsense"`.

If any error message is too vague to be useful (e.g. just
"failed"), don't paper over it in docs — flag it in the PR
description as a candidate for a separate brief that improves
the binary's error text. Documentation cannot fix bad errors.

Keep the tone neutral and operator-focused. No marketing prose
("Argo is delightful when..."). Operators reading RUNBOOK are
already frustrated; they want the fix.
