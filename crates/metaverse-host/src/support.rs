use anyhow::Result;
use kukuri_core::{
    MetaverseBudgetResource, MetaverseBudgetScope, MetaverseColliderV1, MetaverseResourceRejection,
    MetaverseResourceRejectionReason,
};
use rapier3d::prelude::ColliderBuilder;

use crate::RateWindow;

pub(crate) fn rejection(
    scope: MetaverseBudgetScope,
    resource: MetaverseBudgetResource,
    reason: MetaverseResourceRejectionReason,
    observed: u64,
    limit: u64,
) -> anyhow::Error {
    MetaverseResourceRejection::new(scope, resource, reason, observed, limit).into()
}

pub(crate) fn check_limit(
    scope: MetaverseBudgetScope,
    resource: MetaverseBudgetResource,
    observed: u64,
    limit: u64,
) -> Result<()> {
    if observed > limit {
        return Err(rejection(
            scope,
            resource,
            MetaverseResourceRejectionReason::LimitExceeded,
            observed,
            limit,
        ));
    }
    Ok(())
}

pub(crate) fn window_allows(
    window: &mut RateWindow,
    now_millis: i64,
    duration_millis: i64,
    amount: u64,
    limit: u64,
) -> bool {
    if window.started_at == 0 || now_millis.saturating_sub(window.started_at) >= duration_millis {
        window.started_at = now_millis;
        window.count = 0;
    }
    window.count = window.count.saturating_add(amount);
    window.count <= limit
}

pub(crate) fn collider_builder(collider: Option<&MetaverseColliderV1>) -> ColliderBuilder {
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

pub(crate) fn centimeters_to_meters(value: [i64; 3]) -> [f32; 3] {
    value.map(|component| component as f32 / 100.0)
}

pub(crate) fn meters_to_centimeters(value: [f32; 3]) -> [i64; 3] {
    value.map(|component| (component * 100.0).round() as i64)
}

pub(crate) fn milliradians_to_radians(value: [i64; 3]) -> [f32; 3] {
    value.map(|component| component as f32 / 1_000.0)
}

pub(crate) fn radians_to_milliradians(value: [f32; 3]) -> [i64; 3] {
    value.map(|component| (component * 1_000.0).round() as i64)
}
