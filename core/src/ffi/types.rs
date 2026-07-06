//! FFI-safe types for UniFFI bindings

use crate::storage::{Conversation, Message, MessageStatus as InternalMessageStatus};

/// FFI-safe error type (rich error with messages)
#[derive(Debug, thiserror::Error)]
pub enum ZapLivreFfiError {
    #[error("Identity error: {details}")]
    Identity { details: String },

    #[error("Crypto error: {details}")]
    Crypto { details: String },

    #[error("Network error: {details}")]
    Network { details: String },

    #[error("Storage error: {details}")]
    Storage { details: String },

    #[error("Protocol error: {details}")]
    Protocol { details: String },

    #[error("IO error: {details}")]
    Io { details: String },

    #[error("Other error: {details}")]
    Other { details: String },
}

impl From<crate::utils::error::ZapLivreError> for ZapLivreFfiError {
    fn from(err: crate::utils::error::ZapLivreError) -> Self {
        match err {
            crate::utils::error::ZapLivreError::Identity(s) => {
                ZapLivreFfiError::Identity { details: s }
            }
            crate::utils::error::ZapLivreError::Crypto(s) => ZapLivreFfiError::Crypto { details: s },
            crate::utils::error::ZapLivreError::Network(s) => {
                ZapLivreFfiError::Network { details: s }
            }
            crate::utils::error::ZapLivreError::Storage(s) => {
                ZapLivreFfiError::Storage { details: s }
            }
            crate::utils::error::ZapLivreError::Protocol(s) => {
                ZapLivreFfiError::Protocol { details: s }
            }
            crate::utils::error::ZapLivreError::NotFound(s) => {
                ZapLivreFfiError::Other { details: format!("Not found: {}", s) }
            }
            crate::utils::error::ZapLivreError::Permission(s) => {
                ZapLivreFfiError::Other { details: format!("Permission denied: {}", s) }
            }
            crate::utils::error::ZapLivreError::AlreadyExists(s) => {
                ZapLivreFfiError::Other { details: format!("Already exists: {}", s) }
            }
            crate::utils::error::ZapLivreError::Io(e) => ZapLivreFfiError::Io {
                details: e.to_string(),
            },
            crate::utils::error::ZapLivreError::Other(s) => ZapLivreFfiError::Other { details: s },
        }
    }
}

/// FFI-safe message status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
}

impl From<InternalMessageStatus> for MessageStatus {
    fn from(status: InternalMessageStatus) -> Self {
        match status {
            InternalMessageStatus::Pending => MessageStatus::Pending,
            InternalMessageStatus::Sent => MessageStatus::Sent,
            InternalMessageStatus::Delivered => MessageStatus::Delivered,
            InternalMessageStatus::Read => MessageStatus::Read,
            InternalMessageStatus::Failed => MessageStatus::Failed,
        }
    }
}

/// FFI-safe message record
#[derive(Debug, Clone)]
pub struct FfiMessage {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_peer_id: String,
    pub recipient_peer_id: Option<String>,
    pub message_type: String,
    pub content_plaintext: Option<String>,
    pub created_at: i64,
    pub sent_at: Option<i64>,
    pub received_at: Option<i64>,
    pub read_at: Option<i64>,
    pub status: MessageStatus,
    pub is_deleted: bool,
}

impl From<Message> for FfiMessage {
    fn from(msg: Message) -> Self {
        Self {
            message_id: msg.message_id,
            conversation_id: msg.conversation_id,
            sender_peer_id: msg.sender_peer_id,
            recipient_peer_id: msg.recipient_peer_id,
            message_type: msg.message_type,
            content_plaintext: msg.content_plaintext,
            created_at: msg.created_at,
            sent_at: msg.sent_at,
            received_at: msg.received_at,
            read_at: msg.read_at,
            status: msg.status.into(),
            is_deleted: msg.is_deleted,
        }
    }
}

/// FFI-safe conversation record
#[derive(Debug, Clone)]
pub struct FfiConversation {
    pub id: String,
    pub conversation_type: String,
    pub peer_id: Option<String>,
    pub display_name: Option<String>,
    pub last_message_id: Option<String>,
    pub last_message_at: Option<i64>,
    pub unread_count: i32,
    pub is_muted: bool,
    pub is_archived: bool,
    pub created_at: i64,
}

impl From<Conversation> for FfiConversation {
    fn from(conv: Conversation) -> Self {
        Self {
            id: conv.id,
            conversation_type: conv.conversation_type,
            peer_id: conv.peer_id,
            display_name: conv.display_name,
            last_message_id: conv.last_message_id,
            last_message_at: conv.last_message_at,
            unread_count: conv.unread_count,
            is_muted: conv.is_muted,
            is_archived: conv.is_archived,
            created_at: conv.created_at,
        }
    }
}

// ========== VoIP Types ==========

#[cfg(feature = "voip")]
use crate::voip::{Call, CallDirection as InternalCallDirection, CallEndReason as InternalCallEndReason, CallState as InternalCallState, CallStats};

/// FFI-safe call state enum
#[cfg(feature = "voip")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiCallState {
    Initiating,
    Ringing,
    Connecting,
    Active,
    Ending,
    Ended,
}

/// FFI-safe call state enum (stub when voip feature is disabled)
#[cfg(not(feature = "voip"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiCallState {
    Initiating,
    Ringing,
    Connecting,
    Active,
    Ending,
    Ended,
}

#[cfg(feature = "voip")]
impl From<InternalCallState> for FfiCallState {
    fn from(state: InternalCallState) -> Self {
        match state {
            InternalCallState::Initiating => FfiCallState::Initiating,
            InternalCallState::Ringing => FfiCallState::Ringing,
            InternalCallState::Connecting => FfiCallState::Connecting,
            InternalCallState::Active => FfiCallState::Active,
            InternalCallState::Ending => FfiCallState::Ending,
            InternalCallState::Ended { .. } => FfiCallState::Ended,
        }
    }
}

/// FFI-safe call direction enum
#[cfg(feature = "voip")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiCallDirection {
    Outgoing,
    Incoming,
}

/// FFI-safe call direction enum (stub when voip feature is disabled)
#[cfg(not(feature = "voip"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiCallDirection {
    Outgoing,
    Incoming,
}

#[cfg(feature = "voip")]
impl From<InternalCallDirection> for FfiCallDirection {
    fn from(dir: InternalCallDirection) -> Self {
        match dir {
            InternalCallDirection::Outgoing => FfiCallDirection::Outgoing,
            InternalCallDirection::Incoming => FfiCallDirection::Incoming,
        }
    }
}

/// FFI-safe call end reason enum
#[cfg(feature = "voip")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiCallEndReason {
    Hangup,
    Rejected,
    LocalHangup,
    RemoteHangup,
    ConnectionFailed,
    Timeout,
    NetworkError,
}

/// FFI-safe call end reason enum (stub when voip feature is disabled)
#[cfg(not(feature = "voip"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiCallEndReason {
    Hangup,
    Rejected,
    LocalHangup,
    RemoteHangup,
    ConnectionFailed,
    Timeout,
    NetworkError,
}

#[cfg(feature = "voip")]
impl From<InternalCallEndReason> for FfiCallEndReason {
    fn from(reason: InternalCallEndReason) -> Self {
        match reason {
            InternalCallEndReason::Hangup => FfiCallEndReason::Hangup,
            InternalCallEndReason::Rejected => FfiCallEndReason::Rejected,
            InternalCallEndReason::LocalHangup => FfiCallEndReason::LocalHangup,
            InternalCallEndReason::RemoteHangup => FfiCallEndReason::RemoteHangup,
            InternalCallEndReason::ConnectionFailed => FfiCallEndReason::ConnectionFailed,
            InternalCallEndReason::Timeout => FfiCallEndReason::Timeout,
            InternalCallEndReason::NetworkError => FfiCallEndReason::NetworkError,
        }
    }
}

/// FFI-safe call record
#[cfg(feature = "voip")]
#[derive(Debug, Clone)]
pub struct FfiCall {
    pub id: String,
    pub remote_peer_id: String,
    pub direction: FfiCallDirection,
    pub state: FfiCallState,
    pub started_at: i64,
    pub connected_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub audio_muted: bool,
    pub speakerphone: bool,
    pub video_enabled: bool,
    pub video_codec: Option<FfiVideoCodec>,
}

/// FFI-safe call record (stub when voip feature is disabled)
#[cfg(not(feature = "voip"))]
#[derive(Debug, Clone)]
pub struct FfiCall {
    pub id: String,
    pub remote_peer_id: String,
    pub direction: FfiCallDirection,
    pub state: FfiCallState,
    pub started_at: i64,
    pub connected_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub audio_muted: bool,
    pub speakerphone: bool,
    pub video_enabled: bool,
    pub video_codec: Option<FfiVideoCodec>,
}

#[cfg(feature = "voip")]
impl From<Call> for FfiCall {
    fn from(call: Call) -> Self {
        Self {
            id: call.id,
            remote_peer_id: call.remote_peer_id,
            direction: call.direction.into(),
            state: call.state.into(),
            started_at: call.started_at.timestamp(),
            connected_at: call.connected_at.map(|t| t.timestamp()),
            ended_at: call.ended_at.map(|t| t.timestamp()),
            audio_muted: call.audio_muted,
            speakerphone: call.speakerphone,
            video_enabled: call.video_enabled,
            video_codec: call.video_codec.map(Into::into),
        }
    }
}

/// FFI-safe call statistics
#[cfg(feature = "voip")]
#[derive(Debug, Clone)]
pub struct FfiCallStats {
    pub avg_rtt_ms: u32,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_lost: u64,
    pub jitter_ms: u32,
    pub audio_bitrate_kbps: u32,
}

/// FFI-safe call statistics (stub when voip feature is disabled)
#[cfg(not(feature = "voip"))]
#[derive(Debug, Clone)]
pub struct FfiCallStats {
    pub avg_rtt_ms: u32,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_lost: u64,
    pub jitter_ms: u32,
    pub audio_bitrate_kbps: u32,
}

#[cfg(feature = "voip")]
impl From<CallStats> for FfiCallStats {
    fn from(stats: CallStats) -> Self {
        Self {
            avg_rtt_ms: stats.avg_rtt_ms,
            packets_sent: stats.packets_sent,
            packets_received: stats.packets_received,
            packets_lost: stats.packets_lost,
            jitter_ms: stats.jitter_ms,
            audio_bitrate_kbps: stats.audio_bitrate_kbps,
        }
    }
}

// ========== Video Types (FASE 14) ==========

#[cfg(any(feature = "voip", feature = "video"))]
use crate::voip::VideoCodec as InternalVideoCodec;

/// FFI-safe video codec enum
#[cfg(any(feature = "voip", feature = "video"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiVideoCodec {
    H264,
    VP8,
    VP9,
}

/// FFI-safe video codec enum (stub when voip/video features are disabled)
#[cfg(not(any(feature = "voip", feature = "video")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiVideoCodec {
    H264,
    VP8,
    VP9,
}

#[cfg(any(feature = "voip", feature = "video"))]
impl From<InternalVideoCodec> for FfiVideoCodec {
    fn from(codec: InternalVideoCodec) -> Self {
        match codec {
            InternalVideoCodec::H264 => FfiVideoCodec::H264,
            InternalVideoCodec::VP8 => FfiVideoCodec::VP8,
            InternalVideoCodec::VP9 => FfiVideoCodec::VP9,
        }
    }
}

#[cfg(any(feature = "voip", feature = "video"))]
impl From<FfiVideoCodec> for InternalVideoCodec {
    fn from(codec: FfiVideoCodec) -> Self {
        match codec {
            FfiVideoCodec::H264 => InternalVideoCodec::H264,
            FfiVideoCodec::VP8 => InternalVideoCodec::VP8,
            FfiVideoCodec::VP9 => InternalVideoCodec::VP9,
        }
    }
}

/// FFI-safe video resolution
#[cfg(any(feature = "voip", feature = "video"))]
#[derive(Debug, Clone, Copy)]
pub struct FfiVideoResolution {
    pub width: u32,
    pub height: u32,
}

/// FFI-safe video resolution (stub when voip/video features are disabled)
#[cfg(not(any(feature = "voip", feature = "video")))]
#[derive(Debug, Clone, Copy)]
pub struct FfiVideoResolution {
    pub width: u32,
    pub height: u32,
}

/// FFI-safe camera position enum
#[cfg(any(feature = "voip", feature = "video"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiCameraPosition {
    Front,
    Back,
    External,
}

/// FFI-safe camera position enum (stub when voip/video features are disabled)
#[cfg(not(any(feature = "voip", feature = "video")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiCameraPosition {
    Front,
    Back,
    External,
}

/// FFI-safe video statistics
#[cfg(any(feature = "voip", feature = "video"))]
#[derive(Debug, Clone)]
pub struct FfiVideoStats {
    pub resolution: FfiVideoResolution,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub frames_sent: u64,
    pub frames_received: u64,
    pub frames_dropped: u64,
}

/// FFI-safe video statistics (stub when voip/video features are disabled)
#[cfg(not(any(feature = "voip", feature = "video")))]
#[derive(Debug, Clone)]
pub struct FfiVideoStats {
    pub resolution: FfiVideoResolution,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub frames_sent: u64,
    pub frames_received: u64,
    pub frames_dropped: u64,
}

// ========== Video Frame Callback (FASE 14) ==========

// Note: FfiVideoFrameCallback trait is auto-generated by UniFFI from zaplivre.udl
// The callback interface is defined in the UDL as:
// callback interface FfiVideoFrameCallback {
//     void on_video_frame(string call_id, sequence<u8> frame_data, u32 width, u32 height);
// }

// ========== Group Types (FASE 15) ==========

use crate::group::{Group as InternalGroup, GroupRole as InternalGroupRole};

/// FFI-safe group
#[derive(Debug, Clone)]
pub struct FfiGroup {
    /// Group ID
    pub id: String,

    /// Group name
    pub name: String,

    /// Group description (optional)
    pub description: Option<String>,

    /// Avatar hash (optional)
    pub avatar_hash: Option<String>,

    /// Creator peer ID
    pub creator_peer_id: String,

    /// Member count
    pub member_count: u32,

    /// Whether local user is admin
    pub is_admin: bool,

    /// Created timestamp (Unix epoch)
    pub created_at: i64,
}

impl FfiGroup {
    pub fn from_group(group: &InternalGroup, local_peer_id: &str) -> Self {
        Self {
            id: group.id.clone(),
            name: group.name.clone(),
            description: group.description.clone(),
            avatar_hash: group.avatar_hash.clone(),
            creator_peer_id: group.creator_peer_id.clone(),
            member_count: group.member_count() as u32,
            is_admin: group.is_admin(local_peer_id),
            created_at: group.created_at,
        }
    }
}

/// FFI-safe group member
#[derive(Debug, Clone)]
pub struct FfiGroupMember {
    /// Peer ID
    pub peer_id: String,

    /// Member role
    pub role: FfiGroupRole,

    /// Joined timestamp (Unix epoch)
    pub joined_at: i64,
}

/// FFI-safe group role enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiGroupRole {
    Creator,
    Admin,
    Member,
}

impl From<InternalGroupRole> for FfiGroupRole {
    fn from(role: InternalGroupRole) -> Self {
        match role {
            InternalGroupRole::Creator => FfiGroupRole::Creator,
            InternalGroupRole::Admin => FfiGroupRole::Admin,
            InternalGroupRole::Member => FfiGroupRole::Member,
        }
    }
}

impl From<FfiGroupRole> for InternalGroupRole {
    fn from(role: FfiGroupRole) -> Self {
        match role {
            FfiGroupRole::Creator => InternalGroupRole::Creator,
            FfiGroupRole::Admin => InternalGroupRole::Admin,
            FfiGroupRole::Member => InternalGroupRole::Member,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Media types (FASE 16 - Mídia & Polimento)
// ═══════════════════════════════════════════════════════════════════════════

use crate::storage::{Media as InternalMedia, MediaType as InternalMediaType};

/// FFI-safe media type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiMediaType {
    Image,
    Video,
    Audio,
    Document,
    VoiceMessage,
}

impl From<InternalMediaType> for FfiMediaType {
    fn from(media_type: InternalMediaType) -> Self {
        match media_type {
            InternalMediaType::Image => FfiMediaType::Image,
            InternalMediaType::Video => FfiMediaType::Video,
            InternalMediaType::Audio => FfiMediaType::Audio,
            InternalMediaType::Document => FfiMediaType::Document,
            InternalMediaType::VoiceMessage => FfiMediaType::VoiceMessage,
        }
    }
}

impl From<FfiMediaType> for InternalMediaType {
    fn from(media_type: FfiMediaType) -> Self {
        match media_type {
            FfiMediaType::Image => InternalMediaType::Image,
            FfiMediaType::Video => InternalMediaType::Video,
            FfiMediaType::Audio => InternalMediaType::Audio,
            FfiMediaType::Document => InternalMediaType::Document,
            FfiMediaType::VoiceMessage => InternalMediaType::VoiceMessage,
        }
    }
}

/// FFI-safe media record
#[derive(Debug, Clone)]
pub struct FfiMedia {
    pub id: i64,
    pub media_hash: String,
    pub message_id: String,
    pub media_type: FfiMediaType,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub local_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_seconds: Option<i32>,
    pub created_at: i64,
}

impl From<InternalMedia> for FfiMedia {
    fn from(media: InternalMedia) -> Self {
        Self {
            id: media.id,
            media_hash: media.media_hash,
            message_id: media.message_id,
            media_type: media.media_type.into(),
            file_name: media.file_name,
            file_size: media.file_size,
            mime_type: media.mime_type,
            local_path: media.local_path,
            thumbnail_path: media.thumbnail_path,
            width: media.width,
            height: media.height,
            duration_seconds: media.duration_seconds,
            created_at: media.created_at,
        }
    }
}

// ═════════════════════════════════════════════════════════════════════
// Message Reactions (FASE 16 - TRACK 8)
// ═════════════════════════════════════════════════════════════════════

/// FFI-safe reaction record
#[derive(Debug, Clone)]
pub struct FfiReaction {
    pub reaction_id: String,
    pub message_id: String,
    pub peer_id: String,
    pub emoji: String,
    pub created_at: i64,
}

impl From<crate::storage::Reaction> for FfiReaction {
    fn from(reaction: crate::storage::Reaction) -> Self {
        Self {
            reaction_id: reaction.reaction_id,
            message_id: reaction.message_id,
            peer_id: reaction.peer_id,
            emoji: reaction.emoji,
            created_at: reaction.created_at,
        }
    }
}
