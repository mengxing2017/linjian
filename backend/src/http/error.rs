use crate::business::Error;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use cairnkit_jsonapi::api::{JSON_API_MEDIA_TYPE, error_document};

/// Map application policy to Cairnkit's shared JSON:API document model.
pub fn response(status: StatusCode, code: &str, detail: &str) -> Response {
    (
        status,
        [(axum::http::header::CONTENT_TYPE, JSON_API_MEDIA_TYPE)],
        Json(error_document(
            status.as_u16(),
            code,
            status.canonical_reason().unwrap_or("HTTP Error").to_owned(),
            detail.to_owned(),
            None,
        )),
    )
        .into_response()
}

/// Normalize Axum's extractor, body-limit and method rejections through the same
/// document builder. Never reflect raw rejection bodies or credentials to users.
pub async fn normalize_errors(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let api = request.uri().path().starts_with("/api/");
    let result = next.run(request).await;
    let status = result.status();
    if !api
        || !(status.is_client_error() || status.is_server_error())
        || result
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .is_some_and(|v| v == JSON_API_MEDIA_TYPE)
    {
        return result;
    }
    let (code, detail) = match status {
        StatusCode::BAD_REQUEST => ("invalid_request", "请求参数或 JSON 格式不合法"),
        StatusCode::UNPROCESSABLE_ENTITY => ("validation", "请求字段类型错误或缺少必填字段"),
        StatusCode::METHOD_NOT_ALLOWED => ("method_not_allowed", "此接口不支持该请求方式"),
        StatusCode::PAYLOAD_TOO_LARGE => ("payload_too_large", "请求内容超过允许大小"),
        StatusCode::UNSUPPORTED_MEDIA_TYPE => {
            ("unsupported_media_type", "请使用接口要求的 Content-Type")
        }
        StatusCode::NOT_FOUND => ("not_found", "接口不存在"),
        _ => ("http_error", "服务暂时不可用"),
    };
    let (mut parts, _) = result.into_parts();
    let normalized = response(status, code, detail);
    parts.headers.remove(axum::http::header::CONTENT_LENGTH);
    parts.headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static(JSON_API_MEDIA_TYPE),
    );
    Response::from_parts(parts, normalized.into_body())
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "请重新登录".into(),
            ),
            Self::MiniappBanned => (
                StatusCode::FORBIDDEN,
                "miniapp_user_banned",
                "账号已封禁，请联系学校管理员".into(),
            ),
            Self::SchoolUnavailable => (
                StatusCode::NOT_FOUND,
                "school_unavailable",
                "学校暂不可用，请联系管理员".into(),
            ),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "没有操作权限".into()),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "记录不存在".into()),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "请求过于频繁，请稍后重试".into(),
            ),
            Self::Conflict => (
                StatusCode::CONFLICT,
                "conflict",
                "内容已变化，请刷新后重试".into(),
            ),
            Self::Policy(code) => (
                StatusCode::CONFLICT,
                code,
                match code {
                    "school_change_pending" => "已有待审换校申请，请先查询或撤回",
                    "school_change_review_timeout" => {
                        "申请已超过审核期限，由系统自动驳回，请刷新核对结果"
                    }
                    "school_change_relationship_required" => {
                        "旧申请未声明在读或新生关系，请撤回后重新申请"
                    }
                    "school_change_unbound" => "尚未绑定学校，请先完成首次绑定",
                    "school_change_conflict" => "换校申请已被处理或版本变化，请刷新核对结果",
                    "school_change_idempotency_conflict" => {
                        "本次提交标识已用于不同内容，请重新发起申请"
                    }
                    "school_change_document_unavailable" => {
                        "证明材料已过期、已清理或不属于本次申请"
                    }
                    "school_change_target_unavailable" => "目标学校暂不可用，请重新选择或联系平台",
                    "school_change_role_conflict" => "存在学校管理角色，请先从学校权限入口处理",
                    "school_change_source_changed" => "学校归属已变化，请刷新核对结果",
                    "school_change_account_unavailable" => "申请人账号不可用，不能批准换校",
                    "school_change_target_member_revoked" => "目标学校成员已撤销，请先核实准入限制",
                    "school_change_self_review" => "不能审核本人换校申请，请由其他平台审核员处理",
                    _ => "业务条件已变化，请刷新后重试",
                }
                .into(),
            ),
            Self::Invalid(message) => (StatusCode::UNPROCESSABLE_ENTITY, "validation", message),
            Self::Unavailable | Self::Upstream(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "服务暂不可用或尚未配置".into(),
            ),
            Self::Database(_) => {
                tracing::error!("forum database operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "服务处理失败".into(),
                )
            }
        };
        response(status, code, &message)
    }
}
