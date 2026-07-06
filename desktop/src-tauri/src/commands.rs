use zaplivre_core::ffi::ZapLivreClient;
use zaplivre_core::{
    FfiCallEventCallback, FfiMessageEventCallback, FfiVideoCodec, FfiVideoFrameCallback,
    FfiVoipEventCallback,
};
use std::sync::{Arc, Mutex};
use tauri::State;
use base64::{engine::general_purpose, Engine as _};
use zaplivre_core::FfiMediaType;
use tauri_plugin_notification::NotificationExt;
use tauri::Emitter;
use crate::identity_store;
#[cfg(target_os = "macos")]
use crate::macos_video::MacVideoDecoder;

// Global client state - use Arc to allow cloning the handle
type ClientState = Arc<Mutex<Option<Arc<ZapLivreClient>>>>;

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

/// EVT-03: eventos de mensagem do core -> frontend (substitui o polling de 2-5s)
struct MessageEventEmitter {
    app: tauri::AppHandle,
}

impl FfiMessageEventCallback for MessageEventEmitter {
    fn on_message_received(&self, message_id: String, from_peer_id: String) {
        let _ = self.app.emit(
            "message:received",
            serde_json::json!({ "message_id": message_id, "from_peer_id": from_peer_id }),
        );
    }

    fn on_message_status_changed(
        &self,
        message_id: String,
        status: zaplivre_core::ffi::MessageStatus,
        peer_id: Option<String>,
    ) {
        let _ = self.app.emit(
            "message:status",
            serde_json::json!({
                "message_id": message_id,
                "status": format!("{:?}", status),
                "peer_id": peer_id,
            }),
        );
    }

    fn on_typing(&self, peer_id: String, is_typing: bool) {
        let _ = self.app.emit(
            "message:typing",
            serde_json::json!({ "peer_id": peer_id, "is_typing": is_typing }),
        );
    }
}

/// Emite eventos de ciclo de vida de chamada para o frontend - sem isso o
/// callee nunca fica sabendo de uma chamada recebida.
struct CallEventEmitter {
    app: tauri::AppHandle,
}

impl FfiCallEventCallback for CallEventEmitter {
    fn on_incoming_call(&self, call_id: String, from_peer_id: String) {
        tracing::info!("📞 Incoming call {} from {}", call_id, from_peer_id);
        let _ = self.app.emit(
            "voip:incoming_call",
            serde_json::json!({ "call_id": call_id, "from_peer_id": from_peer_id }),
        );
    }

    fn on_call_state_changed(&self, call_id: String, state: zaplivre_core::ffi::FfiCallState) {
        tracing::info!("📞 Call {} state: {:?}", call_id, state);
        let _ = self.app.emit(
            "voip:call_state",
            serde_json::json!({ "call_id": call_id, "state": format!("{:?}", state) }),
        );
    }

    fn on_call_ended(&self, call_id: String, reason: zaplivre_core::ffi::FfiCallEndReason) {
        tracing::info!("📞 Call {} ended: {:?}", call_id, reason);
        let _ = self.app.emit(
            "voip:call_ended",
            serde_json::json!({ "call_id": call_id, "reason": format!("{:?}", reason) }),
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
        std::env::set_var("ZAPLIVRE_IDENTITY_B64", b64);
    } else {
        std::env::remove_var("ZAPLIVRE_IDENTITY_B64");
    }

    // ZapLivreClient::new() is synchronous, not async
    tracing::info!("🔵 Creating ZapLivreClient...");
    let client = Arc::new(ZapLivreClient::new(data_dir.clone()).map_err(|e| {
        tracing::error!("❌ Failed to create ZapLivreClient: {}", e);
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
        if let Err(err) =
            client.register_voip_event_callback(Box::new(VoipEventLogger { app: app.clone() }))
        {
            tracing::warn!("Failed to register VoIP event callback: {}", err);
        }
        if let Err(err) =
            client.register_call_event_callback(Box::new(CallEventEmitter { app: app.clone() }))
        {
            tracing::warn!("Failed to register call event callback: {}", err);
        }
        if let Err(err) =
            client.register_message_event_callback(Box::new(MessageEventEmitter { app }))
        {
            tracing::warn!("Failed to register message event callback: {}", err);
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

    // Convert conversations to JSON (UX-11: com preview da última mensagem)
    let json_conversations: Vec<serde_json::Value> = conversations
        .iter()
        .map(|c| {
            let last_message_preview = c
                .peer_id
                .as_ref()
                .filter(|_| c.last_message_id.is_some())
                .and_then(|peer| {
                    client
                        .get_conversation_messages(peer.clone(), Some(1), None)
                        .ok()
                        .and_then(|msgs| msgs.into_iter().next())
                        .and_then(|m| m.content_plaintext)
                });
            serde_json::json!({
                "id": c.id,
                "peer_id": c.peer_id,
                "display_name": c.display_name,
                "last_message_id": c.last_message_id,
                "last_message_at": c.last_message_at,
                "unread_count": c.unread_count,
                "last_message_preview": last_message_preview,
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
pub async fn get_group_sender_key_seed(
    state: State<'_, ClientState>,
    group_id: String,
) -> Result<Vec<u8>, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .get_group_sender_key_seed(group_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_group_sender_key(
    state: State<'_, ClientState>,
    group_id: String,
    sender_peer_id: String,
    sender_key_seed: Vec<u8>,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .add_group_sender_key(group_id, sender_peer_id, sender_key_seed)
        .await
        .map_err(|e| e.to_string())
}

/// UX-02: reações e forward
#[tauri::command]
pub async fn add_reaction(
    state: State<'_, ClientState>,
    message_id: String,
    emoji: String,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };
    client.add_reaction(message_id, emoji).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_reaction(
    state: State<'_, ClientState>,
    message_id: String,
    emoji: String,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };
    client.remove_reaction(message_id, emoji).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_message_reactions(
    state: State<'_, ClientState>,
    message_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };
    let reactions = client
        .get_message_reactions(message_id)
        .map_err(|e| e.to_string())?;
    Ok(reactions
        .iter()
        .map(|r| {
            serde_json::json!({
                "reaction_id": r.reaction_id,
                "message_id": r.message_id,
                "peer_id": r.peer_id,
                "emoji": r.emoji,
                "created_at": r.created_at,
            })
        })
        .collect())
}

#[tauri::command]
pub async fn forward_message(
    state: State<'_, ClientState>,
    message_id: String,
    to_peer_id: String,
) -> Result<String, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };
    client
        .forward_message(message_id, to_peer_id)
        .await
        .map_err(|e| e.to_string())
}

/// DSK-09: exporta o backup Base64 da identidade guardada no keychain
#[tauri::command]
pub fn export_identity_backup() -> Result<String, String> {
    identity_store::load_identity_b64()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Nenhuma identidade encontrada no keychain".to_string())
}

/// DSK-09: restaura um backup de identidade. Salva no keychain, descarta o
/// banco local (pertence a identidade anterior) e REINICIA o app - o
/// init_client do proximo boot usa a identidade importada.
#[tauri::command]
pub fn import_identity_backup(
    app: tauri::AppHandle,
    backup: String,
    data_dir: String,
) -> Result<(), String> {
    use base64::Engine as _;

    let trimmed = backup.trim();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .map_err(|_| "Backup invalido: nao e Base64".to_string())?;
    if decoded.len() < 32 {
        return Err("Backup invalido: conteudo curto demais".to_string());
    }

    identity_store::save_identity_b64(trimmed).map_err(|e| e.to_string())?;

    let db_path = std::path::Path::new(&data_dir).join("zaplivre.db");
    if db_path.exists() {
        std::fs::remove_file(&db_path)
            .map_err(|e| format!("Falha ao limpar banco local: {}", e))?;
    }

    tracing::info!("🔑 Identity backup imported - restarting app");
    app.restart();
}

/// UX-02: envia um arquivo do disco - imagens vão pelo pipeline de imagem
/// (compressão), o resto como documento
#[tauri::command]
pub async fn send_file_message(
    state: State<'_, ClientState>,
    to_peer_id: String,
    file_path: String,
) -> Result<String, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    let path = std::path::Path::new(&file_path);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("arquivo")
        .to_string();
    let data = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

    const MAX_FILE_BYTES: usize = 50 * 1024 * 1024;
    if data.len() > MAX_FILE_BYTES {
        return Err("Arquivo maior que 50MB".to_string());
    }

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" => client
            .send_image_message(to_peer_id, data, file_name, 80)
            .await
            .map_err(|e| e.to_string()),
        _ => {
            let mime = match extension.as_str() {
                "pdf" => "application/pdf",
                "txt" => "text/plain",
                "mp4" => "video/mp4",
                "mp3" => "audio/mpeg",
                "zip" => "application/zip",
                _ => "application/octet-stream",
            };
            client
                .send_document_message(to_peer_id, data, file_name, mime.to_string())
                .await
                .map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_group_members(
    state: State<'_, ClientState>,
    group_id: String,
) -> Result<Vec<String>, String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client.get_group_members(group_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_group(
    state: State<'_, ClientState>,
    group_id: String,
    name: Option<String>,
    description: Option<String>,
) -> Result<(), String> {
    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client
        .update_group(group_id, name, description)
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

#[tauri::command]
pub async fn switch_camera(
    state: State<'_, ClientState>,
    call_id: String,
) -> Result<(), String> {
    tracing::info!("Switching camera for call: {}", call_id);

    let client = {
        let client_guard = state.lock().map_err(|e| e.to_string())?;
        client_guard
            .as_ref()
            .ok_or_else(|| "Client not initialized".to_string())?
            .clone()
    };

    client.switch_camera(call_id).await.map_err(|e| e.to_string())
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
