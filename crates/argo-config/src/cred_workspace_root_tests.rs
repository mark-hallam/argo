//! Unit tests for [`crate::cred::workspace_root`].
//!
//! Pins the durable-volume credential-path fix: in production the vault
//! base must resolve to `$ARGO_WORKSPACE` (the `/data` durable mount) so
//! connector credentials persist across task restarts/redeploys instead
//! of landing on the throwaway container image layer. Local dev (no env
//! set) keeps the historical `current_dir()` behaviour.

use super::*;
use std::sync::Mutex;

/// Serializes the env-var mutations below so concurrent tests in this
/// crate cannot observe a half-set `ARGO_WORKSPACE` / `ARGO_HOME`.
static ENV_GUARD: Mutex<()> = Mutex::new(());

/// Snapshot + clear both knobs; restore on drop so the test never leaks
/// state into sibling tests regardless of the assertion outcome.
struct EnvRestore {
    workspace: Option<String>,
    home: Option<String>,
}

impl EnvRestore {
    fn take() -> Self {
        let workspace = env::var("ARGO_WORKSPACE").ok();
        let home = env::var("ARGO_HOME").ok();
        env::remove_var("ARGO_WORKSPACE");
        env::remove_var("ARGO_HOME");
        Self { workspace, home }
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        match &self.workspace {
            Some(v) => env::set_var("ARGO_WORKSPACE", v),
            None => env::remove_var("ARGO_WORKSPACE"),
        }
        match &self.home {
            Some(v) => env::set_var("ARGO_HOME", v),
            None => env::remove_var("ARGO_HOME"),
        }
    }
}

#[test]
fn workspace_root_prefers_argo_workspace() {
    let _lock = ENV_GUARD.lock().unwrap();
    let _restore = EnvRestore::take();

    env::set_var("ARGO_WORKSPACE", "/some/dir");
    // ARGO_HOME also set, to prove ARGO_WORKSPACE wins over it.
    env::set_var("ARGO_HOME", "/other/home");

    assert_eq!(workspace_root(), PathBuf::from("/some/dir"));
}

#[test]
fn workspace_root_falls_back_to_argo_home() {
    let _lock = ENV_GUARD.lock().unwrap();
    let _restore = EnvRestore::take();

    // ARGO_WORKSPACE unset → ARGO_HOME is the next precedence rung.
    env::set_var("ARGO_HOME", "/home/from/env");

    assert_eq!(workspace_root(), PathBuf::from("/home/from/env"));
}

#[test]
fn workspace_root_ignores_empty_env_and_falls_back_to_cwd() {
    let _lock = ENV_GUARD.lock().unwrap();
    let _restore = EnvRestore::take();

    // Empty values must be treated as "not set" (non-empty filter), so
    // resolution falls through to current_dir — the local-dev default.
    env::set_var("ARGO_WORKSPACE", "");
    env::set_var("ARGO_HOME", "");

    assert_eq!(workspace_root(), env::current_dir().unwrap());
}

#[test]
fn workspace_root_falls_back_to_cwd_when_unset() {
    let _lock = ENV_GUARD.lock().unwrap();
    let _restore = EnvRestore::take();

    // Neither knob set → current working directory (local dev).
    assert_eq!(workspace_root(), env::current_dir().unwrap());
}
