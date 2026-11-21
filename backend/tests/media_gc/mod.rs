use super::*;
use linjian_api::{
    business::{Actor, SessionKind},
    media::{
        config::Config,
        south::adapter::{filesystem::Files, lifecycle::FileLock},
    },
};
use std::path::PathBuf;

struct Fixture {
    admin: PgPool,
    pool: PgPool,
    schema: String,
    root: PathBuf,
    app: bootstrap::Application,
    actor: Actor,
    school: Uuid,
}
impl Fixture {
    async fn new() -> Self {
        let admin = PgPool::connect(
            &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
        )
        .await
        .unwrap();
        let schema = format!("forum_gc_{}", Uuid::new_v4().simple());
        let pool = isolated(&admin, &schema).await;
        database::migrate(&pool).await.unwrap();
        let root = std::env::temp_dir().join(&schema);
        let files = Files::open(Config::local(root.clone())).unwrap();
        let options = bootstrap::Options {
            files,
            ..bootstrap::Options::local()
        };
        let app = bootstrap::build_application(pool.clone(), options);
        let account = app
            .identity
            .bootstrap("root", "清理测试", "test-password-at-least-12", true)
            .await
            .unwrap();
        let (_, actor) = app
            .identity
            .0
            .session(
                account,
                SessionKind::AdminBrowser,
                Some("http://localhost:5173"),
            )
            .await
            .unwrap();
        let school = app.schools.create(&actor, "清理测试校").await.unwrap()["id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        Self {
            admin,
            pool,
            schema,
            root,
            app,
            actor,
            school,
        }
    }
    async fn upload(&self) -> (Uuid, String) {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let value = self
            .app
            .media
            .put(
                &self.actor,
                self.school,
                "post_image",
                png.into_inner(),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        (
            value["id"].as_str().unwrap().parse().unwrap(),
            value["url"].as_str().unwrap().into(),
        )
    }
    async fn post(&self, url: &str) -> Value {
        self.app.community.post(&self.actor,self.school,None,serde_json::from_value(json!({"title":"清理业务测试","content":"图片引用与删除","images":[url],"tags":["校园"]})).unwrap()).await.unwrap()
    }
    async fn delete(&self, post: &Value) {
        self.app
            .community
            .management_delete(
                &self.actor,
                self.school,
                post["id"].as_str().unwrap().parse().unwrap(),
                post["revision"].as_i64().unwrap(),
            )
            .await
            .unwrap();
    }
    async fn refs(&self, id: Uuid) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM file_asset_references WHERE asset_id=$1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn due(&self, id: Uuid) {
        sqlx::query("UPDATE file_assets SET gc_after=now()-interval '1 second',retry_at=now()-interval '1 second' WHERE id=$1").bind(id).execute(&self.pool).await.unwrap();
    }
    async fn state(&self, id: Uuid) -> String {
        sqlx::query_scalar("SELECT state FROM file_assets WHERE id=$1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
    async fn cleanup_until(&self, id: Uuid, state: &str) {
        // Other integration tests may briefly hold the database-wide snapshot lock.
        for _ in 0..100 {
            self.app.media.0.cleanup().await.unwrap();
            if self.state(id).await == state {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("asset {id} failed to reach {state}");
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .execute(&self.admin)
            .await
            .unwrap();
        self.admin.close().await;
        std::fs::remove_dir_all(self.root).unwrap();
    }
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn deletion_sharing_grace_and_no_preview_resurrection() {
    let f = Fixture::new().await;
    let (id, url) = f.upload().await;
    assert!(f.app.media.get(&f.actor, f.school, id).await.is_ok());
    let a = f.post(&url).await;
    let b = f.post(&url).await;
    assert_eq!(f.refs(id).await, 2);
    assert!(
        f.app
            .community
            .management_delete(
                &f.actor,
                f.school,
                a["id"].as_str().unwrap().parse().unwrap(),
                9999
            )
            .await
            .is_err()
    );
    assert_eq!(f.refs(id).await, 2);
    sqlx::raw_sql("CREATE FUNCTION reject_gc_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test audit failure'; END $$; CREATE TRIGGER reject_gc_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_gc_audit()")
        .execute(&f.pool).await.unwrap();
    assert!(
        f.app
            .community
            .management_delete(
                &f.actor,
                f.school,
                a["id"].as_str().unwrap().parse().unwrap(),
                a["revision"].as_i64().unwrap()
            )
            .await
            .is_err()
    );
    assert_eq!(
        f.refs(id).await,
        2,
        "audit failure must roll back reference removal"
    );
    sqlx::query("DROP TRIGGER reject_gc_audit ON audit_events")
        .execute(&f.pool)
        .await
        .unwrap();
    f.delete(&a).await;
    assert_eq!(f.refs(id).await, 1);
    f.app.media.0.cleanup().await.unwrap();
    assert!(f.root.join(format!("{id}.blob")).is_file());
    f.delete(&b).await;
    assert_eq!(f.refs(id).await, 0);
    let grace: bool = sqlx::query_scalar(
        "SELECT ever_bound AND gc_after>now()+interval '23 hours' FROM file_assets WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert!(grace);
    assert!(f.app.media.get(&f.actor, f.school, id).await.is_err());
    assert!(
        f.app
            .media
            .0
            .lease(&f.actor, f.school, Uuid::new_v4(), vec![id])
            .await
            .is_err()
    );
    assert!(
        f.app
            .community
            .post(
                &f.actor,
                f.school,
                None,
                serde_json::from_value(
                    json!({"title":"不可复活","content":"旧图片","images":[url],"tags":[]})
                )
                .unwrap()
            )
            .await
            .is_err()
    );
    let before: Value =
        sqlx::query_scalar("SELECT to_jsonb(gc_after) FROM file_assets WHERE id=$1")
            .bind(id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    sqlx::query("UPDATE posts SET browse=browse+1 WHERE id=$1")
        .bind(a["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .execute(&f.pool)
        .await
        .unwrap();
    f.app.media.0.cleanup().await.unwrap();
    let after: Value = sqlx::query_scalar("SELECT to_jsonb(gc_after) FROM file_assets WHERE id=$1")
        .bind(id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(f.refs(id).await, 0);
    f.due(id).await;
    f.cleanup_until(id, "deleted").await;
    assert!(!f.root.join(format!("{id}.blob")).exists());
    let (status_id, status_url) = f.upload().await;
    let post = f.post(&status_url).await;
    let pid = post["id"].as_str().unwrap().parse().unwrap();
    let hidden = f
        .app
        .community
        .visibility(
            &f.actor,
            f.school,
            pid,
            json!({"revision":post["revision"],"visibility":"hidden","top_sort":0}),
        )
        .await
        .unwrap();
    assert_eq!(f.refs(status_id).await, 1);
    f.app
        .community
        .visibility(
            &f.actor,
            f.school,
            pid,
            json!({"revision":hidden["revision"],"visibility":"deleted","top_sort":0}),
        )
        .await
        .unwrap();
    assert_eq!(f.refs(status_id).await, 0);
    f.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn draft_expiry_and_immutable_history_protect_files() {
    let f = Fixture::new().await;
    let (id, url) = f.upload().await;
    let post = f.post(&url).await;
    let draft = Uuid::new_v4();
    f.app
        .media
        .0
        .lease(&f.actor, f.school, draft, vec![id])
        .await
        .unwrap();
    f.delete(&post).await;
    assert!(f.app.media.get(&f.actor, f.school, id).await.is_ok());
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT gc_after IS NULL FROM file_assets WHERE id=$1")
            .bind(id)
            .fetch_one(&f.pool)
            .await
            .unwrap()
    );
    let reuse = f.post(&url).await;
    f.delete(&reuse).await;
    sqlx::query(
        "UPDATE file_asset_leases SET expires_at=now()-interval '1 second' WHERE asset_id=$1",
    )
    .bind(id)
    .execute(&f.pool)
    .await
    .unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT gc_after>now()+interval '23 hours' FROM file_assets WHERE id=$1"
        )
        .bind(id)
        .fetch_one(&f.pool)
        .await
        .unwrap()
    );
    assert!(f.app.media.get(&f.actor, f.school, id).await.is_err());
    f.due(id).await;
    f.cleanup_until(id, "deleted").await;

    let (historic, url) = f.upload().await;
    let post = f.post(&url).await;
    let approved = approve_post(
        &f.pool,
        &f.actor,
        post["id"].as_str().unwrap().parse().unwrap(),
    )
    .await;
    let signed = f
        .app
        .legacy
        .signed_media(f.school, &url, "http://localhost:8091")
        .await
        .unwrap();
    let token = signed.rsplit('/').next().unwrap();
    assert!(
        f.app
            .legacy
            .media(token, f.app.media.0.1.clone())
            .await
            .is_ok()
    );
    f.delete(&approved).await;
    assert!(
        f.app
            .legacy
            .media(token, f.app.media.0.1.clone())
            .await
            .is_err()
    );
    assert!(
        f.app
            .legacy
            .signed_media(f.school, &url, "http://localhost:8091")
            .await
            .is_err()
    );
    assert!(sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM media_access_tokens WHERE asset_id=$1 AND expires_at>now())").bind(historic).fetch_one(&f.pool).await.unwrap(),"token still protects storage even though reading is denied");
    assert_eq!(
        f.refs(historic).await,
        1,
        "immutable snapshot must keep its reference"
    );
    f.due(historic).await;
    f.app.media.0.cleanup().await.unwrap();
    assert_eq!(f.state(historic).await, "ready");
    assert!(f.root.join(format!("{historic}.blob")).is_file());
    assert!(
        f.app.media.get(&f.actor, f.school, historic).await.is_err(),
        "snapshot protection grants no read access"
    );
    let snapshot = migration::export(&f.pool, &f.app.media.0.1).await.unwrap();
    assert_eq!(snapshot.schema_version, 6);
    let target_name = format!("forum_gc_restore_{}", Uuid::new_v4().simple());
    let target = isolated(&f.admin, &target_name).await;
    database::migrate(&target).await.unwrap();
    let target_root = f.root.join("restored");
    let target_files = Files::open(Config::local(target_root)).unwrap();
    migration::import(&target, &snapshot, false, &target_files)
        .await
        .unwrap();
    let again = migration::export(&target, &target_files).await.unwrap();
    assert_eq!(snapshot.digest, again.digest);
    assert_eq!(snapshot.files, again.files);
    target.close().await;
    sqlx::query(&format!("DROP SCHEMA {target_name} CASCADE"))
        .execute(&f.admin)
        .await
        .unwrap();
    f.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn durable_retry_missing_file_and_backup_coordination() {
    let f = Fixture::new().await;
    let (id, _) = f.upload().await;
    f.due(id).await;
    let shared = FileLock::shared(&f.pool).await.unwrap();
    f.app.media.0.cleanup().await.unwrap();
    assert_eq!(f.state(id).await, "ready");
    shared.release().await.unwrap();
    let blob = f.root.join(format!("{id}.blob"));
    std::fs::remove_file(&blob).unwrap();
    std::fs::create_dir(&blob).unwrap(); // unlink fails: task must remain durable.
    f.cleanup_until(id, "pending_delete").await;
    assert!(
        f.app
            .media
            .0
            .lease(&f.actor, f.school, Uuid::new_v4(), vec![id])
            .await
            .is_err()
    );
    std::fs::remove_dir(&blob).unwrap();
    f.due(id).await;
    f.cleanup_until(id, "deleted").await; // absent file is idempotent success.
    let (id, _) = f.upload().await;
    f.due(id).await;
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM file_assets WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    f.app.media.0.cleanup().await.unwrap(); // SKIP LOCKED cannot claim it.
    assert_eq!(f.state(id).await, "ready");
    sqlx::query("INSERT INTO file_asset_leases VALUES($1,$2,$3,now()+interval '1 day')")
        .bind(id)
        .bind(f.actor.id)
        .bind(Uuid::new_v4())
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    f.app.media.0.cleanup().await.unwrap();
    assert_eq!(f.state(id).await, "ready");
    assert!(blob.parent().unwrap().join(format!("{id}.blob")).is_file());
    // A cancelled/failed exporter closes its detached connection, releasing its lock.
    drop(FileLock::shared(&f.pool).await.unwrap());
    let mut acquired = false;
    for _ in 0..100 {
        if let Some(lock) = FileLock::try_exclusive(&f.pool).await.unwrap() {
            lock.release().await.unwrap();
            acquired = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(acquired);
    f.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn lease_waiting_on_last_release_cannot_revive_asset() {
    let f = Fixture::new().await;
    let (id, url) = f.upload().await;
    let post = f.post(&url).await;
    let mut tx = f.pool.begin().await.unwrap();
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM file_assets WHERE id=$1 FOR UPDATE")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let media = f.app.media.clone();
    let actor = f.actor.clone();
    let school = f.school;
    let lease = tokio::spawn(async move {
        media
            .0
            .lease(&actor, school, Uuid::new_v4(), vec![id])
            .await
    });
    let mut waiting = false;
    for _ in 0..100 {
        waiting = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
        )
        .bind(blocker)
        .fetch_one(&f.pool)
        .await
        .unwrap();
        if waiting {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(waiting, "renewal must reach the asset lock before release");
    sqlx::query("UPDATE posts SET visibility='deleted' WHERE id=$1")
        .bind(post["id"].as_str().unwrap().parse::<Uuid>().unwrap())
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(
        lease.await.unwrap().is_err(),
        "waiting renewal must recheck the committed release"
    );
    assert_eq!(f.refs(id).await, 0);
    assert_eq!(f.state(id).await, "ready");
    assert!(f.app.media.get(&f.actor, f.school, id).await.is_err());
    // If cleanup claims first, waiting renewals also fail after seeing pending_delete.
    let (pending, _) = f.upload().await;
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("UPDATE file_assets SET state='pending_delete' WHERE id=$1")
        .bind(pending)
        .execute(&mut *tx)
        .await
        .unwrap();
    let media = f.app.media.clone();
    let actor = f.actor.clone();
    let lease = tokio::spawn(async move {
        media
            .0
            .lease(&actor, school, Uuid::new_v4(), vec![pending])
            .await
    });
    tx.commit().await.unwrap();
    assert!(lease.await.unwrap().is_err());
    f.cleanup_until(pending, "deleted").await;
    f.close().await;
}
