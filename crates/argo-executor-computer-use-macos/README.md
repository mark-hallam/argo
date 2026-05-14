# macOS Computer-Use Executor

`argo-executor-computer-use-macos` is the v1 primary computer-use
execution path for the SKYCOT demo. It drives native macOS apps through
the shipped Notes smoke path using `screencapture`, AppleScript, and
the macOS accessibility permissions model.

The E2B, Linux, and Windows computer-use executors remain in the
workspace and continue to compile, but they are classified as
`DEFERRED-v1.x` for v1 launch readiness. They are not deleted and are
not treated as v1 demo blockers.

## Operator Checklist

Run the native smoke on a macOS machine with the required app
permissions granted:

```bash
ARGO_NATIVE_MACOS_INTEGRATION=1 \
  cargo test -p argo-executor-computer-use-macos --test notes_smoke
```

Without `ARGO_NATIVE_MACOS_INTEGRATION=1`, the integration half of the
test skips and the config validation smoke still runs. This keeps the
workspace build green on non-macOS runners while preserving a real
operator-facing macOS check for the demo machine.
