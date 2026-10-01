#!/usr/bin/env bash
set -euo pipefail
TICKET=$(python3 -c "import json; print(json.load(open('/tmp/ticket.json'))['data']['ticket'])")
CSRF=$(python3 -c "import json; print(json.load(open('/tmp/ticket.json'))['data']['CSRFPreventionToken'])")
echo "--- stop ---"
curl -sk -b "PVEAuthCookie=${TICKET}" -H "CSRFPreventionToken: ${CSRF}" -X POST \
  "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/qemu/300/status/stop"
echo
sleep 10
echo "--- status ---"
curl -sk -b "PVEAuthCookie=${TICKET}" \
  "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/qemu/300/status/current"
echo
