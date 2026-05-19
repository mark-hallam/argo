-- Stream `mission-pm-approval-content-preview` — surface the actual
-- draft body (PRD body, email body, roadmap delta) alongside the
-- existing intent + consequences so approvers can read what is about
-- to ship before they decide.
--
-- Slot rationale: latest raw slot on origin/main is 116
-- (`data_export_job`); 117 is the next free slot. Versioned migration
-- counter is v71 — next free after v70 (`support_ticket`). Filename
-- and version counters advance independently per the v55 module's
-- documented convention.
--
-- `draft_content` is nullable so existing rows read as `NULL` (the
-- card stays glanceable when no draft is attached) and back-fills
-- happen lazily as new approval-gated executors land bodies.

ALTER TABLE approval_request
    ADD COLUMN draft_content TEXT NULL;
