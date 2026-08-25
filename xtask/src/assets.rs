use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Component, Path};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[allow(unused_imports)]
use crate::*;

const ASSET_MANIFEST_SCHEMA_VERSION: u32 = 1;
const ASSET_MANIFEST_PATH: &str = "docs/ASSET_MANIFEST.json";
const ASSET_INVENTORY_TARGETS: &[&str] = &[
    "apps/desktop/app-icon.png",
    "apps/desktop/public",
    "apps/desktop/src-tauri/icons",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetManifest {
    schema_version: u32,
    assets: Vec<AssetRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetRecord {
    id: String,
    display_name: String,
    origin: AssetOrigin,
    author: String,
    rights_holder: String,
    source: AssetSource,
    license: AssetLicense,
    modification: AssetModification,
    generation: Option<GenerationInfo>,
    files: Vec<AssetFile>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum AssetOrigin {
    Authored,
    ThirdParty,
    Generated,
    GeneratedAssisted,
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetSource {
    url: Option<String>,
    acquired_on: Option<String>,
    created_on: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetLicense {
    id: String,
    text_url: Option<String>,
    text_path: Option<String>,
    commercial_use: bool,
    repository_redistribution: bool,
    binary_redistribution: bool,
    attribution_required: bool,
    attribution: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetModification {
    modified: bool,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GenerationInfo {
    service: String,
    model: String,
    input_rights: String,
    output_terms_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetFile {
    path: String,
    sha256: String,
    distribution: AssetDistribution,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum AssetDistribution {
    SourceOnly,
    BundledBinary,
}

pub(crate) fn asset_check() -> Result<()> {
    let root = root_dir();
    let manifest_path = root.join(ASSET_MANIFEST_PATH);
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest: AssetManifest = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
    let inventory = collect_asset_inventory(&root)?;
    validate_manifest_against_inventory(&manifest, &inventory)?;
    validate_license_text_paths(&root, &manifest)?;
    println!(
        "[xtask] asset manifest ok: assets={} files={}",
        manifest.assets.len(),
        inventory.len()
    );
    Ok(())
}

fn validate_manifest_against_inventory(
    manifest: &AssetManifest,
    inventory: &BTreeMap<String, String>,
) -> Result<()> {
    if manifest.schema_version != ASSET_MANIFEST_SCHEMA_VERSION {
        bail!(
            "unsupported asset manifest schema version: expected {}, got {}",
            ASSET_MANIFEST_SCHEMA_VERSION,
            manifest.schema_version
        );
    }
    if manifest.assets.is_empty() {
        bail!("asset manifest must contain at least one asset");
    }

    let mut ids = BTreeSet::new();
    let mut registered = BTreeMap::new();
    for asset in &manifest.assets {
        validate_asset(asset)?;
        if !ids.insert(asset.id.as_str()) {
            bail!("duplicate asset id: {}", asset.id);
        }
        for file in &asset.files {
            if registered
                .insert(file.path.as_str(), file.sha256.as_str())
                .is_some()
            {
                bail!("asset path is registered more than once: {}", file.path);
            }
        }
    }

    let registered_paths = registered.keys().copied().collect::<BTreeSet<_>>();
    let inventory_paths = inventory
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let unregistered = inventory_paths
        .difference(&registered_paths)
        .copied()
        .collect::<Vec<_>>();
    if !unregistered.is_empty() {
        bail!(
            "asset manifest is missing files: {}",
            unregistered.join(", ")
        );
    }
    let missing = registered_paths
        .difference(&inventory_paths)
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "asset manifest references missing files: {}",
            missing.join(", ")
        );
    }
    for (path, expected_hash) in registered {
        let actual_hash = inventory
            .get(path)
            .with_context(|| format!("asset inventory is missing {path}"))?;
        if actual_hash != expected_hash {
            bail!(
                "asset content changed without a manifest update: {path} (expected {expected_hash}, got {actual_hash})"
            );
        }
    }
    Ok(())
}

fn validate_asset(asset: &AssetRecord) -> Result<()> {
    require_nonblank("asset id", &asset.id)?;
    if !asset
        .id
        .bytes()
        .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'-')
    {
        bail!(
            "asset id must use lowercase ASCII letters, digits, and hyphens: {}",
            asset.id
        );
    }
    require_nonblank("asset display name", &asset.display_name)?;
    require_nonblank("asset author", &asset.author)?;
    require_nonblank("asset rights holder", &asset.rights_holder)?;

    match asset.origin {
        AssetOrigin::Unknown => bail!("asset origin must be known: {}", asset.id),
        AssetOrigin::Authored => {
            validate_optional_https_url("source url", asset.source.url.as_deref())?;
            validate_required_date("created_on", asset.source.created_on.as_deref())?;
        }
        AssetOrigin::ThirdParty => {
            validate_required_https_url("source url", asset.source.url.as_deref())?;
            validate_required_date("acquired_on", asset.source.acquired_on.as_deref())?;
        }
        AssetOrigin::Generated | AssetOrigin::GeneratedAssisted => {
            validate_optional_https_url("source url", asset.source.url.as_deref())?;
            validate_required_date("created_on", asset.source.created_on.as_deref())?;
        }
    }
    if let Some(value) = asset.source.acquired_on.as_deref() {
        validate_date("acquired_on", value)?;
    }
    if let Some(value) = asset.source.created_on.as_deref() {
        validate_date("created_on", value)?;
    }

    let generated = matches!(
        asset.origin,
        AssetOrigin::Generated | AssetOrigin::GeneratedAssisted
    );
    match (generated, asset.generation.as_ref()) {
        (true, Some(generation)) => validate_generation(generation)?,
        (true, None) => bail!(
            "generated asset is missing generation provenance: {}",
            asset.id
        ),
        (false, Some(_)) => bail!(
            "non-generated asset must not contain generation provenance: {}",
            asset.id
        ),
        (false, None) => {}
    }

    validate_license(asset)?;
    match (
        asset.modification.modified,
        nonblank(asset.modification.description.as_deref()),
    ) {
        (true, None) => bail!(
            "modified asset is missing a modification description: {}",
            asset.id
        ),
        (false, Some(_)) => bail!(
            "unmodified asset must not contain a modification description: {}",
            asset.id
        ),
        _ => {}
    }
    if asset.files.is_empty() {
        bail!("asset must register at least one file: {}", asset.id);
    }
    for file in &asset.files {
        validate_relative_path("asset file path", &file.path)?;
        validate_sha256(&file.path, &file.sha256)?;
    }
    Ok(())
}

fn validate_license(asset: &AssetRecord) -> Result<()> {
    let license = &asset.license;
    require_nonblank("asset license id", &license.id)?;
    let normalized_id = license.id.trim().to_ascii_uppercase();
    if matches!(
        normalized_id.as_str(),
        "UNKNOWN" | "NOASSERTION" | "NONE" | "UNLICENSED"
    ) {
        bail!("asset has an unknown or unusable license: {}", asset.id);
    }
    if license.text_url.is_none() && license.text_path.is_none() {
        bail!(
            "asset license must provide text_url or text_path: {}",
            asset.id
        );
    }
    validate_optional_https_url("license text url", license.text_url.as_deref())?;
    if let Some(path) = license.text_path.as_deref() {
        validate_relative_path("license text path", path)?;
    }
    if !license.commercial_use {
        bail!("asset is not approved for commercial use: {}", asset.id);
    }
    if !license.repository_redistribution {
        bail!(
            "asset is not approved for repository redistribution: {}",
            asset.id
        );
    }
    if asset
        .files
        .iter()
        .any(|file| file.distribution == AssetDistribution::BundledBinary)
        && !license.binary_redistribution
    {
        bail!(
            "asset is bundled but not approved for binary redistribution: {}",
            asset.id
        );
    }
    if license.attribution_required && nonblank(license.attribution.as_deref()).is_none() {
        bail!(
            "asset requires attribution but attribution text is missing: {}",
            asset.id
        );
    }
    Ok(())
}

fn validate_generation(generation: &GenerationInfo) -> Result<()> {
    require_nonblank("generation service", &generation.service)?;
    require_nonblank("generation model", &generation.model)?;
    require_nonblank("generation input rights", &generation.input_rights)?;
    validate_required_https_url(
        "generation output terms url",
        Some(&generation.output_terms_url),
    )
}

fn validate_license_text_paths(root: &Path, manifest: &AssetManifest) -> Result<()> {
    for asset in &manifest.assets {
        if let Some(path) = asset.license.text_path.as_deref() {
            let resolved = root.join(path);
            if !resolved.is_file() {
                bail!(
                    "asset license text file does not exist for {}: {}",
                    asset.id,
                    resolved.display()
                );
            }
        }
    }
    Ok(())
}

fn collect_asset_inventory(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut inventory = BTreeMap::new();
    for relative in ASSET_INVENTORY_TARGETS {
        let path = root.join(relative);
        collect_inventory_path(root, &path, &mut inventory)?;
    }
    Ok(inventory)
}

fn collect_inventory_path(
    root: &Path,
    path: &Path,
    inventory: &mut BTreeMap<String, String>,
) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("asset inventory target is missing: {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!(
            "asset inventory must not contain symlinks: {}",
            path.display()
        );
    }
    if metadata.is_dir() {
        let mut children = std::fs::read_dir(path)
            .with_context(|| format!("failed to read asset directory {}", path.display()))?
            .map(|entry| entry.map(|value| value.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        children.sort();
        for child in children {
            collect_inventory_path(root, &child, inventory)?;
        }
        return Ok(());
    }
    if !metadata.is_file() {
        bail!("asset inventory entry is not a file: {}", path.display());
    }
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("asset path is outside repository: {}", path.display()))?;
    let normalized = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    let hash = sha256_file(path)?;
    if inventory.insert(normalized.clone(), hash).is_some() {
        bail!("asset inventory contains a duplicate path: {normalized}");
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path)
        .with_context(|| format!("failed to open asset for hashing: {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("failed to hash asset: {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn validate_relative_path(label: &str, value: &str) -> Result<()> {
    require_nonblank(label, value)?;
    if value.contains('\\') || value.contains(':') {
        bail!("{label} must use a repository-relative slash path: {value}");
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            !matches!(component, Component::Normal(_))
                || matches!(component, Component::ParentDir | Component::RootDir)
        })
    {
        bail!("{label} must not escape the repository: {value}");
    }
    Ok(())
}

fn validate_sha256(path: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("asset sha256 must be 64 lowercase hexadecimal characters: {path}");
    }
    Ok(())
}

fn validate_required_https_url(label: &str, value: Option<&str>) -> Result<()> {
    let value = nonblank(value).with_context(|| format!("{label} is required"))?;
    validate_https_url(label, value)
}

fn validate_optional_https_url(label: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = nonblank(value) {
        validate_https_url(label, value)?;
    }
    Ok(())
}

fn validate_https_url(label: &str, value: &str) -> Result<()> {
    if !value.starts_with("https://") || value.len() <= "https://".len() {
        bail!("{label} must be an https URL: {value}");
    }
    Ok(())
}

fn validate_required_date(label: &str, value: Option<&str>) -> Result<()> {
    let value = nonblank(value).with_context(|| format!("{label} is required"))?;
    validate_date(label, value)
}

fn validate_date(label: &str, value: &str) -> Result<()> {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        bail!("{label} must use YYYY-MM-DD: {value}");
    }
    let year = parts[0]
        .parse::<u32>()
        .with_context(|| format!("{label} has an invalid year: {value}"))?;
    let month = parts[1]
        .parse::<u32>()
        .with_context(|| format!("{label} has an invalid month: {value}"))?;
    let day = parts[2]
        .parse::<u32>()
        .with_context(|| format!("{label} has an invalid day: {value}"))?;
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > days_in_month {
        bail!("{label} is not a valid calendar date: {value}");
    }
    Ok(())
}

fn require_nonblank(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} must not be blank");
    }
    Ok(())
}

fn nonblank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn authored_asset(path: &str, hash: &str) -> AssetRecord {
        AssetRecord {
            id: "app-icon".to_string(),
            display_name: "kukuri application icon".to_string(),
            origin: AssetOrigin::Authored,
            author: "KingYoSun".to_string(),
            rights_holder: "KingYoSun".to_string(),
            source: AssetSource {
                url: None,
                acquired_on: None,
                created_on: Some("2026-03-26".to_string()),
            },
            license: AssetLicense {
                id: "MIT".to_string(),
                text_url: None,
                text_path: Some("LICENSE".to_string()),
                commercial_use: true,
                repository_redistribution: true,
                binary_redistribution: true,
                attribution_required: true,
                attribution: Some("Copyright (c) KingYoSun".to_string()),
            },
            modification: AssetModification {
                modified: false,
                description: None,
            },
            generation: None,
            files: vec![AssetFile {
                path: path.to_string(),
                sha256: hash.to_string(),
                distribution: AssetDistribution::SourceOnly,
            }],
        }
    }

    fn third_party_asset(path: &str, hash: &str) -> AssetRecord {
        AssetRecord {
            id: "idle-loop".to_string(),
            display_name: "Idle Loop VRMA".to_string(),
            origin: AssetOrigin::ThirdParty,
            author: "Quaternius".to_string(),
            rights_holder: "Quaternius".to_string(),
            source: AssetSource {
                url: Some(
                    "https://quaternius.com/packs/universalanimationlibrary.html".to_string(),
                ),
                acquired_on: Some("2026-05-29".to_string()),
                created_on: None,
            },
            license: AssetLicense {
                id: "CC0-1.0".to_string(),
                text_url: Some("https://creativecommons.org/publicdomain/zero/1.0/".to_string()),
                text_path: None,
                commercial_use: true,
                repository_redistribution: true,
                binary_redistribution: true,
                attribution_required: false,
                attribution: Some(
                    "Universal Animation Library by Quaternius (courtesy credit)".to_string(),
                ),
            },
            modification: AssetModification {
                modified: false,
                description: None,
            },
            generation: None,
            files: vec![AssetFile {
                path: path.to_string(),
                sha256: hash.to_string(),
                distribution: AssetDistribution::BundledBinary,
            }],
        }
    }

    fn manifest(asset: AssetRecord) -> AssetManifest {
        AssetManifest {
            schema_version: ASSET_MANIFEST_SCHEMA_VERSION,
            assets: vec![asset],
        }
    }

    fn inventory(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(path, hash)| ((*path).to_string(), (*hash).to_string()))
            .collect()
    }

    #[test]
    fn valid_authored_asset_matches_inventory() {
        let manifest = manifest(authored_asset("apps/desktop/app-icon.png", HASH_A));
        let inventory = inventory(&[("apps/desktop/app-icon.png", HASH_A)]);

        assert!(validate_manifest_against_inventory(&manifest, &inventory).is_ok());
    }

    #[test]
    fn source_only_asset_does_not_require_binary_redistribution() {
        let mut asset = authored_asset("apps/desktop/app-icon.png", HASH_A);
        asset.license.binary_redistribution = false;
        let manifest = manifest(asset);
        let inventory = inventory(&[("apps/desktop/app-icon.png", HASH_A)]);

        assert!(validate_manifest_against_inventory(&manifest, &inventory).is_ok());
    }

    #[test]
    fn valid_third_party_asset_matches_inventory() {
        let path = "apps/desktop/public/animation/Idle_Loop.vrma";
        let manifest = manifest(third_party_asset(path, HASH_A));
        let inventory = inventory(&[(path, HASH_A)]);

        assert!(validate_manifest_against_inventory(&manifest, &inventory).is_ok());
    }

    #[test]
    fn inventory_drift_is_rejected() {
        let manifest = manifest(authored_asset("apps/desktop/app-icon.png", HASH_A));
        let added = inventory(&[
            ("apps/desktop/app-icon.png", HASH_A),
            ("apps/desktop/public/new.png", HASH_B),
        ]);
        let missing = BTreeMap::new();
        let changed = inventory(&[("apps/desktop/app-icon.png", HASH_B)]);

        assert!(validate_manifest_against_inventory(&manifest, &added).is_err());
        assert!(validate_manifest_against_inventory(&manifest, &missing).is_err());
        assert!(validate_manifest_against_inventory(&manifest, &changed).is_err());
    }

    #[test]
    fn unknown_or_non_redistributable_license_is_rejected() {
        let mut unknown = authored_asset("apps/desktop/app-icon.png", HASH_A);
        unknown.license.id = "UNKNOWN".to_string();
        let mut denied = authored_asset("apps/desktop/app-icon.png", HASH_A);
        denied.license.repository_redistribution = false;
        let inventory = inventory(&[("apps/desktop/app-icon.png", HASH_A)]);

        assert!(validate_manifest_against_inventory(&manifest(unknown), &inventory).is_err());
        assert!(validate_manifest_against_inventory(&manifest(denied), &inventory).is_err());
    }

    #[test]
    fn bundled_asset_requires_binary_redistribution_and_required_credit() {
        let path = "apps/desktop/public/animation/Idle_Loop.vrma";
        let inventory = inventory(&[(path, HASH_A)]);
        let mut denied = third_party_asset(path, HASH_A);
        denied.license.binary_redistribution = false;
        let mut missing_credit = third_party_asset(path, HASH_A);
        missing_credit.license.attribution_required = true;
        missing_credit.license.attribution = None;

        assert!(validate_manifest_against_inventory(&manifest(denied), &inventory).is_err());
        assert!(
            validate_manifest_against_inventory(&manifest(missing_credit), &inventory).is_err()
        );
    }

    #[test]
    fn generated_assets_require_generation_provenance() {
        let mut generated = authored_asset("apps/desktop/app-icon.png", HASH_A);
        generated.origin = AssetOrigin::Generated;
        generated.source.created_on = None;
        let inventory = inventory(&[("apps/desktop/app-icon.png", HASH_A)]);

        assert!(validate_manifest_against_inventory(&manifest(generated), &inventory).is_err());
    }

    #[test]
    fn generated_assets_accept_complete_generation_provenance() {
        let mut generated = authored_asset("apps/desktop/app-icon.png", HASH_A);
        generated.origin = AssetOrigin::GeneratedAssisted;
        generated.generation = Some(GenerationInfo {
            service: "Example service".to_string(),
            model: "Example model".to_string(),
            input_rights: "All inputs are owned by KingYoSun".to_string(),
            output_terms_url: "https://example.com/output-terms".to_string(),
        });
        let inventory = inventory(&[("apps/desktop/app-icon.png", HASH_A)]);

        assert!(validate_manifest_against_inventory(&manifest(generated), &inventory).is_ok());
    }

    #[test]
    fn unknown_origin_bad_schema_and_unsafe_path_are_rejected() {
        let inventory = inventory(&[("apps/desktop/app-icon.png", HASH_A)]);
        let mut unknown = authored_asset("apps/desktop/app-icon.png", HASH_A);
        unknown.origin = AssetOrigin::Unknown;
        let mut bad_schema = manifest(authored_asset("apps/desktop/app-icon.png", HASH_A));
        bad_schema.schema_version += 1;
        let unsafe_path = manifest(authored_asset("../app-icon.png", HASH_A));

        assert!(validate_manifest_against_inventory(&manifest(unknown), &inventory).is_err());
        assert!(validate_manifest_against_inventory(&bad_schema, &inventory).is_err());
        assert!(validate_manifest_against_inventory(&unsafe_path, &inventory).is_err());
    }

    #[test]
    fn duplicate_ids_and_paths_are_rejected() {
        let first = authored_asset("apps/desktop/app-icon.png", HASH_A);
        let mut duplicate_id = third_party_asset("apps/desktop/public/asset.vrma", HASH_B);
        duplicate_id.id = first.id.clone();
        let duplicate_id_manifest = AssetManifest {
            schema_version: ASSET_MANIFEST_SCHEMA_VERSION,
            assets: vec![first, duplicate_id],
        };
        let full_inventory = inventory(&[
            ("apps/desktop/app-icon.png", HASH_A),
            ("apps/desktop/public/asset.vrma", HASH_B),
        ]);
        assert!(
            validate_manifest_against_inventory(&duplicate_id_manifest, &full_inventory).is_err()
        );

        let first = authored_asset("apps/desktop/app-icon.png", HASH_A);
        let mut duplicate_path = third_party_asset("apps/desktop/app-icon.png", HASH_A);
        duplicate_path.id = "other-asset".to_string();
        let duplicate_path_manifest = AssetManifest {
            schema_version: ASSET_MANIFEST_SCHEMA_VERSION,
            assets: vec![first, duplicate_path],
        };
        let single_inventory = inventory(&[("apps/desktop/app-icon.png", HASH_A)]);
        assert!(
            validate_manifest_against_inventory(&duplicate_path_manifest, &single_inventory)
                .is_err()
        );
    }
}
