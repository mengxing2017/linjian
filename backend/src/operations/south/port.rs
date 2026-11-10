use crate::{
    business::{Actor, Result},
    operations::domain::*,
};
use serde_json::Value;
use uuid::Uuid;
pub enum Command {
    List {
        kind: String,
        q: Query,
    },
    Get {
        kind: String,
        id: Uuid,
    },
    Save {
        kind: String,
        id: Option<Uuid>,
        input: ResourceInput,
    },
    Delete {
        kind: String,
        id: Uuid,
        revision: i64,
    },
    Customer,
    SaveCustomer(CustomerInput),
    FeedbackList(Query),
    FeedbackGet(Uuid),
    FeedbackDelete {
        id: Uuid,
        revision: i64,
    },
    Submit {
        input: FeedbackInput,
        ip: Option<String>,
    },
    Display,
    Event(BannerEvent),
    CustomerMedia(String),
    ResourceMedia(Uuid),
    FeedbackMedia {
        case: Uuid,
        media: Uuid,
    },
}
pub trait Store: Clone + Send + Sync + 'static {
    fn execute(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        c: Command,
    ) -> impl Future<Output = Result<Value>> + Send;
}
