use crate::{
    business::{Actor, Error, Result, hash, require, text},
    miniapp_users::{
        domain::*,
        south::port::{ProfileStore, RegistrationStore},
    },
};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;
#[derive(Clone)]
pub struct Repository(pub PgPool);
const VIEW: &str = "SELECT u.account_id AS id,CASE WHEN u.avatar_asset_id IS NULL THEN NULL ELSE '/api/v1/miniapp/avatar-assets/'||u.avatar_asset_id END AS avatar_url,u.avatar_asset_id,u.nickname,u.mobile,a.bound_school_id AS school_id,s.name AS school_name,(SELECT count(*) FROM posts p WHERE p.author_id=u.account_id AND p.school_id=a.bound_school_id AND p.visibility<>'deleted') AS post_count,u.created_at,u.status,u.revision FROM miniapp_users u JOIN accounts a ON a.id=u.account_id LEFT JOIN schools s ON s.id=a.bound_school_id";
fn decode(v: Value) -> Result<UserView> {
    serde_json::from_value(v).map_err(|_| Error::Unavailable)
}
pub fn authorize(a: &Actor, school: Option<Uuid>, write: bool) -> Result<()> {
    if !a.browser {
        return Err(Error::Forbidden);
    }
    if let Some(s) = school {
        if !a.can(s, if write { "admin" } else { "operator" }) {
            return Err(Error::NotFound);
        }
    } else {
        a.platform()?;
    }
    Ok(())
}
pub async fn lock_user(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(Option<Uuid>, String, i64)> {
    let school: Option<Uuid> =
        sqlx::query_scalar("SELECT bound_school_id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(Error::NotFound)?;
    let row =
        sqlx::query("SELECT status,revision FROM miniapp_users WHERE account_id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(Error::NotFound)?;
    Ok((school, row.get("status"), row.get("revision")))
}
impl RegistrationStore for Repository {
    async fn register(&self, app: &str, openid: &str) -> Result<Uuid> {
        let mut tx = self.0.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73109))")
            .bind(json!([app, openid]).to_string())
            .execute(&mut *tx)
            .await?;
        if let Some(id) = sqlx::query_scalar::<_, Uuid>(
            "SELECT account_id FROM miniapp_identities WHERE app_id=$1 AND openid=$2",
        )
        .bind(app)
        .bind(openid)
        .fetch_optional(&mut *tx)
        .await?
        {
            tx.commit().await?;
            return Ok(id);
        }
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,username,display_name) VALUES($1,$2,'微信用户')")
            .bind(id)
            .bind(format!(
                "miniapp_{}",
                hash(&json!([app, openid]).to_string())
            ))
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'微信用户')")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO miniapp_identities(app_id,openid,account_id) VALUES($1,$2,$3)")
            .bind(app)
            .bind(openid)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }
}
impl ProfileStore for Repository {
    async fn own(&self, id: Uuid) -> Result<UserView> {
        let v = sqlx::query_scalar(&format!(
            "SELECT to_jsonb(v) FROM ({VIEW} WHERE u.account_id=$1) v"
        ))
        .bind(id)
        .fetch_optional(&self.0)
        .await?
        .ok_or(Error::NotFound)?;
        decode(v)
    }
}
impl Repository {
    pub async fn list(&self, a: &Actor, school: Option<Uuid>, q: &Query) -> Result<Vec<UserView>> {
        authorize(a, school, false)?;
        let (size, offset) = q.bounds()?;
        let order = q.order()?;
        if school.is_some() && (q.school_id.is_some() || q.unbound.is_some()) {
            return Err(Error::Invalid("学校范围不可覆盖".into()));
        }
        let scope = school.or(q.school_id);
        let sql = format!(
            "SELECT to_jsonb(v) FROM ({VIEW} WHERE ($1::uuid IS NULL OR a.bound_school_id=$1) AND (NOT $2 OR a.bound_school_id IS NULL) AND ($3::text IS NULL OR u.status=$3) AND ($4::text IS NULL OR u.nickname ILIKE '%'||$4||'%') AND ($5::text IS NULL OR u.mobile LIKE '%'||$5||'%') AND ($6::timestamptz IS NULL OR u.created_at>=$6) AND ($7::timestamptz IS NULL OR u.created_at<=$7) ORDER BY {order} LIMIT $8 OFFSET $9) v"
        );
        let rows: Vec<Value> = sqlx::query_scalar(&sql)
            .bind(scope)
            .bind(q.unbound.unwrap_or(false))
            .bind(&q.status)
            .bind(&q.search)
            .bind(&q.mobile)
            .bind(q.created_from)
            .bind(q.created_to)
            .bind(size + 1)
            .bind(offset)
            .fetch_all(&self.0)
            .await?;
        rows.into_iter().map(decode).collect()
    }
    pub async fn detail(&self, a: &Actor, school: Option<Uuid>, id: Uuid) -> Result<UserView> {
        authorize(a, school, false)?;
        let mut v = self.own(id).await?;
        if school.is_some() && v.school_id != school {
            return Err(Error::NotFound);
        }
        let records:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('created_at',created_at,'actor_id',actor_id,'action',action,'reason',detail->>'reason','from',detail->>'from','to',detail->>'to') FROM audit_events WHERE object_id=$1 AND action='miniapp_user.status' AND ($2 OR school_id=$3) ORDER BY created_at DESC,id DESC LIMIT 50").bind(id).bind(a.platform_admin).bind(school).fetch_all(&self.0).await?;
        v.history = Some(records);
        Ok(v)
    }
    pub async fn status(
        &self,
        a: &Actor,
        school: Option<Uuid>,
        id: Uuid,
        v: StatusInput,
    ) -> Result<()> {
        authorize(a, school, true)?;
        require(
            matches!(v.status.as_str(), "normal" | "banned"),
            "用户状态无效",
        )?;
        let reason = reason(&v.reason)?;
        let mut tx = self.0.begin().await?;
        let (bound, status, revision) = lock_user(&mut tx, id).await?;
        if school.is_some() && bound != school {
            return Err(Error::NotFound);
        }
        if v.revision != revision {
            return Err(Error::Conflict);
        }
        if status == v.status {
            tx.commit().await?;
            return Ok(());
        }
        sqlx::query("UPDATE miniapp_users SET status=$2,revision=revision+1,updated_at=now(),status_reason=$3,status_changed_at=now(),status_changed_by=$4 WHERE account_id=$1").bind(id).bind(&v.status).bind(&reason).bind(a.id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM sessions WHERE account_id=$1 AND kind='miniapp'")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        audit(
            &mut tx,
            a,
            bound,
            id,
            "miniapp_user.status",
            json!({"from":status,"to":v.status,"reason":reason}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn profile(&self, a: &Actor, v: ProfileInput) -> Result<UserView> {
        require(!a.browser, "请使用小程序身份")?;
        let nickname = text(&v.nickname, 1, 100)?;
        let mut tx = self.0.begin().await?;
        let (school, status, revision) = lock_user(&mut tx, a.id).await?;
        if status != "normal" {
            return Err(Error::MiniappBanned);
        }
        guard_session(&mut tx, a).await?;
        if revision != v.revision {
            return Err(Error::Conflict);
        }
        if let Some(asset) = v.avatar_asset_id {
            let exists = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM miniapp_avatar_assets WHERE id=$1 AND owner_id=$2 FOR KEY SHARE",
            )
            .bind(asset)
            .bind(a.id)
            .fetch_optional(&mut *tx)
            .await?
            .is_some();
            require(exists, "头像不属于当前用户")?;
        }
        sqlx::query("UPDATE miniapp_users SET nickname=$2,avatar_asset_id=$3,revision=revision+1,updated_at=now() WHERE account_id=$1").bind(a.id).bind(nickname).bind(v.avatar_asset_id).execute(&mut *tx).await?;
        audit(&mut tx, a, school, a.id, "miniapp_user.profile", json!({})).await?;
        tx.commit().await?;
        self.own(a.id).await
    }
    pub async fn mobile(
        &self,
        a: &Actor,
        id: Uuid,
        v: MobileInput,
        admin: bool,
    ) -> Result<UserView> {
        if admin {
            a.platform()?;
        } else {
            require(!a.browser && a.id == id, "请使用本人小程序身份")?;
            require(v.reason.is_none(), "登记不接受管理字段")?;
        }
        let value = mobile(&v.mobile)?;
        let reason = if admin {
            Some(reason(v.reason.as_deref().unwrap_or(""))?)
        } else {
            None
        };
        let mut tx = self.0.begin().await?;
        let (school, status, revision) = lock_user(&mut tx, id).await?;
        if !admin && status != "normal" {
            return Err(Error::MiniappBanned);
        }
        if !admin {
            guard_session(&mut tx, a).await?;
        }
        let old: Option<String> =
            sqlx::query_scalar("SELECT mobile FROM miniapp_users WHERE account_id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if old.as_deref() == Some(value.as_str())
            && (revision == v.revision || (!admin && revision == v.revision + 1))
        {
            tx.commit().await?;
            return self.own(id).await;
        }
        if revision != v.revision || (!admin && old.is_some()) {
            return Err(Error::Conflict);
        }
        sqlx::query("UPDATE miniapp_users SET mobile=$2,mobile_source=$3,mobile_updated_at=now(),revision=revision+1,updated_at=now() WHERE account_id=$1").bind(id).bind(&value).bind(if admin{"admin_corrected"}else{"user_declared"}).execute(&mut *tx).await?;
        audit(
            &mut tx,
            a,
            school,
            id,
            "miniapp_user.mobile",
            json!({"from":masked(old.as_deref()),"to":masked(Some(&value)),"reason":reason}),
        )
        .await?;
        tx.commit().await?;
        self.own(id).await
    }
    pub async fn school(&self, a: &Actor, id: Uuid, v: SchoolInput) -> Result<()> {
        a.platform()?;
        let reason = reason(&v.reason)?;
        let mut tx = self.0.begin().await?;
        let (old, _, revision) = lock_user(&mut tx, id).await?;
        if revision != v.revision {
            return Err(Error::Conflict);
        }
        let active: bool = sqlx::query_scalar("SELECT active FROM schools WHERE id=$1 FOR SHARE")
            .bind(v.school_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        require(active, "学校已停用")?;
        let conflict:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM school_memberships WHERE account_id=$1 AND school_id=ANY($2) AND role IN ('admin','operator'))").bind(id).bind(old.into_iter().chain([v.school_id]).collect::<Vec<_>>()).fetch_one(&mut *tx).await?;
        require(!conflict, "存在学校管理角色，请先从学校权限入口处理")?;
        if old == Some(v.school_id) {
            tx.commit().await?;
            return Ok(());
        }
        if let Some(old) = old {
            sqlx::query("UPDATE school_memberships SET role='revoked' WHERE school_id=$1 AND account_id=$2 AND role='member'").bind(old).bind(id).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO school_memberships VALUES($1,$2,'member') ON CONFLICT(school_id,account_id) DO UPDATE SET role='member'").bind(v.school_id).bind(id).execute(&mut *tx).await?;
        sqlx::query("UPDATE accounts SET bound_school_id=$2 WHERE id=$1")
            .bind(id)
            .bind(v.school_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE miniapp_users SET revision=revision+1,updated_at=now() WHERE account_id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM sessions WHERE account_id=$1 AND kind='miniapp'")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        audit(
            &mut tx,
            a,
            old,
            id,
            "miniapp_user.school",
            json!({"from":old,"to":v.school_id,"reason":reason}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn upload(&self, a: &Actor, bytes: Vec<u8>) -> Result<Uuid> {
        require(!a.browser, "请使用小程序身份")?;
        let (mime, _, _) = crate::media::domain::validate(&bytes)?;
        let mut tx = self.0.begin().await?;
        let (_, status, _) = lock_user(&mut tx, a.id).await?;
        if status != "normal" {
            return Err(Error::MiniappBanned);
        }
        guard_session(&mut tx, a).await?;
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO miniapp_avatar_assets(id,owner_id,mime_type,content) VALUES($1,$2,$3,$4)",
        )
        .bind(id)
        .bind(a.id)
        .bind(mime)
        .bind(bytes)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }
    pub async fn avatar(&self, a: &Actor, id: Uuid) -> Result<(String, Vec<u8>)> {
        let row=sqlx::query("SELECT x.mime_type,x.content FROM miniapp_avatar_assets x JOIN accounts owner ON owner.id=x.owner_id WHERE x.id=$1 AND (x.owner_id=$2 OR ($3 AND EXISTS(SELECT 1 FROM miniapp_users u WHERE u.account_id=owner.id AND u.avatar_asset_id=x.id) AND ($4 OR owner.bound_school_id=ANY($5))))")
  .bind(id).bind(a.id).bind(a.browser).bind(a.platform_admin).bind(a.grants.iter().filter(|g|matches!(g.role.as_str(),"admin"|"operator")).map(|g|g.school_id).collect::<Vec<_>>()).fetch_optional(&self.0).await?.ok_or(Error::NotFound)?;
        Ok((row.get("mime_type"), row.get("content")))
    }
    pub async fn cleanup(&self) -> Result<()> {
        sqlx::query("DELETE FROM miniapp_avatar_assets WHERE id IN (SELECT x.id FROM miniapp_avatar_assets x WHERE x.created_at<now()-interval '1 day' AND NOT EXISTS(SELECT 1 FROM miniapp_users u WHERE u.avatar_asset_id=x.id) FOR UPDATE OF x SKIP LOCKED)").execute(&self.0).await?;
        Ok(())
    }
}
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    school: Option<Uuid>,
    id: Uuid,
    action: &str,
    detail: Value,
) -> Result<()> {
    sqlx::query("INSERT INTO audit_events(id,school_id,actor_id,action,object_id,detail,request_id) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(Uuid::now_v7()).bind(school).bind(a.id).bind(action).bind(id).bind(detail).bind(&a.request_id).execute(&mut **tx).await?;
    Ok(())
}

/// Serialize miniapp content writes with bans, session revocation and school corrections.
/// The account/profile locks precede post and school locks; preparation workers never lock these rows.
pub async fn guard_write(
    tx: &mut Transaction<'_, Postgres>,
    a: &Actor,
    school: Uuid,
) -> Result<()> {
    if a.browser {
        return crate::schools::south::adapter::postgres::profiles::lock_active_school(tx, school)
            .await;
    }
    let (_, status, _) = lock_user(tx, a.id).await?;
    if status != "normal" {
        return Err(Error::MiniappBanned);
    }
    crate::schools::south::adapter::postgres::profiles::lock_active_school(tx, school).await?;
    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sessions se JOIN accounts a ON a.id=se.account_id JOIN school_memberships m ON m.account_id=a.id JOIN schools s ON s.id=m.school_id WHERE se.token_hash=$1 AND se.kind='miniapp' AND a.id=$2 AND a.active AND se.expires_at>now() AND se.last_seen_at>now()-interval '30 minutes' AND m.school_id=$3 AND m.role<>'revoked' AND s.active)").bind(&a.session_hash).bind(a.id).bind(school).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(Error::Unauthorized);
    }
    Ok(())
}

/// Caller holds the account/profile locks, serializing this check with revocation.
pub async fn guard_session(tx: &mut Transaction<'_, Postgres>, a: &Actor) -> Result<()> {
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sessions se JOIN accounts a ON a.id=se.account_id WHERE se.token_hash=$1 AND se.kind='miniapp' AND a.id=$2 AND a.active AND se.expires_at>now() AND se.last_seen_at>now()-interval '30 minutes')")
        .bind(&a.session_hash).bind(a.id).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(Error::Unauthorized);
    }
    Ok(())
}
