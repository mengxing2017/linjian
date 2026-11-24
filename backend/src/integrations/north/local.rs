use super::super::south::adapter::postgres::Repository;
use crate::business::{Error, Result, require};
#[derive(Clone)]
pub struct Service {
    pub repository: Repository,
    pub(crate) client: reqwest::Client,
    wx_app: String,
    wx_secret: String,
}
impl Service {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            repository: Repository(pool),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("TLS client"),
            wx_app: std::env::var("LINJIAN_WX_APP_ID").unwrap_or_default(),
            wx_secret: std::env::var("LINJIAN_WX_SECRET").unwrap_or_default(),
        }
    }
    pub async fn wechat_identity(&self, code: &str) -> Result<(String, String)> {
        require(
            !self.wx_app.is_empty() && !self.wx_secret.is_empty(),
            "尚未配置微信小程序",
        )?;
        let mut response = self
            .client
            .get("https://api.weixin.qq.com/sns/jscode2session")
            .query(&[
                ("appid", self.wx_app.as_str()),
                ("secret", self.wx_secret.as_str()),
                ("js_code", code),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        if !response.status().is_success() {
            return Err(Error::Unavailable);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
            if bytes.len() + chunk.len() > 16384 {
                return Err(Error::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        let v = crate::miniapp_users::domain::wechat_identity(&bytes)?;
        Ok((self.wx_app.clone(), v))
    }
}
