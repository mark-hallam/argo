# Argo Approve — mobile shell

Tauri 2 mobile app (iOS + Android) that lets an operator approve
or deny pending Argo agent actions from their phone. Thin client
over the existing gateway routes; no backend, no sync, no local
state besides the bearer token and gateway URL.

Two screens:

1. **Pending approvals** — fetches `GET /inbox/ws` (snapshot
   JSON) every 5 s and renders one card per pending action.
2. **Settings** — bearer token + gateway URL entry, persisted
   via Tauri's secure key-value store (Keychain on iOS,
   `EncryptedSharedPreferences` on Android).

Push notifications from `D-push` (#798) deep-link
`argo://approve/<token>` straight to the matching card.

## 5-step setup

1. **Install prerequisites** on macOS:
   ```sh
   # Rust + Cargo
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Node.js (LTS or newer)
   brew install node

   # Tauri CLI 2.x
   cargo install tauri-cli --version "^2.0.0"

   # Xcode (App Store) for iOS targets
   xcode-select --install
   sudo xcodebuild -license accept

   # Android Studio + Android SDK / NDK
   brew install --cask android-studio
   ```

2. **Install npm deps and generate icons**:
   ```sh
   cd mobile/argo-approve
   npm install
   cargo tauri icon path/to/source-1024.png
   ```

3. **Initialize the iOS / Android Xcode + Gradle projects**:
   ```sh
   cargo tauri ios init
   cargo tauri android init
   ```

   This creates `src-tauri/gen/` with platform-specific
   manifests and signing stubs. You'll fill in your
   developer-team identifier in `src-tauri/gen/apple/` and
   the package signing config in `src-tauri/gen/android/`.

4. **Run on a simulator**:
   ```sh
   cargo tauri ios dev      # iOS Simulator
   cargo tauri android dev  # Android Emulator
   ```

5. **Configure the app on first launch**: open Settings and paste
   the bearer token from
   ```sh
   argo cred get ARGO_WEBINBOX_BEARER_TOKEN
   ```
   plus your gateway URL (e.g. `https://argo.example.com:7090`).
   Settings persist across app launches.

## What the app actually calls

| App action     | HTTP                                              |
|----------------|---------------------------------------------------|
| List pending   | `GET /inbox/ws` with `Authorization: Bearer …`    |
| Approve        | `POST /inbox/approve/<approval_id>` with bearer   |
| Deny           | `POST /inbox/reject/<approval_id>` with bearer    |
| Push deep-link | `POST /channels/push/decision/<token>?d=approve`  |

These routes live in
`crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs`
and `routes_channels_push.rs`.

## Boundary — what's verified

- Source structure compiles in Tauri 2's mobile project layout
  (`src-tauri/src/lib.rs` with `mobile_entry_point`, `Cargo.toml`
  with `tauri-plugin-deep-link` + `tauri-plugin-store`, frontend
  React 18 entry).
- React frontend is type-checkable with `npm run build` once
  deps are installed.

## Boundary — deferred to operator local install

The standalone Tauri shell is **not** built by
`cargo build --workspace` — `src-tauri/Cargo.toml` declares its
own `[workspace]` and is intentionally outside the Argo
workspace so iOS / Android targets do not break workspace builds.

The following are not validated in this repo's CI; the operator
performs them once during initial setup:

- `cargo tauri ios dev` + `cargo tauri android dev` (require
  Xcode, Android SDK / NDK, signing identities)
- App size verification (target `< 30 MB`; depends on icons,
  WebView system component, and bundle compression)
- Push notification deep-link round-trip (requires APNs / FCM
  certificates and a real device)

This mirrors the
[`claude-installer-macos`](../../Formula/argo.rb) pattern of
shipping the structural artifacts (Homebrew formula,
LaunchAgent plist, ad-hoc-signed `.dmg`) with placeholder
shasums until the first signed release.

## Project layout

```
mobile/argo-approve/
├── README.md             # this file
├── package.json          # npm + Tauri CLI deps
├── tsconfig.json
├── vite.config.ts
├── index.html
├── src/                  # React frontend
│   ├── main.tsx
│   ├── api.ts            # /inbox + decision client
│   ├── store.ts          # tauri-plugin-store wrapper
│   ├── types.ts
│   ├── styles.css
│   └── screens/
│       ├── ApprovalsList.tsx
│       └── Settings.tsx
└── src-tauri/            # Rust shell — own workspace
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs
    ├── icons/            # operator generates via `cargo tauri icon`
    └── src/
        ├── main.rs
        └── lib.rs        # mobile_entry_point + plugin init
```
