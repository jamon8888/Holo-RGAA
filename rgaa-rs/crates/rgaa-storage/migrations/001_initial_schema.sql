-- Union schema for every live rgaa-api code path.
-- Legacy Storage trait (save_audit/get_audit/list_audits/logs) + /v1
-- (put_bundle, get_bundle, list_findings, api_keys auth).
-- Idempotent: safe to re-run manually; sqlx tracks it at startup
-- (PostgresStorage::new). Ids are bound from Rust — no uuid extension needed.

CREATE TABLE IF NOT EXISTS audits (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    data JSONB NOT NULL,
    taux_global DOUBLE PRECISION NOT NULL,
    etat_conformite TEXT NOT NULL,
    schema_version TEXT NOT NULL DEFAULT '1.0',
    audit_id TEXT,
    config JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Converge a legacy self-created `audits` (pre-union startup DDL) in place.
ALTER TABLE audits ADD COLUMN IF NOT EXISTS schema_version TEXT NOT NULL DEFAULT '1.0';
ALTER TABLE audits ADD COLUMN IF NOT EXISTS audit_id TEXT;
ALTER TABLE audits ADD COLUMN IF NOT EXISTS config JSONB;
ALTER TABLE audits ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'audits' AND column_name = 'taux_global'
          AND data_type = 'real'
    ) THEN
        ALTER TABLE audits
            ALTER COLUMN taux_global TYPE DOUBLE PRECISION
            USING taux_global::double precision;
    END IF;
END $$;

-- Legacy rows keep audit_id NULL; put_bundle upserts on PK id (= bundle.audit_id).

CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    audit_id TEXT NOT NULL,
    action TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    details JSONB
);

-- /v1/findings rows written by put_bundle. audit_id is TEXT (bundle.audit_id
-- is a String on every bind path); no FK — legacy rows carry NULL audit_id.
CREATE TABLE IF NOT EXISTS findings (
    id UUID PRIMARY KEY,
    audit_id TEXT NOT NULL,
    finding_id TEXT NOT NULL,
    rule TEXT NOT NULL,
    criterion_id TEXT,
    url TEXT NOT NULL,
    target TEXT NOT NULL,
    component_path TEXT,
    status TEXT NOT NULL,
    severity TEXT,
    fingerprint TEXT NOT NULL,
    evidence_kind TEXT[] NOT NULL DEFAULT '{}',
    evidence_hash TEXT[] NOT NULL DEFAULT '{}',
    source TEXT NOT NULL,
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (audit_id, finding_id)
);

CREATE INDEX IF NOT EXISTS idx_findings_audit_id ON findings (audit_id);
CREATE INDEX IF NOT EXISTS idx_findings_rule ON findings (rule);

CREATE TABLE IF NOT EXISTS checkpoints (
    id UUID PRIMARY KEY,
    audit_id TEXT NOT NULL,
    checkpoint_id TEXT NOT NULL,
    criterion_id TEXT NOT NULL,
    status TEXT NOT NULL,
    evidence JSONB NOT NULL DEFAULT '[]',
    summary TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (audit_id, checkpoint_id)
);

CREATE TABLE IF NOT EXISTS audit_bundle_versions (
    id UUID PRIMARY KEY,
    audit_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    bundle_hash TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (audit_id, version)
);

CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY,
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    scopes TEXT[] NOT NULL DEFAULT '{}',
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);
