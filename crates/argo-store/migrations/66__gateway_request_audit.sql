CREATE TABLE IF NOT EXISTS gateway_request_audit (
    request_id BLOB PRIMARY KEY NOT NULL,
    received_at INTEGER NOT NULL,
    principal TEXT,
    source_ip TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    latency_ms INTEGER NOT NULL,
    request_body_sha256 BLOB,
    response_body_len INTEGER NOT NULL,
    user_agent TEXT
);

CREATE INDEX IF NOT EXISTS idx_gateway_request_audit_received_at
    ON gateway_request_audit(received_at);
CREATE INDEX IF NOT EXISTS idx_gateway_request_audit_principal
    ON gateway_request_audit(principal) WHERE principal IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_gateway_request_audit_status
    ON gateway_request_audit(status_code);
