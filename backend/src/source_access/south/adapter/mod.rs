pub mod postgres;
impl From<sqlx::Error> for crate::source_access::domain::Fault {
    fn from(_: sqlx::Error) -> Self {
        Self("temporarily_unavailable")
    }
}
