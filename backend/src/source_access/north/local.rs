use crate::source_access::south::port::CredentialStore;
use crate::{
    business::Actor,
    source_access::{
        configuration::ConnectionConfig, domain::*, south::adapter::postgres::Repository,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionInput {}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingInput {
    pub route_id: String,
    pub active: bool,
}
#[derive(Serialize)]
pub struct SchoolRoute {
    pub id: Uuid,
    pub route_id: String,
    pub active: bool,
    pub revision: i64,
    pub checked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Serialize)]
pub struct SchoolRouteSettings {
    pub binding: Option<SchoolRoute>,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub backlog: std::collections::BTreeMap<String, i64>,
}
#[derive(Serialize)]
pub struct ConnectionSettings {
    pub configured: bool,
    pub origin: Option<String>,
    pub key_id: Option<String>,
    pub connection: Option<Value>,
}
#[derive(Clone)]
pub struct Service(pub PgPool);
impl Service {
    pub async fn list(&self, a: &Actor) -> Result<ConnectionSettings> {
        ensure(a.platform_admin, "forbidden")?;
        let config = ConnectionConfig::from_env()?;
        let connection = if let Some(config) = &config {
            Repository(self.0.clone())
                .configured_connection(config)
                .await?
        } else {
            None
        };
        Ok(ConnectionSettings {
            configured: config.is_some(),
            origin: config.as_ref().map(|c| c.origin.clone()),
            key_id: config.as_ref().map(|c| c.key_id.clone()),
            connection,
        })
    }
    pub async fn save(&self, a: &Actor, _: ConnectionInput) -> Result<Value> {
        ensure(a.platform_admin, "forbidden")?;
        let config = ConnectionConfig::from_env()?.ok_or(Fault("wekeeper_not_configured"))?;
        let capability = check(
            &config.origin,
            &config.key_id,
            &config.material(&config.origin, &config.key_id)?,
            "/api/v2/source-authentications",
            None,
        )
        .await?;
        ensure(
            capability["schema_versions"]
                .as_array()
                .is_some_and(|v| v.contains(&json!(2)))
                && capability["signature_versions"]
                    .as_array()
                    .is_some_and(|v| v.contains(&json!(2)))
                && capability["paths"]["events"] == "/api/v2/source-events",
            "unsupported_version",
        )?;
        let source = authenticated_caller(&capability)?;
        Repository(self.0.clone())
            .save(a, config, source, capability)
            .await
    }
    pub async fn bind(
        &self,
        a: &Actor,
        school: Uuid,
        v: BindingInput,
        expected: i64,
    ) -> Result<Value> {
        ensure(a.platform_admin, "forbidden")?;
        ensure(identifier(&v.route_id), "invalid_request")?;
        let repo = Repository(self.0.clone());
        // Disabling an existing school route must also work without upstream availability/configuration.
        let existing = if !v.active {
            repo.school_connection(school).await?
        } else {
            None
        };
        let connection_id = if let Some(id) = existing {
            id
        } else {
            let config = ConnectionConfig::from_env()?.ok_or(Fault("wekeeper_not_configured"))?;
            let current = repo
                .configured_connection(&config)
                .await?
                .ok_or(Fault("wekeeper_connection_unverified"))?;
            current["id"]
                .as_str()
                .and_then(|id| id.parse().ok())
                .ok_or(Fault("protocol_error"))?
        };
        let conn = repo.connection(connection_id).await?;
        if v.active {
            let raw = Repository(self.0.clone())
                .load_material(connection_id)
                .await?;
            let cap = check(
                conn["origin"].as_str().ok_or(Fault("invalid_request"))?,
                conn["key_id"].as_str().ok_or(Fault("invalid_request"))?,
                &raw,
                "/api/v2/source-route-checks",
                Some(&v.route_id),
            )
            .await?;
            ensure(
                cap["routes"].as_array().is_some_and(|rs| {
                    rs.iter()
                        .any(|r| r["route_id"] == v.route_id && r["available"] == true)
                }),
                "route_unavailable",
            )?;
        }
        repo.bind(
            a,
            school,
            connection_id,
            v,
            expected,
            conn["revision"].as_i64().ok_or(Fault("protocol_error"))?,
        )
        .await
    }
}
pub fn authenticated_caller(capability: &Value) -> Result<Uuid> {
    capability["source_id"]
        .as_str()
        .and_then(|v| v.parse().ok())
        .ok_or(Fault("protocol_error"))
}
pub async fn check(
    origin: &str,
    key: &str,
    secret_text: &str,
    path: &str,
    route: Option<&str>,
) -> Result<Value> {
    let id = Uuid::new_v4();
    let doc = json!({"data":{"type":if path.ends_with("source-authentications"){"source-authentications"}else{"source-route-checks"},"id":id,"attributes":{"route_id":route}}});
    let body = serde_json::to_vec(&doc).map_err(|_| Fault("invalid_request"))?;
    let response = request(origin, key, secret_text, "POST", path, id, &body).await?;
    ensure(response.status().is_success(), "authentication_failed")?;
    ensure(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.split(';').next() == Some(MEDIA_TYPE)),
        "protocol_error",
    )?;
    let doc: Value = response.json().await.map_err(|_| Fault("protocol_error"))?;
    ensure(
        doc["data"]["id"] == id.to_string()
            && doc["data"]["type"]
                == if path.ends_with("source-authentications") {
                    "source-authentications"
                } else {
                    "source-route-checks"
                },
        "protocol_error",
    )?;
    Ok(doc["data"]["attributes"].clone())
}
pub async fn request(
    base: &str,
    key: &str,
    secret_text: &str,
    method: &str,
    path: &str,
    id: Uuid,
    body: &[u8],
) -> Result<reqwest::Response> {
    let u = origin(base)?;
    ensure(
        u.origin().ascii_serialization() == base
            && path.starts_with("/api/v2/")
            && !path.contains(['?', '#']),
        "invalid_request",
    )?;
    let nonce = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let signature = sign(
        &secret(secret_text)?,
        &canonical(key, method, path, &timestamp, &nonce, &id.to_string(), body),
    );
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| Fault("temporarily_unavailable"))?
        .request(
            if method == "POST" {
                reqwest::Method::POST
            } else {
                reqwest::Method::GET
            },
            format!("{base}{path}"),
        )
        .header("content-type", MEDIA_TYPE)
        .header("accept", MEDIA_TYPE)
        .header("x-source-version", "2")
        .header("x-source-key-id", key)
        .header("x-source-timestamp", timestamp)
        .header("x-source-nonce", nonce)
        .header("idempotency-key", id.to_string())
        .header("x-source-signature", signature)
        .body(body.to_vec())
        .send()
        .await
        .map_err(|_| Fault("temporarily_unavailable"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verification_accepts_no_browser_credential_configuration() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/source-connection-input.json"
        ))
        .unwrap();
        let attrs = fixture["data"]["attributes"].clone();
        let input: ConnectionInput = serde_json::from_value(attrs.clone()).unwrap();
        assert_eq!(serde_json::to_value(input).unwrap(), attrs);
        for field in ["source_id", "key_id", "secret", "origin", "id"] {
            let mut invalid = attrs.clone();
            invalid[field] = json!("injected");
            assert!(serde_json::from_value::<ConnectionInput>(invalid).is_err());
        }
    }
    #[test]
    fn school_route_input_does_not_accept_a_connection_choice() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/school-source-binding-input.json"
        ))
        .unwrap();
        let attrs = fixture["data"]["attributes"].clone();
        let input: BindingInput = serde_json::from_value(attrs.clone()).unwrap();
        assert_eq!(serde_json::to_value(input).unwrap(), attrs);
        let mut invalid = attrs;
        invalid["connection_id"] = json!(Uuid::nil());
        assert!(serde_json::from_value::<BindingInput>(invalid).is_err());
    }
    #[test]
    fn school_route_is_a_single_optional_binding() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/school-source-binding.json"
        ))
        .unwrap();
        let actual = SchoolRouteSettings {
            backlog: Default::default(),
            binding: Some(SchoolRoute {
                id: "20000000-0000-0000-0000-000000000031".parse().unwrap(),
                route_id: "school-route-1".into(),
                active: true,
                revision: 2,
                checked_at: None,
                updated_at: "2026-11-09T00:00:00Z".parse().unwrap(),
            }),
        };
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            fixture["data"]["attributes"]
        );
        assert_eq!(
            serde_json::to_value(SchoolRouteSettings {
                binding: None,
                backlog: Default::default()
            })
            .unwrap(),
            json!({"binding":null})
        );
    }
    #[test]
    fn settings_fixture_contains_no_secret() {
        let actual = ConnectionSettings {
            configured: true,
            origin: Some("https://keeper.example.com".into()),
            key_id: Some("key-example".into()),
            connection: None,
        };
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../api-contract/fixtures/source-connection-settings.json"
        ))
        .unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            fixture["data"]["attributes"]
        );
    }
    #[test]
    fn caller_identity_is_discovered_only_from_authenticated_metadata() {
        let id = Uuid::new_v4();
        assert_eq!(authenticated_caller(&json!({"source_id":id})).unwrap(), id);
        for metadata in [
            json!({}),
            json!({"source_id":"invalid"}),
            json!({"source_id":null}),
        ] {
            assert_eq!(
                authenticated_caller(&metadata).err().unwrap().0,
                "protocol_error"
            );
        }
    }
}
