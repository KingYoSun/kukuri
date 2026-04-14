use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommunityNodeIdentityMode {
    DistinctUsers,
    SharedIdentity,
}

async fn wait_for_public_reaction_summary(
    runtime: &DesktopRuntime,
    topic: &str,
    object_id: &str,
    normalized_reaction_key: &str,
    expected_count: usize,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let timeline = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await?;
            if timeline.items.iter().any(|item| {
                item.object_id == object_id
                    && item.reaction_summary.iter().any(|entry| {
                        entry.normalized_reaction_key == normalized_reaction_key
                            && entry.count >= expected_count
                    })
            }) {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("public reaction propagation timeout")?
}

pub(crate) async fn run_community_node_connectivity(
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
    identity_mode: CommunityNodeIdentityMode,
) -> Result<HarnessResult> {
    unsafe { std::env::set_var("KUKURI_DISABLE_KEYRING", "1") };

    let step_timeout = ci_timeout_floor(
        Duration::from_millis(scenario.timeouts.step_ms),
        Duration::from_secs(180),
    );
    let overall_timeout = ci_timeout_floor(
        Duration::from_millis(scenario.timeouts.overall_ms),
        Duration::from_secs(600),
    );
    let stack = CommunityNodeStack::spawn(match identity_mode {
        CommunityNodeIdentityMode::DistinctUsers => "community_node_public_connectivity",
        CommunityNodeIdentityMode::SharedIdentity => "community_node_multi_device_connectivity",
    })
    .await?;

    let scenario_result = timeout(overall_timeout, async {
        cleanup_runtime_artifacts(&artifacts_dir.join("cn-desktop-a.db"))?;
        cleanup_runtime_artifacts(&artifacts_dir.join("cn-desktop-b.db"))?;

        let db_a = artifacts_dir.join("cn-desktop-a.db");
        let db_b = artifacts_dir.join("cn-desktop-b.db");
        if identity_mode == CommunityNodeIdentityMode::SharedIdentity {
            let shared_keys = KukuriKeys::generate();
            persist_runtime_identity(&db_a, &shared_keys)?;
            persist_runtime_identity(&db_b, &shared_keys)?;
        }
        let mut steps = Vec::new();

        let started_at = Instant::now();
        let runtime_a = DesktopRuntime::new_with_config(&db_a, TransportNetworkConfig::loopback())
            .await
            .context("failed to launch community-node desktop a")?;
        let runtime_b = DesktopRuntime::new_with_config(&db_b, TransportNetworkConfig::loopback())
            .await
            .context("failed to launch community-node desktop b")?;
        push_named_step(&mut steps, "launch_desktops", started_at);

        let started_at = Instant::now();
        runtime_a
            .set_community_node_config(SetCommunityNodeConfigRequest {
                base_urls: vec![stack.base_url.clone()],
            })
            .await
            .context("failed to configure community node for desktop a")?;
        runtime_b
            .set_community_node_config(SetCommunityNodeConfigRequest {
                base_urls: vec![stack.base_url.clone()],
            })
            .await
            .context("failed to configure community node for desktop b")?;
        push_named_step(&mut steps, "configure_community_node", started_at);

        let started_at = Instant::now();
        let authenticated_a = runtime_a
            .authenticate_community_node(CommunityNodeTargetRequest {
                base_url: stack.base_url.clone(),
            })
            .await
            .context("failed to authenticate desktop a with community node")?;
        let authenticated_b = runtime_b
            .authenticate_community_node(CommunityNodeTargetRequest {
                base_url: stack.base_url.clone(),
            })
            .await
            .context("failed to authenticate desktop b with community node")?;
        assert!(authenticated_a.auth_state.authenticated);
        assert!(authenticated_b.auth_state.authenticated);
        assert!(authenticated_a.resolved_urls.is_none());
        assert!(authenticated_b.resolved_urls.is_none());
        assert!(
            !authenticated_a
                .consent_state
                .as_ref()
                .expect("consent state for desktop a")
                .all_required_accepted
        );
        assert!(
            !authenticated_b
                .consent_state
                .as_ref()
                .expect("consent state for desktop b")
                .all_required_accepted
        );
        push_named_step(&mut steps, "authenticate", started_at);

        let started_at = Instant::now();
        let accepted_a = runtime_a
            .accept_community_node_consents(AcceptCommunityNodeConsentsRequest {
                base_url: stack.base_url.clone(),
                policy_slugs: Vec::new(),
            })
            .await
            .context("failed to accept community node consents for desktop a")?;
        let accepted_b = runtime_b
            .accept_community_node_consents(AcceptCommunityNodeConsentsRequest {
                base_url: stack.base_url.clone(),
                policy_slugs: Vec::new(),
            })
            .await
            .context("failed to accept community node consents for desktop b")?;
        assert!(
            accepted_a
                .consent_state
                .as_ref()
                .expect("accepted consent state a")
                .all_required_accepted
        );
        assert!(
            accepted_b
                .consent_state
                .as_ref()
                .expect("accepted consent state b")
                .all_required_accepted
        );
        assert_eq!(
            accepted_a
                .resolved_urls
                .as_ref()
                .expect("resolved urls a")
                .connectivity_urls,
            vec![stack.iroh_relay_url.clone()]
        );
        assert_eq!(
            accepted_b
                .resolved_urls
                .as_ref()
                .expect("resolved urls b")
                .connectivity_urls,
            vec![stack.iroh_relay_url.clone()]
        );
        assert!(!accepted_a.restart_required);
        assert!(!accepted_b.restart_required);
        push_named_step(&mut steps, "accept_consents", started_at);

        let started_at = Instant::now();
        let refreshed_a = runtime_a
            .get_community_node_statuses()
            .await
            .context("failed to load community-node status for desktop a after consent")?
            .into_iter()
            .next()
            .context("missing community-node status for desktop a after consent")?;
        let refreshed_b = runtime_b
            .get_community_node_statuses()
            .await
            .context("failed to load community-node status for desktop b after consent")?
            .into_iter()
            .next()
            .context("missing community-node status for desktop b after consent")?;
        assert!(refreshed_a.auth_state.authenticated);
        assert!(refreshed_b.auth_state.authenticated);
        assert!(!refreshed_a.restart_required);
        assert!(!refreshed_b.restart_required);
        assert_eq!(
            refreshed_a
                .resolved_urls
                .as_ref()
                .expect("resolved urls a after consent")
                .connectivity_urls,
            vec![stack.iroh_relay_url.clone()]
        );
        assert_eq!(
            refreshed_b
                .resolved_urls
                .as_ref()
                .expect("resolved urls b after consent")
                .connectivity_urls,
            vec![stack.iroh_relay_url.clone()]
        );
        let sync_a = runtime_a
            .get_sync_status()
            .await
            .context("failed to load sync status for desktop a after consent")?;
        let sync_b = runtime_b
            .get_sync_status()
            .await
            .context("failed to load sync status for desktop b after consent")?;
        match identity_mode {
            CommunityNodeIdentityMode::DistinctUsers => {
                assert_ne!(sync_a.local_author_pubkey, sync_b.local_author_pubkey);
            }
            CommunityNodeIdentityMode::SharedIdentity => {
                assert_eq!(sync_a.local_author_pubkey, sync_b.local_author_pubkey);
            }
        }
        assert_eq!(sync_a.discovery.connect_mode, ConnectMode::DirectOrRelay);
        assert_eq!(sync_b.discovery.connect_mode, ConnectMode::DirectOrRelay);
        push_named_step(&mut steps, "refresh_connectivity", started_at);

        let topic = scenario.fixtures.topic.as_str();
        let started_at = Instant::now();
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop a to scenario topic")?;
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop b to scenario topic")?;
        wait_for_topic_peer_count(&runtime_a, topic, 1, step_timeout)
            .await
            .context("desktop a did not observe initial topic peer connectivity")?;
        wait_for_topic_peer_count(&runtime_b, topic, 1, step_timeout)
            .await
            .context("desktop b did not observe initial community-node topic peer connectivity")?;
        push_named_step(&mut steps, "community_node_connectivity", started_at);

        if identity_mode == CommunityNodeIdentityMode::DistinctUsers {
            let started_at = Instant::now();
            wait_for_author_social_view(
                &runtime_a,
                sync_b.local_author_pubkey.as_str(),
                step_timeout,
            )
            .await
            .context("desktop a did not warm author social view for desktop b")?;
            wait_for_author_social_view(
                &runtime_b,
                sync_a.local_author_pubkey.as_str(),
                step_timeout,
            )
            .await
            .context("desktop b did not warm author social view for desktop a")?;
            runtime_a
                .follow_author(AuthorRequest {
                    pubkey: sync_b.local_author_pubkey.clone(),
                })
                .await
                .context("desktop a failed to follow desktop b for direct message")?;
            runtime_b
                .follow_author(AuthorRequest {
                    pubkey: sync_a.local_author_pubkey.clone(),
                })
                .await
                .context("desktop b failed to follow desktop a for direct message")?;
            wait_for_mutual_author_view_result(
                &runtime_a,
                sync_b.local_author_pubkey.as_str(),
                topic,
                step_timeout,
            )
            .await
            .context("desktop a did not observe mutual relationship for direct message")?;
            wait_for_mutual_author_view_result(
                &runtime_b,
                sync_a.local_author_pubkey.as_str(),
                topic,
                step_timeout,
            )
            .await
            .context("desktop b did not observe mutual relationship for direct message")?;
            runtime_a
                .open_direct_message(DirectMessageRequest {
                    pubkey: sync_b.local_author_pubkey.clone(),
                })
                .await
                .context("desktop a failed to open direct message in community-node lane")?;
            runtime_b
                .open_direct_message(DirectMessageRequest {
                    pubkey: sync_a.local_author_pubkey.clone(),
                })
                .await
                .context("desktop b failed to open direct message in community-node lane")?;
            let ticket_a = runtime_a
                .local_peer_ticket()
                .await
                .context("failed to load peer ticket for desktop a in community-node lane")?
                .context("missing peer ticket for desktop a in community-node lane")?;
            let ticket_b = runtime_b
                .local_peer_ticket()
                .await
                .context("failed to load peer ticket for desktop b in community-node lane")?
                .context("missing peer ticket for desktop b in community-node lane")?;
            wait_for_direct_message_pair_ready_with_refresh(
                &runtime_a,
                &runtime_b,
                ticket_a.as_str(),
                ticket_b.as_str(),
                sync_a.local_author_pubkey.as_str(),
                sync_b.local_author_pubkey.as_str(),
                step_timeout,
            )
            .await
            .context("community-node desktops did not connect direct message peers")?;
            let message_id = runtime_a
                .send_direct_message(SendDirectMessageRequest {
                    pubkey: sync_b.local_author_pubkey.clone(),
                    text: Some("community node direct message".to_string()),
                    reply_to_message_id: None,
                    attachments: Vec::new(),
                })
                .await
                .context("desktop a failed to send direct message in community-node lane")?;
            let delivered = wait_for_direct_message_result_with_pair_refresh(
                DirectMessagePairRefreshContext {
                    sender_runtime: &runtime_a,
                    sender_ticket: ticket_a.as_str(),
                    sender_peer_pubkey: sync_b.local_author_pubkey.as_str(),
                    receiver_runtime: &runtime_b,
                    receiver_ticket: ticket_b.as_str(),
                    receiver_peer_pubkey: sync_a.local_author_pubkey.as_str(),
                },
                message_id.as_str(),
                step_timeout,
            )
            .await
            .context("desktop b did not receive direct message in community-node lane")?;
            assert_eq!(delivered.text, "community node direct message");
            wait_for_direct_message_outbox_count(
                &runtime_a,
                sync_b.local_author_pubkey.as_str(),
                0,
                step_timeout,
            )
            .await
            .context("desktop a direct message outbox did not drain in community-node lane")?;
            push_named_step(&mut steps, "direct_message", started_at);
        }

        let started_at = Instant::now();
        wait_for_direct_topic_peer_count(&runtime_a, topic, 1, step_timeout)
            .await
            .context("desktop a did not observe direct public connectivity before public events")?;
        wait_for_direct_topic_peer_count(&runtime_b, topic, 1, step_timeout)
            .await
            .context("desktop b did not observe direct public connectivity before public events")?;
        let post_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "community node scenario post".to_string(),
                reply_to: None,
                channel_ref: ChannelRef::Public,
                attachments: Vec::new(),
            })
            .await
            .context("failed to create scenario post on desktop a")?;
        wait_for_topic_doc_index_entry(&runtime_a, topic, post_id.as_str(), step_timeout)
            .await
            .context("desktop a did not persist community post into docs index")?;
        wait_for_timeline_object(&runtime_b, topic, post_id.as_str(), step_timeout)
            .await
            .context("desktop b did not receive community post in timeline")?;
        push_named_step(&mut steps, "post", started_at);

        let started_at = Instant::now();
        runtime_b
            .toggle_reaction(ToggleReactionRequest {
                target_topic_id: topic.to_string(),
                target_object_id: post_id.clone(),
                reaction_key: ReactionKeyRequest::Emoji {
                    emoji: "🔥".to_string(),
                },
                channel_ref: None,
            })
            .await
            .context("failed to toggle scenario reaction on desktop b")?;
        wait_for_public_reaction_summary(
            &runtime_a,
            topic,
            post_id.as_str(),
            "emoji:🔥",
            1,
            step_timeout,
        )
        .await
        .context("desktop a did not receive community reaction summary")?;
        push_named_step(&mut steps, "reaction", started_at);

        let started_at = Instant::now();
        let repost_id = runtime_b
            .create_repost(CreateRepostRequest {
                topic: topic.to_string(),
                source_topic: topic.to_string(),
                source_object_id: post_id.clone(),
                commentary: None,
            })
            .await
            .context("failed to create scenario repost on desktop b")?;
        wait_for_timeline_object(&runtime_a, topic, repost_id.as_str(), step_timeout)
            .await
            .context("desktop a did not receive community repost in timeline")?;
        push_named_step(&mut steps, "repost", started_at);

        let started_at = Instant::now();
        let (reply_thread_attempts, reply_thread_timeout) = public_replication_retry_schedule(
            step_timeout,
            identity_mode == CommunityNodeIdentityMode::SharedIdentity,
        );
        let mut direct_reply_path_error = None;
        for attempt in 1..=reply_thread_attempts {
            match wait_for_direct_topic_peer_count(&runtime_b, topic, 1, reply_thread_timeout).await
            {
                Ok(()) => {
                    direct_reply_path_error = None;
                    break;
                }
                Err(error) if attempt < reply_thread_attempts => {
                    direct_reply_path_error = Some(format!("{error:#}"));
                    refresh_public_pair(&runtime_a, &runtime_b, topic, reply_thread_timeout)
                        .await
                        .context("failed to refresh public topic before community reply")?;
                    let _ = runtime_a
                        .list_thread(ListThreadRequest {
                            topic: topic.to_string(),
                            thread_id: post_id.clone(),
                            cursor: None,
                            limit: Some(20),
                        })
                        .await;
                    let _ = runtime_b
                        .list_thread(ListThreadRequest {
                            topic: topic.to_string(),
                            thread_id: post_id.clone(),
                            cursor: None,
                            limit: Some(20),
                        })
                        .await;
                    sleep(Duration::from_millis(250)).await;
                }
                Err(error) => {
                    direct_reply_path_error = Some(format!("{error:#}"));
                    break;
                }
            }
        }
        if let Some(error) = direct_reply_path_error {
            anyhow::bail!("desktop b did not observe direct public connectivity before community reply: {error}");
        }
        let reply_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "community node scenario reply".to_string(),
                reply_to: Some(post_id.clone()),
                channel_ref: ChannelRef::Public,
                attachments: Vec::new(),
            })
            .await
            .context("failed to create scenario reply on desktop b")?;
        wait_for_topic_doc_index_entry(&runtime_b, topic, reply_id.as_str(), reply_thread_timeout)
            .await
            .context("desktop b did not persist community reply into docs index")?;
        let mut reply_thread_error = None;
        for attempt in 1..=reply_thread_attempts {
            let attempt_result = async {
                wait_for_timeline_object(&runtime_a, topic, reply_id.as_str(), reply_thread_timeout)
                    .await
                    .context("desktop a did not receive community reply in timeline")?;
                wait_for_thread_object(
                    &runtime_a,
                    topic,
                    post_id.as_str(),
                    reply_id.as_str(),
                    reply_thread_timeout,
                )
                .await
                .context("desktop a did not receive community reply in thread")?;
                Ok::<(), anyhow::Error>(())
            }
            .await;
            match attempt_result {
                Ok(()) => {
                    reply_thread_error = None;
                    break;
                }
                Err(error) if attempt < reply_thread_attempts => {
                    reply_thread_error = Some(format!("{error:#}"));
                    refresh_public_pair(&runtime_a, &runtime_b, topic, reply_thread_timeout)
                        .await
                        .context("failed to refresh public topic after reply-thread timeout")?;
                    let _ = runtime_a
                        .list_thread(ListThreadRequest {
                            topic: topic.to_string(),
                            thread_id: post_id.clone(),
                            cursor: None,
                            limit: Some(20),
                        })
                        .await;
                    let _ = runtime_b
                        .list_thread(ListThreadRequest {
                            topic: topic.to_string(),
                            thread_id: post_id.clone(),
                            cursor: None,
                            limit: Some(20),
                        })
                        .await;
                    sleep(Duration::from_millis(250)).await;
                }
                Err(error) => {
                    reply_thread_error = Some(format!("{error:#}"));
                    break;
                }
            }
        }
        if let Some(error) = reply_thread_error {
            anyhow::bail!("desktop a did not receive community reply in thread: {error}");
        }
        push_named_step(&mut steps, "reply_thread", started_at);

        if identity_mode == CommunityNodeIdentityMode::DistinctUsers {
            let (public_feature_attempts, public_feature_timeout) =
                public_replication_retry_schedule(step_timeout, false);

            let started_at = Instant::now();
            let mut live_session = None;
            let mut live_session_error = None;
            for attempt in 1..=public_feature_attempts {
                let (live_owner, live_viewer, live_owner_label, live_viewer_label) =
                    select_public_feature_pair(
                        &runtime_a,
                        &runtime_b,
                        topic,
                        public_feature_timeout,
                        attempt,
                    )
                    .await?;
                let session_id = live_owner
                    .create_live_session(kukuri_desktop_runtime::CreateLiveSessionRequest {
                        topic: topic.to_string(),
                        channel_ref: ChannelRef::Public,
                        title: "community live".to_string(),
                        description: "live session".to_string(),
                    })
                    .await
                    .with_context(|| {
                        format!("failed to create live session on {live_owner_label}")
                    })?;
                wait_for_live_session(live_owner, topic, session_id.as_str(), step_timeout)
                    .await?;
                refresh_public_pair(&runtime_a, &runtime_b, topic, public_feature_timeout)
                    .await
                    .context("failed to refresh public topic after live-session creation")?;
                match wait_for_live_session(
                    live_viewer,
                    topic,
                    session_id.as_str(),
                    public_feature_timeout,
                )
                .await
                {
                    Ok(()) => {
                        live_session_error = None;
                        live_session = Some((
                            live_owner,
                            live_viewer,
                            live_owner_label,
                            live_viewer_label,
                            session_id,
                        ));
                        break;
                    }
                    Err(error) if attempt < public_feature_attempts => {
                        live_session_error = Some(format!("{error:#}"));
                        let _ = live_owner
                            .end_live_session(kukuri_desktop_runtime::LiveSessionCommandRequest {
                                topic: topic.to_string(),
                                session_id: session_id.clone(),
                            })
                            .await;
                        refresh_public_pair(&runtime_a, &runtime_b, topic, public_feature_timeout)
                            .await
                            .context("failed to refresh public topic after live-session timeout")?;
                        sleep(Duration::from_millis(250)).await;
                    }
                    Err(error) => {
                        live_session_error = Some(format!("{error:#}"));
                        break;
                    }
                }
            }
            let Some((live_owner, live_viewer, live_owner_label, live_viewer_label, session_id)) =
                live_session
            else {
                anyhow::bail!("community live session did not establish");
            };
            if let Some(error) = live_session_error {
                anyhow::bail!(
                    "{live_viewer_label} did not receive community live session from {live_owner_label}: {error}"
                );
            }
            live_viewer
                .join_live_session(kukuri_desktop_runtime::LiveSessionCommandRequest {
                    topic: topic.to_string(),
                    session_id: session_id.clone(),
                })
                .await
                .with_context(|| format!("failed to join live session on {live_viewer_label}"))?;
            let mut live_viewer_error = None;
            for attempt in 1..=public_feature_attempts {
                match wait_for_live_viewer_count(
                    live_owner,
                    topic,
                    session_id.as_str(),
                    1,
                    public_feature_timeout,
                )
                .await
                {
                    Ok(()) => {
                        live_viewer_error = None;
                        break;
                    }
                    Err(error) if attempt < public_feature_attempts => {
                        live_viewer_error = Some(format!("{error:#}"));
                        refresh_public_pair(&runtime_a, &runtime_b, topic, public_feature_timeout)
                            .await
                            .context("failed to refresh public topic after live-viewer timeout")?;
                        sleep(Duration::from_millis(250)).await;
                    }
                    Err(error) => {
                        live_viewer_error = Some(format!("{error:#}"));
                        break;
                    }
                }
            }
            if let Some(error) = live_viewer_error {
                anyhow::bail!(
                    "{live_owner_label} did not observe community live viewer count from {live_viewer_label}: {error}"
                );
            }
            live_owner
                .end_live_session(kukuri_desktop_runtime::LiveSessionCommandRequest {
                    topic: topic.to_string(),
                    session_id: session_id.clone(),
                })
                .await
                .with_context(|| format!("failed to end live session on {live_owner_label}"))?;
            wait_for_live_ended(live_owner, topic, session_id.as_str(), step_timeout).await?;
            let mut live_ended_error = None;
            for attempt in 1..=public_feature_attempts {
                match wait_for_live_ended(
                    live_viewer,
                    topic,
                    session_id.as_str(),
                    public_feature_timeout,
                )
                .await
                {
                    Ok(()) => {
                        live_ended_error = None;
                        break;
                    }
                    Err(error) if attempt < public_feature_attempts => {
                        live_ended_error = Some(format!("{error:#}"));
                        refresh_public_pair(&runtime_a, &runtime_b, topic, public_feature_timeout)
                            .await
                            .context("failed to refresh public topic after live-ended timeout")?;
                        sleep(Duration::from_millis(250)).await;
                    }
                    Err(error) => {
                        live_ended_error = Some(format!("{error:#}"));
                        break;
                    }
                }
            }
            if let Some(error) = live_ended_error {
                anyhow::bail!(
                    "{live_viewer_label} did not observe ended community live session from {live_owner_label}: {error}"
                );
            }
            push_named_step(&mut steps, "live", started_at);

            let started_at = Instant::now();
            let mut game_room = None;
            let mut game_room_error = None;
            for attempt in 1..=public_feature_attempts {
                let (game_owner, game_observer, game_owner_label, game_observer_label) =
                    select_public_feature_pair(
                        &runtime_a,
                        &runtime_b,
                        topic,
                        public_feature_timeout,
                        attempt,
                    )
                    .await?;
                let room_id = game_owner
                    .create_game_room(kukuri_desktop_runtime::CreateGameRoomRequest {
                        topic: topic.to_string(),
                        channel_ref: ChannelRef::Public,
                        title: "community finals".to_string(),
                        description: "set".to_string(),
                        participants: vec!["Alice".to_string(), "Bob".to_string()],
                    })
                    .await
                    .with_context(|| format!("failed to create game room on {game_owner_label}"))?;
                let room_owner =
                    wait_for_game_room(game_owner, topic, room_id.as_str(), step_timeout).await?;
                refresh_public_pair(&runtime_a, &runtime_b, topic, public_feature_timeout)
                    .await
                    .context("failed to refresh public topic after game-room creation")?;
                match wait_for_game_room(
                    game_observer,
                    topic,
                    room_id.as_str(),
                    public_feature_timeout,
                )
                .await
                {
                    Ok(_) => {
                        game_room_error = None;
                        game_room = Some((
                            game_owner,
                            game_observer,
                            game_owner_label,
                            game_observer_label,
                            room_id,
                            room_owner,
                        ));
                        break;
                    }
                    Err(error) if attempt < public_feature_attempts => {
                        game_room_error = Some(format!("{error:#}"));
                        refresh_public_pair(&runtime_a, &runtime_b, topic, public_feature_timeout)
                            .await
                            .context("failed to refresh public topic after game-room timeout")?;
                        sleep(Duration::from_millis(250)).await;
                    }
                    Err(error) => {
                        game_room_error = Some(format!("{error:#}"));
                        break;
                    }
                }
            }
            let Some((
                game_owner,
                game_observer,
                game_owner_label,
                game_observer_label,
                room_id,
                room_owner,
            )) = game_room
            else {
                anyhow::bail!("community game room did not establish");
            };
            if let Some(error) = game_room_error {
                anyhow::bail!(
                    "{game_observer_label} did not receive community game room from {game_owner_label}: {error}"
                );
            }
            let scores = room_owner
                .scores
                .iter()
                .map(|entry| {
                    let score = match entry.label.as_str() {
                        "Alice" => 2,
                        "Bob" => 1,
                        _ => entry.score,
                    };
                    GameScoreView {
                        participant_id: entry.participant_id.clone(),
                        label: entry.label.clone(),
                        score,
                    }
                })
                .collect();
            game_owner
                .update_game_room(kukuri_desktop_runtime::UpdateGameRoomRequest {
                    topic: topic.to_string(),
                    room_id: room_id.clone(),
                    status: GameRoomStatus::Running,
                    phase_label: Some("Round 1".to_string()),
                    scores,
                })
                .await
                .with_context(|| format!("failed to update game room on {game_owner_label}"))?;
            let mut game_score_error = None;
            for attempt in 1..=public_feature_attempts {
                match wait_for_game_score(
                    game_observer,
                    topic,
                    room_id.as_str(),
                    "Alice",
                    2,
                    public_feature_timeout,
                )
                .await
                {
                    Ok(()) => {
                        game_score_error = None;
                        break;
                    }
                    Err(error) if attempt < public_feature_attempts => {
                        game_score_error = Some(format!("{error:#}"));
                        refresh_public_pair(&runtime_a, &runtime_b, topic, public_feature_timeout)
                            .await
                            .context("failed to refresh public topic after game-score timeout")?;
                        sleep(Duration::from_millis(250)).await;
                    }
                    Err(error) => {
                        game_score_error = Some(format!("{error:#}"));
                        break;
                    }
                }
            }
            if let Some(error) = game_score_error {
                anyhow::bail!(
                    "{game_observer_label} did not observe community game score from {game_owner_label}: {error}"
                );
            }
            push_named_step(&mut steps, "game", started_at);
        }

        let started_at = Instant::now();
        shutdown_runtime(runtime_b, "desktop b reconnect pre-shutdown")
            .await
            .context("community-node reconnect shutdown timed out")?;
        let runtime_b = DesktopRuntime::new_with_config(&db_b, TransportNetworkConfig::loopback())
            .await
            .context("failed to restart community-node desktop b for reconnect")?;
        let _ = runtime_b
            .refresh_community_node_metadata(CommunityNodeTargetRequest {
                base_url: stack.base_url.clone(),
            })
            .await
            .context("failed to refresh community-node metadata for desktop b after restart")?;
        // Refresh the restarted peer first so the community node publishes its new
        // endpoint before the still-running peer rebuilds active topic subscriptions.
        let _ = runtime_a
            .refresh_community_node_metadata(CommunityNodeTargetRequest {
                base_url: stack.base_url.clone(),
            })
            .await
            .context("failed to refresh community-node metadata for desktop a after restart")?;
        // Refresh the restarted peer again after desktop A renews its own registration so
        // both desktops rebuild against the latest seed-peer endpoints.
        let _ = runtime_b
            .refresh_community_node_metadata(CommunityNodeTargetRequest {
                base_url: stack.base_url.clone(),
            })
            .await
            .context("failed to re-refresh community-node metadata for desktop b after restart")?;
        let reconnect_timeout = ci_timeout_floor(step_timeout, Duration::from_secs(360));
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to resubscribe desktop b to scenario topic after reconnect")?;
        refresh_public_pair(&runtime_a, &runtime_b, topic, reconnect_timeout)
            .await
            .context("failed to refresh public topic after desktop b restart")?;
        wait_for_direct_topic_peer_count_without_pending_join(
            &runtime_b,
            topic,
            1,
            reconnect_timeout,
        )
        .await
        .context("desktop b did not clear reconnect topic-join pending state")?;
        wait_for_topic_peer_count(&runtime_a, topic, 1, reconnect_timeout)
            .await
            .context("desktop a did not restore topic peer connectivity after desktop b restart")?;
        wait_for_topic_peer_count(&runtime_b, topic, 1, reconnect_timeout)
            .await
            .context("desktop b did not restore topic peer connectivity after restart")?;
        let _reconnect_probe_post = replicate_public_post_with_retry(
            &runtime_b,
            &runtime_a,
            topic,
            "community node reconnect probe",
            reconnect_timeout,
            PublicReplicationDirection::PreferOriginalPublisher,
            PublicReplicationLabels {
                failure: "reconnect probe post after restart",
                publisher: "desktop b",
                subscriber: "desktop a",
            },
        )
        .await?;
        wait_for_topic_peer_count(&runtime_a, topic, 1, reconnect_timeout)
            .await
            .context("desktop a lost topic peer connectivity after reconnect probe")?;
        wait_for_topic_peer_count(&runtime_b, topic, 1, reconnect_timeout)
            .await
            .context("desktop b lost topic peer connectivity after reconnect probe")?;
        let _reconnect_post = replicate_public_post_with_retry(
            &runtime_a,
            &runtime_b,
            topic,
            "community node reconnect",
            reconnect_timeout,
            PublicReplicationDirection::PreferDirectConnectedSubscriber,
            PublicReplicationLabels {
                failure: "reconnect post after restart",
                publisher: "desktop a",
                subscriber: "desktop b",
            },
        )
        .await?;
        let metrics_snapshot = if scenario.artifacts.metrics_snapshot {
            Some(
                runtime_b
                    .get_sync_status()
                    .await
                    .context("failed to collect final sync status for desktop b")?,
            )
        } else {
            None
        };
        shutdown_runtime(runtime_a, "desktop a final shutdown")
            .await
            .context("community-node desktop a final shutdown timed out")?;
        shutdown_runtime(runtime_b, "desktop b final shutdown")
            .await
            .context("community-node desktop b final shutdown timed out")?;
        push_named_step(&mut steps, "reconnect", started_at);

        let result = HarnessResult {
            status: HarnessStatus::Pass,
            scenario: scenario.name.clone(),
            steps,
            artifacts: vec![artifacts_dir.join("result.json").display().to_string()],
            metrics_snapshot,
        };
        write_result_artifact(Path::new("."), artifacts_dir, &result)?;
        Ok::<HarnessResult, anyhow::Error>(result)
    })
    .await
    .context("scenario exceeded overall timeout")
    .and_then(|result| result);

    let shutdown_result = timeout(Duration::from_secs(30), stack.shutdown())
        .await
        .context("community-node stack shutdown timed out")
        .and_then(|result| result);
    match (scenario_result, shutdown_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(scenario_error), Err(shutdown_error)) => Err(scenario_error.context(format!(
            "failed to tear down community-node stack after scenario error: {shutdown_error:#}"
        ))),
    }
}
