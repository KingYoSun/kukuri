use super::*;
use kukuri_core::{DomeDirection, DomeProposalDerivedStatusV1, SpatialContextV1};

fn app_with_shared_dome_services(
    docs_sync: Arc<MemoryDocsSync>,
    blob_service: Arc<MemoryBlobService>,
    keys: KukuriKeys,
) -> AppService {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(StaticTransport::new(PeerSnapshot::default()));
    app_service_from_dependencies(
        store.clone(),
        store,
        transport,
        Arc::new(NoopHintTransport),
        docs_sync,
        blob_service,
        keys,
    )
}

#[tokio::test]
async fn dome_connection_proposal_accept_and_revoke_round_trip() {
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let proposer_keys = generate_keys();
    let receiver_keys = generate_keys();
    let proposer =
        app_with_shared_dome_services(docs_sync.clone(), blob_service.clone(), proposer_keys);
    let receiver = app_with_shared_dome_services(docs_sync, blob_service, receiver_keys);
    let topic = "kukuri:topic:dome-connection-round-trip";
    let context = SpatialContextV1::Topic {
        topic_id: TopicId::new(topic),
    };
    let proposer_instance = proposer
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "Proposer Dome".into(),
                description: String::new(),
                max_peers: Some(8),
            },
        )
        .await
        .expect("create proposer Dome");
    let receiver_instance = receiver
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "Receiver Dome".into(),
                description: String::new(),
                max_peers: Some(8),
            },
        )
        .await
        .expect("create receiver Dome");

    let proposal = proposer
        .create_dome_connection_proposal(CreateDomeConnectionProposalInput {
            proposal_id: "proposal-round-trip".into(),
            spatial_context: context.clone(),
            proposer_instance_id: proposer_instance.clone(),
            receiver_instance_id: receiver_instance.clone(),
            proposer_direction: DomeDirection::East,
        })
        .await
        .expect("create proposal");
    assert_eq!(proposal.status, DomeProposalDerivedStatusV1::Proposed);
    assert_eq!(proposal.proposal.receiver.direction, DomeDirection::West);
    let replayed = proposer
        .create_dome_connection_proposal(CreateDomeConnectionProposalInput {
            proposal_id: "proposal-round-trip".into(),
            spatial_context: context.clone(),
            proposer_instance_id: proposer_instance.clone(),
            receiver_instance_id: receiver_instance,
            proposer_direction: DomeDirection::East,
        })
        .await
        .expect("replay proposal operation");
    assert_eq!(replayed.connection_id, proposal.connection_id);
    assert!(
        proposer
            .create_dome_connection_proposal(CreateDomeConnectionProposalInput {
                proposal_id: "proposal-round-trip".into(),
                spatial_context: context.clone(),
                proposer_instance_id: proposer_instance,
                receiver_instance_id: "different-instance".into(),
                proposer_direction: DomeDirection::East,
            })
            .await
            .is_err()
    );

    let connection = receiver
        .accept_dome_connection_proposal(AcceptDomeConnectionProposalInput {
            spatial_context: context.clone(),
            proposal_id: "proposal-round-trip".into(),
        })
        .await
        .expect("accept proposal");
    assert_eq!(
        connection.record.status,
        kukuri_core::DomeConnectionStatusV1::Active
    );

    let proposer_view = proposer
        .list_dome_connection_topology(context.clone())
        .await
        .expect("proposer topology");
    let receiver_view = receiver
        .list_dome_connection_topology(context.clone())
        .await
        .expect("receiver topology");
    assert_eq!(proposer_view.resolution, receiver_view.resolution);
    assert_eq!(proposer_view.resolution.topology.components.len(), 1);
    assert_eq!(
        proposer_view.proposals[0].status,
        DomeProposalDerivedStatusV1::Accepted
    );

    receiver
        .revoke_dome_connection(RevokeDomeConnectionInput {
            spatial_context: context.clone(),
            connection_id: connection.record.agreement.connection_id,
        })
        .await
        .expect("revoke Connection");
    let split = proposer
        .list_dome_connection_topology(context)
        .await
        .expect("split topology");
    assert_eq!(split.resolution.topology.components.len(), 2);
    assert!(split.resolution.topology.active_connection_ids.is_empty());
}

#[tokio::test]
async fn only_proposer_can_withdraw_and_only_receiver_can_accept() {
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let proposer =
        app_with_shared_dome_services(docs_sync.clone(), blob_service.clone(), generate_keys());
    let receiver =
        app_with_shared_dome_services(docs_sync.clone(), blob_service.clone(), generate_keys());
    let outsider = app_with_shared_dome_services(docs_sync, blob_service, generate_keys());
    let topic = "kukuri:topic:dome-connection-auth";
    let context = SpatialContextV1::Topic {
        topic_id: TopicId::new(topic),
    };
    let proposer_instance = proposer
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "A".into(),
                description: String::new(),
                max_peers: None,
            },
        )
        .await
        .expect("create A");
    let receiver_instance = receiver
        .create_metaverse_room(
            topic,
            CreateMetaverseRoomInput {
                title: "B".into(),
                description: String::new(),
                max_peers: None,
            },
        )
        .await
        .expect("create B");
    assert!(
        proposer
            .create_dome_connection_proposal(CreateDomeConnectionProposalInput {
                proposal_id: "../invalid".into(),
                spatial_context: context.clone(),
                proposer_instance_id: proposer_instance.clone(),
                receiver_instance_id: receiver_instance.clone(),
                proposer_direction: DomeDirection::North,
            })
            .await
            .is_err()
    );
    proposer
        .create_dome_connection_proposal(CreateDomeConnectionProposalInput {
            proposal_id: "proposal-auth".into(),
            spatial_context: context.clone(),
            proposer_instance_id: proposer_instance,
            receiver_instance_id: receiver_instance,
            proposer_direction: DomeDirection::North,
        })
        .await
        .expect("create proposal");

    assert!(
        outsider
            .accept_dome_connection_proposal(AcceptDomeConnectionProposalInput {
                spatial_context: context.clone(),
                proposal_id: "proposal-auth".into(),
            })
            .await
            .is_err()
    );
    assert!(
        receiver
            .withdraw_dome_connection_proposal(WithdrawDomeConnectionProposalInput {
                spatial_context: context.clone(),
                proposal_id: "proposal-auth".into(),
            })
            .await
            .is_err()
    );
    let withdrawn = proposer
        .withdraw_dome_connection_proposal(WithdrawDomeConnectionProposalInput {
            spatial_context: context,
            proposal_id: "proposal-auth".into(),
        })
        .await
        .expect("withdraw proposal");
    assert_eq!(withdrawn.status, DomeProposalDerivedStatusV1::Discarded);
}
