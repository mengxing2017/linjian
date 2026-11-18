use crate::{
    business::{Actor, Result},
    statistics::domain::{Query, Report},
};
use uuid::Uuid;
pub trait StatisticsStore: Clone + Send + Sync {
    fn report(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        query: Query,
    ) -> impl std::future::Future<Output = Result<Report>> + Send;
    fn validate_activity(
        &self,
        actor: &Actor,
        input: &crate::statistics::domain::ActivityInput,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn snapshot(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<Report>> + Send;
}
pub trait ActivityRecorder {
    fn record(&self, event: crate::statistics::domain::ActivityEvent);
}
