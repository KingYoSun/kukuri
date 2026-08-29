use crate::{
    DomeHostHeartbeatV1, DomeHostTargetV1, DomeHostingLeaseV1, DomeHostingRecordV1,
    DomeHostingStateKindV1, DomeInstanceManifestV1, DomeInstanceStatusV1, DomeLayoutCandidateV1,
    DomeLayoutCommitV1, DomePhysicsSnapshotV1, DomePresetRefV1, KukuriKeys, MetaverseRoomSpawnV1,
    SpatialContextV1, TopicId, accept_dome_hosting_lease, activate_dome_hosting_lease,
    build_signed_dome_host_heartbeat, build_signed_dome_hosting_lease,
    build_signed_dome_layout_candidate, build_signed_dome_layout_commit,
    build_signed_dome_physics_snapshot, close_dome_hosting_lease, dome_layout_candidate_digest,
    resolve_dome_hosting_state, verify_signed_dome_host_heartbeat,
    verify_signed_dome_layout_commit, verify_signed_dome_physics_snapshot,
};

fn instance(owner: &KukuriKeys) -> DomeInstanceManifestV1 {
    DomeInstanceManifestV1 {
        instance_id: "dome-1".into(),
        spatial_context: SpatialContextV1::Topic {
            topic_id: TopicId("kukuri:topic:hosting".into()),
        },
        owner_pubkey: owner.public_key(),
        preset_ref: DomePresetRefV1 {
            preset_id: "preset-1".into(),
            owner_pubkey: owner.public_key(),
            revision: 1,
            manifest_blob_hash: "manifest-hash-1".into(),
            manifest_mime: "application/vnd.kukuri.dome-preset+json".into(),
            manifest_bytes: 64,
        },
        title: "Hosted Dome".into(),
        description: String::new(),
        max_peers: Some(8),
        default_spawn: MetaverseRoomSpawnV1 {
            position: [0, 0, 0],
            rotation: [0, 0, 0],
        },
        generation: 3,
        status: DomeInstanceStatusV1::Active,
        relationship_detach: None,
        replacement_instance_id: None,
        chat_history: Vec::new(),
        updated_at: 100,
    }
}

#[test]
fn heartbeat_signature_is_bound_to_host_lease_epoch_session_and_sequence() {
    let owner = KukuriKeys::generate();
    let host = KukuriKeys::generate();
    let lease = lease(&owner, &host, 1);
    let signed = build_signed_dome_host_heartbeat(
        &host,
        &lease,
        DomeHostHeartbeatV1 {
            instance_id: lease.instance_id.clone(),
            instance_generation: lease.instance_generation,
            lease_epoch: lease.epoch,
            session_id: "session-1".into(),
            host_pubkey: host.public_key(),
            participants: 2,
            sleeping: false,
            sequence: 7,
            sent_at: 2_500,
        },
    )
    .unwrap();
    verify_signed_dome_host_heartbeat(&signed, &lease, "session-1").unwrap();

    let mut tampered = signed;
    tampered.heartbeat.sequence += 1;
    assert!(verify_signed_dome_host_heartbeat(&tampered, &lease, "session-1").is_err());
}

fn lease(owner: &KukuriKeys, host: &KukuriKeys, epoch: u64) -> DomeHostingLeaseV1 {
    let instance = instance(owner);
    DomeHostingLeaseV1 {
        lease_id: format!("lease-{epoch}"),
        spatial_context: instance.spatial_context,
        instance_id: instance.instance_id,
        instance_generation: instance.generation,
        owner_pubkey: owner.public_key(),
        host: DomeHostTargetV1::OwnerDevice {
            endpoint_id: "endpoint-owner".into(),
            host_pubkey: host.public_key(),
        },
        manifest_blob_hash: instance.preset_ref.manifest_blob_hash,
        manifest_version: instance.preset_ref.revision,
        epoch,
        issued_at: 1_000,
        expires_at: 10_000,
    }
}

#[test]
fn owner_signed_lease_requires_explicit_host_acceptance_and_activation() {
    let owner = KukuriKeys::generate();
    let host = KukuriKeys::generate();
    let instance = instance(&owner);
    let signed = build_signed_dome_hosting_lease(&owner, lease(&owner, &host, 1)).unwrap();

    let transferring = resolve_dome_hosting_state(
        &instance,
        &[DomeHostingRecordV1::LeaseIssued(signed.clone())],
        2_000,
        None,
    )
    .unwrap();
    assert_eq!(transferring.kind, DomeHostingStateKindV1::Transferring);

    let acceptance = accept_dome_hosting_lease(&host, &signed, "session-1", 2_100).unwrap();
    let activation = activate_dome_hosting_lease(&owner, &signed, &acceptance, 2_200).unwrap();
    let state = resolve_dome_hosting_state(
        &instance,
        &[
            DomeHostingRecordV1::LeaseIssued(signed),
            DomeHostingRecordV1::HostAccepted(acceptance),
            DomeHostingRecordV1::LeaseActivated(activation),
        ],
        2_300,
        Some(2_250),
    )
    .unwrap();
    assert_eq!(state.kind, DomeHostingStateKindV1::OwnerHosted);
    assert_eq!(state.session_id.as_deref(), Some("session-1"));
}

#[test]
fn higher_epoch_fences_old_host_and_same_epoch_conflict_fails_closed() {
    let owner = KukuriKeys::generate();
    let old_host = KukuriKeys::generate();
    let new_host = KukuriKeys::generate();
    let instance = instance(&owner);
    let first = build_signed_dome_hosting_lease(&owner, lease(&owner, &old_host, 1)).unwrap();
    let second = build_signed_dome_hosting_lease(&owner, lease(&owner, &new_host, 2)).unwrap();

    let state = resolve_dome_hosting_state(
        &instance,
        &[
            DomeHostingRecordV1::LeaseIssued(first),
            DomeHostingRecordV1::LeaseIssued(second.clone()),
        ],
        2_000,
        None,
    )
    .unwrap();
    assert_eq!(state.kind, DomeHostingStateKindV1::Transferring);
    assert_eq!(state.lease_epoch, Some(2));

    let mut conflicting = lease(&owner, &new_host, 2);
    conflicting.lease_id = "lease-conflict".into();
    let conflicting = build_signed_dome_hosting_lease(&owner, conflicting).unwrap();
    let state = resolve_dome_hosting_state(
        &instance,
        &[
            DomeHostingRecordV1::LeaseIssued(second),
            DomeHostingRecordV1::LeaseIssued(conflicting),
        ],
        2_000,
        None,
    )
    .unwrap();
    assert_eq!(state.kind, DomeHostingStateKindV1::GracePeriod);
    assert_eq!(state.reason.as_deref(), Some("split_brain"));
}

#[test]
fn close_and_expiry_are_closed() {
    let owner = KukuriKeys::generate();
    let host = KukuriKeys::generate();
    let instance = instance(&owner);
    let signed = build_signed_dome_hosting_lease(&owner, lease(&owner, &host, 1)).unwrap();
    let acceptance = accept_dome_hosting_lease(&host, &signed, "session-1", 2_100).unwrap();
    let activation = activate_dome_hosting_lease(&owner, &signed, &acceptance, 2_200).unwrap();
    let closed = close_dome_hosting_lease(&owner, &signed, 2_300).unwrap();
    let records = vec![
        DomeHostingRecordV1::LeaseIssued(signed.clone()),
        DomeHostingRecordV1::HostAccepted(acceptance),
        DomeHostingRecordV1::LeaseActivated(activation),
        DomeHostingRecordV1::LeaseClosed(closed),
    ];
    assert_eq!(
        resolve_dome_hosting_state(&instance, &records, 2_400, None)
            .unwrap()
            .kind,
        DomeHostingStateKindV1::Closed
    );
    assert_eq!(
        resolve_dome_hosting_state(
            &instance,
            &[DomeHostingRecordV1::LeaseIssued(signed)],
            10_001,
            None,
        )
        .unwrap()
        .kind,
        DomeHostingStateKindV1::Closed
    );
}

#[test]
fn higher_epoch_rejects_old_host_snapshot() {
    let owner = KukuriKeys::generate();
    let old_host = KukuriKeys::generate();
    let new_host = KukuriKeys::generate();
    let old_lease = lease(&owner, &old_host, 1);
    let new_lease = lease(&owner, &new_host, 2);
    let snapshot = build_signed_dome_physics_snapshot(
        &old_host,
        &old_lease,
        DomePhysicsSnapshotV1 {
            instance_id: old_lease.instance_id.clone(),
            instance_generation: old_lease.instance_generation,
            lease_epoch: old_lease.epoch,
            session_id: "old-session".into(),
            host_pubkey: old_host.public_key(),
            sequence: 1,
            simulated_at: 2_000,
            sleeping: false,
            bodies: Vec::new(),
        },
    )
    .unwrap();
    verify_signed_dome_physics_snapshot(&snapshot, &old_lease, "old-session").unwrap();
    assert!(verify_signed_dome_physics_snapshot(&snapshot, &new_lease, "new-session").is_err());
}

#[test]
fn layout_candidate_requires_active_host_and_commit_requires_owner() {
    let owner = KukuriKeys::generate();
    let host = KukuriKeys::generate();
    let attacker = KukuriKeys::generate();
    let lease = lease(&owner, &host, 1);
    let candidate = DomeLayoutCandidateV1 {
        operation_id: "layout-1".into(),
        instance_id: lease.instance_id.clone(),
        instance_generation: lease.instance_generation,
        lease_epoch: lease.epoch,
        session_id: "session-1".into(),
        host_pubkey: host.public_key(),
        base_manifest_revision: lease.manifest_version,
        snapshot_sequence: 1,
        captured_at: 2_000,
        persistent_props: Vec::new(),
    };
    assert!(build_signed_dome_layout_candidate(&attacker, &lease, candidate.clone()).is_err());
    let candidate = build_signed_dome_layout_candidate(&host, &lease, candidate).unwrap();
    let commit = DomeLayoutCommitV1 {
        operation_id: candidate.candidate.operation_id.clone(),
        instance_id: lease.instance_id.clone(),
        instance_generation: lease.instance_generation,
        owner_pubkey: owner.public_key(),
        base_manifest_revision: lease.manifest_version,
        next_manifest_revision: lease.manifest_version + 1,
        candidate_digest: dome_layout_candidate_digest(&candidate.candidate).unwrap(),
        manifest_blob_hash: "next-manifest-hash".into(),
        committed_at: 2_100,
    };
    assert!(
        build_signed_dome_layout_commit(&attacker, &lease, &candidate, commit.clone()).is_err()
    );
    let signed = build_signed_dome_layout_commit(&owner, &lease, &candidate, commit).unwrap();
    verify_signed_dome_layout_commit(&signed, &lease, &candidate).unwrap();
}
