//! Backend-driven OS notification dispatch.
//!
//! Issue #304: kukuri needs to keep working in the background. The P2P runtime
//! already persists notification records regardless of window state, but OS
//! toasts used to be fired only by the React frontend — which means no toast
//! appears once the window is hidden to the tray (and historically not even
//! when a section other than "notifications" was open).
//!
//! This module owns notification dispatch on the Rust side: a background task
//! polls the runtime for new notifications and shows OS toasts through the same
//! platform code the manual `show_os_notification` command uses. The frontend
//! only mirrors the user's settings down to us via `set_os_notification_settings`.

use std::{path::PathBuf, sync::Mutex, time::Duration};

use kukuri_app_api::{NotificationKind, NotificationView};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tracing::{debug, warn};

use crate::{
    commands::os_notification::show_platform_notification,
    state::{DesktopStartupState, DesktopStartupStatus, DesktopState},
};

const FALLBACK_POLL_INTERVAL: Duration = Duration::from_secs(60);

/// User-facing OS notification preferences. Mirrors the `OsNotificationSettings`
/// type the frontend persists in `localStorage` (camelCase keys on the wire).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OsNotificationSettings {
    pub enabled: bool,
    pub direct_messages: bool,
    pub mentions_and_replies: bool,
    pub follows_and_reposts: bool,
    pub quiet_mode: bool,
    pub preview_body: bool,
}

impl Default for OsNotificationSettings {
    fn default() -> Self {
        // Keep these defaults in sync with DEFAULT_OS_NOTIFICATION_SETTINGS in
        // apps/desktop/src/lib/releaseReadiness.ts.
        Self {
            enabled: false,
            direct_messages: true,
            mentions_and_replies: true,
            follows_and_reposts: false,
            quiet_mode: false,
            preview_body: false,
        }
    }
}

/// High-water mark over `received_at` plus the ids that share that timestamp,
/// so we never re-toast a notification we've already dispatched while keeping
/// memory bounded (notifications are returned newest-first and unbounded).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DispatchCursor {
    last_received_at: i64,
    ids_at_last: Vec<String>,
}

/// Shared, persisted state for the background dispatcher. Managed by Tauri so the
/// `set_os_notification_settings` command and the poll loop can both reach it.
pub struct OsNotificationBackground {
    settings: Mutex<OsNotificationSettings>,
    settings_path: PathBuf,
    cursor: Mutex<DispatchCursor>,
    cursor_path: PathBuf,
    /// `true` until the first successful poll establishes a baseline. Prevents the
    /// existing notification backlog from bursting as toasts on first launch.
    baseline_pending: Mutex<bool>,
    /// Cached local author pubkey, used to suppress toasts for our own actions.
    local_pubkey: Mutex<String>,
}

impl OsNotificationBackground {
    pub fn new(app: &AppHandle) -> Self {
        let dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        let settings_path = dir.join("os-notification-settings.json");
        let cursor_path = dir.join("os-notification-cursor.json");

        let settings = read_json(&settings_path).unwrap_or_default();
        let cursor_existed = cursor_path.exists();
        let cursor = read_json(&cursor_path).unwrap_or_default();

        Self {
            settings: Mutex::new(settings),
            settings_path,
            cursor: Mutex::new(cursor),
            cursor_path,
            baseline_pending: Mutex::new(!cursor_existed),
            local_pubkey: Mutex::new(String::new()),
        }
    }

    fn settings_snapshot(&self) -> OsNotificationSettings {
        self.settings
            .lock()
            .expect("settings lock poisoned")
            .clone()
    }

    fn replace_settings(&self, next: OsNotificationSettings) {
        if let Ok(mut guard) = self.settings.lock() {
            *guard = next.clone();
        }
        if let Err(error) = write_json(&self.settings_path, &next) {
            warn!(%error, "failed to persist OS notification settings");
        }
    }

    /// #859: アカウント切替後に旧アカウントの pubkey キャッシュと dispatch cursor を
    /// 引き継がないようリセットする。baseline を張り直すことで、新アカウントの
    /// 既存通知がトーストとして一斉に吹き出すのも防ぐ。
    pub(crate) fn reset_for_account_switch(&self) {
        if let Ok(mut pubkey) = self.local_pubkey.lock() {
            pubkey.clear();
        }
        if let Ok(mut baseline) = self.baseline_pending.lock() {
            *baseline = true;
        }
    }
}

/// Push the latest settings from the frontend down to the background dispatcher.
#[tauri::command]
pub fn set_os_notification_settings(
    state: tauri::State<'_, OsNotificationBackground>,
    settings: OsNotificationSettings,
) {
    state.replace_settings(settings);
}

/// Start the background notification dispatcher. Subscribes to runtime events
/// for instant dispatch and falls back to a 60-second poll for resilience.
pub fn spawn(app: AppHandle) {
    let mut startup_status = app.state::<DesktopStartupState>().subscribe();
    let event_app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match startup_status.borrow().clone() {
                DesktopStartupStatus::Ready => break,
                DesktopStartupStatus::Failed { .. } => return,
                DesktopStartupStatus::Initializing
                | DesktopStartupStatus::ConsentRequired { .. } => {}
            }
            if startup_status.changed().await.is_err() {
                return;
            }
        }
        // #859: アカウント切替で runtime が入れ替わると旧チャネルが閉じるため、
        // recv 失敗時は現在の runtime へ再 subscribe する(60 秒ポーリングとは別枠)。
        loop {
            let Some(state) = event_app.try_state::<DesktopState>() else {
                return;
            };
            let mut rx = state.runtime().subscribe_events();
            drop(state);
            while let Ok(event) = rx.recv().await {
                if matches!(
                    event,
                    kukuri_desktop_runtime::RuntimeEvent::NotificationStatusChanged
                ) {
                    if let Err(error) = poll_once(&event_app).await {
                        debug!(%error, "event-driven notification poll skipped");
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(FALLBACK_POLL_INTERVAL).await;
            if let Err(error) = poll_once(&app).await {
                debug!(%error, "background notification poll skipped");
            }
        }
    });
}

async fn poll_once(app: &AppHandle) -> anyhow::Result<()> {
    let Some(state) = app.try_state::<DesktopState>() else {
        // Runtime failed to initialize; nothing to dispatch.
        return Ok(());
    };
    let background = app.state::<OsNotificationBackground>();

    let notifications = state.runtime().list_notifications().await?;
    let local_pubkey = resolve_local_pubkey(&state, &background).await;
    let settings = background.settings_snapshot();
    let adult_content_enabled = state
        .runtime()
        .get_content_display_settings()
        .adult_content_enabled;

    let next_cursor = compute_cursor(&notifications);

    // First successful poll only records a baseline so the existing backlog does
    // not surface as a burst of toasts.
    if std::mem::replace(
        &mut *background
            .baseline_pending
            .lock()
            .expect("baseline lock poisoned"),
        false,
    ) {
        store_cursor(&background, next_cursor);
        return Ok(());
    }

    let previous = background
        .cursor
        .lock()
        .expect("cursor lock poisoned")
        .clone();
    let to_dispatch: Vec<&NotificationView> = notifications
        .iter()
        .filter(|notification| is_new(notification, &previous))
        .filter(|notification| should_send(notification, &settings, &local_pubkey))
        .collect();

    for notification in &to_dispatch {
        let title = notification_title(&notification.kind).to_string();
        let body = notification_body(notification, settings.preview_body, adult_content_enabled);
        if let Err(error) = show_platform_notification(
            app.clone(),
            notification.notification_id.clone(),
            title,
            body,
            settings.quiet_mode,
        ) {
            warn!(%error, "failed to show background OS notification");
        }
    }

    store_cursor(&background, next_cursor);
    Ok(())
}

async fn resolve_local_pubkey(
    state: &DesktopState,
    background: &OsNotificationBackground,
) -> String {
    {
        let cached = background
            .local_pubkey
            .lock()
            .expect("pubkey lock poisoned");
        if !cached.is_empty() {
            return cached.clone();
        }
    }
    let resolved = state
        .runtime()
        .get_sync_status()
        .await
        .map(|status| status.local_author_pubkey)
        .unwrap_or_default();
    if !resolved.is_empty() {
        *background
            .local_pubkey
            .lock()
            .expect("pubkey lock poisoned") = resolved.clone();
    }
    resolved
}

fn store_cursor(background: &OsNotificationBackground, cursor: DispatchCursor) {
    if let Ok(mut guard) = background.cursor.lock() {
        *guard = cursor.clone();
    }
    if let Err(error) = write_json(&background.cursor_path, &cursor) {
        warn!(%error, "failed to persist OS notification cursor");
    }
}

/// A notification is new when it is strictly newer than the cursor, or shares the
/// cursor's timestamp but was not already handled at that timestamp.
fn is_new(notification: &NotificationView, cursor: &DispatchCursor) -> bool {
    if notification.received_at > cursor.last_received_at {
        return true;
    }
    if notification.received_at == cursor.last_received_at {
        return !cursor
            .ids_at_last
            .iter()
            .any(|id| id == &notification.notification_id);
    }
    false
}

/// Recompute the cursor from the full (newest-first) notification list.
fn compute_cursor(notifications: &[NotificationView]) -> DispatchCursor {
    let last_received_at = notifications
        .iter()
        .map(|notification| notification.received_at)
        .max()
        .unwrap_or(0);
    let ids_at_last = notifications
        .iter()
        .filter(|notification| notification.received_at == last_received_at)
        .map(|notification| notification.notification_id.clone())
        .collect();
    DispatchCursor {
        last_received_at,
        ids_at_last,
    }
}

/// Whether an OS toast should be shown for this notification. This is the
/// single implementation: the former TS twin (`shouldSendOsNotification` in
/// releaseReadiness.ts) was production-dead and deleted in WP-Q1 (#517).
fn should_send(
    notification: &NotificationView,
    settings: &OsNotificationSettings,
    local_author_pubkey: &str,
) -> bool {
    if !settings.enabled || settings.quiet_mode || notification.read_at.is_some() {
        return false;
    }
    if notification.actor_pubkey == local_author_pubkey {
        return false;
    }
    match notification.kind {
        NotificationKind::DirectMessage => settings.direct_messages,
        NotificationKind::Mention | NotificationKind::Reply => settings.mentions_and_replies,
        NotificationKind::Followed | NotificationKind::Repost | NotificationKind::QuoteRepost => {
            settings.follows_and_reposts
        }
    }
}

/// OS toast title per notification kind (single implementation; the TS twin
/// was deleted in WP-Q1 #517).
fn notification_title(kind: &NotificationKind) -> &'static str {
    match kind {
        NotificationKind::DirectMessage => "Direct message",
        NotificationKind::Mention => "Mention",
        NotificationKind::Reply => "Reply",
        NotificationKind::Followed => "New follower",
        NotificationKind::QuoteRepost => "Quote repost",
        NotificationKind::Repost => "Repost",
    }
}

/// OS toast body per notification kind (single implementation; the TS twin
/// was deleted in WP-Q1 #517).
fn notification_body(
    notification: &NotificationView,
    preview_body: bool,
    adult_content_enabled: bool,
) -> Option<String> {
    let object_preview_is_gated = notification.object_id.is_some()
        && !adult_content_enabled
        && notification
            .content_labels
            .as_ref()
            .is_none_or(|labels| kukuri_core::has_adult_content_label(labels));
    if object_preview_is_gated {
        return None;
    }
    if !preview_body {
        return Some(
            if matches!(notification.kind, NotificationKind::DirectMessage) {
                "Open kukuri to read this message."
            } else {
                "Open kukuri to view this activity."
            }
            .to_string(),
        );
    }
    notification.preview_text.clone()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_json<T: Serialize>(path: &PathBuf, value: &T) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec(value)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification(
        id: &str,
        kind: NotificationKind,
        received_at: i64,
        read_at: Option<i64>,
        actor: &str,
        preview: Option<&str>,
    ) -> NotificationView {
        NotificationView {
            notification_id: id.to_string(),
            kind,
            actor_pubkey: actor.to_string(),
            actor_name: None,
            actor_display_name: None,
            actor_picture_asset: None,
            source_envelope_id: None,
            source_replica_id: None,
            topic_id: None,
            channel_id: None,
            object_id: None,
            thread_root_object_id: None,
            dm_id: None,
            message_id: None,
            preview_text: preview.map(|value| value.to_string()),
            content_labels: None,
            created_at: received_at,
            received_at,
            read_at,
        }
    }

    fn enabled_settings() -> OsNotificationSettings {
        OsNotificationSettings {
            enabled: true,
            direct_messages: true,
            mentions_and_replies: true,
            follows_and_reposts: true,
            quiet_mode: false,
            preview_body: false,
        }
    }

    #[test]
    fn should_send_respects_enabled_and_quiet_and_read() {
        let dm = notification(
            "a",
            NotificationKind::DirectMessage,
            10,
            None,
            "actor",
            None,
        );
        assert!(should_send(&dm, &enabled_settings(), "me"));

        let mut disabled = enabled_settings();
        disabled.enabled = false;
        assert!(!should_send(&dm, &disabled, "me"));

        let mut quiet = enabled_settings();
        quiet.quiet_mode = true;
        assert!(!should_send(&dm, &quiet, "me"));

        let read = notification(
            "a",
            NotificationKind::DirectMessage,
            10,
            Some(11),
            "actor",
            None,
        );
        assert!(!should_send(&read, &enabled_settings(), "me"));
    }

    #[test]
    fn should_send_suppresses_self_actions() {
        let mine = notification("a", NotificationKind::Mention, 10, None, "me", None);
        assert!(!should_send(&mine, &enabled_settings(), "me"));
    }

    #[test]
    fn should_send_honors_per_kind_toggles() {
        let mut settings = enabled_settings();
        settings.direct_messages = false;
        settings.mentions_and_replies = false;
        settings.follows_and_reposts = false;

        let dm = notification("a", NotificationKind::DirectMessage, 10, None, "x", None);
        let mention = notification("b", NotificationKind::Mention, 10, None, "x", None);
        let repost = notification("c", NotificationKind::Repost, 10, None, "x", None);
        assert!(!should_send(&dm, &settings, "me"));
        assert!(!should_send(&mention, &settings, "me"));
        assert!(!should_send(&repost, &settings, "me"));

        settings.mentions_and_replies = true;
        assert!(should_send(&mention, &settings, "me"));
    }

    #[test]
    fn body_uses_preview_only_when_enabled() {
        let dm = notification(
            "dm",
            NotificationKind::DirectMessage,
            1,
            None,
            "actor",
            Some("hi"),
        );
        let mention = notification(
            "mention",
            NotificationKind::Mention,
            1,
            None,
            "actor",
            Some("hi"),
        );
        assert_eq!(
            notification_body(&dm, false, false).as_deref(),
            Some("Open kukuri to read this message.")
        );
        assert_eq!(
            notification_body(&mention, false, false).as_deref(),
            Some("Open kukuri to view this activity.")
        );
        assert_eq!(
            notification_body(&mention, true, false).as_deref(),
            Some("hi")
        );
        let no_preview = notification("none", NotificationKind::Mention, 1, None, "actor", None);
        assert_eq!(notification_body(&no_preview, true, false), None);
    }

    #[test]
    fn body_suppresses_adult_and_unresolved_object_previews_while_display_is_off() {
        let mut object_notification = notification(
            "adult",
            NotificationKind::Mention,
            1,
            None,
            "actor",
            Some("adult raw preview"),
        );
        object_notification.object_id = Some("adult-object".to_string());
        object_notification.content_labels =
            Some(vec![kukuri_core::ADULT_CONTENT_LABEL.to_string()]);
        assert_eq!(notification_body(&object_notification, true, false), None);
        assert_eq!(notification_body(&object_notification, false, false), None);
        assert_eq!(
            notification_body(&object_notification, true, true).as_deref(),
            Some("adult raw preview")
        );

        object_notification.content_labels = None;
        assert_eq!(notification_body(&object_notification, true, false), None);

        object_notification.content_labels = Some(Vec::new());
        assert_eq!(
            notification_body(&object_notification, true, false).as_deref(),
            Some("adult raw preview")
        );
    }

    #[test]
    fn cursor_detects_only_new_notifications() {
        // Newest-first, like the runtime returns.
        let first = vec![
            notification("n2", NotificationKind::Mention, 20, None, "x", None),
            notification("n1", NotificationKind::Mention, 10, None, "x", None),
        ];
        let cursor = compute_cursor(&first);
        assert_eq!(cursor.last_received_at, 20);
        assert_eq!(cursor.ids_at_last, vec!["n2".to_string()]);

        // Nothing new against its own cursor.
        assert!(!is_new(&first[0], &cursor));
        assert!(!is_new(&first[1], &cursor));

        // A strictly newer item is new; a second item at the same timestamp is new.
        let n3 = notification("n3", NotificationKind::Mention, 30, None, "x", None);
        let n2b = notification("n2b", NotificationKind::Mention, 20, None, "x", None);
        assert!(is_new(&n3, &cursor));
        assert!(is_new(&n2b, &cursor));
    }
}
