use super::Service;
use crate::{
    business::{Actor, Result, require},
    schools::{
        domain::school_recommendations::{OnboardingQuery, Query, Recommendation},
        south::port::school_recommendations::Store,
    },
};

impl<R: super::Store + Store> Service<R> {
    pub async fn school_recommendations(
        &self,
        actor: &Actor,
        query: &Query,
    ) -> Result<Vec<Recommendation>> {
        require(!actor.is_admin_browser(), "请使用小程序身份")?;
        query.validate()?;
        self.0.school_recommendations(actor, query).await
    }

    pub async fn onboarding_school_recommendations(
        &self,
        input: &OnboardingQuery,
    ) -> Result<Vec<Recommendation>> {
        let query = input.query()?;
        self.0
            .onboarding_school_recommendations(&input.ticket, &query)
            .await
    }
}
