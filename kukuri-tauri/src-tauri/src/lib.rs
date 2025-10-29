use std::sync::Arc;

use tauri::{Emitter, Manager};
use tokio::sync::broadcast;
use tracing::info;

// モジュール定義
mod application;
mod domain;
mod infrastructure;
mod modules;
mod presentation;
mod shared;
mod state;

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
        pub use crate::infrastructure::offline;
        pub use crate::infrastructure::p2p;
    }
    pub mod shared {
        pub use crate::shared::config;
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
        .invoke_handler(tauri::generate_handler![
            // 認証関連コマンド
            presentation::commands::generate_keypair,
            presentation::commands::login,
            presentation::commands::logout,
            // セキュアストレージ関連コマンド
            presentation::commands::add_account,
            presentation::commands::list_accounts,
            presentation::commands::switch_account,
            presentation::commands::remove_account,
            presentation::commands::get_current_account,
            presentation::commands::secure_login,
            // テスト用コマンド
            presentation::commands::clear_all_accounts_for_test,
            // トピック関連コマンド
            presentation::commands::create_topic,
            presentation::commands::get_topic,
            presentation::commands::get_topics,
            presentation::commands::get_joined_topics,
            presentation::commands::update_topic,
            presentation::commands::delete_topic,
            presentation::commands::join_topic,
            presentation::commands::leave_topic,
            presentation::commands::get_topic_stats,
            // ポスト関連コマンド
            presentation::commands::create_post,
            presentation::commands::get_posts,
            presentation::commands::delete_post,
            presentation::commands::react_to_post,
            presentation::commands::bookmark_post,
            presentation::commands::unbookmark_post,
            presentation::commands::like_post,
            presentation::commands::boost_post,
            presentation::commands::get_bookmarked_post_ids,
            presentation::commands::batch_get_posts,
            presentation::commands::batch_react,
            presentation::commands::batch_bookmark,
            // Nostr関連コマンド
            presentation::commands::initialize_nostr,
            presentation::commands::publish_text_note,
            presentation::commands::publish_topic_post,
            presentation::commands::send_reaction,
            presentation::commands::update_nostr_metadata,
            presentation::commands::subscribe_to_topic,
            presentation::commands::subscribe_to_user,
            presentation::commands::get_nostr_pubkey,
            presentation::commands::delete_events,
            presentation::commands::disconnect_nostr,
            presentation::commands::set_default_p2p_topic,
            presentation::commands::list_nostr_subscriptions,
            // P2P関連コマンド
            presentation::commands::initialize_p2p,
            presentation::commands::join_p2p_topic,
            presentation::commands::leave_p2p_topic,
            presentation::commands::broadcast_to_topic,
            presentation::commands::get_p2p_status,
            presentation::commands::get_node_address,
            presentation::commands::join_topic_by_name,
            presentation::commands::get_p2p_metrics,
            // オフライン関連コマンド
            presentation::commands::save_offline_action,
            presentation::commands::get_offline_actions,
            presentation::commands::sync_offline_actions,
            presentation::commands::get_cache_status,
            presentation::commands::add_to_sync_queue,
            presentation::commands::update_cache_metadata,
            presentation::commands::save_optimistic_update,
            presentation::commands::confirm_optimistic_update,
            presentation::commands::rollback_optimistic_update,
            presentation::commands::cleanup_expired_cache,
            presentation::commands::update_sync_status,
            // ユーティリティコマンド
            presentation::commands::pubkey_to_npub,
            presentation::commands::npub_to_pubkey,
            // Bootstrap UI commands
            presentation::commands::get_bootstrap_config,
            presentation::commands::set_bootstrap_nodes,
            presentation::commands::clear_bootstrap_nodes,
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
                let event_service = Arc::clone(&app_state.event_service);
                let p2p_service = Arc::clone(&app_state.p2p_service);
                let offline_service = Arc::clone(&app_state.offline_service);
                let user_handler = Arc::clone(&app_state.user_handler);

                // P2P機能を初期化
                if let Err(e) = app_state.initialize_p2p().await {
                    tracing::warn!("Failed to initialize P2P: {}", e);
                }

                // P2Pイベントハンドラーを起動
                spawn_p2p_event_handler(app_handle.clone(), app_state.clone());

                app_handle.manage(app_state.clone());
                app_handle.manage(offline_reindex_job);
                app_handle.manage(user_service);
                app_handle.manage(event_service);
                app_handle.manage(p2p_service);
                app_handle.manage(offline_service);
                app_handle.manage(user_handler);
            });

            info!("Application setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kukuri=debug,info".into()),
        )
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
