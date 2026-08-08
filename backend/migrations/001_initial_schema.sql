-- Migration: Initial schema for RGAA audit platform
-- Run with: sqlx migrate run

-- Custom types
CREATE TYPE audit_status AS ENUM ('pending', 'running', 'completed', 'failed');
CREATE TYPE classification AS ENUM ('deterministe', 'ia_assiste', 'manuel');
CREATE TYPE criterion_status AS ENUM ('pass', 'fail', 'na', 'error');

-- Audits table
CREATE TABLE audits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    url TEXT NOT NULL,
    status audit_status NOT NULL DEFAULT 'pending',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    total_criteria INT NOT NULL DEFAULT 0,
    passed_criteria INT NOT NULL DEFAULT 0,
    failed_criteria INT NOT NULL DEFAULT 0,
    na_criteria INT NOT NULL DEFAULT 0,
    compliance_rate DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Criterion results table
CREATE TABLE criterion_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    audit_id UUID NOT NULL REFERENCES audits(id) ON DELETE CASCADE,
    criterion_id TEXT NOT NULL,        -- ex: "1.1", "9.1"
    criterion_title TEXT NOT NULL,
    classification classification NOT NULL,
    status criterion_status NOT NULL,
    axe_rule TEXT,                     -- axe-core rule id that triggered
    impact TEXT,                       -- critical, serious, moderate, minor
    description TEXT,                  -- human readable description
    nodes_affected INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_criterion_results_audit_id ON criterion_results(audit_id);
CREATE INDEX idx_audits_created_at ON audits(created_at DESC);
CREATE INDEX idx_audits_status ON audits(status);

-- Sample data for testing
INSERT INTO audits (id, url, status, total_criteria, passed_criteria, failed_criteria, na_criteria, compliance_rate)
VALUES 
    ('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11', 'https://www.service-public.fr', 'completed', 77, 70, 4, 3, 94.6),
    ('b0eebc99-9c0b-4ef8-bb6d-6bb9bd380a12', 'https://example.com', 'completed', 77, 60, 4, 13, 93.8)
ON CONFLICT (id) DO NOTHING;