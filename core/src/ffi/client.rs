//! FFI Client implementation using UniFFI (channel-based architecture)
//!
//! This module provides a thread-safe FFI wrapper around the MePassa Client.
//! Since libp2p::Swarm is !Sync by design, we use a channel-based architecture
//! where the Client runs in a dedicated tokio task and receives commands via channels.

use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::{mpsc, oneshot};

use super::types::{
    self as types, FfiConversation, FfiGroup, FfiMessage, FfiReaction, MePassaFfiError,
};
use crate::api::{Client, ClientBuilder};

use std::thread;
use tokio::task::LocalSet;

// Global tokio runtime with LocalSet for !Send futures
static RUNTIME: OnceLock<std::sync::Arc<tokio::runtime::Runtime>> = OnceLock::new();

fn runtime() -> &'static std::sync::Arc<tokio::runtime::Runtime> {
    RUNTIME.get_or_init(|| {
        std::sync::Arc::new(tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"))
    })
}

/// Execute a future, using the current runtime if available (for Desktop/Tauri),
/// otherwise using the global runtime (for Mobile FFI)
fn execute_future<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    // Try to use current runtime handle (for Desktop/Tauri)
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // We're already in a runtime context, spawn and block on a new task
        std::thread::spawn(move || {
            handle.block_on(future)
        })
        .join()
        .expect("Thread join failed")
    } else {
        // No runtime, use the global one (for Mobile FFI)
        runtime().block_on(future)
    }
}

// Global client handle (initialized once)
static CLIENT_HANDLE: OnceLock<ClientHandle> = OnceLock::new();

/// Handle to communicate with the Client running in a dedicated task
struct ClientHandle {
    sender: mpsc::UnboundedSender<ClientCommand>,
}

/// Commands that can be sent to the Client
enum ClientCommand {
    LocalPeerId {
        response: oneshot::Sender<String>,
    },
    GetPrekeyBundleJson {
        response: oneshot::Sender<Result<String, MePassaFfiError>>,
    },
    SetContactPrekeyBundle {
        peer_id: String,
        prekey_bundle_json: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    ListenOn {
        multiaddr: libp2p::Multiaddr,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    ConnectToPeer {
        peer_id: libp2p::PeerId,
        multiaddr: libp2p::Multiaddr,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    SendTextMessage {
        to: libp2p::PeerId,
        content: String,
        response: oneshot::Sender<Result<String, MePassaFfiError>>,
    },
    GetConversationMessages {
        peer_id: String,
        limit: Option<usize>,
        offset: Option<usize>,
        response: oneshot::Sender<Result<Vec<FfiMessage>, MePassaFfiError>>,
    },
    ListConversations {
        response: oneshot::Sender<Result<Vec<FfiConversation>, MePassaFfiError>>,
    },
    SearchMessages {
        query: String,
        limit: Option<usize>,
        response: oneshot::Sender<Result<Vec<FfiMessage>, MePassaFfiError>>,
    },
    MarkConversationRead {
        peer_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    ConnectedPeersCount {
        response: oneshot::Sender<Result<u32, MePassaFfiError>>,
    },
    ListeningAddresses {
        response: oneshot::Sender<Result<Vec<String>, MePassaFfiError>>,
    },
    Bootstrap {
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    // VoIP commands
    #[cfg(feature = "voip")]
    StartCall {
        to_peer_id: String,
        response: oneshot::Sender<Result<String, MePassaFfiError>>,
    },
    #[cfg(feature = "voip")]
    AcceptCall {
        call_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    #[cfg(feature = "voip")]
    RejectCall {
        call_id: String,
        reason: Option<String>,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    #[cfg(feature = "voip")]
    HangupCall {
        call_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    #[cfg(feature = "voip")]
    ToggleMute {
        call_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    #[cfg(feature = "voip")]
    ToggleSpeakerphone {
        call_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    // Video commands (FASE 14)
    #[cfg(any(feature = "voip", feature = "video"))]
    EnableVideo {
        call_id: String,
        codec: types::FfiVideoCodec,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    #[cfg(any(feature = "voip", feature = "video"))]
    DisableVideo {
        call_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    #[cfg(any(feature = "voip", feature = "video"))]
    SendVideoFrame {
        call_id: String,
        frame_data: Vec<u8>,
        width: u32,
        height: u32,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    #[cfg(any(feature = "voip", feature = "video"))]
    SwitchCamera {
        call_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    #[cfg(any(feature = "voip", feature = "video"))]
    RegisterVideoFrameCallback {
        callback: Box<dyn crate::FfiVideoFrameCallback>,
    },
    RegisterVoipEventCallback {
        callback: Box<dyn crate::FfiVoipEventCallback>,
    },
    // Group commands (FASE 15)
    CreateGroup {
        name: String,
        description: Option<String>,
        response: oneshot::Sender<Result<FfiGroup, MePassaFfiError>>,
    },
    JoinGroup {
        group_id: String,
        group_name: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    LeaveGroup {
        group_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    AddGroupMember {
        group_id: String,
        peer_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    RemoveGroupMember {
        group_id: String,
        peer_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    GetGroups {
        response: oneshot::Sender<Result<Vec<FfiGroup>, MePassaFfiError>>,
    },
    GetGroupMessages {
        group_id: String,
        limit: Option<usize>,
        offset: Option<usize>,
        response: oneshot::Sender<Result<Vec<FfiMessage>, MePassaFfiError>>,
    },
    SendGroupMessage {
        group_id: String,
        content: String,
        response: oneshot::Sender<Result<String, MePassaFfiError>>,
    },
    // Media commands (FASE 16 - Mídia & Polimento)
    SendImageMessage {
        to_peer_id: String,
        image_data: Vec<u8>,
        file_name: String,
        quality: u8,
        response: oneshot::Sender<Result<String, MePassaFfiError>>,
    },
    SendVoiceMessage {
        to_peer_id: String,
        audio_data: Vec<u8>,
        file_name: String,
        duration_seconds: i32,
        response: oneshot::Sender<Result<String, MePassaFfiError>>,
    },
    SendDocumentMessage {
        to_peer_id: String,
        file_data: Vec<u8>,
        file_name: String,
        mime_type: String,
        response: oneshot::Sender<Result<String, MePassaFfiError>>,
    },
    SendVideoMessage {
        to_peer_id: String,
        video_data: Vec<u8>,
        file_name: String,
        width: Option<i32>,
        height: Option<i32>,
        duration_seconds: i32,
        thumbnail_data: Option<Vec<u8>>,
        response: oneshot::Sender<Result<String, MePassaFfiError>>,
    },
    DownloadMedia {
        media_hash: String,
        response: oneshot::Sender<Result<Vec<u8>, MePassaFfiError>>,
    },
    GetConversationMedia {
        conversation_id: String,
        media_type: Option<types::FfiMediaType>,
        limit: Option<u32>,
        response: oneshot::Sender<Result<Vec<types::FfiMedia>, MePassaFfiError>>,
    },
    // Message action commands (FASE 16 - Forward & Delete)
    DeleteMessage {
        message_id: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    ForwardMessage {
        message_id: String,
        to_peer_id: String,
        response: oneshot::Sender<Result<String, MePassaFfiError>>,
    },
    // Reaction commands (FASE 16 - TRACK 8)
    AddReaction {
        message_id: String,
        emoji: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    RemoveReaction {
        message_id: String,
        emoji: String,
        response: oneshot::Sender<Result<(), MePassaFfiError>>,
    },
    GetMessageReactions {
        message_id: String,
        response: oneshot::Sender<Result<Vec<FfiReaction>, MePassaFfiError>>,
    },
}

/// Run the client task (processes commands) - takes owned Client
async fn run_client_task(
    receiver: mpsc::UnboundedReceiver<ClientCommand>,
    client: Client,
) {
    run_client_task_arc(receiver, std::sync::Arc::new(client)).await
}

/// Run the client task with Arc<Client> (processes commands)
async fn run_client_task_arc(
    mut receiver: mpsc::UnboundedReceiver<ClientCommand>,
    client: std::sync::Arc<Client>,
) {
    while let Some(cmd) = receiver.recv().await {
        match cmd {
            ClientCommand::LocalPeerId { response } => {
                let _ = response.send(client.local_peer_id().to_string());
            }
            ClientCommand::GetPrekeyBundleJson { response } => {
                let result = client
                    .get_prekey_bundle_json()
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::SetContactPrekeyBundle {
                peer_id,
                prekey_bundle_json,
                response,
            } => {
                let result = client
                    .set_contact_prekey_bundle(peer_id, prekey_bundle_json)
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::ListenOn {
                multiaddr,
                response,
            } => {
                let result = client
                    .listen_on(multiaddr)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::ConnectToPeer {
                peer_id,
                multiaddr,
                response,
            } => {
                let result = client
                    .connect_to_peer(peer_id, multiaddr)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::SendTextMessage { to, content, response } => {
                let result = client
                    .send_text_message(to, content)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::GetConversationMessages {
                peer_id,
                limit,
                offset,
                response,
            } => {
                let result = client
                    .get_conversation_messages(&peer_id, limit, offset)
                    .map(|messages| messages.into_iter().map(FfiMessage::from).collect())
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::ListConversations { response } => {
                let result = client
                    .list_conversations()
                    .map(|convs| convs.into_iter().map(FfiConversation::from).collect())
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::SearchMessages {
                query,
                limit,
                response,
            } => {
                let result = client
                    .search_messages(&query, limit)
                    .map(|messages| messages.into_iter().map(FfiMessage::from).collect())
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::MarkConversationRead { peer_id, response } => {
                let result = client
                    .mark_conversation_read(&peer_id)
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::ConnectedPeersCount { response } => {
                let result = Ok(client.connected_peers_count().await as u32);
                let _ = response.send(result);
            }
            ClientCommand::ListeningAddresses { response } => {
                let result = Ok(client.listening_addresses().await);
                let _ = response.send(result);
            }
            ClientCommand::Bootstrap { response } => {
                let result = client.bootstrap().await.map_err(|e| e.into());
                let _ = response.send(result);
            }
            #[cfg(feature = "voip")]
            ClientCommand::StartCall {
                to_peer_id,
                response,
            } => {
                let result = client.start_call(to_peer_id).await.map_err(|e| e.into());
                let _ = response.send(result);
            }
            #[cfg(feature = "voip")]
            ClientCommand::AcceptCall { call_id, response } => {
                let result = client.accept_call(call_id).await.map_err(|e| e.into());
                let _ = response.send(result);
            }
            #[cfg(feature = "voip")]
            ClientCommand::RejectCall {
                call_id,
                reason,
                response,
            } => {
                let result = client.reject_call(call_id, reason).await.map_err(|e| e.into());
                let _ = response.send(result);
            }
            #[cfg(feature = "voip")]
            ClientCommand::HangupCall { call_id, response } => {
                let result = client.hangup_call(call_id).await.map_err(|e| e.into());
                let _ = response.send(result);
            }
            #[cfg(feature = "voip")]
            ClientCommand::ToggleMute { call_id, response } => {
                let result = client.toggle_mute(call_id).await.map_err(|e| e.into());
                let _ = response.send(result);
            }
            #[cfg(feature = "voip")]
            ClientCommand::ToggleSpeakerphone { call_id, response } => {
                let result = client
                    .toggle_speakerphone(call_id)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            // Video command handlers (FASE 14)
            #[cfg(any(feature = "voip", feature = "video"))]
            ClientCommand::EnableVideo { call_id, codec, response } => {
                let result = client
                    .enable_video(call_id, codec.into())
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            #[cfg(any(feature = "voip", feature = "video"))]
            ClientCommand::DisableVideo { call_id, response } => {
                let result = client
                    .disable_video(call_id)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            #[cfg(any(feature = "voip", feature = "video"))]
            ClientCommand::SendVideoFrame {
                call_id,
                frame_data,
                width,
                height,
                response,
            } => {
                let result = client
                    .send_video_frame(call_id, &frame_data, width, height)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            #[cfg(any(feature = "voip", feature = "video"))]
            ClientCommand::SwitchCamera { call_id, response } => {
                let result = client
                    .switch_camera(call_id)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            #[cfg(any(feature = "voip", feature = "video"))]
            ClientCommand::RegisterVideoFrameCallback { callback } => {
                // Register the callback with VoIPIntegration via Client
                client.register_video_frame_callback(callback).await;
            }
            #[cfg(any(feature = "voip", feature = "video"))]
            ClientCommand::RegisterVoipEventCallback { callback } => {
                client.register_voip_event_callback(callback).await;
            }
            // Group command handlers (FASE 15)
            ClientCommand::CreateGroup {
                name,
                description,
                response,
            } => {
                let result = client
                    .create_group(name, description)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::JoinGroup {
                group_id,
                group_name,
                response,
            } => {
                let result = client
                    .join_group(group_id, group_name)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::LeaveGroup { group_id, response } => {
                let result = client
                    .leave_group(group_id)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::AddGroupMember {
                group_id,
                peer_id,
                response,
            } => {
                let result = client
                    .add_group_member(group_id, peer_id)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::RemoveGroupMember {
                group_id,
                peer_id,
                response,
            } => {
                let result = client
                    .remove_group_member(group_id, peer_id)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::GetGroups { response } => {
                let result = client
                    .get_groups()
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::GetGroupMessages {
                group_id,
                limit,
                offset,
                response,
            } => {
                let result = client
                    .get_group_messages(group_id, limit, offset)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::SendGroupMessage {
                group_id,
                content,
                response,
            } => {
                let result = client
                    .send_group_message(group_id, content)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            // Media command handlers (FASE 16)
            ClientCommand::SendImageMessage {
                to_peer_id,
                image_data,
                file_name,
                quality,
                response,
            } => {
                let to: libp2p::PeerId = match to_peer_id.parse() {
                    Ok(peer_id) => peer_id,
                    Err(_) => {
                        let _ = response.send(Err(MePassaFfiError::Network {
                            details: "Invalid peer ID".to_string(),
                        }));
                        continue;
                    }
                };

                let result = client
                    .send_image_message(to, &image_data, file_name, quality)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::SendVoiceMessage {
                to_peer_id,
                audio_data,
                file_name,
                duration_seconds,
                response,
            } => {
                let to: libp2p::PeerId = match to_peer_id.parse() {
                    Ok(peer_id) => peer_id,
                    Err(_) => {
                        let _ = response.send(Err(MePassaFfiError::Network {
                            details: "Invalid peer ID".to_string(),
                        }));
                        continue;
                    }
                };

                let result = client
                    .send_voice_message(to, &audio_data, file_name, duration_seconds)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::SendDocumentMessage {
                to_peer_id,
                file_data,
                file_name,
                mime_type,
                response,
            } => {
                let to: libp2p::PeerId = match to_peer_id.parse() {
                    Ok(peer_id) => peer_id,
                    Err(_) => {
                        let _ = response.send(Err(MePassaFfiError::Network {
                            details: "Invalid peer ID".to_string(),
                        }));
                        continue;
                    }
                };

                let result = client
                    .send_document_message(to, &file_data, file_name, mime_type)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::SendVideoMessage {
                to_peer_id,
                video_data,
                file_name,
                width,
                height,
                duration_seconds,
                thumbnail_data,
                response,
            } => {
                let to: libp2p::PeerId = match to_peer_id.parse() {
                    Ok(peer_id) => peer_id,
                    Err(_) => {
                        let _ = response.send(Err(MePassaFfiError::Network {
                            details: "Invalid peer ID".to_string(),
                        }));
                        continue;
                    }
                };

                let result = client
                    .send_video_message(
                        to,
                        &video_data,
                        file_name,
                        width,
                        height,
                        duration_seconds,
                        thumbnail_data.as_deref(),
                    )
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::DownloadMedia {
                media_hash,
                response,
            } => {
                let result = client
                    .download_media(&media_hash)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::GetConversationMedia {
                conversation_id,
                media_type,
                limit,
                response,
            } => {
                let internal_media_type = media_type.map(|mt| mt.into());
                let result = client
                    .get_conversation_media(
                        &conversation_id,
                        internal_media_type,
                        limit.map(|l| l as usize),
                    )
                    .map(|media_vec| {
                        media_vec.into_iter().map(|m| m.into()).collect()
                    })
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            // Message action handlers (FASE 16 - Forward & Delete)
            ClientCommand::DeleteMessage {
                message_id,
                response,
            } => {
                let result = client
                    .delete_message(&message_id)
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::ForwardMessage {
                message_id,
                to_peer_id,
                response,
            } => {
                let to: libp2p::PeerId = match to_peer_id.parse() {
                    Ok(peer_id) => peer_id,
                    Err(_) => {
                        let _ = response.send(Err(MePassaFfiError::Network {
                            details: "Invalid peer ID".to_string(),
                        }));
                        continue;
                    }
                };

                let result = client
                    .forward_message(&message_id, to)
                    .await
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            // Reaction handlers (FASE 16 - TRACK 8)
            ClientCommand::AddReaction {
                message_id,
                emoji,
                response,
            } => {
                let result = client
                    .add_reaction(&message_id, &emoji)
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::RemoveReaction {
                message_id,
                emoji,
                response,
            } => {
                let result = client
                    .remove_reaction(&message_id, &emoji)
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
            ClientCommand::GetMessageReactions {
                message_id,
                response,
            } => {
                let result = client
                    .get_message_reactions(&message_id)
                    .map(|reactions| {
                        reactions.into_iter().map(|r| r.into()).collect()
                    })
                    .map_err(|e| e.into());
                let _ = response.send(result);
            }
        }
    }
}

/// MePassa client (exposed via interface pattern)
pub struct MePassaClient {
    data_dir: String,
}

impl MePassaClient {
    /// Create new client and initialize the global client task
    pub fn new(data_dir: String) -> Result<Self, MePassaFfiError> {
        // Initialize the client task if not already done
        CLIENT_HANDLE.get_or_init(|| {
            let (sender, receiver) = mpsc::unbounded_channel();
            let data_dir_clone = data_dir.clone();

            // Spawn a dedicated thread with LocalSet for !Send Client
            thread::spawn(move || {
                let rt = runtime();
                let local = LocalSet::new();

                // Build and run the client task
                local.block_on(rt, async move {
                    let mut builder = ClientBuilder::new()
                        .data_dir(PathBuf::from(&data_dir_clone));

                    if let Ok(url) = std::env::var("MESSAGE_STORE_URL") {
                        if !url.trim().is_empty() {
                            builder = builder.message_store_url(url);
                        }
                    }

                    // Bootstrap peers (produção): substitua pelos seus bootstraps públicos.
                    // Exemplo:
                    // let custom_bootstrap_peers = vec![
                    //     ("/ip4/<PUBLIC_IP>/tcp/4001", "12D3KooW..."),
                    //     ("/ip4/<PUBLIC_IP>/tcp/4002", "12D3KooW..."),
                    // ];
                    let custom_bootstrap_peers = vec![
                        ("/dns4/dht1.associahub.com.br/tcp/4001", "12D3KooWJMY3dKygHLtkruLohCshiPENpJscD5XY33GjfcmS4DKK"),
                        ("/dns4/dht2.associahub.com.br/tcp/4002", "12D3KooWRwysfFEQL5YhFa8bNqeoY34b7Bb7mUzx617sun9GyAPP"),
                    ];

                    // Default bootstrap peers (IPFS public nodes) - fallback
                    let default_bootstrap_peers = vec![
                        ("/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN", "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN"),
                        ("/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa", "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa"),
                        ("/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb", "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb"),
                        ("/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt", "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt"),
                    ];

                    let bootstrap_peers = if custom_bootstrap_peers.is_empty() {
                        default_bootstrap_peers
                    } else {
                        custom_bootstrap_peers
                    };

                    for (addr_str, peer_id_str) in bootstrap_peers {
                        if let (Ok(addr), Ok(peer_id)) = (
                            addr_str.parse::<libp2p::Multiaddr>(),
                            peer_id_str.parse::<libp2p::PeerId>()
                        ) {
                            builder = builder.add_bootstrap_peer(peer_id, addr);
                        }
                    }

                    let client = std::sync::Arc::new(builder.build().await.expect("Failed to build client"));
                    let client_for_network = std::sync::Arc::clone(&client);

                    // Spawn network event loop task using non-blocking polling
                    // This releases the lock between iterations, allowing commands to proceed
                    let network_handle = tokio::task::spawn_local(async move {
                        tracing::info!("🌐 Starting network event loop (non-blocking)...");
                        let mut poll_count: u64 = 0;
                        loop {
                            poll_count += 1;
                            // Log every 1000 polls (~10 seconds) to confirm loop is running
                            if poll_count % 1000 == 0 {
                                tracing::debug!("🔄 Network poll #{}", poll_count);
                            }
                            // Poll for one event at a time, releasing lock between polls
                            match client_for_network.poll_network_once().await {
                                Ok(true) => {
                                    tracing::info!("📡 Network event processed (poll #{})", poll_count);
                                }
                                Ok(false) => {
                                    // No events, yield to allow other tasks
                                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                                }
                                Err(e) => {
                                    tracing::error!("Network poll error: {:?}", e);
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                }
                            }
                        }
                    });

                    // Run client command task (processes API commands)
                    // Note: We use Arc<Client> but run_client_task expects Client
                    // We need to keep client alive for the network task
                    tokio::select! {
                        _ = run_client_task_arc(receiver, client) => {
                            tracing::info!("Client task completed");
                        }
                        _ = network_handle => {
                            tracing::info!("Network event loop completed");
                        }
                    }
                });
            });

            ClientHandle { sender }
        });

        Ok(Self { data_dir })
    }

    /// Get the client handle
    fn handle(&self) -> &ClientHandle {
        CLIENT_HANDLE.get().expect("Client not initialized")
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> Result<String, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::LocalPeerId { response: tx })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        execute_future(rx).map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })
    }

    /// Export prekey bundle as JSON (for sharing)
    pub async fn get_prekey_bundle_json(&self) -> Result<String, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::GetPrekeyBundleJson { response: tx })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Store a contact's prekey bundle JSON
    pub fn set_contact_prekey_bundle(
        &self,
        peer_id: String,
        prekey_bundle_json: String,
    ) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::SetContactPrekeyBundle {
                peer_id,
                prekey_bundle_json,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        execute_future(rx).map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Start listening on an address
    pub async fn listen_on(&self, multiaddr: String) -> Result<(), MePassaFfiError> {
        let addr: libp2p::Multiaddr = multiaddr.parse().map_err(|_| MePassaFfiError::Network {
            details: "Invalid multiaddr".to_string(),
        })?;

        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::ListenOn {
                multiaddr: addr,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Connect to a peer
    pub async fn connect_to_peer(
        &self,
        peer_id: String,
        multiaddr: String,
    ) -> Result<(), MePassaFfiError> {
        let peer_id: libp2p::PeerId = peer_id.parse().map_err(|_| MePassaFfiError::Network {
            details: "Invalid peer ID".to_string(),
        })?;

        let addr: libp2p::Multiaddr = multiaddr.parse().map_err(|_| MePassaFfiError::Network {
            details: "Invalid multiaddr".to_string(),
        })?;

        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::ConnectToPeer {
                peer_id,
                multiaddr: addr,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Send a text message
    pub async fn send_text_message(
        &self,
        to_peer_id: String,
        content: String,
    ) -> Result<String, MePassaFfiError> {
        let to: libp2p::PeerId = to_peer_id.parse().map_err(|_| MePassaFfiError::Network {
            details: "Invalid peer ID".to_string(),
        })?;

        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::SendTextMessage {
                to,
                content,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Get messages for a conversation
    pub fn get_conversation_messages(
        &self,
        peer_id: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<FfiMessage>, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::GetConversationMessages {
                peer_id,
                limit: limit.map(|l| l as usize),
                offset: offset.map(|o| o as usize),
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        execute_future(rx).map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// List all conversations
    pub fn list_conversations(&self) -> Result<Vec<FfiConversation>, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::ListConversations { response: tx })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        execute_future(rx).map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Search messages
    pub fn search_messages(
        &self,
        query: String,
        limit: Option<u32>,
    ) -> Result<Vec<FfiMessage>, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::SearchMessages {
                query,
                limit: limit.map(|l| l as usize),
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        execute_future(rx).map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Mark conversation as read
    pub fn mark_conversation_read(&self, peer_id: String) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::MarkConversationRead {
                peer_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        execute_future(rx).map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Get connected peers count
    pub async fn connected_peers_count(&self) -> Result<u32, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::ConnectedPeersCount { response: tx })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Get current listening addresses
    pub async fn listening_addresses(&self) -> Result<Vec<String>, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::ListeningAddresses { response: tx })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Bootstrap DHT
    pub async fn bootstrap(&self) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::Bootstrap { response: tx })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    // ========== VoIP Methods ==========

    #[cfg(feature = "voip")]
    /// Start a voice call to a peer
    pub async fn start_call(&self, to_peer_id: String) -> Result<String, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::StartCall {
                to_peer_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    #[cfg(feature = "voip")]
    /// Accept an incoming call
    pub async fn accept_call(&self, call_id: String) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::AcceptCall {
                call_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    #[cfg(feature = "voip")]
    /// Reject an incoming call
    pub async fn reject_call(&self, call_id: String, reason: Option<String>) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::RejectCall {
                call_id,
                reason,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    #[cfg(feature = "voip")]
    /// Hang up an active call
    pub async fn hangup_call(&self, call_id: String) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::HangupCall {
                call_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    #[cfg(feature = "voip")]
    /// Toggle audio mute
    pub async fn toggle_mute(&self, call_id: String) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::ToggleMute {
                call_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    #[cfg(feature = "voip")]
    /// Toggle speakerphone
    pub async fn toggle_speakerphone(&self, call_id: String) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::ToggleSpeakerphone {
                call_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    // ========== Video Methods (FASE 14) ==========

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Enable video for an active call
    pub async fn enable_video(
        &self,
        call_id: String,
        codec: types::FfiVideoCodec,
    ) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::EnableVideo {
                call_id,
                codec,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Disable video for an active call
    pub async fn disable_video(&self, call_id: String) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::DisableVideo {
                call_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Send video frame to remote peer
    ///
    /// Frame data should be pre-encoded (H.264 NALUs or VP8/VP9 frames)
    /// Platform-specific encoding happens before calling this method
    pub async fn send_video_frame(
        &self,
        call_id: String,
        frame_data: Vec<u8>,
        width: u32,
        height: u32,
    ) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::SendVideoFrame {
                call_id,
                frame_data,
                width,
                height,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Switch camera (front/back) during video call
    ///
    /// Only applicable on mobile devices with multiple cameras.
    /// Desktop platforms may ignore this call or return an error.
    pub async fn switch_camera(&self, call_id: String) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::SwitchCamera {
                call_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Register a callback for receiving remote video frames
    ///
    /// The callback will be invoked on a background thread whenever a remote
    /// video frame is received during an active video call.
    ///
    /// # Parameters
    /// - `callback`: Implementation of FfiVideoFrameCallback trait (auto-generated by UniFFI)
    ///
    /// # Example
    /// ```ignore
    /// client.register_video_frame_callback(MyVideoFrameHandler::new())?;
    /// ```
    pub fn register_video_frame_callback(
        &self,
        callback: Box<dyn crate::FfiVideoFrameCallback>,
    ) -> Result<(), MePassaFfiError> {
        self.handle()
            .sender
            .send(ClientCommand::RegisterVideoFrameCallback {
                callback,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })
    }

    #[cfg(any(feature = "voip", feature = "video"))]
    /// Register callback for VoIP control events (mute/speaker/camera)
    pub fn register_voip_event_callback(
        &self,
        callback: Box<dyn crate::FfiVoipEventCallback>,
    ) -> Result<(), MePassaFfiError> {
        self.handle()
            .sender
            .send(ClientCommand::RegisterVoipEventCallback { callback })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })
    }

    // ========== VoIP Method Stubs (when feature is disabled) ==========

    #[cfg(not(feature = "voip"))]
    /// Start a voice call (stub - VoIP feature disabled)
    pub async fn start_call(&self, _to_peer_id: String) -> Result<String, MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP feature is not enabled. Rebuild with --features voip".to_string(),
        })
    }

    #[cfg(not(feature = "voip"))]
    /// Accept an incoming call (stub - VoIP feature disabled)
    pub async fn accept_call(&self, _call_id: String) -> Result<(), MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP feature is not enabled. Rebuild with --features voip".to_string(),
        })
    }

    #[cfg(not(feature = "voip"))]
    /// Reject an incoming call (stub - VoIP feature disabled)
    pub async fn reject_call(&self, _call_id: String, _reason: Option<String>) -> Result<(), MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP feature is not enabled. Rebuild with --features voip".to_string(),
        })
    }

    #[cfg(not(feature = "voip"))]
    /// Hang up an active call (stub - VoIP feature disabled)
    pub async fn hangup_call(&self, _call_id: String) -> Result<(), MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP feature is not enabled. Rebuild with --features voip".to_string(),
        })
    }

    #[cfg(not(feature = "voip"))]
    /// Toggle audio mute (stub - VoIP feature disabled)
    pub async fn toggle_mute(&self, _call_id: String) -> Result<(), MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP feature is not enabled. Rebuild with --features voip".to_string(),
        })
    }

    #[cfg(not(feature = "voip"))]
    /// Toggle speakerphone (stub - VoIP feature disabled)
    pub async fn toggle_speakerphone(&self, _call_id: String) -> Result<(), MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP feature is not enabled. Rebuild with --features voip".to_string(),
        })
    }

    #[cfg(not(any(feature = "voip", feature = "video")))]
    /// Enable video (stub - VoIP/video features disabled)
    pub async fn enable_video(&self, _call_id: String, _codec: types::FfiVideoCodec) -> Result<(), MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP/video features are not enabled. Rebuild with --features voip or --features video".to_string(),
        })
    }

    #[cfg(not(any(feature = "voip", feature = "video")))]
    /// Disable video (stub - VoIP/video features disabled)
    pub async fn disable_video(&self, _call_id: String) -> Result<(), MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP/video features are not enabled. Rebuild with --features voip or --features video".to_string(),
        })
    }

    #[cfg(not(any(feature = "voip", feature = "video")))]
    /// Send video frame (stub - VoIP/video features disabled)
    pub async fn send_video_frame(
        &self,
        _call_id: String,
        _frame_data: Vec<u8>,
        _width: u32,
        _height: u32,
    ) -> Result<(), MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP/video features are not enabled. Rebuild with --features voip or --features video".to_string(),
        })
    }

    #[cfg(not(any(feature = "voip", feature = "video")))]
    /// Switch camera (stub - VoIP/video features disabled)
    pub async fn switch_camera(&self, _call_id: String) -> Result<(), MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP/video features are not enabled. Rebuild with --features voip or --features video".to_string(),
        })
    }

    #[cfg(not(any(feature = "voip", feature = "video")))]
    /// Register video frame callback (stub - VoIP/video features disabled)
    pub fn register_video_frame_callback(
        &self,
        _callback: Box<dyn crate::FfiVideoFrameCallback>,
    ) -> Result<(), MePassaFfiError> {
        Err(MePassaFfiError::Other {
            details: "VoIP/video features are not enabled. Rebuild with --features voip or --features video".to_string(),
        })
    }

    // ========== Group Methods (FASE 15) ==========

    /// Create a new group
    pub async fn create_group(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<FfiGroup, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::CreateGroup {
                name,
                description,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Join an existing group (invited by admin)
    pub async fn join_group(
        &self,
        group_id: String,
        group_name: String,
    ) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::JoinGroup {
                group_id,
                group_name,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Leave a group
    pub async fn leave_group(&self, group_id: String) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::LeaveGroup {
                group_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Add a member to a group (admin only)
    pub async fn add_group_member(
        &self,
        group_id: String,
        peer_id: String,
    ) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::AddGroupMember {
                group_id,
                peer_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Remove a member from a group (admin only)
    pub async fn remove_group_member(
        &self,
        group_id: String,
        peer_id: String,
    ) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::RemoveGroupMember {
                group_id,
                peer_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Get all groups
    pub async fn get_groups(&self) -> Result<Vec<FfiGroup>, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::GetGroups { response: tx })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    pub fn get_group_messages(
        &self,
        group_id: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<FfiMessage>, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::GetGroupMessages {
                group_id,
                limit: limit.map(|v| v as usize),
                offset: offset.map(|v| v as usize),
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.blocking_recv().map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    pub async fn send_group_message(
        &self,
        group_id: String,
        content: String,
    ) -> Result<String, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::SendGroupMessage {
                group_id,
                content,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    // ═════════════════════════════════════════════════════════════════════
    // Media Methods (FASE 16 - Mídia & Polimento)
    // ═════════════════════════════════════════════════════════════════════

    /// Send an image message with compression
    pub async fn send_image_message(
        &self,
        to_peer_id: String,
        image_data: Vec<u8>,
        file_name: String,
        quality: u32,
    ) -> Result<String, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::SendImageMessage {
                to_peer_id,
                image_data,
                file_name,
                quality: quality as u8,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Send a voice message
    pub async fn send_voice_message(
        &self,
        to_peer_id: String,
        audio_data: Vec<u8>,
        file_name: String,
        duration_seconds: i32,
    ) -> Result<String, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::SendVoiceMessage {
                to_peer_id,
                audio_data,
                file_name,
                duration_seconds,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Send a document/file message
    pub async fn send_document_message(
        &self,
        to_peer_id: String,
        file_data: Vec<u8>,
        file_name: String,
        mime_type: String,
    ) -> Result<String, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::SendDocumentMessage {
                to_peer_id,
                file_data,
                file_name,
                mime_type,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Send a video message
    pub async fn send_video_message(
        &self,
        to_peer_id: String,
        video_data: Vec<u8>,
        file_name: String,
        width: Option<i32>,
        height: Option<i32>,
        duration_seconds: i32,
        thumbnail_data: Option<Vec<u8>>,
    ) -> Result<String, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::SendVideoMessage {
                to_peer_id,
                video_data,
                file_name,
                width,
                height,
                duration_seconds,
                thumbnail_data,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Download media by hash
    pub async fn download_media(&self, media_hash: String) -> Result<Vec<u8>, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::DownloadMedia {
                media_hash,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Get media for a conversation
    pub fn get_conversation_media(
        &self,
        conversation_id: String,
        media_type: Option<types::FfiMediaType>,
        limit: Option<u32>,
    ) -> Result<Vec<types::FfiMedia>, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::GetConversationMedia {
                conversation_id,
                media_type,
                limit,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.blocking_recv().map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    // ═════════════════════════════════════════════════════════════════════
    // Message Actions (FASE 16 - Forward & Delete)
    // ═════════════════════════════════════════════════════════════════════

    /// Delete message (soft delete - marks as deleted locally)
    pub fn delete_message(&self, message_id: String) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::DeleteMessage {
                message_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.blocking_recv().map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Forward message to another peer/group
    pub async fn forward_message(
        &self,
        message_id: String,
        to_peer_id: String,
    ) -> Result<String, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::ForwardMessage {
                message_id,
                to_peer_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.await.map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    // ═════════════════════════════════════════════════════════════════════
    // Message Reactions (FASE 16 - TRACK 8)
    // ═════════════════════════════════════════════════════════════════════

    /// Add a reaction to a message
    pub fn add_reaction(
        &self,
        message_id: String,
        emoji: String,
    ) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::AddReaction {
                message_id,
                emoji,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.blocking_recv().map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Remove a reaction from a message
    pub fn remove_reaction(
        &self,
        message_id: String,
        emoji: String,
    ) -> Result<(), MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::RemoveReaction {
                message_id,
                emoji,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.blocking_recv().map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    /// Get all reactions for a message
    pub fn get_message_reactions(
        &self,
        message_id: String,
    ) -> Result<Vec<FfiReaction>, MePassaFfiError> {
        let (tx, rx) = oneshot::channel();
        self.handle()
            .sender
            .send(ClientCommand::GetMessageReactions {
                message_id,
                response: tx,
            })
            .map_err(|_| MePassaFfiError::Other {
                details: "Failed to send command".to_string(),
            })?;

        rx.blocking_recv().map_err(|_| MePassaFfiError::Other {
            details: "Failed to receive response".to_string(),
        })?
    }

    // TODO: Re-enable when enum types are fixed
    // pub fn get_current_call(&self) -> Result<Option<FfiCall>, MePassaFfiError>
    // pub fn get_call_stats(&self, _call_id: String) -> Result<Option<FfiCallStats>, MePassaFfiError>
}
