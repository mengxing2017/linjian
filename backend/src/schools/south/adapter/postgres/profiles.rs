use super::Repository;
use crate::{
    business::{Actor, Error, Result, require},
    schools::domain::profile::*,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;
const VIEW: &str = "SELECT s.id,s.name,s.short_name,s.province,s.city,s.area,s.latitude::float8 AS latitude,s.longitude::float8 AS longitude,CASE WHEN s.active THEN 'normal' ELSE 'hidden' END AS status,s.logo_asset_id,s.header_bg_asset_id,CASE WHEN s.logo_asset_id IS NOT NULL THEN '/api/v1/school-directory/'||s.id||'/branding/logo' END AS logo_url,CASE WHEN s.header_bg_asset_id IS NOT NULL THEN '/api/v1/school-directory/'||s.id||'/branding/header' END AS header_bg_image_url,s.theme_primary,s.theme_secondary,s.theme_text_color,s.revision,s.created_at,s.updated_at,s.config FROM schools s";
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
        let all = a.browser && a.platform_admin;
        let rows:Vec<Value>=sqlx::query_scalar(&format!("SELECT to_jsonb(v) FROM ({VIEW} WHERE ($1 OR (s.active AND s.id IN(SELECT school_id FROM school_memberships WHERE account_id=$2 AND role<>'revoked'))) ORDER BY s.name,s.id) v")).bind(all).bind(a.id).fetch_all(&self.0).await?;
        rows.into_iter()
            .map(|v| {
                let s = decode(v)?;
                if a.browser && a.can(s.id, "operator") {
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
        require(a.browser, "请使用后台身份")?;
        let supplied = SchoolInput::parse(raw.clone())?;
        if id.is_none() {
            a.platform()?;
            require(supplied.revision == 0, "新学校修订必须为 0")?;
        }
        if raw.get("status").is_some() {
            a.platform()?;
        }
        let mut tx = self.0.begin().await?;
        if a.platform_admin {
            super::preview::platform(&mut tx, a).await?;
        }
        let mut input = if let Some(id) = id {
            if !a.platform_admin {
                a.authorize(id, "admin")?;
            }
            let v: Value = sqlx::query_scalar(&format!(
                "SELECT to_jsonb(v) FROM ({VIEW} WHERE s.id=$1 FOR UPDATE OF s) v"
            ))
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
            let old = decode(v)?;
            if !a.platform_admin {
                require(old.status == "normal", "学校暂不可用")?;
                let authorized:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM school_memberships WHERE school_id=$1 AND account_id=$2 AND role='admin')").bind(id).bind(a.id).fetch_one(&mut *tx).await?;
                if !authorized {
                    return Err(Error::Forbidden);
                }
            }
            if supplied.revision != old.revision {
                return Err(Error::Conflict);
            }
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
                "SELECT id FROM operational_media WHERE id=$1 AND school_id=$2 FOR KEY SHARE",
            )
            .bind(asset)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::Forbidden)?;
        }
        let mut values = serde_json::to_value(&input).map_err(|_| Error::Unavailable)?;
        values["active"] = json!(input.status.as_deref().unwrap_or("normal") == "normal");
        values["config"] = input.config.clone().unwrap_or(json!({}));
        let columns = "name,short_name,province,city,area,latitude,longitude,active,logo_asset_id,header_bg_asset_id,theme_primary,theme_secondary,theme_text_color,config";
        if input.revision == 0 {
            sqlx::query(&format!("INSERT INTO schools(id,{columns}) SELECT $1,{columns} FROM jsonb_populate_record(NULL::schools,$2)" )).bind(id).bind(values).execute(&mut *tx).await?;
        } else {
            sqlx::query(&format!("UPDATE schools SET ({columns})=(SELECT {columns} FROM jsonb_populate_record(NULL::schools,$2)),revision=revision+1,updated_at=now() WHERE id=$1")).bind(id).bind(values).execute(&mut *tx).await?;
        }
        crate::audit::record(&mut tx,a,id,if input.revision==0{"school.create"}else{"school.update"},id,json!({"status":input.status,"fields":raw.as_object().map(|v|v.keys().collect::<Vec<_>>())})).await?;
        let v = sqlx::query_scalar(&format!("SELECT to_jsonb(v) FROM ({VIEW} WHERE s.id=$1) v"))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        let result = decode(v)?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn branding(&self, school: Uuid, slot: &str) -> Result<(String, Vec<u8>)> {
        let column = match slot {
            "logo" => "logo_asset_id",
            "header" => "header_bg_asset_id",
            _ => return Err(Error::NotFound),
        };
        let r=sqlx::query(&format!("SELECT m.mime_type,m.content FROM schools s JOIN operational_media m ON m.id=s.{column} AND m.school_id=s.id WHERE s.id=$1 AND s.active")).bind(school).fetch_optional(&self.0).await?.ok_or(Error::NotFound)?;
        Ok((r.get("mime_type"), r.get("content")))
    }
}
