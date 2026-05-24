# argo-skill-sdk author-flow audit

Date: 2026-05-24
Brief: `eval-argo-skill-sdk-author-flow`
Verdict: PASS with follow-up contract gaps named.

## Scope

This audit covers the external Rust authoring flow in `argo-skill-sdk`.
It does not change SDK production code or runtime loading. The prior
surface audit (#2132) already pinned public names; this pass checks
whether authors can build realistic graphs through the public builder.

## Public Authoring Surface

- IR re-exports: `BranchArm`, `Edge`, `EdgeCond`, `EdgeId`, `Node`,
  `NodeAnnotations`, `NodeId`, `NodeKind`, `NodePolicy`, `SkillGraph`,
  `SkillGraphMetadata`, `SubSkillRef`, `ToolBinding`, `Variable`,
  `VariableScope`.
- Builder API: `SkillBuilder`, `create_skill`, `branch_arm`,
  `branch_arm_pattern`, `validate_skill_graph`.
- Inputs: `SkillBuilderInput` for graph metadata and
  `BuilderNodeOptions` for per-node annotations.
- Examples: `sample_skills()` returns five validated sample graphs.

## Smoke Coverage Added

New test: `crates/argo-skill-sdk/tests/author_flow_smoke_test.rs`.

- `authors_echo_skill_with_versioned_metadata`: builds an echo action
  with name, version, description, source, and a graph variable.
- `authors_capability_declared_skill_with_tool_binding_and_policy`:
  builds an approval-gated Slack action and verifies `ToolBinding`
  plus `NodePolicy::Ask` survive validation.
- `authors_metadata_bearing_skill_with_human_checkpoint`: builds a
  fetch -> condition -> approval -> action graph, verifies metadata,
  and checks JSON serialization keeps name/version/source.

## Findings

- Version metadata is explicit through `SkillBuilderInput::version`.
- Provenance metadata is explicit through `SkillBuilderInput::source`.
- Capability intent is partially explicit: authors can bind a node to
  an MCP server/tool with `ToolBinding` and can set execution policy via
  public `BuilderNodeOptions.annotations.policy`.
- There is no first-class `CapabilitySet` or permission declaration in
  `argo-skill-sdk`; the current contract is tool-binding metadata, with
  runtime capability enforcement outside this crate.
- There is no first-class audit hook field in the SDK. Human checkpoint
  intent is modelled with `AwaitApproval`; actual audit emission remains
  runtime responsibility.
- Tenant identity is absent from authored graphs. That appears
  intentional for portable skill IR, but it is not explicitly stated in
  the SDK author-flow surface.

## Follow-Up Brief Names

- `docs-skill-sdk-author-contract-boundaries`: document that tenant
  binding and audit emission live in storage/runtime, while SDK graphs
  remain portable.
- `mission-skill-sdk-explicit-capability-contract`: decide whether
  `ToolBinding` is the complete author-facing capability declaration or
  add a first-class capability metadata field before SDK v1.0.

## Verification

- `rustfmt --edition 2021 --check crates/argo-skill-sdk/tests/author_flow_smoke_test.rs`
- `git diff --check`
- `CARGO_TARGET_DIR=/tmp/eval-argo-skill-sdk-author-flow-target cargo test -p argo-skill-sdk --test author_flow_smoke_test -- --nocapture`
- `CARGO_TARGET_DIR=/tmp/eval-argo-skill-sdk-author-flow-target cargo test -p argo-skill-sdk --tests`
