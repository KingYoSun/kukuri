use crate::*;

mod community_node;
mod community_node_index;
mod community_node_trust_relation;
mod desktop_smoke;
mod direct_message;
mod dome_hosting;
mod private_channel;

pub async fn run_named_scenario(
    root: &Path,
    name: &str,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    let path = root
        .join("harness")
        .join("scenarios")
        .join(format!("{name}.yaml"));
    let scenario = load_scenario(&path)?;
    run_scenario(root, &scenario, artifacts_dir).await
}

pub async fn run_scenario(
    root: &Path,
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    std::fs::create_dir_all(artifacts_dir)
        .with_context(|| format!("failed to create artifacts dir {}", artifacts_dir.display()))?;

    match scenario.kind {
        ScenarioKind::DesktopSmoke => {
            desktop_smoke::run_desktop_smoke_scenario(root, scenario, artifacts_dir).await
        }
        ScenarioKind::CommunityNodePublicConnectivity => {
            community_node::run_community_node_connectivity(
                scenario,
                artifacts_dir,
                community_node::CommunityNodeIdentityMode::DistinctUsers,
            )
            .await
        }
        ScenarioKind::CommunityNodeMultiDeviceConnectivity => {
            community_node::run_community_node_connectivity(
                scenario,
                artifacts_dir,
                community_node::CommunityNodeIdentityMode::SharedIdentity,
            )
            .await
        }
        ScenarioKind::PrivateChannelInviteConnectivity => {
            private_channel::run_private_channel_invite_connectivity(scenario, artifacts_dir).await
        }
        ScenarioKind::PairwiseDirectMessageConnectivity => {
            direct_message::run_pairwise_direct_message_connectivity(scenario, artifacts_dir).await
        }
        ScenarioKind::CommunityNodeIndexQueryClient => {
            community_node_index::run_community_node_index_query_client(
                root,
                scenario,
                artifacts_dir,
            )
            .await
        }
        ScenarioKind::CommunityNodeReportRouting => {
            community_node_index::run_community_node_index_query_client(
                root,
                scenario,
                artifacts_dir,
            )
            .await
        }
        ScenarioKind::CommunityNodeTrustRelationClient => {
            community_node_trust_relation::run_community_node_trust_relation_client(
                root,
                scenario,
                artifacts_dir,
            )
            .await
        }
        ScenarioKind::DomeHostingLifecycle => {
            dome_hosting::run_dome_hosting_lifecycle(root, scenario, artifacts_dir).await
        }
    }
}
