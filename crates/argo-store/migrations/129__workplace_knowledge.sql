-- mission-workplace-knowledge-substrate — tenant-scoped wiki substrate
-- for the Wave-A workplace-onboarding pillar (handbook layer).
--
-- One row in `workplace_knowledge_doc` per current head of a wiki
-- doc; one row in `workplace_knowledge_revision` per historical
-- edit. `workplace_knowledge_doc.version` is the head version
-- counter; the revision table preserves the prior body + an
-- optional diff snippet for audit + rollback. Lookups are
-- `(tenant_id, slug)` keyed; cross-tenant reads are guarded by
-- always binding `tenant_id` in the WHERE clause (callers in
-- `argo-workplace-knowledge::doc` / `::revision`).
--
-- Slot rationale: the highest committed raw SQL filename slot on
-- origin/main is 128 (`revoked_tokens`, lazy-bootstrap), so 129 is
-- the next free filename slot. The versioned migration counter is
-- v76; this brief registers v77. Filename and version counters
-- advance independently per the v55 convention.

CREATE TABLE workplace_knowledge_doc (
    id                   TEXT      NOT NULL PRIMARY KEY,
    tenant_id            TEXT      NOT NULL,
    slug                 TEXT      NOT NULL,
    title                TEXT      NOT NULL,
    category             TEXT      NOT NULL,
    body_markdown        TEXT      NOT NULL,
    version              INTEGER   NOT NULL,
    parent_revision_id   TEXT,
    created_at_micros    INTEGER   NOT NULL,
    created_by           TEXT      NOT NULL,
    updated_at_micros    INTEGER   NOT NULL,
    updated_by           TEXT      NOT NULL,
    UNIQUE(tenant_id, slug)
);

CREATE INDEX idx_workplace_knowledge_doc_tenant_category
    ON workplace_knowledge_doc(tenant_id, category);

CREATE TABLE workplace_knowledge_revision (
    id                TEXT      NOT NULL PRIMARY KEY,
    doc_id            TEXT      NOT NULL,
    tenant_id         TEXT      NOT NULL,
    version           INTEGER   NOT NULL,
    body_markdown     TEXT      NOT NULL,
    diff_from_prior   TEXT,
    edited_at_micros  INTEGER   NOT NULL,
    edited_by         TEXT      NOT NULL,
    edit_reason       TEXT
);

CREATE INDEX idx_workplace_knowledge_revision_doc
    ON workplace_knowledge_revision(doc_id, version DESC);
