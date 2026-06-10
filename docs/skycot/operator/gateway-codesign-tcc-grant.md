# Durable macOS permissions for the gateway (stable code signing)

**Status:** operator runbook · DEC-050 G4 · brief
`cu4-stable-codesign-gateway-for-durable-tcc-grants`

## The problem this solves

The desktop-drive demo (computer-use) needs the gateway binary to hold two
macOS permissions:

- **Accessibility** (drive the mouse/keyboard)
- **Screen Recording** (take screenshots)

You grant these once in **System Settings → Privacy & Security**. But until
now the grant kept evaporating: every time the binary was rebuilt
(`cargo build`), the permission was gone and the demo broke.

**Why:** macOS records a permission against a binary's *code identity*. Our
release binary was only *ad-hoc signed* (the default "linker signature"), and
an ad-hoc binary's identity is just a hash of its bytes — which changes on
**every** rebuild. So macOS saw each rebuild as "a different program it has
never heard of," and the old grant no longer applied.

Confirmed 2026-06-11: `codesign -dv` on the live binary reported
`Signature=adhoc`, and the 06:02 rebuild orphaned the 2026-05-26 grant.

## The fix

Sign the binary with a **stable identity** (a self-signed developer
certificate that lives in your keychain). Then macOS keys the permission to
that stable identity instead of the ever-changing byte hash, and the grant
**survives every rebuild**.

This is a **one-time setup**. After it, you never re-grant on a rebuild again.

---

## One-time setup (≈3 minutes)

### 1. Create the stable signing identity

```bash
scripts/create-gateway-signing-identity.sh
```

This generates a self-signed code-signing certificate named
`skycot-gateway-dev`, puts it in your login keychain, and allows `codesign`
to use it without prompting. Re-running it is safe (it no-ops if the identity
already exists). macOS may ask for your login password to import the key.

> Want a different name? `scripts/create-gateway-signing-identity.sh my-name`
> (then use that name in step 2).

### 2. Tell the gateway to use it

Add the identity to your gateway environment file so every restart signs with
it:

```bash
echo "export ARGO_CODESIGN_IDENTITY='skycot-gateway-dev'" >> ~/.argo/env.local
```

(`~/.argo/env.local` is auto-sourced by `restart-live-gateway.sh`.)

### 3. Rebuild, sign, and relaunch the gateway

```bash
scripts/restart-live-gateway.sh --force
```

You should see a `[5b/8] Applying stable code signature` step, ending with
`stable signature applied`. Verify it took:

```bash
codesign -dv "${CARGO_TARGET_DIR:-./target}/release/argo"
```

You want a line like `Identifier=com.skycot.argo` and **no** `Signature=adhoc`.

### 4. Grant the two permissions (the bit you only do once)

Open **System Settings → Privacy & Security**, then:

- **Accessibility** → click **+** → navigate to the signed binary
  (`~/.cache/argo-shared-target/release/argo` or your
  `$CARGO_TARGET_DIR/release/argo`) → add it → toggle it **on**.
- **Screen Recording** → same, **+** → add the same binary → toggle **on**.

> Tip: in the file picker press **⌘⇧G** and paste the full path to the binary.

---

## Verify it survived a rebuild

This is the real acceptance test — that the grant outlives a recompile:

```bash
# rebuild + re-sign + relaunch
scripts/restart-live-gateway.sh --force

# the computer-use smoke test should PASS (not skip) without a re-grant:
cargo test -p argo-executor-computer-use-macos claim_5_live_macos_cu_smoke_test
```

If it passes after a rebuild **without you re-granting anything**, the durable
grant is working.

---

## How it works (one paragraph)

`scripts/restart-live-gateway.sh` step **5b** calls
`scripts/codesign-gateway-binary.sh`, which runs
`codesign --force --sign "$ARGO_CODESIGN_IDENTITY" --identifier com.skycot.argo`
on the freshly built binary. The `--identifier` pins a stable bundle id, and
the signing identity is constant, so the binary's **Designated Requirement**
(the thing macOS TCC keys the permission to) is identical across rebuilds. The
step is **opt-in and fail-soft**: if `ARGO_CODESIGN_IDENTITY` is unset or the
identity isn't in your keychain, it logs one line and leaves the ad-hoc
signature — the gateway still runs, you just fall back to the old
re-grant-on-rebuild behaviour.

## Troubleshooting

- **`identity 'skycot-gateway-dev' not found in any keychain`** — run step 1
  again; check `security find-identity -v -p codesigning` lists it.
- **`codesign` prompts every restart** — re-run step 1; it calls
  `security set-key-partition-list` to authorise `codesign` for the key.
- **Still `Signature=adhoc` after a restart** — confirm
  `ARGO_CODESIGN_IDENTITY` is exported in the shell that runs the restart
  (it lives in `~/.argo/env.local`, auto-sourced by the restart script).
- **Permission still drops after rebuild** — make sure you added the binary at
  its real path to *both* Accessibility *and* Screen Recording, and that the
  binary is now stably signed (`codesign -dv` shows `Identifier=com.skycot.argo`,
  not `adhoc`).
