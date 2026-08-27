use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    DomeDirection, DomeInstanceManifestV1, DomeInstanceStatusV1, KukuriEnvelope, KukuriKeys,
    Pubkey, SpatialContextV1, fixed_dome_v1, sign_envelope_json, validate_dome_instance_manifest,
    validate_dome_relationship_scope,
};

pub const DOME_CONNECTION_MAX_OPEN_OUTBOUND: usize = 32;
pub const DOME_CONNECTION_MAX_PER_PEER_SLOT: usize = 4;
pub const DOME_CONNECTION_MAX_RECEIVER_QUEUE: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeConnectionEndpointV1 {
    pub instance_id: String,
    pub instance_generation: u64,
    pub owner_pubkey: Pubkey,
    pub direction: DomeDirection,
}

impl DomeConnectionEndpointV1 {
    pub fn from_instance(instance: &DomeInstanceManifestV1, direction: DomeDirection) -> Self {
        Self {
            instance_id: instance.instance_id.clone(),
            instance_generation: instance.generation,
            owner_pubkey: instance.owner_pubkey.clone(),
            direction,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeConnectionProposalV1 {
    pub proposal_id: String,
    pub spatial_context: SpatialContextV1,
    pub proposer: DomeConnectionEndpointV1,
    pub receiver: DomeConnectionEndpointV1,
    pub sequence: u64,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeProposalSelectionV1 {
    pub selection_id: String,
    pub proposal_id: String,
    pub spatial_context: SpatialContextV1,
    pub receiver: DomeConnectionEndpointV1,
    pub slot_generation: u64,
    pub observed_active_connection_ids: Vec<String>,
    pub selected_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeConnectionAgreementV1 {
    pub connection_id: String,
    pub proposal_id: String,
    pub spatial_context: SpatialContextV1,
    pub proposer: DomeConnectionEndpointV1,
    pub receiver: DomeConnectionEndpointV1,
    pub activation_generation: u64,
}

impl DomeConnectionAgreementV1 {
    pub fn from_proposal(
        connection_id: impl Into<String>,
        proposal: &DomeConnectionProposalV1,
    ) -> Self {
        Self {
            connection_id: connection_id.into(),
            proposal_id: proposal.proposal_id.clone(),
            spatial_context: proposal.spatial_context.clone(),
            proposer: proposal.proposer.clone(),
            receiver: proposal.receiver.clone(),
            activation_generation: proposal.sequence,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedDomeConnectionAgreementV1 {
    pub agreement: DomeConnectionAgreementV1,
    pub proposer_signature: KukuriEnvelope,
    pub receiver_signature: KukuriEnvelope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DomeConnectionStatusV1 {
    Accepted,
    Active,
    Draining,
    Revoked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DomeConnectionTerminalReasonV1 {
    OwnerRevoked,
    ProposerWithdrew,
    ProposerSlotOccupied,
    InstanceDetached,
    InstanceDeleted,
    OwnersBlocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeConnectionRecordV1 {
    pub agreement: DomeConnectionAgreementV1,
    pub receiver_slot_generation: u64,
    pub observed_active_connection_ids: Vec<String>,
    pub status: DomeConnectionStatusV1,
    pub lifecycle_generation: u64,
    pub lifecycle_actor: Option<Pubkey>,
    pub lifecycle_reason: Option<DomeConnectionTerminalReasonV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DomeProposalDerivedStatusV1 {
    Proposed,
    Reserved,
    Accepted,
    WaitingForSlot,
    Discarded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeComponentTopologyV1 {
    pub root_instance_id: String,
    pub instance_ids: Vec<String>,
    pub connection_ids: Vec<String>,
    pub coordinates_cm: BTreeMap<String, [i64; 3]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeTopologyV1 {
    pub spatial_context: SpatialContextV1,
    pub components: Vec<DomeComponentTopologyV1>,
    pub active_connection_ids: Vec<String>,
    pub topology_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeRejectedConnectionV1 {
    pub connection_id: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeTopologyResolutionV1 {
    pub topology: DomeTopologyV1,
    pub rejected_connections: Vec<DomeRejectedConnectionV1>,
}

pub const fn opposite_dome_direction(direction: DomeDirection) -> DomeDirection {
    match direction {
        DomeDirection::North => DomeDirection::South,
        DomeDirection::East => DomeDirection::West,
        DomeDirection::South => DomeDirection::North,
        DomeDirection::West => DomeDirection::East,
    }
}

pub fn validate_dome_connection_proposal(
    proposal: &DomeConnectionProposalV1,
    proposer: &DomeInstanceManifestV1,
    receiver: &DomeInstanceManifestV1,
) -> Result<()> {
    if !is_valid_dome_record_id(&proposal.proposal_id, 128) || proposal.sequence == 0 {
        bail!("Dome Connection proposal identity is incomplete");
    }
    validate_endpoint_pair(
        &proposal.spatial_context,
        &proposal.proposer,
        &proposal.receiver,
        proposer,
        receiver,
    )
}

pub fn validate_dome_connection_selection(
    selection: &DomeProposalSelectionV1,
    proposal: &DomeConnectionProposalV1,
) -> Result<()> {
    if !is_valid_dome_record_id(&selection.selection_id, 192)
        || selection.proposal_id != proposal.proposal_id
        || selection.slot_generation == 0
        || selection.spatial_context != proposal.spatial_context
        || selection.receiver != proposal.receiver
    {
        bail!("Dome Connection selection does not match proposal");
    }
    validate_unique_ids(&selection.observed_active_connection_ids)
}

pub fn validate_dome_connection_agreement(
    agreement: &DomeConnectionAgreementV1,
    proposer: &DomeInstanceManifestV1,
    receiver: &DomeInstanceManifestV1,
) -> Result<()> {
    if !is_valid_dome_record_id(&agreement.connection_id, 160)
        || !is_valid_dome_record_id(&agreement.proposal_id, 128)
        || agreement.activation_generation == 0
    {
        bail!("Dome Connection agreement identity is incomplete");
    }
    validate_endpoint_pair(
        &agreement.spatial_context,
        &agreement.proposer,
        &agreement.receiver,
        proposer,
        receiver,
    )
}

pub fn validate_dome_connection_record(record: &DomeConnectionRecordV1) -> Result<()> {
    if record.lifecycle_generation == 0 || record.receiver_slot_generation == 0 {
        bail!("Dome Connection lifecycle generations are required");
    }
    validate_unique_ids(&record.observed_active_connection_ids)?;
    match record.status {
        DomeConnectionStatusV1::Accepted | DomeConnectionStatusV1::Active => {
            if record.lifecycle_reason.is_some() {
                bail!("non-terminal Dome Connection cannot have a terminal reason");
            }
        }
        DomeConnectionStatusV1::Draining | DomeConnectionStatusV1::Revoked => {
            let actor = record
                .lifecycle_actor
                .as_ref()
                .context("Dome Connection terminal lifecycle requires an actor")?;
            if actor != &record.agreement.proposer.owner_pubkey
                && actor != &record.agreement.receiver.owner_pubkey
            {
                bail!("Dome Connection lifecycle actor is not an endpoint owner");
            }
            if record.lifecycle_reason.is_none() {
                bail!("Dome Connection terminal lifecycle requires a reason");
            }
        }
    }
    Ok(())
}

pub fn build_dome_connection_proposal_envelope(
    keys: &KukuriKeys,
    proposal: &DomeConnectionProposalV1,
) -> Result<KukuriEnvelope> {
    if proposal.proposer.owner_pubkey != keys.public_key() {
        bail!("Dome Connection proposer must match signer");
    }
    sign_envelope_json(
        keys,
        "dome-connection-proposal",
        connection_tags(
            proposal.proposer.owner_pubkey.as_str(),
            proposal.proposal_id.as_str(),
            proposal.spatial_context.canonical_id(),
        ),
        proposal,
    )
}

pub fn build_dome_connection_selection_envelope(
    keys: &KukuriKeys,
    selection: &DomeProposalSelectionV1,
) -> Result<KukuriEnvelope> {
    if selection.receiver.owner_pubkey != keys.public_key() {
        bail!("Dome Connection receiver must match signer");
    }
    sign_envelope_json(
        keys,
        "dome-connection-selection",
        connection_tags(
            selection.receiver.owner_pubkey.as_str(),
            selection.proposal_id.as_str(),
            selection.spatial_context.canonical_id(),
        ),
        selection,
    )
}

pub fn build_signed_dome_connection_agreement(
    proposer_keys: &KukuriKeys,
    receiver_keys: &KukuriKeys,
    agreement: DomeConnectionAgreementV1,
) -> Result<SignedDomeConnectionAgreementV1> {
    if agreement.proposer.owner_pubkey != proposer_keys.public_key()
        || agreement.receiver.owner_pubkey != receiver_keys.public_key()
    {
        bail!("Dome Connection agreement signers do not match endpoint owners");
    }
    let proposer_signature = build_dome_connection_agreement_envelope(proposer_keys, &agreement)?;
    let receiver_signature = build_dome_connection_agreement_envelope(receiver_keys, &agreement)?;
    Ok(SignedDomeConnectionAgreementV1 {
        agreement,
        proposer_signature,
        receiver_signature,
    })
}

pub fn build_dome_connection_agreement_envelope(
    keys: &KukuriKeys,
    agreement: &DomeConnectionAgreementV1,
) -> Result<KukuriEnvelope> {
    let signer = keys.public_key();
    if signer != agreement.proposer.owner_pubkey && signer != agreement.receiver.owner_pubkey {
        bail!("Dome Connection agreement signer is not an endpoint owner");
    }
    sign_envelope_json(
        keys,
        "dome-connection-agreement",
        connection_tags(
            signer.as_str(),
            agreement.connection_id.as_str(),
            agreement.spatial_context.canonical_id(),
        ),
        agreement,
    )
}

pub fn verify_signed_dome_connection_agreement(
    signed: &SignedDomeConnectionAgreementV1,
) -> Result<()> {
    for (envelope, owner) in [
        (
            &signed.proposer_signature,
            &signed.agreement.proposer.owner_pubkey,
        ),
        (
            &signed.receiver_signature,
            &signed.agreement.receiver.owner_pubkey,
        ),
    ] {
        envelope.verify()?;
        if envelope.kind != "dome-connection-agreement" || &envelope.pubkey != owner {
            bail!("Dome Connection agreement signature owner mismatch");
        }
        let content: DomeConnectionAgreementV1 = serde_json::from_str(envelope.content.as_str())?;
        if content != signed.agreement {
            bail!("Dome Connection signatures must cover identical agreement content");
        }
    }
    Ok(())
}

pub fn derive_dome_proposal_status(
    proposal: &DomeConnectionProposalV1,
    selection: Option<&DomeProposalSelectionV1>,
    connections: &[DomeConnectionRecordV1],
    terminal_reason: Option<DomeConnectionTerminalReasonV1>,
) -> DomeProposalDerivedStatusV1 {
    if terminal_reason.is_some()
        || connections.iter().any(|connection| {
            is_topology_active(connection)
                && connection.agreement.proposer.instance_id == proposal.proposer.instance_id
                && connection.agreement.proposer.direction == proposal.proposer.direction
                && connection.agreement.proposal_id != proposal.proposal_id
        })
    {
        return DomeProposalDerivedStatusV1::Discarded;
    }
    if connections.iter().any(|connection| {
        is_topology_active(connection)
            && connection.agreement.receiver.instance_id == proposal.receiver.instance_id
            && connection.agreement.receiver.direction == proposal.receiver.direction
            && connection.agreement.proposal_id != proposal.proposal_id
    }) {
        return DomeProposalDerivedStatusV1::WaitingForSlot;
    }
    if connections.iter().any(|connection| {
        connection.agreement.proposal_id == proposal.proposal_id
            && matches!(
                connection.status,
                DomeConnectionStatusV1::Accepted
                    | DomeConnectionStatusV1::Active
                    | DomeConnectionStatusV1::Draining
            )
    }) {
        return DomeProposalDerivedStatusV1::Accepted;
    }
    if selection.is_some_and(|selection| selection.proposal_id == proposal.proposal_id) {
        DomeProposalDerivedStatusV1::Reserved
    } else {
        DomeProposalDerivedStatusV1::Proposed
    }
}

pub fn resolve_dome_topology(
    instances: &[DomeInstanceManifestV1],
    connections: &[DomeConnectionRecordV1],
) -> Result<DomeTopologyV1> {
    let mut instance_by_id = BTreeMap::new();
    let mut spatial_context = None;
    for instance in instances {
        validate_dome_instance_manifest(instance)?;
        if instance.status != DomeInstanceStatusV1::Active || instance.relationship_detach.is_some()
        {
            bail!("Dome topology requires active attached instances");
        }
        if let Some(context) = &spatial_context {
            if context != &instance.spatial_context {
                bail!("Dome topology instances must share one Spatial Context");
            }
        } else {
            spatial_context = Some(instance.spatial_context.clone());
        }
        if instance_by_id
            .insert(instance.instance_id.clone(), instance)
            .is_some()
        {
            bail!("duplicate Dome instance in topology input");
        }
    }
    let spatial_context =
        spatial_context.context("Dome topology requires at least one instance")?;

    let active = ordered_topology_connections(connections);

    let mut seen_connection_ids = BTreeSet::new();
    let mut adjacency: BTreeMap<String, Vec<(String, DomeDirection, String)>> = instance_by_id
        .keys()
        .cloned()
        .map(|instance_id| (instance_id, Vec::new()))
        .collect();
    let mut occupied_slots = BTreeSet::new();

    for connection in active {
        validate_dome_connection_record(connection)?;
        let agreement = &connection.agreement;
        if !seen_connection_ids.insert(agreement.connection_id.clone()) {
            bail!("duplicate Dome Connection id");
        }
        let proposer = instance_by_id
            .get(agreement.proposer.instance_id.as_str())
            .context("Dome Connection proposer instance is unavailable")?;
        let receiver = instance_by_id
            .get(agreement.receiver.instance_id.as_str())
            .context("Dome Connection receiver instance is unavailable")?;
        validate_dome_connection_agreement(agreement, proposer, receiver)?;
        let proposer_slot = (
            agreement.proposer.instance_id.clone(),
            agreement.proposer.direction,
        );
        let receiver_slot = (
            agreement.receiver.instance_id.clone(),
            agreement.receiver.direction,
        );
        if !occupied_slots.insert(proposer_slot) || !occupied_slots.insert(receiver_slot) {
            bail!("Dome Connection direction slot is already occupied");
        }

        let proposer_component = reachable_instances(&adjacency, &agreement.proposer.instance_id);
        let receiver_component = reachable_instances(&adjacency, &agreement.receiver.instance_id);
        if proposer_component.contains(&agreement.receiver.instance_id) {
            bail!("Dome Connection would create a cycle");
        }
        if proposer_component.len() > 1 && receiver_component.len() > 1 {
            bail!("Dome Connection cannot merge existing components");
        }

        adjacency
            .get_mut(&agreement.proposer.instance_id)
            .expect("validated proposer")
            .push((
                agreement.receiver.instance_id.clone(),
                agreement.proposer.direction,
                agreement.connection_id.clone(),
            ));
        adjacency
            .get_mut(&agreement.receiver.instance_id)
            .expect("validated receiver")
            .push((
                agreement.proposer.instance_id.clone(),
                agreement.receiver.direction,
                agreement.connection_id.clone(),
            ));

        derive_components(&adjacency)?;
    }

    let components = derive_components(&adjacency)?;
    let active_connection_ids = seen_connection_ids.into_iter().collect::<Vec<_>>();
    let digest_payload = serde_json::to_vec(&serde_json::json!({
        "spatial_context": spatial_context,
        "components": components,
        "active_connection_ids": active_connection_ids,
    }))?;
    let topology_digest = blake3::hash(&digest_payload).to_hex().to_string();
    Ok(DomeTopologyV1 {
        spatial_context,
        components,
        active_connection_ids,
        topology_digest,
    })
}

pub fn resolve_dome_topology_candidates(
    instances: &[DomeInstanceManifestV1],
    connections: &[DomeConnectionRecordV1],
) -> Result<DomeTopologyResolutionV1> {
    let candidates = ordered_topology_connections(connections)
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut accepted = Vec::new();
    let mut rejected_connections = Vec::new();
    for candidate in candidates {
        let mut next = accepted.clone();
        next.push(candidate.clone());
        match resolve_dome_topology(instances, &next) {
            Ok(_) => accepted.push(candidate),
            Err(error) => rejected_connections.push(DomeRejectedConnectionV1 {
                connection_id: candidate.agreement.connection_id,
                reason: error.to_string(),
            }),
        }
    }
    Ok(DomeTopologyResolutionV1 {
        topology: resolve_dome_topology(instances, &accepted)?,
        rejected_connections,
    })
}

fn validate_endpoint_pair(
    spatial_context: &SpatialContextV1,
    proposer_endpoint: &DomeConnectionEndpointV1,
    receiver_endpoint: &DomeConnectionEndpointV1,
    proposer: &DomeInstanceManifestV1,
    receiver: &DomeInstanceManifestV1,
) -> Result<()> {
    validate_dome_relationship_scope(proposer, receiver)?;
    if &proposer.spatial_context != spatial_context
        || &receiver.spatial_context != spatial_context
        || proposer.instance_id != proposer_endpoint.instance_id
        || proposer.generation != proposer_endpoint.instance_generation
        || proposer.owner_pubkey != proposer_endpoint.owner_pubkey
        || receiver.instance_id != receiver_endpoint.instance_id
        || receiver.generation != receiver_endpoint.instance_generation
        || receiver.owner_pubkey != receiver_endpoint.owner_pubkey
    {
        bail!("Dome Connection endpoints do not match current instances");
    }
    if proposer.instance_id == receiver.instance_id
        || proposer.owner_pubkey == receiver.owner_pubkey
    {
        bail!("Dome Connection endpoints must belong to different owners");
    }
    if receiver_endpoint.direction != opposite_dome_direction(proposer_endpoint.direction) {
        bail!("Dome Connection endpoint directions are not opposites");
    }
    Ok(())
}

fn validate_unique_ids(ids: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    if ids
        .iter()
        .any(|id| !is_valid_dome_record_id(id, 160) || !seen.insert(id.as_str()))
    {
        bail!("Dome Connection causal ids must be non-empty and unique");
    }
    Ok(())
}

fn is_valid_dome_record_id(id: &str, max_len: usize) -> bool {
    !id.is_empty()
        && id.len() <= max_len
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn connection_tags(author: &str, object_id: &str, context: String) -> Vec<Vec<String>> {
    vec![
        vec!["author".into(), author.into()],
        vec!["object".into(), "dome-connection".into()],
        vec!["connection".into(), object_id.into()],
        vec!["context".into(), context],
    ]
}

fn is_topology_active(connection: &DomeConnectionRecordV1) -> bool {
    matches!(
        connection.status,
        DomeConnectionStatusV1::Active | DomeConnectionStatusV1::Draining
    )
}

fn ordered_topology_connections(
    connections: &[DomeConnectionRecordV1],
) -> Vec<&DomeConnectionRecordV1> {
    let mut remaining = connections
        .iter()
        .filter(|connection| is_topology_active(connection))
        .collect::<Vec<_>>();
    let candidate_ids = remaining
        .iter()
        .map(|connection| connection.agreement.connection_id.clone())
        .collect::<BTreeSet<_>>();
    let mut emitted = BTreeSet::new();
    let mut ordered = Vec::with_capacity(remaining.len());
    while !remaining.is_empty() {
        let ready = remaining
            .iter()
            .enumerate()
            .filter(|(_, connection)| {
                connection
                    .observed_active_connection_ids
                    .iter()
                    .filter(|id| candidate_ids.contains(*id))
                    .all(|id| emitted.contains(id))
            })
            .min_by(|(_, left), (_, right)| topology_connection_order(left, right))
            .map(|(index, _)| index);
        // A malformed causal cycle must not make peers depend on delivery order. Pick the same
        // digest-ranked record everywhere; validation can still reject it during resolution.
        let index = ready.unwrap_or_else(|| {
            remaining
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| topology_connection_order(left, right))
                .map(|(index, _)| index)
                .expect("remaining topology candidate")
        });
        let connection = remaining.remove(index);
        emitted.insert(connection.agreement.connection_id.clone());
        ordered.push(connection);
    }
    ordered
}

fn topology_connection_order(
    left: &DomeConnectionRecordV1,
    right: &DomeConnectionRecordV1,
) -> std::cmp::Ordering {
    left.agreement
        .activation_generation
        .cmp(&right.agreement.activation_generation)
        .then_with(|| topology_record_digest(left).cmp(&topology_record_digest(right)))
        .then_with(|| {
            left.agreement
                .connection_id
                .cmp(&right.agreement.connection_id)
        })
}

fn topology_record_digest(record: &DomeConnectionRecordV1) -> String {
    let bytes = serde_json::to_vec(record).expect("Dome Connection record is serializable");
    blake3::hash(&bytes).to_hex().to_string()
}

fn reachable_instances(
    adjacency: &BTreeMap<String, Vec<(String, DomeDirection, String)>>,
    start: &str,
) -> BTreeSet<String> {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([start.to_string()]);
    while let Some(instance_id) = queue.pop_front() {
        if !visited.insert(instance_id.clone()) {
            continue;
        }
        if let Some(edges) = adjacency.get(&instance_id) {
            for (neighbor, _, _) in edges {
                if !visited.contains(neighbor) {
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }
    visited
}

fn derive_components(
    adjacency: &BTreeMap<String, Vec<(String, DomeDirection, String)>>,
) -> Result<Vec<DomeComponentTopologyV1>> {
    let mut remaining = adjacency.keys().cloned().collect::<BTreeSet<_>>();
    let mut components = Vec::new();
    while let Some(root) = remaining.first().cloned() {
        let instance_set = reachable_instances(adjacency, root.as_str());
        for instance_id in &instance_set {
            remaining.remove(instance_id);
        }
        let root_instance_id = instance_set
            .first()
            .cloned()
            .context("Dome component is empty")?;
        let mut coordinates_cm = BTreeMap::from([(root_instance_id.clone(), [0, 0, 0])]);
        let mut coordinate_owners = BTreeMap::from([([0, 0, 0], root_instance_id.clone())]);
        let mut connection_ids = BTreeSet::new();
        let mut queue = VecDeque::from([root_instance_id.clone()]);
        while let Some(instance_id) = queue.pop_front() {
            let current = coordinates_cm[&instance_id];
            let mut edges = adjacency.get(&instance_id).cloned().unwrap_or_default();
            edges.sort_by(|left, right| left.2.cmp(&right.2));
            for (neighbor, direction, connection_id) in edges {
                connection_ids.insert(connection_id);
                let offset = direction_offset_cm(direction);
                let coordinate = [
                    current[0] + offset[0],
                    current[1] + offset[1],
                    current[2] + offset[2],
                ];
                if let Some(existing) = coordinates_cm.get(&neighbor) {
                    if *existing != coordinate {
                        bail!("Dome Connection topology assigns inconsistent coordinates");
                    }
                    continue;
                }
                if let Some(existing_owner) = coordinate_owners.get(&coordinate)
                    && existing_owner != &neighbor
                {
                    bail!("Dome Connection topology has a coordinate collision");
                }
                coordinate_owners.insert(coordinate, neighbor.clone());
                coordinates_cm.insert(neighbor.clone(), coordinate);
                queue.push_back(neighbor);
            }
        }
        components.push(DomeComponentTopologyV1 {
            root_instance_id,
            instance_ids: instance_set.into_iter().collect(),
            connection_ids: connection_ids.into_iter().collect(),
            coordinates_cm,
        });
    }
    components.sort_by(|left, right| left.root_instance_id.cmp(&right.root_instance_id));
    Ok(components)
}

fn direction_offset_cm(direction: DomeDirection) -> [i64; 3] {
    fixed_dome_v1()
        .endpoints
        .iter()
        .find(|endpoint| endpoint.direction == direction)
        .map(|endpoint| endpoint.adjacent_dome_offset_cm)
        .expect("fixed Dome contains every direction")
}
