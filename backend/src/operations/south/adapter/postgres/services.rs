use super::*;
use crate::business::Page;
use crate::operations::domain::services::*;
use chrono::{DateTime, Utc};
fn same_time(value: &Value, time: DateTime<Utc>) -> bool {
    value
        .as_str()
        .and_then(|value| value.parse::<DateTime<Utc>>().ok())
        .is_some_and(|prior| prior.timestamp_micros() == time.timestamp_micros())
}

async fn revision(
    tx: &mut Transaction<'_, Postgres>,
    scope: Option<Uuid>,
    lock: bool,
) -> Result<i64> {
    let id = scope.unwrap_or(Uuid::nil());
    sqlx::query("INSERT INTO operation_config_revisions(id) VALUES($1) ON CONFLICT DO NOTHING")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    let suffix = if lock { " FOR UPDATE" } else { "" };
    Ok(sqlx::query_scalar(&format!(
        "SELECT revision FROM operation_config_revisions WHERE id=$1{suffix}"
    ))
    .bind(id)
    .fetch_one(&mut **tx)
    .await?)
}
async fn token(tx: &mut Transaction<'_, Postgres>, scope: Option<Uuid>) -> Result<String> {
    let global = revision(tx, None, false).await?;
    let local = if scope.is_some() {
        revision(tx, scope, false).await?
    } else {
        0
    };
    Ok(format!("{global}:{local}"))
}
async fn bump(tx: &mut Transaction<'_, Postgres>, scope: Option<Uuid>) -> Result<()> {
    sqlx::query("UPDATE operation_config_revisions SET revision=revision+1 WHERE id=$1")
        .bind(scope.unwrap_or(Uuid::nil()))
        .execute(&mut **tx)
        .await?;
    Ok(())
}
fn permission(entity: &str, operation: &str) -> String {
    format!(
        "operation.{}.{operation}",
        if entity == "operation-service-categories" {
            "category"
        } else {
            "shortcut"
        }
    )
}
async fn manage_auth(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    scope: Option<Uuid>,
    entity: &str,
    operation: &str,
) -> Result<()> {
    if matches!(
        entity,
        "operation-service-targets"
            | "operation-partners"
            | "operation-cooperations"
            | "marketing-slots"
    ) {
        require(scope.is_none(), "仅平台可管理此配置")?;
        actor.platform()?;
    }
    auth(tx, actor, scope, &permission(entity, operation)).await
}
fn table(entity: &str) -> Result<&'static str> {
    match entity {
        "home-shortcuts" => Ok("school_resources"),
        "operation-service-categories" => Ok("operation_service_categories"),
        "operation-service-targets" => Ok("operation_service_targets"),
        "operation-partners" => Ok("operation_partners"),
        "operation-cooperations" => Ok("operation_cooperations"),
        "marketing-slots" => Ok("marketing_slots"),
        _ => Err(Error::NotFound),
    }
}
fn shortcut_input(mut row: Value) -> Result<ShortcutInput> {
    let fields = [
        "revision",
        "title",
        "subtitle",
        "badge",
        "image_ref",
        "category_id",
        "cooperation_id",
        "action",
        "active",
        "featured",
        "sort_order",
        "starts_at",
        "ends_at",
    ];
    row.as_object_mut()
        .ok_or(Error::Unavailable)?
        .retain(|key, _| fields.contains(&key.as_str()));
    decode(&row)
}
async fn channel_config(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<TargetInput> {
    let config: Value = sqlx::query_scalar(
        "SELECT config FROM operation_target_versions WHERE id=$1 AND state='approved' FOR SHARE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(Error::NotFound)?;
    decode(&config)
}
async fn validate_schools(tx: &mut Transaction<'_, Postgres>, schools: &[Uuid]) -> Result<()> {
    let unique: std::collections::HashSet<_> = schools.iter().collect();
    require(unique.len() == schools.len(), "学校范围不能重复")?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM schools WHERE id=ANY($1)")
        .bind(schools)
        .fetch_one(&mut **tx)
        .await?;
    require(count == schools.len() as i64, "学校范围不存在")
}
async fn validate_target(tx: &mut Transaction<'_, Postgres>, config: &TargetInput) -> Result<()> {
    validate_schools(tx, &config.school_ids).await?;
    if let Some(partner) = config.partner_id {
        sqlx::query("SELECT id FROM operation_partners WHERE id=$1 AND active FOR SHARE")
            .bind(partner)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(Error::NotFound)?;
    }
    if let Some(cooperation) = config.cooperation_id {
        let partner: Uuid = sqlx::query_scalar(
            "SELECT partner_id FROM operation_cooperations WHERE id=$1 AND active FOR SHARE",
        )
        .bind(cooperation)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::NotFound)?;
        require(
            config.partner_id == Some(partner),
            "目标合作方与合作项目不一致",
        )?;
    }
    Ok(())
}
async fn withdraw_linked(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    key: &str,
    id: Uuid,
) -> Result<()> {
    sqlx::query("WITH withdrawn AS (UPDATE operation_target_versions SET state='withdrawn',revision=revision+1 WHERE config->>$1=$2 AND state IN ('approved','pending','draft') RETURNING id) INSERT INTO operation_target_reviews(id,version_id,actor_id,decision,reason) SELECT gen_random_uuid(),id,$3,'withdraw','合作资料修改，须生成新版本重新审核' FROM withdrawn").bind(key).bind(id.to_string()).bind(actor.id).execute(&mut **tx).await?;
    Ok(())
}
async fn validate_shortcut(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    scope: Option<Uuid>,
    input: &ShortcutInput,
) -> Result<()> {
    let ImageRef::Asset { id } = input.image_ref;
    check_image(tx, scope, id).await?;
    if !actor.platform_admin && scope.is_some() {
        let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM file_assets WHERE id=$1 AND artwork_permission LIKE 'operation.shortcut.%')").bind(id).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(Error::Forbidden);
        }
    }
    if let Some(category) = input.category_id {
        sqlx::query("SELECT id FROM operation_service_categories WHERE id=$1 AND (school_id IS NULL OR school_id=$2) AND active FOR SHARE").bind(category).bind(scope).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
    }
    if let Some(cooperation) = input.cooperation_id {
        sqlx::query("SELECT c.id FROM operation_cooperations c JOIN operation_partners p ON p.id=c.partner_id WHERE c.id=$1 AND c.active AND p.active AND (c.ends_at IS NULL OR c.ends_at>now()) AND ($2::uuid IS NULL OR cardinality(c.school_ids)=0 OR $2=ANY(c.school_ids)) FOR SHARE OF c,p").bind(cooperation).bind(scope).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
    }
    match &input.action {
        ShortcutAction::InternalPage { page, object_id } => {
            check_action(
                tx,
                scope,
                &ResourceAction::InternalPage {
                    page: page.clone(),
                    object_id: *object_id,
                },
            )
            .await?
        }
        ShortcutAction::TargetChannel {
            version_id,
            channel,
        } => {
            let config = channel_config(tx, *version_id).await?;
            require(
                scope.is_none_or(|school| {
                    config.school_ids.is_empty() || config.school_ids.contains(&school)
                }),
                "目标不适用于当前学校",
            )?;
            require(
                config.cooperation_id == input.cooperation_id,
                "入口合作项目与审核版本不一致",
            )?;
            require(
                config.channels.iter().any(|item| &item.key == channel),
                "目标渠道不存在",
            )?;
        }
        ShortcutAction::None => {}
    }
    Ok(())
}
async fn write_shortcut(
    tx: &mut Transaction<'_, Postgres>,
    scope: Option<Uuid>,
    object: Uuid,
    value: &ShortcutInput,
) -> Result<()> {
    let version = if let ShortcutAction::TargetChannel { version_id, .. } = &value.action {
        Some(*version_id)
    } else {
        None
    };
    sqlx::query("INSERT INTO school_resources(id,school_id,kind,title,subtitle,badge,image_ref,category_id,cooperation_id,action,target_version_id,active,featured,sort_order,starts_at,ends_at) VALUES($1,$2,'shortcut',$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) ON CONFLICT(id) DO UPDATE SET title=$3,subtitle=$4,badge=$5,image_ref=$6,category_id=$7,cooperation_id=$8,action=$9,target_version_id=$10,active=$11,featured=$12,sort_order=$13,starts_at=$14,ends_at=$15,revision=school_resources.revision+1").bind(object).bind(scope).bind(&value.title).bind(&value.subtitle).bind(&value.badge).bind(json!(value.image_ref)).bind(value.category_id).bind(value.cooperation_id).bind(json!(value.action)).bind(version).bind(value.active).bind(value.featured).bind(value.sort_order).bind(value.starts_at).bind(value.ends_at).execute(&mut **tx).await?;

    Ok(())
}
async fn select(
    tx: &mut Transaction<'_, Postgres>,
    scope: Uuid,
    at: DateTime<Utc>,
    preview: bool,
) -> Result<Vec<Value>> {
    let rows: Vec<Value> = sqlx::query_scalar("SELECT to_jsonb(r)||jsonb_build_object('override',coalesce(o.presentation,'{}'::jsonb),'override_revision',coalesce(o.revision,0),'category_name',c.name,'category_active',coalesce(c.active,true),'version_state',v.state,'target_config',v.config,'cooperation',CASE WHEN p.id IS NULL THEN NULL ELSE jsonb_build_object('name',p.name,'model',p.model,'disclosure',p.disclosure,'active',p.active AND partner.active,'starts_at',p.starts_at,'ends_at',p.ends_at,'school_ids',p.school_ids) END) FROM school_resources r LEFT JOIN school_shortcut_overrides o ON o.resource_id=r.id AND o.school_id=$1 LEFT JOIN operation_service_categories c ON c.id=r.category_id LEFT JOIN operation_target_versions v ON v.id=r.target_version_id LEFT JOIN operation_cooperations p ON p.id=r.cooperation_id LEFT JOIN operation_partners partner ON partner.id=p.partner_id WHERE r.kind='shortcut' AND r.deleted_at IS NULL AND (r.school_id IS NULL OR r.school_id=$1)").bind(scope).fetch_all(&mut **tx).await?;
    let mut result = Vec::new();
    for mut row in rows {
        let source_active = row["active"] == true;
        let overlay: Presentation = decode(&row["override"])?;
        let inherited_icon = row["image_ref"].clone();
        for (key, value) in [
            ("category_id", overlay.category_id.map(|value| json!(value))),
            ("title", overlay.title.map(Value::String)),
            ("subtitle", overlay.subtitle.map(Value::String)),
            ("badge", overlay.badge.map(Value::String)),
            ("image_ref", overlay.image_ref.map(|value| json!(value))),
            ("active", overlay.active.map(Value::Bool)),
            ("featured", overlay.featured.map(Value::Bool)),
            ("sort_order", overlay.sort_order.map(|value| json!(value))),
        ] {
            if let Some(value) = value {
                row[key] = value;
            }
        }
        if overlay.category_id.is_some() {
            let category: Option<Uuid> = serde_json::from_value(row["category_id"].clone())
                .map_err(|_| Error::Unavailable)?;
            let info:Option<(String,bool)>=sqlx::query_as("SELECT name,active FROM operation_service_categories WHERE id=$1 AND (school_id IS NULL OR school_id=$2)").bind(category).bind(scope).fetch_optional(&mut **tx).await?;
            row["category_name"] = json!(info.as_ref().map(|value| &value.0));
            row["category_active"] = json!(category.is_none() || info.is_some_and(|value| value.1));
        }
        let mut reason = None;
        if row["active"] != true {
            reason = Some("入口已停用");
        }
        if row["school_id"].is_null() && !source_active {
            reason = Some("平台入口已停用");
        }
        if row["category_active"] != true {
            reason = Some("分类已停用");
        }
        for (key, start) in [("starts_at", true), ("ends_at", false)] {
            if let Some(value) = row[key]
                .as_str()
                .and_then(|value| value.parse::<DateTime<Utc>>().ok())
                && ((start && at < value) || (!start && at >= value))
            {
                reason = Some("入口不在展示期限");
            }
        }
        if row["cooperation"].is_object() {
            if row["cooperation"]["school_ids"]
                .as_array()
                .is_some_and(|items| !items.is_empty() && !items.contains(&json!(scope)))
            {
                reason = Some("合作不适用于当前学校");
            }
            if row["cooperation"]["active"] != true {
                reason = Some("合作已暂停");
            }
            for (key, start) in [("starts_at", true), ("ends_at", false)] {
                if let Some(value) = row["cooperation"][key]
                    .as_str()
                    .and_then(|value| value.parse::<DateTime<Utc>>().ok())
                    && ((start && at < value) || (!start && at >= value))
                {
                    reason = Some("合作不在有效期限");
                }
            }
        }
        let mut channels = Vec::new();
        let mut disclosure = Value::Null;
        if row["action"]["kind"] == "target_channel" {
            if row["version_state"] != "approved" {
                reason = Some("服务目标未批准或已撤回");
            }
            if let Ok(config) = decode::<TargetInput>(&row["target_config"]) {
                if !config.school_ids.is_empty() && !config.school_ids.contains(&scope) {
                    reason = Some("目标不适用于当前学校");
                }
                if config.cooperation_id
                    != serde_json::from_value::<Option<Uuid>>(row["cooperation_id"].clone())
                        .map_err(|_| Error::Unavailable)?
                {
                    reason = Some("合作与目标版本不一致");
                }
                disclosure = json!({"provider":config.provider,"purpose":config.purpose,"disclosure":config.disclosure,"partner_id":config.partner_id,"contact":config.contact,"complaint":config.complaint});
                for mut channel in config.channels {
                    if let ChannelAction::QrCode { asset_id } = channel.action {
                        let key = channel.key.clone();
                        channels.push(json!({"key":key,"label":channel.label,"action":{"kind":"qr_code","image_url":format!("/api/v1/schools/{scope}/operation-services/{}/media/{asset_id}?channel={key}&version={}",row["id"].as_str().unwrap_or(""),row["action"]["version_id"].as_str().unwrap_or(""))}}));
                    } else {
                        if let Some(code) = sqlx_channel_code(tx, &row).await? {
                            match &mut channel.action {
                                ChannelAction::MiniProgram { path, .. } => {
                                    let separator = if path.contains('?') { "&" } else { "?" };
                                    *path = format!(
                                        "{path}{separator}linjian_channel={code}&linjian_school={scope}"
                                    );
                                }
                                ChannelAction::WebView { url }
                                | ChannelAction::CopyLink { url } => {
                                    let mut parsed =
                                        url::Url::parse(url).map_err(|_| Error::Unavailable)?;
                                    parsed
                                        .query_pairs_mut()
                                        .append_pair("linjian_channel", &code)
                                        .append_pair("linjian_school", &scope.to_string());
                                    *url = parsed.into();
                                }
                                _ => {}
                            }
                        }
                        channels.push(json!(channel));
                    }
                }
                if !channels
                    .iter()
                    .any(|item| item["key"] == row["action"]["channel"])
                {
                    reason = Some("目标渠道不可用");
                }
            } else {
                reason = Some("目标配置不可用");
            }
        } else if row["action"]["kind"] == "internal_page" && row["action"]["page"] == "post" {
            let post_id = row["action"]["object_id"]
                .as_str()
                .and_then(|value| value.parse::<Uuid>().ok());
            let visible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM posts WHERE id=$1 AND school_id=$2 AND visibility='visible' AND audit_status='approved')").bind(post_id).bind(scope).fetch_one(&mut **tx).await?;
            if !visible {
                reason = Some("帖子不可见");
            }
        }
        let image: Uuid = row["image_ref"]["id"]
            .as_str()
            .and_then(|value| value.parse().ok())
            .ok_or(Error::Unavailable)?;
        let image_scope = if row["image_ref"] != inherited_icon {
            Some(scope)
        } else {
            row["school_id"]
                .as_str()
                .and_then(|value| value.parse::<Uuid>().ok())
        };
        let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM operational_media WHERE id=$1 AND school_id IS NOT DISTINCT FROM $2)").bind(image).bind(image_scope).fetch_one(&mut **tx).await?;
        if !valid {
            reason = Some("素材不可用");
        }
        if reason.is_some() && !preview {
            continue;
        }
        if matches!(
            row["cooperation"]["model"].as_str(),
            Some("sponsor" | "cps")
        ) {
            row["badge"] = json!("合作推广");
        }
        channels.sort_by_key(|channel| channel["key"] != row["action"]["channel"]);
        let mut value = json!({"id":row["id"],"school_id":row["school_id"],"title":row["title"],"subtitle":row["subtitle"],"badge":row["badge"],"image_url":format!("/api/v1/schools/{scope}/operation-services/{}/media/{image}",row["id"].as_str().unwrap_or("")),"category_id":row["category_id"],"category_name":row["category_name"],"cooperation_id":row["cooperation_id"],"featured":row["featured"],"sort_order":row["sort_order"],"starts_at":row["starts_at"],"ends_at":row["ends_at"],"action":row["action"],"channels":channels,"disclosure":disclosure,"cooperation":if row["cooperation"].is_object(){json!({"name":row["cooperation"]["name"],"model":row["cooperation"]["model"],"disclosure":row["cooperation"]["disclosure"]})}else{Value::Null},"revision":row["revision"],"override_revision":row["override_revision"]});
        if preview {
            value["channels"] = json!([]);
            value["hidden_reason"] = json!(reason);
        }
        result.push(value);
    }
    result.sort_by(|left, right| {
        right["sort_order"]
            .as_i64()
            .cmp(&left["sort_order"].as_i64())
            .then_with(|| {
                left["school_id"]
                    .is_null()
                    .cmp(&right["school_id"].is_null())
            })
            .then_with(|| left["id"].as_str().cmp(&right["id"].as_str()))
    });
    Ok(result)
}
async fn sqlx_channel_code(
    tx: &mut Transaction<'_, Postgres>,
    row: &Value,
) -> Result<Option<String>> {
    let Some(id) = row["cooperation_id"]
        .as_str()
        .and_then(|value| value.parse::<Uuid>().ok())
    else {
        return Ok(None);
    };
    Ok(
        sqlx::query_scalar(
            "SELECT nullif(channel_code,'') FROM operation_cooperations WHERE id=$1",
        )
        .bind(id)
        .fetch_one(&mut **tx)
        .await?,
    )
}
async fn selected(tx: &mut Transaction<'_, Postgres>, scope: Uuid, id: Uuid) -> Result<Value> {
    select(tx, scope, Utc::now(), false)
        .await?
        .into_iter()
        .find(|value| value["id"] == id.to_string())
        .ok_or(Error::NotFound)
}
async fn get_manage(
    tx: &mut Transaction<'_, Postgres>,
    scope: Option<Uuid>,
    entity: &str,
    id: Uuid,
) -> Result<Value> {
    let name = table(entity)?;
    if entity == "operation-service-targets" {
        return sqlx::query_scalar("SELECT to_jsonb(t)||jsonb_build_object('versions',(SELECT coalesce(jsonb_agg(to_jsonb(v)||jsonb_build_object('reviews',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY r.created_at,r.id),'[]') FROM operation_target_reviews r WHERE r.version_id=v.id)) ORDER BY v.created_at DESC,v.id DESC),'[]') FROM operation_target_versions v WHERE v.target_id=t.id)) FROM operation_service_targets t WHERE t.id=$1").bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound);
    }
    let predicate = if entity == "home-shortcuts" {
        " AND school_id IS NOT DISTINCT FROM $2 AND kind='shortcut' AND deleted_at IS NULL"
    } else if entity == "operation-service-categories" {
        " AND school_id IS NOT DISTINCT FROM $2"
    } else {
        " AND $2::uuid IS NULL"
    };
    sqlx::query_scalar(&format!(
        "SELECT to_jsonb(t) FROM {name} t WHERE id=$1{predicate}"
    ))
    .bind(id)
    .bind(scope)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(Error::NotFound)
}
pub(super) async fn home_catalog(tx: &mut Transaction<'_, Postgres>, scope: Uuid) -> Result<Value> {
    revision(tx, None, false).await?;
    revision(tx, Some(scope), false).await?;
    sqlx::query("SELECT id FROM operation_config_revisions WHERE id=ANY($1) ORDER BY id FOR SHARE")
        .bind(vec![Uuid::nil(), scope])
        .fetch_all(&mut **tx)
        .await?;
    let shortcuts: Vec<Value> = select(tx, scope, Utc::now(), false)
        .await?
        .into_iter()
        .filter(|value| value["featured"] == true)
        .take(8)
        .collect();
    let categories:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'name',name) FROM operation_service_categories WHERE active AND (school_id IS NULL OR school_id=$1) ORDER BY sort_order DESC,id").bind(scope).fetch_all(&mut **tx).await?;
    Ok(json!({"shortcuts":shortcuts,"categories":categories,"token":token(tx,Some(scope)).await?}))
}
pub(super) async fn execute(
    repository: &Repository,
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    scope: Option<Uuid>,
    command: ServiceCommand,
) -> Result<Value> {
    let preview_command = matches!(&command, ServiceCommand::Preview(_));
    if matches!(
        &command,
        ServiceCommand::Catalog(_)
            | ServiceCommand::Preview(_)
            | ServiceCommand::Detail(_)
            | ServiceCommand::Resolve { .. }
            | ServiceCommand::Media { .. }
            | ServiceCommand::ManageList { .. }
            | ServiceCommand::ManageDetail { .. }
            | ServiceCommand::Options
            | ServiceCommand::Metric(_)
    ) {
        revision(tx, None, false).await?;
        sqlx::query("SELECT id FROM operation_config_revisions WHERE id=$1 FOR SHARE")
            .bind(Uuid::nil())
            .fetch_one(&mut **tx)
            .await?;
        if let Some(school) = scope {
            revision(tx, Some(school), false).await?;
            sqlx::query("SELECT id FROM operation_config_revisions WHERE id=$1 FOR SHARE")
                .bind(school)
                .fetch_one(&mut **tx)
                .await?;
        }
    }
    match command {
        ServiceCommand::Options => {
            auth(tx, actor, scope, "operation.shortcut.read").await?;
            let targets:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'name',config->>'name','provider',config->>'provider','channels',(SELECT jsonb_agg(jsonb_build_object('key',c->>'key','label',c->>'label')) FROM jsonb_array_elements(config->'channels') c)) FROM operation_target_versions WHERE state='approved' AND ($1::uuid IS NULL OR coalesce(config->'school_ids','[]')='[]'::jsonb OR config->'school_ids' @> to_jsonb(ARRAY[$1])) ORDER BY id").bind(scope).fetch_all(&mut **tx).await?;
            let categories:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'name',name) FROM operation_service_categories WHERE active AND (school_id IS NULL OR school_id=$1) ORDER BY sort_order DESC,id").bind(scope).fetch_all(&mut **tx).await?;
            let cooperations:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',c.id,'name',c.name,'disclosure',c.disclosure) FROM operation_cooperations c JOIN operation_partners p ON p.id=c.partner_id WHERE c.active AND p.active AND (c.ends_at IS NULL OR c.ends_at>now()) AND ($1::uuid IS NULL OR cardinality(c.school_ids)=0 OR $1=ANY(c.school_ids)) ORDER BY c.id").bind(scope).fetch_all(&mut **tx).await?;
            Ok(
                json!({"id":"current","targets":targets,"categories":categories,"cooperations":cooperations}),
            )
        }
        ServiceCommand::ManageDetail { entity, id } => {
            manage_auth(tx, actor, scope, &entity, "read").await?;
            get_manage(tx, scope, &entity, id).await
        }
        ServiceCommand::ManageList { entity, query } => {
            manage_auth(tx, actor, scope, &entity, "read").await?;
            let name = table(&entity)?;
            let predicate = if entity == "home-shortcuts" {
                "school_id IS NOT DISTINCT FROM $1 AND kind='shortcut' AND deleted_at IS NULL"
            } else if entity == "operation-service-categories" {
                "school_id IS NOT DISTINCT FROM $1"
            } else {
                "$1::uuid IS NULL"
            };
            let rows: Vec<Uuid> = sqlx::query_scalar(&format!(
                "SELECT id FROM {name} WHERE {predicate} ORDER BY id"
            ))
            .bind(scope)
            .fetch_all(&mut **tx)
            .await?;
            let mut values = Vec::new();
            for id in rows {
                values.push(get_manage(tx, scope, &entity, id).await?);
            }
            let search = query.search.as_deref().unwrap_or_default().to_lowercase();
            values.retain(|value| value.to_string().to_lowercase().contains(&search));
            let (size, offset) = query.page().fetch_bounds()?;
            let items: Vec<_> = values
                .into_iter()
                .skip(offset as usize)
                .take(size as usize)
                .collect();
            Ok(json!({"id":"current","token":token(tx,scope).await?,"items":items}))
        }
        ServiceCommand::Save { entity, id, input } => {
            manage_auth(
                tx,
                actor,
                scope,
                &entity,
                if id.is_some() { "update" } else { "create" },
            )
            .await?;
            revision(tx, None, true).await?;
            if scope.is_some() {
                revision(tx, scope, true).await?;
            }
            let expected = input["revision"]
                .as_i64()
                .ok_or_else(|| Error::Invalid("版本无效".into()))?;
            if let Some(id) = id {
                if get_manage(tx, scope, &entity, id).await?["revision"] != expected {
                    return Err(Error::Conflict);
                }
            } else {
                require(expected == 0, "新增不能指定旧版本")?;
            }
            let object = id.unwrap_or_else(Uuid::now_v7);
            match entity.as_str() {
                "home-shortcuts" => {
                    let value: ShortcutInput = decode(&input)?;
                    validate_shortcut(tx, actor, scope, &value).await?;
                    write_shortcut(tx, scope, object, &value).await?;
                    let snapshot = get_manage(tx, scope, &entity, object).await?;
                    sqlx::query("INSERT INTO resource_publications(id,resource_id,school_id,actor_id,snapshot) VALUES($1,$2,$3,$4,$5)").bind(Uuid::now_v7()).bind(object).bind(scope).bind(actor.id).bind(snapshot).execute(&mut **tx).await?;
                }
                "operation-service-targets" => {
                    let config: TargetInput = decode(&input)?;
                    validate_target(tx, &config).await?;
                    sqlx::query("INSERT INTO operation_service_targets(id) VALUES($1) ON CONFLICT(id) DO UPDATE SET revision=operation_service_targets.revision+1").bind(object).execute(&mut **tx).await?;
                    sqlx::query("WITH withdrawn AS (UPDATE operation_target_versions SET state='withdrawn',revision=revision+1 WHERE target_id=$1 AND state IN ('approved','pending','draft') RETURNING id) INSERT INTO operation_target_reviews(id,version_id,actor_id,decision,reason) SELECT gen_random_uuid(),id,$2,'withdraw','目标配置修改，撤回旧版本' FROM withdrawn").bind(object).bind(actor.id).execute(&mut **tx).await?;
                    sqlx::query("INSERT INTO operation_target_versions(id,target_id,config,state,creator_id,version_number,content_digest) SELECT $1,$2,$3,'draft',$4,coalesce(max(version_number),0)+1,$5 FROM operation_target_versions WHERE target_id=$2").bind(Uuid::now_v7()).bind(object).bind(&input).bind(actor.id).bind(crate::business::hash(&input.to_string())).execute(&mut **tx).await?;
                }
                "operation-service-categories" => {
                    let value: CategoryInput = decode(&input)?;
                    sqlx::query("INSERT INTO operation_service_categories(id,school_id,name,active,sort_order) VALUES($1,$2,$3,$4,$5) ON CONFLICT(id) DO UPDATE SET name=$3,active=$4,sort_order=$5,revision=operation_service_categories.revision+1").bind(object).bind(scope).bind(value.name).bind(value.active).bind(value.sort_order).execute(&mut **tx).await?;
                }
                "operation-partners" => {
                    let value: PartnerInput = decode(&input)?;
                    if id.is_some() {
                        withdraw_linked(tx, actor, "partner_id", object).await?;
                    }
                    sqlx::query("INSERT INTO operation_partners(id,name,contact,active) VALUES($1,$2,$3,$4) ON CONFLICT(id) DO UPDATE SET name=$2,contact=$3,active=$4,revision=operation_partners.revision+1").bind(object).bind(value.name).bind(value.contact).bind(value.active).execute(&mut **tx).await?;
                }
                "operation-cooperations" => {
                    let value: CooperationInput = decode(&input)?;
                    validate_schools(tx, &value.school_ids).await?;
                    if id.is_some() {
                        withdraw_linked(tx, actor, "cooperation_id", object).await?;
                    }
                    sqlx::query(
                        "SELECT id FROM operation_partners WHERE id=$1 AND active FOR SHARE",
                    )
                    .bind(value.partner_id)
                    .fetch_optional(&mut **tx)
                    .await?
                    .ok_or(Error::NotFound)?;
                    sqlx::query("INSERT INTO operation_cooperations(id,partner_id,name,model,disclosure,channel_code,active,starts_at,ends_at,school_ids) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT(id) DO UPDATE SET school_ids=$10,partner_id=$2,name=$3,model=$4,disclosure=$5,channel_code=$6,active=$7,starts_at=$8,ends_at=$9,revision=operation_cooperations.revision+1").bind(object).bind(value.partner_id).bind(value.name).bind(value.model).bind(value.disclosure).bind(value.channel_code).bind(value.active).bind(value.starts_at).bind(value.ends_at).bind(value.school_ids).execute(&mut **tx).await?;
                }
                "marketing-slots" => {
                    let value: MarketingInput = decode(&input)?;
                    sqlx::query("INSERT INTO marketing_slots(id,name,ad_unit_id,format,qualification,active,reward_scene) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(id) DO UPDATE SET name=$2,ad_unit_id=$3,format=$4,qualification=$5,active=$6,reward_scene=$7,revision=marketing_slots.revision+1").bind(object).bind(value.name).bind(value.ad_unit_id).bind(value.format).bind(value.qualification).bind(value.active).bind(value.reward_scene).execute(&mut **tx).await?;
                }
                _ => return Err(Error::NotFound),
            }
            bump(tx, scope).await?;
            audit(
                tx,
                actor,
                scope,
                "operation.service.save",
                object,
                json!({"entity":entity}),
            )
            .await?;
            get_manage(tx, scope, &entity, object).await
        }
        ServiceCommand::Delete {
            entity,
            id,
            revision: expected,
        } => {
            require(
                matches!(
                    entity.as_str(),
                    "home-shortcuts" | "operation-service-categories"
                ),
                "此配置请通过停用或撤回处理",
            )?;
            manage_auth(tx, actor, scope, &entity, "delete").await?;
            revision(tx, None, true).await?;
            if scope.is_some() {
                revision(tx, scope, true).await?;
            }
            let current = get_manage(tx, scope, &entity, id).await?;
            if current["revision"] != expected {
                return Err(Error::Conflict);
            }
            if entity == "home-shortcuts" {
                sqlx::query("UPDATE school_resources SET deleted_at=now(),active=false,revision=revision+1 WHERE id=$1").bind(id).execute(&mut **tx).await?;
            } else {
                let used: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM school_resources WHERE category_id=$1 AND deleted_at IS NULL)").bind(id).fetch_one(&mut **tx).await?;
                require(!used, "分类仍被入口引用，请先移除引用或停用分类")?;
                sqlx::query("DELETE FROM operation_service_categories WHERE id=$1")
                    .bind(id)
                    .execute(&mut **tx)
                    .await?;
            }
            bump(tx, scope).await?;
            audit(
                tx,
                actor,
                scope,
                "operation.service.delete",
                id,
                json!({"entity":entity}),
            )
            .await?;
            Ok(Value::Null)
        }
        ServiceCommand::TargetDecision { id, input } => {
            manage_auth(tx, actor, scope, "operation-service-targets", "update").await?;
            revision(tx, None, true).await?;
            let current=sqlx::query("SELECT state,revision,submitter_id,target_id FROM operation_target_versions WHERE id=$1 FOR UPDATE").bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
            if current.get::<i64, _>("revision") != input.revision {
                return Err(Error::Conflict);
            }
            let target_id: Uuid = current.get("target_id");
            let target_revision: i64 = sqlx::query_scalar(
                "SELECT revision FROM operation_service_targets WHERE id=$1 FOR UPDATE",
            )
            .bind(target_id)
            .fetch_one(&mut **tx)
            .await?;
            if input.target_id.is_some_and(|value| value != target_id)
                || input
                    .target_revision
                    .is_some_and(|value| value != target_revision)
            {
                return Err(Error::Conflict);
            }
            let state: String = current.get("state");
            let next = match input.decision.as_str() {
                "submit" if state == "draft" => "pending",
                "approve" | "reject" if state == "pending" => {
                    require(
                        current.get::<Option<Uuid>, _>("submitter_id") != Some(actor.id),
                        "提交人与审核人必须不同",
                    )?;
                    if input.decision == "approve" {
                        let config: Value = sqlx::query_scalar(
                            "SELECT config FROM operation_target_versions WHERE id=$1",
                        )
                        .bind(id)
                        .fetch_one(&mut **tx)
                        .await?;
                        validate_target(tx, &decode::<TargetInput>(&config)?).await?;
                        "approved"
                    } else {
                        "rejected"
                    }
                }
                "withdraw" if matches!(state.as_str(), "approved" | "pending") => "withdrawn",
                _ => return Err(Error::Invalid("目标状态不允许此操作".into())),
            };
            sqlx::query("UPDATE operation_target_versions SET state=$2,revision=revision+1,submitter_id=CASE WHEN $2='pending' THEN $3 ELSE submitter_id END WHERE id=$1").bind(id).bind(next).bind(actor.id).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO operation_target_reviews(id,version_id,actor_id,decision,reason) VALUES($1,$2,$3,$4,$5)").bind(Uuid::now_v7()).bind(id).bind(actor.id).bind(input.decision).bind(input.reason).execute(&mut **tx).await?;
            sqlx::query("UPDATE operation_service_targets SET revision=revision+1 WHERE id=$1")
                .bind(target_id)
                .execute(&mut **tx)
                .await?;
            bump(tx, None).await?;
            audit(
                tx,
                actor,
                None,
                "operation.target.review",
                id,
                json!({"state":next}),
            )
            .await?;
            Ok(json!({"id":id,"state":next,"revision":input.revision+1}))
        }
        ServiceCommand::Catalog(query) | ServiceCommand::Preview(query) => {
            let preview = preview_command;
            let scope = school(scope)?;
            auth(
                tx,
                actor,
                Some(scope),
                if preview {
                    "operation.shortcut.preview"
                } else {
                    "member"
                },
            )
            .await?;
            let rows = select(tx, scope, query.at.unwrap_or_else(Utc::now), preview).await?;
            let categories:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'name',name) FROM operation_service_categories WHERE active AND (school_id IS NULL OR school_id=$1) ORDER BY sort_order DESC,id").bind(scope).fetch_all(&mut **tx).await?;
            let search = query.search.as_deref().unwrap_or("").to_lowercase();
            let all: Vec<_> = rows
                .into_iter()
                .filter(|value| {
                    (query.category_id.is_none()
                        || value["category_id"] == json!(query.category_id))
                        && (value["title"]
                            .as_str()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&search)
                            || value["subtitle"]
                                .as_str()
                                .unwrap_or("")
                                .to_lowercase()
                                .contains(&search))
                })
                .collect();
            let featured: Vec<_> = all
                .iter()
                .filter(|value| value["featured"] == true)
                .take(8)
                .cloned()
                .collect();
            let (size, offset) = query.page().bounds()?;
            let more = all.len() > ((offset + size) as usize);
            let items: Vec<_> = all
                .into_iter()
                .skip(offset as usize)
                .take(size as usize)
                .collect();
            Ok(
                json!({"id":"current","server_time":Utc::now(),"token":token(tx,Some(scope)).await?,"featured":featured,"items":items,"categories":categories,"has_more":more,"preview":preview}),
            )
        }
        ServiceCommand::Detail(id) => {
            let scope = school(scope)?;
            auth(tx, actor, Some(scope), "member").await?;
            let mut result = selected(tx, scope, id).await?;
            result["server_time"] = json!(Utc::now());
            Ok(result)
        }
        ServiceCommand::Resolve {
            id,
            version,
            channel,
        } => {
            let scope = school(scope)?;
            auth(tx, actor, Some(scope), "member").await?;
            let mut result = selected(tx, scope, id).await?;
            if let Some(version) = version {
                require(
                    result["action"]["version_id"] == version.to_string(),
                    "服务目标版本已变化，请刷新",
                )?;
            }
            if let Some(channel) = channel {
                require(
                    result["channels"]
                        .as_array()
                        .is_some_and(|items| items.iter().any(|item| item["key"] == channel)),
                    "服务渠道已不可用",
                )?;
            }
            result["server_time"] = json!(Utc::now());
            Ok(result)
        }
        ServiceCommand::DraftPreview(input) => {
            let school = school(scope)?;
            auth(tx, actor, Some(school), "operation.shortcut.preview").await?;
            let source = if input.platform_default {
                actor.platform()?;
                None
            } else {
                Some(school)
            };
            manage_auth(
                tx,
                actor,
                source,
                "home-shortcuts",
                if input.resource_id.is_some() {
                    "update"
                } else {
                    "create"
                },
            )
            .await?;
            revision(tx, None, true).await?;
            revision(tx, Some(school), true).await?;
            if token(tx, Some(school)).await? != input.token {
                return Err(Error::Conflict);
            }
            if let Some(id) = input.resource_id {
                if get_manage(tx, source, "home-shortcuts", id).await?["revision"]
                    != input.draft.revision
                {
                    return Err(Error::Conflict);
                }
            } else {
                require(input.draft.revision == 0, "新增不能指定旧版本")?;
            }
            validate_shortcut(tx, actor, source, &input.draft).await?;
            let id = input.resource_id.unwrap_or_else(Uuid::now_v7);
            sqlx::query("SAVEPOINT operation_preview")
                .execute(&mut **tx)
                .await?;
            write_shortcut(tx, source, id, &input.draft).await?;
            let rows = select(tx, school, input.at, true).await?;
            sqlx::query("ROLLBACK TO SAVEPOINT operation_preview")
                .execute(&mut **tx)
                .await?;
            sqlx::query("RELEASE SAVEPOINT operation_preview")
                .execute(&mut **tx)
                .await?;
            Ok(
                json!({"id":"current","server_time":Utc::now(),"token":input.token,"items":rows,"featured":[],"categories":[],"has_more":false,"preview":true}),
            )
        }
        ServiceCommand::Override { id, input } => {
            let scope = school(scope)?;
            auth(tx, actor, Some(scope), "operation.shortcut.update").await?;
            revision(tx, None, true).await?;
            revision(tx, Some(scope), true).await?;
            if token(tx, Some(scope)).await? != input.token {
                return Err(Error::Conflict);
            }
            sqlx::query("SELECT id FROM school_resources WHERE id=$1 AND kind='shortcut' AND school_id IS NULL AND deleted_at IS NULL FOR SHARE").bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
            if input.presentation.active == Some(true) {
                let value = shortcut_input(get_manage(tx, None, "home-shortcuts", id).await?)?;
                require(value.active, "平台停用的入口不能由学校重新启用")?;
                validate_shortcut(tx, actor, None, &value).await?;
            }
            if let Some(Some(category)) = input.presentation.category_id {
                sqlx::query("SELECT id FROM operation_service_categories WHERE id=$1 AND active AND (school_id IS NULL OR school_id=$2)").bind(category).bind(scope).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
            }
            if let Some(ImageRef::Asset { id: asset }) = &input.presentation.image_ref {
                check_image(tx, Some(scope), *asset).await?;
                if !actor.platform_admin {
                    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM file_assets WHERE id=$1 AND artwork_permission LIKE 'operation.shortcut.%')").bind(asset).fetch_one(&mut **tx).await?;
                    if !valid {
                        return Err(Error::Forbidden);
                    }
                }
            }
            let current:Option<i64>=sqlx::query_scalar("SELECT revision FROM school_shortcut_overrides WHERE school_id=$1 AND resource_id=$2").bind(scope).bind(id).fetch_optional(&mut **tx).await?;
            if current.unwrap_or(0) != input.revision {
                return Err(Error::Conflict);
            }
            sqlx::query("INSERT INTO school_shortcut_overrides(id,school_id,resource_id,presentation) VALUES($1,$2,$3,$4) ON CONFLICT(school_id,resource_id) DO UPDATE SET presentation=$4,revision=school_shortcut_overrides.revision+1").bind(Uuid::now_v7()).bind(scope).bind(id).bind(json!(input.presentation)).execute(&mut **tx).await?;
            bump(tx, Some(scope)).await?;
            audit(
                tx,
                actor,
                Some(scope),
                "operation.override.save",
                id,
                json!({"revision":input.revision+1}),
            )
            .await?;
            Ok(json!({"id":id,"token":token(tx,Some(scope)).await?}))
        }
        ServiceCommand::Reset {
            id,
            revision: expected,
        } => {
            let scope = school(scope)?;
            auth(tx, actor, Some(scope), "operation.shortcut.update").await?;
            revision(tx, None, true).await?;
            revision(tx, Some(scope), true).await?;
            let affected=sqlx::query("DELETE FROM school_shortcut_overrides WHERE school_id=$1 AND resource_id=$2 AND revision=$3").bind(scope).bind(id).bind(expected).execute(&mut **tx).await?.rows_affected();
            if affected != 1 {
                return Err(Error::Conflict);
            }
            bump(tx, Some(scope)).await?;
            audit(
                tx,
                actor,
                Some(scope),
                "operation.override.reset",
                id,
                json!({}),
            )
            .await?;
            Ok(json!({"id":id,"token":token(tx,Some(scope)).await?}))
        }
        ServiceCommand::Batch { reorder, input } => {
            let scope = school(scope)?;
            auth(tx, actor, Some(scope), "operation.shortcut.update").await?;
            revision(tx, None, true).await?;
            revision(tx, Some(scope), true).await?;
            if token(tx, Some(scope)).await? != input.token {
                return Err(Error::Conflict);
            }
            let rows = select(tx, scope, Utc::now(), true).await?;
            if reorder {
                require(
                    rows.len() == input.items.len()
                        && rows.iter().all(|row| {
                            input
                                .items
                                .iter()
                                .any(|item| row["id"] == item.id.to_string())
                        }),
                    "排序必须提交完整集合",
                )?;
            }
            for (position, item) in input.items.iter().enumerate() {
                let row = rows
                    .iter()
                    .find(|row| row["id"] == item.id.to_string())
                    .ok_or(Error::NotFound)?;
                let local = !row["school_id"].is_null();
                if row[if local {
                    "revision"
                } else {
                    "override_revision"
                }] != item.revision
                {
                    return Err(Error::Conflict);
                }
                let mut patch = json!({});
                if reorder {
                    patch["sort_order"] = json!(input.items.len() - position);
                } else {
                    let active = item
                        .active
                        .ok_or_else(|| Error::Invalid("缺少启停状态".into()))?;
                    if active {
                        let source = if local { Some(scope) } else { None };
                        let value = shortcut_input(
                            get_manage(tx, source, "home-shortcuts", item.id).await?,
                        )?;
                        require(local || value.active, "平台停用的入口不能由学校重新启用")?;
                        require(
                            value.ends_at.is_none_or(|end| end > Utc::now()),
                            "入口已结束展示",
                        )?;
                        validate_shortcut(tx, actor, source, &value).await?;
                    }
                    patch["active"] = json!(active);
                }
                if local {
                    sqlx::query("UPDATE school_resources SET sort_order=coalesce(($2::jsonb->>'sort_order')::integer,sort_order),active=coalesce(($2::jsonb->>'active')::boolean,active),revision=revision+1 WHERE id=$1").bind(item.id).bind(patch).execute(&mut **tx).await?;
                } else {
                    sqlx::query("INSERT INTO school_shortcut_overrides(id,school_id,resource_id,presentation) VALUES($1,$2,$3,$4) ON CONFLICT(school_id,resource_id) DO UPDATE SET presentation=school_shortcut_overrides.presentation||$4,revision=school_shortcut_overrides.revision+1").bind(Uuid::now_v7()).bind(scope).bind(item.id).bind(patch).execute(&mut **tx).await?;
                }
            }
            bump(tx, Some(scope)).await?;
            audit(
                tx,
                actor,
                Some(scope),
                "operation.shortcut.batch",
                scope,
                json!({"reorder":reorder,"count":input.items.len()}),
            )
            .await?;
            Ok(json!({"id":"current","token":token(tx,Some(scope)).await?}))
        }
        ServiceCommand::Metric(value) => metric(tx, actor, school(scope)?, value).await,
        ServiceCommand::Outcome(value) => outcome(tx, actor, school(scope)?, value).await,
        ServiceCommand::Media {
            id,
            asset: expected_asset,
            version: expected_version,
            channel,
        } => {
            let scope = school(scope)?;
            auth(tx, actor, Some(scope), "member").await?;
            selected(tx, scope, id).await?;
            let asset = if let Some(channel) = channel {
                let version: Uuid = sqlx::query_scalar(
                    "SELECT target_version_id FROM school_resources WHERE id=$1",
                )
                .bind(id)
                .fetch_one(&mut **tx)
                .await?;
                require(expected_version == Some(version), "二维码目标版本已变化")?;
                let config = channel_config(tx, version).await?;
                match config
                    .channels
                    .into_iter()
                    .find(|item| item.key == channel)
                    .map(|item| item.action)
                {
                    Some(ChannelAction::QrCode { asset_id }) => asset_id,
                    _ => return Err(Error::NotFound),
                }
            } else {
                let reference:Value=sqlx::query_scalar("SELECT coalesce(o.presentation->'image_ref',r.image_ref) FROM school_resources r LEFT JOIN school_shortcut_overrides o ON o.resource_id=r.id AND o.school_id=$2 WHERE r.id=$1").bind(id).bind(scope).fetch_one(&mut **tx).await?;
                crate::business::uuid(&reference, "id")?
            };
            require(asset == expected_asset, "服务素材已变化")?;
            let (mime, bytes) = crate::media::south::adapter::postgres::Repository(
                repository.0.clone(),
                repository.1.clone(),
            )
            .read(asset)
            .await?;
            Ok(json!({"mime":mime,"bytes":bytes}))
        }
        ServiceCommand::Report(input) => report(tx, actor, scope, input).await,
        ServiceCommand::ReadReport { id, export, page } => {
            if scope.is_none() {
                actor.platform()?;
            }
            auth(
                tx,
                actor,
                scope,
                if export {
                    "operation.report.export"
                } else {
                    "operation.report.read"
                },
            )
            .await?;
            let value:Value=sqlx::query_scalar("SELECT report FROM operation_report_snapshots WHERE id=$1 AND school_id IS NOT DISTINCT FROM $2 AND actor_id=$3 AND expires_at>now()").bind(id).bind(scope).bind(actor.id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
            Ok(json!({"id":id,"report":if export {value}else{report_page(value,page)?}}))
        }
        ServiceCommand::Marketing => {
            let scope = school(scope)?;
            auth(tx, actor, Some(scope), "member").await?;
            let rows:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',m.id,'name',m.name,'ad_unit_id',m.ad_unit_id,'format',m.format,'reward_scene',m.reward_scene) FROM marketing_slots m JOIN school_marketing_settings s ON s.slot_id=m.id AND s.school_id=$1 WHERE m.active AND s.enabled ORDER BY m.id").bind(scope).fetch_all(&mut **tx).await?;
            Ok(json!({"id":"current","items":rows}))
        }
        ServiceCommand::MarketingSettings => {
            let scope = school(scope)?;
            auth(tx, actor, Some(scope), "operation.marketing.read").await?;
            let rows:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',m.id,'name',m.name,'format',m.format,'available',m.active,'enabled',coalesce(s.enabled,false),'revision',coalesce(s.revision,0)) FROM marketing_slots m LEFT JOIN school_marketing_settings s ON s.slot_id=m.id AND s.school_id=$1 ORDER BY m.id").bind(scope).fetch_all(&mut **tx).await?;
            Ok(json!({"id":"current","items":rows}))
        }
        ServiceCommand::MarketingEvent(value) => {
            let scope = school(scope)?;
            auth(tx, actor, Some(scope), "member").await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73121))")
                .bind(actor.id.to_string())
                .execute(&mut **tx)
                .await?;
            let previous: Option<Value> =
                sqlx::query_scalar("SELECT to_jsonb(e) FROM marketing_events e WHERE event_id=$1")
                    .bind(value.event_id)
                    .fetch_optional(&mut **tx)
                    .await?;
            if let Some(previous) = previous {
                if previous["account_id"] != actor.id.to_string()
                    || previous["school_id"] != scope.to_string()
                    || previous["slot_id"] != value.slot_id.to_string()
                    || previous["event_type"] != value.event_type
                    || !same_time(&previous["occurred_at"], value.occurred_at)
                {
                    return Err(Error::Conflict);
                }
            } else {
                sqlx::query("SELECT m.id FROM marketing_slots m JOIN school_marketing_settings s ON s.slot_id=m.id AND s.school_id=$2 WHERE m.id=$1 AND m.active AND s.enabled FOR SHARE OF m,s").bind(value.slot_id).bind(scope).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM marketing_events WHERE account_id=$1 AND created_at>now()-interval '1 minute'").bind(actor.id).fetch_one(&mut **tx).await?;
                if count >= 60 {
                    return Err(Error::RateLimited);
                }
                sqlx::query("INSERT INTO marketing_events(event_id,slot_id,school_id,account_id,event_type,occurred_at) VALUES($1,$2,$3,$4,$5,$6)").bind(value.event_id).bind(value.slot_id).bind(scope).bind(actor.id).bind(value.event_type).bind(value.occurred_at).execute(&mut **tx).await?;
            }
            Ok(json!({"id":value.event_id,"accepted":true}))
        }
        ServiceCommand::MarketingSetting {
            id,
            revision: expected,
            enabled,
        } => {
            let scope = school(scope)?;
            auth(tx, actor, Some(scope), "operation.marketing.update").await?;
            revision(tx, None, true).await?;
            revision(tx, Some(scope), true).await?;
            sqlx::query("SELECT id FROM marketing_slots WHERE id=$1 AND active FOR SHARE")
                .bind(id)
                .fetch_optional(&mut **tx)
                .await?
                .ok_or(Error::NotFound)?;
            let current: Option<i64> = sqlx::query_scalar(
                "SELECT revision FROM school_marketing_settings WHERE school_id=$1 AND slot_id=$2",
            )
            .bind(scope)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
            if current.unwrap_or(0) != expected {
                return Err(Error::Conflict);
            }
            sqlx::query("INSERT INTO school_marketing_settings(id,school_id,slot_id,enabled) VALUES($1,$2,$3,$4) ON CONFLICT(school_id,slot_id) DO UPDATE SET enabled=$4,revision=school_marketing_settings.revision+1").bind(Uuid::now_v7()).bind(scope).bind(id).bind(enabled).execute(&mut **tx).await?;
            bump(tx, Some(scope)).await?;
            audit(
                tx,
                actor,
                Some(scope),
                "operation.marketing.setting",
                id,
                json!({"enabled":enabled}),
            )
            .await?;
            Ok(json!({"id":id,"revision":expected+1,"enabled":enabled}))
        }
    }
}

async fn metric(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    scope: Uuid,
    value: MetricInput,
) -> Result<Value> {
    auth(tx, actor, Some(scope), "member").await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73121))")
        .bind(actor.id.to_string())
        .execute(&mut **tx)
        .await?;
    let prior: Option<Value> =
        sqlx::query_scalar("SELECT to_jsonb(e) FROM operation_metric_events e WHERE event_id=$1")
            .bind(value.event_id)
            .fetch_optional(&mut **tx)
            .await?;
    if let Some(prior) = prior {
        if !(prior["account_id"] == actor.id.to_string()
            && prior["school_id"] == scope.to_string()
            && prior["resource_id"] == value.resource_id.to_string()
            && prior["event_type"] == value.event_type
            && prior["display_cycle"] == value.display_cycle.to_string()
            && prior["channel"] == json!(value.channel)
            && prior["target_version_id"] == json!(value.target_version_id)
            && prior["placement"] == value.placement
            && prior["attempt_id"] == json!(value.attempt_id)
            && same_time(&prior["occurred_at"], value.occurred_at))
        {
            return Err(Error::Conflict);
        }
        return Ok(json!({"id":value.event_id,"accepted":true}));
    }
    let kind: String = sqlx::query_scalar("SELECT kind FROM school_resources WHERE id=$1")
        .bind(value.resource_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::NotFound)?;
    let selected = if kind == "shortcut" {
        selected(tx, scope, value.resource_id).await?
    } else {
        effective_resources_for_metrics(tx, scope, value.resource_id).await?
    };
    if kind == "shortcut" && value.event_type == "click" {
        require(
            selected["action"]["kind"] != "none",
            "仅说明入口不产生访问点击",
        )?;
    }
    require(
        if kind == "banner" {
            value.placement == "home_banner"
        } else {
            value.placement != "home_banner"
        },
        "展示位置与资源类型不一致",
    )?;
    if kind == "shortcut"
        && value.event_type == "click"
        && selected["action"]["kind"] == "target_channel"
    {
        require(
            selected["action"]["version_id"] == json!(value.target_version_id)
                && selected["channels"].as_array().is_some_and(|items| {
                    items.iter().any(|item| item["key"] == json!(value.channel))
                }),
            "点击渠道或目标版本无效",
        )?;
    } else {
        require(
            value.channel.is_none() && value.target_version_id.is_none(),
            "此事件不允许渠道字段",
        )?;
    }
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM operation_metric_events WHERE account_id=$1 AND created_at>now()-interval '1 minute'").bind(actor.id).fetch_one(&mut **tx).await?;
    if count >= 120 {
        return Err(Error::RateLimited);
    }
    let dimensions = json!({"title":selected["title"],"provider":selected["disclosure"]["provider"],"partner_id":selected["disclosure"]["partner_id"],"kind":kind,"placement":value.placement,"is_alternative":value.channel.is_some() && selected["action"]["channel"]!=json!(value.channel),"category_id":selected["category_id"],"cooperation":selected["cooperation"],"cooperation_id":selected["cooperation_id"],"channel_kind":selected["channels"].as_array().and_then(|items| items.iter().find(|item| item["key"]==json!(value.channel))).map(|item| item["action"]["kind"].clone())});
    let inserted=sqlx::query("INSERT INTO operation_metric_events(event_id,resource_id,school_id,account_id,event_type,display_cycle,occurred_at,channel,target_version_id,dimensions,placement,attempt_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) ON CONFLICT DO NOTHING").bind(value.event_id).bind(value.resource_id).bind(scope).bind(actor.id).bind(&value.event_type).bind(value.display_cycle).bind(value.occurred_at).bind(&value.channel).bind(value.target_version_id).bind(&dimensions).bind(&value.placement).bind(value.attempt_id).execute(&mut **tx).await?.rows_affected();
    if inserted == 0 {
        let raced: Option<Value> = sqlx::query_scalar(
            "SELECT to_jsonb(e) FROM operation_metric_events e WHERE event_id=$1",
        )
        .bind(value.event_id)
        .fetch_optional(&mut **tx)
        .await?;
        if raced.is_none() && value.event_type == "click" {
            return Err(Error::Conflict);
        }
        if let Some(prior) = raced
            && !(prior["account_id"] == actor.id.to_string()
                && prior["school_id"] == scope.to_string()
                && prior["resource_id"] == value.resource_id.to_string()
                && prior["event_type"] == value.event_type
                && prior["display_cycle"] == value.display_cycle.to_string()
                && prior["channel"] == json!(value.channel)
                && prior["target_version_id"] == json!(value.target_version_id)
                && prior["placement"] == value.placement
                && prior["attempt_id"] == json!(value.attempt_id)
                && same_time(&prior["occurred_at"], value.occurred_at))
        {
            return Err(Error::Conflict);
        }
    }
    if inserted == 1 {
        let column = if value.event_type == "view" {
            "views"
        } else {
            "clicks"
        };
        sqlx::query(&format!(
            "UPDATE school_resources SET {column}={column}+1 WHERE id=$1"
        ))
        .bind(value.resource_id)
        .execute(&mut **tx)
        .await?;
        sqlx::query(&format!("INSERT INTO operation_daily_metrics(school_id,day,resource_id,channel,target_version,dimensions,dimension_key,{column}) VALUES($1,($2 AT TIME ZONE 'Asia/Shanghai')::date,$3,$4,$5,$6,$7,1) ON CONFLICT(school_id,day,resource_id,channel,target_version,dimension_key) DO UPDATE SET {column}=operation_daily_metrics.{column}+1")).bind(scope).bind(value.occurred_at).bind(value.resource_id).bind(value.channel.unwrap_or_default()).bind(value.target_version_id.map(|id|id.to_string()).unwrap_or_default()).bind(&dimensions).bind(crate::business::hash(&dimensions.to_string())).execute(&mut **tx).await?;
    }
    Ok(json!({"id":value.event_id,"accepted":true}))
}
async fn effective_resources_for_metrics(
    tx: &mut Transaction<'_, Postgres>,
    scope: Uuid,
    id: Uuid,
) -> Result<Value> {
    super::effective_resources_with(&mut **tx, scope)
        .await?
        .into_iter()
        .find(|row| row["id"] == id.to_string() && row["kind"] == "banner")
        .ok_or(Error::NotFound)
}
async fn outcome(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    scope: Uuid,
    value: OutcomeInput,
) -> Result<Value> {
    auth(tx, actor, Some(scope), "member").await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73121))")
        .bind(actor.id.to_string())
        .execute(&mut **tx)
        .await?;
    let click=sqlx::query("SELECT * FROM operation_metric_events WHERE event_id=$1 AND school_id=$2 AND account_id=$3 AND event_type='click' AND channel IS NOT NULL FOR SHARE").bind(value.click_id).bind(scope).bind(actor.id).fetch_optional(&mut **tx).await?.ok_or(Error::Policy("click_not_received"))?;
    require(
        click.get::<Option<Uuid>, _>("attempt_id") == Some(value.attempt_id),
        "结果 attempt 与关联点击不一致",
    )?;
    let prior: Option<Value> = sqlx::query_scalar(
        "SELECT to_jsonb(e) FROM operation_action_events e WHERE (click_id=$1 AND stage=$3) OR event_id=$2",
    )
    .bind(value.click_id)
    .bind(value.event_id)
    .bind(&value.stage)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(prior) = prior {
        if prior["event_id"] != value.event_id.to_string()
            || prior["click_id"] != value.click_id.to_string()
            || prior["outcome"] != value.outcome
            || prior["stage"] != value.stage
            || prior["attempt_id"] != value.attempt_id.to_string()
            || !same_time(&prior["occurred_at"], value.occurred_at)
        {
            return Err(Error::Conflict);
        }
        return Ok(json!({"id":value.event_id,"accepted":true}));
    }
    let occurred: DateTime<Utc> = click.get("occurred_at");
    require(
        value.occurred_at >= occurred && value.occurred_at - occurred <= chrono::Duration::days(7),
        "阶段结果时间无效",
    )?;
    let dimensions: Value = click.get("dimensions");
    let allowed = match (dimensions["channel_kind"].as_str(), value.stage.as_str()) {
        (Some("copy_link"), "clipboard_api") => {
            matches!(value.outcome.as_str(), "copy_success" | "fail")
        }
        (Some("qr_code"), "qr_display") => {
            matches!(value.outcome.as_str(), "qr_displayed" | "fail")
        }
        (Some("mini_program" | "web_view"), "navigation_api") => {
            matches!(value.outcome.as_str(), "api_success" | "cancel" | "fail")
        }
        (Some("web_view"), "webview_load") => {
            matches!(value.outcome.as_str(), "webview_loaded" | "webview_failed")
        }
        _ => false,
    };
    require(allowed, "阶段结果与渠道不一致")?;
    sqlx::query("INSERT INTO operation_action_events(event_id,click_id,outcome,occurred_at,stage,attempt_id) VALUES($1,$2,$3,$4,$5,$6)").bind(value.event_id).bind(value.click_id).bind(&value.outcome).bind(value.occurred_at).bind(&value.stage).bind(value.attempt_id).execute(&mut **tx).await?;
    let version: Option<Uuid> = click.get("target_version_id");
    sqlx::query(&format!("UPDATE operation_daily_metrics SET {column}={column}+1 WHERE school_id=$1 AND day=($2 AT TIME ZONE 'Asia/Shanghai')::date AND resource_id=$3 AND channel=$4 AND target_version=$5 AND dimension_key=$6",column=value.outcome)).bind(scope).bind(occurred).bind(click.get::<Uuid,_>("resource_id")).bind(click.get::<String,_>("channel")).bind(version.map(|id|id.to_string()).unwrap_or_default()).bind(crate::business::hash(&dimensions.to_string())).execute(&mut **tx).await?;
    Ok(json!({"id":value.event_id,"accepted":true}))
}
fn report_page(mut report: Value, page: Page) -> Result<Value> {
    let (size, offset) = page.bounds()?;
    let all = report["rows"].as_array().ok_or(Error::Unavailable)?;
    let total = all.len();
    let rows: Vec<Value> = all
        .iter()
        .skip(offset as usize)
        .take(size as usize)
        .cloned()
        .collect();
    report["rows"] = json!(rows);
    report["pagination"] = json!({"page":page.page.unwrap_or(1),"size":size,"total":total});
    Ok(report)
}
async fn report(
    tx: &mut Transaction<'_, Postgres>,
    actor: &Actor,
    scope: Option<Uuid>,
    input: ReportInput,
) -> Result<Value> {
    if scope.is_none() {
        actor.platform()?;
    }
    auth(tx, actor, scope, "operation.report.read").await?;
    let schools = if let Some(school) = scope {
        require(
            input.school_ids.is_empty() || input.school_ids.iter().all(|value| *value == school),
            "仅可查询当前学校",
        )?;
        Some(vec![school])
    } else if input.school_ids.is_empty() {
        None
    } else {
        Some(input.school_ids.clone())
    };
    let mut rows:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(m)||jsonb_build_object('unreported',greatest(0,clicks-api_success-cancel-fail-copy_success-qr_displayed),'ctr',CASE WHEN views=0 THEN NULL ELSE clicks::numeric/views END) FROM operation_daily_metrics m WHERE ($1::uuid[] IS NULL OR school_id=ANY($1)) AND day>=($2 AT TIME ZONE 'Asia/Shanghai')::date AND day<($3 AT TIME ZONE 'Asia/Shanghai')::date AND ($4::uuid IS NULL OR resource_id=$4) AND ($5::text IS NULL OR target_version=$5) AND ($6::text IS NULL OR dimensions->>'cooperation_id'=$6) AND ($7::text IS NULL OR dimensions->>'placement'=$7) AND ($8::text IS NULL OR channel=$8) AND ($9::text IS NULL OR dimensions->>'partner_id'=$9) ORDER BY day,school_id,resource_id,channel LIMIT 50001").bind(&schools).bind(input.starts_at).bind(input.ends_at).bind(input.resource_id).bind(input.target_version_id.map(|id|id.to_string())).bind(input.cooperation_id.map(|id|id.to_string())).bind(&input.placement).bind(&input.channel).bind(input.partner_id.map(|id|id.to_string())).fetch_all(&mut **tx).await?;
    require(rows.len() <= 50000, "报表超过五万行，请缩小范围")?;
    for row in &mut rows {
        let count = |key: &str| row[key].as_i64().unwrap_or(0);
        let terminal = count("api_success")
            + count("cancel")
            + count("fail")
            + count("copy_success")
            + count("qr_displayed");
        let navigation = matches!(
            row["dimensions"]["channel_kind"].as_str(),
            Some("mini_program" | "web_view")
        );
        let successes = count("api_success") + count("copy_success") + count("qr_displayed");
        let cancellations = count("cancel");
        let loaded = count("webview_loaded");
        let load_terminal = loaded + count("webview_failed");
        row["success_rate"] = if terminal > 0 {
            json!(successes as f64 / terminal as f64)
        } else {
            Value::Null
        };
        row["cancel_rate"] = if navigation && terminal > 0 {
            json!(cancellations as f64 / terminal as f64)
        } else {
            Value::Null
        };
        row["webview_load_rate"] = if load_terminal > 0 {
            json!(loaded as f64 / load_terminal as f64)
        } else {
            Value::Null
        };
    }
    let mut summary = json!({});
    for field in [
        "views",
        "clicks",
        "api_success",
        "cancel",
        "fail",
        "copy_success",
        "qr_displayed",
        "webview_loaded",
        "webview_failed",
    ] {
        summary[field] = json!(
            rows.iter()
                .map(|row| row[field].as_i64().unwrap_or(0))
                .sum::<i64>()
        );
    }
    let views = summary["views"].as_i64().unwrap_or(0);
    summary["ctr"] = if views > 0 {
        json!(summary["clicks"].as_i64().unwrap_or(0) as f64 / views as f64)
    } else {
        Value::Null
    };
    let mut trends = std::collections::BTreeMap::<String, (i64, i64)>::new();
    for row in &rows {
        let day = row["day"].as_str().unwrap_or("").to_owned();
        let values = trends.entry(day).or_default();
        values.0 += row["views"].as_i64().unwrap_or(0);
        values.1 += row["clicks"].as_i64().unwrap_or(0);
    }
    let trends:Vec<Value>=trends.into_iter().map(|(day,(views,clicks))|json!({"day":day,"views":views,"clicks":clicks,"ctr":if views>0 {Some(clicks as f64/views as f64)}else{None}})).collect();
    let launched_at: DateTime<Utc> =
        sqlx::query_scalar("SELECT created_at FROM application_metadata WHERE singleton")
            .fetch_one(&mut **tx)
            .await?;
    let coverage_from = launched_at.max(Utc::now() - chrono::Duration::days(365));
    let report = json!({"filters":input,"coverage":{"available_from":coverage_from,"requested_from":input.starts_at,"partial":input.starts_at < coverage_from,"notice":"启用前及保留期外不可追溯；无数据不等于零访问"},"starts_at":input.starts_at,"ends_at":input.ends_at,"timezone":"Asia/Shanghai","generated_at":Utc::now(),"retention_days":365,"rows":rows,"summary":summary,"trends":trends,"notice":"API 成功、复制及二维码展示不代表交易或服务完成；按自然日汇总"});
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO operation_report_snapshots(id,school_id,actor_id,report) VALUES($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(scope)
    .bind(actor.id)
    .bind(&report)
    .execute(&mut **tx)
    .await?;
    Ok(json!({"id":id,"report":report_page(report,Page::default())?}))
}
