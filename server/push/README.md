# ZapLivre Push Notification Server

Push notification service for the ZapLivre P2P messaging platform.

## Features

- **FCM Integration**: Send push notifications to Android devices via Firebase Cloud Messaging
- **APNs Integration**: Send push notifications to iOS devices via Apple Push Notification service
- **Token Management**: Register, update, and deactivate device tokens
- **Multi-device Support**: Send notifications to all devices of a peer
- **Automatic Cleanup**: Mark invalid tokens as inactive
- **PostgreSQL Storage**: Store device tokens with metadata
- **JWT Authentication**: Token-based APNs authentication with automatic refresh

## Architecture

- **Framework**: Axum (async web framework)
- **Database**: PostgreSQL with sqlx
- **Push Services**:
  - Firebase Cloud Messaging (FCM) for Android
  - Apple Push Notification service (APNs) with HTTP/2 for iOS
- **Authentication**: JWT with ES256 for APNs token-based auth
- **Logging**: tracing with structured logs

## API Endpoints

### Health Check
```
GET /health
```

### Register Device Token
```
POST /api/v1/register
Content-Type: application/json

{
  "peer_id": "string",
  "platform": "fcm" | "apns",  // "fcm" for Android, "apns" for iOS
  "device_id": "string",
  "token": "string",
  "device_name": "string (optional)",
  "app_version": "string (optional)"
}
```

### Send Push Notification
```
POST /api/v1/send
Content-Type: application/json

{
  "peer_id": "string",
  "title": "string",
  "body": "string",
  "data": {  // optional
    "key": "value"
  }
}
```

### Unregister Device
```
DELETE /api/v1/unregister
Content-Type: application/json

{
  "peer_id": "string",
  "device_id": "string"
}
```

## Environment Variables

Create a `.env` file in the `server/push` directory:

```env
# Database
DATABASE_URL=postgresql://zaplivre:zaplivre_dev_password@localhost:5432/zaplivre

# Firebase Cloud Messaging (Android)
FCM_SERVER_KEY=your_fcm_server_key_here

# Apple Push Notification Service (iOS) - Optional
APNS_KEY_PATH=/path/to/AuthKey_XXXXXXXXXX.p8
APNS_KEY_ID=AB12CD34EF          # Your 10-character Key ID
APNS_TEAM_ID=XY98ZW76UV         # Your 10-character Team ID
APNS_BUNDLE_ID=com.zaplivre.ios  # Your app's Bundle ID
APNS_PRODUCTION=false           # Use false for development/TestFlight, true for App Store

# Server (optional)
RUST_LOG=zaplivre_push=debug,info
```

**Note:** APNs configuration is optional. If not provided, the server will only support FCM (Android) notifications. For complete APNs setup instructions, see [APNS_SETUP_GUIDE.md](../../docs/APNS_SETUP_GUIDE.md).

## Setup

### 1. Install Dependencies

```bash
# Rust toolchain (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# PostgreSQL (via Docker)
cd /Users/edsonmartins/desenvolvimento/zaplivre
docker-compose up -d postgres
```

### 2. Get FCM Server Key

1. Go to [Firebase Console](https://console.firebase.google.com)
2. Select your project (or create one)
3. Go to **Project Settings** → **Cloud Messaging**
4. Copy the **Server key** (legacy) or create a new one

### 3. Configure Environment

```bash
cd server/push
cp .env.example .env
# Edit .env and add your FCM_SERVER_KEY
```

### 4. Build

```bash
cargo build --release
```

Binary will be at: `../../target/release/zaplivre-push`

## Running

### Development
```bash
cd server/push
cargo run
```

### Production
```bash
./target/release/zaplivre-push
```

Server will start on `http://0.0.0.0:8081`

## Docker

### Build Image
```bash
docker build -t zaplivre-push:latest .
```

### Run Container
```bash
docker run -d \
  --name zaplivre-push \
  -p 8081:8081 \
  -e DATABASE_URL=postgresql://zaplivre:password@postgres:5432/zaplivre \
  -e FCM_SERVER_KEY=your_key \
  --network zaplivre_network \
  zaplivre-push:latest
```

## Testing

### Manual Test with curl

**1. Register a device:**
```bash
curl -X POST http://localhost:8081/api/v1/register \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "test_peer_123",
    "platform": "fcm",
    "device_id": "device_001",
    "token": "fcm_token_here",
    "device_name": "Test Device",
    "app_version": "0.1.0"
  }'
```

**2. Send a notification:**
```bash
curl -X POST http://localhost:8081/api/v1/send \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "test_peer_123",
    "title": "Test Notification",
    "body": "This is a test message",
    "data": {
      "type": "message",
      "from": "peer_456"
    }
  }'
```

**3. Unregister a device:**
```bash
curl -X DELETE http://localhost:8081/api/v1/unregister \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "test_peer_123",
    "device_id": "device_001"
  }'
```

## Database Schema

The push server uses the `push_tokens` table:

```sql
CREATE TABLE push_tokens (
    id SERIAL PRIMARY KEY,
    peer_id VARCHAR(255) NOT NULL,
    platform VARCHAR(50) NOT NULL,  -- 'fcm' or 'apns'
    device_id VARCHAR(255) NOT NULL,
    token TEXT NOT NULL,
    device_name VARCHAR(255),
    app_version VARCHAR(50),
    created_at TIMESTAMP DEFAULT NOW(),
    last_used_at TIMESTAMP DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true,
    UNIQUE(peer_id, device_id)
);
```

## Monitoring

### Logs
```bash
# Debug logs
RUST_LOG=zaplivre_push=debug cargo run

# Info logs (default)
RUST_LOG=info cargo run
```

### Health Check
```bash
curl http://localhost:8081/health
# Response: OK
```

## Integration with ZapLivre Core

The push server is designed to integrate with the ZapLivre core system:

1. **On app startup**: Each client registers its FCM/APNs token
2. **On message send**: If peer is offline, trigger push notification
3. **On notification received**: Wake up app to poll Message Store

## Future Improvements

- [x] APNs support for iOS - **COMPLETED (FASE 8)**
- [ ] Rate limiting
- [ ] Notification analytics
- [ ] Retry logic for failed notifications
- [ ] Token expiration and cleanup
- [ ] Silent notifications (data-only)
- [ ] Rich notifications with images
- [ ] APNs notification service extension for rich notifications

## Troubleshooting

### "Database connection failed"
- Ensure PostgreSQL is running: `docker-compose up postgres`
- Check DATABASE_URL in .env
- Verify database exists: `psql -U zaplivre -d zaplivre -c "\dt"`

### "FCM send failed"
- Verify FCM_SERVER_KEY is correct
- Check device token is valid
- Ensure Firebase project has FCM enabled
- Check logs for specific FCM error codes

### "Token not found"
- Device must register before receiving notifications
- Check if token was marked inactive due to errors

### "APNs error: BadDeviceToken"
- Device token doesn't match APNs environment (sandbox vs production)
- Use `APNS_PRODUCTION=false` for development/TestFlight
- Use `APNS_PRODUCTION=true` for App Store builds
- See [APNS_SETUP_GUIDE.md](../../docs/APNS_SETUP_GUIDE.md) for complete troubleshooting

### "APNs error: InvalidProviderToken"
- JWT token is invalid or expired
- Verify `APNS_KEY_ID` matches Apple Developer Portal
- Verify `APNS_TEAM_ID` is correct
- Ensure .p8 file is valid EC P-256 key

### "Failed to read APNs private key"
- Verify file path in `APNS_KEY_PATH`
- Check file permissions: `chmod 600 /path/to/AuthKey_*.p8`
- Ensure Docker volume mount is correct (for containerized deployment)

## License

AGPL-3.0

## Authors

Integrall Tech <contato@integralltech.com.br>
