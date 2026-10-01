#!/usr/bin/env bash
set -euo pipefail
ticket() { python3 -c "import json; print(json.load(open('/tmp/ticket.json'))['data']['ticket'])"; }
snap() {
  curl -sk -b "PVEAuthCookie=$(ticket)" \
    "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/qemu/300/status/current" \
    | grep -o '"diskwrite":[0-9]*\|"diskread":[0-9]*\|"cpu":[0-9.]*\|"uptime":[0-9]*'
}
echo "--- before ---"
snap
sleep 10
echo "--- after (10s later) ---"
snap
