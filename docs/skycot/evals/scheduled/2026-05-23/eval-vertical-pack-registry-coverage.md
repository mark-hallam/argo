# eval-vertical-pack-registry-coverage — `argo-vertical-packs` registry audit

**Author**: claude worker
**Round**: gap-manifest pickup (Claim 3 row `argo-vertical-packs`)
**Date**: 2026-05-23
**Brief**: `docs/build-plan/streams/eval-vertical-pack-registry-coverage.md`
**Scope (verbatim from brief)**: "List installed verticals on a fresh `~/.argo/`,
compare to `verticals/` subdir + marketplace catalog, surface drift."
**Mission claim served**: #3 — "The platform builds workers." Genesis +
marketplace need a coherent vertical-pack catalog so the worker that gets
hired is the worker the operator picked.

---

## TL;DR

🟡 **WIRE BROKEN — three parallel vertical-discovery surfaces with no
operational bridge between them.**

| Surface | Read by | Today's coverage on a fresh checkout |
|---|---|---|
| `~/.argo/packs/` cache (filled by `argo packs install`) | `PackLoader::list()` → `argo packs list` CLI only | **empty** (no auto-seed, no bootstrap, no default catalog) |
| Repo `verticals/` subdir | `argo_vertical::load_dir()` → `/api/v1/verticals` + hire wizard + workflow runner + worker portrait + 20+ other call sites | 36 subdirs; **32 loadable**, 4 demo-only stubs without `vertical.toml` |
| Repo `packs/` subdir | `argo_skill::lint_pack` (offline lint only) | 10 packs; **0 consumed by any runtime path** |

Three things to note:

1. The `argo-vertical-packs` crate is the substrate for downloading remote pack
   tarballs from `https://github.com/mark-hallam/skycot-packs/releases/download`
   (`crates/argo-vertical-packs/src/manifest.rs:147`). On a fresh `~/.argo/`,
   `argo packs list` returns `(no vertical packs installed)`
   (`crates/argo-cli/src/handlers/packs.rs:128`). There is no first-run install,
   no operator wizard step, and no manifest-of-manifests index file telling the
   loader which packs to seed.

2. The repo's `packs/` directory ships 10 pack-format trees
   (`pack.toml` + `worker.toml` + `skills/*.skill.json` + `evals/*.json` +
   `memory/{schema,seed}.json`). These match the `PackLoader::install`
   archive shape, but **nothing in the workspace ever installs them.** The
   sole production consumer is `argo_skill::pack_lint` for offline schema
   checks (`crates/argo-skill/src/pack_lint.rs:151`,
   `crates/argo-skill/src/pack_lint_tests.rs:236`).

3. The repo's `verticals/` directory ships a different template format
   (`vertical.toml` + `prompts/` + `memory/schema.toml` + `eval/*.jsonl`).
   This is what every production read-path uses today via
   `crate::handlers::vertical::resolve_verticals_dir`
   (`crates/argo-cli/src/handlers/vertical.rs:19`). 4 of 36 subdirectories are
   demo-only stubs that ship a `demo/learning_loop.sh` script but no
   `vertical.toml`, so `load_dir` silently skips them.

The net effect for an operator on a fresh install is:

- `argo packs list` → empty.
- `/api/v1/verticals` → 32 verticals (the ones with `vertical.toml` checked
  into the repo at `verticals/`), but only if the gateway is run from a
  working directory that contains the `verticals/` folder OR `ARGO_VERTICALS_DIR`
  is set. Outside the repo checkout, the gateway sees zero verticals.
- `argo packs install <name> --version 1.0.0` against the default base URL
  would download from `mark-hallam/skycot-packs` (private repo, requires
  `GH_TOKEN`/`GITHUB_TOKEN`), but even on success, the extracted pack root
  contains `worker.toml` (not `vertical.toml`) — so the gateway's vertical
  loader would not pick it up even if `~/.argo/packs/<name>/<version>/` were
  added to `ARGO_VERTICALS_DIR`.

---

## What "drift" means here (and why this audit is FINDINGS-ONLY rather
than fix-now)

The brief sits in the 🟡 P2 lane of the gap manifest (Claim 3 row
`argo-vertical-packs`) because none of the gaps named above are
ship-blockers for the v1 demo — the demo runs the gateway from the repo
checkout where `./verticals/` is already on disk. The "registry drift"
gaps are about what an external customer sees on Day 1 with a freshly
installed binary, which is not yet a v1.0 user journey.

This audit pins the current behavior with one LIVE structural test and a
set of `#[ignore]`'d FINDINGS-ONLY assertions documenting the gaps the
follow-up briefs should close. No code is moved; no schemas are renamed;
no callers are added.

---

## §1 — Three discovery surfaces, side by side

### 1a · `~/.argo/packs/` (the remote-marketplace cache)

**Owner**: `argo-vertical-packs` crate (5 files, 934 LOC under `src/`).

**Entry point**: `PackLoader::default_loader()` →
`PackLoaderConfig::default_from_env()` → `~/.argo/packs`
(`crates/argo-vertical-packs/src/loader.rs:48`).

**Bootstrap behavior**: `PackLoader::list()` is a passive `read_dir` over
`<cache_root>/<name>/<version>/pack.toml` — no defaults, no bundled
manifests, no fallback to `packs/` in the repo
(`crates/argo-vertical-packs/src/loader.rs:147`).

**Consumers in production code**:
- `crates/argo-cli/src/handlers/packs.rs` — the `argo packs {install,list,update,dev-link,remove}` CLI surface and **only** consumer of `PackLoader`.

No gateway route, no hire wizard, no agent runner, no genesis path, no
workflow loader consumes `PackLoader`. `rg -n 'PackLoader' crates/` returns
exactly the `packs.rs` line plus `argo-vertical-packs`' own internal uses.

**Marketplace catalog source**: the default `FetcherConfig::base_url` is
`https://github.com/mark-hallam/skycot-packs/releases/download`
(`crates/argo-vertical-packs/src/manifest.rs:147`). The repo is **private**
during the build phase (operator-confirmed 2026-05-14). No index endpoint,
no `GET /packs` route on the gateway, no `argo packs search` subcommand —
operators must already know the `<name> <version>` pair before calling
`argo packs install`.

### 1b · Repo `verticals/` (the production discovery surface)

**Owner**: `argo-vertical` crate.

**Entry point**: `argo_vertical::default_dir()` returns
`$ARGO_VERTICALS_DIR` or `./verticals` (cwd-relative)
(`crates/argo-vertical/src/lib.rs:89`).

**Inventory** (filed against `verticals/<id>/` subdirs):

| Subdir | `vertical.toml`? | Notes |
|---|---|---|
| `_starter-library` | ✅ | Template scaffold (intentionally not a shipped vertical). |
| `ap-specialist` | ❌ | Demo script only (`demo/learning_loop.sh`). |
| `bookkeeping-assistant` | ✅ | |
| `customer-onboarding` | ✅ | |
| `customer-support` | ✅ | |
| `data-entry-clerk` | ✅ | |
| `deal-desk` | ❌ | Demo script only. |
| `event-coordinator` | ✅ | |
| `executive-assistant` | ✅ | |
| `finance-ar` | ✅ | |
| `fundraising-coordinator` | ✅ | |
| `gtm-engineer` | ✅ | |
| `healthcare-intake` | ✅ | |
| `hr-screener` | ✅ | |
| `lead-research` | ✅ | |
| `legal-paralegal` | ✅ | |
| `marketing-intern` | ✅ | |
| `marketplace-seller` | ✅ | |
| `marketplace-seller-browser` | ✅ | |
| `medical-coder` | ✅ | |
| `ops-on-call` | ✅ | |
| `pr-checker` | ✅ | |
| `procurement-assistant` | ✅ | |
| `product-manager` | ✅ | |
| `qa-engineer` | ✅ | |
| `real-estate-agent` | ✅ | |
| `receptionist` | ✅ | |
| `recruiter` | ✅ | |
| `recruiter-screener` | ✅ | |
| `research-analyst` | ✅ | |
| `sales-follow-up` | ✅ | |
| `sales-sdr` | ✅ | |
| `social-media-drafter` | ✅ | |
| `support-tier1` | ❌ | Demo script only. |
| `tax-prep` | ✅ | |
| `vendor-manager` | ❌ | Demo script only. |

**32 loadable, 4 demo-stubs, 36 total directories.**

Side gap: `verticals/README.md`'s "Verticals shipped today" table lists
only 4 verticals (pr-checker, receptionist, gtm-engineer, sales-follow-up)
when the current count is 32. The README is **stale by 28 entries.**

### 1c · Repo `packs/` (the in-tree pack distribution prototype)

**Owner**: nominally `argo-skill::pack_lint`, but the directory itself has
no declared owner crate.

**Inventory** (filed against `packs/<name>/pack.toml`):

| Pack | Version | `worker.toml`? | Loadable as a vertical via `argo_vertical::load_dir`? |
|---|---|---|---|
| `ap-specialist` | 1.0.0 | ✅ | ❌ — no `vertical.toml` |
| `customer-support` | 1.0.0 | ✅ | ❌ |
| `deal-desk` | 1.0.0 | ✅ | ❌ |
| `finance-ar` | 1.0.0 | ✅ | ❌ |
| `marketing-intern` | 1.0.0 | ❌ | ❌ |
| `project-coordinator` | 1.0.0 | ✅ | ❌ |
| `recruiter` | 1.0.0 | ✅ | ❌ |
| `sales-sdr` | 1.0.0 | ❌ | ❌ |
| `support-tier1` | 1.0.0 | ✅ | ❌ |
| `vendor-manager` | 1.0.0 | ✅ | ❌ |

Every `pack.toml` here declares `[[verticals]] id = <name>, path = "."`
(see `packs/recruiter/pack.toml:13-15`) — a claim that "this pack ships
one vertical at the pack root." But the pack root contains `worker.toml`,
not `vertical.toml`. The schemas are not compatible: `worker.toml`
declares `[worker]`/`[escalation]`/`[policy]`/`[memory]`/`[tools]`
(see `packs/recruiter/worker.toml:1-40`); `vertical.toml` declares
`[vertical]`/`[role]`/`[provider]`/`[tools]`/`[memory]`/`[evaluation]`
(see `verticals/customer-support/vertical.toml:1-30`).

**Two pack ids are NOT covered by the `verticals/` subdir at all** —
even with the schema gap aside, the directories simply don't both exist:

| Pack id | `verticals/<id>/`? | `verticals/<id>/vertical.toml`? |
|---|---|---|
| `ap-specialist` | yes | ❌ demo-stub |
| `deal-desk` | yes | ❌ demo-stub |
| `support-tier1` | yes | ❌ demo-stub |
| `vendor-manager` | yes | ❌ demo-stub |
| `project-coordinator` | ❌ | n/a |
| others (`customer-support`, `finance-ar`, `marketing-intern`, `recruiter`, `sales-sdr`) | yes | ✅ |

So `packs/project-coordinator` is a pack with **no corresponding vertical
template** anywhere in the repo. (Project & Ops cohorts memory
`project_pm_cohorts_peta_patrick.md` confirms Peta is the first PM worker
and was added 2026-05-19; the vertical-template side appears to not have
caught up.)

---

## §2 — What `argo packs list` shows on a fresh install

The cache is `~/.argo/packs`. `PackLoader::list` returns an empty `Vec`
when `cache_root` doesn't exist or contains no `<name>/<version>/pack.toml`
trios (`crates/argo-vertical-packs/src/loader.rs:147`). The CLI handler
prints `(no vertical packs installed)` for the empty case
(`crates/argo-cli/src/handlers/packs.rs:128`).

There is no bundled `~/.argo/packs/.bootstrap.toml` or equivalent
auto-install manifest. No code path in the workspace calls
`PackLoader::install` outside of `argo packs install <name> --version <v>`,
and no production caller of `PackLoader::install` is wired into install,
onboarding, hire, or first-run flows. `rg -n 'PackLoader::install' crates/`
returns:

```
crates/argo-cli/src/handlers/packs.rs:67     row = loader.install(&name, ...)
crates/argo-cli/src/handlers/packs.rs:79     row = loader.install(&name, ...)
crates/argo-vertical-packs/src/loader.rs:85  pub async fn install(...)
crates/argo-vertical-packs/src/loader.rs:367 (test)
crates/argo-vertical-packs/src/loader.rs:386 (test)
```

—just the two CLI handlers (Install, Update) and the crate's own unit
tests.

---

## §3 — Findings & fix-brief shapes

### Finding F-1 — 🟡 P2 — `~/.argo/packs/` has no bootstrap; remote marketplace has no listing

**Surface**: `PackLoader::list()` empty on Day 1; no `argo packs search` /
`GET /api/v1/packs` endpoint; the only way to find a pack name is to read
the README of the private `mark-hallam/skycot-packs` repo.

**Why it matters**: An operator who hires a worker via the wizard and
chooses, say, "recruiter" is served from `verticals/recruiter/` baked into
the binary's working directory. They never touch the marketplace. The
marketplace is currently a code path with no entrance.

**Fix-brief shape**: `mission-packs-marketplace-listing-endpoint` —
land a workspace-internal pack-index file (or a thin scan of the remote
GitHub releases) and surface it under `GET /api/v1/packs` so the hire
wizard can offer "install another vertical" without operators having to
know pack names by memory.

### Finding F-2 — 🟡 P2 — `PackLoader::install` produces directories the vertical loader cannot read

**Surface**: a pack archive's contents (`pack.toml` + `worker.toml` + `skills/*.skill.json` + …) do not contain a `vertical.toml`, so even if the operator runs `argo packs install recruiter --version 1.0.0` and points `ARGO_VERTICALS_DIR` at the cache, `argo_vertical::load_dir` will silently skip the directory.

**Why it matters**: The two template formats represent two design generations of the same idea. The newer `worker.toml` shape encodes "policy + approval + tools" cleanly; the older `vertical.toml` shape encodes "prompts + provider + KPIs." The mission claim "the platform builds workers" needs ONE format, not two — and the runtime needs to load packs from the marketplace cache, not just from the repo checkout.

**Fix-brief shape**: `mission-pack-format-adapter-to-vertical-loader` — either (a) compile a `worker.toml`-style pack into a synthetic `vertical.toml` on install so the existing loader picks it up unchanged, or (b) extend `argo_vertical::load_dir` to recognize `worker.toml` packs as an alternate manifest. Operator-decisional which direction wins; both are tractable. Until then, the marketplace cache stays operationally invisible.

### Finding F-3 — 🟡 P3 — 4 demo-only stubs sit in `verticals/` without manifests

**Surface**: `verticals/ap-specialist/`, `verticals/deal-desk/`,
`verticals/support-tier1/`, `verticals/vendor-manager/` each contain only
a `demo/learning_loop.sh`. `argo_vertical::load_dir` silently skips them
(per `verticals/README.md:15-17`). The repo `packs/` side has
*matching-named* pack trees for 3 of the 4, suggesting the demo dirs
predate the pack split and never migrated.

**Why it matters**: A reader of `verticals/` cannot tell from `ls` which
directories are real templates and which are stubs. The 32-vs-36 gap
makes the directory's "what ships" question harder than it should be.

**Fix-brief shape**: `verticals-demo-stubs-promote-or-archive` — for
each of the 4 stubs, either (a) author a `vertical.toml` from the
corresponding `packs/<name>/worker.toml` and inline prompts, or (b) move
the demo script under `verticals/_demos/<name>/learning_loop.sh` so the
top-level listing reflects only loadable verticals. Operator picks.

### Finding F-4 — 🟡 P3 — `verticals/README.md` table is stale by 28 entries

**Surface**: the "Verticals shipped today" table at
`verticals/README.md:55-60` still lists only the original 4 (pr-checker,
receptionist, gtm-engineer, sales-follow-up); the loader actually finds
32. The table cites PR numbers `#652`/`#657`/`#732`/`#746` that all
landed pre-2026-04. Every vertical added since is invisible to a reader
of the README.

**Fix-brief shape**: `docs-verticals-readme-table-from-load-dir` —
generate the README table from `cargo run --bin argo -- vertical list
--json` at doc-test time so the README cannot drift again.

### Finding F-5 — 🟡 P3 — `packs/project-coordinator` has no `verticals/project-coordinator/` counterpart

**Surface**: `packs/project-coordinator/` (Peta — see operator memory
`project_pm_cohorts_peta_patrick.md`, 2026-05-19) ships a pack-format
worker but the vertical-format `verticals/project-coordinator/` directory
does not exist. Operators on a repo-checkout install cannot hire Peta via
the wizard until either the pack format is wired into the loader (F-2) or
a parallel `vertical.toml` is authored.

**Fix-brief shape**: covered by F-2's chosen direction — adapter wins,
the pack becomes operator-visible automatically; vertical.toml wins, this
specific pack needs a sibling vertical authored.

---

## §4 — Pin tests shipped with this audit

In `crates/argo-vertical-packs/tests/registry_coverage_pin_test.rs`:

1. **LIVE**: `pack_loader_fresh_cache_returns_no_installed_packs` — pins
   the "empty on Day 1" behavior so a future bootstrap that breaks this
   contract gets caught.

2. **LIVE**: `repo_packs_all_pack_toml_round_trip_through_pack_manifest`
   — pins that every `packs/*/pack.toml` parses cleanly through
   `PackManifest::load`. Acts as a structural inventory: future-author of
   a new pack who breaks the schema gets a fast local failure rather than
   a CI surprise.

3. **`#[ignore]`'d FINDINGS-ONLY** —
   `repo_pack_ids_have_loadable_vertical_counterpart`: every
   `packs/<name>/pack.toml` `[[verticals]] id` has a corresponding
   `verticals/<id>/vertical.toml` on disk. **Fails today** for
   `ap-specialist`, `deal-desk`, `support-tier1`, `vendor-manager`,
   `project-coordinator`. Flip to live once F-2 lands.

4. **`#[ignore]`'d FINDINGS-ONLY** —
   `verticals_subdirs_all_have_manifest_or_demo_marker`: every directory
   under `verticals/` either ships a `vertical.toml` or sits under
   `verticals/_demos/` (or `_starter-library/`). **Fails today** for the
   4 demo-only stubs. Flip to live once F-3 lands.

---

## §5 — What we did NOT audit (deliberate scope cuts)

- **Network behavior of `GitHubReleaseFetcher`.** The fetcher uses
  `reqwest::Client::new()` with bearer-auth + status check
  (`crates/argo-vertical-packs/src/fetcher.rs:53`); we did not exercise it
  against the real private repo because `GH_TOKEN` is operator-only.
- **Tar archive safety beyond the existing `safe_extract_tar_gz` guard.**
  The crate already rejects parent-dir / absolute-path entries
  (`crates/argo-vertical-packs/src/loader.rs:243-269`). We did not write a
  fuzzer; the existing logic is small and obvious.
- **Per-tenant pack scoping.** `PackLoaderConfig` has no `tenant_id`
  field, but `~/.argo/packs/` is a per-operator-machine cache, not a
  multi-tenant server-side store — it sits at the wrong layer for tenant
  isolation to matter. If/when the marketplace cache moves server-side,
  this becomes a P1 audit; today it's correctly out of scope.

---

## §6 — Gap manifest update

The row for `argo-vertical-packs` in
`docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
flips from 🔴 NOT AUDITED to 🟡 AUDITED — substrate present, **wire
broken between cache and loader.** 5 fix-briefs queued (F-1 through F-5).
Net new P1 surfaced: F-2 (pack format adapter). Net new P2 surfaced: F-1
(marketplace listing endpoint). F-3/4/5 are P3 housekeeping.

The "drift" the brief asked to surface is real and named in 5 specific,
authorable fix-briefs. No code is moved by this audit; the pin tests
freeze today's behavior and the `#[ignore]`'d tests document the
expected end-state.
