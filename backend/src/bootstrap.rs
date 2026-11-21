use crate::system::{north::local::SystemService, south::adapter::postgres::PostgresReadiness};
use sqlx::PgPool;
#[derive(Clone, Default)]
pub struct Options {
    pub endpoints: crate::config::origins::PublicEndpoints,
    pub insecure_local_auth: bool,
    pub statistics: crate::statistics::config::Config,
    pub files: crate::media::south::adapter::filesystem::Files,
}
impl Options {
    pub fn from_env(local: bool) -> Result<Self, String> {
        let files = crate::media::south::adapter::filesystem::Files::open(
            crate::media::config::Config::from_env()?,
        )
        .map_err(|_| "media directory unavailable")?;
        Self::from_env_with_files(local, files)
    }
    pub(crate) fn from_env_with_files(
        local: bool,
        files: crate::media::south::adapter::filesystem::Files,
    ) -> Result<Self, String> {
        Ok(Self {
            endpoints: crate::config::origins::PublicEndpoints::from_env(local)?,
            insecure_local_auth: local,
            statistics: crate::statistics::config::Config::from_env()?,
            files,
        })
    }
    pub fn local() -> Self {
        Self::local_with_files(Default::default())
    }
    pub(crate) fn local_with_files(files: crate::media::south::adapter::filesystem::Files) -> Self {
        Self {
            endpoints: crate::config::origins::PublicEndpoints::local_admin(),
            insecure_local_auth: true,
            statistics: Default::default(),
            files,
        }
    }
}
#[derive(Clone)]
pub struct Application {
    pub statistics: crate::statistics::north::local::Service,
    pub activity: crate::statistics::south::adapter::recorder::Recorder,
    pub system: SystemService<PostgresReadiness>,
    pub pool: PgPool,
    pub legacy: crate::legacy::south::adapter::postgres::Repository,
    pub options: Options,
    pub identity: crate::identity::north::local::Service,
    pub schools: crate::schools::north::local::Service,
    pub site: crate::site::north::local::Service,
    pub registration_settings: crate::miniapp_users::north::local::RegistrationSettingsService,
    pub community: crate::community::north::local::Service,
    pub post_seeds: crate::community::north::local::post_seed::Service,
    pub integrations: crate::integrations::north::local::Service,
    pub media: crate::media::north::local::Service,
}
pub fn build_application(pool: PgPool, options: Options) -> Application {
    let files = options.files.clone();
    Application {
        statistics: crate::statistics::north::local::Service::new(
            crate::statistics::south::adapter::postgres::Repository(
                pool.clone(),
                options.statistics.clone(),
            ),
            options.statistics.concurrency,
        ),
        activity: crate::statistics::south::adapter::recorder::Recorder::new(
            options.statistics.queue_capacity,
        ),
        legacy: crate::legacy::south::adapter::postgres::Repository(pool.clone()),
        system: SystemService::new(PostgresReadiness::new(pool.clone())),
        options,
        identity: crate::identity::north::local::Service(
            crate::identity::south::adapter::postgres::Repository(pool.clone()),
        ),
        schools: crate::schools::north::local::Service(
            crate::schools::south::adapter::postgres::Repository(pool.clone()),
        ),
        site: crate::site::north::local::Service(
            crate::site::south::adapter::postgres::Repository(pool.clone(), files.clone()),
        ),
        registration_settings: crate::miniapp_users::north::local::RegistrationSettingsService(
            crate::miniapp_users::south::adapter::postgres::Repository(pool.clone()),
        ),
        post_seeds: crate::community::north::local::post_seed::Service(
            crate::community::south::adapter::postgres::post_seed::Repository(
                pool.clone(),
                files.clone(),
            ),
        ),
        community: crate::community::north::local::Service(
            crate::community::south::adapter::postgres::Repository(pool.clone()),
        ),
        integrations: crate::integrations::north::local::Service::new(pool.clone()),
        media: crate::media::north::local::Service(
            crate::media::south::adapter::postgres::Repository(pool.clone(), files),
        ),
        pool,
    }
}
