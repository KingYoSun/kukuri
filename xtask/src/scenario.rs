use anyhow::{Context, Result};

#[allow(unused_imports)]
use crate::*;

pub(crate) fn e2e_smoke(name: &str) -> Result<()> {
    run_timed_step(format!("scenario {name}"), || {
        let root = root_dir();
        let artifacts = artifacts_dir(name);
        let name = name.to_string();
        let handle = std::thread::Builder::new()
            .name(format!("scenario-{name}"))
            .stack_size(64 * 1024 * 1024)
            .spawn(move || -> Result<_> {
                let runtime =
                    tokio::runtime::Runtime::new().context("failed to build tokio runtime")?;
                runtime.block_on(kukuri_harness::run_named_scenario(
                    &root,
                    name.as_str(),
                    &artifacts,
                ))
            })
            .context("failed to spawn scenario runner thread")?;
        let result = handle
            .join()
            .map_err(|_| anyhow::anyhow!("scenario runner thread panicked"))??;
        let metrics = kukuri_harness::summarize_metrics(&result);
        for (key, value) in metrics {
            println!("{key}={value}");
        }
        Ok(())
    })
}

pub(crate) fn scenario(name: &str) -> Result<()> {
    if scenario_requires_cn_postgres(name) {
        with_cn_postgres(|| e2e_smoke(name))
    } else {
        e2e_smoke(name)
    }
}

pub(crate) fn scenario_requires_cn_postgres(name: &str) -> bool {
    if std::env::var_os("KUKURI_HARNESS_COMMUNITY_NODE_BASE_URL").is_some() {
        return false;
    }
    matches!(
        name,
        "community_node_public_connectivity" | "community_node_multi_device_connectivity"
    )
}

#[cfg(test)]
pub(crate) fn scenario_ci_lane(name: &str) -> Option<&'static str> {
    match name {
        "desktop_smoke_post_persist" => Some("fast+release+nightly"),
        "community_node_public_connectivity" => Some("fast+release+nightly"),
        "community_node_index_query_client" => Some("fast"),
        "community_node_report_routing" => Some("fast"),
        "community_node_trust_relation_client" => Some("fast"),
        "community_node_multi_device_connectivity" => Some("nightly"),
        "desktop_smoke_bookmark_workflow"
        | "desktop_smoke_game_room_persist"
        | "desktop_smoke_metaverse_dome_persist"
        | "desktop_smoke_live_session_persist"
        | "pairwise_dm_offline_text_image_video_delivery_and_local_delete"
        | "private_channel_invite_connectivity" => Some("nightly"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_scenario_yaml_has_a_ci_lane() {
        let scenario_dir = root_dir().join("harness").join("scenarios");
        let scenario_names = std::fs::read_dir(&scenario_dir)
            .expect("scenario directory")
            .map(|entry| {
                entry
                    .expect("scenario entry")
                    .path()
                    .file_stem()
                    .expect("scenario file stem")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(scenario_names.len(), 12, "scenario inventory changed");
        let orphaned = scenario_names
            .iter()
            .filter(|name| scenario_ci_lane(name).is_none())
            .collect::<Vec<_>>();
        assert!(orphaned.is_empty(), "orphaned scenarios: {orphaned:?}");
    }
}
