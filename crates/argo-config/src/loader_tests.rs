//! Unit tests for [`crate::loader`].
//!
//! Lives in its own file so [`crate::loader`] stays under the
//! 500-LOC ceiling; brief allows new `loader_test` files.
use super::*;
use crate::ArgoConfig;

#[test]
fn env_overrides_take_precedence_over_toml() {
    let dir = tempfile::tempdir().unwrap();
    let toml_path = dir.path().join("argo.toml");
    std::fs::write(
        &toml_path,
        "[gateway]\nport = 9999\nurl = \"http://from-toml/\"\n",
    )
    .unwrap();

    scoped_env(
        &[
            ("ARGO_GATEWAY_PORT", Some("4242")),
            ("HOME", None),
            ("XDG_CONFIG_HOME", None),
        ],
        || {
            let cfg = load_from(dir.path()).unwrap();
            assert_eq!(cfg.gateway.port, 4242, "env should beat toml");
            assert_eq!(
                cfg.gateway.url, "http://from-toml/",
                "toml layer still wins for unset env keys"
            );
        },
    );
}

#[test]
fn defaults_when_no_layers_present() {
    let dir = tempfile::tempdir().unwrap();
    scoped_env(
        &[
            ("ARGO_GATEWAY_PORT", None),
            ("ARGO_GATEWAY_URL", None),
            ("HOME", None),
            ("XDG_CONFIG_HOME", None),
        ],
        || {
            let cfg = load_from(dir.path()).unwrap();
            assert_eq!(cfg.gateway.port, defaults::DEFAULT_GATEWAY_PORT);
            assert_eq!(
                cfg.gateway.duty_scheduler_secs,
                defaults::DEFAULT_DUTY_SCHEDULER_SECS
            );
        },
    );
}

#[test]
fn merge_overlays_only_marked_fields() {
    let mut base = ArgoConfig::default();
    base.gateway.port = 1111;
    base.gateway.url = "http://base/".into();

    let mut overlay = ArgoConfig::default();
    overlay.gateway.port = 2222;
    overlay.overrides.gateway_port = true;
    // overlay.gateway.url is left at default and unmarked, so merge
    // must NOT overwrite base's url.

    base.merge(overlay);
    assert_eq!(base.gateway.port, 2222);
    assert_eq!(base.gateway.url, "http://base/");
}

#[test]
fn from_env_parses_typed_fields() {
    scoped_env(
        &[
            ("ARGO_GATEWAY_PORT", Some("12345")),
            ("ARGO_DUTY_SCHEDULER_SECS", Some("7")),
            ("ARGO_TELEMETRY_ENABLED", Some("0")),
        ],
        || {
            let cfg = ArgoConfig::from_env().unwrap();
            assert_eq!(cfg.gateway.port, 12345);
            assert_eq!(cfg.gateway.duty_scheduler_secs, 7);
            assert!(!cfg.telemetry.enabled);
            assert!(cfg.overrides.gateway_port);
            assert!(cfg.overrides.telemetry_enabled);
            // Untouched env vars stay unmarked, so merging them
            // upstream is a no-op.
            assert!(!cfg.overrides.gateway_url);
        },
    );
}

#[test]
fn from_env_surfaces_parse_errors() {
    scoped_env(&[("ARGO_GATEWAY_PORT", Some("not-a-number"))], || {
        assert!(ArgoConfig::from_env().is_err());
    });
}

#[test]
fn skycot_path_preferred_over_legacy() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".skycot")).unwrap();
    std::fs::write(
        dir.path().join(".skycot/argo.toml"),
        "[gateway]\nport = 5555\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("argo.toml"), "[gateway]\nport = 6666\n").unwrap();

    scoped_env(
        &[
            ("ARGO_GATEWAY_PORT", None),
            ("HOME", None),
            ("XDG_CONFIG_HOME", None),
        ],
        || {
            let cfg = load_from(dir.path()).unwrap();
            assert_eq!(cfg.gateway.port, 5555, ".skycot path should win");
        },
    );
}

/// Set / unset env vars for the duration of `f`, restoring on drop.
/// Required because `argo_config::load` reads the live process env
/// and tests in the same crate share it. Serialized via a process-wide
/// mutex so parallel test runs do not see each other's env edits.
fn scoped_env<F: FnOnce()>(pairs: &[(&str, Option<&str>)], f: F) {
    use std::sync::Mutex;
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    let _lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let originals: Vec<(String, Option<String>)> = pairs
        .iter()
        .map(|(k, _)| ((*k).to_string(), std::env::var(k).ok()))
        .collect();
    for (k, v) in pairs {
        match v {
            Some(val) => std::env::set_var(k, val),
            None => std::env::remove_var(k),
        }
    }
    let _guard = ScopedEnvGuard { originals };
    f();
}

struct ScopedEnvGuard {
    originals: Vec<(String, Option<String>)>,
}
impl Drop for ScopedEnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.originals {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }
}
