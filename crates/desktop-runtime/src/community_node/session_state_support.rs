use super::*;

impl DesktopRuntime {
    pub(crate) async fn set_community_node_session_phase(
        &self,
        base_url: &str,
        phase: CommunityNodeSessionPhase,
    ) {
        let mut sessions = self.community_node_sessions.lock().await;
        let entry = sessions
            .entry(base_url.to_string())
            .or_insert_with(CommunityNodeSessionState::default);
        entry.session_phase = phase;
        if phase != CommunityNodeSessionPhase::Ready
            && matches!(
                phase,
                CommunityNodeSessionPhase::Idle | CommunityNodeSessionPhase::Retrying
            )
        {
            entry.ready_refresh_pending = false;
        }
    }

    pub(crate) async fn set_community_node_session_ready(
        &self,
        base_url: &str,
        schedule_immediate_refresh: bool,
    ) {
        let mut sessions = self.community_node_sessions.lock().await;
        let entry = sessions
            .entry(base_url.to_string())
            .or_insert_with(CommunityNodeSessionState::default);
        let previous = Some(entry.session_phase);
        entry.session_phase = CommunityNodeSessionPhase::Ready;
        if schedule_immediate_refresh {
            entry.ready_refresh_pending = true;
            debug!(
                %base_url,
                previous_phase = ?previous,
                "scheduled immediate community-node metadata refresh after ready transition"
            );
        } else {
            entry.ready_refresh_pending = false;
            debug!(
                %base_url,
                previous_phase = ?previous,
                "keeping community-node metadata refresh pending state cleared for an already-ready session"
            );
        }
    }

    pub(crate) async fn community_node_session_was_ready(&self, base_url: &str) -> bool {
        self.community_node_sessions
            .lock()
            .await
            .get(base_url)
            .map(|s| s.session_phase)
            == Some(CommunityNodeSessionPhase::Ready)
    }

    pub(crate) async fn set_community_node_cached_consent(
        &self,
        base_url: &str,
        consent_state: Option<CommunityNodeConsentStatus>,
    ) {
        let mut sessions = self.community_node_sessions.lock().await;
        let entry = sessions
            .entry(base_url.to_string())
            .or_insert_with(CommunityNodeSessionState::default);
        entry.cached_consent = consent_state;
    }

    pub(crate) async fn clear_community_node_retry_state(&self, base_url: &str) {
        let mut sessions = self.community_node_sessions.lock().await;
        if let Some(entry) = sessions.get_mut(base_url) {
            entry.session_retry_deadline = 0;
            entry.last_error = None;
        }
    }

    pub(crate) async fn set_community_node_retry_state(
        &self,
        base_url: &str,
        error: anyhow::Error,
    ) {
        let now = Utc::now().timestamp();
        {
            let mut sessions = self.community_node_sessions.lock().await;
            let entry = sessions
                .entry(base_url.to_string())
                .or_insert_with(CommunityNodeSessionState::default);
            entry.last_error = Some(error.to_string());
            entry.session_retry_deadline = now.saturating_add(COMMUNITY_NODE_SESSION_RETRY_SECONDS);
        }
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
