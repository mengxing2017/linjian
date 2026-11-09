use super::super::south::adapter::postgres::Repository;
use crate::business::{Error, Result, require};
use serde_json::Value;
#[derive(Clone)]
pub struct Service {
    pub repository: Repository,
    pub(crate) client: reqwest::Client,
    pub corp: String,
    pub agent: String,
    secret: String,
    #[cfg(test)]
    api_origin: Option<String>,
    wx_app: String,
    wx_secret: String,
}
impl Service {
    pub fn new(pool: sqlx::PgPool) -> Self {
        let env = |k| std::env::var(k).unwrap_or_default();
        Self {
            repository: Repository(pool),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("TLS client"),
            corp: env("LINJIAN_WECOM_CORP_ID"),
            agent: env("LINJIAN_WECOM_AGENT_ID"),
            secret: env("LINJIAN_WECOM_SECRET"),
            #[cfg(test)]
            api_origin: None,
            wx_app: env("LINJIAN_WX_APP_ID"),
            wx_secret: env("LINJIAN_WX_SECRET"),
        }
    }
    pub fn server_configured(&self) -> bool {
        !self.corp.trim().is_empty() && !self.secret.trim().is_empty()
    }
    pub fn client_configured(&self) -> bool {
        self.server_configured()
            && self.agent.bytes().all(|b| b.is_ascii_digit())
            && self.agent.parse::<u32>().is_ok_and(|id| id > 0)
    }
    pub fn capabilities(&self, origin: &str, insecure: bool) -> WecomStatus {
        let deployment = !insecure
            && reqwest::Url::parse(origin)
                .is_ok_and(|url| url.scheme() == "https" && url.host_str().is_some());
        WecomStatus {
            server_configured: self.server_configured(),
            client_configured: self.client_configured(),
            oauth_available: self.server_configured() && deployment,
            share_available: false,
        }
    }
    pub(crate) fn credential_scope(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        digest.update(b"linjian:wecom:credentials:v2");
        for value in [&self.corp, &self.secret] {
            digest.update((value.len() as u64).to_be_bytes());
            digest.update(value.as_bytes());
        }
        format!("{:x}", digest.finalize())
    }
    pub(crate) fn cache_key(&self, kind: &str) -> String {
        format!("wecom:v2:{}:{kind}", self.credential_scope())
    }
    pub(crate) fn endpoint(&self, path: &str) -> String {
        #[cfg(test)]
        if let Some(origin) = &self.api_origin {
            return format!("{origin}/cgi-bin/{path}");
        }
        format!("https://qyapi.weixin.qq.com/cgi-bin/{path}")
    }
    async fn json(&self, request: reqwest::RequestBuilder) -> Result<Value> {
        let response = request.send().await.map_err(|_| Error::Unavailable)?;
        if !response.status().is_success() {
            return Err(Error::Unavailable);
        }
        let v: Value = response.json().await.map_err(|_| Error::Unavailable)?;
        if v["errcode"].as_i64().unwrap_or(0) != 0 {
            // Never log the URL, token, secret, OAuth code or upstream response body.
            tracing::warn!(
                errcode = v["errcode"].as_i64(),
                "official platform API rejected request"
            );
            return Err(Error::Upstream(v["errcode"].as_i64().unwrap_or(-1)));
        }
        Ok(v)
    }
    pub(crate) async fn token(&self) -> Result<String> {
        if !self.server_configured() {
            return Err(Error::Unavailable);
        }
        let key = self.cache_key("token");
        self.repository
            .cached_or_fetch(&key, || async {
                let v = self
                    .json(
                        self.client
                            .get(self.endpoint("gettoken"))
                            .query(&[("corpid", &self.corp), ("corpsecret", &self.secret)]),
                    )
                    .await?;
                Ok((
                    v["access_token"]
                        .as_str()
                        .ok_or(Error::Unavailable)?
                        .to_owned(),
                    v["expires_in"].as_i64().unwrap_or(7200) - 300,
                ))
            })
            .await
    }
    pub async fn oauth_member(&self, code: &str) -> Result<(uuid::Uuid, String, i64)> {
        let token = self.token().await?;
        let v = self
            .json(
                self.client
                    .get(self.endpoint("auth/getuserinfo"))
                    .query(&[("access_token", token.as_str()), ("code", code)]),
            )
            .await?;
        let userid = v["userid"]
            .as_str()
            .or(v["UserId"].as_str())
            .ok_or(Error::Forbidden)?;
        let generation = self.directory().verify(userid).await?;
        let id = self
            .repository
            .member(
                v["userid"]
                    .as_str()
                    .or(v["UserId"].as_str())
                    .ok_or(Error::Forbidden)?,
            )
            .await?;
        Ok((id, userid.to_owned(), generation))
    }
    pub async fn miniapp(&self, code: &str) -> Result<uuid::Uuid> {
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
        crate::miniapp_users::north::local::register(
            &crate::miniapp_users::south::adapter::postgres::Repository(self.repository.0.clone()),
            &self.wx_app,
            &v,
        )
        .await
    }
}

impl Service {
    pub fn directory(&self) -> super::directory::Service {
        super::directory::Service {
            store: crate::integrations::south::adapter::directory_postgres::Repository(
                self.repository.0.clone(),
            ),
            client: crate::integrations::south::adapter::directory_http::Client(self.clone()),
            corp: self.corp.clone(),
            credential_scope: self.credential_scope(),
            server_configured: self.server_configured(),
        }
    }
}

#[derive(serde::Serialize)]
pub struct WecomStatus {
    pub server_configured: bool,
    pub client_configured: bool,
    pub oauth_available: bool,
    pub share_available: bool,
}
