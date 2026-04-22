use super::*;

impl DesktopRuntime {
    pub(crate) async fn request_community_node_authentication_token(
        &self,
        base_url: &str,
    ) -> Result<StoredCommunityNodeToken> {
        let base_url = normalize_http_url(base_url)?;
        let client = community_node_http_client()?;
        let challenge_url = format!("{}/v1/auth/challenge", base_url);
        let pubkey = self.author_keys.public_key_hex();
        let seed_peer = self.local_community_node_seed_peer("auth").await?;
        let challenge = client
            .post(challenge_url)
            .json(&serde_json::json!({ "pubkey": pubkey }))
            .send()
            .await
            .context("failed to request auth challenge")?
            .error_for_status()
            .context("auth challenge request failed")?
            .json::<AuthChallengeResponse>()
            .await
            .context("failed to decode auth challenge response")?;

        let public_base_url = self
            .community_node_config
            .lock()
            .await
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .and_then(|node| {
                node.resolved_urls
                    .as_ref()
                    .map(|resolved| resolved.public_base_url.clone())
            })
            .unwrap_or_else(|| base_url.clone());
        let auth_envelope_json = build_auth_envelope_json(
            self.author_keys.as_ref(),
            challenge.challenge.as_str(),
            public_base_url.as_str(),
        )?;
        let verify_url = format!("{}/v1/auth/verify", base_url);
        let verify = client
            .post(verify_url)
            .json(&serde_json::json!({
                "auth_envelope_json": auth_envelope_json,
                "endpoint_id": seed_peer.endpoint_id,
                "addr_hint": seed_peer.addr_hint,
            }))
            .send()
            .await
            .context("failed to verify auth envelope")?
            .error_for_status()
            .context("auth verify request failed")?
            .json::<AuthVerifyResponse>()
            .await
            .context("failed to decode auth verify response")?;
        let token = StoredCommunityNodeToken {
            access_token: verify.access_token,
            expires_at: verify.expires_at,
        };
        persist_community_node_token(&self.db_path, self.identity_mode, base_url.as_str(), &token)?;
        Ok(token)
    }

    async fn request_community_node_consent_status(
        &self,
        base_url: &str,
        access_token: &str,
    ) -> std::result::Result<CommunityNodeConsentStatus, CommunityNodeRequestError> {
        let client = community_node_http_client().map_err(CommunityNodeRequestError::Other)?;
        let response = client
            .get(format!("{}/v1/consents/status", base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to fetch community node consent status",
                    error,
                )
            })?;
        let response = response.error_for_status().map_err(|error| {
            Self::map_community_node_status_error(
                "community node consent status request failed",
                error,
            )
        })?;
        response
            .json::<CommunityNodeConsentStatus>()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to decode community node consent status",
                    error,
                )
            })
    }

    async fn request_accept_community_node_consents(
        &self,
        base_url: &str,
        access_token: &str,
        policy_slugs: &[String],
    ) -> std::result::Result<CommunityNodeConsentStatus, CommunityNodeRequestError> {
        let client = community_node_http_client().map_err(CommunityNodeRequestError::Other)?;
        let response = client
            .post(format!("{}/v1/consents", base_url))
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "policy_slugs": policy_slugs }))
            .send()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to accept community node consents",
                    error,
                )
            })?;
        let response = response.error_for_status().map_err(|error| {
            Self::map_community_node_status_error(
                "community node consent accept request failed",
                error,
            )
        })?;
        response
            .json::<CommunityNodeConsentStatus>()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to decode accepted community node consents",
                    error,
                )
            })
    }

    async fn sync_community_node_bootstrap_metadata(
        &self,
        base_url: &str,
        access_token: &str,
    ) -> std::result::Result<CommunityNodeNodeConfig, CommunityNodeRequestError> {
        let base_url = normalize_http_url(base_url).map_err(CommunityNodeRequestError::Other)?;
        let local_seed_peer_before = self
            .local_community_node_seed_peer("metadata-refresh-baseline")
            .await
            .ok();
        let config = self.community_node_config.lock().await.clone();
        let Some(index) = config
            .nodes
            .iter()
            .position(|node| node.base_url == base_url)
        else {
            return Err(CommunityNodeRequestError::Other(anyhow!(
                "community node `{base_url}` is not configured"
            )));
        };
        let client = community_node_http_client().map_err(CommunityNodeRequestError::Other)?;
        let response = client
            .get(format!("{}/v1/bootstrap/nodes", base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to refresh community node metadata",
                    error,
                )
            })?;
        let bootstrap = response
            .error_for_status()
            .map_err(|error| {
                Self::map_community_node_status_error(
                    "community node bootstrap request failed",
                    error,
                )
            })?
            .json::<BootstrapNodesResponse>()
            .await
            .map_err(|error| {
                Self::map_community_node_send_error(
                    "failed to decode community node bootstrap response",
                    error,
                )
            })?;
        let resolved_urls = bootstrap
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .map(|node| node.resolved_urls.clone())
            .ok_or_else(|| {
                CommunityNodeRequestError::Other(anyhow!(
                    "community node bootstrap response is missing self metadata"
                ))
            })?;
        debug!(
            %base_url,
            relay_url_count = resolved_urls.connectivity_urls.len(),
            seed_peer_count = resolved_urls.seed_peers.len(),
            "community-node metadata sync resolved bootstrap metadata"
        );
        let mut next_config = config;
        next_config.nodes[index].resolved_urls = Some(resolved_urls);
        let normalized = normalize_community_node_config(next_config)
            .map_err(CommunityNodeRequestError::Other)?;
        save_community_node_config(&self.db_path, &normalized)
            .map_err(CommunityNodeRequestError::Other)?;
        *self.community_node_config.lock().await = normalized.clone();
        self.apply_runtime_connectivity_assist()
            .await
            .map_err(CommunityNodeRequestError::Other)?;
        self.apply_effective_seed_peers()
            .await
            .map_err(CommunityNodeRequestError::Other)?;
        let local_seed_peer_after = self
            .local_community_node_seed_peer("metadata-refresh-post-apply")
            .await
            .ok();
        if local_seed_peer_before != local_seed_peer_after {
            self.community_node_heartbeat_deadlines
                .lock()
                .await
                .remove(base_url.as_str());
            debug!(
                %base_url,
                before = ?local_seed_peer_before,
                after = ?local_seed_peer_after,
                "scheduled immediate community-node heartbeat after local seed peer changed during metadata sync"
            );
        }
        normalized
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .cloned()
            .ok_or_else(|| {
                CommunityNodeRequestError::Other(anyhow!(
                    "community node `{base_url}` disappeared after normalization"
                ))
            })
    }

    pub(crate) async fn community_node_bootstrap_metadata_retry_due(
        &self,
        base_url: &str,
        now: i64,
    ) -> bool {
        let seed_peers_empty = self
            .community_node_config
            .lock()
            .await
            .nodes
            .iter()
            .find(|node| node.base_url == base_url)
            .and_then(|node| node.resolved_urls.as_ref())
            .is_none_or(|resolved_urls| resolved_urls.seed_peers.is_empty());
        if !seed_peers_empty {
            self.community_node_metadata_refresh_deadlines
                .lock()
                .await
                .remove(base_url);
            return false;
        }
        let next_due_at = self
            .community_node_metadata_refresh_deadlines
            .lock()
            .await
            .get(base_url)
            .copied()
            .unwrap_or_default();
        next_due_at <= now
    }

    pub(crate) async fn record_community_node_bootstrap_metadata_refresh(
        &self,
        base_url: &str,
        seed_peers_empty: bool,
        now: i64,
    ) {
        let mut deadlines = self.community_node_metadata_refresh_deadlines.lock().await;
        if seed_peers_empty {
            deadlines.insert(
                base_url.to_string(),
                now.saturating_add(COMMUNITY_NODE_BOOTSTRAP_METADATA_RETRY_SECONDS),
            );
        } else {
            deadlines.remove(base_url);
        }
    }

    pub(crate) async fn local_community_node_seed_peer(
        &self,
        operation: &str,
    ) -> Result<CommunityNodeSeedPeer> {
        let endpoint_id = self
            .iroh_stack
            .transport
            .discovery()
            .await
            .with_context(|| {
                format!("failed to read local endpoint id for community node {operation}")
            })?
            .local_endpoint_id;
        let addr_hint = self
            .local_peer_ticket()
            .await
            .with_context(|| {
                format!("failed to read local peer ticket for community node {operation}")
            })?
            .and_then(|ticket| {
                ticket
                    .split_once('@')
                    .map(|(_, addr)| addr.trim().to_string())
                    .filter(|addr| !addr.is_empty())
            });
        CommunityNodeSeedPeer::new(endpoint_id, addr_hint)
    }

    pub(crate) async fn fetch_community_node_consent_status_with_retry(
        &self,
        base_url: &str,
        token: &mut StoredCommunityNodeToken,
        allow_reauthenticate: bool,
    ) -> Result<CommunityNodeConsentStatus> {
        match self
            .request_community_node_consent_status(base_url, token.access_token.as_str())
            .await
        {
            Ok(status) => Ok(status),
            Err(CommunityNodeRequestError::AuthRequired) if allow_reauthenticate => {
                self.set_community_node_session_phase(
                    base_url,
                    CommunityNodeSessionPhase::Authenticating,
                )
                .await;
                *token = self
                    .request_community_node_authentication_token(base_url)
                    .await?;
                self.request_community_node_consent_status(base_url, token.access_token.as_str())
                    .await
                    .map_err(CommunityNodeRequestError::into_anyhow)
            }
            Err(error) => Err(error.into_anyhow()),
        }
    }

    pub(crate) async fn accept_community_node_consents_with_retry(
        &self,
        base_url: &str,
        token: &mut StoredCommunityNodeToken,
        policy_slugs: &[String],
    ) -> Result<CommunityNodeConsentStatus> {
        match self
            .request_accept_community_node_consents(
                base_url,
                token.access_token.as_str(),
                policy_slugs,
            )
            .await
        {
            Ok(status) => Ok(status),
            Err(CommunityNodeRequestError::AuthRequired) => {
                self.set_community_node_session_phase(
                    base_url,
                    CommunityNodeSessionPhase::Authenticating,
                )
                .await;
                *token = self
                    .request_community_node_authentication_token(base_url)
                    .await?;
                self.request_accept_community_node_consents(
                    base_url,
                    token.access_token.as_str(),
                    policy_slugs,
                )
                .await
                .map_err(CommunityNodeRequestError::into_anyhow)
            }
            Err(error) => Err(error.into_anyhow()),
        }
    }

    async fn refresh_community_node_registration_with_token_if_due_once(
        &self,
        base_url: &str,
        access_token: &str,
        force_heartbeat: bool,
    ) -> std::result::Result<(), CommunityNodeRequestError> {
        let base_url = normalize_http_url(base_url).map_err(CommunityNodeRequestError::Other)?;
        let now = Utc::now().timestamp();
        let next_due_at = self
            .community_node_heartbeat_deadlines
            .lock()
            .await
            .get(base_url.as_str())
            .copied()
            .unwrap_or_default();
        if !force_heartbeat && next_due_at > now {
            let ready_refresh_pending = self
                .community_node_ready_refresh_pending
                .lock()
                .await
                .remove(base_url.as_str())
                .unwrap_or(false);
            if !self
                .community_node_bootstrap_metadata_retry_due(base_url.as_str(), now)
                .await
                && !ready_refresh_pending
            {
                debug!(
                    %base_url,
                    next_due_at,
                    now,
                    "skipping community-node heartbeat because the next refresh is not due"
                );
                return Ok(());
            }
            info!(
                %base_url,
                next_due_at,
                now,
                ready_refresh_pending,
                "running community-node metadata refresh without waiting for the next heartbeat"
            );
            return match self
                .sync_community_node_bootstrap_metadata(base_url.as_str(), access_token)
                .await
            {
                Ok(node) => {
                    self.record_community_node_bootstrap_metadata_refresh(
                        base_url.as_str(),
                        node.resolved_urls
                            .as_ref()
                            .is_none_or(|resolved_urls| resolved_urls.seed_peers.is_empty()),
                        now,
                    )
                    .await;
                    Ok(())
                }
                Err(error) => {
                    self.record_community_node_bootstrap_metadata_refresh(
                        base_url.as_str(),
                        true,
                        now,
                    )
                    .await;
                    Err(error)
                }
            };
        }
        if force_heartbeat && next_due_at > now {
            info!(
                %base_url,
                next_due_at,
                now,
                "forcing community-node heartbeat before bootstrap metadata refresh"
            );
        }
        let seed_peer = self
            .local_community_node_seed_peer("heartbeat")
            .await
            .map_err(CommunityNodeRequestError::Other)?;
        info!(
            %base_url,
            next_due_at,
            now,
            "refreshing community-node bootstrap heartbeat"
        );
        let client = community_node_http_client().map_err(CommunityNodeRequestError::Other)?;
        let response = client
            .post(format!("{}/v1/bootstrap/heartbeat", base_url))
            .bearer_auth(access_token)
            .json(&serde_json::json!({
                "endpoint_id": seed_peer.endpoint_id,
                "addr_hint": seed_peer.addr_hint,
            }))
            .send()
            .await;
        match response {
            Ok(response) => {
                let heartbeat = response
                    .error_for_status()
                    .map_err(|error| {
                        Self::map_community_node_status_error(
                            "community node bootstrap heartbeat request failed",
                            error,
                        )
                    })?
                    .json::<BootstrapHeartbeatResponse>()
                    .await
                    .map_err(|error| {
                        Self::map_community_node_send_error(
                            "failed to decode community node bootstrap heartbeat response",
                            error,
                        )
                    })?;
                self.community_node_heartbeat_deadlines.lock().await.insert(
                    base_url.clone(),
                    heartbeat
                        .expires_at
                        .saturating_sub(COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_INTERVAL_SECONDS),
                );
                debug!(
                    %base_url,
                    expires_at = heartbeat.expires_at,
                    "community-node bootstrap heartbeat refreshed"
                );
                match self
                    .sync_community_node_bootstrap_metadata(base_url.as_str(), access_token)
                    .await
                {
                    Ok(node) => {
                        self.record_community_node_bootstrap_metadata_refresh(
                            base_url.as_str(),
                            node.resolved_urls
                                .as_ref()
                                .is_none_or(|resolved_urls| resolved_urls.seed_peers.is_empty()),
                            now,
                        )
                        .await;
                        Ok(())
                    }
                    Err(error) => {
                        self.record_community_node_bootstrap_metadata_refresh(
                            base_url.as_str(),
                            true,
                            now,
                        )
                        .await;
                        Err(error)
                    }
                }
            }
            Err(error) => {
                self.community_node_heartbeat_deadlines.lock().await.insert(
                    base_url,
                    now.saturating_add(COMMUNITY_NODE_BOOTSTRAP_HEARTBEAT_RETRY_SECONDS),
                );
                Err(Self::map_community_node_send_error(
                    "failed to refresh community node bootstrap registration",
                    error,
                ))
            }
        }
    }

    pub(crate) async fn refresh_community_node_registration_with_token_if_due(
        &self,
        base_url: &str,
        token: &mut StoredCommunityNodeToken,
        auto_approve: bool,
        force_heartbeat: bool,
    ) -> Result<()> {
        match self
            .refresh_community_node_registration_with_token_if_due_once(
                base_url,
                token.access_token.as_str(),
                force_heartbeat,
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(CommunityNodeRequestError::AuthRequired) => {
                self.set_community_node_session_phase(
                    base_url,
                    CommunityNodeSessionPhase::Authenticating,
                )
                .await;
                *token = self
                    .request_community_node_authentication_token(base_url)
                    .await?;
                let consent_status = self
                    .fetch_community_node_consent_status_with_retry(base_url, token, false)
                    .await?;
                self.set_community_node_cached_consent(base_url, Some(consent_status.clone()))
                    .await;
                if !consent_status.all_required_accepted {
                    if !auto_approve {
                        self.set_community_node_session_phase(
                            base_url,
                            CommunityNodeSessionPhase::Idle,
                        )
                        .await;
                        return Ok(());
                    }
                    self.set_community_node_session_phase(
                        base_url,
                        CommunityNodeSessionPhase::Accepting,
                    )
                    .await;
                    let accepted = self
                        .accept_community_node_consents_with_retry(base_url, token, &[])
                        .await?;
                    self.set_community_node_cached_consent(base_url, Some(accepted))
                        .await;
                }
                self.refresh_community_node_registration_with_token_if_due_once(
                    base_url,
                    token.access_token.as_str(),
                    force_heartbeat,
                )
                .await
                .map_err(CommunityNodeRequestError::into_anyhow)
            }
            Err(CommunityNodeRequestError::ConsentRequired) if auto_approve => {
                self.set_community_node_session_phase(
                    base_url,
                    CommunityNodeSessionPhase::Accepting,
                )
                .await;
                let accepted = self
                    .accept_community_node_consents_with_retry(base_url, token, &[])
                    .await?;
                self.set_community_node_cached_consent(base_url, Some(accepted))
                    .await;
                self.refresh_community_node_registration_with_token_if_due_once(
                    base_url,
                    token.access_token.as_str(),
                    force_heartbeat,
                )
                .await
                .map_err(CommunityNodeRequestError::into_anyhow)
            }
            Err(CommunityNodeRequestError::ConsentRequired) => Ok(()),
            Err(error) => Err(error.into_anyhow()),
        }
    }
}
