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
            _ => 400,
        };
        (axum::http::StatusCode::from_u16(status).unwrap(),[("content-type",MEDIA_TYPE)],Json(json!({"errors":[{"status":status.to_string(),"code":self.0,"title":match self.0 { "wekeeper_connection_unverified"=>"请先验证全局 WeKeeper 连接", "route_already_bound"=>"该消息路由已绑定其他学校", "wekeeper_not_configured"=>"请在部署配置中填写 WeKeeper 地址、key_id 和 secret", "wekeeper_configuration_incomplete"=>"WeKeeper 地址、key_id 和 secret 必须同时配置", "wekeeper_origin_invalid"=>"WeKeeper 地址必须为规范的 HTTPS 域名地址", "wekeeper_key_id_invalid"=>"WeKeeper key_id 格式不正确", "wekeeper_secret_invalid"=>"WeKeeper secret 格式不正确", _=>"来源连接操作失败" }}]}))).into_response()
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
    input: Input,
) -> Result<Response> {
    let v = input
        .decode::<BindingInput>("source-bindings")
        .map_err(|_| Fault("invalid_request"))?;
    Ok(resource(
        "source-bindings",
        Service(a.pool).bind(&actor, school, v).await?,
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
