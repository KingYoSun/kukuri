use tracing::info;

// モジュール定義
mod modules;

// Tauriコマンドのインポート
use modules::auth::commands as auth_commands;

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
        ])
        .setup(|_app| {
            // アプリケーション初期化処理
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
