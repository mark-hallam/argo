# V1.4 — macOS installer

**Owner**: claude. **Depends on**: nothing.

## Scope

Today operators must `cargo install` from source — 10+ minute
build, requires Rust toolchain. Ship a macOS-native install
path:

1. **Homebrew formula** at `Formula/argo.rb` in a `homebrew-argo`
   tap. Operator runs `brew tap mark-hallam/argo && brew install
   argo`.
2. **Signed `.dmg`** built via GitHub Actions on each release
   tag. Includes argo binary + a uninstaller script + post-install
   hook to suggest `argo onboard`.
3. **launchd plist** at `~/Library/LaunchAgents/com.skycot.argo.plist`
   installed via `argo daemon install`. Starts gateway on boot.

## Files you may touch
- `Formula/argo.rb` (NEW — the Homebrew formula; lives in repo
  for now, can be moved to a tap later)
- `installer/macos/build-dmg.sh` (NEW — packaging script)
- `installer/macos/post-install.sh` (NEW — hint to run argo onboard)
- `installer/macos/com.skycot.argo.plist` (NEW — launchd template)
- `crates/argo-cli/src/handlers/daemon_install.rs` (NEW)
- `crates/argo-cli/src/handlers/daemon_uninstall.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 2 pub mods
- `crates/argo-cli/src/cli_daemon_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: register
  `Daemon` subcommand
- `.github/workflows/release.yml` (NEW or extend) — build .dmg
  on release tag

## Acceptance criteria
- [ ] `brew install` works from local tap
- [ ] `.dmg` builds via release workflow + is code-signed (or
      ad-hoc signed if no Apple Developer cert; document the
      gap)
- [ ] `argo daemon install` writes the launchd plist + loads it
- [ ] `argo daemon uninstall` cleans up
- [ ] `--json` envelope on daemon subcommands
- [ ] 6 unit tests (plist gen + path resolution) + 1 CI test
      that runs `brew install` against the formula
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-installer-macos`
- PR title: `dist: macOS Homebrew formula + .dmg + launchd integration (claude-installer-macos)`
