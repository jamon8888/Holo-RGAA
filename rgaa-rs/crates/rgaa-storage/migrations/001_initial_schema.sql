-- Initial schema for RGAA audit bundle storage
-- Created: 2024

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Audits table - stores audit bundle metadata
CREATE TABLE audits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    url TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    result JSONB,
    schema_version TEXT NOT NULL DEFAULT '1.0',
    audit_id TEXT NOT NULL UNIQUE,
    config JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audits_audit_id ON audits(audit_id);
CREATE INDEX idx_audits_created_at ON audits(created_at DESC);
CREATE INDEX idx_audits_status ON audits(status);

-- Pages table - stores page-level audit results
CREATE TABLE pages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    page_id TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT,
    criteria JSONB NOT NULL DEFAULT '[]',
    findings JSONB NOT NULL DEFAULT '[]',
    errors JSONB NOT NULL DEFAULT '[]',
    completed BOOLEAN NOT NULL DEFAULT FALSE,
    duration_ms BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pages_audit_id ON pages(audit_id);

-- Findings table - stores individual accessibility findings
CREATE TABLE findings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    page_id UUID REFERENCES pages(id) ON DELETE CASCADE,
    finding_id TEXT NOT NULL,
    rule TEXT NOT NULL,
    criterion_id TEXT,
    url TEXT NOT NULL,
    target TEXT NOT NULL,
    component_path TEXT,
    evidence JSONB NOT NULL DEFAULT '[]',
    status TEXT NOT NULL,
    severity TEXT,
    description TEXT,
    remediation TEXT,
    html TEXT,
    details TEXT,
    source TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_findings_audit_id ON findings(audit_id);
CREATE INDEX idx_findings_fingerprint ON findings(fingerprint);
CREATE INDEX idx_findings_rule ON findings(rule);

-- Checkpoints table - stores manual/guided test checkpoints
CREATE TABLE checkpoints (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    checkpoint_id TEXT NOT NULL,
    criterion_id TEXT NOT NULL,
    status TEXT NOT NULL,
    evidence JSONB NOT NULL DEFAULT '[]',
    summary TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_checkpoints_audit_id ON checkpoints(audit_id);

-- Remediation proposals table - stores generated remediation proposals
CREATE TABLE remediation_proposals (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    proposal_id TEXT NOT NULL,
    finding_ids TEXT[] NOT NULL,
    diff TEXT NOT NULL,
    files TEXT[] NOT NULL,
    rationale TEXT NOT NULL,
    risks TEXT[] NOT NULL DEFAULT '{}',
    validation_commands TEXT[] NOT NULL DEFAULT '{}',
    expected_effect TEXT NOT NULL,
    proposal_hash TEXT NOT NULL,
    approval_state TEXT NOT NULL DEFAULT 'Required',
    approval_token TEXT,
    approver TEXT,
    approved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_remediation_proposals_audit_id ON remediation_proposals(audit_id);
CREATE INDEX idx_remediation_proposals_proposal_hash ON remediation_proposals(proposal_hash);

-- Suppressions table - stores finding suppressions with expiry
CREATE TABLE suppressions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    finding_fingerprint TEXT NOT NULL,
    reason TEXT NOT NULL,
    expires_at TIMESTAMPTZ,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_suppressions_audit_id ON suppressions(audit_id);
CREATE INDEX idx_suppressions_fingerprint ON suppressions(finding_fingerprint);
CREATE INDEX idx_suppressions_expires_at ON suppressions(expires_at);

-- Evidence store metadata
CREATE TABLE evidence_store (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    hash TEXT NOT NULL,
    path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_evidence_store_audit_id ON evidence_store(audit_id);
CREATE INDEX idx_evidence_store_hash ON evidence_store(hash);

-- API keys for remote access
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    scopes TEXT[] NOT NULL DEFAULT '{}',
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);

-- Audit bundle uploads (for idempotent uploads)
CREATE TABLE audit_bundle_uploads (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    bundle_hash TEXT NOT NULL,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(audit_id, schema_version, bundle_hash)
);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_audits_updated_at
    BEFORE UPDATE ON audits
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Function to clean expired suppressions
CREATE OR REPLACE FUNCTION clean_expired_suppressions()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM suppressions
    WHERE expires_at IS NOT NULL AND expires_at < NOW()
    RETURNING 1 INTO deleted_count;
    RETURN COALESCE(deleted_count, 0);
END;
$$ LANGUAGE plpgsql;

-- View for audit summaries
CREATE VIEW audit_summary AS
SELECT
    a.id,
    a.audit_id,
    a.url,
    a.status,
    a.schema_version,
    a.created_at,
    a.updated_at,
    COUNT(DISTINCT f.id) AS total_findings,
    COUNT(DISTINCT f.id) FILTER (WHERE f.status = 'Fail') AS failed_count,
    COUNT(DISTINCT f.id) FILTER (WHERE f.status = 'Pass') AS passed_count,
    COUNT(DISTINCT f.id) FILTER (WHERE f.status = 'NeedsReview') AS needs_review_count,
    COUNT(DISTINCT p.id) AS total_pages
FROM audits a
LEFT JOIN findings f ON f.audit_id = a.id
LEFT JOIN pages p ON p.audit_id = a.id
GROUP BY a.id, a.audit_id, a.url, a.status, a.schema_version, a.created_at, a.updated_at;