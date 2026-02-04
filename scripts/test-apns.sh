#!/usr/bin/env bash
set -euo pipefail

# Template for APNs push test via apns-push.
# Fill the values below or export them before running.

APNS_KEY_PATH="${APNS_KEY_PATH:-<CAMINHO/DA/CHAVE.p8>}"
APNS_KEY_ID="${APNS_KEY_ID:-<KEY_ID>}"
APNS_TEAM_ID="${APNS_TEAM_ID:-<TEAM_ID>}"
APNS_BUNDLE_ID="${APNS_BUNDLE_ID:-<BUNDLE_ID>}"
APNS_DEVICE_TOKEN="${APNS_DEVICE_TOKEN:-<DEVICE_TOKEN>}"
APNS_ENV="${APNS_ENV:-<development|production>}"
PEER_ID_DESTINO="${PEER_ID_DESTINO:-<PEER_ID_DESTINO>}"

if ! command -v apns-push >/dev/null 2>&1; then
  echo "apns-push not found. Install with: brew install apns-push"
  exit 1
fi

apns-push \
  --token "${APNS_KEY_PATH}" \
  --key-id "${APNS_KEY_ID}" \
  --team-id "${APNS_TEAM_ID}" \
  --topic "${APNS_BUNDLE_ID}" \
  --env "${APNS_ENV}" \
  --payload "{
    \"aps\": {
      \"alert\": {
        \"title\": \"Nova mensagem\",
        \"body\": \"Teste push iOS\"
      },
      \"sound\": \"default\"
    },
    \"peer_id\": \"${PEER_ID_DESTINO}\"
  }" \
  "${APNS_DEVICE_TOKEN}"
