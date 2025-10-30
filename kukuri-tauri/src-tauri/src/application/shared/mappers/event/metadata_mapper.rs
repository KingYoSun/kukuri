use crate::domain::entities::event_gateway::{ProfileMetadata, RelayEndpoint};
use crate::presentation::dto::event::{Nip65RelayDto, NostrMetadataDto};
use crate::shared::{AppError, ValidationFailureKind};
use nostr_sdk::JsonUtil;
use nostr_sdk::prelude::{Metadata, Url};
use serde_json::{Map, Value};

pub(crate) fn dto_to_profile_metadata(dto: NostrMetadataDto) -> Result<ProfileMetadata, AppError> {
    let relays = dto.relays.map(convert_relays).transpose()?;

    ProfileMetadata::new(
        dto.name,
        dto.display_name,
        dto.about,
        dto.picture,
        dto.banner,
        dto.nip05,
        dto.lud16,
        dto.website,
        relays,
    )
    .map_err(|err| {
        AppError::validation(
            ValidationFailureKind::Generic,
            format!("Invalid profile metadata: {err}"),
        )
    })
}

pub(crate) fn profile_metadata_to_nostr(metadata: &ProfileMetadata) -> Result<Metadata, AppError> {
    if let Some(website) = metadata.website.as_ref() {
        Url::parse(website).map_err(|_| {
            AppError::validation(ValidationFailureKind::Generic, "Invalid website URL")
        })?;
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
    if let Some(relays) = metadata.relays.as_ref() {
        let relay_values = relays
            .iter()
            .map(|relay| {
                let mut item = Map::new();
                item.insert("url".to_string(), Value::String(relay.url.clone()));
                item.insert("read".to_string(), Value::Bool(relay.read));
                item.insert("write".to_string(), Value::Bool(relay.write));
                Value::Object(item)
            })
            .collect();
        map.insert("relays".to_string(), Value::Array(relay_values));
    }

    Metadata::from_json(Value::Object(map).to_string())
        .map_err(|err| AppError::NostrError(err.to_string()))
}

fn convert_relays(relays: Vec<Nip65RelayDto>) -> Result<Vec<RelayEndpoint>, AppError> {
    relays
        .into_iter()
        .map(|relay| {
            RelayEndpoint::new(relay.url, relay.read, relay.write)
                .map_err(|err| AppError::validation(ValidationFailureKind::Generic, err))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dto_relays_are_converted_into_domain() {
        let dto = NostrMetadataDto {
            name: None,
            display_name: None,
            about: None,
            picture: None,
            banner: None,
            nip05: None,
            lud16: None,
            website: None,
            relays: Some(vec![Nip65RelayDto {
                url: "wss://relay.example".to_string(),
                read: true,
                write: false,
            }]),
        };

        let metadata = dto_to_profile_metadata(dto).expect("metadata conversion");
        let relay = metadata
            .relays
            .and_then(|mut list| list.pop())
            .expect("relay entry");
        assert_eq!(relay.url, "wss://relay.example");
        assert!(relay.read);
        assert!(!relay.write);
    }

    #[test]
    fn profile_metadata_includes_relays_in_json() {
        let relay = RelayEndpoint::new("wss://relay.example".to_string(), true, false).unwrap();
        let metadata = ProfileMetadata::new(
            Some("name".into()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(vec![relay]),
        )
        .expect("valid metadata");

        let nostr = profile_metadata_to_nostr(&metadata).expect("nostr metadata");
        let json_value: serde_json::Value =
            serde_json::from_str(&nostr.as_json()).expect("valid json");
        let relays = json_value
            .get("relays")
            .and_then(|value| value.as_array())
            .expect("relays array");
        assert_eq!(relays.len(), 1);
        let entry = relays[0].as_object().expect("relay object");
        assert_eq!(entry.get("url").unwrap(), &json!("wss://relay.example"));
        assert_eq!(entry.get("read").unwrap(), &json!(true));
        assert_eq!(entry.get("write").unwrap(), &json!(false));
    }
}
