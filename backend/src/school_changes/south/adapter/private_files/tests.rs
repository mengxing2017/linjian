use super::*;
use crate::media::config::Config;
use std::os::unix::fs::PermissionsExt;
use uuid::Uuid;

fn public_files() -> Files {
    Files::open(Config::local(
        std::env::temp_dir().join(format!("linjian-school-default-test-{}", Uuid::new_v4())),
    ))
    .unwrap()
}

#[tokio::test]
async fn defaults_reuse_storage_but_keep_documents_out_of_public_reads() {
    let public = public_files();
    let policy = defaults(&public).unwrap();
    policy.validate().unwrap();
    assert_eq!(
        policy.private_root,
        public
            .config
            .storage
            .filesystem_root
            .join(".school-change-documents")
    );
    assert_eq!(policy.review_working_days, 3);
    let private = open(&policy, &public).unwrap();
    assert_ne!(private.store_id, public.store_id);
    let document = Uuid::new_v4();
    let bytes = vec![1, 2, 3];
    let hash = crate::media::south::adapter::filesystem::digest(&bytes);
    private.write(document, bytes.clone()).await.unwrap();
    assert!(
        public
            .read(public.store_id, document, 3, hash.clone())
            .await
            .is_err()
    );
    assert_eq!(
        private
            .read(private.store_id, document, 3, hash)
            .await
            .unwrap(),
        bytes
    );
    assert_eq!(
        std::fs::metadata(&policy.private_root)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(policy.private_root.join(".linjian-school-change-key"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    std::fs::remove_dir_all(&public.config.storage.filesystem_root).unwrap();
}

#[test]
fn concurrent_initialization_and_restarts_preserve_the_digest_key_and_store() {
    let public = public_files();
    let barrier = std::sync::Barrier::new(4);
    let policies = std::thread::scope(|scope| {
        let handles = (0..4)
            .map(|_| {
                let barrier = &barrier;
                let public = &public;
                scope.spawn(move || {
                    barrier.wait();
                    let policy = defaults(public).unwrap();
                    let store = open(&policy, public).unwrap().store_id;
                    (policy, store)
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    for (policy, store) in &policies {
        assert!(policy.digest_key == policies[0].0.digest_key);
        assert_eq!(*store, policies[0].1);
    }
    let restarted = defaults(&public).unwrap();
    assert!(restarted.digest_key == policies[0].0.digest_key);
    std::fs::remove_dir_all(&public.config.storage.filesystem_root).unwrap();
}

#[test]
fn corrupted_or_symlinked_keys_are_rejected_without_rotation() {
    let public = public_files();
    let policy = defaults(&public).unwrap();
    let path = policy.private_root.join(".linjian-school-change-key");
    std::fs::write(&path, "invalid").unwrap();
    assert!(defaults(&public).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "invalid");
    std::fs::remove_file(&path).unwrap();
    let outside = public.config.storage.filesystem_root.join("outside-key");
    std::fs::write(&outside, "a".repeat(64)).unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    assert!(defaults(&public).is_err());
    std::fs::remove_dir_all(&public.config.storage.filesystem_root).unwrap();
}

#[test]
fn default_private_directory_cannot_be_replaced_by_a_symlink() {
    let public = public_files();
    let outside = public.config.storage.filesystem_root.join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(
        &outside,
        public
            .config
            .storage
            .filesystem_root
            .join(".school-change-documents"),
    )
    .unwrap();
    assert!(defaults(&public).is_err());
    std::fs::remove_dir_all(&public.config.storage.filesystem_root).unwrap();
}

#[test]
fn defaults_inherit_existing_upload_limits_and_reject_the_public_directory() {
    let root = std::env::temp_dir().join(format!("linjian-school-limits-test-{}", Uuid::new_v4()));
    let mut config = Config::local(root.clone());
    config.upload.max_image_bytes = 1024;
    config.upload.max_pixels = 10000;
    let public = Files::open(config).unwrap();
    let mut policy = defaults(&public).unwrap();
    assert_eq!(policy.max_file_bytes, 1024);
    assert_eq!(policy.max_total_bytes, 3072);
    assert_eq!(policy.max_pixels, 10000);
    policy.validate().unwrap();
    policy.private_root = root.clone();
    assert!(open(&policy, &public).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
