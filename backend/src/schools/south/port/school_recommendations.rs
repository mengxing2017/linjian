use crate::{
    business::{Actor, Result},
    schools::domain::school_recommendations::{Query, Recommendation},
};

pub trait Store: Clone + Send + Sync {
    fn school_recommendations(
        &self,
        actor: &Actor,
        query: &Query,
    ) -> impl Future<Output = Result<Vec<Recommendation>>> + Send;
    fn onboarding_school_recommendations(
        &self,
        ticket: &str,
        query: &Query,
    ) -> impl Future<Output = Result<Vec<Recommendation>>> + Send;
}
