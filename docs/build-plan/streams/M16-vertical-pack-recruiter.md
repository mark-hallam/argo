# M16-vertical-pack-recruiter - Track M (M-Tmpl), Phase 3, Tier 2

**Owner**: claude. **Depends on**: M15 MERGED; operator pre-req: skycot-packs repo exists.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M16-vertical-pack-recruiter`.

## Scope

The Recruiter vertical pack. Worker manifest defines role: tone, escalation policy, default_display_name=Recruiter (overridable by tenant on hire per M0 section 5 naming convention). 5 skills: screen-candidate, schedule-screen, draft-rejection, source-from-linkedin, extract-resume-fields. Memory seeded with screening rubric. Tools: Greenhouse/Lever/LinkedIn (MCP), Calendar, Slack approvals. 25+ eval fixtures.

LOC: ~2000 (mostly data + skills). Branch: `claude-M16-vertical-pack-recruiter`. PR title: `feat(packs): Recruiter vertical pack (M16)`.

## Files you may touch

- NEW packs/recruiter/ directory (or in separate skycot-packs repo if linked)
- - pack.toml, worker.toml
- - skills/screen-candidate.skill.json
- - skills/schedule-screen.skill.json
- - skills/draft-rejection.skill.json
- - skills/source-from-linkedin.skill.json
- - skills/extract-resume-fields.skill.json
- - memory/seed.json, memory/schema.json
- - evals/*.json (>=25 fixtures)
- - README.md

## Files you must NOT touch

- M15 loader internals; L1/L2 schemas.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Pack installs cleanly on fresh tenant.
- [ ] The Recruiter pack runs end-to-end: receives a JD + 10 resumes, screens, schedules qualifying candidates, drafts rejections for the rest.
- [ ] Eval suite green on golden fixtures.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M16-vertical-pack-recruiter`
- PR title: `feat(packs): Recruiter vertical pack (M16)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 1 + 3 (Teammate personas + platform vertical packs).
