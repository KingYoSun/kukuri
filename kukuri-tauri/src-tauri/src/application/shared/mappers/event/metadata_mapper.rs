use crate::domain::entities::event_gateway::ProfileMetadata;
use crate::presentation::dto::event::NostrMetadataDto;
use crate::shared::error::AppError;
use nostr_sdk::JsonUtil;
use nostr_sdk::prelude::{Metadata, Url};
use serde_json::{Map, Value};

pub(crate) fn dto_to_profile_metadata(dto: NostrMetadataDto) -> Result<ProfileMetadata, AppError> {
    ProfileMetadata::new(
        dto.name,
        dto.display_name,
        dto.about,
        dto.picture,
        dto.banner,
        dto.nip05,
        dto.lud16,
        dto.website,
    )
    .map_err(|err| AppError::ValidationError(format!("Invalid profile metadata: {err}")))
}

pub(crate) fn profile_metadata_to_nostr(metadata: &ProfileMetadata) -> Result<Metadata, AppError> {
    if let Some(website) = metadata.website.as_ref() {
        Url::parse(website)
            .map_err(|_| AppError::ValidationError("Invalid website URL".to_string()))?;
    }

    let mut map = Map::new();
    if let Some(name) = metadata.name.as_ref() {
        map.insert("name".to_string(), Value::String(name.clone()));
    }
    if let Some(display_name) = metadata.display_name.as_ref() {
        map.insert(
            "display_name".to_string(),
            Value::String(display_name.clone()),
        );
    }
    if let Some(about) = metadata.about.as_ref() {
        map.insert("about".to_string(), Value::String(about.clone()));
    }
    if let Some(picture) = metadata.picture.as_ref() {
        map.insert("picture".to_string(), Value::String(picture.clone()));
    }
    if let Some(banner) = metadata.banner.as_ref() {
        map.insert("banner".to_string(), Value::String(banner.clone()));
    }
    if let Some(nip05) = metadata.nip05.as_ref() {
        map.insert("nip05".to_string(), Value::String(nip05.clone()));
    }
    if let Some(lud16) = metadata.lud16.as_ref() {
        map.insert("lud16".to_string(), Value::String(lud16.clone()));
    }
    if let Some(website) = metadata.website.as_ref() {
        map.insert("website".to_string(), Value::String(website.clone()));
    }

    Metadata::from_json(Value::Object(map).to_string())
        .map_err(|err| AppError::NostrError(err.to_string()))
}
