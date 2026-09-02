use super::*;

impl DesktopRuntime {
    pub(crate) async fn ensure_community_node_session(&self, base_url: &str) -> Result<()> {
        let base_url = normalize_http_url(base_url)?;
        let now = Utc::now().timestamp();
        let session_gate = self
            .community_node_sessions
            .lock()
            .await
            .get(base_url.as_str())
            .map(|s| (s.session_retry_deadline, s.session_phase));
        if session_gate
            .is_some_and(|(_, phase)| phase == CommunityNodeSessionPhase::AwaitingAdmission)
        {
            return Ok(());
        }
        let retry_after = session_gate.map(|(retry_after, _)| retry_after);
        if retry_after.is_some_and(|retry_after| retry_after > now) {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Retrying,
            )
            .await;
            return Ok(());
        }

        let _guard = self.community_node_session_guard.lock().await;
        let now = Utc::now().timestamp();
        let session_gate = self
            .community_node_sessions
            .lock()
            .await
            .get(base_url.as_str())
            .map(|s| (s.session_retry_deadline, s.session_phase));
        if session_gate
            .is_some_and(|(_, phase)| phase == CommunityNodeSessionPhase::AwaitingAdmission)
        {
            return Ok(());
        }
        let retry_after = session_gate.map(|(retry_after, _)| retry_after);
        if retry_after.is_some_and(|retry_after| retry_after > now) {
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Retrying,
            )
            .await;
            return Ok(());
        }

        // #857: 未同意 node へは一切通信しない(公開 manifest / 法務文書の取得は UI 操作
        // 起点のみ)。scheduler tick もここで止まるため、同意前の登録・認証は発火しない。
        let local_consent = load_community_node_local_consents(
            &self.db_path,
            self.identity_mode,
            base_url.as_str(),
        )?;
        if !local_consent.has_active_consent() {
            self.clear_community_node_retry_state(base_url.as_str())
                .await;
            self.set_community_node_cached_consent(base_url.as_str(), None)
                .await;
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Idle,
            )
            .await;
            return Ok(());
        }

        let was_ready = self
            .community_node_session_was_ready(base_url.as_str())
            .await;
        self.set_community_node_session_phase(
            base_url.as_str(),
            CommunityNodeSessionPhase::Connecting,
        )
        .await;
        self.require_community_node(base_url.as_str()).await?;
        let mut token =
            load_community_node_token(&self.db_path, self.identity_mode, base_url.as_str())?;

        if token
            .as_ref()
            .is_none_or(|token| Self::community_node_token_requires_refresh(token, now))
        {
            // #857: 認証(JWT 発行)前に、公開カタログの現行版への同意を確認する。
            // 版が上がっていたら再同意待ちとして停止し、認証を開始しない。
            let catalog = self
                .request_community_node_policies(base_url.as_str(), None)
                .await?;
            if !community_node_local_consent_satisfies_policies(&local_consent, &catalog.policies) {
                self.set_community_node_local_consent_update_pending(base_url.as_str(), true)
                    .await;
                self.clear_community_node_retry_state(base_url.as_str())
                    .await;
                self.set_community_node_session_phase(
                    base_url.as_str(),
                    CommunityNodeSessionPhase::Idle,
                )
                .await;
                return Ok(());
            }
            self.set_community_node_local_consent_update_pending(base_url.as_str(), false)
                .await;
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Authenticating,
            )
            .await;
            token = Some(
                self.request_community_node_authentication_token(base_url.as_str())
                    .await?,
            );
        }

        let mut token = token.expect("token must exist after authentication");
        let consent_status = self
            .fetch_community_node_consent_status_with_retry(base_url.as_str(), &mut token, true)
            .await?;
        self.set_community_node_cached_consent(base_url.as_str(), Some(consent_status.clone()))
            .await;
        if !consent_status.all_required_accepted {
            if !community_node_local_consent_covers_status(&local_consent, &consent_status) {
                // ローカル同意が現行版をカバーしない = 重要変更の再同意待ち。黙って
                // 再受諾せず、UI が本文を再提示するまでセッションを進めない。
                self.set_community_node_local_consent_update_pending(base_url.as_str(), true)
                    .await;
                self.clear_community_node_retry_state(base_url.as_str())
                    .await;
                self.set_community_node_session_phase(
                    base_url.as_str(),
                    CommunityNodeSessionPhase::Idle,
                )
                .await;
                return Ok(());
            }
            // ローカル同意済みの内容をサーバ記録へ同期する(#857)。
            self.set_community_node_session_phase(
                base_url.as_str(),
                CommunityNodeSessionPhase::Accepting,
            )
            .await;
            let accepted = self
                .accept_community_node_consents_with_retry(
                    base_url.as_str(),
                    &mut token,
                    &[],
                    consent_status.policy_snapshot_revision.as_deref(),
                )
                .await?;
            self.set_community_node_cached_consent(base_url.as_str(), Some(accepted))
                .await;
        }
        self.set_community_node_local_consent_update_pending(base_url.as_str(), false)
            .await;

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
        self.set_community_node_session_ready(base_url.as_str(), !was_ready)
            .await;
        Ok(())
    }

    pub(crate) async fn refresh_community_node_registration_if_due(
        &self,
        base_url: &str,
    ) -> Result<()> {
        let base_url = normalize_http_url(base_url)?;
        match self.ensure_community_node_session(base_url.as_str()).await {
            Ok(()) => Ok(()),
            Err(error) => {
                if let Some(rejection) = Self::community_node_admission_rejection(&error).cloned() {
                    self.set_community_node_admission_rejection(base_url.as_str(), rejection)
                        .await;
                } else {
                    self.set_community_node_retry_state(base_url.as_str(), error)
                        .await;
                }
                Ok(())
            }
        }
    }

    pub(crate) async fn require_community_node(
        &self,
        base_url: &str,
    ) -> Result<CommunityNodeNodeConfig> {
        self.community_node_config
            .lock()
            .await
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .cloned()
            .ok_or_else(|| anyhow!("community node `{base_url}` is not configured"))
    }

    pub(crate) async fn community_node_status(
        &self,
        node: CommunityNodeNodeConfig,
        consent_state: Option<CommunityNodeConsentStatus>,
        last_error: Option<String>,
    ) -> Result<CommunityNodeNodeStatus> {
        let now = Utc::now().timestamp();
        let token =
            load_community_node_token(&self.db_path, self.identity_mode, node.base_url.as_str())?;
        let auth_state = match token {
            Some(token) if token.expires_at > now => CommunityNodeAuthState {
                authenticated: true,
                expires_at: Some(token.expires_at),
            },
            Some(token) => CommunityNodeAuthState {
                authenticated: false,
                expires_at: Some(token.expires_at),
            },
            None => CommunityNodeAuthState::default(),
        };
        let local_consent = load_community_node_local_consents(
            &self.db_path,
            self.identity_mode,
            node.base_url.as_str(),
        )?;
        let sessions = self.community_node_sessions.lock().await;
        let session = sessions.get(node.base_url.as_str());
        let consent_state =
            consent_state.or_else(|| session.and_then(|s| s.cached_consent.clone()));
        let last_error = last_error.or_else(|| session.and_then(|s| s.last_error.clone()));
        let admission_rejection = session.and_then(|session| session.admission_rejection.clone());
        let consent_update_pending = session
            .map(|session| session.local_consent_update_pending)
            .unwrap_or(false);
        let retry_after = session
            .map(|s| s.session_retry_deadline)
            .filter(|deadline| *deadline > now);
        let session_phase = session.map(|s| s.session_phase).unwrap_or_else(|| {
            if auth_state.authenticated
                && consent_state
                    .as_ref()
                    .is_none_or(|consent| consent.all_required_accepted)
                && node.resolved_urls.is_some()
            {
                CommunityNodeSessionPhase::Ready
            } else {
                CommunityNodeSessionPhase::Idle
            }
        });
        drop(sessions);
        let invite_code_saved = load_community_node_invite_code(
            &self.db_path,
            self.identity_mode,
            node.base_url.as_str(),
        )?
        .is_some();
        let current_connectivity_urls = relay_config_from_community_node_config(
            &self.community_node_config.lock().await.clone(),
        )
        .iroh_relay_urls;
        Ok(CommunityNodeNodeStatus {
            base_url: node.base_url,
            auth_state,
            consent_state,
            local_consent,
            consent_update_pending,
            resolved_urls: node.resolved_urls,
            last_error,
            invite_code_saved,
            admission_rejection,
            session_phase,
            retry_after,
            restart_required: current_connectivity_urls
                != *self.active_connectivity_urls.lock().await,
        })
    }

    async fn apply_runtime_connectivity_assist_with_mode(&self, force: bool) -> Result<()> {
        let discovery_config = self.discovery_config.lock().await.clone();
        let community_node_config = self.community_node_config.lock().await.clone();
        let mut next_state =
            runtime_connectivity_assist_state(&discovery_config, &community_node_config);
        let rendezvous_seed_peers = self
            .community_node_rendezvous_seed_peers
            .lock()
            .await
            .clone();
        next_state.bootstrap_seed_peers = normalize_seed_peers(
            next_state
                .bootstrap_seed_peers
                .into_iter()
                .chain(rendezvous_seed_peers)
                .collect(),
        );
        if !force {
            let current_state = self.last_runtime_connectivity_assist_state.lock().await;
            if current_state.as_ref() == Some(&next_state) {
                debug!(
                    relay_url_count = next_state.relay_urls.len(),
                    bootstrap_seed_peer_count = next_state.bootstrap_seed_peers.len(),
                    "skipping runtime connectivity apply because relay and seed inputs are unchanged"
                );
                return Ok(());
            }
        }
        let relay_config = TransportRelayConfig {
            iroh_relay_urls: next_state.relay_urls.clone(),
        };
        self.iroh_stack
            .apply_runtime_connectivity(
                &discovery_config,
                &next_state.bootstrap_seed_peers,
                relay_config.clone(),
            )
            .await?;
        debug!(
            relay_url_count = relay_config.iroh_relay_urls.len(),
            bootstrap_seed_peer_count = next_state.bootstrap_seed_peers.len(),
            "applied runtime connectivity assist from community-node metadata"
        );
        *self.active_connectivity_urls.lock().await = relay_config.iroh_relay_urls;
        *self.last_runtime_connectivity_assist_state.lock().await = Some(next_state);
        self.runtime_connectivity_apply_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub(crate) async fn apply_runtime_connectivity_assist(&self) -> Result<()> {
        self.apply_runtime_connectivity_assist_with_mode(false)
            .await
    }

    pub(crate) async fn force_rebuild_runtime_connectivity_assist(&self) -> Result<()> {
        let discovery_config = self.discovery_config.lock().await.clone();
        let community_node_config = self.community_node_config.lock().await.clone();
        let mut next_state =
            runtime_connectivity_assist_state(&discovery_config, &community_node_config);
        let rendezvous_seed_peers = self
            .community_node_rendezvous_seed_peers
            .lock()
            .await
            .clone();
        next_state.bootstrap_seed_peers = normalize_seed_peers(
            next_state
                .bootstrap_seed_peers
                .into_iter()
                .chain(rendezvous_seed_peers)
                .collect(),
        );
        let relay_config = TransportRelayConfig {
            iroh_relay_urls: next_state.relay_urls.clone(),
        };
        self.iroh_stack
            .force_rebuild_runtime_connectivity(
                &discovery_config,
                &next_state.bootstrap_seed_peers,
                relay_config.clone(),
            )
            .await?;
        debug!(
            relay_url_count = relay_config.iroh_relay_urls.len(),
            bootstrap_seed_peer_count = next_state.bootstrap_seed_peers.len(),
            "force rebuilt runtime connectivity assist from community-node metadata"
        );
        *self.active_connectivity_urls.lock().await = relay_config.iroh_relay_urls;
        *self.last_runtime_connectivity_assist_state.lock().await = Some(next_state);
        self.runtime_connectivity_apply_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    async fn apply_effective_seed_peers_with_mode(&self, force: bool) -> Result<()> {
        let discovery_config = self.discovery_config.lock().await.clone();
        let community_node_config = self.community_node_config.lock().await.clone();
        let mut next_state =
            effective_seed_peer_apply_state(&discovery_config, &community_node_config);
        let rendezvous_seed_peers = self
            .community_node_rendezvous_seed_peers
            .lock()
            .await
            .clone();
        next_state.bootstrap_seed_peers = normalize_seed_peers(
            next_state
                .bootstrap_seed_peers
                .into_iter()
                .chain(rendezvous_seed_peers)
                .collect(),
        );
        if !force {
            let current_state = self.last_effective_seed_peer_apply_state.lock().await;
            if current_state.as_ref() == Some(&next_state) {
                debug!(
                    bootstrap_seed_peer_count = next_state.bootstrap_seed_peers.len(),
                    configured_seed_peer_count = next_state.configured_seed_peers.len(),
                    "skipping discovery seed apply because the effective seed inputs are unchanged"
                );
                return Ok(());
            }
        }
        self.app_service
            .set_discovery_seeds(
                next_state.discovery_mode.clone(),
                next_state.discovery_env_locked,
                next_state.configured_seed_peers.clone(),
                next_state.bootstrap_seed_peers.clone(),
            )
            .await?;
        debug!(
            bootstrap_seed_peer_count = next_state.bootstrap_seed_peers.len(),
            configured_seed_peer_count = next_state.configured_seed_peers.len(),
            "applied effective discovery seeds from community-node metadata"
        );
        *self.last_effective_seed_peer_apply_state.lock().await = Some(next_state);
        self.effective_seed_peer_apply_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub(crate) async fn apply_effective_seed_peers(&self) -> Result<()> {
        self.apply_effective_seed_peers_with_mode(false).await
    }

    pub(crate) async fn force_apply_effective_seed_peers(&self) -> Result<()> {
        self.apply_effective_seed_peers_with_mode(true).await
    }
}
