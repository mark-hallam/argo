-- Stream `claude-dreams-plan-templates` (V2.0 — Dreams plan-template
-- learning). Persists polished plan templates that the Dreams API
-- distilled from cohorts of successful Planner decompositions.
--
-- Lifecycle (TrustGrade ladder):
--   * `pending`  - distilled by Dreams, not yet operator-approved.
--   * `promoted` - operator approved via `argo plan template promote`,
--                  TemplatePlanner consults these before falling
--                  through to LlmPlanner.
--   * `rejected` - operator-rejected, never matched.
--
-- The `parent_goal` field is the anchor cohort goal — useful for
-- audit so operators can trace a learned template back to its
-- source decompositions in `subtask_ledger`.
--
-- The `keywords_csv` field is comma-separated lower-case tokens the
-- TemplatePlanner uses for keyword-based matching. The `yaml` field
-- stores the workflow body the Dreams call returned.

CREATE TABLE IF NOT EXISTS learned_plan_template (
    -- UUIDv7 string id (sortable insert order).
    id TEXT PRIMARY KEY,
    -- Stable name used in TemplatePlanner reasoning + CLI display.
    name TEXT NOT NULL,
    -- One-line description shown to operators.
    description TEXT NOT NULL,
    -- Anchor cohort goal — first parent_goal in the cohort whose
    -- decompositions seeded this template.
    parent_goal TEXT NOT NULL,
    -- Comma-separated lower-case keywords for matching. Empty
    -- string is allowed if the template should only match when
    -- explicitly named.
    keywords_csv TEXT NOT NULL DEFAULT '',
    -- Workflow YAML the Dreams call distilled.
    yaml TEXT NOT NULL,
    -- Lifecycle state: pending / promoted / rejected.
    trust_grade TEXT NOT NULL DEFAULT 'pending',
    -- Cohort size that produced this template (always >= 2 in
    -- practice, the floor is enforced by the runtime
    -- `MIN_PLAN_COHORT_SIZE`).
    cohort_size INTEGER NOT NULL DEFAULT 0,
    -- Created-at micros since epoch.
    created_at INTEGER NOT NULL,
    -- Promoted-at micros since epoch (NULL while pending).
    promoted_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_learned_plan_template_grade
    ON learned_plan_template(trust_grade, created_at);

CREATE INDEX IF NOT EXISTS idx_learned_plan_template_name
    ON learned_plan_template(name);

-- Two distinct templates may share a `name` if their cohorts
-- diverged enough to warrant separate bodies — but the same id
-- must never collide. The PRIMARY KEY guards id uniqueness.
