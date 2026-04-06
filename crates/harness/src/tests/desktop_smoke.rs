use super::*;

#[tokio::test]
async fn desktop_smoke_post_persist() {
    disable_keyring_for_tests();
    let _serial = acquire_scenario_test_lock().await;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root");
    let artifacts = root
        .join("test-results")
        .join("kukuri")
        .join("desktop-smoke-test");
    let result = run_named_scenario(root, "desktop_smoke_post_persist", &artifacts)
        .await
        .expect("scenario");

    assert_eq!(result.status, HarnessStatus::Pass);
    assert!(artifacts.join("result.json").exists());
}
#[tokio::test]
async fn desktop_smoke_live_session_persist() {
    disable_keyring_for_tests();
    let _serial = acquire_scenario_test_lock().await;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root");
    let artifacts = root
        .join("test-results")
        .join("kukuri")
        .join("desktop-smoke-live-session");
    let result = run_named_scenario(root, "desktop_smoke_live_session_persist", &artifacts)
        .await
        .expect("scenario");

    assert_eq!(result.status, HarnessStatus::Pass);
    assert!(artifacts.join("result.json").exists());
}
#[tokio::test]
async fn desktop_smoke_game_room_persist() {
    disable_keyring_for_tests();
    let _serial = acquire_scenario_test_lock().await;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root");
    let artifacts = root
        .join("test-results")
        .join("kukuri")
        .join("desktop-smoke-game-room");
    let result = run_named_scenario(root, "desktop_smoke_game_room_persist", &artifacts)
        .await
        .expect("scenario");

    assert_eq!(result.status, HarnessStatus::Pass);
    assert!(artifacts.join("result.json").exists());
}
#[tokio::test]
async fn desktop_smoke_bookmark_workflow() {
    disable_keyring_for_tests();
    let _serial = acquire_scenario_test_lock().await;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root");
    let artifacts = root
        .join("test-results")
        .join("kukuri")
        .join("desktop-smoke-bookmark-workflow");
    let result = run_named_scenario(root, "desktop_smoke_bookmark_workflow", &artifacts)
        .await
        .expect("scenario");

    assert_eq!(result.status, HarnessStatus::Pass);
    assert!(artifacts.join("result.json").exists());
}
