//! Runtime instrumentation hooks.
//!
//! Process-global emit/dispatch surfaces that any future IR runtime
//! caller can drive without taking a runtime dependency on the
//! subsystem that consumes the events (gateway WS, telemetry sink,
//! etc.). Submodules:
//!
//! - [`ir_trace`] — Skill IR node lifecycle (`entered` / `completed`
//!   / `failed`); consumed by the canvas live-trace overlay
//!   (Track L L5).

#![warn(missing_docs)]

pub mod ir_trace;
