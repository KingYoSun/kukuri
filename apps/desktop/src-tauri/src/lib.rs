mod commands;
mod state;
mod tracing;

use ::tracing::{error, info};
use tauri::{
    AppHandle, Manager, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::{
    commands::background_notifications::OsNotificationBackground,
    state::{
        DesktopStartupState, build_desktop_state, consent_satisfied, load_app_consent,
        resolve_db_path,
    },
    tracing::init_tracing,
};

/// Bring the main window back from the tray.
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Build the system tray so kukuri stays resident after the window is closed
/// (issue #304). Closing the window hides it; the app keeps syncing in the
/// background and only exits via the tray "Quit" entry.
fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "Open kukuri", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("kukuri")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            info!(?argv, "received kukuri desktop single-instance activation");
            // The app may be resident in the tray with its window hidden
            // (issue #304); a re-launch should bring it back to the front.
            show_main_window(app);
        }));
    }

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .on_window_event(|window, event| {
            // Issue #304: closing the window keeps kukuri running in the
            // background (tray) instead of exiting. Only the tray "Quit" entry
            // terminates the process.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let accepted_bundle_version = resolve_db_path(app.handle())
                .ok()
                .and_then(|db_path| load_app_consent(&db_path))
                .map(|record| record.accepted_bundle_version);

            let startup_state = if consent_satisfied(accepted_bundle_version) {
                match build_desktop_state(app.handle()) {
                    Ok(state) => {
                        info!("initialized kukuri desktop runtime");
                        app.manage(state);
                        DesktopStartupState::ready()
                    }
                    Err(error) => {
                        error!(%error, "failed to initialize desktop runtime");
                        let db_path = resolve_db_path(app.handle()).ok();
                        DesktopStartupState::failed(error, db_path)
                    }
                }
            } else {
                info!("app-level legal consent required; deferring runtime startup");
                DesktopStartupState::consent_required(
                    crate::state::LEGAL_BUNDLE_VERSION,
                    accepted_bundle_version,
                )
            };
            app.manage(startup_state);
            app.manage(OsNotificationBackground::new(app.handle()));
            if let Err(error) = build_tray(app.handle()) {
                error!(%error, "failed to build system tray");
            }
            commands::background_notifications::spawn(app.handle().clone());
            #[cfg(any(windows, target_os = "linux"))]
            app.deep_link().register_all()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::startup::get_desktop_startup_status,
            commands::app_consent::get_app_consent_status,
            commands::app_consent::accept_app_consents,
            commands::posts::create_post,
            commands::posts::create_repost,
            commands::reactions::toggle_reaction,
            commands::reactions::list_my_custom_reaction_assets,
            commands::reactions::list_recent_reactions,
            commands::reactions::create_custom_reaction_asset,
            commands::reactions::list_bookmarked_custom_reactions,
            commands::reactions::bookmark_custom_reaction,
            commands::reactions::remove_bookmarked_custom_reaction,
            commands::posts::list_bookmarked_posts,
            commands::posts::bookmark_post,
            commands::posts::remove_bookmarked_post,
            commands::community_node::create_private_channel,
            commands::community_node::export_private_channel_invite,
            commands::community_node::import_private_channel_invite,
            commands::community_node::export_channel_access_token,
            commands::community_node::preview_channel_access_token,
            commands::community_node::import_channel_access_token,
            commands::community_node::export_friend_only_grant,
            commands::community_node::import_friend_only_grant,
            commands::community_node::export_friend_plus_share,
            commands::community_node::import_friend_plus_share,
            commands::community_node::freeze_private_channel,
            commands::community_node::rotate_private_channel,
            commands::community_node::leave_private_channel,
            commands::community_node::list_joined_private_channels,
            commands::posts::list_timeline,
            commands::posts::list_thread,
            commands::posts::list_profile_timeline,
            commands::profile::get_my_profile,
            commands::profile::set_my_profile,
            commands::profile::follow_author,
            commands::profile::unfollow_author,
            commands::profile::get_author_social_view,
            commands::profile::mute_author,
            commands::profile::unmute_author,
            commands::profile::list_social_connections,
            commands::profile::list_notifications,
            commands::profile::mark_notification_read,
            commands::profile::mark_all_notifications_read,
            commands::profile::get_notification_status,
            commands::direct_messages::open_direct_message,
            commands::direct_messages::list_direct_messages,
            commands::direct_messages::list_direct_message_messages,
            commands::direct_messages::send_direct_message,
            commands::direct_messages::delete_direct_message_message,
            commands::direct_messages::clear_direct_message,
            commands::direct_messages::get_direct_message_status,
            commands::community_node::get_sync_status,
            commands::community_node::get_discovery_config,
            commands::live_game::list_live_sessions,
            commands::live_game::create_live_session,
            commands::live_game::end_live_session,
            commands::live_game::join_live_session,
            commands::live_game::leave_live_session,
            commands::live_game::list_game_rooms,
            commands::live_game::create_game_room,
            commands::live_game::update_game_room,
            commands::live_game::create_metaverse_room,
            commands::live_game::update_metaverse_room,
            commands::live_game::publish_metaverse_room_event,
            commands::live_game::list_metaverse_room_events,
            commands::live_game::import_metaverse_room_asset,
            commands::community_node::import_peer_ticket,
            commands::community_node::set_discovery_seeds,
            commands::community_node::unsubscribe_topic,
            commands::community_node::set_topic_gossip_enabled,
            commands::community_node::set_channel_gossip_enabled,
            commands::community_node::get_local_peer_ticket,
            commands::posts::get_blob_media_payload,
            commands::posts::get_blob_preview_url,
            commands::community_node::get_community_node_config,
            commands::community_node::get_community_node_statuses,
            commands::community_node::set_community_node_config,
            commands::community_node::clear_community_node_config,
            commands::community_node::authenticate_community_node,
            commands::community_node::clear_community_node_token,
            commands::community_node::get_community_node_consent_status,
            commands::community_node::accept_community_node_consents,
            commands::community_node::refresh_community_node_metadata,
            commands::community_node::fetch_community_node_manifest,
            commands::community_node::submit_community_node_report,
            commands::os_notification::show_os_notification,
            commands::background_notifications::set_os_notification_settings
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kukuri desktop tauri app");
}
