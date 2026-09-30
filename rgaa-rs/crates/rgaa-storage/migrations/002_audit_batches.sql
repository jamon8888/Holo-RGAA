-- Batch audit state for the REST batch endpoints (#167).
-- The whole `BatchRecord` lives in one JSONB document: it is read and
-- written only by rgaa-api's batch module, never joined or filtered on in
-- SQL, so a column per progress field would buy nothing but a migration
-- every time the record gains a field. The two fields the server does query
-- on are lifted out as columns.
CREATE TABLE IF NOT EXISTS audit_batches (
    batch_id TEXT PRIMARY KEY,
    record JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    -- Expiry is enforced on read from the record itself; this column exists
    -- so the periodic purge can delete without deserializing every row.
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS audit_batches_expires_at_idx ON audit_batches (expires_at);
