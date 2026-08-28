use std::fmt;
use std::io::Cursor;

use anyhow::{Context, Result, bail};
use image::ImageReader;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;
const MAX_CONFIGURED_ASSET_BYTES: u64 = 2 * GIB;
const MAX_CONFIGURED_TRIANGLES: u64 = 20_000_000;
const MAX_TEXTURE_DIMENSION: u32 = 16_384;
const MAX_SNAPSHOT_HZ: u32 = 60;
const MAX_GLTF_JSON_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DomeResourceBudget {
    pub max_persistent_props: u32,
    pub max_texture_bytes: u64,
    pub max_texture_dimension: u32,
    pub max_model_bytes: u64,
    pub max_model_triangles: u64,
    pub max_colliders: u32,
    pub max_rigid_bodies: u32,
    pub max_snapshot_hz: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct PlayerResourceBudget {
    pub max_guest_props: u32,
    pub max_guest_prop_bytes: u64,
    pub max_avatar_asset_bytes: u64,
    pub max_prop_spawns_per_minute: u32,
    pub max_interactions_per_second: u32,
    pub max_input_bytes_per_second: u64,
    pub max_proposals_per_ten_minutes_per_slot: u32,
    pub max_impulse_centimeters: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct HostResourceBudget {
    pub max_participants: u32,
    pub max_simulated_rigid_bodies: u32,
    pub max_snapshot_bytes_per_second: u64,
    pub max_session_asset_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct ClientResourceBudget {
    pub max_rendered_avatars: u32,
    pub max_texture_memory_bytes: u64,
    pub max_rendered_triangles: u64,
    pub max_interpolated_bodies: u32,
    pub max_neighbor_domes: u32,
    pub cache_capacity_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct MetaverseResourceBudgetConfig {
    pub dome: DomeResourceBudget,
    pub player: PlayerResourceBudget,
    pub host: HostResourceBudget,
    pub client: ClientResourceBudget,
}

impl Default for MetaverseResourceBudgetConfig {
    fn default() -> Self {
        Self {
            dome: DomeResourceBudget {
                max_persistent_props: 64,
                max_texture_bytes: 256 * MIB,
                max_texture_dimension: 8_192,
                max_model_bytes: 256 * MIB,
                max_model_triangles: 2_000_000,
                max_colliders: 128,
                max_rigid_bodies: 256,
                max_snapshot_hz: 10,
            },
            player: PlayerResourceBudget {
                max_guest_props: 16,
                max_guest_prop_bytes: 64 * MIB,
                max_avatar_asset_bytes: 32 * MIB,
                max_prop_spawns_per_minute: 12,
                max_interactions_per_second: 30,
                max_input_bytes_per_second: 256 * 1024,
                max_proposals_per_ten_minutes_per_slot: 8,
                max_impulse_centimeters: 5_000,
            },
            host: HostResourceBudget {
                max_participants: 64,
                max_simulated_rigid_bodies: 512,
                max_snapshot_bytes_per_second: 4 * MIB,
                max_session_asset_bytes: 512 * MIB,
            },
            client: ClientResourceBudget {
                max_rendered_avatars: 32,
                max_texture_memory_bytes: 512 * MIB,
                max_rendered_triangles: 3_000_000,
                max_interpolated_bodies: 256,
                max_neighbor_domes: 4,
                cache_capacity_bytes: GIB,
            },
        }
    }
}

impl MetaverseResourceBudgetConfig {
    pub fn community_node_default() -> Self {
        let mut budget = Self::default();
        budget.client.cache_capacity_bytes = 10 * GIB;
        budget
    }

    pub fn from_json_override(value: Option<&str>) -> Result<Self> {
        let budget = match value.map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) => {
                serde_json::from_str(value).context("metaverse resource budget JSON is invalid")?
            }
            None => Self::default(),
        };
        budget.validate()?;
        Ok(budget)
    }

    pub fn community_node_from_json_override(value: Option<&str>) -> Result<Self> {
        let budget = match value.map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) => serde_json::from_str(value)
                .context("Community Node metaverse resource budget JSON is invalid")?,
            None => Self::community_node_default(),
        };
        budget.validate()?;
        Ok(budget)
    }

    pub fn validate(&self) -> Result<()> {
        let positive = [
            self.dome.max_persistent_props as u64,
            self.dome.max_texture_bytes,
            self.dome.max_texture_dimension as u64,
            self.dome.max_model_bytes,
            self.dome.max_model_triangles,
            self.dome.max_colliders as u64,
            self.dome.max_rigid_bodies as u64,
            self.dome.max_snapshot_hz as u64,
            self.player.max_guest_props as u64,
            self.player.max_guest_prop_bytes,
            self.player.max_avatar_asset_bytes,
            self.player.max_prop_spawns_per_minute as u64,
            self.player.max_interactions_per_second as u64,
            self.player.max_input_bytes_per_second,
            self.player.max_proposals_per_ten_minutes_per_slot as u64,
            self.host.max_participants as u64,
            self.host.max_simulated_rigid_bodies as u64,
            self.host.max_snapshot_bytes_per_second,
            self.host.max_session_asset_bytes,
            self.client.max_rendered_avatars as u64,
            self.client.max_texture_memory_bytes,
            self.client.max_rendered_triangles,
            self.client.max_interpolated_bodies as u64,
            self.client.cache_capacity_bytes,
        ];
        if positive.contains(&0) || self.player.max_impulse_centimeters <= 0 {
            bail!("metaverse resource budget values must be positive");
        }
        if self.dome.max_snapshot_hz > MAX_SNAPSHOT_HZ
            || self.dome.max_persistent_props > 1_024
            || self.dome.max_texture_dimension > MAX_TEXTURE_DIMENSION
            || self.dome.max_texture_bytes > MAX_CONFIGURED_ASSET_BYTES
            || self.dome.max_model_bytes > MAX_CONFIGURED_ASSET_BYTES
            || self.host.max_session_asset_bytes > MAX_CONFIGURED_ASSET_BYTES
            || self.client.max_texture_memory_bytes > MAX_CONFIGURED_ASSET_BYTES
            || self.client.cache_capacity_bytes > 64 * GIB
            || self.dome.max_model_triangles > MAX_CONFIGURED_TRIANGLES
            || self.client.max_rendered_triangles > MAX_CONFIGURED_TRIANGLES
            || self.client.max_neighbor_domes > 4
            || self.dome.max_colliders > 4_096
            || self.dome.max_rigid_bodies > 4_096
            || self.player.max_guest_props > 1_024
            || self.host.max_participants > 512
            || self.host.max_simulated_rigid_bodies > 8_192
            || self.client.max_rendered_avatars > 512
            || self.client.max_interpolated_bodies > 4_096
        {
            bail!("metaverse resource budget exceeds the safety ceiling");
        }
        if self.dome.max_rigid_bodies > self.host.max_simulated_rigid_bodies {
            bail!("Dome rigid body budget cannot exceed host capacity");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MetaverseBudgetScope {
    Dome,
    Player,
    Host,
    Client,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MetaverseBudgetResource {
    PersistentProps,
    GuestProps,
    TextureBytes,
    TextureDimension,
    ModelBytes,
    ModelTriangles,
    Colliders,
    RigidBodies,
    Participants,
    AvatarAssetBytes,
    PropSpawnRate,
    InteractionRate,
    InputBandwidth,
    ProposalRate,
    Impulse,
    SnapshotFrequency,
    SnapshotBandwidth,
    SessionAssetBytes,
    RenderedAvatars,
    TextureMemory,
    RenderedTriangles,
    InterpolatedBodies,
    NeighborDomes,
    CacheCapacity,
    AssetFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum MetaverseResourceRejectionReason {
    LimitExceeded,
    RateExceeded,
    InvalidValue,
    UnverifiedAsset,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct MetaverseResourceRejection {
    pub scope: MetaverseBudgetScope,
    pub resource: MetaverseBudgetResource,
    pub reason: MetaverseResourceRejectionReason,
    pub observed: u64,
    pub limit: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct MetaverseResourceMetricCountV1 {
    pub code: String,
    pub count: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct MetaverseResourceMetricsV1 {
    pub rejected_total: u64,
    pub rejection_counts: Vec<MetaverseResourceMetricCountV1>,
    pub participant_high_water: u32,
    pub rigid_body_high_water: u32,
    pub snapshot_bytes: u64,
    pub snapshot_throttled: u64,
}

impl MetaverseResourceRejection {
    pub fn new(
        scope: MetaverseBudgetScope,
        resource: MetaverseBudgetResource,
        reason: MetaverseResourceRejectionReason,
        observed: u64,
        limit: u64,
    ) -> Self {
        Self {
            scope,
            resource,
            reason,
            observed,
            limit,
        }
    }

    pub fn code(&self) -> String {
        format!(
            "METAVERSE_{}_{}_{}",
            enum_code(self.scope),
            enum_code(self.resource),
            enum_code(self.reason)
        )
    }
}

fn enum_code(value: impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "UNKNOWN".to_string())
        .to_ascii_uppercase()
}

impl fmt::Display for MetaverseResourceRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} (observed {}, limit {})",
            self.code(),
            self.observed,
            self.limit
        )
    }
}

impl std::error::Error for MetaverseResourceRejection {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
#[serde(deny_unknown_fields)]
pub struct MetaverseAssetBudgetMetadataV1 {
    pub stored_bytes: u64,
    pub texture_width: Option<u32>,
    pub texture_height: Option<u32>,
    pub decoded_texture_bytes: u64,
    pub model_triangles: u64,
    pub model_primitives: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MetaverseDomeResourceUsage {
    pub texture_bytes: u64,
    pub decoded_texture_bytes: u64,
    pub model_bytes: u64,
    pub model_triangles: u64,
    pub session_asset_bytes: u64,
}

pub fn validate_metaverse_asset_metadata(
    kind: &crate::MetaverseAssetKind,
    declared_size: Option<u64>,
    metadata: Option<&MetaverseAssetBudgetMetadataV1>,
) -> Result<()> {
    let metadata = metadata.context("metaverse asset budget metadata is required")?;
    if declared_size != Some(metadata.stored_bytes) || metadata.stored_bytes == 0 {
        bail!("metaverse asset size does not match inspected metadata");
    }
    match kind {
        crate::MetaverseAssetKind::Texture => {
            let (Some(width), Some(height)) = (metadata.texture_width, metadata.texture_height)
            else {
                bail!("metaverse texture dimensions are required");
            };
            if width == 0
                || height == 0
                || width > MAX_TEXTURE_DIMENSION
                || height > MAX_TEXTURE_DIMENSION
                || metadata.decoded_texture_bytes
                    != u64::from(width)
                        .saturating_mul(u64::from(height))
                        .saturating_mul(4)
                || metadata.model_triangles != 0
                || metadata.model_primitives != 0
            {
                bail!("metaverse texture budget metadata is invalid");
            }
        }
        crate::MetaverseAssetKind::Glb | crate::MetaverseAssetKind::Vrm => {
            let embedded_texture_shape_valid = match (
                metadata.texture_width,
                metadata.texture_height,
                metadata.decoded_texture_bytes,
            ) {
                (None, None, 0) => true,
                (Some(width), Some(height), decoded) => {
                    width > 0
                        && height > 0
                        && width <= MAX_TEXTURE_DIMENSION
                        && height <= MAX_TEXTURE_DIMENSION
                        && decoded > 0
                        && decoded <= MAX_CONFIGURED_ASSET_BYTES
                }
                _ => false,
            };
            if !embedded_texture_shape_valid
                || metadata.model_triangles > MAX_CONFIGURED_TRIANGLES
                || metadata.model_primitives == 0
            {
                bail!("metaverse model budget metadata is invalid");
            }
        }
        crate::MetaverseAssetKind::Other => {
            if metadata.texture_width.is_some()
                || metadata.texture_height.is_some()
                || metadata.decoded_texture_bytes != 0
                || metadata.model_triangles != 0
                || metadata.model_primitives != 0
            {
                bail!("generic metaverse asset budget metadata is invalid");
            }
        }
    }
    Ok(())
}

pub fn metaverse_dome_resource_usage(
    assets: &[crate::MetaverseAssetRef],
) -> Result<MetaverseDomeResourceUsage> {
    let mut usage = MetaverseDomeResourceUsage::default();
    for asset in assets {
        validate_metaverse_asset_metadata(
            &asset.kind,
            asset.size_bytes,
            asset.budget_metadata.as_ref(),
        )?;
        let metadata = asset.budget_metadata.as_ref().expect("validated above");
        usage.session_asset_bytes = usage
            .session_asset_bytes
            .saturating_add(metadata.stored_bytes);
        match asset.kind {
            crate::MetaverseAssetKind::Texture => {
                usage.texture_bytes = usage.texture_bytes.saturating_add(metadata.stored_bytes);
                usage.decoded_texture_bytes = usage
                    .decoded_texture_bytes
                    .saturating_add(metadata.decoded_texture_bytes);
            }
            crate::MetaverseAssetKind::Glb | crate::MetaverseAssetKind::Vrm => {
                usage.model_bytes = usage.model_bytes.saturating_add(metadata.stored_bytes);
                usage.model_triangles = usage
                    .model_triangles
                    .saturating_add(metadata.model_triangles);
                usage.decoded_texture_bytes = usage
                    .decoded_texture_bytes
                    .saturating_add(metadata.decoded_texture_bytes);
            }
            crate::MetaverseAssetKind::Other => {}
        }
    }
    Ok(usage)
}

pub fn validate_dome_asset_budget(
    assets: &[crate::MetaverseAssetRef],
    budget: &MetaverseResourceBudgetConfig,
) -> Result<MetaverseDomeResourceUsage> {
    let usage = metaverse_dome_resource_usage(assets)?;
    for (scope, resource, observed, limit) in [
        (
            MetaverseBudgetScope::Dome,
            MetaverseBudgetResource::TextureBytes,
            usage.texture_bytes,
            budget.dome.max_texture_bytes,
        ),
        (
            MetaverseBudgetScope::Dome,
            MetaverseBudgetResource::ModelBytes,
            usage.model_bytes,
            budget.dome.max_model_bytes,
        ),
        (
            MetaverseBudgetScope::Dome,
            MetaverseBudgetResource::ModelTriangles,
            usage.model_triangles,
            budget.dome.max_model_triangles,
        ),
        (
            MetaverseBudgetScope::Host,
            MetaverseBudgetResource::SessionAssetBytes,
            usage.session_asset_bytes,
            budget.host.max_session_asset_bytes,
        ),
    ] {
        if observed > limit {
            return Err(MetaverseResourceRejection::new(
                scope,
                resource,
                MetaverseResourceRejectionReason::LimitExceeded,
                observed,
                limit,
            )
            .into());
        }
    }
    for asset in assets {
        let metadata = asset.budget_metadata.as_ref().expect("validated above");
        if metadata.texture_width.is_none() {
            continue;
        }
        let observed = u64::from(
            metadata
                .texture_width
                .unwrap_or_default()
                .max(metadata.texture_height.unwrap_or_default()),
        );
        let limit = u64::from(budget.dome.max_texture_dimension);
        if observed > limit {
            return Err(MetaverseResourceRejection::new(
                MetaverseBudgetScope::Dome,
                MetaverseBudgetResource::TextureDimension,
                MetaverseResourceRejectionReason::LimitExceeded,
                observed,
                limit,
            )
            .into());
        }
    }
    Ok(usage)
}

pub fn inspect_metaverse_asset(
    kind: crate::MetaverseAssetKind,
    bytes: &[u8],
) -> Result<MetaverseAssetBudgetMetadataV1> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_CONFIGURED_ASSET_BYTES {
        bail!("metaverse asset size is outside the inspection limit");
    }
    let mut metadata = MetaverseAssetBudgetMetadataV1 {
        stored_bytes: bytes.len() as u64,
        ..MetaverseAssetBudgetMetadataV1::default()
    };
    match kind {
        crate::MetaverseAssetKind::Texture => {
            let (width, height) = inspect_texture_dimensions(bytes)?;
            metadata.texture_width = Some(width);
            metadata.texture_height = Some(height);
            metadata.decoded_texture_bytes = u64::from(width)
                .saturating_mul(u64::from(height))
                .saturating_mul(4);
        }
        crate::MetaverseAssetKind::Glb | crate::MetaverseAssetKind::Vrm => {
            let (json, binary) = glb_parts(bytes)?;
            let document: Value =
                serde_json::from_slice(json).context("metaverse model JSON is invalid")?;
            let accessors = document
                .get("accessors")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let meshes = document
                .get("meshes")
                .and_then(Value::as_array)
                .context("metaverse model has no meshes")?;
            let mut triangles = 0_u64;
            let mut primitives = 0_u32;
            for mesh in meshes {
                let mesh_primitives = mesh
                    .get("primitives")
                    .and_then(Value::as_array)
                    .context("metaverse model mesh has no primitives")?;
                for primitive in mesh_primitives {
                    primitives = primitives.saturating_add(1);
                    let accessor_index = primitive
                        .get("indices")
                        .and_then(Value::as_u64)
                        .or_else(|| {
                            primitive
                                .get("attributes")
                                .and_then(|value| value.get("POSITION"))
                                .and_then(Value::as_u64)
                        })
                        .context("metaverse model primitive has no countable vertices")?;
                    let count = accessors
                        .get(accessor_index as usize)
                        .and_then(|value| value.get("count"))
                        .and_then(Value::as_u64)
                        .context("metaverse model accessor count is invalid")?;
                    let mode = primitive.get("mode").and_then(Value::as_u64).unwrap_or(4);
                    let primitive_triangles = match mode {
                        4 => count / 3,
                        5 | 6 => count.saturating_sub(2),
                        _ => 0,
                    };
                    triangles = triangles.saturating_add(primitive_triangles);
                    if triangles > MAX_CONFIGURED_TRIANGLES || primitives > 100_000 {
                        bail!("metaverse model complexity exceeds the inspection ceiling");
                    }
                }
            }
            metadata.model_triangles = triangles;
            metadata.model_primitives = primitives;

            if let Some(images) = document.get("images").and_then(Value::as_array)
                && !images.is_empty()
            {
                let binary = binary.context(
                    "metaverse model with embedded images must contain a binary GLB chunk",
                )?;
                let buffer_views = document
                    .get("bufferViews")
                    .and_then(Value::as_array)
                    .context("metaverse model embedded images require buffer views")?;
                let mut max_width = 0_u32;
                let mut max_height = 0_u32;
                let mut decoded_bytes = 0_u64;
                for image in images {
                    if image.get("uri").is_some() {
                        bail!("metaverse model external or data URI images are not supported");
                    }
                    let view_index = image
                        .get("bufferView")
                        .and_then(Value::as_u64)
                        .context("metaverse model image buffer view is missing")?
                        as usize;
                    let view = buffer_views
                        .get(view_index)
                        .context("metaverse model image buffer view is invalid")?;
                    if view.get("buffer").and_then(Value::as_u64).unwrap_or(0) != 0 {
                        bail!("metaverse model image references an unsupported buffer");
                    }
                    let offset = view
                        .get("byteOffset")
                        .and_then(Value::as_u64)
                        .unwrap_or_default() as usize;
                    let length = view
                        .get("byteLength")
                        .and_then(Value::as_u64)
                        .context("metaverse model image byte length is missing")?
                        as usize;
                    let end = offset
                        .checked_add(length)
                        .context("metaverse model image range overflows")?;
                    let image_bytes = binary
                        .get(offset..end)
                        .context("metaverse model image range is outside the binary chunk")?;
                    let (width, height) = inspect_texture_dimensions(image_bytes)?;
                    max_width = max_width.max(width);
                    max_height = max_height.max(height);
                    decoded_bytes = decoded_bytes.saturating_add(
                        u64::from(width)
                            .saturating_mul(u64::from(height))
                            .saturating_mul(4),
                    );
                    if decoded_bytes > MAX_CONFIGURED_ASSET_BYTES {
                        bail!(
                            "metaverse model embedded texture memory exceeds the inspection ceiling"
                        );
                    }
                }
                metadata.texture_width = Some(max_width);
                metadata.texture_height = Some(max_height);
                metadata.decoded_texture_bytes = decoded_bytes;
            }
        }
        crate::MetaverseAssetKind::Other => {}
    }
    Ok(metadata)
}

fn inspect_texture_dimensions(bytes: &[u8]) -> Result<(u32, u32)> {
    let (width, height) =
        if bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" && &bytes[12..16] == b"IHDR" {
            (
                u32::from_be_bytes(bytes[16..20].try_into()?),
                u32::from_be_bytes(bytes[20..24].try_into()?),
            )
        } else {
            let reader = ImageReader::new(Cursor::new(bytes))
                .with_guessed_format()
                .context("metaverse texture format is not recognized")?;
            reader
                .into_dimensions()
                .context("metaverse texture dimensions are invalid")?
        };
    if width == 0 || height == 0 || width > MAX_TEXTURE_DIMENSION || height > MAX_TEXTURE_DIMENSION
    {
        bail!("metaverse texture dimensions exceed the inspection ceiling");
    }
    Ok((width, height))
}

fn glb_parts(bytes: &[u8]) -> Result<(&[u8], Option<&[u8]>)> {
    if bytes.len() < 20 || &bytes[..4] != b"glTF" {
        bail!("metaverse model must be a binary glTF/VRM asset");
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into()?);
    let declared_length = u32::from_le_bytes(bytes[8..12].try_into()?) as usize;
    let json_length = u32::from_le_bytes(bytes[12..16].try_into()?) as usize;
    let chunk_type = u32::from_le_bytes(bytes[16..20].try_into()?);
    if version != 2
        || declared_length != bytes.len()
        || chunk_type != 0x4e4f534a
        || json_length == 0
        || json_length > MAX_GLTF_JSON_BYTES
        || 20_usize.saturating_add(json_length) > bytes.len()
    {
        bail!("metaverse model GLB header is invalid");
    }
    let json_end = 20 + json_length;
    let binary = if json_end == bytes.len() {
        None
    } else {
        if bytes.len().saturating_sub(json_end) < 8 {
            bail!("metaverse model binary chunk header is invalid");
        }
        let binary_length = u32::from_le_bytes(bytes[json_end..json_end + 4].try_into()?) as usize;
        let binary_type = u32::from_le_bytes(bytes[json_end + 4..json_end + 8].try_into()?);
        let binary_start = json_end + 8;
        let binary_end = binary_start
            .checked_add(binary_length)
            .context("metaverse model binary chunk overflows")?;
        if binary_type != 0x004e4942 || binary_end != bytes.len() {
            bail!("metaverse model binary chunk is invalid");
        }
        Some(&bytes[binary_start..binary_end])
    };
    Ok((&bytes[20..json_end], binary))
}
