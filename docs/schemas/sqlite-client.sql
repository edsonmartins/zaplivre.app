-- ZapLivre Client Database Schema (SQLite)
-- Local storage for messages, contacts, groups, and user data

-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- Set journal mode for better concurrency
PRAGMA journal_mode = WAL;

-- ============================================================================
-- IDENTITY
-- ============================================================================

-- User's own identity
CREATE TABLE IF NOT EXISTS identity (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Only one row allowed
    peer_id TEXT NOT NULL UNIQUE,
    username TEXT UNIQUE, -- @username (ADR 001)
    keypair_blob BLOB NOT NULL, -- Ed25519 keypair (encrypted)
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_backup_at INTEGER
);

-- Prekeys pool
CREATE TABLE IF NOT EXISTS prekeys (
    prekey_id INTEGER PRIMARY KEY,
    prekey_blob BLOB NOT NULL, -- X25519 prekey
    is_signed BOOLEAN NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- CONTACTS
-- ============================================================================

-- Contacts (other users)
CREATE TABLE IF NOT EXISTS contacts (
    peer_id TEXT PRIMARY KEY,

    -- Identity Server fields (ADR 001)
    username TEXT UNIQUE, -- @username from Identity Server
    display_name TEXT,    -- Local nickname (user can edit)

    -- Crypto
    public_key BLOB NOT NULL,
    prekey_bundle_json TEXT, -- Cached prekey bundle from Identity Server

    -- Metadata
    last_seen INTEGER,
    status TEXT DEFAULT 'offline' CHECK (status IN ('online', 'offline', 'away')),

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX idx_contacts_username ON contacts(username);
CREATE INDEX idx_contacts_last_seen ON contacts(last_seen DESC);

-- ============================================================================
-- CONVERSATIONS
-- ============================================================================

-- Conversations (1:1 or group)
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY, -- UUID or peer_id (1:1) or group_id (group)
    type TEXT NOT NULL CHECK (type IN ('direct', 'group')),

    -- Display info
    title TEXT, -- Group name or contact display name
    avatar_path TEXT,

    -- Last message
    last_message_id TEXT,
    last_message_text TEXT,
    last_message_at INTEGER,

    -- Counters
    unread_count INTEGER NOT NULL DEFAULT 0,
    total_messages INTEGER NOT NULL DEFAULT 0,

    -- Status
    is_archived BOOLEAN NOT NULL DEFAULT 0,
    is_muted BOOLEAN NOT NULL DEFAULT 0,
    is_pinned BOOLEAN NOT NULL DEFAULT 0,

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    FOREIGN KEY (last_message_id) REFERENCES messages(id) ON DELETE SET NULL
);

CREATE INDEX idx_conversations_updated_at ON conversations(updated_at DESC);
CREATE INDEX idx_conversations_type ON conversations(type);

-- ============================================================================
-- MESSAGES
-- ============================================================================

-- Messages (E2E encrypted)
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY, -- UUID
    conversation_id TEXT NOT NULL,

    -- Sender
    sender_peer_id TEXT NOT NULL,

    -- Content (decrypted for display)
    content_type TEXT NOT NULL CHECK (content_type IN ('text', 'image', 'video', 'audio', 'file', 'location')),
    content_text TEXT, -- For text messages
    content_data_path TEXT, -- For media files (local path)

    -- Encrypted payload (raw)
    encrypted_payload BLOB,

    -- Metadata
    timestamp INTEGER NOT NULL,

    -- Status
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'delivered', 'read', 'failed')),
    is_outgoing BOOLEAN NOT NULL DEFAULT 0,

    -- Reactions (JSON array)
    reactions_json TEXT,

    -- Reply/Forward
    reply_to_message_id TEXT,
    forwarded_from_peer_id TEXT,

    -- Delivery tracking
    delivered_at INTEGER,
    read_at INTEGER,
    failed_reason TEXT,

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (sender_peer_id) REFERENCES contacts(peer_id),
    FOREIGN KEY (reply_to_message_id) REFERENCES messages(id) ON DELETE SET NULL
);

CREATE INDEX idx_messages_conversation_id ON messages(conversation_id, timestamp DESC);
CREATE INDEX idx_messages_sender ON messages(sender_peer_id);
CREATE INDEX idx_messages_status ON messages(status);

-- Full-text search index for messages
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    message_id UNINDEXED,
    content_text,
    content='messages',
    content_rowid='rowid'
);

-- Triggers to keep FTS index in sync
CREATE TRIGGER IF NOT EXISTS messages_fts_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, message_id, content_text)
    VALUES (new.rowid, new.id, new.content_text);
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_delete AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, message_id, content_text)
    VALUES ('delete', old.rowid, old.id, old.content_text);
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_update AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, message_id, content_text)
    VALUES ('delete', old.rowid, old.id, old.content_text);
    INSERT INTO messages_fts(rowid, message_id, content_text)
    VALUES (new.rowid, new.id, new.content_text);
END;

-- ============================================================================
-- GROUPS
-- ============================================================================

-- Groups
CREATE TABLE IF NOT EXISTS groups (
    id TEXT PRIMARY KEY, -- Group ID
    name TEXT NOT NULL,
    description TEXT,
    avatar_path TEXT,

    -- Admin
    creator_peer_id TEXT NOT NULL,

    -- Settings
    is_public BOOLEAN NOT NULL DEFAULT 0,
    max_members INTEGER NOT NULL DEFAULT 256,

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Group members
CREATE TABLE IF NOT EXISTS group_members (
    group_id TEXT NOT NULL,
    peer_id TEXT NOT NULL,

    -- Role
    role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('admin', 'moderator', 'member')),

    -- Status
    is_active BOOLEAN NOT NULL DEFAULT 1,

    -- Timestamps
    joined_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    left_at INTEGER,

    PRIMARY KEY (group_id, peer_id),
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
    FOREIGN KEY (peer_id) REFERENCES contacts(peer_id)
);

CREATE INDEX idx_group_members_group_id ON group_members(group_id);
CREATE INDEX idx_group_members_peer_id ON group_members(peer_id);

-- ============================================================================
-- MEDIA
-- ============================================================================

-- Media files (images, videos, audio, files)
CREATE TABLE IF NOT EXISTS media_files (
    id TEXT PRIMARY KEY, -- UUID
    message_id TEXT NOT NULL,

    -- File info
    file_path TEXT NOT NULL, -- Local path
    file_name TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    mime_type TEXT NOT NULL,

    -- Thumbnail (for images/videos)
    thumbnail_path TEXT,

    -- Dimensions (for images/videos)
    width INTEGER,
    height INTEGER,
    duration INTEGER, -- For audio/video (seconds)

    -- Upload status
    upload_status TEXT DEFAULT 'pending' CHECK (upload_status IN ('pending', 'uploading', 'uploaded', 'failed')),
    upload_progress INTEGER DEFAULT 0, -- 0-100%

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE INDEX idx_media_files_message_id ON media_files(message_id);

-- ============================================================================
-- CALL HISTORY
-- ============================================================================

-- Call history (VoIP)
CREATE TABLE IF NOT EXISTS call_history (
    id TEXT PRIMARY KEY, -- UUID
    conversation_id TEXT NOT NULL,

    -- Participants
    caller_peer_id TEXT NOT NULL,
    callee_peer_id TEXT NOT NULL,

    -- Call type
    call_type TEXT NOT NULL CHECK (call_type IN ('audio', 'video')),

    -- Status
    status TEXT NOT NULL CHECK (status IN ('missed', 'rejected', 'completed', 'failed')),

    -- Duration
    started_at INTEGER,
    ended_at INTEGER,
    duration INTEGER, -- Seconds

    -- Quality metrics
    quality_rating INTEGER, -- 1-5 (MOS score * 10)

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (caller_peer_id) REFERENCES contacts(peer_id),
    FOREIGN KEY (callee_peer_id) REFERENCES contacts(peer_id)
);

CREATE INDEX idx_call_history_conversation_id ON call_history(conversation_id, created_at DESC);
CREATE INDEX idx_call_history_caller ON call_history(caller_peer_id);
CREATE INDEX idx_call_history_callee ON call_history(callee_peer_id);

-- ============================================================================
-- SETTINGS
-- ============================================================================

-- App settings
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- SYNC STATE (for multi-device)
-- ============================================================================

-- Device sync state
CREATE TABLE IF NOT EXISTS sync_state (
    device_id TEXT PRIMARY KEY,
    last_sync_at INTEGER NOT NULL,
    sync_vector_clock TEXT NOT NULL, -- CRDT vector clock
    is_active BOOLEAN NOT NULL DEFAULT 1
);

-- ============================================================================
-- COMMENTS
-- ============================================================================

-- Username support (ADR 001):
-- - contacts.username: @username from Identity Server
-- - contacts.display_name: Local nickname (editable by user)
-- - contacts.prekey_bundle_json: Cached prekey bundle for X3DH
-- - identity.username: User's own @username

-- Schema version
INSERT OR IGNORE INTO settings (key, value) VALUES ('schema_version', '1');
INSERT OR IGNORE INTO settings (key, value) VALUES ('created_at', strftime('%s', 'now'));
