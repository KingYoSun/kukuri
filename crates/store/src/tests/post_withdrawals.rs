use super::*;
use kukuri_core::{PostWithdrawalReason, WithdrawalReasonVisibility};

fn withdrawal(generation: u64, withdrawn_at: i64, envelope_id: &str) -> PostWithdrawalRow {
    PostWithdrawalRow {
        target_object_id: EnvelopeId::from("object-1"),
        target_author_pubkey: "author-1".into(),
        source_replica_id: ReplicaId::new("topic::withdrawal"),
        withdrawal_envelope_id: EnvelopeId::from(envelope_id),
        withdrawn_at,
        generation,
        replacement_object_id: None,
        reason_visibility: WithdrawalReasonVisibility::Public,
        reason: Some(PostWithdrawalReason::AuthorRequest),
    }
}

async fn withdrawal_contract(store: &dyn PostWithdrawalStore) {
    assert!(
        store
            .put_post_withdrawal(withdrawal(1, 100, "withdrawal-1"))
            .await
            .expect("insert first withdrawal")
    );
    assert!(
        !store
            .put_post_withdrawal(withdrawal(1, 99, "withdrawal-old"))
            .await
            .expect("ignore older withdrawal")
    );
    assert!(
        store
            .put_post_withdrawal(withdrawal(2, 90, "withdrawal-2"))
            .await
            .expect("new generation wins")
    );
    let stored = store
        .get_post_withdrawal(&EnvelopeId::from("object-1"))
        .await
        .expect("load withdrawal")
        .expect("withdrawal exists");
    assert_eq!(stored.generation, 2);
    assert_eq!(stored.withdrawal_envelope_id.as_str(), "withdrawal-2");
}

#[tokio::test]
async fn memory_post_withdrawal_uses_deterministic_generation_order() {
    withdrawal_contract(&MemoryStore::default()).await;
}

#[tokio::test]
async fn sqlite_post_withdrawal_uses_deterministic_generation_order() {
    let store = SqliteStore::connect_memory().await.expect("sqlite store");
    withdrawal_contract(&store).await;
}
