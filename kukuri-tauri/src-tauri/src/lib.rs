use tauri::{Emitter, Manager};
use tracing::info;

// モジュール定義
mod modules;
mod state;
mod application;
mod infrastructure;
mod domain;
mod presentation;
mod shared;

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
            // 認証関連コマンド（旧）
            // auth_commands::generate_keypair,  // v2に移行済み
            // auth_commands::login,  // v2に移行済み
            // auth_commands::logout,  // v2に移行済み
            // v2認証コマンド
            presentation::commands::generate_keypair_v2,
            presentation::commands::login_v2,
            presentation::commands::logout_v2,
            // セキュアストレージ関連コマンド（旧）
            // secure_storage_commands::add_account,  // v2に移行済み
            // secure_storage_commands::list_accounts,  // v2に移行済み
            // secure_storage_commands::switch_account,  // v2に移行済み
            // secure_storage_commands::remove_account,  // v2に移行済み
            // secure_storage_commands::get_current_account,  // v2に移行済み
            // secure_storage_commands::secure_login,  // v2に移行済み
            // v2セキュアストレージコマンド
            presentation::commands::add_account_v2,
            presentation::commands::list_accounts_v2,
            presentation::commands::switch_account_v2,
            presentation::commands::remove_account_v2,
            presentation::commands::get_current_account_v2,
            presentation::commands::secure_login_v2,
            // トピック関連コマンド（旧）
            // topic_commands::get_topics,  // v2に移行済み
            // topic_commands::update_topic,  // v2に移行済み
            // v2トピックコマンド
            presentation::commands::create_topic_v2,
            presentation::commands::get_topics_v2,
            presentation::commands::update_topic_v2,
            presentation::commands::delete_topic_v2,
            presentation::commands::join_topic_v2,
            presentation::commands::leave_topic_v2,
            presentation::commands::get_topic_stats_v2,
            // ポスト関連コマンド（旧）
            // post_commands::get_posts,  // v2に移行済み
            // post_commands::bookmark_post,  // v2に移行済み
            // post_commands::unbookmark_post,  // v2に移行済み
            // post_commands::get_bookmarked_post_ids,  // v2に移行済み
            // v2コマンド
            presentation::commands::create_post_v2,
            presentation::commands::get_posts_v2,
            presentation::commands::delete_post_v2,
            presentation::commands::react_to_post_v2,
            presentation::commands::bookmark_post_v2,
            presentation::commands::unbookmark_post_v2,
            presentation::commands::like_post_v2,
            presentation::commands::boost_post_v2,
            presentation::commands::get_bookmarked_post_ids_v2,
            presentation::commands::batch_get_posts_v2,
            presentation::commands::batch_react_v2,
            presentation::commands::batch_bookmark_v2,
            // Nostr関連コマンド（旧）
            // event_commands::initialize_nostr,  // v2に移行済み
            // event_commands::publish_text_note,  // v2に移行済み
            // event_commands::publish_topic_post,  // v2に移行済み
            // event_commands::send_reaction,  // v2に移行済み
            // event_commands::update_nostr_metadata,  // v2に移行済み
            // event_commands::subscribe_to_topic,  // v2に移行済み
            // event_commands::subscribe_to_user,  // v2に移行済み
            // event_commands::get_nostr_pubkey,  // v2に移行済み
            // event_commands::delete_events,  // v2に移行済み
            // event_commands::disconnect_nostr,  // v2に移行済み
            // v2 Nostrコマンド
            presentation::commands::initialize_nostr_v2,
            presentation::commands::publish_text_note_v2,
            presentation::commands::publish_topic_post_v2,
            presentation::commands::send_reaction_v2,
            presentation::commands::update_nostr_metadata_v2,
            presentation::commands::subscribe_to_topic_v2,
            presentation::commands::subscribe_to_user_v2,
            presentation::commands::get_nostr_pubkey_v2,
            presentation::commands::delete_events_v2,
            presentation::commands::disconnect_nostr_v2,
            // P2P関連コマンド（旧）
            // p2p_commands::initialize_p2p,  // v2に移行済み
            // p2p_commands::join_p2p_topic,  // v2に移行済み
            // p2p_commands::leave_p2p_topic,  // v2に移行済み
            // p2p_commands::broadcast_to_topic,  // v2に移行済み
            // p2p_commands::get_p2p_status,  // v2に移行済み
            // p2p_commands::get_node_address,  // v2に移行済み
            // p2p_commands::join_topic_by_name,  // v2に移行済み
            // v2 P2Pコマンド
            presentation::commands::initialize_p2p_v2,
            presentation::commands::join_p2p_topic_v2,
            presentation::commands::leave_p2p_topic_v2,
            presentation::commands::broadcast_to_topic_v2,
            presentation::commands::get_p2p_status_v2,
            presentation::commands::get_node_address_v2,
            presentation::commands::join_topic_by_name_v2,
            // オフライン関連コマンド（旧）
            // offline_commands::save_offline_action,  // v2に移行済み
            // offline_commands::get_offline_actions,  // v2に移行済み
            // offline_commands::sync_offline_actions,  // v2に移行済み
            // offline_commands::get_cache_status,  // v2に移行済み
            // offline_commands::add_to_sync_queue,  // v2に移行済み
            // offline_commands::update_cache_metadata,  // v2に移行済み
            // offline_commands::save_optimistic_update,  // v2に移行済み
            // offline_commands::confirm_optimistic_update,  // v2に移行済み
            // offline_commands::rollback_optimistic_update,  // v2に移行済み
            // offline_commands::cleanup_expired_cache,  // v2に移行済み
            // offline_commands::update_sync_status,  // v2に移行済み
            // v2 オフラインコマンド
            presentation::commands::save_offline_action_v2,
            presentation::commands::get_offline_actions_v2,
            presentation::commands::sync_offline_actions_v2,
            presentation::commands::get_cache_status_v2,
            presentation::commands::add_to_sync_queue_v2,
            presentation::commands::update_cache_metadata_v2,
            presentation::commands::save_optimistic_update_v2,
            presentation::commands::confirm_optimistic_update_v2,
            presentation::commands::rollback_optimistic_update_v2,
            presentation::commands::cleanup_expired_cache_v2,
            presentation::commands::update_sync_status_v2,
            // ユーティリティコマンド（旧）
            // utils_commands::pubkey_to_npub,  // v2に移行済み
            // utils_commands::npub_to_pubkey,  // v2に移行済み
            // v2 ユーティリティコマンド
            presentation::commands::pubkey_to_npub_v2,
            presentation::commands::npub_to_pubkey_v2,
        ])
        .setup(|app| {
            // アプリケーション初期化処理
            let app_handle = app.handle();

            tauri::async_runtime::block_on(async move {
                let app_state = AppState::new(app_handle)
                    .await
                    .expect("Failed to initialize app state");

                // EventManagerにAppHandleを設定
                app_state
                    .event_manager
                    .set_app_handle(app_handle.clone())
                    .await;

                // P2P機能を初期化
                if let Err(e) = app_state.initialize_p2p().await {
                    tracing::warn!("Failed to initialize P2P: {}", e);
                }

                // P2Pイベントハンドラーを起動
                spawn_p2p_event_handler(app_handle.clone(), app_state.clone());

                app_handle.manage(app_state);
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
            while let Some(event) = rx.recv().await {
                match event {
                    modules::p2p::P2PEvent::MessageReceived {
                        topic_id,
                        message,
                        _from_peer: _,
                    } => {
                        let event_data = P2PMessageEvent {
                            topic_id,
                            message_type: format!("{:?}", message.msg_type),
                            payload: message.payload,
                            sender: message.sender,
                            timestamp: message.timestamp,
                        };

                        if let Err(e) = app_handle.emit("p2p://message", event_data) {
                            tracing::error!("Failed to emit P2P message event: {}", e);
                        }
                    }
                    modules::p2p::P2PEvent::PeerJoined { topic_id, peer_id } => {
                        let event_data = P2PPeerEvent {
                            topic_id,
                            peer_id,
                            event_type: "joined".to_string(),
                        };

                        if let Err(e) = app_handle.emit("p2p://peer", event_data) {
                            tracing::error!("Failed to emit P2P peer joined event: {}", e);
                        }
                    }
                    modules::p2p::P2PEvent::PeerLeft { topic_id, peer_id } => {
                        let event_data = P2PPeerEvent {
                            topic_id,
                            peer_id,
                            event_type: "left".to_string(),
                        };

                        if let Err(e) = app_handle.emit("p2p://peer", event_data) {
                            tracing::error!("Failed to emit P2P peer left event: {}", e);
                        }
                    }
                }
            }

            tracing::info!("P2P event handler terminated");
        }
    });
}
