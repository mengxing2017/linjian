use crate::system::{north::local::SystemService, south::adapter::postgres::PostgresReadiness};
use sqlx::PgPool;
#[derive(Clone, Default)]
pub struct Options {
    pub endpoints: crate::config::origins::PublicEndpoints,
    pub insecure_local_auth: bool,
}
impl Options {
    pub fn from_env(local: bool) -> Result<Self, String> {
        Ok(Self {
            endpoints: crate::config::origins::PublicEndpoints::from_env(local)?,
            insecure_local_auth: local,
        })
    }
    pub fn local() -> Self {
        Self {
            endpoints: crate::config::origins::PublicEndpoints::local_admin(),
            insecure_local_auth: true,
        }
    }
}
#[derive(Clone)]
pub struct Application {
    pub system: SystemService<PostgresReadiness>,
    pub pool: PgPool,
    pub legacy: crate::legacy::south::adapter::postgres::Repository,
    pub options: Options,
    pub identity: crate::identity::north::local::Service,
    pub schools: crate::schools::north::local::Service,
    pub community: crate::community::north::local::Service,
    pub integrations: crate::integrations::north::local::Service,
    pub media: crate::media::north::local::Service,
}
pub fn build_application(pool: PgPool, options: Options) -> Application {
    Application {
        legacy: crate::legacy::south::adapter::postgres::Repository(pool.clone()),
        system: SystemService::new(PostgresReadiness::new(pool.clone())),
        options,
        identity: crate::identity::north::local::Service(
            crate::identity::south::adapter::postgres::Repository(pool.clone()),
        ),
        schools: crate::schools::north::local::Service(
            crate::schools::south::adapter::postgres::Repository(pool.clone()),
        ),
        community: crate::community::north::local::Service(
            crate::community::south::adapter::postgres::Repository(pool.clone()),
        ),
        integrations: crate::integrations::north::local::Service::new(pool.clone()),
        media: crate::media::north::local::Service(
            crate::media::south::adapter::postgres::Repository(pool.clone()),
        ),
        pool,
    }
}
