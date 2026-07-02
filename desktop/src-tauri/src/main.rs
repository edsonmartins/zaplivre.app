// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod identity_store;
#[cfg(target_os = "macos")]
mod macos_video;

use std::sync::{Arc, Mutex};
use tracing_subscriber;

fn main() {
    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter("mepassa_desktop=debug,mepassa_core=debug")
        .init();

    // Initialize client state
    let client_state: Arc<Mutex<Option<Arc<mepassa_core::ffi::MePassaClient>>>> =
        Arc::new(Mutex::new(None));

    if std::env::var("MESSAGE_STORE_URL").is_err() {
        std::env::set_var("MESSAGE_STORE_URL", "https://store.associahub.com.br");
    }
    if std::env::var("SIGNALING_SERVER_URL").is_err() {
        std::env::set_var("SIGNALING_SERVER_URL", "wss://signaling.associahub.com.br/ws");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(client_state)
        .invoke_handler(tauri::generate_handler![
            commands::init_client,
            commands::get_local_peer_id,
            commands::listen_on,
            commands::connect_to_peer,
            commands::send_text_message,
            commands::get_conversation_messages,
            commands::get_conversation_media,
            commands::download_media,
            commands::list_conversations,
            commands::search_messages,
            commands::mark_conversation_read,
            commands::get_connected_peers_count,
            commands::get_listening_addresses,
            commands::bootstrap,
            commands::show_notification,
            // Group commands (FASE 15)
            commands::create_group,
            commands::join_group,
            commands::leave_group,
            commands::add_group_member,
            commands::remove_group_member,
            commands::get_groups,
            commands::get_group_messages,
            commands::send_group_message,
            commands::get_group_sender_key_seed,
            commands::add_group_sender_key,
            commands::get_group_members,
            commands::update_group,
            commands::send_file_message,
            commands::add_reaction,
            commands::remove_reaction,
            commands::get_message_reactions,
            commands::forward_message,
            commands::export_identity_backup,
            commands::import_identity_backup,
            // VoIP commands (FASE 12)
            commands::start_call,
            commands::accept_call,
            commands::reject_call,
            commands::hangup_call,
            commands::toggle_mute,
            commands::toggle_speakerphone,
            commands::switch_camera,
            commands::enable_video,
            commands::disable_video,
            commands::register_video_frame_callback,
        ])
        .setup(|_app| {
            // Setup app-specific initialization here
            tracing::info!("MePassa Desktop starting...");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
