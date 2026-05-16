-- Stream `claude-data-classification` — typed PII / PHI /
-- financial / secret tags over memory rows, audit payloads, and
-- worker actions. One row per (record_kind, record_id, class) so
-- a record can carry multiple classes. The DataClass enum lives
-- in argo_identity::classification, this table persists the
-- string form.

CREATE TABLE IF NOT EXISTS data_classification_tag (
    -- Logical record kind ('memory_entry', 'audit_record', etc.).
    record_kind TEXT NOT NULL,
    -- Record id (caller-defined, usually a UUID string or seq).
    record_id TEXT NOT NULL,
    -- One of the DataClass enum string forms (pii / phi /
    -- financial / secret / public / untagged). Validated at the
    -- typed-helper layer, sqlite stores raw text.
    class TEXT NOT NULL,
    -- Microseconds since epoch when the tag was inserted.
    tagged_at_micros INTEGER NOT NULL,
    -- Free-form actor id (worker, operator, automation rule). NULL
    -- when the tag was set without an explicit author.
    tagged_by TEXT,
    PRIMARY KEY (record_kind, record_id, class)
);

CREATE INDEX IF NOT EXISTS idx_dc_tag_class
    ON data_classification_tag(class);

CREATE INDEX IF NOT EXISTS idx_dc_tag_record
    ON data_classification_tag(record_kind, record_id);

CREATE INDEX IF NOT EXISTS idx_dc_tag_tagged_at
    ON data_classification_tag(tagged_at_micros);
