use crate::{
    bootstrap::Application,
    business::{Actor, Result, Revision},
    http::jsonapi::{ApiResponse, Input, completed, resource},
    schools::{
        domain::home_menu::{CatalogQuery, ImportCommit, OrderInput, SettingsInput, TagInput},
        north::home_menu::Service,
    },
};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    routing::{get, post, put},
};
use uuid::Uuid;
async fn list(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        "home-menu-catalog",
        Service::from_pool(a.pool).list(&actor, s, false).await?,
    ))
}
async fn filtered(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    Query(q): Query<CatalogQuery>,
) -> Result<ApiResponse> {
    Ok(resource(
        "home-menu-catalog",
        Service::from_pool(a.pool)
            .filtered(&actor, s, &q, false)
            .await?,
    ))
}
fn csv_cell(value: &str) -> String {
    let value = if value
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@') || c.is_control())
    {
        format!("'{value}")
    } else {
        value.to_owned()
    };
    format!("\"{}\"", value.replace('"', "\"\""))
}
async fn export(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    Query(q): Query<CatalogQuery>,
) -> Result<axum::response::Response> {
    use axum::response::IntoResponse;
    let result = Service::from_pool(a.pool)
        .filtered(&actor, s, &q, true)
        .await?;
    let mut csv = String::from("\u{feff}名称,菜单状态,来源类型,是否启用,排序,创建时间\r\n");
    for row in result["tags"].as_array().into_iter().flatten() {
        let menu = match row["menu_status"].as_str() {
            Some("shown") => "显示",
            Some("hot") => "显示并作为热门标签",
            _ => "隐藏",
        };
        let source = if row["source_type"] == "admin" {
            "后台配置"
        } else {
            "用户创建"
        };
        let enabled = match row["enable_status"].as_str() {
            Some("enabled") => "启用",
            Some("pending") => "待审核",
            _ => "停用",
        };
        let fields = [
            row["name"].as_str().unwrap_or(""),
            menu,
            source,
            enabled,
            row["created_at"].as_str().unwrap_or(""),
        ];
        csv.push_str(&format!(
            "{},{},{},{},{},{}\r\n",
            csv_cell(fields[0]),
            csv_cell(fields[1]),
            csv_cell(fields[2]),
            csv_cell(fields[3]),
            row["sort_order"],
            csv_cell(fields[4])
        ));
    }
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=home-menu-tags.csv",
            ),
            (axum::http::header::CACHE_CONTROL, "private, no-store"),
        ],
        csv,
    )
        .into_response())
}
async fn public(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        "home-menu",
        Service::from_pool(a.pool).list(&actor, s, true).await?,
    ))
}
async fn create(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "home-menu-tags",
        Service::from_pool(a.pool)
            .save(&actor, s, None, input.decode::<TagInput>("home-menu-tags")?)
            .await?,
    ))
}
async fn update(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "home-menu-tags",
        Service::from_pool(a.pool)
            .save(
                &actor,
                s,
                Some(id),
                input.decode::<TagInput>("home-menu-tags")?,
            )
            .await?,
    ))
}
async fn delete(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    let v = input.decode::<Revision>("home-menu-tags")?;
    Service::from_pool(a.pool)
        .delete(&actor, s, id, v.revision)
        .await?;
    Ok(completed())
}
async fn order(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Service::from_pool(a.pool)
        .order(&actor, s, input.decode::<OrderInput>("home-menu-order")?)
        .await?;
    Ok(completed())
}
async fn get_settings(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
) -> Result<ApiResponse> {
    Ok(resource(
        "home-menu-settings",
        Service::from_pool(a.pool).settings(&actor, s).await?,
    ))
}
async fn settings(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    input: Input,
) -> Result<ApiResponse> {
    Ok(resource(
        "home-menu-settings",
        Service::from_pool(a.pool)
            .save_settings(
                &actor,
                s,
                input.decode::<SettingsInput>("home-menu-settings")?,
            )
            .await?,
    ))
}
async fn import_preview(
    State(a): State<Application>,
    actor: Actor,
    Path(s): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<ApiResponse> {
    let field = multipart
        .next_field()
        .await
        .map_err(|_| crate::business::Error::Invalid("文件格式错误".into()))?
        .ok_or_else(|| crate::business::Error::Invalid("请选择文件".into()))?;
    crate::business::require(field.name() == Some("file"), "需要 file 文件字段")?;
    let name = field
        .file_name()
        .ok_or_else(|| crate::business::Error::Invalid("文件名缺失".into()))?
        .to_owned();
    let bytes = field
        .bytes()
        .await
        .map_err(|_| crate::business::Error::Invalid("文件读取失败或超过限制".into()))?;
    crate::business::require(
        multipart
            .next_field()
            .await
            .map_err(|_| crate::business::Error::Invalid("文件格式错误".into()))?
            .is_none(),
        "仅允许一个文件",
    )?;
    Ok(resource(
        "home-menu-imports",
        Service::from_pool(a.pool)
            .import_preview(&actor, s, &name, &bytes)
            .await?,
    ))
}
async fn import_commit(
    State(a): State<Application>,
    actor: Actor,
    Path((s, id)): Path<(Uuid, Uuid)>,
    input: Input,
) -> Result<ApiResponse> {
    let v = input.decode::<ImportCommit>("home-menu-imports")?;
    Ok(resource(
        "home-menu-imports",
        Service::from_pool(a.pool)
            .import_commit(&actor, s, id, &v.digest)
            .await?,
    ))
}
pub fn router() -> Router<Application> {
    Router::new()
        .route(
            "/api/v1/schools/{school}/home-menu-imports",
            post(import_preview).layer(DefaultBodyLimit::max(5 * 1024 * 1024 + 65536)),
        )
        .route(
            "/api/v1/schools/{school}/home-menu-imports/{id}/commit",
            post(import_commit),
        )
        .route(
            "/api/v1/schools/{school}/home-menu-settings",
            get(get_settings).put(settings),
        )
        .route("/api/v1/schools/{school}/home-menu", get(public))
        .route(
            "/api/v1/schools/{school}/home-menu-tags/search",
            get(filtered),
        )
        .route(
            "/api/v1/schools/{school}/home-menu-tags/export",
            get(export),
        )
        .route(
            "/api/v1/schools/{school}/home-menu-tags",
            get(list).post(create),
        )
        .route("/api/v1/schools/{school}/home-menu-tags/order", post(order))
        .route(
            "/api/v1/schools/{school}/home-menu-tags/{id}",
            put(update).delete(delete),
        )
}

#[cfg(test)]
mod tests {
    use super::csv_cell;
    #[test]
    fn exported_cells_escape_formulas_and_quotes() {
        assert_eq!(csv_cell("=1+1"), "\"'=1+1\"");
        assert_eq!(csv_cell("名字,\"甲\""), "\"名字,\"\"甲\"\"\"");
        assert_eq!(csv_cell("\t公式"), "\"'\t公式\"");
    }
}
