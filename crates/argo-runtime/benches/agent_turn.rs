//! Criterion bench for runtime per-turn overhead. Stream
//! `mission-criterion-load-tests`.
//!
//! Why this matters: every agent turn pays the planner's per-call
//! cost (prompt assembly, response parse, workflow validate)
//! on top of the LLM round-trip. A regression in the loop's
//! internal cost is invisible behind the dominant LLM latency,
//! so we measure it with a mock caller that returns in 5ms.
//!
//! Mock caller returns a fixed 3-task workflow YAML response.
//! The bench wraps `LlmPlanner::decompose` so we exercise the
//! whole planner loop (prompt build → caller → JSON parse →
//! YAML parse → workflow validate → context apply).
//!
//! Brief drift note: the brief named `agent_turn::p99_latency`;
//! the runtime crate's actual loop-level entry is the
//! [`argo_runtime::Planner`] trait. Bench what exists — the
//! planner is the per-turn runtime work argo-runtime owns.
//!
//! Run locally:
//!   `cargo bench -p argo-runtime --bench agent_turn`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).
use argo_runtime::{AnthropicCaller, LlmPlanner, Planner, PlannerContext, PlannerError};
use async_trait::async_trait;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fmt::Display;
use std::time::Duration;
use tokio::runtime::Runtime;

const MOCK_DELAY_MS: u64 = 5;
const TURNS_PER_BATCH: usize = 100;
const FIXED_RESPONSE: &str = r#"{
  "reasoning": "Decompose the goal into three sequenced subtasks: draft, review, publish.",
  "workflow_yaml": "id: bench-flow\ndescription: Three-step demo\ntasks:\n  - id: draft\n  - id: review\n    depends_on: [draft]\n  - id: publish\n    depends_on: [review]\n"
}"#;

struct MockCaller;

#[async_trait]
impl AnthropicCaller for MockCaller {
    async fn call(&self, _prompt: String) -> Result<String, PlannerError> {
        tokio::time::sleep(Duration::from_millis(MOCK_DELAY_MS)).await;
        Ok(FIXED_RESPONSE.to_string())
    }
}

fn p99_latency(c: &mut Criterion) {
    let runtime = must(Runtime::new(), "tokio runtime");
    let mut group = c.benchmark_group("agent_turn");
    group.sample_size(20);

    let ctx = PlannerContext::for_worker("bench-worker")
        .with_vertical("bench-vertical");
    let goal = "Decompose a launch announcement into draft, review, publish subtasks.";
    let planner = LlmPlanner::new(Box::new(MockCaller));

    group.bench_function("loop_overhead_one_turn", |b| {
        b.iter(|| {
            let result = runtime.block_on(planner.decompose(black_box(goal), black_box(&ctx)));
            black_box(must(result, "planner decompose"));
        });
    });

    group.bench_function("loop_overhead_100_turns", |b| {
        b.iter(|| {
            runtime.block_on(async {
                for _ in 0..TURNS_PER_BATCH {
                    let result = planner.decompose(black_box(goal), black_box(&ctx)).await;
                    black_box(must(result, "planner decompose"));
                }
            });
        });
    });

    group.finish();
}

fn must<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

criterion_group!(benches, p99_latency);
criterion_main!(benches);
