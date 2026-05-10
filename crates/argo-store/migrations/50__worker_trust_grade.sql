-- Stream `mvp-worker-trust-grade-column` (Tier M prereq for
-- mission-routing-skillmatch-rolematch and mission-skill-propagation-loop).
-- Adds the worker-side trust grade + recency columns the routing engine
-- needs to rank candidates for `TargetSelector::SkillMatch`/`RoleMatch`.
--
-- `trust_grade` is the same numeric scale as `argo_skill::trust::TrustGrade`
-- so the worker-side and skill-side ladders compose. `0` is the "unknown"
-- floor for fresh rows; the routing engine maps higher values to higher
-- trust under the brief's documented numeric ladder. `last_used_at` is
-- ISO-8601 UTC text (NULL = never used) so SQLite ORDER BY DESC sorts
-- the most-recent worker first.
--
-- Slot rationale: SQL filename slots 47 (`skill_canary`), 48
-- (`approvals`), and 49 (`worker_archived_at`, sibling brief
-- `mvp-identity-archive-worker`) were already taken when this brief
-- landed; per the brief's "if 48 is also taken, use the next free slot"
-- rule this rolls forward to 50.
ALTER TABLE worker ADD COLUMN trust_grade INTEGER NOT NULL DEFAULT 0;
ALTER TABLE worker ADD COLUMN last_used_at TEXT;

CREATE INDEX IF NOT EXISTS idx_worker_trust_grade ON worker(trust_grade DESC);
CREATE INDEX IF NOT EXISTS idx_worker_last_used_at ON worker(last_used_at DESC);
