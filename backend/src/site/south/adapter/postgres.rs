use super::super::port::Store;
use crate::business::{Actor, Error, Result};
use crate::media::south::adapter::filesystem::Files;
use crate::site::domain::{
    AdminSiteSettings, PublicSiteSettings, SiteConfig, SiteSettingsHistory, SiteSettingsInput,
    StoredSiteSettings, business_year, diff,
};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct Repository(pub PgPool, pub Files);

const COLUMNS: &str = "s.id,s.revision,s.site_name,s.logo_asset_id,s.title_subtitle,s.login_intro,s.icp_number,s.police_number,s.police_url,s.copyright_owner,s.copyright_start_year,s.copyright_end_mode,s.copyright_end_year,s.copyright_notice,s.updated_at,(SELECT coalesce(nullif(btrim(a.display_name),''),a.username) FROM accounts a WHERE a.id=s.updated_by) AS updated_by_name";

fn decode(row: &sqlx::postgres::PgRow) -> Result<StoredSiteSettings> {
    let config = SiteConfig {
        site_name: row.get("site_name"),
        logo_asset_id: row.get("logo_asset_id"),
        title_subtitle: row.get("title_subtitle"),
        login_intro: row.get("login_intro"),
        icp_number: row.get("icp_number"),
        police_number: row.get("police_number"),
        police_url: row.get("police_url"),
        copyright_owner: row.get("copyright_owner"),
        copyright_start_year: row.get("copyright_start_year"),
        copyright_end_mode: serde_json::from_value(
            row.get::<Option<String>, _>("copyright_end_mode")
                .map(Value::String)
                .unwrap_or(Value::Null),
        )
        .map_err(|_| Error::Unavailable)?,
        copyright_end_year: row.get("copyright_end_year"),
        copyright_notice: row.get("copyright_notice"),
    };
    Ok(StoredSiteSettings {
        id: row.get("id"),
        revision: row.get("revision"),
        config,
        updated_at: row.get("updated_at"),
        updated_by_name: row.get("updated_by_name"),
    })
}

async fn load(conn: &mut PgConnection) -> Result<StoredSiteSettings> {
    let row = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM site_settings s WHERE s.singleton"
    ))
    .fetch_optional(conn)
    .await?
    .ok_or(Error::Unavailable)?;
    decode(&row)
}

async fn load_for_update(tx: &mut Transaction<'_, Postgres>) -> Result<StoredSiteSettings> {
    let row = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM site_settings s WHERE s.singleton FOR UPDATE"
    ))
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(Error::Unavailable)?;
    decode(&row)
}

async fn history(conn: &mut PgConnection, id: Uuid) -> Result<Vec<SiteSettingsHistory>> {
    let rows = sqlx::query("SELECT e.id,e.detail,e.created_at,(SELECT coalesce(nullif(btrim(a.display_name),''),a.username) FROM accounts a WHERE a.id=e.actor_id) AS actor_name FROM audit_events e WHERE e.action='site.settings.update' AND e.object_id=$1 ORDER BY e.created_at DESC,e.id DESC LIMIT 20")
        .bind(id)
        .fetch_all(conn)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let detail: Value = row.get("detail");
            SiteSettingsHistory {
                id: row.get("id"),
                actor_name: row.get("actor_name"),
                created_at: row.get("created_at"),
                revision: detail["revision"].as_i64().unwrap_or(0),
                changed: strings(&detail["changed"]),
                cleared: strings(&detail["cleared"]),
                previous: detail.get("previous").cloned().unwrap_or(Value::Null),
                current: detail.get("current").cloned().unwrap_or(Value::Null),
            }
        })
        .collect())
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

impl Repository {
    pub async fn public(&self) -> Result<PublicSiteSettings> {
        let mut conn = self.0.acquire().await?;
        let stored = load(&mut conn).await?;
        Ok(stored.public(business_year()))
    }

    pub async fn logo(&self) -> Result<(String, Vec<u8>)> {
        let id: Option<Uuid> = sqlx::query_scalar("SELECT s.logo_asset_id FROM site_settings s JOIN operational_media m ON m.id=s.logo_asset_id JOIN file_assets f ON f.id=m.id WHERE s.singleton AND m.school_id IS NULL AND f.purpose='operation_artwork' AND f.state='ready'")
            .fetch_optional(&self.0)
            .await?
            .flatten();
        let id = id.ok_or(Error::NotFound)?;
        crate::media::south::adapter::postgres::Repository(self.0.clone(), self.1.clone())
            .read(id)
            .await
    }

    pub async fn admin(&self, a: &Actor) -> Result<AdminSiteSettings> {
        let mut tx = self.0.begin().await?;
        crate::schools::south::adapter::postgres::preview::platform(&mut tx, a).await?;
        let stored = load(&mut tx).await?;
        let history = history(&mut tx, stored.id).await?;
        tx.commit().await?;
        Ok(stored.admin(business_year(), history))
    }

    pub async fn update(&self, a: &Actor, input: SiteSettingsInput) -> Result<AdminSiteSettings> {
        let current_year = business_year();
        let config = input.clone().normalize(current_year)?;
        let mut tx = self.0.begin().await?;
        crate::schools::south::adapter::postgres::preview::platform(&mut tx, a).await?;
        let stored = load_for_update(&mut tx).await?;
        if input.revision != stored.revision {
            return Err(Error::Conflict);
        }
        if let Some(asset) = config.logo_asset_id {
            sqlx::query("SELECT f.id FROM file_assets f JOIN operational_media m ON m.id=f.id WHERE f.id=$1 AND f.purpose='operation_artwork' AND f.state='ready' AND f.school_id IS NULL AND m.school_id IS NULL FOR KEY SHARE OF f")
                .bind(asset)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| Error::Invalid("平台 Logo 无效或不可用".into()))?;
        }
        if config == stored.config {
            let history = history(&mut tx, stored.id).await?;
            tx.commit().await?;
            return Ok(stored.admin(current_year, history));
        }
        let (changed, cleared) = diff(&stored.config, &config);
        let updated_at: DateTime<Utc> = sqlx::query_scalar("UPDATE site_settings SET revision=revision+1,site_name=$1,logo_asset_id=$2,title_subtitle=$3,login_intro=$4,icp_number=$5,police_number=$6,police_url=$7,copyright_owner=$8,copyright_start_year=$9,copyright_end_mode=$10,copyright_end_year=$11,copyright_notice=$12,updated_by=$13,updated_at=now() WHERE singleton RETURNING updated_at")
            .bind(&config.site_name)
            .bind(config.logo_asset_id)
            .bind(&config.title_subtitle)
            .bind(&config.login_intro)
            .bind(&config.icp_number)
            .bind(&config.police_number)
            .bind(&config.police_url)
            .bind(&config.copyright_owner)
            .bind(config.copyright_start_year)
            .bind(config.copyright_end_mode.map(|mode| match mode {
                crate::site::domain::CopyrightEndMode::Auto => "auto",
                crate::site::domain::CopyrightEndMode::Fixed => "fixed",
            }))
            .bind(config.copyright_end_year)
            .bind(&config.copyright_notice)
            .bind(a.id)
            .fetch_one(&mut *tx)
            .await?;
        crate::audit::record(
            &mut tx,
            a,
            Uuid::nil(),
            "site.settings.update",
            stored.id,
            json!({
                "previous": &stored.config,
                "current": &config,
                "changed": changed,
                "cleared": cleared,
                "revision": stored.revision + 1
            }),
        )
        .await?;
        let history = history(&mut tx, stored.id).await?;
        tx.commit().await?;
        let updated_by_name = if a.display_name.trim().is_empty() {
            a.username.clone()
        } else {
            a.display_name.clone()
        };
        Ok(SiteConfig::admin(
            &config,
            stored.revision + 1,
            current_year,
            updated_at,
            Some(updated_by_name),
            history,
        ))
    }
}

impl Store for Repository {
    async fn public(&self) -> Result<PublicSiteSettings> {
        Repository::public(self).await
    }

    async fn logo(&self) -> Result<(String, Vec<u8>)> {
        Repository::logo(self).await
    }

    async fn admin(&self, a: &Actor) -> Result<AdminSiteSettings> {
        Repository::admin(self, a).await
    }

    async fn update(&self, a: &Actor, input: SiteSettingsInput) -> Result<AdminSiteSettings> {
        Repository::update(self, a, input).await
    }
}
