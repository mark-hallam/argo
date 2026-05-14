# argo-acp

`argo-acp` is the standalone Agent Client Protocol server library for
IDE and harness integrations such as VS Code, Zed, JetBrains, Copilot,
Cursor, and generic ACP clients.

The crate is intentionally kept as a workspace member even though no
other workspace crate depends on it in v1. The previous `argo acp serve`
CLI stub was removed because it advertised an unreachable command, but
the library itself is not dead code: it owns the protocol wire types,
harness capability model, ACP authentication mapping, capability mapping,
and dispatcher trait used by a future server entry point.

Current disposition:

- **Kept** as a standalone ACP library.
- **Not wired** to `argo-cli` in v1, because the deleted CLI command had
  no working runtime dispatch path.
- **Future wire-up** should add an explicit server binary or CLI command
  that constructs an `AcpService` implementation and runs the dispatcher.

This status is recorded so crate-inventory sweeps do not classify
`argo-acp` as an orphan solely because it has no compile-time workspace
consumer.
