use crate::*;

#[test]
fn host_liveness_uses_five_second_offline_and_fifteen_second_close_boundaries() {
    assert_eq!(
        resolve_dome_host_liveness(Some(1_000), 6_000),
        DomeHostLivenessV1::Online
    );
    assert_eq!(
        resolve_dome_host_liveness(Some(1_000), 6_001),
        DomeHostLivenessV1::OfflineGrace
    );
    assert_eq!(
        resolve_dome_host_liveness(Some(1_000), 16_000),
        DomeHostLivenessV1::OfflineGrace
    );
    assert_eq!(
        resolve_dome_host_liveness(Some(1_000), 16_001),
        DomeHostLivenessV1::Closed
    );
    assert_eq!(
        resolve_dome_host_liveness(None, 1_000),
        DomeHostLivenessV1::OfflineGrace
    );
}

#[test]
fn evacuation_candidates_are_safe_ranked_stable_and_exclude_current() {
    let candidates = vec![
        DomeEvacuationCandidateV1 {
            instance_id: "current".into(),
            kind: DomeEvacuationCandidateKindV1::OwnHosted,
            available: true,
        },
        DomeEvacuationCandidateV1 {
            instance_id: "fallback-b".into(),
            kind: DomeEvacuationCandidateKindV1::StableFallback,
            available: true,
        },
        DomeEvacuationCandidateV1 {
            instance_id: "adjacent".into(),
            kind: DomeEvacuationCandidateKindV1::ActiveAdjacent,
            available: true,
        },
        DomeEvacuationCandidateV1 {
            instance_id: "adjacent".into(),
            kind: DomeEvacuationCandidateKindV1::LastVisited,
            available: true,
        },
        DomeEvacuationCandidateV1 {
            instance_id: "unavailable".into(),
            kind: DomeEvacuationCandidateKindV1::EstablishedTransition,
            available: false,
        },
        DomeEvacuationCandidateV1 {
            instance_id: "home".into(),
            kind: DomeEvacuationCandidateKindV1::OwnHosted,
            available: true,
        },
        DomeEvacuationCandidateV1 {
            instance_id: "fallback-a".into(),
            kind: DomeEvacuationCandidateKindV1::StableFallback,
            available: true,
        },
    ];
    let ids = order_dome_evacuation_candidates("current", &candidates)
        .into_iter()
        .map(|candidate| candidate.instance_id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["adjacent", "home", "fallback-a", "fallback-b"]);
}

#[test]
fn recovery_reason_codes_are_stable() {
    assert_eq!(
        DomeEvacuationReasonV1::HostOffline.code(),
        "DOME_EVACUATION_HOST_OFFLINE"
    );
    assert_eq!(
        DomeEvacuationReasonV1::AccessRevoked.code(),
        "DOME_EVACUATION_ACCESS_REVOKED"
    );
    assert_eq!(
        DomeEvacuationReasonV1::Blocked.code(),
        "DOME_EVACUATION_BLOCKED"
    );
    assert_eq!(
        DomeEvacuationReasonV1::TopologyInvalid.code(),
        "DOME_EVACUATION_TOPOLOGY_INVALID"
    );
    assert_eq!(
        DomeEvacuationReasonV1::UserRequested.code(),
        "DOME_EVACUATION_USER_REQUESTED"
    );
}
