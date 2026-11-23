use super::*;
use linjian_api::{
    business::{Actor, Error, SessionKind},
    content_publication::{
        domain::{Failure, Link, Preparation},
        north::local::Worker,
        south::port::MiniappLinkProvider,
    },
    message_advertisements::{
        domain::{Command, Filters},
        south::adapter::postgres::candidate,
    },
};
use serde::de::DeserializeOwned;

fn input<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}
fn id(value: &Value) -> Uuid {
    value["id"].as_str().unwrap().parse().unwrap()
}
fn revision_action(config: &Value) -> Value {
    json!({"expected_config_version":config["version"],"expected_revision_version":config["candidate"]["version"]})
}

struct Fixture {
    admin: PgPool,
    pool: PgPool,
    schema: String,
    app: bootstrap::Application,
    root: Actor,
    manager: Actor,
    operator: Actor,
    manager_token: String,
    root_token: String,
    school: Uuid,
    other: Uuid,
}
impl Fixture {
    async fn new() -> Self {
        let admin = PgPool::connect(
            &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated URL required"),
        )
        .await
        .unwrap();
        let schema = format!("message_advertisements_{}", Uuid::new_v4().simple());
        let pool = isolated(&admin, &schema).await;
        database::migrate(&pool).await.unwrap();
        let app = bootstrap::build_application(pool.clone(), bootstrap::Options::local());
        let root_id = app
            .identity
            .bootstrap("ads-root", "平台管理", "test-password-at-least-12", true)
            .await
            .unwrap();
        let (root_token, root) = app
            .identity
            .0
            .session(
                root_id,
                SessionKind::AdminBrowser,
                Some("http://localhost:5173"),
            )
            .await
            .unwrap();
        let school = id(&app.schools.create(&root, "广告甲校").await.unwrap());
        let other = id(&app.schools.create(&root, "广告乙校").await.unwrap());
        let manager_id = app
            .identity
            .bootstrap(
                "ads-manager",
                "学校管理",
                "test-password-at-least-12",
                false,
            )
            .await
            .unwrap();
        let operator_id = app
            .identity
            .bootstrap(
                "ads-operator",
                "学校运营",
                "test-password-at-least-12",
                false,
            )
            .await
            .unwrap();
        app.schools
            .grant(&root, school, manager_id, "admin")
            .await
            .unwrap();
        app.schools
            .grant(&root, school, operator_id, "operator")
            .await
            .unwrap();
        let (manager_token, manager) = app
            .identity
            .0
            .session(
                manager_id,
                SessionKind::AdminBrowser,
                Some("http://localhost:5173"),
            )
            .await
            .unwrap();
        let (_, operator) = app
            .identity
            .0
            .session(
                operator_id,
                SessionKind::AdminBrowser,
                Some("http://localhost:5173"),
            )
            .await
            .unwrap();
        Self {
            admin,
            pool,
            schema,
            app,
            root,
            manager,
            operator,
            manager_token,
            root_token,
            school,
            other,
        }
    }
    async fn create(&self, content: &str) -> Value {
        self.app
            .message_advertisements
            .write(
                &self.manager,
                Some(self.school),
                None,
                None,
                Command::Create(input(json!({"content":content}))),
            )
            .await
            .unwrap()
    }
    async fn submit(&self, config: &Value) -> Value {
        self.app
            .message_advertisements
            .write(
                &self.manager,
                Some(self.school),
                Some(id(config)),
                Some(id(&config["candidate"])),
                Command::Submit(input(revision_action(config))),
            )
            .await
            .unwrap()
    }
    fn decision(&self, config: &Value, approve: bool) -> Command {
        Command::Decide(input(
            json!({"expected_config_version":config["version"],"expected_revision_version":config["candidate"]["version"],"review_digest":config["candidate"]["review_digest"],"decision":if approve {"approve"} else {"reject"},"reason":if approve {""} else {"请补充广告标识"}}),
        ))
    }
    async fn approve(&self, config: &Value) -> Value {
        self.app
            .message_advertisements
            .write(
                &self.root,
                None,
                None,
                Some(id(&config["candidate"])),
                self.decision(config, true),
            )
            .await
            .unwrap()
    }
    async fn edit(&self, config: &Value, content: &str) -> Value {
        self.app.message_advertisements.write(&self.manager, Some(self.school), Some(id(config)), None, Command::Draft(input(json!({"content":content,"expected_config_version":config["version"],"expected_revision_version":config["candidate"]["version"]})))).await.unwrap()
    }
    async fn delete(&self, config: &Value) -> Value {
        self.app
            .message_advertisements
            .write(
                &self.manager,
                Some(self.school),
                Some(id(config)),
                None,
                Command::Delete(input(json!({"expected_config_version":config["version"]}))),
            )
            .await
            .unwrap()
    }
    async fn selected(&self) -> Option<String> {
        let mut tx = self.pool.begin().await.unwrap();
        candidate(&mut tx, self.school)
            .await
            .unwrap()
            .map(|value| value.content)
    }
    async fn prepare_route(&self) {
        let connection = Uuid::new_v4();
        let source = Uuid::new_v4();
        sqlx::query("INSERT INTO source_connections(id,name,source_id,origin,key_id,checked_at) VALUES($1,'test',$2,'https://keeper.example.com','test-key',now())").bind(connection).bind(source).execute(&self.pool).await.unwrap();
        sqlx::query("INSERT INTO source_connection_versions(id,connection_id,revision,source_id,origin) VALUES($1,$2,1,$3,'https://keeper.example.com')").bind(Uuid::new_v4()).bind(connection).bind(source).execute(&self.pool).await.unwrap();
        sqlx::query("UPDATE source_active_configuration SET connection_id=$1")
            .bind(connection)
            .execute(&self.pool)
            .await
            .unwrap();
        linjian_api::source_access::south::adapter::postgres::Repository(self.pool.clone())
            .bind(
                &self.root,
                self.school,
                connection,
                linjian_api::source_access::north::local::BindingInput {
                    route_id: "ads-route".into(),
                    active: true,
                },
                0,
                1,
            )
            .await
            .unwrap();
    }
    async fn post(&self) -> Uuid {
        let post = self
            .app
            .community
            .management_write(
                &self.root,
                self.school,
                None,
                input(json!({"title":"","content":"原有消息正文","images":[],"tags":[]})),
            )
            .await
            .unwrap();
        approve_post(&self.pool, &self.root, post.post.id).await;
        sqlx::query_scalar(
            "SELECT id FROM forum_outbox WHERE post_id=$1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(post.post.id)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .execute(&self.admin)
            .await
            .unwrap();
        self.admin.close().await;
    }
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn message_advertisements_workflow_scope_permissions_and_evidence() {
    let fixture = Fixture::new().await;
    assert!(
        fixture
            .app
            .message_advertisements
            .current(&fixture.manager, fixture.school)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .app
            .message_advertisements
            .current(&fixture.manager, fixture.other)
            .await
            .is_err()
    );
    assert!(
        fixture
            .app
            .message_advertisements
            .current(&fixture.operator, fixture.school)
            .await
            .is_err()
    );
    let draft = fixture.create("  【广告】打印服务😀  ").await;
    assert_eq!(draft["candidate"]["content"], "【广告】打印服务😀");
    assert!(fixture.selected().await.is_none());
    assert!(
        fixture
            .app
            .message_advertisements
            .write(
                &fixture.root,
                Some(fixture.school),
                None,
                None,
                Command::Create(input(json!({"content":"平台代写"})))
            )
            .await
            .is_err()
    );
    assert!(
        fixture
            .app
            .message_advertisements
            .write(
                &fixture.manager,
                Some(fixture.other),
                Some(id(&draft)),
                None,
                Command::Delete(input(json!({"expected_config_version":draft["version"]})))
            )
            .await
            .is_err()
    );
    let pending = fixture.submit(&draft).await;
    assert!(
        fixture
            .app
            .message_advertisements
            .write(
                &fixture.manager,
                None,
                None,
                Some(id(&pending["candidate"])),
                fixture.decision(&pending, true)
            )
            .await
            .is_err()
    );
    assert!(fixture.selected().await.is_none());
    let approved = fixture.approve(&pending).await;
    assert_eq!(
        fixture.selected().await.as_deref(),
        Some("【广告】打印服务😀")
    );
    let revision = id(&approved["current_revision"]);
    assert!(
        sqlx::query("UPDATE school_message_advertisement_revisions SET content='篡改' WHERE id=$1")
            .bind(revision)
            .execute(&fixture.pool)
            .await
            .is_err()
    );
    let draft = fixture.edit(&approved, "【广告】新服务").await;
    let pending = fixture.submit(&draft).await;
    assert_eq!(
        fixture.selected().await.as_deref(),
        Some("【广告】打印服务😀")
    );
    let rejected = fixture
        .app
        .message_advertisements
        .write(
            &fixture.root,
            None,
            None,
            Some(id(&pending["candidate"])),
            fixture.decision(&pending, false),
        )
        .await
        .unwrap();
    assert_eq!(rejected["current_revision"]["id"], revision.to_string());
    assert!(rejected["candidate"].is_null());
    let history = fixture
        .app
        .message_advertisements
        .history(
            &fixture.manager,
            fixture.school,
            id(&rejected),
            &Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(history[0]["status"], "rejected");
    assert_eq!(history[0]["decision"]["reason"], "请补充广告标识");
    let audit: Value = sqlx::query_scalar("SELECT detail FROM audit_events WHERE object_id=$1 AND action='message_advertisement.reject'")
        .bind(id(&rejected)).fetch_one(&fixture.pool).await.unwrap();
    assert_eq!(audit["before_config_version"], pending["version"]);
    assert_eq!(audit["after_config_version"], rejected["version"]);
    assert_eq!(
        audit["before_revision"]["version"],
        pending["candidate"]["version"]
    );
    assert_eq!(audit["after_revision"]["version"], history[0]["version"]);
    assert_eq!(audit["before_revision"]["status"], "pending");
    assert_eq!(audit["after_revision"]["status"], "rejected");
    assert_eq!(audit["decision"], "reject");
    assert_eq!(audit["reason"], "请补充广告标识");
    assert!(audit.get("content").is_none());
    let draft = fixture.edit(&rejected, "【广告】再提交").await;
    let pending = fixture.submit(&draft).await;
    let deleted = fixture.delete(&pending).await;
    assert!(deleted["current_revision"].is_null());
    assert!(fixture.selected().await.is_none());
    assert!(matches!(
        fixture
            .app
            .message_advertisements
            .write(
                &fixture.root,
                None,
                None,
                Some(id(&pending["candidate"])),
                fixture.decision(&pending, true)
            )
            .await,
        Err(Error::Conflict)
    ));
    let replacement = fixture.create("【广告】新配置").await;
    assert_ne!(replacement["id"], deleted["id"]);
    assert!(fixture.selected().await.is_none());
    assert!(
        sqlx::query("UPDATE school_message_advertisements SET current_revision_id=$2 WHERE id=$1")
            .bind(id(&replacement))
            .bind(revision)
            .execute(&fixture.pool)
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "advertisement actions never publish posts");
    sqlx::query(
        "UPDATE school_memberships SET role='revoked' WHERE school_id=$1 AND account_id=$2",
    )
    .bind(fixture.school)
    .bind(fixture.manager.id)
    .execute(&fixture.pool)
    .await
    .unwrap();
    assert!(
        fixture
            .app
            .message_advertisements
            .write(
                &fixture.manager,
                Some(fixture.school),
                Some(id(&replacement)),
                None,
                Command::Delete(input(
                    json!({"expected_config_version":replacement["version"]})
                ))
            )
            .await
            .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn message_advertisements_http_csrf_strict_inputs_and_platform_queue() {
    let fixture = Fixture::new().await;
    let router = linjian_api::http::app_router(fixture.app.clone());
    let path = format!("/api/v1/schools/{}/message-advertisements", fixture.school);
    for (origin, csrf, attributes) in [
        (
            "https://other.example",
            fixture.manager.csrf.as_str(),
            json!({"content":"广告"}),
        ),
        ("http://localhost:5173", "wrong", json!({"content":"广告"})),
        (
            "http://localhost:5173",
            fixture.manager.csrf.as_str(),
            json!({"content":"广告","status":"approved"}),
        ),
    ] {
        let response = router.clone().oneshot(Request::builder().method("POST").uri(&path).header("Cookie", format!("linjian_session={}",fixture.manager_token)).header("Origin",origin).header("X-CSRF-Token",csrf).header("Content-Type","application/vnd.api+json").body(Body::from(json!({"data":{"type":"school-message-advertisements","attributes":attributes}}).to_string())).unwrap()).await.unwrap();
        assert!(!response.status().is_success());
    }
    let response = router.clone().oneshot(Request::builder().method("POST").uri(&path)
        .header("Cookie",format!("linjian_session={}",fixture.manager_token)).header("Origin","http://localhost:5173")
        .header("X-CSRF-Token",&fixture.manager.csrf).header("Content-Type","application/vnd.api+json")
        .body(Body::from(json!({"data":{"type":"school-message-advertisements","attributes":{"content":"【广告】本校服务"}}}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let document: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    let mut draft = document["data"]["attributes"].clone();
    draft["id"] = document["data"]["id"].clone();
    fixture.submit(&draft).await;
    let list = fixture
        .app
        .message_advertisements
        .list(&fixture.root, &Filters::default())
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    let response=router.clone().oneshot(Request::builder().uri(format!("/api/v1/platform/message-advertisement-revisions?status=pending&school_id={}&page%5Bnumber%5D=1",fixture.school))
        .header("Cookie",format!("linjian_session={}",fixture.root_token)).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let document: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(document["data"].as_array().unwrap().len(), 1);
    assert!(
        fixture
            .app
            .message_advertisements
            .list(&fixture.manager, &Filters::default())
            .await
            .is_err()
    );
    let response = router.clone().oneshot(Request::builder().uri("/api/v1/platform/message-advertisement-revisions?status=pending&page%5Bnumber%5D=1").header("Cookie", format!("linjian_session={}",fixture.manager_token)).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    fixture.close().await;
}

async fn wait_for_freeze_gate(pool: &PgPool, key: i64) {
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND classid=0 AND objid=$1::bigint::oid AND NOT granted)")
                .bind(key).fetch_one(pool).await.unwrap();
            if waiting { return; }
            tokio::task::yield_now().await;
        }
    }).await.expect("freeze worker must reach the advisory test gate");
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn message_advertisements_live_permission_and_platform_removal() {
    let fixture = Fixture::new().await;
    let draft = fixture.create("【广告】待审内容").await;
    sqlx::query("INSERT INTO school_member_permission_overrides(school_id,account_id,permission_key,effect,updated_by) VALUES($1,$2,'school.message_advertisement.manage','deny',$3)")
        .bind(fixture.school).bind(fixture.manager.id).bind(fixture.root.id).execute(&fixture.pool).await.unwrap();
    assert!(
        fixture
            .app
            .message_advertisements
            .write(
                &fixture.manager,
                Some(fixture.school),
                Some(id(&draft)),
                Some(id(&draft["candidate"])),
                Command::Submit(input(revision_action(&draft)))
            )
            .await
            .is_err()
    );
    assert!(
        fixture
            .app
            .message_advertisements
            .current(&fixture.manager, fixture.school)
            .await
            .is_ok()
    );
    assert!(
        fixture
            .app
            .message_advertisements
            .write(
                &fixture.root,
                None,
                Some(id(&draft)),
                None,
                Command::Remove(input(
                    json!({"expected_config_version":draft["version"],"reason":""})
                ))
            )
            .await
            .is_err()
    );
    sqlx::query("UPDATE schools SET active=false WHERE id=$1")
        .bind(fixture.school)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let removed = fixture
        .app
        .message_advertisements
        .write(
            &fixture.root,
            None,
            Some(id(&draft)),
            None,
            Command::Remove(input(
                json!({"expected_config_version":draft["version"],"reason":"平台下架"}),
            )),
        )
        .await
        .unwrap();
    assert!(removed["deleted_at"].is_string());
    assert_eq!(removed["deletion_reason"], "平台下架");
    assert!(removed["candidate"].is_null());
    let history = fixture
        .app
        .message_advertisements
        .history(
            &fixture.root,
            fixture.school,
            id(&draft),
            &Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(history[0]["status"], "withdrawn");
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn message_advertisements_freeze_serializes_approval_and_deletion() {
    let fixture = Fixture::new().await;
    fixture.prepare_route().await;
    let first = fixture
        .approve(
            &fixture
                .submit(&fixture.create("【广告】旧版本").await)
                .await,
        )
        .await;
    let pending = fixture
        .submit(&fixture.edit(&first, "【广告】新版本").await)
        .await;
    let key = (Uuid::new_v4().as_u128() % i32::MAX as u128) as i64;
    sqlx::raw_sql(&format!("CREATE FUNCTION test_freeze_advertisement_gate() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN PERFORM pg_advisory_xact_lock({key}::bigint); RETURN NEW; END$$; CREATE TRIGGER test_freeze_advertisement_gate BEFORE INSERT ON forum_message_advertisement_snapshots FOR EACH ROW EXECUTE FUNCTION test_freeze_advertisement_gate()"))
        .execute(&fixture.pool).await.unwrap();
    let event = fixture.post().await;
    let mut gate = fixture.pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(key)
        .execute(&mut *gate)
        .await
        .unwrap();
    let freeze = {
        let worker = Worker::new(fixture.pool.clone()).unwrap();
        tokio::spawn(async move { worker.run_with(&NoLink).await })
    };
    wait_for_freeze_gate(&fixture.pool, key).await;
    let review = {
        let service = fixture.app.message_advertisements.clone();
        let root = fixture.root.clone();
        let revision = id(&pending["candidate"]);
        let command = fixture.decision(&pending, true);
        tokio::spawn(async move {
            service
                .write(&root, None, None, Some(revision), command)
                .await
        })
    };
    gate.commit().await.unwrap();
    assert!(freeze.await.unwrap().unwrap());
    let approved = review.await.unwrap().unwrap();
    let content: String =
        sqlx::query_scalar("SELECT payload->>'content' FROM forum_outbox WHERE id=$1")
            .bind(event)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(content, "原有消息正文\n【广告】旧版本");
    assert_eq!(fixture.selected().await.as_deref(), Some("【广告】新版本"));

    let next = fixture.post().await;
    let mut gate = fixture.pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(key)
        .execute(&mut *gate)
        .await
        .unwrap();
    let freeze = {
        let worker = Worker::new(fixture.pool.clone()).unwrap();
        tokio::spawn(async move { worker.run_with(&NoLink).await })
    };
    wait_for_freeze_gate(&fixture.pool, key).await;
    let deletion = {
        let service = fixture.app.message_advertisements.clone();
        let manager = fixture.manager.clone();
        let school = fixture.school;
        tokio::spawn(async move {
            service
                .write(
                    &manager,
                    Some(school),
                    Some(id(&approved)),
                    None,
                    Command::Delete(input(
                        json!({"expected_config_version":approved["version"]}),
                    )),
                )
                .await
        })
    };
    gate.commit().await.unwrap();
    assert!(freeze.await.unwrap().unwrap());
    deletion.await.unwrap().unwrap();
    let content: String =
        sqlx::query_scalar("SELECT payload->>'content' FROM forum_outbox WHERE id=$1")
            .bind(next)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(content, "原有消息正文\n【广告】新版本");
    assert!(fixture.selected().await.is_none());
    fixture.close().await;
}

struct NoLink;
impl MiniappLinkProvider for NoLink {
    async fn generate(&self, _: &Preparation) -> Result<Link, Failure> {
        panic!("no additional link generation")
    }
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn message_advertisements_freeze_atomic_snapshot_and_retry_bytes() {
    use linjian_api::outbox::south::{
        adapter::postgres::Repository as Deliveries, port::OutboxStore,
    };
    let fixture = Fixture::new().await;
    fixture.prepare_route().await;
    let pending = fixture
        .submit(&fixture.create("【广告】打印服务😀").await)
        .await;
    let approved = fixture.approve(&pending).await;
    let event = fixture.post().await;
    let worker = Worker::new(fixture.pool.clone()).unwrap();
    assert!(worker.run_with(&NoLink).await.unwrap());
    let (body, hash): (String, String) =
        sqlx::query_as("SELECT request_body,body_sha256 FROM forum_outbox WHERE id=$1")
            .bind(event)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    let document: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        document["data"]["attributes"]["content"],
        "原有消息正文\n【广告】打印服务😀"
    );
    assert_eq!(document["data"]["attributes"].as_object().unwrap().len(), 6);
    assert_eq!(
        linjian_api::source_access::domain::digest(body.as_bytes()),
        hash
    );
    let snapshot: Value = sqlx::query_scalar(
        "SELECT to_jsonb(s) FROM forum_message_advertisement_snapshots s WHERE event_id=$1",
    )
    .bind(event)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(snapshot["revision_id"], approved["current_revision"]["id"]);
    assert_eq!(snapshot["result"], "applied");
    assert!(
        sqlx::query(
            "UPDATE forum_message_advertisement_snapshots SET content='篡改' WHERE event_id=$1"
        )
        .bind(event)
        .execute(&fixture.pool)
        .await
        .is_err()
    );
    let deliveries = Deliveries(fixture.pool.clone());
    let first = deliveries.claim().await.unwrap().unwrap();
    assert_eq!(first.body, body);
    fixture.delete(&approved).await;
    sqlx::query("UPDATE forum_outbox SET lease_until=now()-interval '1 second' WHERE id=$1")
        .bind(event)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let retry = deliveries.claim().await.unwrap().unwrap();
    assert_eq!(retry.id, event);
    assert_eq!(retry.body, body);
    assert_eq!(retry.body_sha256, hash);
    let next = fixture.post().await;
    assert!(worker.run_with(&NoLink).await.unwrap());
    let next_content: String =
        sqlx::query_scalar("SELECT payload->>'content' FROM forum_outbox WHERE id=$1")
            .bind(next)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(next_content, "原有消息正文");
    let next_result: String = sqlx::query_scalar(
        "SELECT result FROM forum_message_advertisement_snapshots WHERE event_id=$1",
    )
    .bind(next)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(next_result, "no_approved_advertisement");
    let failing = fixture.post().await;
    sqlx::raw_sql("CREATE FUNCTION test_advertisement_snapshot_failure() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'injected failure'; END$$; CREATE TRIGGER test_advertisement_snapshot_failure BEFORE INSERT ON forum_message_advertisement_snapshots FOR EACH ROW EXECUTE FUNCTION test_advertisement_snapshot_failure()")
        .execute(&fixture.pool).await.unwrap();
    assert!(worker.run_with(&NoLink).await.is_err());
    let unfrozen: bool =
        sqlx::query_scalar("SELECT request_body IS NULL FROM forum_outbox WHERE id=$1")
            .bind(failing)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert!(unfrozen);
    sqlx::query(
        "DROP TRIGGER test_advertisement_snapshot_failure ON forum_message_advertisement_snapshots",
    )
    .execute(&fixture.pool)
    .await
    .unwrap();
    let snapshot = migration::export(&fixture.pool, &fixture.app.media.0.1)
        .await
        .unwrap();
    assert_eq!(snapshot.schema_version, 7);
    let mut obsolete: migration::Snapshot =
        serde_json::from_value(serde_json::to_value(&snapshot).unwrap()).unwrap();
    obsolete.schema_version = 6;
    assert!(
        matches!(migration::import(&fixture.pool,&obsolete,false,&fixture.app.media.0.1).await,
        Err(Error::Invalid(message)) if message.contains("版本"))
    );
    let target_schema = format!("message_advertisements_restore_{}", Uuid::new_v4().simple());
    let target = isolated(&fixture.admin, &target_schema).await;
    database::migrate(&target).await.unwrap();
    migration::import(&target, &snapshot, false, &fixture.app.media.0.1)
        .await
        .unwrap();
    let restored = migration::export(&target, &fixture.app.media.0.1)
        .await
        .unwrap();
    assert_eq!(snapshot.digest, restored.digest);
    target.close().await;
    sqlx::query(&format!("DROP SCHEMA {target_schema} CASCADE"))
        .execute(&fixture.admin)
        .await
        .unwrap();
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn message_advertisements_concurrent_creation_and_delete_before_approval() {
    let fixture = Fixture::new().await;
    let service = &fixture.app.message_advertisements;
    let (first, second) = tokio::join!(
        service.write(
            &fixture.manager,
            Some(fixture.school),
            None,
            None,
            Command::Create(input(json!({"content":"广告甲"})))
        ),
        service.write(
            &fixture.manager,
            Some(fixture.school),
            None,
            None,
            Command::Create(input(json!({"content":"广告乙"})))
        )
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let pending = fixture.submit(&first.or(second).unwrap()).await;
    let mut blocker = fixture.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM schools WHERE id=$1 FOR UPDATE")
        .bind(fixture.school)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let delete = {
        let application = fixture.app.clone();
        let manager = fixture.manager.clone();
        let school = fixture.school;
        let configuration = pending.clone();
        let barrier = barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            application
                .message_advertisements
                .write(
                    &manager,
                    Some(school),
                    Some(id(&configuration)),
                    None,
                    Command::Delete(input(
                        json!({"expected_config_version":configuration["version"]}),
                    )),
                )
                .await
        })
    };
    barrier.wait().await;
    sqlx::query("UPDATE school_message_advertisement_revisions SET status='withdrawn',version=version+1 WHERE id=$1").bind(id(&pending["candidate"])).execute(&mut *blocker).await.unwrap();
    sqlx::query("UPDATE school_message_advertisements SET current_revision_id=NULL,deleted_at=now(),deleted_by=$2,deletion_kind='school_delete',version=version+1 WHERE id=$1").bind(id(&pending)).bind(fixture.manager.id).execute(&mut *blocker).await.unwrap();
    blocker.commit().await.unwrap();
    let stale = service
        .write(
            &fixture.root,
            None,
            None,
            Some(id(&pending["candidate"])),
            fixture.decision(&pending, true),
        )
        .await;
    assert!(matches!(stale, Err(Error::Conflict)));
    assert!(matches!(delete.await.unwrap(), Err(Error::Conflict)));
    assert!(fixture.selected().await.is_none());
    fixture.close().await;
}
