-- claude-pricing-plans-config — additive `tier` column on the
-- workspace (tenant) table. Defaults to 'free' so existing rows
-- migrate cleanly. Read/write helpers live in `tenant_query.rs`.

ALTER TABLE workspace ADD COLUMN tier TEXT NOT NULL DEFAULT 'free';
