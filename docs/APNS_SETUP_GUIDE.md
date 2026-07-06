# APNs Setup Guide - ZapLivre

Complete guide for setting up Apple Push Notification service (APNs) for the ZapLivre iOS app.

## 📋 Prerequisites

1. **Apple Developer Account** ($99/year)
   - Individual or Organization account
   - Enrolled in the Apple Developer Program
   - Access to [Apple Developer Portal](https://developer.apple.com/account)

2. **Xcode Project**
   - Bundle ID registered (e.g., `com.zaplivre.ios`)
   - App ID configured in Apple Developer Portal

3. **ZapLivre Push Server**
   - Running instance with APNs support enabled
   - Access to server configuration files

---

## 🔑 Step 1: Create APNs Authentication Key (.p8 file)

### 1.1 Generate Key in Apple Developer Portal

1. Go to [Apple Developer Portal - Keys](https://developer.apple.com/account/resources/authkeys/list)
2. Click the **"+"** button to create a new key
3. Enter a **Key Name** (e.g., "ZapLivre APNs Key")
4. Check **"Apple Push Notifications service (APNs)"**
5. Click **"Continue"** and then **"Register"**
6. **Download the .p8 file** immediately
   - ⚠️ **IMPORTANT:** You can only download this once!
   - File name format: `AuthKey_XXXXXXXXXX.p8`
   - Save it securely (do NOT commit to git)

### 1.2 Note Important IDs

After creating the key, note down:

- **Key ID** (10 characters, e.g., `AB12CD34EF`)
  - Shown in the key details page
- **Team ID** (10 characters, e.g., `XY98ZW76UV`)
  - Found in [Membership Details](https://developer.apple.com/account/#!/membership)

---

## 📱 Step 2: Configure iOS App

### 2.1 Register App ID

1. Go to [Identifiers](https://developer.apple.com/account/resources/identifiers/list)
2. Click **"+"** to create a new identifier
3. Select **"App IDs"** → **"Continue"**
4. Select **"App"** → **"Continue"**
5. Fill in details:
   - **Description:** ZapLivre iOS App
   - **Bundle ID:** Explicit (e.g., `com.zaplivre.ios`)
   - **Capabilities:** Check **"Push Notifications"**
6. Click **"Continue"** → **"Register"**

### 2.2 Xcode Project Configuration

In your Xcode project (`ios/ZapLivre.xcodeproj`):

1. **Signing & Capabilities** tab:
   - Add **"Push Notifications"** capability
   - Add **"Background Modes"** capability
     - Check **"Remote notifications"** (already configured)

2. **Build Settings**:
   - Set **Bundle Identifier** to match registered App ID

---

## 🖥️ Step 3: Configure Push Server

### 3.1 Copy .p8 Key to Server

```bash
# Create secure directory for APNs key
mkdir -p /etc/zaplivre/apns
chmod 700 /etc/zaplivre/apns

# Copy the .p8 key file
cp AuthKey_XXXXXXXXXX.p8 /etc/zaplivre/apns/
chmod 600 /etc/zaplivre/apns/AuthKey_XXXXXXXXXX.p8
```

### 3.2 Update Environment Variables

Edit `.env` or `docker-compose.yml`:

```bash
# APNs Configuration
APNS_KEY_PATH=/etc/zaplivre/apns/AuthKey_XXXXXXXXXX.p8
APNS_KEY_ID=AB12CD34EF          # Your 10-character Key ID
APNS_TEAM_ID=XY98ZW76UV         # Your 10-character Team ID
APNS_BUNDLE_ID=com.zaplivre.ios  # Your app's Bundle ID
APNS_PRODUCTION=false           # Use false for development/sandbox
```

**Important:**
- Use `APNS_PRODUCTION=false` for TestFlight and development builds
- Use `APNS_PRODUCTION=true` only for App Store production builds

### 3.3 Update docker-compose.yml

Add volume mount for the APNs key:

```yaml
services:
  push-server:
    image: zaplivre/push-server:latest
    environment:
      - APNS_KEY_PATH=/etc/apns/AuthKey_XXXXXXXXXX.p8
      - APNS_KEY_ID=${APNS_KEY_ID}
      - APNS_TEAM_ID=${APNS_TEAM_ID}
      - APNS_BUNDLE_ID=${APNS_BUNDLE_ID}
      - APNS_PRODUCTION=${APNS_PRODUCTION:-false}
    volumes:
      - /etc/zaplivre/apns:/etc/apns:ro  # Read-only mount
```

---

## 🧪 Step 4: Test APNs Integration

### 4.1 Build and Run iOS App

```bash
cd ios
./build-all.sh --build

# Or open in Xcode
open ZapLivre.xcodeproj
```

### 4.2 Check Device Token Registration

1. Run the app on a physical device or simulator
2. Accept push notification permissions when prompted
3. Check logs for device token:
   ```
   🍎 APNs device token: 1234567890abcdef...
   ✅ Device token registered with push server
   ```

### 4.3 Test Push Notification

Use curl to send a test notification:

```bash
# Get the peer_id from the iOS app logs
PEER_ID="12D3KooW..."

# Send test notification
curl -X POST http://localhost:8081/api/v1/send \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "'$PEER_ID'",
    "title": "Test Notification",
    "body": "Hello from ZapLivre Push Server!",
    "data": {
      "message_id": "test_123",
      "sender_peer_id": "test_sender"
    }
  }'
```

**Expected response:**
```json
{
  "success": true,
  "sent_count": 1,
  "failed_count": 0,
  "message": "Sent 1 notification(s), 0 failed"
}
```

### 4.4 Verify Notification Received

- **App in foreground:** Notification should appear as banner
- **App in background:** Notification should appear in notification center
- **Badge count:** Should update if specified in payload

---

## 🔍 Troubleshooting

### Error: "APNs error: BadDeviceToken"

**Cause:** Device token doesn't match the APNs environment.

**Solution:**
- Sandbox (development): Use `APNS_PRODUCTION=false`
- Production: Use `APNS_PRODUCTION=true`
- Device tokens from TestFlight use **sandbox** endpoint
- Device tokens from App Store use **production** endpoint

### Error: "APNs error: InvalidProviderToken"

**Cause:** JWT token is invalid or expired.

**Solutions:**
- Verify `APNS_KEY_ID` matches the Key ID from Apple Developer Portal
- Verify `APNS_TEAM_ID` matches your Team ID
- Ensure .p8 file is valid EC P-256 key
- Check file permissions (must be readable by push server)

### Error: "APNs error: BadTopic"

**Cause:** Bundle ID mismatch.

**Solution:**
- Verify `APNS_BUNDLE_ID` exactly matches the iOS app's Bundle Identifier
- Check Xcode project settings

### Error: "Failed to read APNs private key"

**Cause:** .p8 file not found or no read permissions.

**Solutions:**
- Verify file path in `APNS_KEY_PATH`
- Check file permissions: `chmod 600 /path/to/AuthKey_*.p8`
- Ensure Docker volume mount is correct (for containerized deployment)

### No Notifications Received

**Checklist:**
1. ✅ Push permissions granted in iOS app
2. ✅ Device token successfully registered with push server
3. ✅ Push server logs show "APNs notification sent successfully"
4. ✅ Using correct APNs endpoint (sandbox vs production)
5. ✅ Device is connected to internet
6. ✅ App has valid provisioning profile

---

## 🔐 Security Best Practices

### 1. Protect Your .p8 Key

```bash
# Correct permissions
chmod 600 AuthKey_*.p8
chown zaplivre:zaplivre AuthKey_*.p8

# Never commit to version control
echo "*.p8" >> .gitignore
```

### 2. Use Environment Variables

Don't hardcode credentials in source code:

```rust
// ✅ Good
let key_path = env::var("APNS_KEY_PATH")?;

// ❌ Bad
let key_path = "/path/to/AuthKey_ABC123.p8";
```

### 3. Rotate Keys Periodically

- Create a new APNs key annually
- Update server configuration
- Revoke old key in Apple Developer Portal

### 4. Separate Development and Production

Use different keys for dev and prod environments:

```bash
# Development
APNS_KEY_PATH=/etc/apns/AuthKey_DEV123.p8
APNS_PRODUCTION=false

# Production
APNS_KEY_PATH=/etc/apns/AuthKey_PROD456.p8
APNS_PRODUCTION=true
```

---

## 📊 Monitoring

### Push Server Logs

Monitor APNs activity:

```bash
# Follow push server logs
docker logs -f zaplivre-push-server

# Look for:
# ✅ "APNs client initialized"
# ✅ "APNs notification sent successfully"
# ❌ "APNs error: ..."
```

### Key Metrics to Track

1. **Success Rate:** `sent_count / (sent_count + failed_count)`
2. **Invalid Tokens:** Count of `BadDeviceToken` errors
3. **Token Registrations:** Daily new device token registrations
4. **Notification Latency:** Time from API call to APNs response

---

## 📚 References

- [APNs Provider API](https://developer.apple.com/documentation/usernotifications/setting_up_a_remote_notification_server/sending_notification_requests_to_apns)
- [Token-Based Authentication](https://developer.apple.com/documentation/usernotifications/setting_up_a_remote_notification_server/establishing_a_token-based_connection_to_apns)
- [Payload Key Reference](https://developer.apple.com/documentation/usernotifications/setting_up_a_remote_notification_server/generating_a_remote_notification)
- [APNs Error Codes](https://developer.apple.com/documentation/usernotifications/setting_up_a_remote_notification_server/handling_notification_responses_from_apns)

---

## ✅ Checklist

Before going to production:

- [ ] APNs key (.p8) created and downloaded
- [ ] Key ID and Team ID noted
- [ ] App ID registered with Push Notifications capability
- [ ] iOS app configured with correct Bundle ID
- [ ] Push server environment variables set
- [ ] .p8 key file secured (correct permissions)
- [ ] Test notification successfully received
- [ ] Production endpoint tested with App Store build
- [ ] Monitoring and logging configured
- [ ] Key rotation schedule established

---

**Last Updated:** 2026-01-21
**ZapLivre Version:** FASE 8 - Push Notifications
