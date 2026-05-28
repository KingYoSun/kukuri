use super::super::*;

#[tokio::test]
async fn create_post_and_list_timeline() {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new("app", FakeNetwork::default()));
    let app = AppService::new(store, transport);

    let object_id = app
        .create_post("kukuri:topic:api", "hello app", None)
        .await
        .expect("create post");
    let timeline = app
        .list_timeline("kukuri:topic:api", None, 10)
        .await
        .expect("timeline");

    assert_eq!(timeline.items.len(), 1);
    assert_eq!(timeline.items[0].object_id, object_id);
    assert_eq!(timeline.items[0].content, "hello app");
}
