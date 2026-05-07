-- Stream `claude-worker-charter-storage` (mission claim 2: delegation contract).
-- Per-worker-instance charter (markdown body, ≤ 16 KB) and KPIs (free-form
-- JSON, ≤ 8 KB). Optional / nullable so older rows continue to load via
-- `#[serde(default)]`.
ALTER TABLE worker ADD COLUMN charter_md TEXT;
ALTER TABLE worker ADD COLUMN kpi_json TEXT;
