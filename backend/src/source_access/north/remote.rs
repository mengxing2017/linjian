use crate::{
    bootstrap::Application,
    business::Actor,
    http::jsonapi::{Input, resource, singleton},
    source_access::{
        domain::*,
        north::local::{BindingInput, ConnectionInput, Service},
        south::adapter::postgres::Repository,
    },
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::json;
use uuid::Uuid;
impl IntoResponse for Fault {
    fn into_response(self) -> Response {
        let status = match self.0 {
            "forbidden" => 403,
            "not_found" => 404,
            "revision_conflict" | "route_already_bound" => 409,
            "temporarily_unavailable" => 503,
            "revision_required" => 428,
            _ => 400,
        };
        let title = match self.0 {
            "revision_conflict" => "配置已被其他操作更新，请刷新后重新保存",
            "revision_required" => "保存学校路由需要当前修订 If-Match",
            "wekeeper_connection_unverified" => "请先验证全局 WeKeeper 连接",
            "route_already_bound" => "该消息路由已绑定其他学校",
            "wekeeper_not_configured" => "请在部署配置中填写 WeKeeper 地址、key_id 和 secret",
            "wekeeper_configuration_incomplete" => "WeKeeper 地址、key_id 和 secret 必须同时配置",
            "wekeeper_origin_invalid" => "WeKeeper 地址必须为规范的 HTTPS 域名地址",
            "wekeeper_key_id_invalid" => "WeKeeper key_id 格式不正确",
            "wekeeper_secret_invalid" => "WeKeeper secret 格式不正确",
            "authentication_failed" => {
                "WeKeeper 拒绝了验证请求，请检查调用方凭据、授权和两端时钟，并查看 API 日志中的上游状态码"
            }
            "temporarily_unavailable" => {
                "来源连接服务暂时不可用，请查看 API 日志并检查网络、TLS 和数据库后重试"
            }
            "protocol_error" => {
                "WeKeeper 响应格式不符合来源 v2 协议，请检查服务地址、反向代理和上游版本"
            }
            "unsupported_version" => "WeKeeper 不支持所需的来源 v2 协议，请检查上游版本",
            "route_unavailable" => "WeKeeper 消息路由不可用，请检查路由 ID、启用状态和调用方授权",
            "credential_unavailable" => "当前部署凭据与原连接不匹配，请检查 WeKeeper 地址和 key_id",
            _ => "来源连接操作失败",
        };
        (
            axum::http::StatusCode::from_u16(status).unwrap(),
            [("content-type", MEDIA_TYPE)],
            Json(json!({"errors":[{"status":status.to_string(),"code":self.0,"title":title}]})),
        )
            .into_response()
    }
}
async fn list(State(a): State<Application>, actor: Actor) -> Result<Response> {
    Ok(singleton(
        "source-connections",
        "list",
        serde_json::to_value(Service(a.pool).list(&actor).await?)
            .map_err(|_| Fault("temporarily_unavailable"))?,
    )
    .into_response())
}
#[tracing::instrument(skip_all, fields(operation = "verify_wekeeper_connection"), err(Debug))]
async fn save(State(a): State<Application>, actor: Actor, input: Input) -> Result<Response> {
    let v = input
        .decode::<ConnectionInput>("source-connections")
        .map_err(|_| Fault("invalid_request"))?;
    Ok(resource("source-connections", Service(a.pool).save(&actor, v).await?).into_response())
}
async fn bindings(
    State(a): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
) -> Result<Response> {
    Ok(singleton(
        "source-bindings",
        school,
        serde_json::to_value(Repository(a.pool).bindings(&actor, school).await?)
            .map_err(|_| Fault("temporarily_unavailable"))?,
    )
    .into_response())
}
async fn bind(
    State(a): State<Application>,
    actor: Actor,
    Path(school): Path<Uuid>,
    headers: HeaderMap,
    input: Input,
) -> Result<Response> {
    let v = input
        .decode::<BindingInput>("source-bindings")
        .map_err(|_| Fault("invalid_request"))?;
    Ok(resource(
        "source-bindings",
        Service(a.pool)
            .bind(&actor, school, v, expected_revision(&headers)?)
            .await?,
    )
    .into_response())
}
pub fn router() -> Router<Application> {
    Router::new()
        .route("/api/v2/source-connections", get(list).post(save))
        .route(
            "/api/v2/schools/{school}/source-bindings",
            get(bindings).post(bind),
        )
}

fn expected_revision(headers: &HeaderMap) -> Result<i64> {
    let raw = headers
        .get("if-match")
        .and_then(|h| h.to_str().ok())
        .ok_or(Fault("revision_required"))?;
    let value = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or(Fault("invalid_request"))?;
    let revision = value.parse::<i64>().map_err(|_| Fault("invalid_request"))?;
    ensure(
        revision >= 0 && revision.to_string() == value,
        "invalid_request",
    )?;
    Ok(revision)
}

#[cfg(test)]
mod revision_tests {
    use super::*;
    #[tokio::test]
    async fn verification_errors_preserve_codes_and_explain_recovery() {
        for (code, status, hint) in [
            ("authentication_failed", 400, "调用方凭据"),
            ("temporarily_unavailable", 503, "API 日志"),
            ("protocol_error", 400, "来源 v2 协议"),
            ("unsupported_version", 400, "上游版本"),
        ] {
            let response = Fault(code).into_response();
            assert_eq!(response.status().as_u16(), status);
            assert_eq!(response.headers()["content-type"], MEDIA_TYPE);
            let body = axum::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap();
            let document: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(document["errors"][0]["code"], code);
            assert!(
                document["errors"][0]["title"]
                    .as_str()
                    .unwrap()
                    .contains(hint)
            );
        }
    }

    #[test]
    fn binding_writes_require_a_quoted_nonnegative_revision() {
        let mut headers = HeaderMap::new();
        assert_eq!(
            expected_revision(&headers).unwrap_err().0,
            "revision_required"
        );
        for value in ["*", "W/\"1\"", "1", "\"-1\"", "\"01\""] {
            headers.insert("if-match", value.parse().unwrap());
            assert!(expected_revision(&headers).is_err());
        }
        headers.insert("if-match", "\"0\"".parse().unwrap());
        assert_eq!(expected_revision(&headers).unwrap(), 0);
        headers.insert("if-match", "\"2\"".parse().unwrap());
        assert_eq!(expected_revision(&headers).unwrap(), 2);
    }
}
