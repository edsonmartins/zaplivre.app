use mepassa_core::ffi::MePassaClient;
use mepassa_core::{FfiVideoCodec, FfiVideoFrameCallback, FfiVoipEventCallback};
use std::sync::{Arc, Mutex};
use tauri::State;
use base64::{engine::general_purpose, Engine as _};
use mepassa_core::FfiMediaType;
use tauri_plugin_notification::NotificationExt;
use tauri::Emitter;
use crate::identity_store;
#[cfg(target_os = "macos")]
use crate::macos_video::MacVideoDecoder;

// Global client state - use Arc to allow cloning the handle
type ClientState = Arc<Mutex<Option<Arc<MePassaClient>>>>;

struct VoipEventLogger {
    app: tauri::AppHandle,
}

impl FfiVoipEventCallback for VoipEventLogger {
    fn on_mute_changed(&self, call_id: String, is_muted: bool) {
        tracing::info!("🔇 VoIP mute changed: {} -> {}", call_id, is_muted);
        let _ = self.app.emit(
            "voip:mute_changed",
            serde_json::json!({ "call_id": call_id, "is_muted": is_muted }),
        );
    }

    fn on_speakerphone_changed(&self, call_id: String, enabled: bool) {
        tracing::info!("🔊 VoIP speaker changed: {} -> {}", call_id, enabled);
        let _ = self.app.emit(
            "voip:speaker_changed",
            serde_json::json!({ "call_id": call_id, "enabled": enabled }),
        );
    }

    fn on_camera_switch_requested(&self, call_id: String) {
        tracing::info!("📸 VoIP camera switch requested: {}", call_id);
        let _ = self.app.emit(
            "voip:camera_switch_requested",
            serde_json::json!({ "call_id": call_id }),
        );
    }
}

struct VideoFrameEmitter {
    app: tauri::AppHandle,
    #[cfg(target_os = "macos")]
    decoder: std::sync::Mutex<MacVideoDecoder>,
}

impl FfiVideoFrameCallback for VideoFrameEmitter {
    fn on_video_frame(&self, call_id: String, frame_data: Vec<u8>, width: u32, height: u32) {
        #[cfg(target_os = "macos")]
        {
            if let Ok(mut decoder) = self.decoder.lock() {
                if let Some(decoded) = decoder.decode_annexb(&frame_data) {
                    let payload = serde_json::json!({
                        "call_id": call_id,
                        "width": decoded.width,
                        "height": decoded.height,
                        "size": decoded.rgba.len(),
                        "data_b64": general_purpose::STANDARD.encode(decoded.rgba),
                    });
                    let _ = self.app.emit("voip:video_frame_rgba", payload);
                    return;
                }
            }
        }

        let payload = serde_json::json!({
            "call_id": call_id,
            "width": width,
            "height": height,
            "size": frame_data.len(),
            "data_b64": general_purpose::STANDARD.encode(frame_data),
        });
        let _ = self.app.emit("voip:video_frame", payload);
    }
}

#[tauri::command]
pub async fn init_client(
    state: State<'_, ClientState>,
    data_dir: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tracing::info!("🔵 init_client CALLED with data_dir: {}", data_dir);

    if let Ok(Some(b64)) = identity_store::load_identity_b64() {
        std::env::set_var("MEPASSA_IDENTITY_B64", b64);
    } else {
        std::env::remove_var("MEPASSA_IDENTITY_B64");
    }

    // MePassaClient::new() is synchronous, not async
    tracing::info!("🔵 Creating MePassaClient...");
    let client = Arc::new(MePassaClient::new(data_dir.clone()).map_err(|e| {
        tracing::error!("❌ Failed to create MePassaClient: {}", e);
        e.to_string()
    })?);

    tracing::info!("🔵 Getting local peer ID...");
    let client_clone = client.clone();
    let peer_id = tokio::task::spawn_blocking(move || {
        client_clone.local_peer_id()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| {
        tracing::error!("❌ Failed to get local peer ID: {}", e);
        e.to_string()
    })?;

    tracing::info!("🔵 Storing client in state...");
    let mut client_guard = state.lock().map_err(|e| {
        tracing::error!("❌ Failed to lock state: {}", e);
        e.to_string()
    })?;
    *client_guard = Some(client);

    if let Some(client) = client_guard.as_ref() {
        if let Err(err) = client.register_voip_event_callback(Box::new(VoipEventLogger { app })) {
            tracing::warn!("Failed to register VoIP event callback: {}", err);
        }
    }

    if let Some(encoded) = read_identity_key_b64(&data_dir) {
        let _ = identity_store::save_identity_b64(&encoded);
        let _ = remove_identity_key_file(&data_dir);
    }

    tracing::info!("✅ Client initialized successfully with peer_id: {}", peer_id);
    Ok(peer_id)
}

fn read_identity_key_b64(data_dir: &str) -> Option<String> {
    let path = std::path::Path::new(data_dir).join("identity.key");
    let bytes = std::fs::read(path).ok()?;
    Some(general_purpose::STANDARD.encode(bytes))
}

fn remove_identity_key_file(data_dir: &str) -> std::io::Result<()> {
    let path = std::path::Path::new(data_dir).join("identity.key");
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_local_peer_id(state: State<'_, ClientState>) -> Result<String, String> {
    let client_guard = state.lock().map_err(|e| e.to_string())?;
    let client = client_guard
        .as_ref()
        .ok_or_else(|| "Client not initialized".to_string())?;

    // local_peer_id() is synchronous
    client.local_peer_id().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn listen_on(
    state: State<'_, ClientState>,
    multiaddr: String,
) -> Result<(), String> {
    tracing::info!("Listening on: {}", multiaddr);

    // Clone Arc to avoid holding MutexGuard across await
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    // listen_on() is async and takes owned String
    client.listen_on(multiaddr).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect_to_peer(
    state: State<'_, ClientState>,
    peer_id: String,
    multiaddr: String,
) -> Result<(), String> {
    tracing::info!("Connecting to peer {} at {}", peer_id, multiaddr);

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .connect_to_peer(peer_id, multiaddr)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_text_message(
    state: State<'_, ClientState>,
    to_peer_id: String,
    content: String,
) -> Result<String, String> {
    tracing::info!("Sending message to peer: {}", to_peer_id);

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .send_text_message(to_peer_id, content)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_conversation_messages(
    state: State<'_, ClientState>,
    peer_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<serde_json::Value>, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    // get_conversation_messages() is synchronous
    let messages = client
        .get_conversation_messages(peer_id, limit, offset)
        .map_err(|e| e.to_string())?;

    // Convert messages to JSON
    let json_messages: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.message_id,
                "sender_peer_id": m.sender_peer_id,
                "recipient_peer_id": m.recipient_peer_id,
                "content": m.content_plaintext,
                "message_type": m.message_type,
                "created_at": m.created_at,
                "status": format!("{:?}", m.status),
            })
        })
        .collect();

    Ok(json_messages)
}

#[tauri::command]
pub async fn get_conversation_media(
    state: State<'_, ClientState>,
    conversation_id: String,
    media_type: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<serde_json::Value>, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    let media_type = media_type.map(|mt| match mt.as_str() {
        "image" => FfiMediaType::Image,
        "video" => FfiMediaType::Video,
        "audio" => FfiMediaType::Audio,
        "document" => FfiMediaType::Document,
        "voice_message" => FfiMediaType::VoiceMessage,
        _ => FfiMediaType::Document,
    });

    let media = client
        .get_conversation_media(conversation_id, media_type, limit)
        .map_err(|e| e.to_string())?;

    let json_media: Vec<serde_json::Value> = media
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "media_hash": m.media_hash,
                "message_id": m.message_id,
                "media_type": format!("{:?}", m.media_type).to_lowercase(),
                "file_name": m.file_name,
                "file_size": m.file_size,
                "mime_type": m.mime_type,
                "local_path": m.local_path,
                "thumbnail_path": m.thumbnail_path,
                "width": m.width,
                "height": m.height,
                "duration_seconds": m.duration_seconds,
            })
        })
        .collect();

    Ok(json_media)
}

#[tauri::command]
pub async fn download_media(
    state: State<'_, ClientState>,
    media_hash: String,
) -> Result<String, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    let bytes = client
        .download_media(media_hash)
        .await
        .map_err(|e| e.to_string())?;

    Ok(general_purpose::STANDARD.encode(bytes))
}

#[tauri::command]
pub async fn list_conversations(
    state: State<'_, ClientState>,
) -> Result<Vec<serde_json::Value>, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    // list_conversations() is synchronous
    let conversations = client.list_conversations().map_err(|e| e.to_string())?;

    // Convert conversations to JSON
    let json_conversations: Vec<serde_json::Value> = conversations
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "peer_id": c.peer_id,
                "display_name": c.display_name,
                "last_message_id": c.last_message_id,
                "last_message_at": c.last_message_at,
                "unread_count": c.unread_count,
            })
        })
        .collect();

    Ok(json_conversations)
}

#[tauri::command]
pub async fn search_messages(
    state: State<'_, ClientState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<serde_json::Value>, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    // search_messages() is synchronous
    let messages = client
        .search_messages(query, limit)
        .map_err(|e| e.to_string())?;

    // Convert messages to JSON
    let json_messages: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.message_id,
                "sender_peer_id": m.sender_peer_id,
                "recipient_peer_id": m.recipient_peer_id,
                "content": m.content_plaintext,
                "created_at": m.created_at,
                "status": format!("{:?}", m.status),
            })
        })
        .collect();

    Ok(json_messages)
}

#[tauri::command]
pub async fn mark_conversation_read(
    state: State<'_, ClientState>,
    peer_id: String,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    // mark_conversation_read() is synchronous
    client
        .mark_conversation_read(peer_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_connected_peers_count(state: State<'_, ClientState>) -> Result<u32, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    // connected_peers_count() is async and returns u32
    client.connected_peers_count().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_listening_addresses(state: State<'_, ClientState>) -> Result<Vec<String>, String> {
    tracing::info!("🔵 get_listening_addresses CALLED");

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    // listening_addresses() is async and returns Vec<String>
    let addresses = client.listening_addresses().await.map_err(|e| e.to_string())?;
    tracing::info!("📍 Returning {} listening addresses: {:?}", addresses.len(), addresses);
    Ok(addresses)
}

#[tauri::command]
pub async fn bootstrap(state: State<'_, ClientState>) -> Result<(), String> {
    tracing::info!("Bootstrapping DHT...");

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client.bootstrap().await.map_err(|e| e.to_string())
}

// ============================================================================
// Group Commands (FASE 15)
// ============================================================================

#[tauri::command]
pub async fn create_group(
    state: State<'_, ClientState>,
    name: String,
    description: Option<String>,
) -> Result<serde_json::Value, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    let group = client
        .create_group(name, description)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "id": group.id,
        "name": group.name,
        "description": group.description,
        "member_count": group.member_count,
        "is_admin": group.is_admin,
        "created_at": group.created_at,
    }))
}

#[tauri::command]
pub async fn join_group(
    state: State<'_, ClientState>,
    group_id: String,
    group_name: String,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .join_group(group_id, group_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn leave_group(
    state: State<'_, ClientState>,
    group_id: String,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client.leave_group(group_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_group_member(
    state: State<'_, ClientState>,
    group_id: String,
    peer_id: String,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .add_group_member(group_id, peer_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_group_member(
    state: State<'_, ClientState>,
    group_id: String,
    peer_id: String,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .remove_group_member(group_id, peer_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_groups(state: State<'_, ClientState>) -> Result<Vec<serde_json::Value>, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    let groups = client.get_groups().await.map_err(|e| e.to_string())?;
    let json_groups = groups
        .into_iter()
        .map(|group| {
            serde_json::json!({
                "id": group.id,
                "name": group.name,
                "description": group.description,
                "member_count": group.member_count,
                "is_admin": group.is_admin,
                "created_at": group.created_at,
            })
        })
        .collect();

    Ok(json_groups)
}

#[tauri::command]
pub async fn get_group_messages(
    state: State<'_, ClientState>,
    group_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<serde_json::Value>, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    let messages = client
        .get_group_messages(group_id, limit, offset)
        .map_err(|e| e.to_string())?;

    let json_messages = messages
        .into_iter()
        .map(|msg| {
            serde_json::json!({
                "message_id": msg.message_id,
                "sender_peer_id": msg.sender_peer_id,
                "content_plaintext": msg.content_plaintext,
                "created_at": msg.created_at,
            })
        })
        .collect();

    Ok(json_messages)
}

#[tauri::command]
pub async fn send_group_message(
    state: State<'_, ClientState>,
    group_id: String,
    content: String,
) -> Result<String, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .send_group_message(group_id, content)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn show_notification(
    app: tauri::AppHandle,
    title: String,
    body: String,
) -> Result<(), String> {
    tracing::info!("Showing notification: {} - {}", title, body);

    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// VoIP Commands (FASE 12)
// ============================================================================

#[tauri::command]
pub async fn start_call(
    state: State<'_, ClientState>,
    to_peer_id: String,
) -> Result<String, String> {
    tracing::info!("Starting call to peer: {}", to_peer_id);

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client.start_call(to_peer_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn accept_call(
    state: State<'_, ClientState>,
    call_id: String,
) -> Result<(), String> {
    tracing::info!("Accepting call: {}", call_id);

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client.accept_call(call_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reject_call(
    state: State<'_, ClientState>,
    call_id: String,
    reason: Option<String>,
) -> Result<(), String> {
    tracing::info!("Rejecting call: {} (reason: {:?})", call_id, reason);

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .reject_call(call_id, reason)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hangup_call(
    state: State<'_, ClientState>,
    call_id: String,
) -> Result<(), String> {
    tracing::info!("Hanging up call: {}", call_id);

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client.hangup_call(call_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_mute(
    state: State<'_, ClientState>,
    call_id: String,
) -> Result<(), String> {
    tracing::info!("Toggling mute for call: {}", call_id);

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client.toggle_mute(call_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_speakerphone(
    state: State<'_, ClientState>,
    call_id: String,
) -> Result<(), String> {
    tracing::info!("Toggling speakerphone for call: {}", call_id);

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .toggle_speakerphone(call_id)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// VoIP Video Commands (FASE 14)
// ============================================================================

#[tauri::command]
pub async fn enable_video(
    state: State<'_, ClientState>,
    call_id: String,
    codec: String,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    let codec = match codec.to_lowercase().as_str() {
        "vp8" => FfiVideoCodec::VP8,
        "vp9" => FfiVideoCodec::VP9,
        _ => FfiVideoCodec::H264,
    };

    client.enable_video(call_id, codec).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disable_video(
    state: State<'_, ClientState>,
    call_id: String,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client.disable_video(call_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn register_video_frame_callback(
    state: State<'_, ClientState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .register_video_frame_callback(Box::new(VideoFrameEmitter {
            app,
            #[cfg(target_os = "macos")]
            decoder: std::sync::Mutex::new(MacVideoDecoder::new()),
        }))
        .map_err(|e| e.to_string())
}
