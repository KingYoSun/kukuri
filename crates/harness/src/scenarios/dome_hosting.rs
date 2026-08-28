use crate::*;

pub(crate) async fn run_dome_hosting_lifecycle(
    root: &Path,
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    let store = Arc::new(MemoryStore::default());
    let transport = Arc::new(FakeTransport::new(
        "harness-dome-host",
        FakeNetwork::default(),
    ));
    let app = AppService::new(store, transport);
    let context = SpatialContextV1::Topic {
        topic_id: TopicId::new(scenario.fixtures.topic.clone()),
    };
    let mut steps = Vec::new();

    let started = Instant::now();
    let instance_id = app
        .create_metaverse_room(
            &scenario.fixtures.topic,
            CreateMetaverseRoomInput {
                title: "Dome Hosting Harness".into(),
                description: "explicit lease lifecycle".into(),
                max_peers: Some(8),
            },
        )
        .await?;
    let owner = app
        .start_owner_dome_hosting(StartOwnerDomeHostingInput {
            spatial_context: context.clone(),
            instance_id: instance_id.clone(),
            endpoint_id: "harness-owner-device".into(),
            lease_duration_millis: 60_000,
        })
        .await?;
    anyhow::ensure!(owner.state.kind == DomeHostingStateKindV1::OwnerHosted);
    push_named_step(&mut steps, "owner_hosted", started);

    let started = Instant::now();
    app.submit_dome_session_input(SubmitDomeSessionInput {
        spatial_context: context.clone(),
        instance_id: instance_id.clone(),
        sequence: 1,
        input: DomeSessionInputKindV1::Join,
    })
    .await?;
    let room = app
        .list_game_rooms(&scenario.fixtures.topic)
        .await?
        .into_iter()
        .find(|room| room.room_id == instance_id)
        .context("created Dome room")?;
    let mut prop = room
        .metaverse
        .context("metaverse state")?
        .dome
        .customization
        .persistent_props
        .into_iter()
        .next()
        .context("default persistent prop")?;
    prop.position[0] += 250;
    app.submit_dome_session_input(SubmitDomeSessionInput {
        spatial_context: context.clone(),
        instance_id: instance_id.clone(),
        sequence: 2,
        input: DomeSessionInputKindV1::UpsertPersistentProp { prop },
    })
    .await?;
    let committed = app
        .commit_dome_layout(CommitDomeLayoutInput {
            spatial_context: context.clone(),
            instance_id: instance_id.clone(),
            operation_id: "harness-layout-commit-1".into(),
            signed_candidate_json: None,
        })
        .await?;
    anyhow::ensure!(committed.outcome == DomeLayoutCommitOutcome::Committed);
    anyhow::ensure!(committed.revision == 2);
    anyhow::ensure!(committed.hosting.state.kind == DomeHostingStateKindV1::OwnerHosted);
    push_named_step(&mut steps, "owner_layout_committed", started);

    let started = Instant::now();
    let node = KukuriKeys::generate();
    let transferring = app
        .prepare_community_node_dome_hosting(PrepareCommunityNodeDomeHostingInput {
            spatial_context: context.clone(),
            instance_id: instance_id.clone(),
            node_id: node.public_key_hex(),
            api_base_url: "https://community.example".into(),
            lease_duration_millis: 60_000,
        })
        .await?;
    anyhow::ensure!(transferring.state.kind == DomeHostingStateKindV1::Transferring);
    let unchanged = app.get_dome_hosting(context.clone(), &instance_id).await?;
    anyhow::ensure!(unchanged.state.kind == DomeHostingStateKindV1::Transferring);
    push_named_step(&mut steps, "owner_online_does_not_reclaim", started);

    let started = Instant::now();
    let lease: SignedDomeHostingLeaseV1 = serde_json::from_str(
        transferring
            .signed_lease_json
            .as_deref()
            .context("pending signed lease")?,
    )?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis()
        .try_into()?;
    let acceptance = accept_dome_hosting_lease(&node, &lease, "harness-cn-session", now)?;
    let active = app
        .activate_community_node_dome_hosting(ActivateCommunityNodeDomeHostingInput {
            spatial_context: context.clone(),
            instance_id: instance_id.clone(),
            signed_acceptance_json: serde_json::to_string(&acceptance)?,
        })
        .await?;
    anyhow::ensure!(active.state.kind == DomeHostingStateKindV1::CommunityNodeHosted);
    push_named_step(&mut steps, "community_node_activated", started);

    let started = Instant::now();
    let closed = app
        .close_dome_hosting(CloseDomeHostingInput {
            spatial_context: context,
            instance_id,
        })
        .await?;
    anyhow::ensure!(closed.state.kind == DomeHostingStateKindV1::Closed);
    push_named_step(&mut steps, "owner_closed", started);

    let result = HarnessResult {
        status: HarnessStatus::Pass,
        scenario: scenario.name.clone(),
        steps,
        artifacts: Vec::new(),
        metrics_snapshot: None,
    };
    write_result_artifact(root, artifacts_dir, &result)?;
    Ok(result)
}
