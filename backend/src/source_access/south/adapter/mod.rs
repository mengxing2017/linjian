pub mod postgres;
impl From<sqlx::Error> for crate::source_access::domain::Fault {
    fn from(error: sqlx::Error) -> Self {
        // Database messages may contain row values; only record the error category and SQLSTATE.
        tracing::error!(
            database_code = ?error.as_database_error().and_then(|e| e.code()),
            pool_timeout = matches!(error, sqlx::Error::PoolTimedOut),
            io = matches!(error, sqlx::Error::Io(_)),
            "source access database operation failed"
        );
        Self("temporarily_unavailable")
    }
}
