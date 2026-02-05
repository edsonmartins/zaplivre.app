#!/usr/bin/env bash
set -euo pipefail

# Template for FCM push test via HTTP v1.
# Fill the values below or export them before running.

FCM_PROJECT_ID="${FCM_PROJECT_ID:-<PROJECT_ID>}"
FCM_ACCESS_TOKEN="${FCM_ACCESS_TOKEN:-<OAUTH2_ACCESS_TOKEN>}"
FCM_DEVICE_TOKEN="${FCM_DEVICE_TOKEN:-<DEVICE_TOKEN>}"
PEER_ID_DESTINO="${PEER_ID_DESTINO:-<PEER_ID_DESTINO>}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl not found."
  exit 1
fi

curl -sS -X POST \
  "https://fcm.googleapis.com/v1/projects/${FCM_PROJECT_ID}/messages:send" \
  -H "Authorization: Bearer ${FCM_ACCESS_TOKEN}" \
  -H "Content-Type: application/json; UTF-8" \
  -d "{
    \"message\": {
      \"token\": \"${FCM_DEVICE_TOKEN}\",
      \"notification\": {
        \"title\": \"Nova mensagem\",
        \"body\": \"Teste push Android\"
      },
      \"data\": {
        \"peer_id\": \"${PEER_ID_DESTINO}\",
        \"title\": \"Nova mensagem\",
        \"body\": \"Teste push Android\"
      }
    }
  }"
