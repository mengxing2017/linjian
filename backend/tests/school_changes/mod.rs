use super::*;
use linjian_api::{
    business::{Actor, Error, SessionKind},
    miniapp_users::{domain::SchoolInput, south::adapter::postgres::Repository as Users},
    school_changes::{
        domain::{Decision, Submit, policy::Policy},
        south::adapter::postgres::Settings,
    },
};
use std::io::Cursor;

struct Fixture {
    admin: PgPool,
    pool: PgPool,
    schema: String,
    app: bootstrap::Application,
    reviewer: Actor,
    applicant: Actor,
    token: String,
    reviewer_token: String,
    schools: [Uuid; 3],
    root: std::path::PathBuf,
}
impl Fixture {
    async fn new() -> Self {
        let admin = PgPool::connect(
            &std::env::var("LINJIAN_TEST_DATABASE_URL").expect("isolated test URL required"),
        )
        .await
        .unwrap();
        let schema = format!("school_changes_{}", Uuid::new_v4().simple());
        let pool = isolated(&admin, &schema).await;
        database::migrate(&pool).await.unwrap();
        let root = std::env::temp_dir().join(format!("linjian-private-test-{}", Uuid::new_v4()));
        let mut options = bootstrap::Options::local();
        options.school_changes = Settings::new(
            Policy {
                version: "test-v1".into(),
                notice: "仅使用虚构测试材料；用于目标学校学生关系审核，按测试期限清理".into(),
                review_guidance: "核验目标学校、本人关系、材料有效性".into(),
                private_root: root.clone(),
                digest_key: "only-a-synthetic-test-key-not-a-production-secret".into(),
                student_number_required: false,
                restore_revoked_member: false,
                max_file_bytes: 1024 * 1024,
                max_total_bytes: 3 * 1024 * 1024,
                max_pixels: 1_000_000,
                max_documents: 3,
                max_unbound_documents: 10,
                min_submission_seconds: 1,
                temporary_seconds: 3600,
                review_working_days: 3,
                review_holidays: vec![],
                review_workdays: vec![],
                document_retention_seconds: 3600,
                personal_retention_seconds: 3600,
                audit_retention_seconds: 86400,
                cleanup_interval_seconds: 60,
            },
            &options.files,
        )
        .unwrap();
        let app = bootstrap::build_application(pool.clone(), options);
        let root_id = app
            .identity
            .bootstrap("reviewer", "平台审核员", "test-password-at-least-12", true)
            .await
            .unwrap();
        let (reviewer_token, reviewer) = app
            .identity
            .0
            .session(
                root_id,
                SessionKind::AdminBrowser,
                Some("http://localhost:5173"),
            )
            .await
            .unwrap();
        let mut schools = [Uuid::nil(); 3];
        for (index, name) in ["原学校", "目标学校", "其他授权学校"].iter().enumerate()
        {
            schools[index] = app.schools.create(&reviewer, name).await.unwrap()["id"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
        }
        let owner = linjian_api::miniapp_users::north::local::register(
            &Users(pool.clone()),
            "test-app",
            "test-student",
        )
        .await
        .unwrap();
        let (token, applicant) = app
            .identity
            .0
            .session(owner, SessionKind::Miniapp, None)
            .await
            .unwrap();
        app.schools.bind(&applicant, schools[0]).await.unwrap();
        let applicant = app
            .identity
            .0
            .authenticate(&token, SessionKind::Miniapp)
            .await
            .unwrap();
        Self {
            admin,
            pool,
            schema,
            app,
            reviewer,
            applicant,
            token,
            reviewer_token,
            schools,
            root,
        }
    }
    async fn document(&self) -> Uuid {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(3, 2)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        self.app
            .school_changes
            .upload(&self.applicant, bytes.into_inner())
            .await
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    }
    async fn input(&self) -> Submit {
        Submit {
            to_school_id: self.schools[1],
            student_relationship:
                linjian_api::school_changes::domain::StudentRelationship::NewStudent,
            declared_student_name: "测试申请人".into(),
            declared_student_number: None,
            reason: "升学进入目标学校".into(),
            document_ids: vec![self.document().await],
            consent_version: "test-v1".into(),
            submission_key: Uuid::new_v4(),
        }
    }
    async fn request(&self) -> Value {
        self.app
            .school_changes
            .submit(&self.applicant, self.input().await)
            .await
            .unwrap()
    }
    async fn revision(&self) -> i64 {
        sqlx::query_scalar("SELECT revision FROM miniapp_users WHERE account_id=$1")
            .bind(self.applicant.id)
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
        std::fs::remove_dir_all(self.root).unwrap();
    }
}
fn id(value: &Value) -> Uuid {
    value["id"].as_str().unwrap().parse().unwrap()
}
fn decision(revision: i64) -> Decision {
    Decision {
        revision,
        reason: "已核验与目标学校的学生关系".into(),
    }
}

async fn make_overdue(fixture: &Fixture, request: Uuid) -> chrono::DateTime<chrono::Utc> {
    sqlx::query_scalar("UPDATE miniapp_school_change_requests SET review_due_at=clock_timestamp()-interval '1 second',submitted_at=clock_timestamp()-interval '10 days' WHERE id=$1 RETURNING review_due_at")
        .bind(request).fetch_one(&fixture.pool).await.unwrap()
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn default_settings_accept_review_and_delete_materials_using_existing_storage() {
    let fixture = Fixture::new().await;
    let mut repository = fixture.app.school_changes.0.clone();
    repository.settings = Settings::defaults(&fixture.app.options.files).unwrap();
    let path = repository.settings.policy.private_root.clone();
    let service = linjian_api::school_changes::north::local::Service(repository);
    let policy = service.policy(&fixture.applicant).unwrap();
    assert_eq!(policy["enabled"], true);
    assert_eq!(policy["review_working_days"], 3);
    let mut input = fixture.input().await;
    let bytes = fixture
        .app
        .school_changes
        .document(&fixture.applicant, input.document_ids[0], None)
        .await
        .unwrap();
    let uploaded = service.upload(&fixture.applicant, bytes).await.unwrap();
    let document = id(&uploaded);
    input.document_ids = vec![document];
    input.consent_version = policy["version"].as_str().unwrap().into();
    let request = service
        .submit(&fixture.applicant, input.clone())
        .await
        .unwrap();
    let mut restarted = fixture.app.school_changes.0.clone();
    restarted.settings = Settings::defaults(&fixture.app.options.files).unwrap();
    let restarted = linjian_api::school_changes::north::local::Service(restarted);
    assert_eq!(
        id(&restarted.submit(&fixture.applicant, input).await.unwrap()),
        id(&request)
    );
    let approved = restarted
        .review(&fixture.reviewer, id(&request), decision(1), true)
        .await
        .unwrap();
    assert_eq!(approved["status"], "approved");
    assert_eq!(approved["documents"][0]["available"], false);
    assert!(!path.join(format!("{document}.blob")).exists());
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn overdue_requests_are_rejected_once_without_a_human_reviewer() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    let deadline = make_overdue(&fixture, id(&request)).await;
    let service = fixture.app.school_changes.clone();
    service.cleanup().await.unwrap();
    service.cleanup().await.unwrap();
    let result = fixture
        .app
        .school_changes
        .current(&fixture.applicant)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["status"], "rejected");
    assert_eq!(result["result_reason_code"], "review_timeout");
    assert_eq!(result["reviewed_by"], Value::Null);
    assert_eq!(result["reviewed_at"], Value::Null);
    assert_eq!(result["revision"], 2);
    assert_eq!(result["documents"][0]["available"], true);
    let closed: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT closed_at FROM miniapp_school_change_requests WHERE id=$1")
            .bind(id(&request))
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(closed, deadline);
    let purge: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "SELECT purge_after FROM miniapp_school_change_documents WHERE request_id=$1",
    )
    .bind(id(&request))
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(purge, deadline + chrono::Duration::seconds(3600));
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM miniapp_school_change_events WHERE request_id=$1 AND action='rejected' AND actor_id IS NULL AND reason_code='review_timeout'").bind(id(&request)).fetch_one(&fixture.pool).await.unwrap();
    assert_eq!(events, 1);
    let bound: Uuid = sqlx::query_scalar("SELECT bound_school_id FROM accounts WHERE id=$1")
        .bind(fixture.applicant.id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(bound, fixture.schools[0]);
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn overdue_approval_racing_cleanup_cannot_change_school_or_create_two_results() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    make_overdue(&fixture, id(&request)).await;
    let request_id = id(&request);
    let (review, cleanup) = tokio::join!(
        fixture
            .app
            .school_changes
            .review(&fixture.reviewer, request_id, decision(1), true),
        fixture.app.school_changes.cleanup()
    );
    assert!(review.is_err());
    cleanup.unwrap();
    let result = fixture
        .app
        .school_changes
        .current(&fixture.applicant)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["status"], "rejected");
    assert_eq!(result["result_reason_code"], "review_timeout");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM miniapp_school_change_events WHERE request_id=$1 AND action IN ('approved','rejected','cancelled')").bind(request_id).fetch_one(&fixture.pool).await.unwrap();
    assert_eq!(count, 1);
    let bound: Uuid = sqlx::query_scalar("SELECT bound_school_id FROM accounts WHERE id=$1")
        .bind(fixture.applicant.id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(bound, fixture.schools[0]);
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn reading_an_overdue_result_closes_it_and_allows_a_new_enrolled_application() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    make_overdue(&fixture, id(&request)).await;
    let result = fixture
        .app
        .school_changes
        .current(&fixture.applicant)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["result_reason_code"], "review_timeout");
    let mut input = fixture.input().await;
    input.student_relationship = linjian_api::school_changes::domain::StudentRelationship::Enrolled;
    let next = fixture
        .app
        .school_changes
        .submit(&fixture.applicant, input)
        .await
        .unwrap();
    assert_ne!(next["id"], result["id"]);
    assert_eq!(next["student_relationship"], "enrolled");
    sqlx::query("UPDATE miniapp_school_change_requests SET student_relationship=NULL WHERE id=$1")
        .bind(id(&next))
        .execute(&fixture.pool)
        .await
        .unwrap();
    let failure = fixture
        .app
        .school_changes
        .review(&fixture.reviewer, id(&next), decision(1), true)
        .await;
    assert!(matches!(
        failure,
        Err(Error::Policy("school_change_relationship_required"))
    ));
    fixture
        .app
        .school_changes
        .cancel(&fixture.applicant, id(&next), 1)
        .await
        .unwrap();
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn withdrawal_after_deadline_persists_automatic_rejection_before_returning_conflict() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    make_overdue(&fixture, id(&request)).await;
    let result = fixture
        .app
        .school_changes
        .cancel(&fixture.applicant, id(&request), 1)
        .await;
    assert!(matches!(
        result,
        Err(Error::Policy("school_change_review_timeout"))
    ));
    let result = fixture
        .app
        .school_changes
        .current(&fixture.applicant)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["status"], "rejected");
    assert_eq!(result["revision"], 2);
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn overdue_material_reads_enforce_terminal_retention_without_waiting_for_the_worker() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    let document = request["documents"][0]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    sqlx::query("UPDATE miniapp_school_change_requests SET review_due_at=clock_timestamp()-interval '2 hours' WHERE id=$1").bind(id(&request)).execute(&fixture.pool).await.unwrap();
    assert!(
        fixture
            .app
            .school_changes
            .document(&fixture.applicant, document, None)
            .await
            .is_err()
    );
    let result = fixture
        .app
        .school_changes
        .current(&fixture.applicant)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["result_reason_code"], "review_timeout");
    assert_eq!(result["documents"][0]["available"], false);
    assert_eq!(result["personal_data_purged"], true);
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn submission_is_idempotent_account_scoped_and_matches_contract() {
    let fixture = Fixture::new().await;
    assert!(
        fixture
            .app
            .school_changes
            .current(&fixture.applicant)
            .await
            .unwrap()
            .is_none()
    );
    sqlx::query("UPDATE schools SET active=false WHERE id=$1")
        .bind(fixture.schools[0])
        .execute(&fixture.pool)
        .await
        .unwrap();
    let input = fixture.input().await;
    let result = fixture
        .app
        .school_changes
        .submit(&fixture.applicant, input.clone())
        .await
        .unwrap();
    let retried = fixture
        .app
        .school_changes
        .submit(&fixture.applicant, input.clone())
        .await
        .unwrap();
    assert_eq!(result["id"], retried["id"]);
    sqlx::query("UPDATE miniapp_school_change_events SET purge_after=now()-interval '1 second' WHERE request_id=$1 AND action='uploaded'")
        .bind(id(&result)).execute(&fixture.pool).await.unwrap();
    fixture.app.school_changes.cleanup().await.unwrap();
    let uploads: i64 = sqlx::query_scalar("SELECT count(*) FROM miniapp_school_change_events WHERE request_id=$1 AND action='uploaded'")
        .bind(id(&result)).fetch_one(&fixture.pool).await.unwrap();
    assert_eq!(uploads, 1);
    let fixture_document: Value = serde_json::from_str(include_str!(
        "../../../api-contract/fixtures/school-change-pending.json"
    ))
    .unwrap();
    let mut keys: Vec<_> = result
        .as_object()
        .unwrap()
        .keys()
        .filter(|key| *key != "id")
        .collect();
    let mut expected: Vec<_> = fixture_document["data"]["attributes"]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    keys.sort();
    expected.sort();
    assert_eq!(keys, expected);
    let mut edited = input.clone();
    edited.reason = "另一个原因".into();
    assert!(matches!(
        fixture
            .app
            .school_changes
            .submit(&fixture.applicant, edited)
            .await,
        Err(Error::Policy("school_change_idempotency_conflict"))
    ));
    let mut another = input;
    another.submission_key = Uuid::new_v4();
    assert!(matches!(
        fixture
            .app
            .school_changes
            .submit(&fixture.applicant, another)
            .await,
        Err(Error::Policy("school_change_pending"))
    ));
    let document = result["documents"][0]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        !fixture
            .app
            .school_changes
            .document(&fixture.applicant, document, None)
            .await
            .unwrap()
            .is_empty()
    );
    let mut operator = fixture.reviewer.clone();
    operator.platform_admin = false;
    assert!(
        fixture
            .app
            .school_changes
            .detail(&operator, id(&result), true)
            .await
            .is_err()
    );
    assert!(
        fixture
            .app
            .school_changes
            .document(&operator, document, Some(id(&result)))
            .await
            .is_err()
    );
    let list = fixture
        .app
        .school_changes
        .list(&fixture.reviewer, &Default::default())
        .await
        .unwrap();
    assert!(list[0].get("declared_student_name").is_none());
    assert!(list[0].get("documents").is_none());
    let events: String = sqlx::query_scalar("SELECT coalesce(string_agg(row_to_json(e)::text,''),'') FROM miniapp_school_change_events e").fetch_one(&fixture.pool).await.unwrap();
    assert!(!events.contains("测试申请人"));
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn approval_switches_atomically_revokes_sessions_and_keeps_other_grants() {
    let fixture = Fixture::new().await;
    sqlx::query("INSERT INTO school_memberships(school_id,account_id,role) VALUES($1,$2,'member')")
        .bind(fixture.schools[2])
        .bind(fixture.applicant.id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let request = fixture.request().await;
    let before = fixture.revision().await;
    let approved = fixture
        .app
        .school_changes
        .review(&fixture.reviewer, id(&request), decision(1), true)
        .await
        .unwrap();
    assert_eq!(approved["status"], "approved");
    let documents = approved["documents"].as_array().unwrap();
    assert!(!documents.is_empty());
    for document in documents {
        assert_eq!(document["available"], false);
        let document_id = id(document);
        assert!(!fixture.root.join(format!("{document_id}.blob")).exists());
        let deleted: bool = sqlx::query_scalar("SELECT state='deleted' AND deleted_at IS NOT NULL AND sha256='' AND purge_after<=(SELECT closed_at FROM miniapp_school_change_requests WHERE id=$2) FROM miniapp_school_change_documents WHERE id=$1")
            .bind(document_id).bind(id(&request)).fetch_one(&fixture.pool).await.unwrap();
        assert!(deleted);
        assert!(matches!(
            fixture
                .app
                .school_changes
                .document(&fixture.reviewer, document_id, Some(id(&request)))
                .await,
            Err(Error::Policy("school_change_document_unavailable"))
        ));
    }
    assert_eq!(fixture.revision().await, before + 1);
    assert!(
        fixture
            .app
            .identity
            .0
            .authenticate(&fixture.token, SessionKind::Miniapp)
            .await
            .is_err()
    );
    let (_, renewed) = fixture
        .app
        .identity
        .0
        .session(fixture.applicant.id, SessionKind::Miniapp, None)
        .await
        .unwrap();
    assert_eq!(renewed.bound_school_id, Some(fixture.schools[1]));
    assert!(
        renewed
            .grants
            .iter()
            .any(|grant| grant.school_id == fixture.schools[2])
    );
    assert!(
        !renewed
            .grants
            .iter()
            .any(|grant| grant.school_id == fixture.schools[0])
    );
    assert_eq!(
        fixture
            .app
            .school_changes
            .current(&renewed)
            .await
            .unwrap()
            .unwrap()["status"],
        "approved"
    );
    assert!(
        fixture
            .app
            .school_changes
            .review(&fixture.reviewer, id(&request), decision(1), true)
            .await
            .is_err()
    );
    assert!(
        fixture
            .app
            .school_changes
            .cancel(&renewed, id(&request), 1)
            .await
            .is_err()
    );
    let outbox: i64 = sqlx::query_scalar("SELECT count(*) FROM forum_outbox")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(outbox, 0);
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn correction_invalidates_old_requests_even_after_returning_to_source() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    for target in [fixture.schools[2], fixture.schools[0]] {
        Users(fixture.pool.clone())
            .school(
                &fixture.reviewer,
                fixture.applicant.id,
                SchoolInput {
                    school_id: target,
                    reason: "纠正错误归属".into(),
                    revision: fixture.revision().await,
                },
            )
            .await
            .unwrap();
    }
    let result = fixture
        .app
        .school_changes
        .detail(&fixture.reviewer, id(&request), true)
        .await
        .unwrap();
    assert_eq!(result["status"], "invalidated");
    assert_eq!(result["reviewed_by"], Value::Null);
    assert!(
        fixture
            .app
            .school_changes
            .review(&fixture.reviewer, id(&request), decision(1), true)
            .await
            .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn failed_approval_rolls_back_result_membership_and_session_revocation() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    sqlx::query(&format!("ALTER TABLE accounts ADD CONSTRAINT reject_test_target CHECK(bound_school_id IS DISTINCT FROM '{}')", fixture.schools[1])).execute(&fixture.pool).await.unwrap();
    let revision = fixture.revision().await;
    assert!(
        fixture
            .app
            .school_changes
            .review(&fixture.reviewer, id(&request), decision(1), true)
            .await
            .is_err()
    );
    assert_eq!(fixture.revision().await, revision);
    assert_eq!(
        fixture
            .app
            .school_changes
            .current(&fixture.applicant)
            .await
            .unwrap()
            .unwrap()["status"],
        "pending"
    );
    assert!(
        fixture
            .app
            .identity
            .0
            .authenticate(&fixture.token, SessionKind::Miniapp)
            .await
            .is_ok()
    );
    let active: bool = sqlx::query_scalar(
        "SELECT role='member' FROM school_memberships WHERE account_id=$1 AND school_id=$2",
    )
    .bind(fixture.applicant.id)
    .bind(fixture.schools[0])
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert!(active);
    let document_id = id(&request["documents"][0]);
    assert!(fixture.root.join(format!("{document_id}.blob")).exists());
    assert!(
        fixture
            .app
            .school_changes
            .document(&fixture.applicant, document_id, None)
            .await
            .is_ok()
    );
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn revoked_targets_role_conflicts_and_stopped_schools_cannot_be_approved() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    for role in ["revoked", "admin", "operator"] {
        sqlx::query("INSERT INTO school_memberships(school_id,account_id,role) VALUES($1,$2,$3) ON CONFLICT(school_id,account_id) DO UPDATE SET role=EXCLUDED.role").bind(fixture.schools[1]).bind(fixture.applicant.id).bind(role).execute(&fixture.pool).await.unwrap();
        let result = fixture
            .app
            .school_changes
            .review(&fixture.reviewer, id(&request), decision(1), true)
            .await;
        let expected = if role == "revoked" {
            "school_change_target_member_revoked"
        } else {
            "school_change_role_conflict"
        };
        assert!(
            matches!(result, Err(linjian_api::business::Error::Policy(code)) if code == expected)
        );
    }
    sqlx::query("UPDATE school_memberships SET role='member' WHERE school_id=$1 AND account_id=$2")
        .bind(fixture.schools[1])
        .bind(fixture.applicant.id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE schools SET active=false WHERE id=$1")
        .bind(fixture.schools[1])
        .execute(&fixture.pool)
        .await
        .unwrap();
    assert!(
        fixture
            .app
            .school_changes
            .review(&fixture.reviewer, id(&request), decision(1), true)
            .await
            .is_err()
    );
    let result = fixture
        .app
        .school_changes
        .cancel(&fixture.applicant, id(&request), 1)
        .await
        .unwrap();
    assert_eq!(result["status"], "cancelled");
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn cancelled_materials_expire_and_cleanup_does_not_ban_reuploading_same_image() {
    let fixture = Fixture::new().await;
    let original = fixture.document().await;
    let repeated = fixture.document().await;
    assert_ne!(original, repeated);
    let input = Submit {
        document_ids: vec![original],
        ..fixture.input().await
    };
    let request = fixture
        .app
        .school_changes
        .submit(&fixture.applicant, input)
        .await
        .unwrap();
    fixture
        .app
        .school_changes
        .cancel(&fixture.applicant, id(&request), 1)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE miniapp_school_change_requests SET closed_at=now()-interval '2 hours' WHERE id=$1",
    )
    .bind(id(&request))
    .execute(&fixture.pool)
    .await
    .unwrap();
    sqlx::query("UPDATE miniapp_school_change_documents SET purge_after=now()-interval '1 second' WHERE request_id=$1").bind(id(&request)).execute(&fixture.pool).await.unwrap();
    assert!(
        fixture
            .app
            .school_changes
            .document(&fixture.applicant, original, None)
            .await
            .is_err()
    );
    let result = fixture
        .app
        .school_changes
        .current(&fixture.applicant)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["declared_student_name"], Value::Null);
    fixture.app.school_changes.cleanup().await.unwrap();
    let state: String =
        sqlx::query_scalar("SELECT state FROM miniapp_school_change_documents WHERE id=$1")
            .bind(original)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(state, "deleted");
    assert!(
        !fixture
            .app
            .school_changes
            .document(&fixture.applicant, repeated, None)
            .await
            .unwrap()
            .is_empty()
    );
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn concurrent_submit_and_review_actions_have_only_one_winner() {
    let fixture = Fixture::new().await;
    let input = fixture.input().await;
    let (first, retried) = tokio::join!(
        fixture
            .app
            .school_changes
            .submit(&fixture.applicant, input.clone()),
        fixture.app.school_changes.submit(&fixture.applicant, input)
    );
    let first = first.unwrap();
    assert_eq!(first["id"], retried.unwrap()["id"]);
    let request = id(&first);
    let barrier = tokio::sync::Barrier::new(2);
    let approve = async {
        barrier.wait().await;
        fixture
            .app
            .school_changes
            .review(&fixture.reviewer, request, decision(1), true)
            .await
    };
    let cancel = async {
        barrier.wait().await;
        fixture
            .app
            .school_changes
            .cancel(&fixture.applicant, request, 1)
            .await
    };
    let (approved, cancelled) = tokio::join!(approve, cancel);
    assert_ne!(approved.is_ok(), cancelled.is_ok());
    let status: String =
        sqlx::query_scalar("SELECT status FROM miniapp_school_change_requests WHERE id=$1")
            .bind(request)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert!(matches!(status.as_str(), "approved" | "cancelled"));
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM miniapp_school_change_events WHERE request_id=$1 AND action IN ('approved','cancelled')").bind(request).fetch_one(&fixture.pool).await.unwrap();
    assert_eq!(events, 1);
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn account_scoped_http_queries_enforce_no_store_and_jsonapi() {
    use tower::ServiceExt;
    let fixture = Fixture::new().await;
    let router = linjian_api::http::app_router(fixture.app.clone());
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/miniapp/school-change-requests/current")
                .header("authorization", format!("Bearer {}", fixture.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap()).unwrap();
    assert_eq!(value, json!({"data":null}));
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/platform/school-change-requests")
                .header("authorization", format!("Bearer {}", fixture.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    fixture.request().await;
    let response = router.oneshot(Request::builder().uri(format!("/api/v1/platform/school-change-requests?status=pending&page%5Bnumber%5D=1&created_from=2026-01-01T00:00:00Z&account_id={}", fixture.applicant.id)).header("cookie", format!("linjian_session={}", fixture.reviewer_token)).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap()).unwrap();
    assert_eq!(value["data"].as_array().unwrap().len(), 1);
    let other = fixture
        .app
        .school_changes
        .list(
            &fixture.reviewer,
            &linjian_api::school_changes::domain::Filters {
                account_id: Some(fixture.reviewer.id),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(other.is_empty());
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn account_closure_revokes_material_access_before_physical_cleanup() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    let document = request["documents"][0]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    fixture
        .app
        .school_changes
        .purge_account_data(fixture.applicant.id)
        .await
        .unwrap();
    assert!(
        fixture
            .app
            .school_changes
            .document(&fixture.applicant, document, None)
            .await
            .is_err()
    );
    let result = fixture
        .app
        .school_changes
        .current(&fixture.applicant)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["status"], "invalidated");
    assert_eq!(result["result_reason_code"], "account_closed");
    assert_eq!(result["declared_student_name"], Value::Null);
    fixture.app.school_changes.cleanup().await.unwrap();
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn self_review_and_cross_account_documents_are_forbidden() {
    let fixture = Fixture::new().await;
    let document = fixture.document().await;
    sqlx::query("INSERT INTO miniapp_users(account_id,nickname) VALUES($1,'测试审核员学生身份')")
        .bind(fixture.reviewer.id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let (_, reviewer_student) = fixture
        .app
        .identity
        .0
        .session(fixture.reviewer.id, SessionKind::Miniapp, None)
        .await
        .unwrap();
    fixture
        .app
        .schools
        .bind(&reviewer_student, fixture.schools[0])
        .await
        .unwrap();
    assert!(matches!(
        fixture
            .app
            .school_changes
            .document(&reviewer_student, document, None)
            .await,
        Err(Error::NotFound)
    ));
    let mut input = fixture.input().await;
    input.document_ids = vec![document];
    assert!(matches!(
        fixture
            .app
            .school_changes
            .submit(&reviewer_student, input.clone())
            .await,
        Err(Error::Policy("school_change_document_unavailable"))
    ));
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(3, 2)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let own = fixture
        .app
        .school_changes
        .upload(&reviewer_student, bytes.into_inner())
        .await
        .unwrap();
    input.document_ids = vec![id(&own)];
    let request = fixture
        .app
        .school_changes
        .submit(&reviewer_student, input)
        .await
        .unwrap();
    assert!(matches!(
        fixture
            .app
            .school_changes
            .review(&fixture.reviewer, id(&request), decision(1), true)
            .await,
        Err(Error::Policy("school_change_self_review"))
    ));
    assert!(matches!(
        fixture
            .app
            .school_changes
            .review(&fixture.reviewer, id(&request), decision(1), false)
            .await,
        Err(Error::Policy("school_change_self_review"))
    ));
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn previously_approved_materials_are_revoked_by_the_purge_migration_and_deleted() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    let document_id = id(&request["documents"][0]);
    let path = fixture.root.join(format!("{document_id}.blob"));
    let bytes = std::fs::read(&path).unwrap();
    fixture
        .app
        .school_changes
        .review(&fixture.reviewer, id(&request), decision(1), true)
        .await
        .unwrap();
    std::fs::write(&path, bytes).unwrap();
    sqlx::query("UPDATE miniapp_school_change_documents SET state='ready',deleted_at=NULL,purge_after=now()+interval '1 hour',retry_at=now()+interval '1 hour' WHERE id=$1")
        .bind(document_id).execute(&fixture.pool).await.unwrap();
    assert!(matches!(
        fixture
            .app
            .school_changes
            .document(&fixture.reviewer, document_id, Some(id(&request)))
            .await,
        Err(Error::Policy("school_change_document_unavailable"))
    ));
    sqlx::raw_sql(include_str!(
        "../../migrations/202611210008_school_change_approved_document_purge.sql"
    ))
    .execute(&fixture.pool)
    .await
    .unwrap();
    let due: bool = sqlx::query_scalar("SELECT d.purge_after<=r.closed_at AND d.retry_at<=clock_timestamp() FROM miniapp_school_change_documents d JOIN miniapp_school_change_requests r ON r.id=d.request_id WHERE d.id=$1")
        .bind(document_id).fetch_one(&fixture.pool).await.unwrap();
    assert!(due);
    assert!(path.exists());
    assert!(matches!(
        fixture
            .app
            .school_changes
            .document(&fixture.reviewer, document_id, Some(id(&request)))
            .await,
        Err(Error::Policy("school_change_document_unavailable"))
    ));
    let detail = fixture
        .app
        .school_changes
        .detail(&fixture.reviewer, id(&request), true)
        .await
        .unwrap();
    assert_eq!(detail["documents"][0]["available"], false);
    fixture.app.school_changes.cleanup().await.unwrap();
    assert!(!path.exists());
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn approved_material_deletion_failures_do_not_undo_the_result_and_retry() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    let document_id = id(&request["documents"][0]);
    let path = fixture.root.join(format!("{document_id}.blob"));
    sqlx::query("ALTER TABLE miniapp_school_change_documents ADD CONSTRAINT reject_test_deletion CHECK(state<>'deleting')")
        .execute(&fixture.pool).await.unwrap();
    let approved = fixture
        .app
        .school_changes
        .review(&fixture.reviewer, id(&request), decision(1), true)
        .await
        .unwrap();
    assert_eq!(approved["status"], "approved");
    assert_eq!(approved["documents"][0]["available"], false);
    assert!(path.exists());
    assert!(matches!(
        fixture
            .app
            .school_changes
            .document(&fixture.reviewer, document_id, Some(id(&request)))
            .await,
        Err(Error::Policy("school_change_document_unavailable"))
    ));
    sqlx::query("ALTER TABLE miniapp_school_change_documents DROP CONSTRAINT reject_test_deletion")
        .execute(&fixture.pool)
        .await
        .unwrap();
    fixture.app.school_changes.cleanup().await.unwrap();
    assert!(!path.exists());
    let status: String =
        sqlx::query_scalar("SELECT status FROM miniapp_school_change_requests WHERE id=$1")
            .bind(id(&request))
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(status, "approved");
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn physical_deletion_failures_remain_unreadable_and_retry_durably() {
    let fixture = Fixture::new().await;
    let document = fixture.document().await;
    sqlx::query("UPDATE miniapp_school_change_documents SET expires_at=now()-interval '1 second' WHERE id=$1").bind(document).execute(&fixture.pool).await.unwrap();
    let path = fixture.root.join(format!("{document}.blob"));
    let saved = fixture.root.join(format!("{document}.saved"));
    std::fs::rename(&path, &saved).unwrap();
    std::fs::create_dir(&path).unwrap();
    fixture.app.school_changes.cleanup().await.unwrap();
    let state: String =
        sqlx::query_scalar("SELECT state FROM miniapp_school_change_documents WHERE id=$1")
            .bind(document)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(state, "deleting");
    assert!(
        fixture
            .app
            .school_changes
            .document(&fixture.applicant, document, None)
            .await
            .is_err()
    );
    std::fs::remove_dir(&path).unwrap();
    std::fs::rename(saved, path).unwrap();
    sqlx::query("UPDATE miniapp_school_change_documents SET retry_at=now() WHERE id=$1")
        .bind(document)
        .execute(&fixture.pool)
        .await
        .unwrap();
    fixture.app.school_changes.cleanup().await.unwrap();
    let state: String =
        sqlx::query_scalar("SELECT state FROM miniapp_school_change_documents WHERE id=$1")
            .bind(document)
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(state, "deleted");
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn profile_revision_updates_do_not_invalidate_pending_requests() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    Users(fixture.pool.clone())
        .profile(
            &fixture.applicant,
            linjian_api::miniapp_users::domain::ProfileInput {
                nickname: "新的普通昵称".into(),
                avatar_asset_id: None,
                revision: fixture.revision().await,
            },
        )
        .await
        .unwrap();
    let result = fixture
        .app
        .school_changes
        .review(&fixture.reviewer, id(&request), decision(1), true)
        .await
        .unwrap();
    assert_eq!(result["status"], "approved");
    fixture.close().await;
}

#[tokio::test]
#[ignore = "requires an explicit isolated LINJIAN_TEST_DATABASE_URL"]
async fn a_role_grant_holding_the_shared_account_lock_cannot_be_overwritten() {
    let fixture = Fixture::new().await;
    let request = fixture.request().await;
    let request = id(&request);
    let mut grant = fixture.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(fixture.applicant.id)
        .fetch_one(&mut *grant)
        .await
        .unwrap();
    let barrier = tokio::sync::Barrier::new(2);
    let approval = async {
        barrier.wait().await;
        fixture
            .app
            .school_changes
            .review(&fixture.reviewer, request, decision(1), true)
            .await
    };
    let commit_role = async {
        barrier.wait().await;
        sqlx::query(
            "INSERT INTO school_memberships(school_id,account_id,role) VALUES($1,$2,'operator')",
        )
        .bind(fixture.schools[1])
        .bind(fixture.applicant.id)
        .execute(&mut *grant)
        .await
        .unwrap();
        grant.commit().await.unwrap();
    };
    let (approved, ()) = tokio::join!(approval, commit_role);
    assert!(approved.is_err());
    let role: String = sqlx::query_scalar(
        "SELECT role FROM school_memberships WHERE school_id=$1 AND account_id=$2",
    )
    .bind(fixture.schools[1])
    .bind(fixture.applicant.id)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(role, "operator");
    fixture.close().await;
}
