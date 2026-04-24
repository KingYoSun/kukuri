use super::*;

impl DesktopRuntime {
    pub async fn get_community_node_config(&self) -> Result<CommunityNodeConfig> {
        Ok(self.community_node_config.lock().await.clone())
    }

    pub async fn get_community_node_statuses(&self) -> Result<Vec<CommunityNodeNodeStatus>> {
        let config = self.community_node_config.lock().await.clone();
        let mut statuses = Vec::with_capacity(config.nodes.len());
        for node in config.nodes {
            let base_url = node.base_url.clone();
            let _ = self
                .refresh_community_node_registration_if_due(base_url.as_str())
                .await;
            let current_node = self
                .community_node_config
                .lock()
                .await
                .nodes
                .iter()
                .find(|candidate| candidate.base_url == base_url)
                .cloned()
                .unwrap_or(node);
            statuses.push(self.community_node_status(current_node, None, None).await?);
        }
        Ok(statuses)
    }

    pub async fn set_community_node_config(
        &self,
        request: SetCommunityNodeConfigRequest,
    ) -> Result<CommunityNodeConfig> {
        let current_config = self.community_node_config.lock().await.clone();
        let nodes = request
            .nodes
            .into_iter()
            .map(|base_url| -> Result<CommunityNodeNodeConfig> {
                let normalized_base_url = normalize_http_url(base_url.base_url.as_str())?;
                let resolved_urls = current_config
                    .nodes
                    .iter()
                    .find(|node| node.base_url == normalized_base_url)
                    .and_then(|node| node.resolved_urls.clone());
                Ok(CommunityNodeNodeConfig {
                    base_url: normalized_base_url,
                    auto_approve: base_url.auto_approve,
                    resolved_urls,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let next_config = normalize_community_node_config(CommunityNodeConfig { nodes })?;
        save_community_node_config(&self.db_path, &next_config)?;
        *self.community_node_config.lock().await = next_config.clone();
        self.community_node_heartbeat_deadlines.lock().await.clear();
        self.community_node_metadata_refresh_deadlines
            .lock()
            .await
            .clear();
        self.community_node_session_retry_deadlines
            .lock()
            .await
            .clear();
        self.community_node_session_phases.lock().await.clear();
        self.community_node_ready_refresh_pending
            .lock()
            .await
            .clear();
        self.community_node_last_errors.lock().await.clear();
        self.community_node_cached_consents.lock().await.clear();
        self.apply_runtime_connectivity_assist().await?;
        self.apply_effective_seed_peers().await?;
        Ok(next_config)
    }

    pub async fn clear_community_node_config(&self) -> Result<()> {
        let existing = self.community_node_config.lock().await.clone();
        for node in existing.nodes {
            self.clear_community_node_token(CommunityNodeTargetRequest {
                base_url: node.base_url,
            })
            .await?;
        }
        save_community_node_config(&self.db_path, &CommunityNodeConfig::default())?;
        *self.community_node_config.lock().await = CommunityNodeConfig::default();
        self.community_node_heartbeat_deadlines.lock().await.clear();
        self.community_node_metadata_refresh_deadlines
            .lock()
            .await
            .clear();
        self.community_node_session_retry_deadlines
            .lock()
            .await
            .clear();
        self.community_node_session_phases.lock().await.clear();
        self.community_node_ready_refresh_pending
            .lock()
            .await
            .clear();
        self.community_node_last_errors.lock().await.clear();
        self.community_node_cached_consents.lock().await.clear();
        self.apply_runtime_connectivity_assist().await?;
        self.apply_effective_seed_peers().await?;
        Ok(())
    }

    pub async fn authenticate_community_node(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let node = self.require_community_node(base_url.as_str()).await?;
        self.set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::Authenticating,
        )
        .await;
        let mut token = self
            .request_community_node_authentication_token(base_url.as_str())
            .await?;
        let mut consent_state = self
            .fetch_community_node_consent_status_with_retry(base_url.as_str(), &mut token, false)
            .await?;
        self.set_community_node_cached_consent(base_url.as_str(), Some(consent_state.clone()))
            .await;
        if !consent_state.all_required_accepted && node.auto_approve {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Accepting,
            )
            .await;
            consent_state = self
                .accept_community_node_consents_with_retry(base_url.as_str(), &mut token, &[])
                .await?;
            self.set_community_node_cached_consent(base_url.as_str(), Some(consent_state.clone()))
                .await;
        }
        if consent_state.all_required_accepted {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Refreshing,
            )
            .await;
            self.refresh_community_node_registration_with_token_if_due(
                base_url.as_str(),
                &mut token,
                node.auto_approve,
                false,
            )
            .await?;
            self.clear_community_node_retry_state(base_url.as_str())
                .await;
            self.set_community_node_session_ready(base_url.as_str(), true)
                .await;
            let refreshed = self.require_community_node(base_url.as_str()).await?;
            return self
                .community_node_status(refreshed, Some(consent_state), None)
                .await;
        }
        self.clear_community_node_retry_state(base_url.as_str())
            .await;
        self.set_community_node_session_phase(base_url.as_str(), CommunityNodeSessionPhase::Idle)
            .await;
        self.community_node_status(node, Some(consent_state), None)
            .await
    }

    pub async fn clear_community_node_token(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        delete_optional_secret(
            &self.db_path,
            self.identity_mode,
            COMMUNITY_NODE_TOKEN_PURPOSE,
            base_url.as_str(),
        )?;
        self.community_node_heartbeat_deadlines
            .lock()
            .await
            .remove(base_url.as_str());
        self.community_node_metadata_refresh_deadlines
            .lock()
            .await
            .remove(base_url.as_str());
        self.community_node_session_retry_deadlines
            .lock()
            .await
            .remove(base_url.as_str());
        self.community_node_last_errors
            .lock()
            .await
            .remove(base_url.as_str());
        self.community_node_cached_consents
            .lock()
            .await
            .remove(base_url.as_str());
        self.community_node_session_phases
            .lock()
            .await
            .insert(base_url.clone(), CommunityNodeSessionPhase::Idle);
        self.community_node_ready_refresh_pending
            .lock()
            .await
            .remove(base_url.as_str());
        let node = self
            .community_node_config
            .lock()
            .await
            .nodes
            .clone()
            .into_iter()
            .find(|node| node.base_url == base_url)
            .ok_or_else(|| anyhow!("community node `{base_url}` is not configured"))?;
        self.community_node_status(node, None, None).await
    }

    pub async fn get_community_node_consent_status(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let node = self.require_community_node(base_url.as_str()).await?;
        let mut token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?
                .ok_or_else(|| anyhow!("community node authentication is required"))?;
        let status = self
            .fetch_community_node_consent_status_with_retry(base_url.as_str(), &mut token, true)
            .await
            .context("failed to fetch community node consent status")?;
        self.set_community_node_cached_consent(base_url.as_str(), Some(status.clone()))
            .await;
        self.community_node_status(node, Some(status), None).await
    }

    pub async fn accept_community_node_consents(
        &self,
        request: AcceptCommunityNodeConsentsRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let node = self.require_community_node(base_url.as_str()).await?;
        let mut token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?
                .ok_or_else(|| anyhow!("community node authentication is required"))?;
        self.set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::Accepting,
        )
        .await;
        let status = self
            .accept_community_node_consents_with_retry(
                base_url.as_str(),
                &mut token,
                &request.policy_slugs,
            )
            .await
            .context("failed to accept community node consents")?;
        self.set_community_node_cached_consent(base_url.as_str(), Some(status.clone()))
            .await;
        if status.all_required_accepted {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Refreshing,
            )
            .await;
            self.refresh_community_node_registration_with_token_if_due(
                base_url.as_str(),
                &mut token,
                node.auto_approve,
                false,
            )
            .await?;
            self.clear_community_node_retry_state(base_url.as_str())
                .await;
            self.set_community_node_session_ready(base_url.as_str(), true)
                .await;
            let refreshed = self.require_community_node(base_url.as_str()).await?;
            return self
                .community_node_status(refreshed, Some(status), None)
                .await;
        }
        self.set_community_node_session_phase(base_url.as_str(), CommunityNodeSessionPhase::Idle)
            .await;
        self.community_node_status(node, Some(status), None).await
    }

    pub async fn refresh_community_node_metadata(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        let node = self.require_community_node(base_url.as_str()).await?;
        let mut token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?
                .ok_or_else(|| anyhow!("community node authentication is required"))?;
        self.set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::Refreshing,
        )
        .await;
        self.refresh_community_node_registration_with_token_if_due(
            base_url.as_str(),
            &mut token,
            node.auto_approve,
            true,
        )
        .await?;
        self.clear_community_node_retry_state(base_url.as_str())
            .await;
        self.set_community_node_session_ready(base_url.as_str(), false)
            .await;
        let refreshed = self.require_community_node(base_url.as_str()).await?;
        self.community_node_status(refreshed, None, None).await
    }

    pub async fn reapply_community_node_connectivity(&self) -> Result<()> {
        self.force_apply_runtime_connectivity_assist().await?;
        self.force_apply_effective_seed_peers().await?;
        Ok(())
    }

    pub async fn shutdown(&self) {
        self.app_service.shutdown().await;
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            self.iroh_stack.shutdown(),
        )
        .await;
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), self.store.close()).await;
    }
}
