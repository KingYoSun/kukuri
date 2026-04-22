use super::*;

impl DesktopRuntime {
    pub(crate) async fn set_community_node_session_phase(
        &self,
        base_url: &str,
        phase: CommunityNodeSessionPhase,
    ) {
        self.community_node_session_phases
            .lock()
            .await
            .insert(base_url.to_string(), phase);
        if phase != CommunityNodeSessionPhase::Ready
            && matches!(
                phase,
                CommunityNodeSessionPhase::Idle | CommunityNodeSessionPhase::Retrying
            )
        {
            self.community_node_ready_refresh_pending
                .lock()
                .await
                .remove(base_url);
        }
    }

    pub(crate) async fn set_community_node_session_ready(
        &self,
        base_url: &str,
        schedule_immediate_refresh: bool,
    ) {
        let previous = self
            .community_node_session_phases
            .lock()
            .await
            .insert(base_url.to_string(), CommunityNodeSessionPhase::Ready);
        if schedule_immediate_refresh {
            self.community_node_ready_refresh_pending
                .lock()
                .await
                .insert(base_url.to_string(), true);
            debug!(
                %base_url,
                previous_phase = ?previous,
                "scheduled immediate community-node metadata refresh after ready transition"
            );
        } else {
            self.community_node_ready_refresh_pending
                .lock()
                .await
                .remove(base_url);
            debug!(
                %base_url,
                previous_phase = ?previous,
                "keeping community-node metadata refresh pending state cleared for an already-ready session"
            );
        }
    }

    pub(crate) async fn community_node_session_was_ready(&self, base_url: &str) -> bool {
        self.community_node_session_phases
            .lock()
            .await
            .get(base_url)
            .copied()
            == Some(CommunityNodeSessionPhase::Ready)
    }

    pub(crate) async fn set_community_node_cached_consent(
        &self,
        base_url: &str,
        consent_state: Option<CommunityNodeConsentStatus>,
    ) {
        let mut cached = self.community_node_cached_consents.lock().await;
        if let Some(consent_state) = consent_state {
            cached.insert(base_url.to_string(), consent_state);
        } else {
            cached.remove(base_url);
        }
    }

    pub(crate) async fn clear_community_node_retry_state(&self, base_url: &str) {
        self.community_node_session_retry_deadlines
            .lock()
            .await
            .remove(base_url);
        self.community_node_last_errors
            .lock()
            .await
            .remove(base_url);
    }

    pub(crate) async fn set_community_node_retry_state(
        &self,
        base_url: &str,
        error: anyhow::Error,
    ) {
        let now = Utc::now().timestamp();
        self.community_node_last_errors
            .lock()
            .await
            .insert(base_url.to_string(), error.to_string());
        self.community_node_session_retry_deadlines
            .lock()
            .await
            .insert(
                base_url.to_string(),
                now.saturating_add(COMMUNITY_NODE_SESSION_RETRY_SECONDS),
            );
        self.set_community_node_session_phase(base_url, CommunityNodeSessionPhase::Retrying)
            .await;
    }

    pub(crate) fn community_node_token_requires_refresh(
        token: &StoredCommunityNodeToken,
        now: i64,
    ) -> bool {
        token.expires_at <= now.saturating_add(COMMUNITY_NODE_AUTH_REFRESH_SKEW_SECONDS)
    }

    pub(crate) fn map_community_node_send_error(
        action: &str,
        error: reqwest::Error,
    ) -> CommunityNodeRequestError {
        CommunityNodeRequestError::Other(anyhow!(error).context(action.to_string()))
    }

    pub(crate) fn map_community_node_status_error(
        action: &str,
        error: reqwest::Error,
    ) -> CommunityNodeRequestError {
        match error.status() {
            Some(StatusCode::UNAUTHORIZED) => CommunityNodeRequestError::AuthRequired,
            Some(StatusCode::FORBIDDEN) => CommunityNodeRequestError::ConsentRequired,
            _ => CommunityNodeRequestError::Other(anyhow!(error).context(action.to_string())),
        }
    }
}
