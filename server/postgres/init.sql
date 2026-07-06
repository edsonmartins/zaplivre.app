-- ZapLivre Database Initialization Script
-- PostgreSQL 16+

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Set timezone
SET timezone = 'UTC';

-- ============================================================================
-- TABLES
-- ============================================================================

-- Offline Messages (Store & Forward)
CREATE TABLE IF NOT EXISTS offline_messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- Recipient
    recipient_peer_id TEXT NOT NULL,

    -- Sender
    sender_peer_id TEXT NOT NULL,

    -- Message content (E2E encrypted)
    encrypted_payload BYTEA NOT NULL,

    -- Metadata
    message_type TEXT NOT NULL DEFAULT 'text',
    message_id TEXT NOT NULL UNIQUE,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (NOW() + INTERVAL '14 days'),
    delivered_at TIMESTAMP WITH TIME ZONE,

    -- Status
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'delivered', 'expired', 'failed')),

    -- Retry tracking
    delivery_attempts INTEGER NOT NULL DEFAULT 0,
    last_attempt_at TIMESTAMP WITH TIME ZONE,

    -- Size tracking
    payload_size_bytes INTEGER NOT NULL,

    -- Indexes
    CONSTRAINT offline_messages_recipient_idx CHECK (recipient_peer_id <> ''),
    CONSTRAINT offline_messages_sender_idx CHECK (sender_peer_id <> '')
);

-- Indexes for offline_messages
CREATE INDEX idx_offline_messages_recipient ON offline_messages(recipient_peer_id) WHERE status = 'pending';
CREATE INDEX idx_offline_messages_created_at ON offline_messages(created_at DESC);
CREATE INDEX idx_offline_messages_expires_at ON offline_messages(expires_at) WHERE status = 'pending';
CREATE INDEX idx_offline_messages_status ON offline_messages(status);

-- Push Notification Tokens
CREATE TABLE IF NOT EXISTS push_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- User (multiple devices per peer allowed; uniqueness is (peer_id, device_id))
    peer_id TEXT NOT NULL,

    -- Token
    token TEXT NOT NULL,
    platform TEXT NOT NULL CHECK (platform IN ('fcm', 'apns')),

    -- Device info
    device_id TEXT NOT NULL,
    device_name TEXT,
    app_version TEXT,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMP WITH TIME ZONE,

    -- Status
    is_active BOOLEAN NOT NULL DEFAULT TRUE,

    CONSTRAINT push_tokens_peer_device UNIQUE (peer_id, device_id)
);

-- Indexes for push_tokens
CREATE INDEX idx_push_tokens_peer_id ON push_tokens(peer_id) WHERE is_active = TRUE;
CREATE INDEX idx_push_tokens_platform ON push_tokens(platform);

-- User Presence (tracked by Redis primarily, but DB for backup)
CREATE TABLE IF NOT EXISTS user_presence (
    peer_id TEXT PRIMARY KEY,

    -- Status
    status TEXT NOT NULL CHECK (status IN ('online', 'offline', 'away')) DEFAULT 'offline',

    -- Connection info
    connected_nodes TEXT[],
    last_seen_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Timestamps
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Index for user_presence
CREATE INDEX idx_user_presence_status ON user_presence(status);
CREATE INDEX idx_user_presence_last_seen ON user_presence(last_seen_at DESC);

-- Message Delivery Stats (for monitoring)
CREATE TABLE IF NOT EXISTS message_stats (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- Date bucket
    date DATE NOT NULL,

    -- Metrics
    total_messages BIGINT NOT NULL DEFAULT 0,
    p2p_direct BIGINT NOT NULL DEFAULT 0,
    turn_relay BIGINT NOT NULL DEFAULT 0,
    store_forward BIGINT NOT NULL DEFAULT 0,

    -- Delivery success
    delivered BIGINT NOT NULL DEFAULT 0,
    failed BIGINT NOT NULL DEFAULT 0,

    -- Updated
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    UNIQUE(date)
);

-- Index for message_stats
CREATE INDEX idx_message_stats_date ON message_stats(date DESC);

-- Usernames (Identity Server) - ADR 001
CREATE TABLE IF NOT EXISTS usernames (
    -- Username (unique, 3-20 chars, lowercase alphanumeric + underscore)
    username TEXT PRIMARY KEY,

    -- Peer ID mapping
    peer_id TEXT NOT NULL UNIQUE,

    -- Public key (for verification)
    public_key BYTEA NOT NULL,

    -- Prekey bundle for X3DH (stored as JSONB)
    prekey_bundle JSONB NOT NULL,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Username format validation (3-20 chars, lowercase alphanumeric + underscore)
    CONSTRAINT username_format CHECK (username ~ '^[a-z0-9_]{3,20}$')
);

-- Indexes for usernames
CREATE INDEX idx_usernames_peer_id ON usernames(peer_id);
CREATE INDEX idx_usernames_created_at ON usernames(created_at DESC);

-- ============================================================================
-- FUNCTIONS
-- ============================================================================

-- Function to auto-delete expired messages
CREATE OR REPLACE FUNCTION delete_expired_messages()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    WITH deleted AS (
        DELETE FROM offline_messages
        WHERE status = 'pending'
          AND expires_at < NOW()
        RETURNING id
    )
    SELECT COUNT(*) INTO deleted_count FROM deleted;

    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Function to mark message as delivered
CREATE OR REPLACE FUNCTION mark_message_delivered(message_id_param TEXT)
RETURNS BOOLEAN AS $$
BEGIN
    UPDATE offline_messages
    SET status = 'delivered',
        delivered_at = NOW()
    WHERE message_id = message_id_param
      AND status = 'pending';

    RETURN FOUND;
END;
$$ LANGUAGE plpgsql;

-- Function to update presence
CREATE OR REPLACE FUNCTION update_presence(
    peer_id_param TEXT,
    status_param TEXT,
    connected_nodes_param TEXT[]
)
RETURNS VOID AS $$
BEGIN
    INSERT INTO user_presence (peer_id, status, connected_nodes, last_seen_at, updated_at)
    VALUES (peer_id_param, status_param, connected_nodes_param, NOW(), NOW())
    ON CONFLICT (peer_id)
    DO UPDATE SET
        status = EXCLUDED.status,
        connected_nodes = EXCLUDED.connected_nodes,
        last_seen_at = NOW(),
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- Function to increment message stats
CREATE OR REPLACE FUNCTION increment_message_stats(
    delivery_type TEXT
)
RETURNS VOID AS $$
BEGIN
    INSERT INTO message_stats (date, total_messages, p2p_direct, turn_relay, store_forward)
    VALUES (
        CURRENT_DATE,
        1,
        CASE WHEN delivery_type = 'p2p' THEN 1 ELSE 0 END,
        CASE WHEN delivery_type = 'turn' THEN 1 ELSE 0 END,
        CASE WHEN delivery_type = 'store' THEN 1 ELSE 0 END
    )
    ON CONFLICT (date)
    DO UPDATE SET
        total_messages = message_stats.total_messages + 1,
        p2p_direct = message_stats.p2p_direct + CASE WHEN delivery_type = 'p2p' THEN 1 ELSE 0 END,
        turn_relay = message_stats.turn_relay + CASE WHEN delivery_type = 'turn' THEN 1 ELSE 0 END,
        store_forward = message_stats.store_forward + CASE WHEN delivery_type = 'store' THEN 1 ELSE 0 END,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- SCHEDULED JOBS (Using pg_cron extension if available)
-- ============================================================================

-- Try to create pg_cron extension (will fail if not available, that's OK)
DO $$
BEGIN
    CREATE EXTENSION IF NOT EXISTS pg_cron;
EXCEPTION
    WHEN OTHERS THEN
        RAISE NOTICE 'pg_cron extension not available, skipping scheduled jobs';
END
$$;

-- Schedule cleanup job (if pg_cron available)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_cron') THEN
        -- Run cleanup every hour
        PERFORM cron.schedule('cleanup-expired-messages', '0 * * * *', $$SELECT delete_expired_messages()$$);
    END IF;
EXCEPTION
    WHEN OTHERS THEN
        RAISE NOTICE 'Could not schedule cleanup job: %', SQLERRM;
END
$$;

-- ============================================================================
-- INITIAL DATA
-- ============================================================================

-- Insert initial stats row
INSERT INTO message_stats (date, total_messages, p2p_direct, turn_relay, store_forward)
VALUES (CURRENT_DATE, 0, 0, 0, 0)
ON CONFLICT (date) DO NOTHING;

-- ============================================================================
-- GRANTS (for production)
-- ============================================================================

-- Grant permissions to zaplivre user
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO zaplivre;
-- GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO zaplivre;
-- GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO zaplivre;

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE offline_messages IS 'Stores encrypted messages for offline recipients (TTL 14 days)';
COMMENT ON TABLE push_tokens IS 'Push notification tokens for FCM/APNs';
COMMENT ON TABLE user_presence IS 'User online/offline status (backup to Redis)';
COMMENT ON TABLE message_stats IS 'Daily aggregated message delivery statistics';
COMMENT ON TABLE usernames IS 'Username → peer_id mapping for Identity Server (ADR 001) with prekey bundles for X3DH';

COMMENT ON FUNCTION delete_expired_messages() IS 'Deletes messages older than TTL (14 days)';
COMMENT ON FUNCTION mark_message_delivered(TEXT) IS 'Marks message as delivered and sets delivery timestamp';
COMMENT ON FUNCTION update_presence(TEXT, TEXT, TEXT[]) IS 'Updates user presence status';
COMMENT ON FUNCTION increment_message_stats(TEXT) IS 'Increments daily message statistics by delivery type';

-- ============================================================================
-- VACUUM & ANALYZE
-- ============================================================================

VACUUM ANALYZE;

-- ============================================================================
-- SUCCESS MESSAGE
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '==========================================================';
    RAISE NOTICE 'ZapLivre Database Initialization Complete';
    RAISE NOTICE '==========================================================';
    RAISE NOTICE 'Database: zaplivre';
    RAISE NOTICE 'Tables created: 5 (offline_messages, push_tokens, user_presence, message_stats, usernames)';
    RAISE NOTICE 'Functions created: 4';
    RAISE NOTICE 'TTL: 14 days for offline messages';
    RAISE NOTICE 'Identity Server: username → peer_id mapping enabled';
    RAISE NOTICE '==========================================================';
END
$$;
