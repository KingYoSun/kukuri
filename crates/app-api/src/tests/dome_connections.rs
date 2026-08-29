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

async fn open_proposal_fixture(
    suffix: &str,
) -> (AppService, AppService, SpatialContextV1, String, String) {
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let proposer_keys = generate_keys();
    let receiver_keys = generate_keys();
    let proposer_pubkey = proposer_keys.public_key_hex();
    let receiver_pubkey = receiver_keys.public_key_hex();
    let proposer =
        app_with_shared_dome_services(docs_sync.clone(), blob_service.clone(), proposer_keys);
    let receiver = app_with_shared_dome_services(docs_sync, blob_service, receiver_keys);
    let topic = format!("kukuri:topic:dome-open-proposal-{suffix}");
    let context = SpatialContextV1::Topic {
        topic_id: TopicId::new(topic.clone()),
    };
    let proposer_instance = proposer
        .create_metaverse_room(
            &topic,
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
            &topic,
            CreateMetaverseRoomInput {
                title: "Receiver Dome".into(),
                description: String::new(),
                max_peers: Some(8),
            },
        )
        .await
        .expect("create receiver Dome");
    proposer
        .create_dome_connection_proposal(CreateDomeConnectionProposalInput {
            proposal_id: format!("proposal-{suffix}"),
            spatial_context: context.clone(),
            proposer_instance_id: proposer_instance,
            receiver_instance_id: receiver_instance,
            proposer_direction: DomeDirection::East,
        })
        .await
        .expect("create proposal");
    (
        proposer,
        receiver,
        context,
        proposer_pubkey,
        receiver_pubkey,
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

    let revoke = receiver.revoke_dome_connection(RevokeDomeConnectionInput {
        spatial_context: context.clone(),
        connection_id: connection.record.agreement.connection_id,
    });
    tokio::pin!(revoke);
    tokio::select! {
        _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
        _ = &mut revoke => panic!("normal revoke must expose the draining interval"),
    }
    let draining = proposer
        .list_dome_connection_topology(context.clone())
        .await
        .expect("draining topology");
    assert_eq!(
        draining.connections[0].record.status,
        kukuri_core::DomeConnectionStatusV1::Draining
    );
    assert!(
        draining.connections[0]
            .record
            .lifecycle_deadline_at
            .is_some()
    );
    assert_eq!(draining.resolution.topology.components.len(), 1);
    revoke.await.expect("revoke Connection");
    let split = proposer
        .list_dome_connection_topology(context)
        .await
        .expect("split topology");
    assert_eq!(split.resolution.topology.components.len(), 2);
    assert!(split.resolution.topology.active_connection_ids.is_empty());
}

#[tokio::test]
async fn owner_block_revokes_connection_and_unblock_does_not_restore_it() {
    let docs_sync = Arc::new(MemoryDocsSync::default());
    let blob_service = Arc::new(MemoryBlobService::default());
    let proposer_keys = generate_keys();
    let receiver_keys = generate_keys();
    let receiver_pubkey = receiver_keys.public_key_hex();
    let proposer =
        app_with_shared_dome_services(docs_sync.clone(), blob_service.clone(), proposer_keys);
    let receiver = app_with_shared_dome_services(docs_sync, blob_service, receiver_keys);
    let topic = "kukuri:topic:dome-connection-owner-block";
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
    proposer
        .create_dome_connection_proposal(CreateDomeConnectionProposalInput {
            proposal_id: "proposal-owner-block".into(),
            spatial_context: context.clone(),
            proposer_instance_id: proposer_instance,
            receiver_instance_id: receiver_instance,
            proposer_direction: DomeDirection::East,
        })
        .await
        .expect("create proposal");
    receiver
        .accept_dome_connection_proposal(AcceptDomeConnectionProposalInput {
            spatial_context: context.clone(),
            proposal_id: "proposal-owner-block".into(),
        })
        .await
        .expect("accept proposal");

    proposer
        .block_author(receiver_pubkey.as_str())
        .await
        .expect("block endpoint owner");
    let blocked = proposer
        .list_dome_connection_topology(context.clone())
        .await
        .expect("blocked topology");
    assert!(blocked.resolution.topology.active_connection_ids.is_empty());
    assert_eq!(
        blocked.connections[0].record.lifecycle_reason,
        Some(kukuri_core::DomeConnectionTerminalReasonV1::OwnersBlocked)
    );

    proposer
        .unblock_author(receiver_pubkey.as_str())
        .await
        .expect("unblock endpoint owner");
    let unblocked = proposer
        .list_dome_connection_topology(context)
        .await
        .expect("topology after unblock");
    assert!(
        unblocked
            .resolution
            .topology
            .active_connection_ids
            .is_empty()
    );
    assert_eq!(
        unblocked.connections[0].record.status,
        kukuri_core::DomeConnectionStatusV1::Revoked
    );
}

#[tokio::test]
async fn proposer_block_discards_open_proposal_and_unblock_does_not_restore_it() {
    let (proposer, receiver, context, _, receiver_pubkey) =
        open_proposal_fixture("proposer-block").await;

    proposer
        .block_author(&receiver_pubkey)
        .await
        .expect("block receiver owner");
    let blocked = proposer
        .list_dome_connection_topology(context.clone())
        .await
        .expect("blocked topology");
    assert_eq!(
        blocked.proposals[0].status,
        DomeProposalDerivedStatusV1::Discarded
    );
    assert_eq!(
        blocked.proposals[0].terminal_reason,
        Some(kukuri_core::DomeConnectionTerminalReasonV1::OwnersBlocked)
    );
    assert!(blocked.connections.is_empty());
    assert!(
        receiver
            .accept_dome_connection_proposal(AcceptDomeConnectionProposalInput {
                spatial_context: context.clone(),
                proposal_id: "proposal-proposer-block".into(),
            })
            .await
            .is_err()
    );

    proposer
        .unblock_author(&receiver_pubkey)
        .await
        .expect("unblock receiver owner");
    let unblocked = proposer
        .list_dome_connection_topology(context)
        .await
        .expect("topology after unblock");
    assert_eq!(
        unblocked.proposals[0].status,
        DomeProposalDerivedStatusV1::Discarded
    );
}

#[tokio::test]
async fn receiver_block_discards_open_proposal_and_prevents_accept() {
    let (proposer, receiver, context, proposer_pubkey, _) =
        open_proposal_fixture("receiver-block").await;

    receiver
        .block_author(&proposer_pubkey)
        .await
        .expect("block proposer owner");
    let blocked = receiver
        .list_dome_connection_topology(context.clone())
        .await
        .expect("blocked topology");
    assert_eq!(
        blocked.proposals[0].status,
        DomeProposalDerivedStatusV1::Discarded
    );
    assert_eq!(
        blocked.proposals[0].terminal_reason,
        Some(kukuri_core::DomeConnectionTerminalReasonV1::OwnersBlocked)
    );
    assert!(blocked.connections.is_empty());
    assert!(
        receiver
            .accept_dome_connection_proposal(AcceptDomeConnectionProposalInput {
                spatial_context: context.clone(),
                proposal_id: "proposal-receiver-block".into(),
            })
            .await
            .is_err()
    );
    assert!(
        proposer
            .list_dome_connection_topology(context)
            .await
            .expect("proposer topology")
            .connections
            .is_empty()
    );
}

#[tokio::test]
async fn accept_rechecks_owner_block_before_persisting_connection_records() {
    let (_, receiver, context, proposer_pubkey, _) =
        open_proposal_fixture("accept-block-recheck").await;
    let block = build_block_edge_envelope(
        receiver.services.keys.as_ref(),
        &Pubkey::from(proposer_pubkey),
        BlockEdgeStatus::Active,
    )
    .expect("build block edge");
    receiver
        .services
        .store
        .put_envelope(block)
        .await
        .expect("store block edge without reconciliation");

    let error = receiver
        .accept_dome_connection_proposal(AcceptDomeConnectionProposalInput {
            spatial_context: context.clone(),
            proposal_id: "proposal-accept-block-recheck".into(),
        })
        .await
        .expect_err("block must be rechecked before accept persistence");
    assert!(error.to_string().contains("DOME_CONNECTION_OWNERS_BLOCKED"));
    let topology = receiver
        .list_dome_connection_topology(context)
        .await
        .expect("topology after rejected accept");
    assert!(topology.connections.is_empty());
    assert_eq!(
        topology.proposals[0].terminal_reason,
        Some(kukuri_core::DomeConnectionTerminalReasonV1::OwnersBlocked)
    );
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
