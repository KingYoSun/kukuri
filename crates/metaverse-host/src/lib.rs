use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Result, bail};
use kukuri_core::{
    DOME_SNAPSHOT_RING_CAPACITY, DomeHostHeartbeatV1, DomeHostingLeaseV1, DomeInstanceManifestV1,
    DomeLayoutCandidateV1, DomePhysicsBodyKindV1, DomePhysicsBodyV1, DomePhysicsSnapshotV1,
    DomePresetManifestV1, DomeSessionInputKindV1, DomeSessionInputV1, KukuriKeys,
    MetaverseColliderV1, MetaversePersistentPropV1, SignedDomeHostHeartbeatV1,
    SignedDomeHostingLeaseV1, SignedDomeLayoutCandidateV1, SignedDomePhysicsSnapshotV1,
    SignedDomeSessionInputV1, build_signed_dome_host_heartbeat, build_signed_dome_layout_candidate,
    build_signed_dome_physics_snapshot, fixed_dome_v1, validate_dome_instance_manifest,
    validate_dome_preset_manifest, verify_signed_dome_hosting_lease,
    verify_signed_dome_session_input,
};
use rapier3d::prelude::*;
use uuid::Uuid;

pub const DOME_SIMULATION_HZ: u32 = 30;
pub const DOME_SNAPSHOT_HZ: u32 = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestPropSpec {
    pub prop_id: String,
    pub position: [i64; 3],
    pub expires_at: i64,
}

#[derive(Clone, Debug)]
struct RuntimeBody {
    handle: RigidBodyHandle,
    kind: DomePhysicsBodyKindV1,
    animation: Option<String>,
    grabbed_by: Option<String>,
    expires_at: Option<i64>,
    persistent_definition: Option<MetaversePersistentPropV1>,
}

pub struct DomeSessionRuntime {
    lease: SignedDomeHostingLeaseV1,
    host_keys: KukuriKeys,
    session_id: String,
    participants: BTreeSet<String>,
    last_input_sequence: BTreeMap<String, u64>,
    bodies_by_id: BTreeMap<String, RuntimeBody>,
    pipeline: PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    island_manager: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    rigid_bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    tick: u64,
    snapshot_sequence: u64,
    snapshot_ring: VecDeque<SignedDomePhysicsSnapshotV1>,
    last_simulated_at: i64,
}

impl DomeSessionRuntime {
    pub fn start(
        lease: SignedDomeHostingLeaseV1,
        host_keys: KukuriKeys,
        instance: &DomeInstanceManifestV1,
        preset: &DomePresetManifestV1,
        started_at: i64,
    ) -> Result<Self> {
        Self::start_with_session_id(
            lease,
            host_keys,
            instance,
            preset,
            format!("dome-session-{}", Uuid::new_v4()),
            started_at,
        )
    }

    pub fn start_with_session_id(
        lease: SignedDomeHostingLeaseV1,
        host_keys: KukuriKeys,
        instance: &DomeInstanceManifestV1,
        preset: &DomePresetManifestV1,
        session_id: impl Into<String>,
        started_at: i64,
    ) -> Result<Self> {
        validate_dome_instance_manifest(instance)?;
        validate_dome_preset_manifest(preset)?;
        verify_signed_dome_hosting_lease(&lease, instance)?;
        if lease.lease.host.signing_pubkey() != &host_keys.public_key() {
            bail!("Dome session host key does not match lease target");
        }
        if instance.preset_ref.preset_id != preset.preset_id
            || instance.preset_ref.manifest_blob_hash != lease.lease.manifest_blob_hash
            || started_at < lease.lease.issued_at
            || started_at >= lease.lease.expires_at
        {
            bail!("Dome session manifest or start time does not match lease");
        }
        let session_id = session_id.into();
        if session_id.trim().is_empty() {
            bail!("Dome session id is required");
        }

        let mut runtime = Self {
            lease,
            host_keys,
            session_id,
            participants: BTreeSet::new(),
            last_input_sequence: BTreeMap::new(),
            bodies_by_id: BTreeMap::new(),
            pipeline: PhysicsPipeline::new(),
            integration_parameters: IntegrationParameters {
                dt: 1.0 / DOME_SIMULATION_HZ as f32,
                ..IntegrationParameters::default()
            },
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            tick: 0,
            snapshot_sequence: 0,
            snapshot_ring: VecDeque::with_capacity(DOME_SNAPSHOT_RING_CAPACITY),
            last_simulated_at: started_at,
        };
        runtime.insert_dome_boundaries();
        for prop in &preset.dome.customization.persistent_props {
            runtime.insert_prop(prop.clone(), DomePhysicsBodyKindV1::PersistentProp, None)?;
        }
        Ok(runtime)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn lease(&self) -> &DomeHostingLeaseV1 {
        &self.lease.lease
    }

    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    pub fn is_sleeping(&self) -> bool {
        self.participants.is_empty()
    }

    pub fn add_guest_prop(&mut self, spec: GuestPropSpec) -> Result<()> {
        if spec.prop_id.trim().is_empty()
            || spec.expires_at <= self.last_simulated_at
            || self.bodies_by_id.contains_key(&spec.prop_id)
        {
            bail!("guest prop specification is invalid");
        }
        self.insert_prop(
            MetaversePersistentPropV1 {
                prop_id: spec.prop_id,
                asset_ref: None,
                primitive_fallback: kukuri_core::MetaversePrimitive::Cube,
                position: spec.position,
                rotation: [0, 0, 0],
                scale: [100, 100, 100],
                visual_only: false,
                interactions: Vec::new(),
                collider: None,
            },
            DomePhysicsBodyKindV1::GuestProp,
            Some(spec.expires_at),
        )
    }

    pub fn apply_signed_input(&mut self, signed: &SignedDomeSessionInputV1) -> Result<()> {
        verify_signed_dome_session_input(signed, &self.lease.lease, &self.session_id)?;
        self.apply_input(&signed.input)
    }

    pub fn advance_to(&mut self, now_millis: i64) -> Result<()> {
        if now_millis < self.last_simulated_at {
            bail!("Dome session clock cannot move backwards");
        }
        self.expire_guest_props(now_millis);
        if self.participants.is_empty() {
            self.last_simulated_at = now_millis;
            return Ok(());
        }

        let elapsed_millis = now_millis.saturating_sub(self.last_simulated_at);
        let step_millis = 1_000 / i64::from(DOME_SIMULATION_HZ);
        let steps = (elapsed_millis / step_millis).min(i64::from(DOME_SIMULATION_HZ) * 5);
        for _ in 0..steps {
            self.pipeline.step(
                Vector::new(0.0, -9.81, 0.0),
                &self.integration_parameters,
                &mut self.island_manager,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.rigid_bodies,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                &mut self.ccd_solver,
                &(),
                &(),
            );
            self.clamp_bodies_to_dome();
            self.tick = self.tick.saturating_add(1);
        }
        self.last_simulated_at = now_millis;
        Ok(())
    }

    pub fn signed_snapshot(&mut self, now_millis: i64) -> Result<SignedDomePhysicsSnapshotV1> {
        self.advance_to(now_millis)?;
        self.snapshot_sequence = self.snapshot_sequence.saturating_add(1);
        let bodies = self
            .bodies_by_id
            .iter()
            .filter_map(|(entity_id, runtime_body)| {
                let body = self.rigid_bodies.get(runtime_body.handle)?;
                let translation = body.translation();
                let rotation = body.rotation().to_scaled_axis();
                let velocity = body.linvel();
                Some(DomePhysicsBodyV1 {
                    entity_id: entity_id.clone(),
                    kind: runtime_body.kind,
                    position: meters_to_centimeters([translation.x, translation.y, translation.z]),
                    rotation: radians_to_milliradians([rotation.x, rotation.y, rotation.z]),
                    linear_velocity: meters_to_centimeters([velocity.x, velocity.y, velocity.z]),
                    animation: runtime_body.animation.clone(),
                    grabbed_by: runtime_body.grabbed_by.clone(),
                    expires_at: runtime_body.expires_at,
                })
            })
            .collect();
        let signed = build_signed_dome_physics_snapshot(
            &self.host_keys,
            &self.lease.lease,
            DomePhysicsSnapshotV1 {
                instance_id: self.lease.lease.instance_id.clone(),
                instance_generation: self.lease.lease.instance_generation,
                lease_epoch: self.lease.lease.epoch,
                session_id: self.session_id.clone(),
                host_pubkey: self.host_keys.public_key(),
                sequence: self.snapshot_sequence,
                simulated_at: now_millis,
                sleeping: self.is_sleeping(),
                bodies,
            },
        )?;
        self.snapshot_ring.push_back(signed.clone());
        while self.snapshot_ring.len() > DOME_SNAPSHOT_RING_CAPACITY {
            self.snapshot_ring.pop_front();
        }
        Ok(signed)
    }

    pub fn snapshots_after(&self, after_sequence: u64) -> Vec<SignedDomePhysicsSnapshotV1> {
        let Some(first) = self.snapshot_ring.front() else {
            return Vec::new();
        };
        if after_sequence != 0 && after_sequence < first.snapshot.sequence {
            return self.snapshot_ring.back().cloned().into_iter().collect();
        }
        let snapshots = self
            .snapshot_ring
            .iter()
            .filter(|snapshot| snapshot.snapshot.sequence > after_sequence)
            .cloned()
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            return snapshots;
        }
        Vec::new()
    }

    pub fn snapshot_ring_len(&self) -> usize {
        self.snapshot_ring.len()
    }

    pub fn signed_layout_candidate(
        &mut self,
        operation_id: impl Into<String>,
        now_millis: i64,
    ) -> Result<SignedDomeLayoutCandidateV1> {
        let snapshot = self.signed_snapshot(now_millis)?;
        let mut persistent_props = Vec::new();
        for runtime_body in self.bodies_by_id.values() {
            let Some(mut prop) = runtime_body.persistent_definition.clone() else {
                continue;
            };
            let body = self
                .rigid_bodies
                .get(runtime_body.handle)
                .context("persistent prop rigid body is missing")?;
            let translation = body.translation();
            let rotation = body.rotation().to_scaled_axis();
            prop.position = meters_to_centimeters([translation.x, translation.y, translation.z]);
            prop.rotation = radians_to_milliradians([rotation.x, rotation.y, rotation.z]);
            persistent_props.push(prop);
        }
        persistent_props.sort_by(|left, right| left.prop_id.cmp(&right.prop_id));
        build_signed_dome_layout_candidate(
            &self.host_keys,
            &self.lease.lease,
            DomeLayoutCandidateV1 {
                operation_id: operation_id.into(),
                instance_id: self.lease.lease.instance_id.clone(),
                instance_generation: self.lease.lease.instance_generation,
                lease_epoch: self.lease.lease.epoch,
                session_id: self.session_id.clone(),
                host_pubkey: self.host_keys.public_key(),
                base_manifest_revision: self.lease.lease.manifest_version,
                snapshot_sequence: snapshot.snapshot.sequence,
                captured_at: now_millis,
                persistent_props,
            },
        )
    }

    pub fn signed_heartbeat(&self, now_millis: i64) -> Result<SignedDomeHostHeartbeatV1> {
        build_signed_dome_host_heartbeat(
            &self.host_keys,
            &self.lease.lease,
            DomeHostHeartbeatV1 {
                instance_id: self.lease.lease.instance_id.clone(),
                instance_generation: self.lease.lease.instance_generation,
                lease_epoch: self.lease.lease.epoch,
                session_id: self.session_id.clone(),
                host_pubkey: self.host_keys.public_key(),
                participants: self.participant_count().try_into().unwrap_or(u32::MAX),
                sleeping: self.is_sleeping(),
                sent_at: now_millis,
            },
        )
    }

    fn apply_input(&mut self, input: &DomeSessionInputV1) -> Result<()> {
        let participant_id = input.participant_pubkey.as_str().to_string();
        let previous_sequence = self
            .last_input_sequence
            .get(&participant_id)
            .copied()
            .unwrap_or(0);
        if input.sequence <= previous_sequence {
            bail!("stale Dome session input sequence");
        }
        match &input.input {
            DomeSessionInputKindV1::Join => {
                self.participants.insert(participant_id.clone());
                self.ensure_avatar(&participant_id)?;
            }
            DomeSessionInputKindV1::Leave => {
                self.participants.remove(&participant_id);
                self.remove_body(&format!("avatar:{participant_id}"));
                for runtime_body in self.bodies_by_id.values_mut() {
                    if runtime_body.grabbed_by.as_deref() == Some(participant_id.as_str()) {
                        runtime_body.grabbed_by = None;
                    }
                }
            }
            DomeSessionInputKindV1::Move {
                position,
                rotation,
                animation,
            } => {
                self.require_participant(&participant_id)?;
                let entity_id = format!("avatar:{participant_id}");
                let runtime_body = self
                    .bodies_by_id
                    .get_mut(&entity_id)
                    .context("participant avatar body is missing")?;
                let body = self
                    .rigid_bodies
                    .get_mut(runtime_body.handle)
                    .context("participant avatar rigid body is missing")?;
                let position = centimeters_to_meters(*position);
                let rotation = milliradians_to_radians(*rotation);
                body.set_translation(Vector::new(position[0], position[1], position[2]), true);
                body.set_rotation(
                    Rotation::from_scaled_axis(Vector::new(rotation[0], rotation[1], rotation[2])),
                    true,
                );
                runtime_body.animation = Some(animation.clone());
            }
            DomeSessionInputKindV1::Grab { prop_id } => {
                self.require_participant(&participant_id)?;
                let body = self
                    .bodies_by_id
                    .get_mut(prop_id)
                    .context("Dome prop does not exist")?;
                body.grabbed_by = Some(participant_id.clone());
            }
            DomeSessionInputKindV1::Throw { prop_id, impulse }
            | DomeSessionInputKindV1::Push { prop_id, impulse } => {
                self.require_participant(&participant_id)?;
                let runtime_body = self
                    .bodies_by_id
                    .get_mut(prop_id)
                    .context("Dome prop does not exist")?;
                if matches!(input.input, DomeSessionInputKindV1::Throw { .. })
                    && runtime_body.grabbed_by.as_deref() != Some(participant_id.as_str())
                {
                    bail!("participant cannot throw a prop it is not grabbing");
                }
                let impulse = centimeters_to_meters(*impulse);
                self.rigid_bodies
                    .get_mut(runtime_body.handle)
                    .context("Dome prop rigid body is missing")?
                    .apply_impulse(Vector::new(impulse[0], impulse[1], impulse[2]), true);
                runtime_body.grabbed_by = None;
            }
            DomeSessionInputKindV1::Sit { prop_id } => {
                self.require_participant(&participant_id)?;
                if !self.bodies_by_id.contains_key(prop_id) {
                    bail!("Dome seat prop does not exist");
                }
            }
            DomeSessionInputKindV1::SpawnGuestProp { prop, expires_at } => {
                self.require_participant(&participant_id)?;
                if *expires_at <= self.last_simulated_at {
                    bail!("Dome guest prop is already expired");
                }
                self.insert_prop(
                    prop.clone(),
                    DomePhysicsBodyKindV1::GuestProp,
                    Some(*expires_at),
                )?;
            }
            DomeSessionInputKindV1::UpsertPersistentProp { prop } => {
                self.require_owner(input)?;
                self.remove_body(&prop.prop_id);
                self.insert_prop(prop.clone(), DomePhysicsBodyKindV1::PersistentProp, None)?;
            }
            DomeSessionInputKindV1::DeletePersistentProp { prop_id } => {
                self.require_owner(input)?;
                if self
                    .bodies_by_id
                    .get(prop_id)
                    .is_none_or(|body| body.kind != DomePhysicsBodyKindV1::PersistentProp)
                {
                    bail!("Dome persistent prop does not exist");
                }
                self.remove_body(prop_id);
            }
        }
        self.last_input_sequence
            .insert(participant_id, input.sequence);
        Ok(())
    }

    fn require_participant(&self, participant_id: &str) -> Result<()> {
        if !self.participants.contains(participant_id) {
            bail!("Dome session input requires a joined participant");
        }
        Ok(())
    }

    fn require_owner(&self, input: &DomeSessionInputV1) -> Result<()> {
        if input.participant_pubkey != self.lease.lease.owner_pubkey {
            bail!("only the Dome owner can change persistent props");
        }
        Ok(())
    }

    fn insert_dome_boundaries(&mut self) {
        let floor = RigidBodyBuilder::fixed()
            .translation(Vector::new(0.0, -0.1, 0.0))
            .build();
        let floor_handle = self.rigid_bodies.insert(floor);
        self.colliders.insert_with_parent(
            ColliderBuilder::cuboid(20.0, 0.1, 20.0)
                .friction(0.8)
                .build(),
            floor_handle,
            &mut self.rigid_bodies,
        );
    }

    fn insert_prop(
        &mut self,
        prop: MetaversePersistentPropV1,
        kind: DomePhysicsBodyKindV1,
        expires_at: Option<i64>,
    ) -> Result<()> {
        let prop_id = prop.prop_id.clone();
        if self.bodies_by_id.contains_key(&prop_id) {
            bail!("Dome physics entity id already exists");
        }
        let position = centimeters_to_meters(prop.position);
        let rotation = milliradians_to_radians(prop.rotation);
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(Vector::new(position[0], position[1], position[2]))
            .rotation(Vector::new(rotation[0], rotation[1], rotation[2]))
            .ccd_enabled(true)
            .build();
        let handle = self.rigid_bodies.insert(rigid_body);
        let collider = collider_builder(prop.collider.as_ref())
            .friction(0.7)
            .restitution(0.15)
            .build();
        self.colliders
            .insert_with_parent(collider, handle, &mut self.rigid_bodies);
        self.bodies_by_id.insert(
            prop_id,
            RuntimeBody {
                handle,
                kind,
                animation: None,
                grabbed_by: None,
                expires_at,
                persistent_definition: (kind == DomePhysicsBodyKindV1::PersistentProp)
                    .then_some(prop),
            },
        );
        Ok(())
    }

    fn ensure_avatar(&mut self, participant_id: &str) -> Result<()> {
        let entity_id = format!("avatar:{participant_id}");
        if self.bodies_by_id.contains_key(&entity_id) {
            return Ok(());
        }
        let body = RigidBodyBuilder::kinematic_position_based()
            .translation(Vector::new(0.0, 0.9, 0.0))
            .build();
        let handle = self.rigid_bodies.insert(body);
        self.colliders.insert_with_parent(
            ColliderBuilder::capsule_y(0.65, 0.25).build(),
            handle,
            &mut self.rigid_bodies,
        );
        self.bodies_by_id.insert(
            entity_id,
            RuntimeBody {
                handle,
                kind: DomePhysicsBodyKindV1::Avatar,
                animation: Some("idle".into()),
                grabbed_by: None,
                expires_at: None,
                persistent_definition: None,
            },
        );
        Ok(())
    }

    fn expire_guest_props(&mut self, now_millis: i64) {
        let expired: Vec<String> = self
            .bodies_by_id
            .iter()
            .filter(|(_, body)| {
                body.kind == DomePhysicsBodyKindV1::GuestProp
                    && body
                        .expires_at
                        .is_some_and(|expires_at| expires_at <= now_millis)
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in expired {
            self.remove_body(&id);
        }
    }

    fn remove_body(&mut self, entity_id: &str) {
        if let Some(body) = self.bodies_by_id.remove(entity_id) {
            self.rigid_bodies.remove(
                body.handle,
                &mut self.island_manager,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                true,
            );
        }
    }

    fn clamp_bodies_to_dome(&mut self) {
        let radius = fixed_dome_v1().inner_radius_cm as f32 / 100.0;
        for runtime_body in self.bodies_by_id.values() {
            let Some(body) = self.rigid_bodies.get_mut(runtime_body.handle) else {
                continue;
            };
            let mut translation = body.translation();
            translation.y = translation.y.max(0.0);
            let distance = translation.length();
            if distance > radius {
                translation *= (radius - 0.05) / distance;
            }
            body.set_translation(translation, true);
        }
    }
}

fn collider_builder(collider: Option<&MetaverseColliderV1>) -> ColliderBuilder {
    match collider {
        Some(MetaverseColliderV1::Capsule {
            radius,
            half_height,
            ..
        }) => ColliderBuilder::capsule_y(*half_height as f32 / 100.0, *radius as f32 / 100.0),
        Some(MetaverseColliderV1::Cuboid { half_extents, .. }) => ColliderBuilder::cuboid(
            half_extents[0] as f32 / 100.0,
            half_extents[1] as f32 / 100.0,
            half_extents[2] as f32 / 100.0,
        ),
        None => ColliderBuilder::capsule_y(0.5, 0.25),
    }
}

fn centimeters_to_meters(value: [i64; 3]) -> [f32; 3] {
    value.map(|component| component as f32 / 100.0)
}

fn meters_to_centimeters(value: [f32; 3]) -> [i64; 3] {
    value.map(|component| (component * 100.0).round() as i64)
}

fn milliradians_to_radians(value: [i64; 3]) -> [f32; 3] {
    value.map(|component| component as f32 / 1_000.0)
}

fn radians_to_milliradians(value: [f32; 3]) -> [i64; 3] {
    value.map(|component| (component * 1_000.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use kukuri_core::{
        DomeHostTargetV1, DomeHostingLeaseV1, DomeInstanceStatusV1, DomePresetRefV1,
        MetaverseDomeV1, MetaversePersistentPropV1, MetaversePrimitive, MetaverseRoomSpawnV1,
        SpatialContextV1, TopicId, build_signed_dome_hosting_lease,
    };

    use super::*;

    fn fixture() -> (
        KukuriKeys,
        SignedDomeHostingLeaseV1,
        DomeInstanceManifestV1,
        DomePresetManifestV1,
    ) {
        let owner = KukuriKeys::generate();
        let context = SpatialContextV1::Topic {
            topic_id: TopicId("kukuri:topic:runtime".into()),
        };
        let preset = DomePresetManifestV1 {
            preset_id: "preset-1".into(),
            owner_pubkey: owner.public_key(),
            revision: 1,
            dome: MetaverseDomeV1::default(),
            asset_refs: Vec::new(),
            updated_at: 1_000,
        };
        let preset_ref = DomePresetRefV1 {
            preset_id: preset.preset_id.clone(),
            owner_pubkey: owner.public_key(),
            revision: preset.revision,
            manifest_blob_hash: "manifest-hash".into(),
            manifest_mime: "application/vnd.kukuri.dome-preset+json".into(),
            manifest_bytes: 100,
        };
        let instance = DomeInstanceManifestV1 {
            instance_id: "dome-1".into(),
            spatial_context: context.clone(),
            owner_pubkey: owner.public_key(),
            preset_ref,
            title: "Dome".into(),
            description: String::new(),
            max_peers: Some(8),
            default_spawn: MetaverseRoomSpawnV1 {
                position: [0, 0, 0],
                rotation: [0, 0, 0],
            },
            generation: 1,
            status: DomeInstanceStatusV1::Active,
            relationship_detach: None,
            replacement_instance_id: None,
            chat_history: Vec::new(),
            updated_at: 1_000,
        };
        let lease = build_signed_dome_hosting_lease(
            &owner,
            DomeHostingLeaseV1 {
                lease_id: "lease-1".into(),
                spatial_context: context,
                instance_id: instance.instance_id.clone(),
                instance_generation: 1,
                owner_pubkey: owner.public_key(),
                host: DomeHostTargetV1::OwnerDevice {
                    endpoint_id: "endpoint-1".into(),
                    host_pubkey: owner.public_key(),
                },
                manifest_blob_hash: "manifest-hash".into(),
                manifest_version: 1,
                epoch: 1,
                issued_at: 1_000,
                expires_at: 20_000,
            },
        )
        .unwrap();
        (owner, lease, instance, preset)
    }

    fn signed_input(
        participant: &KukuriKeys,
        sequence: u64,
        input: DomeSessionInputKindV1,
    ) -> SignedDomeSessionInputV1 {
        kukuri_core::build_signed_dome_session_input(
            participant,
            DomeSessionInputV1 {
                input_id: format!("input-{sequence}"),
                instance_id: "dome-1".into(),
                instance_generation: 1,
                lease_epoch: 1,
                session_id: "session-1".into(),
                participant_pubkey: participant.public_key(),
                sequence,
                sent_at: 1_000 + sequence as i64,
                input,
            },
        )
        .unwrap()
    }

    #[test]
    fn zero_participants_sleep_but_wall_clock_ttl_expires() {
        let (owner, lease, instance, preset) = fixture();
        let mut runtime = DomeSessionRuntime::start_with_session_id(
            lease,
            owner,
            &instance,
            &preset,
            "session-1",
            1_000,
        )
        .unwrap();
        runtime
            .add_guest_prop(GuestPropSpec {
                prop_id: "guest-1".into(),
                position: [0, 100, 0],
                expires_at: 2_000,
            })
            .unwrap();
        assert!(runtime.is_sleeping());
        let snapshot = runtime.signed_snapshot(2_100).unwrap();
        assert!(snapshot.snapshot.sleeping);
        assert!(
            snapshot
                .snapshot
                .bodies
                .iter()
                .all(|body| body.entity_id != "guest-1")
        );
    }

    #[test]
    fn joined_participant_wakes_physics_and_stale_input_is_rejected() {
        let (owner, lease, instance, preset) = fixture();
        let participant = KukuriKeys::generate();
        let mut runtime = DomeSessionRuntime::start_with_session_id(
            lease,
            owner,
            &instance,
            &preset,
            "session-1",
            1_000,
        )
        .unwrap();
        let join = signed_input(&participant, 1, DomeSessionInputKindV1::Join);
        runtime.apply_signed_input(&join).unwrap();
        assert!(!runtime.is_sleeping());
        assert!(runtime.apply_signed_input(&join).is_err());
        runtime.advance_to(1_100).unwrap();
        assert!(runtime.signed_snapshot(1_200).unwrap().snapshot.sequence > 0);
    }

    #[test]
    fn restart_uses_manifest_initial_state_and_new_session() {
        let (owner, lease, instance, mut preset) = fixture();
        preset.dome.customization.persistent_props = vec![MetaversePersistentPropV1 {
            prop_id: "prop-1".into(),
            asset_ref: None,
            primitive_fallback: MetaversePrimitive::Cube,
            position: [100, 200, 300],
            rotation: [0, 0, 0],
            scale: [100, 100, 100],
            visual_only: false,
            interactions: Vec::new(),
            collider: None,
        }];
        let restarted = DomeSessionRuntime::start_with_session_id(
            lease,
            owner,
            &instance,
            &preset,
            "session-after-restart",
            2_000,
        )
        .unwrap();
        assert_eq!(restarted.session_id(), "session-after-restart");
        assert_eq!(restarted.participant_count(), 0);
        let body = restarted.bodies_by_id.get("prop-1").unwrap();
        let translation = restarted.rigid_bodies[body.handle].translation();
        assert_eq!(
            meters_to_centimeters([translation.x, translation.y, translation.z]),
            [100, 200, 300]
        );
    }

    #[test]
    fn snapshot_ring_is_bounded_and_resync_falls_back_to_latest() {
        let (owner, lease, instance, preset) = fixture();
        let mut runtime = DomeSessionRuntime::start_with_session_id(
            lease,
            owner,
            &instance,
            &preset,
            "session-1",
            1_000,
        )
        .unwrap();
        for index in 1..=DOME_SNAPSHOT_RING_CAPACITY + 5 {
            runtime.signed_snapshot(1_000 + index as i64).unwrap();
        }
        assert_eq!(runtime.snapshot_ring_len(), DOME_SNAPSHOT_RING_CAPACITY);
        let latest = runtime.snapshots_after(1);
        assert_eq!(latest.len(), 1);
        assert_eq!(
            latest[0].snapshot.sequence,
            (DOME_SNAPSHOT_RING_CAPACITY + 5) as u64
        );
    }

    #[test]
    fn layout_candidate_contains_only_owner_managed_persistent_props() {
        let (owner, lease, instance, preset) = fixture();
        let participant = KukuriKeys::generate();
        let mut runtime = DomeSessionRuntime::start_with_session_id(
            lease,
            owner.clone(),
            &instance,
            &preset,
            "session-1",
            1_000,
        )
        .unwrap();
        runtime
            .apply_signed_input(&signed_input(&participant, 1, DomeSessionInputKindV1::Join))
            .unwrap();
        let guest = MetaversePersistentPropV1 {
            prop_id: "guest-1".into(),
            asset_ref: None,
            primitive_fallback: MetaversePrimitive::Sphere,
            position: [0, 100, 0],
            rotation: [0, 0, 0],
            scale: [100, 100, 100],
            visual_only: false,
            interactions: Vec::new(),
            collider: None,
        };
        runtime
            .apply_signed_input(&signed_input(
                &participant,
                2,
                DomeSessionInputKindV1::SpawnGuestProp {
                    prop: guest,
                    expires_at: 10_000,
                },
            ))
            .unwrap();
        let persistent = MetaversePersistentPropV1 {
            prop_id: "owner-prop".into(),
            asset_ref: None,
            primitive_fallback: MetaversePrimitive::Cube,
            position: [100, 100, 100],
            rotation: [0, 0, 0],
            scale: [120, 120, 120],
            visual_only: false,
            interactions: Vec::new(),
            collider: None,
        };
        runtime
            .apply_signed_input(&signed_input(
                &owner,
                1,
                DomeSessionInputKindV1::UpsertPersistentProp { prop: persistent },
            ))
            .unwrap();
        let candidate = runtime.signed_layout_candidate("layout-1", 1_100).unwrap();
        assert!(
            candidate
                .candidate
                .persistent_props
                .iter()
                .any(|prop| prop.prop_id == "owner-prop")
        );
        assert!(
            candidate
                .candidate
                .persistent_props
                .iter()
                .all(|prop| prop.prop_id != "guest-1")
        );
        assert!(
            candidate
                .candidate
                .persistent_props
                .iter()
                .all(|prop| !prop.prop_id.starts_with("avatar:"))
        );
    }

    #[test]
    fn non_owner_cannot_mutate_persistent_props() {
        let (owner, lease, instance, preset) = fixture();
        let participant = KukuriKeys::generate();
        let mut runtime = DomeSessionRuntime::start_with_session_id(
            lease,
            owner,
            &instance,
            &preset,
            "session-1",
            1_000,
        )
        .unwrap();
        let prop = MetaversePersistentPropV1 {
            prop_id: "unauthorized".into(),
            asset_ref: None,
            primitive_fallback: MetaversePrimitive::Cube,
            position: [0, 100, 0],
            rotation: [0, 0, 0],
            scale: [100, 100, 100],
            visual_only: false,
            interactions: Vec::new(),
            collider: None,
        };
        let input = signed_input(
            &participant,
            1,
            DomeSessionInputKindV1::UpsertPersistentProp { prop },
        );
        assert!(runtime.apply_signed_input(&input).is_err());
    }
}
