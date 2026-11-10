use crate::{
    business::{Actor, Result},
    operations::south::{
        adapter::postgres::Repository,
        port::{Command, Store},
    },
};
use serde_json::Value;
use uuid::Uuid;
#[derive(Clone)]
pub struct Service<R: Store = Repository>(pub R);
impl<R: Store> Service<R> {
    pub async fn execute(&self, a: &Actor, s: Option<Uuid>, mut c: Command) -> Result<Value> {
        match &mut c {
            Command::List { q, .. } | Command::FeedbackList(q) => q.validate()?,
            Command::Save { kind, input, .. } => input.validate(kind)?,
            Command::SaveCustomer(v) => v.validate()?,
            Command::Submit { input, .. } => input.validate()?,
            Command::Event(v) => v.validate()?,
            _ => {}
        }
        self.0.execute(a, s, c).await
    }
}
