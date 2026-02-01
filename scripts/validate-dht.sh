#!/usr/bin/env bash
set -euo pipefail

DHT1_HOST=${DHT1_HOST:-dht1.associahub.com.br}
DHT2_HOST=${DHT2_HOST:-dht2.associahub.com.br}
DHT1_PORT=${DHT1_PORT:-4001}
DHT2_PORT=${DHT2_PORT:-4002}

health_url() { printf "https://%s/health" "$1"; }

check_dns() {
  local host=$1
  if command -v getent >/dev/null 2>&1; then
    getent hosts "$host" >/dev/null 2>&1
  else
    ping -c1 -W1 "$host" >/dev/null 2>&1
  fi
}

check_port() {
  local host=$1
  local port=$2
  if command -v nc >/dev/null 2>&1; then
    nc -vz "$host" "$port" >/dev/null 2>&1
  else
    (echo > "/dev/tcp/$host/$port") >/dev/null 2>&1
  fi
}

extract_uptime() {
  local json=$1
  echo "$json" | sed -n 's/.*"uptime_seconds":\([0-9][0-9]*\).*/\1/p'
}

for host in "$DHT1_HOST" "$DHT2_HOST"; do
  echo "==> DNS: $host"
  if check_dns "$host"; then
    echo "OK"
  else
    echo "FAIL"
  fi

done

for host in "$DHT1_HOST" "$DHT2_HOST"; do
  echo "==> Health: $(health_url "$host")"
  if json=$(curl -fsS "$(health_url "$host")" 2>/dev/null); then
    uptime=$(extract_uptime "$json")
    if [ -n "$uptime" ]; then
      echo "OK uptime_seconds=$uptime"
    else
      echo "OK"
    fi
  else
    echo "FAIL"
  fi

done

for entry in "$DHT1_HOST:$DHT1_PORT" "$DHT2_HOST:$DHT2_PORT"; do
  host=${entry%:*}
  port=${entry#*:}
  echo "==> Port: $host:$port"
  if check_port "$host" "$port"; then
    echo "OK"
  else
    echo "FAIL"
  fi

done
