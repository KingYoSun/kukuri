use super::*;

pub(crate) async fn current_mutual_direct_message_peers(
    store: &dyn Store,
    local_author_pubkey: &str,
) -> Result<BTreeSet<String>> {
    let following = store
        .list_follow_edges_by_subject(local_author_pubkey)
        .await?
        .into_iter()
        .filter(|edge| edge.status == FollowEdgeStatus::Active)
        .map(|edge| edge.target_pubkey.as_str().to_string())
        .collect::<BTreeSet<_>>();
    let followed_by = store
        .list_follow_edges_by_target(local_author_pubkey)
        .await?
        .into_iter()
        .filter(|edge| edge.status == FollowEdgeStatus::Active)
        .map(|edge| edge.subject_pubkey.as_str().to_string())
        .collect::<BTreeSet<_>>();
    Ok(following.intersection(&followed_by).cloned().collect())
}

pub(crate) async fn stop_direct_message_subscription(
    direct_message_subscriptions: &Mutex<HashMap<String, JoinHandle<()>>>,
    services: &ServiceHandles,
    peer_pubkey: &str,
) -> Result<()> {
    let peer_pubkey = normalize_author_pubkey(peer_pubkey)?;
    if let Some(handle) = direct_message_subscriptions
        .lock()
        .await
        .remove(peer_pubkey.as_str())
    {
        handle.abort();
    }
    let topic =
        derive_direct_message_topic(services.keys.as_ref(), &Pubkey::from(peer_pubkey.as_str()))?;
    services.hint_transport.unsubscribe_hints(&topic).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn schedule_direct_message_reconcile(
    services: ServiceHandles,
    last_sync: Arc<Mutex<Option<i64>>>,
    direct_message_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    notification_inserted: Arc<tokio::sync::Notify>,
    local_author_pubkey: String,
    author_pubkey: String,
) {
    tokio::spawn(async move {
        if let Err(error) = reconcile_direct_message_subscriptions(
            services,
            last_sync,
            direct_message_subscriptions,
            notification_inserted,
            local_author_pubkey.as_str(),
        )
        .await
        {
            warn!(
                author_pubkey = %author_pubkey,
                error = %error,
                "failed to reconcile direct message subscriptions after author update"
            );
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn reconcile_direct_message_subscriptions(
    services: ServiceHandles,
    last_sync: Arc<Mutex<Option<i64>>>,
    direct_message_subscriptions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    notification_inserted: Arc<tokio::sync::Notify>,
    local_author_pubkey: &str,
) -> Result<()> {
    let desired_peers =
        current_mutual_direct_message_peers(services.store.as_ref(), local_author_pubkey).await?;
    let current_entries = {
        let subscriptions = direct_message_subscriptions.lock().await;
        subscriptions
            .iter()
            .map(|(peer_pubkey, handle)| (peer_pubkey.clone(), handle.is_finished()))
            .collect::<Vec<_>>()
    };

    for (peer_pubkey, finished) in &current_entries {
        if *finished || !desired_peers.contains(peer_pubkey) {
            stop_direct_message_subscription(
                direct_message_subscriptions.as_ref(),
                &services,
                peer_pubkey.as_str(),
            )
            .await?;
        }
    }

    for peer_pubkey in desired_peers {
        AppService::spawn_direct_message_subscription(
            Arc::clone(&direct_message_subscriptions),
            services.clone(),
            Arc::clone(&last_sync),
            Arc::clone(&notification_inserted),
            local_author_pubkey,
            peer_pubkey.as_str(),
        )
        .await?;
    }
    Ok(())
}

pub(crate) async fn rebuild_author_relationships(
    store: &dyn Store,
    projection_store: &dyn ProjectionStore,
    local_author_pubkey: &str,
) -> Result<()> {
    let following_edges = store
        .list_follow_edges_by_subject(local_author_pubkey)
        .await?
        .into_iter()
        .filter(|edge| edge.status == FollowEdgeStatus::Active)
        .collect::<Vec<_>>();
    let followed_by_edges = store
        .list_follow_edges_by_target(local_author_pubkey)
        .await?
        .into_iter()
        .filter(|edge| edge.status == FollowEdgeStatus::Active)
        .collect::<Vec<_>>();

    let following = following_edges
        .iter()
        .map(|edge| edge.target_pubkey.as_str().to_string())
        .collect::<BTreeSet<_>>();
    let followed_by = followed_by_edges
        .iter()
        .map(|edge| edge.subject_pubkey.as_str().to_string())
        .collect::<BTreeSet<_>>();

    let mut friend_of_friend_via = BTreeMap::<String, BTreeSet<String>>::new();
    for via_author in &following {
        for edge in store
            .list_follow_edges_by_subject(via_author.as_str())
            .await?
        {
            if edge.status != FollowEdgeStatus::Active {
                continue;
            }
            let target = edge.target_pubkey.as_str();
            if target == local_author_pubkey || following.contains(target) {
                continue;
            }
            friend_of_friend_via
                .entry(target.to_string())
                .or_default()
                .insert(via_author.clone());
        }
    }

    let derived_at = Utc::now().timestamp_millis();
    let mut author_pubkeys = BTreeSet::new();
    author_pubkeys.extend(following.iter().cloned());
    author_pubkeys.extend(followed_by.iter().cloned());
    author_pubkeys.extend(friend_of_friend_via.keys().cloned());
    author_pubkeys.remove(local_author_pubkey);

    let rows = author_pubkeys
        .into_iter()
        .map(|author_pubkey| {
            let following_flag = following.contains(author_pubkey.as_str());
            let followed_by_flag = followed_by.contains(author_pubkey.as_str());
            let via_pubkeys = friend_of_friend_via
                .get(author_pubkey.as_str())
                .map(|values| values.iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            AuthorRelationshipProjectionRow {
                local_author_pubkey: local_author_pubkey.to_string(),
                author_pubkey: author_pubkey.clone(),
                following: following_flag,
                followed_by: followed_by_flag,
                mutual: following_flag && followed_by_flag,
                friend_of_friend: !following_flag && !via_pubkeys.is_empty(),
                friend_of_friend_via_pubkeys: via_pubkeys,
                derived_at,
            }
        })
        .collect::<Vec<_>>();
    projection_store
        .rebuild_author_relationships(local_author_pubkey, rows)
        .await
}
