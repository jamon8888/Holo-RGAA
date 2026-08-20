-- Migration 002: Audit bundles and findings with full normalization
-- Adds tables for normalized finding storage and bundle versioning

-- Normalized findings table with structured fields for querying
CREATE TABLE findings_normalized (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    finding_id TEXT NOT NULL,
    rule TEXT NOT NULL,
    criterion_id TEXT,
    url TEXT NOT NULL,
    target TEXT NOT NULL,
    component_path TEXT,
    status TEXT NOT NULL,
    severity TEXT,
    fingerprint TEXT NOT NULL,
    evidence_kind TEXT[],
    evidence_hash TEXT[],
    source TEXT NOT NULL,
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(audit_id, finding_id)
);

CREATE INDEX idx_findings_norm_audit_id ON findings_normalized(audit_id);
CREATE INDEX idx_findings_norm_fingerprint ON findings_normalized(fingerprint);
CREATE INDEX idx_findings_norm_rule ON findings_normalized(rule);
CREATE INDEX idx_findings_norm_status ON findings_normalized(status);

-- Evidence references with full paths
CREATE TABLE evidence_refs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    finding_id UUID REFERENCES findings_normalized(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    hash TEXT NOT NULL,
    location TEXT,
    path TEXT,
    sha256 TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_evidence_refs_audit_id ON evidence_refs(audit_id);
CREATE INDEX idx_evidence_refs_finding_id ON evidence_refs(finding_id);

-- Bundle versions for history
CREATE TABLE audit_bundle_versions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    bundle_hash TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    uploaded_by UUID REFERENCES api_keys(id),
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(audit_id, version)
);

CREATE INDEX idx_bundle_versions_audit_id ON audit_bundle_versions(audit_id);

-- Finding lifecycle events
CREATE TABLE finding_lifecycle (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    finding_fingerprint TEXT NOT NULL,
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    state TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT,
    proposal_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_finding_lifecycle_fingerprint ON finding_lifecycle(finding_fingerprint);
CREATE INDEX idx_finding_lifecycle_audit_id ON finding_lifecycle(audit_id);

-- Policy evaluations history
CREATE TABLE policy_evaluations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    baseline_id UUID REFERENCES audits(id),
    passed BOOLEAN NOT NULL,
    failures JSONB NOT NULL DEFAULT '[]',
    warnings JSONB NOT NULL DEFAULT '[]',
    counts JSONB NOT NULL DEFAULT '{}',
    evaluated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_policy_eval_audit_id ON policy_evaluations(audit_id);

-- Guided test runs
CREATE TABLE guided_test_runs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    test_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    terminated_reason TEXT NOT NULL,
    completed_steps INTEGER NOT NULL DEFAULT 0,
    issues JSONB NOT NULL DEFAULT '[]',
    unanalyzed_elements JSONB NOT NULL DEFAULT '[]',
    evidence JSONB NOT NULL DEFAULT '[]',
    manual_review_required BOOLEAN NOT NULL DEFAULT FALSE,
    run_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_guided_test_runs_audit_id ON guided_test_runs(audit_id);

-- Remediation batch tracking
CREATE TABLE remediation_batches (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    proposal_ids UUID[] NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    applied_by UUID REFERENCES api_keys(id),
    applied_at TIMESTAMPTZ,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_remediation_batches_audit_id ON remediation_batches(audit_id);

-- Update triggers for new tables
CREATE TRIGGER update_findings_norm_updated_at
    BEFORE UPDATE ON findings_normalized
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_evidence_refs_updated_at
    BEFORE UPDATE ON evidence_refs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Indexes for common queries
CREATE INDEX idx_findings_norm_audit_status ON findings_normalized(audit_id, status);
CREATE INDEX idx_findings_norm_rule_status ON findings_normalized(rule, status);
CREATE INDEX idx_evidence_refs_finding_hash ON evidence_refs(finding_id, hash);
CREATE INDEX idx_guided_test_runs_test_id ON guided_test_runs(test_id);
CREATE INDEX idx_remediation_batches_status ON remediation_batches(status);