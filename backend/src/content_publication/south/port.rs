use super::super::domain::{Failure, Link, Preparation};
use crate::business::Result;
pub trait MiniappLinkProvider: Send + Sync {
    fn generate(
        &self,
        work: &Preparation,
    ) -> impl std::future::Future<Output = std::result::Result<Link, Failure>> + Send;
}
pub trait PreparationStore {
    fn claim(&self) -> impl std::future::Future<Output = Result<Option<Preparation>>> + Send;
    fn finish(
        &self,
        work: &Preparation,
        result: std::result::Result<Link, Failure>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}
