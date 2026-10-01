#!/usr/bin/env bash
set -euo pipefail
ticket() { python3 -c "import json; print(json.load(open('/tmp/ticket.json'))['data']['ticket'])"; }
csrf() { python3 -c "import json; print(json.load(open('/tmp/ticket.json'))['data']['CSRFPreventionToken'])"; }
curl -sk -b "PVEAuthCookie=$(ticket)" -H "CSRFPreventionToken: $(csrf)" -X DELETE \
  "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/qemu/300?purge=1"
echo
