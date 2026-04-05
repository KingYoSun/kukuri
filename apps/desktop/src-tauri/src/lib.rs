mod commands;
mod state;
mod tracing;

use ::tracing::info;
use tauri::Manager;

use crate::{state::build_desktop_state, tracing::init_tracing};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .setup(|app| {
            let state = build_desktop_state(app.handle())?;
            info!("initialized kukuri desktop runtime");
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            commands::community_node::import_channel_access_token,
            commands::community_node::export_friend_only_grant,
            commands::community_node::import_friend_only_grant,
            commands::community_node::export_friend_plus_share,
            commands::community_node::import_friend_plus_share,
            commands::community_node::freeze_private_channel,
            commands::community_node::rotate_private_channel,
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
            commands::community_node::import_peer_ticket,
            commands::community_node::set_discovery_seeds,
            commands::community_node::unsubscribe_topic,
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
            commands::community_node::refresh_community_node_metadata
        ])
        .run(tauri::generate_context!())
        .expect("failed to run kukuri desktop tauri app");
}
