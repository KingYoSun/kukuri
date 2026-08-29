use crate::*;

fn context() -> SpatialContextV1 {
    SpatialContextV1::Topic {
        topic_id: TopicId::new("kukuri:topic:dome-connections"),
    }
}

fn instance(keys: &KukuriKeys, id: &str) -> DomeInstanceManifestV1 {
    let owner = keys.public_key();
    DomeInstanceManifestV1 {
        instance_id: id.into(),
        spatial_context: context(),
        owner_pubkey: owner.clone(),
        preset_ref: DomePresetRefV1 {
            preset_id: format!("preset-{id}"),
            owner_pubkey: owner,
            revision: 1,
            manifest_blob_hash: "a".repeat(64),
            manifest_mime: DOME_PRESET_MANIFEST_MIME.into(),
            manifest_bytes: 1,
        },
        title: id.into(),
        description: String::new(),
        max_peers: Some(8),
        default_spawn: MetaverseRoomSpawnV1 {
            position: [0, 0, 260],
            rotation: [0, 180, 0],
        },
        generation: 1,
        status: DomeInstanceStatusV1::Active,
        relationship_detach: None,
        replacement_instance_id: None,
        chat_history: Vec::new(),
        updated_at: 1,
    }
}

fn agreement(
    id: &str,
    proposal_id: &str,
    proposer: &DomeInstanceManifestV1,
    proposer_direction: DomeDirection,
    receiver: &DomeInstanceManifestV1,
    activation_generation: u64,
) -> DomeConnectionAgreementV1 {
    DomeConnectionAgreementV1 {
        connection_id: id.into(),
        proposal_id: proposal_id.into(),
        spatial_context: proposer.spatial_context.clone(),
        proposer: DomeConnectionEndpointV1::from_instance(proposer, proposer_direction),
        receiver: DomeConnectionEndpointV1::from_instance(
            receiver,
            opposite_dome_direction(proposer_direction),
        ),
        activation_generation,
    }
}

fn active(agreement: DomeConnectionAgreementV1) -> DomeConnectionRecordV1 {
    DomeConnectionRecordV1 {
        agreement,
        receiver_slot_generation: 1,
        observed_active_connection_ids: Vec::new(),
        status: DomeConnectionStatusV1::Active,
        lifecycle_generation: 1,
        lifecycle_actor: None,
        lifecycle_reason: None,
        lifecycle_deadline_at: None,
    }
}

#[test]
fn dome_directions_have_fixed_opposites() {
    assert_eq!(
        opposite_dome_direction(DomeDirection::North),
        DomeDirection::South
    );
    assert_eq!(
        opposite_dome_direction(DomeDirection::East),
        DomeDirection::West
    );
    assert_eq!(
        opposite_dome_direction(DomeDirection::South),
        DomeDirection::North
    );
    assert_eq!(
        opposite_dome_direction(DomeDirection::West),
        DomeDirection::East
    );
}

#[test]
fn draining_requires_deadline_and_keeps_topology_until_terminal_revoke() {
    let proposer_keys = generate_keys();
    let receiver_keys = generate_keys();
    let proposer = instance(&proposer_keys, "dome-a");
    let receiver = instance(&receiver_keys, "dome-b");
    let mut connection = active(agreement(
        "connection-ab",
        "proposal-ab",
        &proposer,
        DomeDirection::East,
        &receiver,
        1,
    ));
    connection.status = DomeConnectionStatusV1::Draining;
    connection.lifecycle_generation = 2;
    connection.lifecycle_actor = Some(proposer.owner_pubkey.clone());
    connection.lifecycle_reason = Some(DomeConnectionTerminalReasonV1::OwnerRevoked);
    connection.lifecycle_deadline_at = Some(4_000);
    validate_dome_connection_record(&connection).unwrap();
    assert_eq!(
        resolve_dome_topology(&[proposer.clone(), receiver.clone()], &[connection.clone()])
            .unwrap()
            .components
            .len(),
        1
    );

    connection.lifecycle_deadline_at = None;
    assert!(validate_dome_connection_record(&connection).is_err());
    connection.status = DomeConnectionStatusV1::Revoked;
    validate_dome_connection_record(&connection).unwrap();
    assert_eq!(
        resolve_dome_topology(&[proposer, receiver], &[connection])
            .unwrap()
            .components
            .len(),
        2
    );
}

#[test]
fn connection_agreement_requires_two_owner_signatures_over_identical_content() {
    let proposer_keys = generate_keys();
    let receiver_keys = generate_keys();
    let proposer = instance(&proposer_keys, "dome-a");
    let receiver = instance(&receiver_keys, "dome-b");
    let agreement = agreement(
        "connection-ab",
        "proposal-ab",
        &proposer,
        DomeDirection::East,
        &receiver,
        1,
    );

    let signed =
        build_signed_dome_connection_agreement(&proposer_keys, &receiver_keys, agreement.clone())
            .expect("signed agreement");
    verify_signed_dome_connection_agreement(&signed).expect("valid signatures");

    let mut tampered = signed;
    tampered.agreement.receiver.direction = DomeDirection::North;
    assert!(verify_signed_dome_connection_agreement(&tampered).is_err());
}

#[test]
fn topology_is_component_local_and_input_order_independent() {
    let a = instance(&generate_keys(), "a");
    let b = instance(&generate_keys(), "b");
    let c = instance(&generate_keys(), "c");
    let ab = active(agreement(
        "ab",
        "proposal-ab",
        &a,
        DomeDirection::East,
        &b,
        1,
    ));
    let bc = active(agreement(
        "bc",
        "proposal-bc",
        &c,
        DomeDirection::South,
        &b,
        2,
    ));

    let first = resolve_dome_topology(
        &[a.clone(), b.clone(), c.clone()],
        &[ab.clone(), bc.clone()],
    )
    .expect("topology");
    let reversed = resolve_dome_topology(&[c, b, a], &[bc, ab]).expect("reversed topology");

    assert_eq!(first, reversed);
    assert_eq!(first.components.len(), 1);
    let component = &first.components[0];
    assert_eq!(component.root_instance_id, "a");
    assert_eq!(component.coordinates_cm.get("a"), Some(&[0, 0, 0]));
    assert_eq!(component.coordinates_cm.get("b"), Some(&[5_700, 0, 0]));
    assert_eq!(component.coordinates_cm.get("c"), Some(&[5_700, 0, -5_700]));
}

#[test]
fn topology_rejects_component_merge_cycle_and_coordinate_collision() {
    let a = instance(&generate_keys(), "a");
    let b = instance(&generate_keys(), "b");
    let c = instance(&generate_keys(), "c");
    let d = instance(&generate_keys(), "d");
    let ab = active(agreement("ab", "p-ab", &a, DomeDirection::East, &b, 1));
    let cd = active(agreement("cd", "p-cd", &c, DomeDirection::East, &d, 1));
    let bc = active(agreement("bc", "p-bc", &b, DomeDirection::South, &c, 2));
    assert!(
        resolve_dome_topology(
            &[a.clone(), b.clone(), c.clone(), d.clone()],
            &[ab.clone(), cd, bc]
        )
        .is_err()
    );

    let ba = active(agreement("ba", "p-ba", &b, DomeDirection::West, &a, 2));
    assert!(resolve_dome_topology(&[a.clone(), b.clone()], &[ab.clone(), ba]).is_err());

    let c_to_b = active(agreement("cb", "p-cb", &c, DomeDirection::West, &b, 2));
    let d_to_c = active(agreement("dc", "p-dc", &d, DomeDirection::South, &c, 3));
    let a_to_d = active(agreement("ad", "p-ad", &a, DomeDirection::South, &d, 4));
    assert!(resolve_dome_topology(&[a, b, c, d], &[ab, c_to_b, d_to_c, a_to_d]).is_err());
}

#[test]
fn revoke_splits_components_without_deleting_either_side() {
    let a = instance(&generate_keys(), "a");
    let b = instance(&generate_keys(), "b");
    let c = instance(&generate_keys(), "c");
    let ab = active(agreement("ab", "p-ab", &a, DomeDirection::East, &b, 1));
    let mut bc = active(agreement("bc", "p-bc", &b, DomeDirection::East, &c, 2));
    bc.status = DomeConnectionStatusV1::Revoked;
    bc.lifecycle_generation = 2;
    bc.lifecycle_actor = Some(b.owner_pubkey.clone());
    bc.lifecycle_reason = Some(DomeConnectionTerminalReasonV1::OwnerRevoked);

    let topology = resolve_dome_topology(&[a, b, c], &[ab, bc]).expect("split topology");
    assert_eq!(topology.components.len(), 2);
    assert!(
        topology
            .components
            .iter()
            .any(|component| { component.instance_ids == vec!["a".to_string(), "b".to_string()] })
    );
    assert!(
        topology
            .components
            .iter()
            .any(|component| { component.instance_ids == vec!["c".to_string()] })
    );
}

#[test]
fn concurrent_same_slot_candidates_have_one_deterministic_winner() {
    let a = instance(&generate_keys(), "a");
    let b = instance(&generate_keys(), "b");
    let c = instance(&generate_keys(), "c");
    let ab = active(agreement(
        "connection-ab",
        "proposal-ab",
        &a,
        DomeDirection::East,
        &b,
        1,
    ));
    let ac = active(agreement(
        "connection-ac",
        "proposal-ac",
        &a,
        DomeDirection::East,
        &c,
        1,
    ));

    let first = resolve_dome_topology_candidates(
        &[a.clone(), b.clone(), c.clone()],
        &[ab.clone(), ac.clone()],
    )
    .expect("resolve concurrent candidates");
    let reversed = resolve_dome_topology_candidates(&[c, b, a], &[ac, ab])
        .expect("resolve reversed concurrent candidates");
    assert_eq!(first, reversed);
    assert_eq!(first.topology.active_connection_ids.len(), 1);
    assert_eq!(first.rejected_connections.len(), 1);
}

#[test]
fn observed_active_predecessor_is_not_reordered_by_a_later_candidate() {
    let a = instance(&generate_keys(), "a");
    let b = instance(&generate_keys(), "b");
    let c = instance(&generate_keys(), "c");
    let predecessor = active(agreement(
        "predecessor",
        "proposal-ab",
        &a,
        DomeDirection::East,
        &b,
        9,
    ));
    let mut later = active(agreement(
        "later",
        "proposal-ac",
        &a,
        DomeDirection::East,
        &c,
        1,
    ));
    later.observed_active_connection_ids = vec!["predecessor".into()];

    let resolution = resolve_dome_topology_candidates(&[a, b, c], &[later, predecessor])
        .expect("resolve causal candidates");
    assert_eq!(
        resolution.topology.active_connection_ids,
        vec!["predecessor".to_string()]
    );
    assert_eq!(resolution.rejected_connections[0].connection_id, "later");
}

#[test]
fn proposal_status_distinguishes_receiver_waiting_and_proposer_discard() {
    let proposer = instance(&generate_keys(), "a");
    let receiver = instance(&generate_keys(), "b");
    let other = instance(&generate_keys(), "c");
    let proposal = DomeConnectionProposalV1 {
        proposal_id: "proposal-target".into(),
        spatial_context: context(),
        proposer: DomeConnectionEndpointV1::from_instance(&proposer, DomeDirection::East),
        receiver: DomeConnectionEndpointV1::from_instance(&receiver, DomeDirection::West),
        sequence: 1,
        created_at: 1,
    };
    let receiver_occupied = active(agreement(
        "receiver-occupied",
        "other-receiver-proposal",
        &other,
        DomeDirection::East,
        &receiver,
        1,
    ));
    assert_eq!(
        derive_dome_proposal_status(&proposal, None, &[receiver_occupied], None),
        DomeProposalDerivedStatusV1::WaitingForSlot
    );

    let proposer_occupied = active(agreement(
        "proposer-occupied",
        "other-proposer-proposal",
        &proposer,
        DomeDirection::East,
        &other,
        1,
    ));
    assert_eq!(
        derive_dome_proposal_status(&proposal, None, &[proposer_occupied], None),
        DomeProposalDerivedStatusV1::Discarded
    );
    assert_eq!(
        derive_dome_proposal_status(
            &proposal,
            None,
            &[],
            Some(DomeConnectionTerminalReasonV1::OwnersBlocked)
        ),
        DomeProposalDerivedStatusV1::Discarded
    );
}
