use super::*;

impl DesktopRuntime {
    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        let community_node_config = self.community_node_config.lock().await.clone();
        for node in community_node_config.nodes {
            if let Err(error) = self
                .refresh_community_node_registration_if_due(node.base_url.as_str())
                .await
            {
                tracing::warn!(
                    base_url = %node.base_url,
                    error = %error,
                    "failed to refresh community-node registration while loading sync status"
                );
            }
        }
        self.app_service.get_sync_status().await
    }

    pub async fn has_topic_timeline_doc_index_entry(
        &self,
        topic: &str,
        object_id: &str,
    ) -> Result<bool> {
        let replica = kukuri_docs_sync::topic_replica_id(topic);
        let current = self.iroh_stack.current.lock().await;
        let docs_sync = current
            .as_ref()
            .context("desktop runtime stack is not initialized")?
            .docs_sync
            .clone();
        drop(current);
        let rows = docs_sync
            .query_replica(&replica, DocQuery::Prefix("indexes/timeline/".into()))
            .await?;
        Ok(rows.iter().any(|row| row.key.ends_with(object_id)))
    }

    pub async fn get_discovery_config(&self) -> Result<DiscoveryConfig> {
        Ok(self.discovery_config.lock().await.clone())
    }

    pub async fn list_live_sessions(
        &self,
        request: ListLiveSessionsRequest,
    ) -> Result<Vec<LiveSessionView>> {
        self.app_service
            .list_live_sessions_scoped(request.topic.as_str(), request.scope)
            .await
    }

    pub async fn create_live_session(&self, request: CreateLiveSessionRequest) -> Result<String> {
        self.app_service
            .create_live_session_in_channel(
                request.topic.as_str(),
                request.channel_ref,
                CreateLiveSessionInput {
                    title: request.title,
                    description: request.description,
                },
            )
            .await
    }

    pub async fn end_live_session(&self, request: LiveSessionCommandRequest) -> Result<()> {
        self.app_service
            .end_live_session(request.topic.as_str(), request.session_id.as_str())
            .await
    }

    pub async fn join_live_session(&self, request: LiveSessionCommandRequest) -> Result<()> {
        self.app_service
            .join_live_session(request.topic.as_str(), request.session_id.as_str())
            .await
    }

    pub async fn leave_live_session(&self, request: LiveSessionCommandRequest) -> Result<()> {
        self.app_service
            .leave_live_session(request.topic.as_str(), request.session_id.as_str())
            .await
    }

    pub async fn list_game_rooms(
        &self,
        request: ListGameRoomsRequest,
    ) -> Result<Vec<GameRoomView>> {
        self.app_service
            .list_game_rooms_scoped(request.topic.as_str(), request.scope)
            .await
    }

    pub async fn create_game_room(&self, request: CreateGameRoomRequest) -> Result<String> {
        self.app_service
            .create_game_room_in_channel(
                request.topic.as_str(),
                request.channel_ref,
                CreateGameRoomInput {
                    title: request.title,
                    description: request.description,
                    participants: request.participants,
                },
            )
            .await
    }
}
