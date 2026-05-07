# V3.0 prep — Plugin System Design Doc

**Owner**: claude. **Depends on**: nothing (design-doc only,
no code). Lays groundwork for V3.x extensibility — pluggable
executors, NotifyChannels, and Genesis templates so third
parties can extend Argo without forking the workspace.

## Scope

DESIGN-DOC ONLY. No code lands in this brief. Output is a
single markdown design doc that the next implementation
brief(s) will reference. The doc must answer:

1. **Extension points** — what can a plugin extend?
   - NEW executor kinds (e.g. third-party connector
     like a vertical-specific SaaS API)
   - NEW NotifyChannel variants (e.g. plugin-defined
     approval routing — Teams, Discord)
   - NEW Genesis templates (vertical packs distributed
     out-of-band)
   - Explicitly OUT of scope for V3.x: pluggable LLM
     providers, pluggable storage backends, pluggable
     audit sinks (these stay in-tree).

2. **Loading model** — how does a plugin get into the
   binary?
   - Decision needed: dynamic load (libloading / dylib),
     static cargo feature, or out-of-process subprocess.
   - Doc must lay out trade-offs and recommend ONE.
   - ABI stability implications of each.

3. **Versioning + ABI** — how do we promise compatibility?
   - SemVer rules for the plugin trait crate (e.g.
     `argo-plugin-api`)
   - Breaking-change policy
   - Compatibility matrix shape (plugin X works with
     gateway versions Y..=Z)

4. **Security model** — what can a plugin do?
   - Permissions a plugin declares at load time
     (network egress, fs access, executor namespaces)
   - Tenant isolation — plugins run per-tenant or
     globally?
   - Code-signing requirements (if any)
   - Failure isolation — plugin panic must not take down
     the gateway

5. **Discovery + distribution** — how do operators find
   plugins?
   - Plugin registry shape (JSON manifest, similar to
     the skill marketplace)
   - Install mechanism (CLI command? part of marketplace?)
   - Sandboxing posture (V3.0 may ship with no sandbox;
     documented risk)

6. **Migration path** — how do existing in-tree executors
   relate?
   - Do we extract any current executors (gmail, slack,
     stripe) to plugins to dogfood the API? Recommend.
   - Backward compatibility for in-tree code paths.

## What lands

- `docs/skycot/PLUGIN_SYSTEM.md` (NEW, ≤ 500 LOC) — the
  design doc covering all 6 sections above
- README link from `docs/skycot/README.md` adding the new
  doc to the index (DELTA only)

## Acceptance criteria
- [ ] Doc exists at `docs/skycot/PLUGIN_SYSTEM.md`
- [ ] All 6 sections present and substantive
- [ ] Loading-model decision is RECOMMENDED (not just
      enumerated) with trade-off table
- [ ] Versioning + ABI policy is concrete enough that an
      implementation brief could derive crate boundaries
      from it
- [ ] Security model identifies the V3.0 minimum bar AND
      the V3.x stretch bar (sandboxing)
- [ ] Migration recommendation states which (if any)
      current executors get extracted as the dogfood case
- [ ] Doc renders cleanly (no broken markdown)
- [ ] Listed from `docs/skycot/README.md`
- [ ] No code changes outside docs/

## Branch + PR
- Branch: `claude-plugin-system-design`
- PR title: `docs: Plugin system design doc (claude-plugin-system-design)`

## Notes
- This brief is intentionally design-only. Reviewers
  approving it are committing to the SHAPE of the plugin
  system, not the implementation.
- After merge, the next brief in this line is
  `claude-plugin-api-crate` which extracts
  `argo-plugin-api` per this doc's recommendations.
- The loading-model recommendation is the single most
  important decision in this doc. Author should pre-write
  the trade-off table before drafting prose.
- Reference reading: how cargo, helix, neovim, and obsidian
  do plugins. Cite each in a "prior art" appendix.
- Word target: 2000-3000 words. Stay under the 500-LOC
  ceiling (markdown lines, not words).
