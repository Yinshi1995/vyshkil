#!/usr/bin/env bash
set -uo pipefail
for i in $(seq 1 40); do
  OUT=$(curl -sk -b "PVEAuthCookie=$(python3 -c "import json; print(json.load(open('/tmp/ticket.json'))['data']['ticket'])")" \
    "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/qemu/300/agent/network-get-interfaces" 2>/dev/null)
  echo "[$i] $OUT"
  if echo "$OUT" | grep -q '"result"'; then
    echo "AGENT_UP"
    exit 0
  fi
  sleep 8
done
echo "TIMEOUT"
exit 1
