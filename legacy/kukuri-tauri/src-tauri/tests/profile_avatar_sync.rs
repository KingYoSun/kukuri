use kukuri_lib::test_support::application::services::{
    ProfileAvatarService, UploadProfileAvatarInput,
};
use kukuri_lib::test_support::domain::entities::ProfileAvatarAccessLevel;

#[tokio::test]
async fn profile_avatar_sync_between_nodes() {
    let node1_dir = tempfile::tempdir().expect("node1 dir");
    let node2_dir = tempfile::tempdir().expect("node2 dir");

    let service_a = ProfileAvatarService::new(node1_dir.path().to_path_buf())
        .await
        .expect("service a");
    let service_b = ProfileAvatarService::new(node2_dir.path().to_path_buf())
        .await
        .expect("service b");

    let npub = "npub1syncavatar".to_string();
    let avatar_bytes: Vec<u8> = (0..128).collect();

    let upload_entry = service_a
        .upload_avatar(UploadProfileAvatarInput {
            npub: npub.clone(),
            bytes: avatar_bytes.clone(),
            format: "image/png".to_string(),
            access_level: ProfileAvatarAccessLevel::ContactsOnly,
        })
        .await
        .expect("upload");

    let package = service_a
        .export_sync_package(&npub)
        .await
        .expect("export")
        .expect("package");

    let imported_entry = service_b
        .import_sync_package(package)
        .await
        .expect("import");

    assert_eq!(upload_entry.blob_hash, imported_entry.blob_hash);
    assert_eq!(upload_entry.version, imported_entry.version);
    assert_eq!(upload_entry.share_ticket, imported_entry.share_ticket);

    let fetched = service_b.fetch_avatar(&npub).await.expect("fetch on node2");

    assert_eq!(fetched.bytes, avatar_bytes);
    assert_eq!(fetched.metadata.blob_hash, upload_entry.blob_hash);
}
