//! Argo approval app — Tauri 2 mobile shell.
//!
//! Thin wrapper that mounts the WebView and registers two
//! Tauri plugins:
//!
//! - `tauri-plugin-store` — settings persistence (bearer token,
//!   gateway URL) via Tauri's secure key-value store. On iOS this
//!   maps to the Keychain; on Android, to EncryptedSharedPreferences.
//! - `tauri-plugin-deep-link` — handles `argo://approve/<token>`
//!   deep links from push notifications.
//!
//! The frontend (TypeScript + minimal React) does all the UI work
//! over the existing gateway HTTP routes; no Rust commands are
//! exposed.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running argo-approve");
}
