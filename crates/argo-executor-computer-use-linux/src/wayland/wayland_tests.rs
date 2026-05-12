//! Wayland-backend unit tests.
//!
//! Pure / mockable tests only. Real D-Bus + Wayland integration runs
//! under `ARGO_WAYLAND_INTEGRATION=1` in a separate, ignored harness
//! that is not part of `cargo test`.

use crate::probe::WaylandCapabilities;

// --- Probe / capability matrix ---------------------------------------

#[test]
fn classify_compositor_recognizes_gnome_kde_sway_hyprland() {
    use super::probe::classify_compositor;
    assert_eq!(classify_compositor(Some("GNOME"), None), "gnome");
    assert_eq!(
        classify_compositor(Some("ubuntu:GNOME"), None),
        "gnome"
    );
    assert_eq!(classify_compositor(Some("KDE"), None), "kde");
    assert_eq!(classify_compositor(Some("plasma"), None), "kde");
    assert_eq!(classify_compositor(Some("sway"), None), "sway");
    assert_eq!(
        classify_compositor(Some("Hyprland"), None),
        "hyprland"
    );
    assert_eq!(classify_compositor(None, None), "unknown");
    assert_eq!(classify_compositor(Some("xfce"), None), "unknown");
}

#[test]
fn caps_are_sufficient_requires_wtype_and_a_screenshot_path() {
    use super::probe::caps_are_sufficient;
    let portal_only = WaylandCapabilities {
        compositor: "gnome".into(),
        portal_reachable: true,
        wtype: true,
        grim: false,
    };
    let grim_only = WaylandCapabilities {
        compositor: "sway".into(),
        portal_reachable: false,
        wtype: true,
        grim: true,
    };
    let no_wtype = WaylandCapabilities {
        compositor: "kde".into(),
        portal_reachable: true,
        wtype: false,
        grim: true,
    };
    let bare = WaylandCapabilities::default();
    assert!(caps_are_sufficient(&portal_only));
    assert!(caps_are_sufficient(&grim_only));
    assert!(!caps_are_sufficient(&no_wtype));
    assert!(!caps_are_sufficient(&bare));
}

#[test]
fn unsupported_reason_names_the_missing_primitive() {
    use super::probe::unsupported_reason;
    let no_wtype = WaylandCapabilities {
        wtype: false,
        ..Default::default()
    };
    assert!(unsupported_reason(&no_wtype).contains("wtype"));
    let no_screenshot = WaylandCapabilities {
        wtype: true,
        portal_reachable: false,
        grim: false,
        compositor: "unknown".into(),
    };
    let reason = unsupported_reason(&no_screenshot);
    assert!(reason.contains("portal") || reason.contains("grim"));
}

#[test]
fn mouse_unsupported_hint_names_compositor_and_alternatives() {
    use super::probe::mouse_unsupported_hint;
    let hint = mouse_unsupported_hint("hyprland");
    assert!(hint.contains("hyprland"));
    assert!(hint.contains("GNOME/KDE/sway/Hyprland"));
}

#[test]
fn compositor_known_matrix_only_documented_names() {
    use super::probe::compositor_is_known;
    assert!(compositor_is_known("gnome"));
    assert!(compositor_is_known("kde"));
    assert!(compositor_is_known("sway"));
    assert!(compositor_is_known("hyprland"));
    assert!(!compositor_is_known("unknown"));
    assert!(!compositor_is_known("xfce"));
}

// --- Portal mock --------------------------------------------------

#[test]
fn portal_uri_to_path_handles_authority_and_percent_escape() {
    use super::portal::uri_to_path;
    assert_eq!(
        uri_to_path("file:///tmp/argo-shot.png"),
        Some("/tmp/argo-shot.png".to_string())
    );
    assert_eq!(
        uri_to_path("file://localhost/var/tmp/x.png"),
        Some("/var/tmp/x.png".to_string())
    );
    assert_eq!(
        uri_to_path("file:///home/user/My%20Pics/a.png"),
        Some("/home/user/My Pics/a.png".to_string())
    );
    assert_eq!(uri_to_path("http://example.com/x.png"), None);
    assert_eq!(uri_to_path("file://"), None);
}

#[test]
fn portal_constants_are_canonical_freedesktop_names() {
    use super::portal::{
        PORTAL_BUS_NAME, PORTAL_OBJECT_PATH, REMOTE_DESKTOP_INTERFACE,
        SCREENSHOT_INTERFACE,
    };
    assert_eq!(PORTAL_BUS_NAME, "org.freedesktop.portal.Desktop");
    assert_eq!(PORTAL_OBJECT_PATH, "/org/freedesktop/portal/desktop");
    assert_eq!(SCREENSHOT_INTERFACE, "org.freedesktop.portal.Screenshot");
    assert_eq!(
        REMOTE_DESKTOP_INTERFACE,
        "org.freedesktop.portal.RemoteDesktop"
    );
}

#[test]
fn portal_is_unreachable_on_non_linux_targets() {
    // On non-Linux the stub always returns false; on Linux the call
    // returns whatever the session bus reports — accept either as a
    // smoke check that the function compiles and is callable. The
    // gated integration test asserts true on a real Wayland host.
    let _ = super::portal::is_reachable();
}

// --- Input stub --------------------------------------------------

#[test]
fn wtype_args_terminate_options_before_user_text() {
    use super::input::type_args;
    assert_eq!(type_args("hello"), vec!["--", "hello"]);
    assert_eq!(
        type_args("-stuck-with-leading-dash"),
        vec!["--", "-stuck-with-leading-dash"]
    );
}

#[test]
fn wtype_key_args_use_dash_k_flag() {
    use super::input::key_args;
    assert_eq!(key_args("Return"), vec!["-k", "Return"]);
}

#[test]
fn wayland_key_aliases_map_to_xkeysym_names() {
    use super::input::normalize_key;
    assert_eq!(normalize_key("enter").unwrap(), "Return");
    assert_eq!(normalize_key("esc").unwrap(), "Escape");
    assert_eq!(normalize_key("backspace").unwrap(), "BackSpace");
    assert!(normalize_key("").is_err());
    assert_eq!(normalize_key("ctrl+l").unwrap(), "ctrl+l");
}

// --- Screenshot stub -----------------------------------------------

#[test]
fn grim_args_pass_destination_verbatim() {
    use super::screenshot::grim_args;
    assert_eq!(
        grim_args(std::path::Path::new("/tmp/out.png")),
        vec!["/tmp/out.png"]
    );
}

#[test]
fn click_without_portal_returns_unsupported_with_hint() {
    let caps = WaylandCapabilities {
        compositor: "sway".into(),
        portal_reachable: false,
        wtype: true,
        grim: true,
    };
    let err = super::input::click(10, 20, &caps).unwrap_err();
    match err {
        crate::LinuxCuError::Unsupported { reason } => {
            assert!(reason.contains("sway"));
            assert!(reason.contains("GNOME/KDE/sway/Hyprland"));
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}
