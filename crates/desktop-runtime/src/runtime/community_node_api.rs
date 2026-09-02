use super::*;

use chrono::Utc;
use kukuri_cn_protocol::CommunityNodePoliciesResponse;

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

    pub async fn submit_community_node_tester_feedback(
        &self,
        request: CommunityNodeTesterFeedbackSubmission,
    ) -> std::result::Result<CommunityNodeTesterFeedbackResponse, CommunityNodeTesterFeedbackError>
    {
        self.submit_tester_feedback(request).await
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
        // これが無いと新規 Node の bootstrap がスケジューラ次 tick(最大 15 秒)まで
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
        // #857: 同意前に認証(JWT 発行)を開始しない。ローカル同意が無ければ拒否し、
        // 版が上がっていれば再同意待ちとして停止する。
        let local_consent = load_community_node_local_consents(
            &self.db_path,
            self.identity_mode,
            base_url.as_str(),
        )?;
        if !local_consent.has_active_consent() {
            return Err(anyhow!(
                "community node consent is required before authentication"
            ));
        }
        let catalog = self
            .request_community_node_policies(base_url.as_str(), None)
            .await?;
        if !community_node_local_consent_satisfies_policies(&local_consent, &catalog.policies) {
            self.set_community_node_local_consent_update_pending(base_url.as_str(), true)
                .await;
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Idle,
            )
            .await;
            return self.community_node_status(node, None, None).await;
        }
        self.set_community_node_local_consent_update_pending(base_url.as_str(), false)
            .await;
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
            && community_node_local_consent_covers_status(&local_consent, &consent_state)
        {
            // ローカル同意済みの内容をサーバ記録へ同期する(#857)。
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Accepting,
            )
            .await;
            consent_state = self
                .accept_community_node_consents_with_retry(
                    base_url.as_str(),
                    &mut token,
                    &[],
                    consent_state.policy_snapshot_revision.as_deref(),
                )
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

    /// 認証不要の公開 policy カタログ取得(#857)。同意モーダルの提示内容を組み立てる
    /// ために UI 操作起点で呼ぶ。Node 同意前に許可される通信に含まれる。
    pub async fn fetch_community_node_policies(
        &self,
        request: FetchCommunityNodePoliciesRequest,
    ) -> Result<CommunityNodePoliciesResponse> {
        self.request_community_node_policies(request.base_url.as_str(), request.language.as_deref())
            .await
    }

    /// Node 同意の成立(#857)。提示済み文書をローカル記録へ保存してから、
    /// セッション確立(認証 → サーバ同期 → 登録)を即時 1 tick 実行する。
    /// ネットワーク失敗はセッション retry 状態として status に載る(同意記録は保存済み)。
    pub async fn accept_community_node_consents(
        &self,
        request: AcceptCommunityNodeConsentsRequest,
        app_version: &str,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        self.require_community_node(base_url.as_str()).await?;
        if request.documents.is_empty() {
            return Err(anyhow!("consent documents must not be empty"));
        }
        let mut state = load_community_node_local_consents(
            &self.db_path,
            self.identity_mode,
            base_url.as_str(),
        )?;
        record_community_node_local_consents(
            &mut state,
            &request.documents,
            request.language.as_str(),
            app_version,
            Utc::now().timestamp(),
        );
        persist_community_node_local_consents(
            &self.db_path,
            self.identity_mode,
            base_url.as_str(),
            &state,
        )?;
        self.set_community_node_local_consent_update_pending(base_url.as_str(), false)
            .await;
        self.clear_community_node_retry_state(base_url.as_str())
            .await;
        self.refresh_community_node_registration_if_due(base_url.as_str())
            .await?;
        let refreshed = self.require_community_node(base_url.as_str()).await?;
        self.community_node_status(refreshed, None, None).await
    }

    /// Node 同意の撤回(#857)。同意記録は履歴として残し `withdrawn_at` を立てる。
    /// トークンを破棄しセッションを停止するが、node 設定自体は残す(非 Node 機能や
    /// 直接 P2P はブロックしない)。
    pub async fn withdraw_community_node_consents(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        self.require_community_node(base_url.as_str()).await?;
        let mut state = load_community_node_local_consents(
            &self.db_path,
            self.identity_mode,
            base_url.as_str(),
        )?;
        state.withdrawn_at = Some(Utc::now().timestamp());
        persist_community_node_local_consents(
            &self.db_path,
            self.identity_mode,
            base_url.as_str(),
            &state,
        )?;
        self.set_community_node_local_consent_update_pending(base_url.as_str(), false)
            .await;
        self.clear_community_node_token(CommunityNodeTargetRequest { base_url })
            .await
    }

    pub async fn refresh_community_node_metadata(
        &self,
        request: CommunityNodeTargetRequest,
    ) -> Result<CommunityNodeNodeStatus> {
        let base_url = normalize_http_url(request.base_url.as_str())?;
        self.require_community_node(base_url.as_str()).await?;
        let mut token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?
                .ok_or_else(|| anyhow!("community node authentication is required"))?;
        self.set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::Refreshing,
        )
        .await;
        match self
            .refresh_community_node_registration_with_token_if_due(
                base_url.as_str(),
                &mut token,
                true,
            )
            .await
        {
            Ok(()) => {
                self.clear_community_node_retry_state(base_url.as_str())
                    .await;
                self.set_community_node_session_ready(base_url.as_str(), false)
                    .await;
            }
            Err(error) => {
                // 心拍 401 → 再認証 → 参加拒否(403)の経路は、定期処理と同じく AwaitingAdmission へ
                // 落として利用者の操作待ちにする。それ以外の失敗は従来どおり呼び出し元へ返す(#708)。
                let Some(rejection) = Self::community_node_admission_rejection(&error).cloned()
                else {
                    return Err(error);
                };
                self.set_community_node_admission_rejection(base_url.as_str(), rejection)
                    .await;
            }
        }
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
