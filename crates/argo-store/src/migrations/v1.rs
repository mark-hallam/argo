//! Schema migration v1 — full v1 table set.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V1_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v1 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V1_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v1 down: {e}")))?;
    }
    Ok(())
}

const V1_UP: &[&str] = &[
    "CREATE TABLE workspace (
        id           BLOB PRIMARY KEY,
        tenant_id    BLOB NOT NULL,
        name         TEXT NOT NULL,
        root_path    TEXT NOT NULL,
        created_at   INTEGER NOT NULL,
        UNIQUE(tenant_id, name)
    )",
    "CREATE TABLE session (
        id              BLOB PRIMARY KEY,
        tenant_id       BLOB NOT NULL,
        workspace_id    BLOB NOT NULL REFERENCES workspace(id),
        parent_id       BLOB REFERENCES session(id),
        title           TEXT,
        started_at      INTEGER NOT NULL,
        ended_at        INTEGER,
        end_reason      TEXT,
        model           TEXT NOT NULL,
        provider        TEXT NOT NULL,
        system_prompt   TEXT,
        summary         TEXT,
        token_in_total  INTEGER NOT NULL DEFAULT 0,
        token_out_total INTEGER NOT NULL DEFAULT 0,
        spend_usd_micros INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE INDEX idx_session_tenant_started ON session(tenant_id, started_at DESC)",
    "CREATE INDEX idx_session_workspace ON session(workspace_id)",
    "CREATE TABLE message (
        id              BLOB PRIMARY KEY,
        session_id      BLOB NOT NULL REFERENCES session(id) ON DELETE CASCADE,
        role            TEXT NOT NULL,
        content_json    TEXT NOT NULL,
        tool_calls_json TEXT NOT NULL DEFAULT '[]',
        tool_call_id    BLOB,
        reasoning       TEXT,
        provider_meta   TEXT NOT NULL DEFAULT '{}',
        created_at      INTEGER NOT NULL,
        seq             INTEGER NOT NULL,
        UNIQUE(session_id, seq)
    )",
    "CREATE INDEX idx_message_session_seq ON message(session_id, seq)",
    "CREATE VIRTUAL TABLE message_fts USING fts5(
        content,
        content='message',
        content_rowid='rowid',
        tokenize='unicode61 remove_diacritics 2'
    )",
    "CREATE TRIGGER message_fts_ai AFTER INSERT ON message BEGIN
        INSERT INTO message_fts(rowid, content) VALUES (new.rowid, new.content_json);
    END",
    "CREATE TRIGGER message_fts_ad AFTER DELETE ON message BEGIN
        INSERT INTO message_fts(message_fts, rowid, content) VALUES ('delete', old.rowid, old.content_json);
    END",
    "CREATE TABLE tool_call (
        id              BLOB PRIMARY KEY,
        session_id      BLOB NOT NULL REFERENCES session(id) ON DELETE CASCADE,
        message_id      BLOB NOT NULL REFERENCES message(id) ON DELETE CASCADE,
        name            TEXT NOT NULL,
        arguments_json  TEXT NOT NULL,
        result_json     TEXT,
        is_error        INTEGER NOT NULL DEFAULT 0,
        duration_ms     INTEGER,
        tokens_in       INTEGER,
        tokens_out      INTEGER,
        started_at      INTEGER NOT NULL,
        ended_at        INTEGER
    )",
    "CREATE INDEX idx_toolcall_session ON tool_call(session_id, started_at)",
    "CREATE TABLE memory_entry (
        id              BLOB PRIMARY KEY,
        workspace_id    BLOB NOT NULL REFERENCES workspace(id),
        file            TEXT NOT NULL,
        body            TEXT NOT NULL,
        learned_in      BLOB,
        learned_at      INTEGER NOT NULL,
        superseded_by   BLOB REFERENCES memory_entry(id),
        hash            BLOB NOT NULL
    )",
    "CREATE INDEX idx_memory_active ON memory_entry(workspace_id, file)",
    "CREATE TABLE skill (
        id              BLOB PRIMARY KEY,
        name            TEXT NOT NULL,
        version         TEXT NOT NULL,
        body            TEXT NOT NULL,
        frontmatter     TEXT NOT NULL,
        source          TEXT NOT NULL,
        hash            BLOB NOT NULL,
        learned_in      BLOB,
        superseded_by   BLOB REFERENCES skill(id),
        created_at      INTEGER NOT NULL,
        UNIQUE(name, version)
    )",
    "CREATE TABLE agent (
        tenant_id       BLOB NOT NULL,
        name            TEXT NOT NULL,
        persona_file    TEXT,
        default_model   TEXT NOT NULL,
        default_toolset TEXT NOT NULL,
        created_at      INTEGER NOT NULL,
        PRIMARY KEY (tenant_id, name)
    )",
    "CREATE TABLE role (
        tenant_id       BLOB NOT NULL,
        name            TEXT NOT NULL,
        inherits_from   TEXT,
        capabilities    TEXT NOT NULL,
        PRIMARY KEY (tenant_id, name)
    )",
    "CREATE TABLE role_binding (
        tenant_id       BLOB NOT NULL,
        principal_kind  TEXT NOT NULL,
        principal_key   TEXT NOT NULL,
        role_name       TEXT NOT NULL,
        bound_at        INTEGER NOT NULL,
        bound_by        TEXT NOT NULL,
        PRIMARY KEY (tenant_id, principal_kind, principal_key)
    )",
    "CREATE TABLE routing_decision (
        id              BLOB PRIMARY KEY,
        session_id      BLOB NOT NULL REFERENCES session(id) ON DELETE CASCADE,
        turn            INTEGER NOT NULL,
        policy          TEXT NOT NULL,
        tier            TEXT NOT NULL,
        classifier_latency_ms INTEGER,
        chosen_provider TEXT NOT NULL,
        chosen_model    TEXT NOT NULL,
        reason          TEXT,
        decided_at      INTEGER NOT NULL
    )",
    "CREATE TABLE judge_record (
        id              BLOB PRIMARY KEY,
        session_id      BLOB NOT NULL REFERENCES session(id) ON DELETE CASCADE,
        subject_kind    TEXT NOT NULL,
        subject_ref     TEXT NOT NULL,
        verdict         TEXT NOT NULL,
        reason          TEXT,
        judge_provider  TEXT NOT NULL,
        judge_model     TEXT NOT NULL,
        rubric_id       TEXT NOT NULL,
        rubric_version  TEXT NOT NULL,
        evidence_hashes TEXT NOT NULL,
        created_at      INTEGER NOT NULL
    )",
    "CREATE INDEX idx_judge_subject ON judge_record(subject_kind, subject_ref)",
    "CREATE TABLE skill_lock (
        skill_id        BLOB PRIMARY KEY REFERENCES skill(id) ON DELETE CASCADE,
        lock            TEXT NOT NULL,
        set_at          INTEGER NOT NULL,
        set_by          TEXT NOT NULL
    )",
    "CREATE TABLE memory_lock (
        entry_id        BLOB PRIMARY KEY REFERENCES memory_entry(id) ON DELETE CASCADE,
        lock            TEXT NOT NULL,
        set_at          INTEGER NOT NULL,
        set_by          TEXT NOT NULL
    )",
    "CREATE TABLE forge_run (
        id              BLOB PRIMARY KEY,
        skill_name      TEXT NOT NULL,
        started_at      INTEGER NOT NULL,
        ended_at        INTEGER,
        base_version    TEXT NOT NULL,
        candidate_version TEXT,
        examples_count  INTEGER NOT NULL,
        iterations      INTEGER NOT NULL,
        spend_usd_micros INTEGER NOT NULL,
        final_score     REAL,
        status          TEXT NOT NULL
    )",
    "CREATE TABLE marketplace_artifact (
        id              BLOB PRIMARY KEY,
        workspace_id    BLOB NOT NULL REFERENCES workspace(id),
        oci_ref         TEXT NOT NULL,
        digest          TEXT NOT NULL,
        pushed_at       INTEGER NOT NULL,
        pushed_by       TEXT NOT NULL
    )",
    "CREATE TABLE audit_record (
        seq             INTEGER PRIMARY KEY AUTOINCREMENT,
        id              BLOB NOT NULL UNIQUE,
        session_id      BLOB,
        kind            TEXT NOT NULL,
        payload_json    TEXT NOT NULL,
        parent_hash     BLOB NOT NULL,
        self_hash       BLOB NOT NULL,
        created_at      INTEGER NOT NULL
    )",
    "CREATE INDEX idx_audit_session ON audit_record(session_id, seq)",
];

const V1_DOWN: &[&str] = &[
    "DROP TABLE IF EXISTS audit_record",
    "DROP TABLE IF EXISTS marketplace_artifact",
    "DROP TABLE IF EXISTS forge_run",
    "DROP TABLE IF EXISTS memory_lock",
    "DROP TABLE IF EXISTS skill_lock",
    "DROP TABLE IF EXISTS judge_record",
    "DROP TABLE IF EXISTS routing_decision",
    "DROP TABLE IF EXISTS role_binding",
    "DROP TABLE IF EXISTS role",
    "DROP TABLE IF EXISTS agent",
    "DROP TABLE IF EXISTS skill",
    "DROP TABLE IF EXISTS memory_entry",
    "DROP TABLE IF EXISTS tool_call",
    "DROP TRIGGER IF EXISTS message_fts_ad",
    "DROP TRIGGER IF EXISTS message_fts_ai",
    "DROP TABLE IF EXISTS message_fts",
    "DROP TABLE IF EXISTS message",
    "DROP TABLE IF EXISTS session",
    "DROP TABLE IF EXISTS workspace",
];
