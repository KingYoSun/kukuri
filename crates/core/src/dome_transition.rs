use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::{DomeDirection, Pubkey, SpatialContextV1, fixed_dome_v1};

pub const DOME_TRANSITION_TICKET_TTL_MILLIS: i64 = 15_000;
pub const DOME_TRANSITION_CROSSING_HYSTERESIS_CM: i64 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DomeTransitionDenialReasonV1 {
    HostUnavailable,
    AccessDenied,
    OwnersBlocked,
    VisitorBlocked,
    CapacityFull,
    AssetsUnavailable,
    StaleTopology,
    StaleSession,
    InvalidTicket,
}

impl DomeTransitionDenialReasonV1 {
    pub const fn code(self) -> &'static str {
        match self {
            Self::HostUnavailable => "DOME_TRANSITION_HOST_UNAVAILABLE",
            Self::AccessDenied => "DOME_TRANSITION_ACCESS_DENIED",
            Self::OwnersBlocked => "DOME_TRANSITION_OWNERS_BLOCKED",
            Self::VisitorBlocked => "DOME_TRANSITION_VISITOR_BLOCKED",
            Self::CapacityFull => "DOME_TRANSITION_CAPACITY_FULL",
            Self::AssetsUnavailable => "DOME_TRANSITION_ASSETS_UNAVAILABLE",
            Self::StaleTopology => "DOME_TRANSITION_STALE_TOPOLOGY",
            Self::StaleSession => "DOME_TRANSITION_STALE_SESSION",
            Self::InvalidTicket => "DOME_TRANSITION_INVALID_TICKET",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DomeBoundaryStateV1 {
    Closed,
    Loading,
    Ready,
    Denied,
    Full,
    Unhosted,
    Error,
    Stale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DomeTransitionPhaseV1 {
    Closed,
    Loading,
    Ready,
    Preparing,
    Provisional,
    Committing,
    Committed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeTransitionAdmissionRequestV1 {
    pub transition_id: String,
    pub connection_id: String,
    pub topology_digest: String,
    pub spatial_context: SpatialContextV1,
    pub source_instance_id: String,
    pub source_instance_generation: u64,
    pub target_instance_id: String,
    pub target_instance_generation: u64,
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub participant_pubkey: Pubkey,
    pub direction: DomeDirection,
    pub requested_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeTransitionAdmissionTicketV1 {
    pub request: DomeTransitionAdmissionRequestV1,
    pub target_lease_epoch: u64,
    pub target_session_id: String,
    pub expires_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum DomeTransitionAccessDecisionV1 {
    Allowed,
    Denied {
        reason: DomeTransitionDenialReasonV1,
    },
}

impl DomeTransitionAdmissionRequestV1 {
    pub fn validate(&self) -> Result<()> {
        if self.transition_id.trim().is_empty()
            || self.connection_id.trim().is_empty()
            || self.topology_digest.trim().is_empty()
            || self.source_instance_id.trim().is_empty()
            || self.target_instance_id.trim().is_empty()
            || self.source_instance_id == self.target_instance_id
            || self.source_instance_generation == 0
            || self.target_instance_generation == 0
            || self.requested_at <= 0
        {
            bail!("Dome transition admission request is invalid");
        }
        Ok(())
    }
}

impl DomeTransitionAdmissionTicketV1 {
    pub fn validate_for(
        &self,
        request: &DomeTransitionAdmissionRequestV1,
        lease_epoch: u64,
        session_id: &str,
        now_millis: i64,
    ) -> Result<()> {
        request.validate()?;
        if self.request != *request
            || self.target_lease_epoch != lease_epoch
            || self.target_session_id != session_id
            || self.expires_at <= now_millis
        {
            bail!("Dome transition admission ticket is invalid or expired");
        }
        Ok(())
    }
}

pub fn advance_dome_transition_phase(
    current: DomeTransitionPhaseV1,
    next: DomeTransitionPhaseV1,
) -> Result<DomeTransitionPhaseV1> {
    use DomeTransitionPhaseV1::*;
    let valid = matches!(
        (current, next),
        (Closed, Loading)
            | (Loading, Ready)
            | (Ready, Preparing)
            | (Preparing, Provisional)
            | (Provisional, Committing)
            | (Committing, Committed)
            | (
                Loading | Ready | Preparing | Provisional | Committing,
                Failed
            )
    );
    if current == next {
        return Ok(current);
    }
    if !valid {
        bail!("invalid Dome transition phase change");
    }
    Ok(next)
}

pub fn dome_transition_component_position_cm(
    dome_coordinate_cm: [i64; 3],
    local_position_cm: [i64; 3],
) -> [i64; 3] {
    [
        dome_coordinate_cm[0] + local_position_cm[0],
        dome_coordinate_cm[1] + local_position_cm[1],
        dome_coordinate_cm[2] + local_position_cm[2],
    ]
}

pub fn dome_transition_local_position_cm(
    component_position_cm: [i64; 3],
    dome_coordinate_cm: [i64; 3],
) -> [i64; 3] {
    [
        component_position_cm[0] - dome_coordinate_cm[0],
        component_position_cm[1] - dome_coordinate_cm[1],
        component_position_cm[2] - dome_coordinate_cm[2],
    ]
}

pub fn transform_avatar_between_domes_cm(
    source_local_position_cm: [i64; 3],
    source_coordinate_cm: [i64; 3],
    target_coordinate_cm: [i64; 3],
) -> [i64; 3] {
    dome_transition_local_position_cm(
        dome_transition_component_position_cm(source_coordinate_cm, source_local_position_cm),
        target_coordinate_cm,
    )
}

pub fn dome_transition_axis_cm(position_cm: [i64; 3], direction: DomeDirection) -> i64 {
    match direction {
        DomeDirection::North => -position_cm[2],
        DomeDirection::East => position_cm[0],
        DomeDirection::South => position_cm[2],
        DomeDirection::West => -position_cm[0],
    }
}

pub fn dome_transition_progress_millionths(position_cm: [i64; 3], direction: DomeDirection) -> u32 {
    let spec = fixed_dome_v1();
    let start = spec.connection_boundary_offset_cm - spec.connection_zone_depth_cm / 2;
    let axis = dome_transition_axis_cm(position_cm, direction);
    let travelled = axis
        .saturating_sub(start)
        .clamp(0, spec.connection_zone_depth_cm);
    ((travelled * 1_000_000) / spec.connection_zone_depth_cm) as u32
}

pub fn crossed_dome_transition_center(
    previous_position_cm: [i64; 3],
    current_position_cm: [i64; 3],
    direction: DomeDirection,
) -> bool {
    let center = fixed_dome_v1().connection_boundary_offset_cm;
    dome_transition_axis_cm(previous_position_cm, direction)
        <= center - DOME_TRANSITION_CROSSING_HYSTERESIS_CM
        && dome_transition_axis_cm(current_position_cm, direction)
            >= center + DOME_TRANSITION_CROSSING_HYSTERESIS_CM
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms_avatar_between_each_adjacent_dome_without_moving_component_position() {
        let cases = [
            (DomeDirection::North, [0, 0, -5_700]),
            (DomeDirection::East, [5_700, 0, 0]),
            (DomeDirection::South, [0, 0, 5_700]),
            (DomeDirection::West, [-5_700, 0, 0]),
        ];
        for (_, target) in cases {
            let source_local = [125, 90, -2_860];
            let target_local = transform_avatar_between_domes_cm(source_local, [0, 0, 0], target);
            assert_eq!(
                dome_transition_component_position_cm(target, target_local),
                source_local
            );
        }
    }

    #[test]
    fn progress_uses_the_fifteen_meter_zone_and_center_is_half() {
        assert_eq!(
            dome_transition_progress_millionths([0, 0, -2_100], DomeDirection::North),
            0
        );
        assert_eq!(
            dome_transition_progress_millionths([0, 0, -2_850], DomeDirection::North),
            500_000
        );
        assert_eq!(
            dome_transition_progress_millionths([0, 0, -3_600], DomeDirection::North),
            1_000_000
        );
    }

    #[test]
    fn crossing_requires_hysteresis_in_the_travel_direction() {
        assert!(crossed_dome_transition_center(
            [2_830, 0, 0],
            [2_870, 0, 0],
            DomeDirection::East
        ));
        assert!(!crossed_dome_transition_center(
            [2_870, 0, 0],
            [2_830, 0, 0],
            DomeDirection::East
        ));
        assert!(!crossed_dome_transition_center(
            [2_845, 0, 0],
            [2_855, 0, 0],
            DomeDirection::East
        ));
    }

    #[test]
    fn transition_phase_is_idempotent_and_rejects_skips() {
        assert_eq!(
            advance_dome_transition_phase(
                DomeTransitionPhaseV1::Loading,
                DomeTransitionPhaseV1::Loading
            )
            .expect("idempotent"),
            DomeTransitionPhaseV1::Loading
        );
        assert!(
            advance_dome_transition_phase(
                DomeTransitionPhaseV1::Ready,
                DomeTransitionPhaseV1::Committed
            )
            .is_err()
        );
    }
}
