//! Fixed official endpoints. Never surface reqwest errors (they may contain credentials).
use crate::{
    business::hash,
    content_publication::{
        domain::{Failure, Link, Preparation},
        south::port::MiniappLinkProvider,
    },
};
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;
#[derive(Clone)]
pub struct Client {
    pool: PgPool,
    http: reqwest::Client,
    app_id: String,
    secret: String,
}
impl Client {
    pub fn from_env(pool: PgPool) -> crate::business::Result<Self> {
        Ok(Self {
            pool,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| crate::business::Error::Unavailable)?,
            app_id: std::env::var("LINJIAN_WX_APP_ID").unwrap_or_default(),
            secret: std::env::var("LINJIAN_WX_SECRET").unwrap_or_default(),
        })
    }
    async fn json(&self, request: reqwest::RequestBuilder) -> Result<Value, Failure> {
        let mut response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                Failure::transient("timeout")
            } else {
                Failure::transient("network")
            }
        })?;
        if !response.status().is_success() {
            return Err(Failure::transient("upstream_unavailable"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Failure::transient("network"))?
        {
            if bytes.len() + chunk.len() > 32768 {
                return Err(Failure::blocked("invalid_response"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let v: Value =
            serde_json::from_slice(&bytes).map_err(|_| Failure::blocked("invalid_response"))?;
        if let Some(code) = v.get("errcode") {
            let code = code.as_i64().ok_or(Failure::blocked("invalid_response"))?;
            if code != 0 {
                return Err(Failure::upstream(code));
            }
        }
        Ok(v)
    }
    async fn token(&self) -> Result<(String, Uuid), Failure> {
        let scope =
            hash(&json!(["linjian:miniapp:stable-token:v1", self.app_id, self.secret]).to_string());
        let db = |_| Failure::transient("database_unavailable");
        sqlx::query(
            "INSERT INTO miniapp_token_cache(credential_scope) VALUES($1) ON CONFLICT DO NOTHING",
        )
        .bind(&scope)
        .execute(&self.pool)
        .await
        .map_err(db)?;
        let row=sqlx::query("SELECT token,generation FROM miniapp_token_cache WHERE credential_scope=$1 AND expires_at>now() AND token IS NOT NULL").bind(&scope).fetch_optional(&self.pool).await.map_err(db)?;
        if let Some(row) = row {
            return Ok((row.get("token"), row.get("generation")));
        }
        let lease = Uuid::new_v4();
        let n=sqlx::query("UPDATE miniapp_token_cache SET lease_token=$2,lease_until=now()+interval '30 seconds' WHERE credential_scope=$1 AND (lease_until IS NULL OR lease_until<now()) AND (expires_at IS NULL OR expires_at<=now() OR token IS NULL)").bind(&scope).bind(lease).execute(&self.pool).await.map_err(db)?.rows_affected();
        if n == 0 {
            return Err(Failure::transient("token_refresh_in_progress"));
        }
        let started = Utc::now();
        let result=self.json(self.http.post("https://api.weixin.qq.com/cgi-bin/stable_token").json(&json!({"grant_type":"client_credential","appid":self.app_id,"secret":self.secret,"force_refresh":false}))).await;
        let result = result.and_then(|value| {
            let token = value["access_token"]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 4096)
                .ok_or(Failure::blocked("invalid_response"))?
                .to_owned();
            let ttl = value["expires_in"]
                .as_i64()
                .filter(|s| *s > 0 && *s <= 86400)
                .ok_or(Failure::blocked("invalid_response"))?;
            Ok((
                token,
                started + Duration::seconds(ttl - (ttl / 10).min(300)),
            ))
        });
        match result {
            Ok((token, expires)) => {
                let generation = Uuid::new_v4();
                let n=sqlx::query("UPDATE miniapp_token_cache SET token=$3,expires_at=$4,generation=$5,lease_token=NULL,lease_until=NULL WHERE credential_scope=$1 AND lease_token=$2 AND lease_until>now()")
                    .bind(&scope).bind(lease).bind(&token).bind(expires).bind(generation).execute(&self.pool).await.map_err(db)?.rows_affected();
                if n == 0 {
                    return Err(Failure::transient("token_refresh_in_progress"));
                }
                Ok((token, generation))
            }
            Err(error) => {
                sqlx::query("UPDATE miniapp_token_cache SET lease_token=NULL,lease_until=NULL WHERE credential_scope=$1 AND lease_token=$2").bind(&scope).bind(lease).execute(&self.pool).await.map_err(db)?;
                Err(error)
            }
        }
    }
    async fn invalidate(&self, generation: Uuid) -> Result<(), Failure> {
        sqlx::query(
            "UPDATE miniapp_token_cache SET token=NULL,expires_at=NULL WHERE generation=$1",
        )
        .bind(generation)
        .execute(&self.pool)
        .await
        .map_err(|_| Failure::transient("database_unavailable"))?;
        Ok(())
    }
}
impl MiniappLinkProvider for Client {
    async fn generate(&self, w: &Preparation) -> Result<Link, Failure> {
        if self.app_id.is_empty() || self.secret.is_empty() {
            return Err(Failure::blocked("miniapp_not_configured"));
        }
        if w.policy.app_id != self.app_id {
            return Err(Failure::blocked("appid_mismatch"));
        }
        let query = format!("school={}&id={}", w.school, w.post);
        let title = w.title.chars().take(20).collect::<String>();
        for attempt in 0..2 {
            let (token, generation) = self.token().await?;
            let started = Utc::now();
            let (endpoint, input, field) = if w.policy.link_type == "short_link" {
                (
                    "https://api.weixin.qq.com/wxa/genwxashortlink",
                    json!({"page_url":format!("pages/index/detail?{query}"),"page_title":title,"is_permanent":w.policy.permanent}),
                    "link",
                )
            } else {
                (
                    "https://api.weixin.qq.com/wxa/generate_urllink",
                    json!({"path":"pages/index/detail","query":query,"env_version":"release","expire_type":1,"expire_interval":w.policy.days}),
                    "url_link",
                )
            };
            let response = self
                .json(
                    self.http
                        .post(endpoint)
                        .query(&[("access_token", token)])
                        .json(&input),
                )
                .await;
            match response {
                Err(error) if error.code == "token_invalid" && attempt == 0 => {
                    self.invalidate(generation).await?;
                }
                Err(error) => return Err(error),
                Ok(value) => {
                    let link = value[field]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .ok_or(Failure::blocked("invalid_response"))?;
                    if link.chars().count() > 4096 {
                        return Err(Failure::blocked("link_too_long"));
                    }
                    if link.chars().any(|c| c.is_control()) {
                        return Err(Failure::blocked("invalid_response"));
                    }
                    let valid = if field == "link" {
                        link.starts_with("#小程序://")
                    } else {
                        url::Url::parse(link).is_ok_and(|u| {
                            u.scheme() == "https"
                                && u.host_str().is_some()
                                && u.username().is_empty()
                                && u.password().is_none()
                        })
                    };
                    if !valid {
                        return Err(Failure::blocked("invalid_response"));
                    }
                    return Ok(Link {
                        value: link.into(),
                        generated_at: started,
                        expires_at: if w.policy.permanent {
                            None
                        } else {
                            Some(
                                started
                                    + Duration::days(if field == "link" {
                                        30
                                    } else {
                                        w.policy.days
                                    }),
                            )
                        },
                    });
                }
            }
        }
        Err(Failure::blocked("token_invalid"))
    }
}
