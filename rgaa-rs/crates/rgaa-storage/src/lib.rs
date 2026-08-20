pub mod repository;

pub use repository::{Repository, AuditRow, CriterionResultRow, hash_api_key};

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_create_audit() {
        let pool = PgPool::connect("postgres://localhost/rgaa").await.unwrap();
        let repo = Repository::new(&pool);
        let id = repo.create_audit("https://example.test").await.unwrap();
        assert!(!id.is_nil());
    }
}