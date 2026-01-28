use std::sync::Arc;

use tauri::{Emitter, Manager};
use tokio::sync::broadcast;
use tracing::info;

// モジュール定義
mod application;
pub mod domain;
mod infrastructure;
mod presentation;
mod shared;
mod state;

pub use application::ports::repositories::TopicMetricsRepository;
pub use domain::entities::{TopicMetricsRecord, TopicMetricsSnapshot};
pub use infrastructure::database::{
    connection_pool::ConnectionPool, sqlite_repository::SqliteRepository,
};
pub use shared::config::AppConfig;

pub mod ops {
    pub mod p2p {
        pub use crate::infrastructure::p2p::metrics;
    }
}

#[doc(hidden)]
pub mod test_support {
    pub mod application {
        pub use crate::application::ports;
        pub use crate::application::services;
        pub use crate::application::shared;
    }
    pub mod domain {
        pub use crate::domain::entities;
        pub use crate::domain::p2p;
        pub use crate::domain::value_objects;
    }
    pub mod infrastructure {
        pub use crate::infrastructure::crypto;
        pub use crate::infrastructure::database;
        pub use crate::infrastructure::event;
        pub use crate::infrastructure::offline;
        pub use crate::infrastructure::p2p;
        pub use crate::infrastructure::storage;
    }
    pub mod presentation {
        pub use crate::presentation::dto;
    }
    pub mod shared {
        pub use crate::shared::{config, error};
    }
}

#[doc(hidden)]
pub mod contract_testing;

// Tauriコマンドのインポート
// v2アーキテクチャへの移行完了につき、旧コマンドのインポートは削除
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Run the Tauri application
///
/// # Panics
///
/// Panics if the Tauri application fails to run
pub fn run() {
    // ログ設定の初期化
    init_logging();

    info!("Kukuri Tauri application starting...");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            // 認証関連コマンド
            presentation::commands::generate_keypair,
            presentation::commands::login,
            presentation::commands::logout,
            presentation::commands::export_private_key,
            // セキュアストレージ関連コマンド
            presentation::commands::add_account,
            presentation::commands::list_accounts,
            presentation::commands::switch_account,
            presentation::commands::remove_account,
            presentation::commands::get_current_account,
            presentation::commands::secure_login,
            // ユーザー関連コマンド
            presentation::commands::get_user,
            presentation::commands::get_user_by_pubkey,
            presentation::commands::search_users,
            presentation::commands::update_privacy_settings,
            presentation::commands::follow_user,
            presentation::commands::unfollow_user,
            presentation::commands::get_followers,
            presentation::commands::get_following,
            presentation::commands::send_direct_message,
            presentation::commands::list_direct_messages,
            presentation::commands::list_direct_message_conversations,
            presentation::commands::mark_direct_message_conversation_read,
            presentation::commands::seed_direct_message_for_e2e,
            presentation::commands::upload_profile_avatar,
            presentation::commands::fetch_profile_avatar,
            presentation::commands::profile_avatar_sync,
            // トピック関連コマンド
            presentation::commands::create_topic,
            presentation::commands::enqueue_topic_creation,
            presentation::commands::list_pending_topics,
            presentation::commands::mark_pending_topic_synced,
            presentation::commands::mark_pending_topic_failed,
            presentation::commands::get_topics,
            presentation::commands::update_topic,
            presentation::commands::delete_topic,
            presentation::commands::join_topic,
            presentation::commands::leave_topic,
            presentation::commands::get_topic_stats,
            presentation::commands::list_trending_topics,
            // ポスト関連コマンド
            presentation::commands::create_post,
            presentation::commands::get_posts,
            presentation::commands::list_trending_posts,
            presentation::commands::delete_post,
            presentation::commands::bookmark_post,
            presentation::commands::unbookmark_post,
            presentation::commands::like_post,
            presentation::commands::boost_post,
            presentation::commands::get_bookmarked_post_ids,
            presentation::commands::list_following_feed,
            // Nostr関連コマンド
            presentation::commands::initialize_nostr,
            presentation::commands::publish_text_note,
            presentation::commands::publish_topic_post,
            presentation::commands::send_reaction,
            presentation::commands::update_nostr_metadata,
            presentation::commands::subscribe_to_topic,
            presentation::commands::subscribe_to_user,
            presentation::commands::disconnect_nostr,
            presentation::commands::list_nostr_subscriptions,
            // P2P関連コマンド
            presentation::commands::initialize_p2p,
            presentation::commands::join_p2p_topic,
            presentation::commands::leave_p2p_topic,
            presentation::commands::broadcast_to_topic,
            presentation::commands::get_p2p_status,
            presentation::commands::get_node_address,
            presentation::commands::get_p2p_metrics,
            // オフライン関連コマンド
            presentation::commands::save_offline_action,
            presentation::commands::get_offline_actions,
            presentation::commands::sync_offline_actions,
            presentation::commands::get_cache_status,
            presentation::commands::list_sync_queue_items,
            presentation::commands::add_to_sync_queue,
            presentation::commands::update_cache_metadata,
            presentation::commands::save_optimistic_update,
            presentation::commands::confirm_optimistic_update,
            presentation::commands::rollback_optimistic_update,
            presentation::commands::cleanup_expired_cache,
            presentation::commands::update_sync_status,
            presentation::commands::record_offline_retry_outcome,
            presentation::commands::get_offline_retry_metrics,
            // Community Node commands
            presentation::commands::set_community_node_config,
            presentation::commands::get_community_node_config,
            presentation::commands::clear_community_node_config,
            presentation::commands::community_node_authenticate,
            presentation::commands::community_node_clear_token,
            presentation::commands::community_node_list_group_keys,
            presentation::commands::community_node_sync_key_envelopes,
            presentation::commands::community_node_redeem_invite,
            presentation::commands::community_node_list_labels,
            presentation::commands::community_node_submit_report,
            presentation::commands::community_node_trust_report_based,
            presentation::commands::community_node_trust_communication_density,
            presentation::commands::community_node_search,
            presentation::commands::community_node_list_bootstrap_nodes,
            presentation::commands::community_node_list_bootstrap_services,
            presentation::commands::community_node_get_consent_status,
            presentation::commands::community_node_accept_consents,
            // Access Control (P2P join)
            presentation::commands::access_control_issue_invite,
            presentation::commands::access_control_request_join,
            // ユーティリティコマンド
            presentation::commands::pubkey_to_npub,
            presentation::commands::npub_to_pubkey,
            // Bootstrap UI commands
            presentation::commands::get_bootstrap_config,
            presentation::commands::set_bootstrap_nodes,
            presentation::commands::clear_bootstrap_nodes,
            presentation::commands::apply_cli_bootstrap_nodes,
            presentation::commands::get_relay_status,
        ])
        .setup(|app| {
            // アプリケーション初期化処理
            let app_handle = app.handle();

            tauri::async_runtime::block_on(async move {
                let app_state = AppState::new(app_handle)
                    .await
                    .expect("Failed to initialize app state");
                let offline_reindex_job = Arc::clone(&app_state.offline_reindex_job);
                let user_service = Arc::clone(&app_state.user_service);
                let user_search_service = Arc::clone(&app_state.user_search_service);
                let event_service = Arc::clone(&app_state.event_service);
                let p2p_service = Arc::clone(&app_state.p2p_service);
                let offline_service = Arc::clone(&app_state.offline_service);
                let profile_avatar_service = Arc::clone(&app_state.profile_avatar_service);

                // P2P機能を初期化
                if let Err(e) = app_state.initialize_p2p().await {
                    tracing::warn!("Failed to initialize P2P: {}", e);
                }

                // P2Pイベントハンドラーを起動
                spawn_p2p_event_handler(app_handle.clone(), app_state.clone());

                app_handle.manage(app_state.clone());
                app_handle.manage(offline_reindex_job);
                app_handle.manage(user_service);
                app_handle.manage(user_search_service);
                app_handle.manage(event_service);
                app_handle.manage(p2p_service);
                app_handle.manage(offline_service);
                app_handle.manage(profile_avatar_service);
            });

            info!("Application setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    const DEFAULT_DIRECTIVES: &str = "info,kukuri=debug";
    const MAINLINE_SUPPRESS_DIRECTIVE: &str = "mainline::rpc::socket=error";

    let mut env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| DEFAULT_DIRECTIVES.into());

    let suppress_mainline = std::env::var("RUST_LOG")
        .map(|value| !value.contains("mainline::rpc::socket"))
        .unwrap_or(true);

    if suppress_mainline {
        if let Ok(directive) = MAINLINE_SUPPRESS_DIRECTIVE.parse() {
            env_filter = env_filter.add_directive(directive);
        } else {
            tracing::warn!(
                "Failed to parse mainline log suppression directive: {}",
                MAINLINE_SUPPRESS_DIRECTIVE
            );
        }
    }

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// P2Pイベントハンドラーを起動
fn spawn_p2p_event_handler(app_handle: tauri::AppHandle, app_state: AppState) {
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize)]
    struct P2PMessageEvent {
        topic_id: String,
        message_type: String,
        payload: Vec<u8>,
        sender: Vec<u8>,
        timestamp: i64,
    }

    #[derive(Debug, Clone, Serialize)]
    struct P2PPeerEvent {
        topic_id: String,
        peer_id: Vec<u8>,
        event_type: String, // "joined" or "left"
    }

    tauri::async_runtime::spawn(async move {
        // event_rxを取得してRwLockを即座に解放
        let rx = {
            let p2p_state = app_state.p2p_state.read().await;
            let mut event_rx = p2p_state.event_rx.write().await;
            event_rx.take()
        };

        if let Some(mut rx) = rx {
            loop {
                match rx.recv().await {
                    Ok(event) => match event {
                        crate::domain::p2p::P2PEvent::MessageReceived {
                            topic_id,
                            message,
                            _from_peer: _,
                        } => {
                            // 旧GossipMessage経路はUIの期待ペイロードと形状が異なるため、
                            // 衝突回避のためイベント名を変更（デバッグ用途）
                            let event_data = P2PMessageEvent {
                                topic_id,
                                message_type: format!("{:?}", message.msg_type),
                                payload: message.payload,
                                sender: message.sender,
                                timestamp: message.timestamp,
                            };

                            if let Err(e) = app_handle.emit("p2p://message/raw", event_data) {
                                tracing::error!("Failed to emit P2P raw message event: {}", e);
                            }
                        }
                        crate::domain::p2p::P2PEvent::PeerJoined { topic_id, peer_id } => {
                            let event_data = P2PPeerEvent {
                                topic_id,
                                peer_id,
                                event_type: "joined".to_string(),
                            };

                            if let Err(e) = app_handle.emit("p2p://peer", event_data) {
                                tracing::error!("Failed to emit P2P peer joined event: {}", e);
                            }
                        }
                        crate::domain::p2p::P2PEvent::PeerLeft { topic_id, peer_id } => {
                            let event_data = P2PPeerEvent {
                                topic_id,
                                peer_id,
                                event_type: "left".to_string(),
                            };

                            if let Err(e) = app_handle.emit("p2p://peer", event_data) {
                                tracing::error!("Failed to emit P2P peer left event: {}", e);
                            }
                        }
                        crate::domain::p2p::P2PEvent::NetworkConnected { node_id, addresses } => {
                            if let Err(e) = app_handle.emit(
                                "p2p://network",
                                serde_json::json!({
                                    "event": "connected",
                                    "nodeId": node_id,
                                    "addresses": addresses,
                                }),
                            ) {
                                tracing::error!("Failed to emit P2P network connected: {}", e);
                            }
                        }
                        crate::domain::p2p::P2PEvent::NetworkDisconnected { node_id } => {
                            if let Err(e) = app_handle.emit(
                                "p2p://network",
                                serde_json::json!({
                                    "event": "disconnected",
                                    "nodeId": node_id,
                                }),
                            ) {
                                tracing::error!("Failed to emit P2P network disconnected: {}", e);
                            }
                        }
                    },
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!("P2P event handler lagged and skipped {} events", skipped);
                        continue;
                    }
                }
            }

            tracing::info!("P2P event handler terminated");
        }
    });
}
