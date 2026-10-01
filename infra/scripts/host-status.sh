#!/usr/bin/env bash
set -euo pipefail
ticket() { python3 -c "import json; print(json.load(open('/tmp/ticket.json'))['data']['ticket'])"; }
curl -sk -b "PVEAuthCookie=$(ticket)" \
  "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/status" \
  | python3 -c "import json,sys; d=json.load(sys.stdin)['data']; print('mem used/total:', d['memory']['used'], '/', d['memory']['total']); print('swap used/total:', d['swap']['used'], '/', d['swap']['total']); print('loadavg:', d['loadavg']); print('cpu:', d['cpu'])"
