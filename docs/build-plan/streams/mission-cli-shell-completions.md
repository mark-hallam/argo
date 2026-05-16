# mission-cli-shell-completions — `argo completions <shell>` (post-v1 polish P2)

**Owner**: codex. **Depends on**: none (single-crate, mechanical).
**Authored against**: origin/main `223d3fab` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cli-shell-completions`.
**Source**: post-v1 polish — operator UX. Standard ship-with-CLI feature.

## Mission alignment

- **Repeatability** (CLAUDE.md priority #3) — `argo` is a `clap`-driven CLI; `clap_complete` generates completions for free.
- **Commercial usability** (#5) — `tab`-completing `argo audit verify-chain --tenant <TAB>` is the difference between adopted and abandoned CLIs.

## Scope

P2. New subcommand `argo completions <bash|zsh|fish|powershell|elvish>` that prints the completion script for the named shell to stdout.

Standard usage:
```bash
argo completions bash > /usr/local/etc/bash_completion.d/argo
argo completions zsh  > "${fpath[1]}/_argo"
argo completions fish > ~/.config/fish/completions/argo.fish
```

LOC: ~150.

## Files you may touch

- `crates/argo-cli/Cargo.toml` — narrowly: add `clap_complete = "4"` dependency. ≤3 modified LOC.
- `crates/argo-cli/src/cli_command_enum.rs` — add `Completions { shell: clap_complete::Shell }` variant. ≤8 modified LOC.
- `crates/argo-cli/src/handlers/completions.rs` (NEW) — handler. Calls `clap_complete::generate(shell, &mut Cli::command(), "argo", &mut io::stdout())`. ≤30 LOC.
- `crates/argo-cli/src/handlers/mod.rs` — register handler. ≤3 modified LOC.
- `crates/argo-cli/src/handlers/completions_tests.rs` (NEW) — verify each shell produces non-empty output containing the binary name. ≤40 LOC.
- `docs/skycot/cli-reference.md` — add `completions` row (the `cli-reference-drift.yml` CI workflow will lint this; reflect the new subcommand). ≤6 modified LOC.
- `docs/skycot/operator/install.md` — add a "shell completions" section under installation. ≤25 modified LOC.

## Files you must NOT touch

- Any non-CLI crate. Completions are pure clap surface.
- The `Cli` derive-impl macro — clap_complete consumes `Cli::command()` directly.

## Acceptance criteria

- [ ] `argo completions bash` prints a bash completion script (non-empty, contains `_argo` function name).
- [ ] `argo completions zsh` prints zsh script (contains `#compdef argo`).
- [ ] `argo completions fish` prints fish script (contains `complete -c argo`).
- [ ] `argo completions powershell` and `argo completions elvish` also produce output.
- [ ] Generated script for bash is shellcheck-clean (run `argo completions bash | shellcheck -`).
- [ ] `cargo test -p argo-cli completions` passes (3-5 tests, one per shell).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cli-reference-drift.yml` CI workflow passes after the doc update.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-cli-shell-completions`
- Title: `feat(cli): argo completions <shell> via clap_complete (mission-cli-shell-completions)`
- Worker verification: crate-scoped clippy + tests + `cargo clippy --workspace` per SWARM_PROTOCOL.md.

## Hidden constraints

- **`clap_complete::Shell` is value_enum** — clap will auto-validate shell names. Don't write your own string-matching.
- **Completion scripts are stdout, not stderr** — operator pipes to file.
- **No `eprintln!` warnings in the output** — that breaks `> file` redirection.
- **Cargo.lock change is unavoidable** — adding clap_complete updates lock. Commit it.
