use tracing::info;
use tauri::Manager;

// モジュール定義
mod modules;
mod state;

// Tauriコマンドのインポート
use modules::auth::commands as auth_commands;
use modules::topic::commands as topic_commands;
use modules::post::commands as post_commands;
use modules::event::commands as event_commands;
use modules::p2p::commands as p2p_commands;
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
            // Nostr関連コマンド
            event_commands::initialize_nostr,
            event_commands::add_relay,
            event_commands::publish_text_note,
            event_commands::publish_topic_post,
            event_commands::send_reaction,
            event_commands::update_nostr_metadata,
            event_commands::subscribe_to_topic,
            event_commands::subscribe_to_user,
            event_commands::get_nostr_pubkey,
            event_commands::delete_events,
            event_commands::disconnect_nostr,
            event_commands::get_relay_status,
            // P2P関連コマンド
            p2p_commands::initialize_p2p,
            p2p_commands::join_p2p_topic,
            p2p_commands::leave_p2p_topic,
            p2p_commands::broadcast_to_topic,
            p2p_commands::get_p2p_status,
            p2p_commands::get_node_address,
            p2p_commands::join_topic_by_name,
        ])
        .setup(|app| {
            // アプリケーション初期化処理
            let app_handle = app.handle();
            
            tauri::async_runtime::block_on(async move {
                let app_state = AppState::new().await
                    .expect("Failed to initialize app state");
                
                // EventManagerにAppHandleを設定
                app_state.event_manager.set_app_handle(app_handle.clone()).await;
                
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
