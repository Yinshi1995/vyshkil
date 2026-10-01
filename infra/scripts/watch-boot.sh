#!/usr/bin/env bash
set -uo pipefail
ticket() { python3 -c "import json; print(json.load(open('/tmp/ticket.json'))['data']['ticket'])"; }
for i in $(seq 1 32); do
  T=$(ticket)
  S=$(curl -sk -b "PVEAuthCookie=${T}" "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/qemu/300/status/current" \
    | grep -o '"diskwrite":[0-9]*\|"uptime":[0-9]*\|"cpu":[0-9.]*\|"qmpstatus":"[a-z]*"')
  A=$(curl -sk -b "PVEAuthCookie=${T}" "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/qemu/300/agent/network-get-interfaces")
  echo "[$i] $(echo $S | tr '\n' ' ') | agent: $A"
  if echo "$A" | grep -q '"result"'; then
    echo "AGENT_UP"
    exit 0
  fi
  sleep 15
done
echo TIMEOUT
exit 1
