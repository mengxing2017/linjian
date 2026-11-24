use super::Repository;
use crate::{
    business::{Actor, Error, Result, require},
    schools::domain::profile::*,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;
pub(super) const VIEW: &str = "SELECT s.id,s.name,s.short_name,s.province,s.city,s.area,s.latitude::float8 AS latitude,s.longitude::float8 AS longitude,CASE WHEN s.active THEN 'normal' ELSE 'hidden' END AS status,s.logo_asset_id,s.header_bg_asset_id,CASE WHEN s.logo_asset_id IS NOT NULL THEN '/api/v1/school-directory/'||s.id||'/branding/logo' END AS logo_url,CASE WHEN s.header_bg_asset_id IS NOT NULL THEN '/api/v1/school-directory/'||s.id||'/branding/header' END AS header_bg_image_url,s.theme_primary,s.theme_secondary,s.theme_text_color,s.revision,s.created_at,s.updated_at,s.config FROM schools s";
fn decode(v: Value) -> Result<SchoolView> {
    serde_json::from_value(v).map_err(|_| Error::Unavailable)
}
pub async fn lock_active_school(tx: &mut Transaction<'_, Postgres>, school: Uuid) -> Result<()> {
    sqlx::query("SELECT id FROM schools WHERE id=$1 AND active FOR UPDATE")
        .bind(school)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::SchoolUnavailable)?;
    Ok(())
}
impl Repository {
    pub async fn require_active(&self, id: Uuid) -> Result<()> {
        let active: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM schools WHERE id=$1 AND active)")
                .bind(id)
                .fetch_one(&self.0)
                .await?;
        if active {
            Ok(())
        } else {
            Err(Error::SchoolUnavailable)
        }
    }
    pub async fn school_detail(&self, a: &Actor, id: Uuid) -> Result<SchoolView> {
        if !a.platform_admin {
            a.authorize(id, "member")?;
            self.require_active(id).await?;
        }
        let v = sqlx::query_scalar(&format!("SELECT to_jsonb(v) FROM ({VIEW} WHERE s.id=$1) v"))
            .bind(id)
            .fetch_optional(&self.0)
            .await?
            .ok_or(Error::NotFound)?;
        decode(v)
    }
    pub async fn school_list(&self, a: &Actor) -> Result<Vec<Value>> {
        let all = a.is_admin_browser() && a.platform_admin;
        let rows:Vec<Value>=sqlx::query_scalar(&format!("SELECT to_jsonb(v) FROM ({VIEW} WHERE ($1 OR (s.active AND s.id IN(SELECT school_id FROM school_memberships WHERE account_id=$2 AND role<>'revoked'))) ORDER BY s.name,s.id) v")).bind(all).bind(a.id).fetch_all(&self.0).await?;
        rows.into_iter()
            .map(|v| {
                let s = decode(v)?;
                if a.is_admin_browser() && a.has_permission(s.id, "school.profile.read") {
                    serde_json::to_value(s).map_err(|_| Error::Unavailable)
                } else {
                    Ok(s.directory())
                }
            })
            .collect()
    }
    pub async fn search_schools(
        &self,
        a: &Actor,
        q: &SchoolQuery,
        platform: bool,
    ) -> Result<Vec<Value>> {
        if platform {
            a.platform()?;
        } else {
            require(
                q.status.as_deref().is_none_or(|s| s == "normal"),
                "目录仅支持正常学校",
            )?;
        }
        let (n, off) = q.bounds()?;
        let rows:Vec<Value>=sqlx::query_scalar(&format!("SELECT to_jsonb(v) FROM ({VIEW} WHERE ($1 OR s.active) AND ($2::text IS NULL OR s.active=($2='normal')) AND ($3::text IS NULL OR s.name ILIKE '%'||$3||'%' OR s.short_name ILIKE '%'||$3||'%' OR s.area ILIKE '%'||$3||'%') AND ($4::text IS NULL OR s.province=$4) AND ($5::text IS NULL OR s.city=$5) AND ($6::text IS NULL OR s.area=$6) ORDER BY s.name,s.id LIMIT $7 OFFSET $8) v"))
 .bind(platform).bind(&q.status).bind(&q.search).bind(&q.province).bind(&q.city).bind(&q.area).bind(n+1).bind(off).fetch_all(&self.0).await?;
        rows.into_iter()
            .map(|v| {
                let s = decode(v)?;
                if platform {
                    serde_json::to_value(s).map_err(|_| Error::Unavailable)
                } else {
                    Ok(s.directory())
                }
            })
            .collect()
    }
    pub async fn save_school(&self, a: &Actor, id: Option<Uuid>, raw: Value) -> Result<SchoolView> {
        require(a.is_admin_browser(), "请使用后台身份")?;
        let supplied = SchoolInput::parse(raw.clone())?;
        if id.is_none() {
            a.platform()?;
            require(supplied.revision == 0, "新学校修订必须为 0")?;
        }
        if raw.get("status").is_some() {
            a.platform()?;
        }
        let mut tx = crate::media::south::adapter::lifecycle::begin(&self.0).await?;
        if let Some(school) = id {
            super::permissions::fence(&mut tx, school, raw.get("status").is_some()).await?;
            if !a.platform_admin {
                super::permissions::authorize(&mut tx, a, school, "school.profile.read").await?;
            }
        }
        if a.platform_admin {
            super::preview::platform(&mut tx, a).await?;
        }
        let mut previous_config = None;
        let mut input = if let Some(id) = id {
            super::permissions::fence(&mut tx, id, false).await?;
            let v: Value = sqlx::query_scalar(&format!(
                "SELECT to_jsonb(v) FROM ({VIEW} WHERE s.id=$1 FOR UPDATE OF s) v"
            ))
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
            let old = decode(v)?;
            if !a.platform_admin {
                let previous = serde_json::to_value(old.input()).map_err(|_| Error::Unavailable)?;
                let changed_profile = raw
                    .as_object()
                    .ok_or(Error::Invalid("学校资料无效".into()))?
                    .iter()
                    .any(|(key, value)| {
                        !matches!(key.as_str(), "revision" | "config" | "status")
                            && previous.get(key) != Some(value)
                    });
                let changed_policy =
                    raw.get("config")
                        .and_then(Value::as_object)
                        .is_some_and(|config| {
                            config.iter().any(|(key, value)| {
                                matches!(key.as_str(), "post_audit_mode" | "comment_audit_mode")
                                    && old.config.get(key) != Some(value)
                            })
                        });
                let changed_profile = changed_profile
                    || raw
                        .get("config")
                        .and_then(Value::as_object)
                        .is_some_and(|config| {
                            config.iter().any(|(key, value)| {
                                !matches!(key.as_str(), "post_audit_mode" | "comment_audit_mode")
                                    && old.config.get(key) != Some(value)
                            })
                        });
                if changed_profile {
                    super::permissions::authorize(&mut tx, a, id, "school.profile.update").await?;
                }
                if changed_policy {
                    super::permissions::authorize(&mut tx, a, id, "school.review_settings.update")
                        .await?;
                }
                if !changed_profile && !changed_policy {
                    super::permissions::authorize(&mut tx, a, id, "school.profile.read").await?;
                }
            }
            if old.status != "normal"
                && raw.get("config").is_some_and(|c| {
                    ["post_audit_mode", "comment_audit_mode"]
                        .iter()
                        .any(|k| c.get(k).is_some_and(|v| Some(v) != old.config.get(k)))
                })
            {
                return Err(Error::SchoolUnavailable);
            }
            if !a.platform_admin {
                require(old.status == "normal", "学校暂不可用")?;
            }
            if supplied.revision != old.revision {
                return Err(Error::Conflict);
            }
            previous_config = Some(old.config.clone());
            let mut merged = serde_json::to_value(old.input()).map_err(|_| Error::Unavailable)?;
            for (k, v) in raw
                .as_object()
                .ok_or(Error::Invalid("学校资料无效".into()))?
            {
                if k == "config"
                    && let (Some(old), Some(new)) = (merged[k].as_object_mut(), v.as_object())
                {
                    old.extend(new.clone());
                } else {
                    merged[k] = v.clone();
                }
            }
            SchoolInput::parse(merged)?
        } else {
            supplied
        };
        input.validate()?;
        let id = id.unwrap_or_else(Uuid::now_v7);
        for asset in [input.logo_asset_id, input.header_bg_asset_id]
            .into_iter()
            .flatten()
        {
            sqlx::query(
                "SELECT m.id FROM operational_media m JOIN file_assets f ON f.id=m.id WHERE m.id=$1 AND m.school_id=$2 AND ($3 OR f.artwork_permission='school.profile.update') FOR KEY SHARE OF m",
            )
            .bind(asset)
            .bind(id)
            .bind(a.platform_admin)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::Forbidden)?;
        }
        let mut values = serde_json::to_value(&input).map_err(|_| Error::Unavailable)?;
        values["active"] = json!(input.status.as_deref().unwrap_or("normal") == "normal");
        values["config"] = input.config.clone().unwrap_or(json!({}));
        for key in ["post_audit_mode", "comment_audit_mode"] {
            if values["config"].get(key).is_none() {
                values["config"][key] = json!("manual");
            }
        }
        let config = values["config"].clone();
        let columns = "name,short_name,province,city,area,latitude,longitude,active,logo_asset_id,header_bg_asset_id,theme_primary,theme_secondary,theme_text_color,config";
        if input.revision == 0 {
            sqlx::query(&format!("INSERT INTO schools(id,{columns}) SELECT $1,{columns} FROM jsonb_populate_record(NULL::schools,$2)" )).bind(id).bind(values).execute(&mut *tx).await?;
        } else {
            sqlx::query(&format!("UPDATE schools SET ({columns})=(SELECT {columns} FROM jsonb_populate_record(NULL::schools,$2)),revision=revision+1,updated_at=now() WHERE id=$1")).bind(id).bind(values).execute(&mut *tx).await?;
        }
        crate::audit::record(&mut tx,a,id,if input.revision==0{"school.create"}else{"school.update"},id,json!({"status":input.status,"fields":raw.as_object().map(|v|v.keys().collect::<Vec<_>>())})).await?;
        if ["post_audit_mode", "comment_audit_mode"].iter().any(|key| {
            previous_config
                .as_ref()
                .and_then(|c| c.get(key))
                .unwrap_or(&Value::Null)
                != &config[key]
        }) {
            crate::audit::record(&mut tx,a,id,"moderation.policy.update",id,json!({"previous":previous_config.map(|c|json!({"post_audit_mode":c["post_audit_mode"],"comment_audit_mode":c["comment_audit_mode"]})),"current":{"post_audit_mode":config["post_audit_mode"],"comment_audit_mode":config["comment_audit_mode"]},"revision":input.revision+1})).await?;
        }
        let v = sqlx::query_scalar(&format!("SELECT to_jsonb(v) FROM ({VIEW} WHERE s.id=$1) v"))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        let result = decode(v)?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn branding(
        &self,
        school: Uuid,
        slot: &str,
        files: crate::media::south::adapter::filesystem::Files,
    ) -> Result<(String, Vec<u8>)> {
        let column = match slot {
            "logo" => "logo_asset_id",
            "header" => "header_bg_asset_id",
            _ => return Err(Error::NotFound),
        };
        let r=sqlx::query(&format!("SELECT m.id FROM schools s JOIN operational_media m ON m.id=s.{column} AND m.school_id=s.id WHERE s.id=$1 AND s.active")).bind(school).fetch_optional(&self.0).await?.ok_or(Error::NotFound)?;
        crate::media::south::adapter::postgres::Repository(self.0.clone(), files)
            .read(r.get("id"))
            .await
    }
}
