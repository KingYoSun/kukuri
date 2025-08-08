use tauri::{Emitter, Manager};
use tracing::info;

// モジュール定義
mod modules;
mod state;

// Tauriコマンドのインポート
use modules::auth::commands as auth_commands;
use modules::event::commands as event_commands;
use modules::offline::commands as offline_commands;
use modules::p2p::commands as p2p_commands;
use modules::post::commands as post_commands;
use modules::secure_storage as secure_storage_commands;
use modules::topic::commands as topic_commands;
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
            auth_commands::generate_keypair,
            auth_commands::login,
            auth_commands::logout,
            // セキュアストレージ関連コマンド
            secure_storage_commands::add_account,
            secure_storage_commands::list_accounts,
            secure_storage_commands::switch_account,
            secure_storage_commands::remove_account,
            secure_storage_commands::get_current_account,
            secure_storage_commands::secure_login,
            // トピック関連コマンド
            topic_commands::get_topics,
            topic_commands::create_topic,
            topic_commands::update_topic,
            topic_commands::delete_topic,
            // ポスト関連コマンド
            post_commands::get_posts,
            post_commands::create_post,
            post_commands::delete_post,
            post_commands::like_post,
            post_commands::boost_post,
            post_commands::bookmark_post,
            post_commands::unbookmark_post,
            post_commands::get_bookmarked_post_ids,
            // Nostr関連コマンド
            event_commands::initialize_nostr,
            event_commands::publish_text_note,
            event_commands::publish_topic_post,
            event_commands::send_reaction,
            event_commands::update_nostr_metadata,
            event_commands::subscribe_to_topic,
            event_commands::subscribe_to_user,
            event_commands::get_nostr_pubkey,
            event_commands::delete_events,
            event_commands::disconnect_nostr,
            // P2P関連コマンド
            p2p_commands::initialize_p2p,
            p2p_commands::join_p2p_topic,
            p2p_commands::leave_p2p_topic,
            p2p_commands::broadcast_to_topic,
            p2p_commands::get_p2p_status,
            p2p_commands::get_node_address,
            p2p_commands::join_topic_by_name,
            // オフライン関連コマンド
            offline_commands::save_offline_action,
            offline_commands::get_offline_actions,
            offline_commands::sync_offline_actions,
            offline_commands::get_cache_status,
            offline_commands::add_to_sync_queue,
            offline_commands::update_cache_metadata,
            offline_commands::save_optimistic_update,
            offline_commands::confirm_optimistic_update,
            offline_commands::rollback_optimistic_update,
            offline_commands::cleanup_expired_cache,
            offline_commands::update_sync_status,
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
