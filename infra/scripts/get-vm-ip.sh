#!/usr/bin/env bash
set -euo pipefail
TICKET=$(python3 -c "import json; print(json.load(open('/tmp/ticket.json'))['data']['ticket'])")
curl -sk -b "PVEAuthCookie=${TICKET}" \
  "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/qemu/300/agent/network-get-interfaces"
