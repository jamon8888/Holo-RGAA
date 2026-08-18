# Task 5 Review: PostgreSQL Storage + Axum REST API

## Spec Compliance

**Spec ✅**

All specified constraints are met:

- **rgaa-storage**: sqlx 0.7 with PostgreSQL, `Repository` struct with CRUD methods (`create_audit`, `update_audit_status`, `complete_audit`, `store_criterion_results`, `get_audit`, `list_audits`)
- **rgaa-api**: Axum REST with all 4 required endpoints: `POST /audits`, `GET /audits/:id`, `GET /audits`, `GET /health`
- **Workspace**: Both `crates/rgaa-storage` and `crates/rgaa-api` present in workspace `Cargo.toml` members list
- **Crate types**: `rgaa-storage` is a library crate, `rgaa-api` is a binary crate
- **Compilation**: `cargo check` passes successfully

## Quality

**Quality ❌**

### Critical Issues

1. **Dead code** — `CriterionResultRow` (repository.rs:19-32) is defined but never used. Remove it or use it for a proper `get_criterion_results` query.

2. **N+1 INSERT in `store_criterion_results`** (repository.rs:93-115) — Executes one INSERT per criterion in a loop. For 106 criteria this is 106 round-trips. Use a single batch INSERT with `sqlx::query` + `push`/`bind` in a loop, or `sqlx::query_builder` for a bulk insert.

3. **Debug formatting for enums** (repository.rs:106-107) — `format!("{:?}", criterion.classification)` and `format!("{:?}", criterion.status)` produce Rust debug output (`Deterministe`, `Pass`, `Na`, `Error`). Use `Display` impl or explicit match to produce clean values. The `Na` variant is particularly confusing for downstream consumers.

4. **No database migrations** — The code assumes `audits` and `criterion_results` tables exist. No migration files, no schema setup script. This makes the crate unusable without manual DB prep.

### Moderate Issues

5. **No tests** — Zero unit or integration tests. `Repository` methods are all async and require PostgreSQL. At minimum, add `#[cfg(test)]` module with test fixtures using `sqlx::test` macro or a `testcontainers` setup.

6. **`anyhow::Result` for library errors** — `rgaa-storage` is a library crate using `anyhow::Result` everywhere. This makes it impossible for callers to match on specific error variants. Define a dedicated `StorageError` enum (e.g., `thiserror`-based) with variants for `Pool`, `RowNotFound`, `Serialization`, etc.

7. **Partial failure in `store_criterion_results`** — If criterion INSERT #50 fails, the first 49 are already committed. There's no transaction wrapping. Use `PgPool::begin()` + `tx.commit()` for atomicity.

8. **No input validation on URL** — `POST /audits` accepts any string as `url`. No validation, no sanitization, no length limit.

### Minor Issues

9. **`tower-http` version mismatch** — `rgaa-api/Cargo.toml` uses `tower-http = "0.5"` but the workspace already has `tower-http 0.6.11` in the lockfile from `reqwest`. Consider upgrading to `0.6` for consistency.

10. **CORS allows all origins** (main.rs:121-124) — `allow_origin(Any)` is fine for development but should be configurable or restricted in production.

11. **`list_audits` default limit is 20** (main.rs:85) — No maximum cap. A client can pass `limit=999999` and dump the entire table. Add an upper bound.

12. **`AuditResult.audit_id` is `String`** (rgaa-core/types.rs:49) while the storage layer uses `Uuid`. This is a type mismatch that should be reconciled in rgaa-core.

13. **No structured error responses** — API errors return bare strings (`(StatusCode, String)`). Should return a JSON error body with `error`, `message`, `code` fields for proper client handling.

14. **Future incompatibility warning** — `sqlx-postgres v0.7.4` has code that will be rejected by future Rust versions. The report notes this but no mitigation is in place.

## Verdict

- **Spec ✅** — All stated constraints satisfied.
- **Quality ❌** — Compiles and structurally correct, but has dead code, no migrations, no tests, no error types, N+1 queries, and no transaction safety. Needs a follow-up pass before production use.
