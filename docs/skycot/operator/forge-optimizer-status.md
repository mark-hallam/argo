# Forge Optimizer Status

Forge optimization is intentionally stubbed in v1. The Rust `argo-forge`
crate keeps the stable orchestration seam, cost model, run metadata, and
feedback ingestion path, but the production DSPy/GEPA optimizer worker is
not configured in the v1 runtime.

When the stub path is invoked, `StubForge` returns
`ForgeError::OptimizerNotConfigured` and emits the
`FORGE_STUB_INVOCATION` audit event. Operator-facing error rendering
includes this document path so the failure is recognizable as a known
v1 limitation, not an accidental outage.

The follow-on implementation is tracked by
`docs/build-plan/streams/mission-forge-stubforge-cleanup.md`. That brief
depends on the plugin registry surface and will replace the stub with a
configured worker plugin dispatch path.

Until that lands, do not treat Forge optimizer failures as retryable
provider incidents. Existing skill evaluation, marketplace publishing,
operator feedback ingestion, and cost attribution remain active; only
automatic DSPy/GEPA optimization is deferred.
