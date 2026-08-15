use super::*;

impl DesktopRuntime {
    pub async fn read_community_node_trust_user(
        &self,
        request: CommunityNodeUserAdvisoryRequest,
    ) -> std::result::Result<TrustUserReadResponse, CommunityNodeTrustRelationError> {
        self.request_community_node_trust_user(request).await
    }

    pub async fn read_community_node_relation_user(
        &self,
        request: CommunityNodeUserAdvisoryRequest,
    ) -> std::result::Result<RelationReadResponse, CommunityNodeTrustRelationError> {
        self.request_community_node_relation_user(request).await
    }

    pub async fn list_community_node_relation_neighbors(
        &self,
        request: CommunityNodeRelationNeighborsRequest,
    ) -> std::result::Result<RelationNeighborsResponse, CommunityNodeTrustRelationError> {
        self.request_community_node_relation_neighbors(request)
            .await
    }

    pub async fn get_community_node_relation_optout(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> std::result::Result<RelationOptoutResponse, CommunityNodeTrustRelationError> {
        self.request_community_node_relation_optout(request.base_url.as_str(), reqwest::Method::GET)
            .await
    }

    pub async fn set_community_node_relation_optout(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> std::result::Result<RelationOptoutResponse, CommunityNodeTrustRelationError> {
        self.request_community_node_relation_optout(request.base_url.as_str(), reqwest::Method::PUT)
            .await
    }

    pub async fn clear_community_node_relation_optout(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> std::result::Result<RelationOptoutResponse, CommunityNodeTrustRelationError> {
        self.request_community_node_relation_optout(
            request.base_url.as_str(),
            reqwest::Method::DELETE,
        )
        .await
    }

    pub async fn submit_community_node_indexing_request(
        &self,
        request: CommunityNodeIndexingRequest,
    ) -> std::result::Result<SubmitIndexingRequestResponse, CommunityNodeIndexingRequestError> {
        self.request_community_node_indexing(request).await
    }

    pub async fn search_community_node_index(
        &self,
        request: CommunityNodeIndexQueryRequest,
    ) -> std::result::Result<IndexQueryResponse, CommunityNodeIndexQueryError> {
        self.query_community_node_index(IndexOperation::Search, request)
            .await
    }

    pub async fn discover_community_node_index(
        &self,
        request: CommunityNodeIndexQueryRequest,
    ) -> std::result::Result<IndexQueryResponse, CommunityNodeIndexQueryError> {
        self.query_community_node_index(IndexOperation::Discovery, request)
            .await
    }

    pub async fn recommend_community_node_index(
        &self,
        request: CommunityNodeIndexQueryRequest,
    ) -> std::result::Result<IndexQueryResponse, CommunityNodeIndexQueryError> {
        self.query_community_node_index(IndexOperation::Recommendations, request)
            .await
    }

    pub async fn get_community_node_config(&self) -> Result<CommunityNodeConfig> {
        Ok(self.community_node_config.lock().await.clone())
    }

    /// 読み取り専用。CN セッションの establish/refresh・self-heal は
    /// セッション維持スケジューラ(`run_community_node_session_maintenance_once`)が
    /// 担い、getter は副作用(registration refresh のネットワーク I/O)を持たない(WP-Q2)。
    /// config は tick 側が resolved_urls を書き戻すため、ここで読んだ node が最新。
    pub async fn get_community_node_statuses(&self) -> Result<Vec<CommunityNodeNodeStatus>> {
        let config = self.community_node_config.lock().await.clone();
        let mut statuses = Vec::with_capacity(config.nodes.len());
        for node in config.nodes {
            statuses.push(self.community_node_status(node, None, None).await?);
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
        for removed_node in current_config.nodes.iter().filter(|current| {
            next_config
                .nodes
                .iter()
                .all(|next| next.base_url != current.base_url)
        }) {
            delete_community_node_invite_code(
                &self.db_path,
                self.identity_mode,
                removed_node.base_url.as_str(),
            )?;
        }
        save_community_node_config(&self.db_path, &next_config)?;
        *self.community_node_config.lock().await = next_config.clone();
        self.community_node_sessions.lock().await.clear();
        *self.community_node_reconnect_state.lock().await = Default::default();
        self.apply_runtime_connectivity_assist().await?;
        self.apply_effective_seed_peers().await?;
        // getter を読取専用化した(WP-Q2)ため、config 変更直後の登録はここで 1 tick 即時実行する。
        // これが無いと新規 auto_approve ノードの bootstrap がスケジューラ次 tick(最大 15 秒)まで
        // 遅延する。tick は deadline ゲート済みの冪等設計で、直後の scheduler tick と二重でも安全。
        self.run_community_node_session_maintenance_once().await;
        Ok(next_config)
    }

    pub async fn clear_community_node_config(&self) -> Result<()> {
        let existing = self.community_node_config.lock().await.clone();
        for node in existing.nodes {
            delete_community_node_invite_code(
                &self.db_path,
                self.identity_mode,
                node.base_url.as_str(),
            )?;
            self.clear_community_node_token(CommunityNodeTargetRequest {
                base_url: node.base_url,
            })
            .await?;
        }
        save_community_node_config(&self.db_path, &CommunityNodeConfig::default())?;
        *self.community_node_config.lock().await = CommunityNodeConfig::default();
        self.community_node_sessions.lock().await.clear();
        *self.community_node_reconnect_state.lock().await = Default::default();
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
        let mut token = match self
            .request_community_node_authentication_token(base_url.as_str())
            .await
        {
            Ok(token) => token,
            Err(error) => {
                if let Some(rejection) = Self::community_node_admission_rejection(&error).cloned() {
                    self.set_community_node_admission_rejection(base_url.as_str(), rejection)
                        .await;
                    return self.community_node_status(node, None, None).await;
                }
                return Err(error);
            }
        };
        let mut consent_state = self
            .fetch_community_node_consent_status_with_retry(base_url.as_str(), &mut token, false)
            .await?;
        self.set_community_node_cached_consent(base_url.as_str(), Some(consent_state.clone()))
            .await;
        if !consent_state.all_required_accepted
            && node.auto_approve
            && !community_node_consent_has_pending_update(&consent_state)
        {
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

    pub async fn set_community_node_invite_code(
        &self,
        request: SetCommunityNodeInviteCodeRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        self.require_community_node(base_url.as_str()).await?;
        match request
            .invite_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(invite_code) => persist_community_node_invite_code(
                &self.db_path,
                self.identity_mode,
                base_url.as_str(),
                invite_code,
            )?,
            None => delete_community_node_invite_code(
                &self.db_path,
                self.identity_mode,
                base_url.as_str(),
            )?,
        }
        self.clear_community_node_retry_state(base_url.as_str())
            .await;
        self.set_community_node_session_phase(base_url.as_str(), CommunityNodeSessionPhase::Idle)
            .await;
        self.authenticate_community_node(CommunityNodeTargetRequest { base_url })
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
        self.community_node_sessions.lock().await.insert(
            base_url.clone(),
            CommunityNodeSessionState {
                session_phase: CommunityNodeSessionPhase::Idle,
                ..Default::default()
            },
        );
        *self.community_node_reconnect_state.lock().await = Default::default();
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

    /// public manifest endpoint (#356) から node manifest を取得する。
    /// dependency 表示（#357）のため client が unauthenticated に呼ぶ。
    pub async fn fetch_community_node_manifest(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeManifestFetch> {
        self.request_community_node_manifest(request.base_url.as_str())
            .await
    }

    /// 解決済みの通報先 node へ通報を送信する（#310 の分散通報ルーティング）。
    /// 通報先は client が provenance + manifest から解決し、その report endpoint を渡す。
    pub async fn submit_community_node_report(
        &self,
        request: SubmitCommunityNodeReportRequest,
    ) -> Result<SubmitCommunityNodeReportResult, CommunityNodeReportError> {
        self.request_community_node_report_submit(&request).await
    }

    pub async fn reapply_community_node_connectivity(&self) -> Result<()> {
        self.force_rebuild_runtime_connectivity_assist().await?;
        self.force_apply_effective_seed_peers().await?;
        Ok(())
    }

    pub async fn shutdown(&self) {
        if let Some(handle) = self.sync_status_observer_task.lock().await.take() {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        if let Some(handle) = self.community_node_scheduler_task.lock().await.take() {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }
        self.app_service.shutdown().await;
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            self.iroh_stack.shutdown(),
        )
        .await;
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), self.store.close()).await;
    }
}
