use crate::business::{Error, Result, require};
use crate::outbox::domain::{
    MEDIA_TYPE, ReceiptAttributes, SourceEvent, canonical, retry_seconds, sign,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;
#[derive(Clone)]
pub struct Worker {
    pool: PgPool,
    client: reqwest::Client,
    endpoint: url::Url,
    key_id: String,
    secret: String,
    origin: String,
}
impl Worker {
    pub fn from_env(pool: PgPool) -> Result<Option<Self>> {
        let get = |key| std::env::var(key).unwrap_or_default();
        let endpoint = get("LINJIAN_SOURCE_ENDPOINT");
        let key_id = get("LINJIAN_SOURCE_KEY_ID");
        let secret = get("LINJIAN_SOURCE_SECRET");
        if endpoint.is_empty() && key_id.is_empty() && secret.is_empty() {
            return Ok(None);
        }
        require(
            !key_id.is_empty() && !key_id.contains(['\n', '\r']) && secret.len() >= 32,
            "来源凭据不完整",
        )?;
        let endpoint =
            url::Url::parse(&endpoint).map_err(|_| Error::Invalid("来源地址无效".into()))?;
        require(
            endpoint.scheme() == "https"
                && endpoint.path() == "/api/v1/source-events"
                && endpoint.query().is_none()
                && endpoint.fragment().is_none()
                && endpoint.username().is_empty()
                && endpoint.password().is_none(),
            "来源地址必须为 HTTPS source-events 接口",
        )?;
        let origin = get("LINJIAN_PUBLIC_ORIGIN");
        let public_url =
            url::Url::parse(&origin).map_err(|_| Error::Invalid("论坛公开地址无效".into()))?;
        require(
            public_url.scheme() == "https" && public_url.host_str().is_some(),
            "来源推送必须配置论坛 HTTPS 公开地址",
        )?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| Error::Unavailable)?;
        Ok(Some(Self {
            pool,
            client,
            endpoint,
            key_id,
            secret,
            origin,
        }))
    }
    pub async fn run_once(&self) -> Result<bool> {
        // Hold a row lock through the bounded request. A crash rolls back the claim;
        // an accepted-but-unrecorded response retries the same id/body safely.
        let mut tx = self.pool.begin().await?;
        let row=sqlx::query("SELECT id,payload,request_body,attempts,receipt_id FROM forum_outbox WHERE next_attempt_at<=now() AND (receipt_status IS NULL OR receipt_status NOT IN ('ignored_stale','rejected')) ORDER BY next_attempt_at,created_at,id FOR UPDATE SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await?;
        let Some(row) = row else {
            return Ok(false);
        };
        let id: Uuid = row.get("id");
        let attempts: i32 = row.get("attempts");
        let body = match row.get::<Option<String>, _>("request_body") {
            Some(body) => body,
            None => {
                let event: SourceEvent =
                    serde_json::from_value(row.get("payload")).map_err(|_| Error::Conflict)?;
                let body = event.document(&self.origin)?;
                // Persist before networking, so restarts/config edits never change a retry body.
                sqlx::query("UPDATE forum_outbox SET request_body=$2 WHERE id=$1")
                    .bind(id)
                    .bind(&body)
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                return Ok(true);
            }
        };
        let receipt: Option<String> = row.get("receipt_id");
        let mut endpoint = self.endpoint.clone();
        let method = if let Some(receipt) = receipt.as_deref() {
            require(Uuid::parse_str(receipt).is_ok(), "收件 ID 无效")?;
            endpoint.set_path(&format!("/api/v1/source-receipts/{receipt}"));
            "GET"
        } else {
            "POST"
        };
        let bytes = if method == "POST" {
            body.as_bytes()
        } else {
            b""
        };
        let timestamp = chrono::Utc::now().timestamp().to_string();
        let nonce = Uuid::new_v4().to_string();
        let key = id.to_string();
        let signature = sign(
            self.secret.as_bytes(),
            &canonical(
                &self.key_id,
                method,
                endpoint.path(),
                &timestamp,
                &nonce,
                &key,
                bytes,
            ),
        );
        let request = self
            .client
            .request(
                if method == "POST" {
                    reqwest::Method::POST
                } else {
                    reqwest::Method::GET
                },
                endpoint,
            )
            .header("Accept", MEDIA_TYPE)
            .header("Content-Type", MEDIA_TYPE)
            .header("X-Source-Version", "1")
            .header("X-Source-Key-Id", &self.key_id)
            .header("X-Source-Timestamp", timestamp)
            .header("X-Source-Nonce", nonce)
            .header("Idempotency-Key", key)
            .header("X-Source-Signature", signature)
            .body(bytes.to_vec());
        let result = async {
            let response = request
                .send()
                .await
                .map_err(|_| "transport_error".to_owned())?;
            if !response.status().is_success() {
                return Err(format!("http_{}", response.status().as_u16()));
            }
            if !response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.split(';').next() == Some(MEDIA_TYPE))
            {
                return Err("invalid_media_type".into());
            }
            let document: serde_json::Value = response
                .json()
                .await
                .map_err(|_| "invalid_receipt".to_owned())?;
            let data = &document["data"];
            let id = data["id"]
                .as_str()
                .filter(|v| Uuid::parse_str(v).is_ok())
                .ok_or("invalid_receipt")?;
            if receipt.as_deref().is_some_and(|expected| expected != id) {
                return Err("receipt_identity_mismatch".into());
            }
            if data["type"] != "source-receipts" {
                return Err("invalid_receipt".into());
            }
            let state = data["attributes"]["status"]
                .as_str()
                .filter(|v| ["received", "applied", "ignored_stale", "rejected"].contains(v))
                .ok_or("invalid_receipt_status")?;
            Ok((
                id.to_owned(),
                state.to_owned(),
                serde_json::to_value(
                    serde_json::from_value::<ReceiptAttributes>(data["attributes"].clone())
                        .map_err(|_| "invalid_receipt_summary".to_owned())?,
                )
                .map_err(|_| "invalid_receipt_summary".to_owned())?,
            ))
        }
        .await;
        match result {
            Ok((receipt, state, summary)) => {
                sqlx::query("UPDATE forum_outbox SET attempts=attempts+1,receipt_id=$2,receipt_status=$3,receipt_summary=$4,last_error=NULL,accepted_at=coalesce(accepted_at,now()),next_attempt_at=now()+interval '30 seconds' WHERE id=$1").bind(id).bind(receipt).bind(state).bind(summary).execute(&mut *tx).await?;
            }
            Err(code) => {
                sqlx::query("UPDATE forum_outbox SET attempts=attempts+1,last_error=$2,next_attempt_at=now()+make_interval(secs=>$3) WHERE id=$1").bind(id).bind(code).bind(retry_seconds(attempts) as f64).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        Ok(true)
    }
}
