use crate::*;

pub(crate) async fn run_desktop_smoke_scenario(
    root: &Path,
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    let db_path = artifacts_dir.join("scenario.db");
    if db_path.exists() {
        std::fs::remove_file(&db_path)
            .with_context(|| format!("failed to remove stale db {}", db_path.display()))?;
    }

    let mut runtime = ScenarioRuntime {
        db_path,
        network: FakeNetwork::default(),
        app: None,
        current_topic: None,
        current_channel_id: None,
        private_channels: BTreeMap::new(),
    };
    let overall_timeout = Duration::from_millis(scenario.timeouts.overall_ms);
    let step_timeout = Duration::from_millis(scenario.timeouts.step_ms);

    timeout(overall_timeout, async {
        let mut steps = Vec::new();

        for step in &scenario.steps {
            let started_at = Instant::now();
            match step {
                ScenarioStep::LaunchDesktop => runtime.launch().await?,
                ScenarioStep::SelectTopic { topic } => {
                    runtime.current_topic = Some(topic.clone());
                    runtime.current_channel_id = None;
                    let _ = runtime
                        .app()?
                        .list_timeline_scoped(topic, TimelineScope::Public, None, 50)
                        .await?;
                }
                ScenarioStep::SelectPublicTimeline => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    runtime.current_channel_id = None;
                    let _ = runtime
                        .app()?
                        .list_timeline_scoped(&topic, TimelineScope::Public, None, 50)
                        .await?;
                }
                ScenarioStep::CreatePrivateChannel { label } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let channel = runtime
                        .app()?
                        .create_private_channel(CreatePrivateChannelInput {
                            topic_id: TopicId::new(topic.clone()),
                            label: label.clone(),
                            audience_kind: ChannelAudienceKind::InviteOnly,
                        })
                        .await?;
                    runtime
                        .private_channels
                        .insert(label.clone(), channel.channel_id.clone());
                }
                ScenarioStep::SelectPrivateChannel { label } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let channel_id = runtime
                        .private_channels
                        .get(label.as_str())
                        .cloned()
                        .with_context(|| format!("private channel not found for label: {label}"))?;
                    runtime.current_channel_id = Some(channel_id.clone());
                    let _ = runtime
                        .app()?
                        .list_timeline_scoped(
                            &topic,
                            TimelineScope::Channel {
                                channel_id: ChannelId::new(channel_id),
                            },
                            None,
                            50,
                        )
                        .await?;
                }
                ScenarioStep::CreatePost { content } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    match runtime.current_channel_id.clone() {
                        Some(channel_id) => {
                            runtime
                                .app()?
                                .create_post_in_channel(
                                    &topic,
                                    ChannelRef::PrivateChannel {
                                        channel_id: ChannelId::new(channel_id),
                                    },
                                    content,
                                    None,
                                )
                                .await?;
                        }
                        None => {
                            runtime.app()?.create_post(&topic, content, None).await?;
                        }
                    }
                }
                ScenarioStep::AssertTimelineContains { text } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let scope = runtime.current_scope();
                    let assertion = timeout(step_timeout, async {
                        loop {
                            let timeline = runtime
                                .app()?
                                .list_timeline_scoped(&topic, scope.clone(), None, 50)
                                .await?;
                            if timeline.items.iter().any(|item| item.content == *text) {
                                return Ok::<(), anyhow::Error>(());
                            }
                            sleep(Duration::from_millis(50)).await;
                        }
                    });
                    assertion.await.context("assertion timeout")??;
                }
                ScenarioStep::BookmarkPost { content } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let timeline = runtime
                        .app()?
                        .list_timeline_scoped(&topic, runtime.current_scope(), None, 50)
                        .await?;
                    let post = timeline
                        .items
                        .iter()
                        .find(|item| item.content == *content)
                        .with_context(|| format!("bookmark target not found in timeline: {content}"))?;
                    runtime
                        .app()?
                        .bookmark_post(&topic, post.object_id.as_str())
                        .await?;
                }
                ScenarioStep::AssertBookmarkListContains { text } => {
                    let expected = text.clone();
                    let assertion = timeout(step_timeout, async {
                        loop {
                            let bookmarks = runtime.app()?.list_bookmarked_posts().await?;
                            if bookmarks
                                .iter()
                                .any(|item| item.post.content == expected)
                            {
                                return Ok::<(), anyhow::Error>(());
                            }
                            sleep(Duration::from_millis(50)).await;
                        }
                    });
                    assertion.await.context("assertion timeout")??;
                }
                ScenarioStep::AssertBookmarkListMissing { text } => {
                    let bookmarks = runtime.app()?.list_bookmarked_posts().await?;
                    if bookmarks.iter().any(|item| item.post.content == *text) {
                        anyhow::bail!("bookmark still present: {text}");
                    }
                }
                ScenarioStep::RemoveBookmark { text } => {
                    let bookmarks = runtime.app()?.list_bookmarked_posts().await?;
                    let bookmarked = bookmarks
                        .iter()
                        .find(|item| item.post.content == *text)
                        .with_context(|| format!("bookmarked post not found: {text}"))?;
                    runtime
                        .app()?
                        .remove_bookmarked_post(bookmarked.post.object_id.as_str())
                        .await?;
                }
                ScenarioStep::CreateLiveSession { title, description } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    runtime
                        .app()?
                        .create_live_session(
                            &topic,
                            CreateLiveSessionInput {
                                title: title.clone(),
                                description: description.clone(),
                            },
                        )
                        .await?;
                }
                ScenarioStep::JoinLiveSession { title } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let session = runtime
                        .app()?
                        .list_live_sessions(&topic)
                        .await?
                        .into_iter()
                        .find(|session| session.title == *title)
                        .with_context(|| format!("live session not found: {title}"))?;
                    runtime
                        .app()?
                        .join_live_session(&topic, session.session_id.as_str())
                        .await?;
                }
                ScenarioStep::AssertLiveViewerCount { title, viewer_count } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let expected = *viewer_count;
                    let target = title.clone();
                    let assertion = timeout(step_timeout, async {
                        loop {
                            let sessions = runtime.app()?.list_live_sessions(&topic).await?;
                            if sessions
                                .iter()
                                .any(|session| session.title == target && session.viewer_count == expected)
                            {
                                return Ok::<(), anyhow::Error>(());
                            }
                            sleep(Duration::from_millis(50)).await;
                        }
                    });
                    match assertion.await {
                        Ok(result) => result?,
                        Err(_) => {
                            let sessions = runtime.app()?.list_live_sessions(&topic).await?;
                            let observed = sessions
                                .iter()
                                .map(|session| {
                                    format!(
                                        "{}:{}:{}",
                                        session.title, session.viewer_count, session.joined_by_me
                                    )
                                })
                                .collect::<Vec<_>>();
                            anyhow::bail!(
                                "assertion timeout for live viewer count title={target} expected={expected} observed={observed:?}"
                            );
                        }
                    }
                }
                ScenarioStep::EndLiveSession { title } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let session = runtime
                        .app()?
                        .list_live_sessions(&topic)
                        .await?
                        .into_iter()
                        .find(|session| session.title == *title)
                        .with_context(|| format!("live session not found: {title}"))?;
                    runtime
                        .app()?
                        .end_live_session(&topic, session.session_id.as_str())
                        .await?;
                }
                ScenarioStep::CreateGameRoom {
                    title,
                    description,
                    participants,
                } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    runtime
                        .app()?
                        .create_game_room(
                            &topic,
                            CreateGameRoomInput {
                                title: title.clone(),
                                description: description.clone(),
                                participants: participants.clone(),
                            },
                        )
                        .await?;
                }
                ScenarioStep::UpdateGameRoom {
                    title,
                    status,
                    phase_label,
                    scores,
                } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let room = runtime
                        .app()?
                        .list_game_rooms(&topic)
                        .await?
                        .into_iter()
                        .find(|room| room.title == *title)
                        .with_context(|| format!("game room not found: {title}"))?;
                    let next_scores = room
                        .scores
                        .iter()
                        .map(|score| {
                            let next = scores
                                .iter()
                                .find(|update| update.label == score.label)
                                .map(|update| update.score)
                                .unwrap_or(score.score);
                            GameScoreView {
                                participant_id: score.participant_id.clone(),
                                label: score.label.clone(),
                                score: next,
                            }
                        })
                        .collect::<Vec<_>>();
                    runtime
                        .app()?
                        .update_game_room(
                            &topic,
                            room.room_id.as_str(),
                            UpdateGameRoomInput {
                                status: parse_game_status(status.as_str())?,
                                phase_label: phase_label.clone(),
                                scores: next_scores,
                            },
                        )
                        .await?;
                }
                ScenarioStep::AssertGameScore { title, label, score } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let expected_title = title.clone();
                    let expected_label = label.clone();
                    let expected_score = *score;
                    let assertion = timeout(step_timeout, async {
                        loop {
                            let rooms = runtime.app()?.list_game_rooms(&topic).await?;
                            if rooms.iter().any(|room| {
                                room.title == expected_title
                                    && room.scores.iter().any(|entry| {
                                        entry.label == expected_label && entry.score == expected_score
                                    })
                            }) {
                                return Ok::<(), anyhow::Error>(());
                            }
                            sleep(Duration::from_millis(50)).await;
                        }
                    });
                    assertion.await.context("assertion timeout")??;
                }
                ScenarioStep::CreateMetaverseDome {
                    title,
                    description,
                    max_peers,
                } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    runtime
                        .app()?
                        .create_metaverse_room(
                            &topic,
                            CreateMetaverseRoomInput {
                                title: title.clone(),
                                description: description.clone(),
                                max_peers: *max_peers,
                            },
                        )
                        .await?;
                }
                ScenarioStep::CustomizeMetaverseDome {
                    title,
                    gravity_milli,
                    wall_material,
                    prop_position,
                } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let room = runtime
                        .app()?
                        .list_game_rooms(&topic)
                        .await?
                        .into_iter()
                        .find(|room| room.title == *title)
                        .with_context(|| format!("metaverse Dome not found: {title}"))?;
                    let mut customization = room
                        .metaverse
                        .as_ref()
                        .context("metaverse Dome state missing")?
                        .dome
                        .customization
                        .clone();
                    customization.environment.gravity_milli = *gravity_milli;
                    customization.surface.wall_material = parse_dome_material(wall_material)?;
                    customization.persistent_props[0].position = *prop_position;
                    runtime
                        .app()?
                        .update_metaverse_room(
                            &topic,
                            room.room_id.as_str(),
                            UpdateMetaverseRoomInput {
                                status: GameRoomStatus::Running,
                                customization,
                            },
                        )
                        .await?;
                }
                ScenarioStep::AssertMetaverseDome {
                    title,
                    gravity_milli,
                    wall_material,
                    prop_position,
                } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let expected_material = parse_dome_material(wall_material)?;
                    let room = runtime
                        .app()?
                        .list_game_rooms(&topic)
                        .await?
                        .into_iter()
                        .find(|room| room.title == *title)
                        .with_context(|| format!("metaverse Dome not found: {title}"))?;
                    let dome = &room
                        .metaverse
                        .as_ref()
                        .context("metaverse Dome state missing")?
                        .dome;
                    if dome.spec_id != "fixed_dome_v1"
                        || dome.customization.environment.gravity_milli != *gravity_milli
                        || dome.customization.surface.wall_material != expected_material
                        || dome.customization.persistent_props[0].position != *prop_position
                    {
                        anyhow::bail!("metaverse Dome customization mismatch for {title}");
                    }
                }
                ScenarioStep::AssertMetaverseDomeRejectsInvalid { title } => {
                    let topic = runtime.topic_or_default(&scenario.fixtures.topic);
                    let before = runtime
                        .app()?
                        .list_game_rooms(&topic)
                        .await?
                        .into_iter()
                        .find(|room| room.title == *title)
                        .with_context(|| format!("metaverse Dome not found: {title}"))?;
                    let mut customization = before
                        .metaverse
                        .as_ref()
                        .context("metaverse Dome state missing")?
                        .dome
                        .customization
                        .clone();
                    customization.environment.gravity_milli = 999;
                    if runtime
                        .app()?
                        .update_metaverse_room(
                            &topic,
                            before.room_id.as_str(),
                            UpdateMetaverseRoomInput {
                                status: GameRoomStatus::Running,
                                customization,
                            },
                        )
                        .await
                        .is_ok()
                    {
                        anyhow::bail!("invalid metaverse Dome customization was accepted");
                    }
                    let after = runtime
                        .app()?
                        .list_game_rooms(&topic)
                        .await?
                        .into_iter()
                        .find(|room| room.title == *title)
                        .context("metaverse Dome disappeared after invalid update")?;
                    let after_gravity = after
                        .metaverse
                        .as_ref()
                        .context("metaverse Dome state missing after invalid update")?
                        .dome
                        .customization
                        .environment
                        .gravity_milli;
                    if after_gravity == 999 {
                        anyhow::bail!("invalid metaverse Dome customization was persisted");
                    }
                }
                ScenarioStep::RestartDesktop => {
                    runtime.app.take();
                    runtime.launch().await?;
                }
                other => anyhow::bail!(
                    "unsupported step for desktop smoke scenario: {}",
                    step_name(other)
                ),
            }

            steps.push(StepResult {
                action: step_name(step).to_string(),
                duration_ms: started_at.elapsed().as_millis(),
            });
        }

        let metrics_snapshot = if scenario.artifacts.metrics_snapshot {
            Some(runtime.app()?.get_sync_status().await?)
        } else {
            None
        };
        let result = HarnessResult {
            status: HarnessStatus::Pass,
            scenario: scenario.name.clone(),
            steps,
            artifacts: vec![artifacts_dir.join("result.json").display().to_string()],
            metrics_snapshot,
        };

        write_result_artifact(root, artifacts_dir, &result)?;
        Ok::<HarnessResult, anyhow::Error>(result)
    })
    .await
    .context("scenario exceeded overall timeout")?
}

fn parse_dome_material(value: &str) -> Result<DomeMaterialPreset> {
    match value {
        "concrete" => Ok(DomeMaterialPreset::Concrete),
        "stone" => Ok(DomeMaterialPreset::Stone),
        "metal" => Ok(DomeMaterialPreset::Metal),
        "wood" => Ok(DomeMaterialPreset::Wood),
        _ => anyhow::bail!("unsupported Dome material: {value}"),
    }
}
