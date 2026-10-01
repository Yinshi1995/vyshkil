#!/usr/bin/env bash
set -uo pipefail
ticket() { python3 -c "import json; print(json.load(open('/tmp/ticket.json'))['data']['ticket'])"; }
for i in $(seq 1 24); do
  T=$(ticket)
  S=$(curl -sk -b "PVEAuthCookie=${T}" "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/qemu/300/status/current" \
    | grep -o '"diskwrite":[0-9]*\|"uptime":[0-9]*\|"qmpstatus":"[a-z]*"')
  A=$(curl -sk -b "PVEAuthCookie=${T}" "https://192.168.1.106:8006/api2/json/nodes/pidhotovka/qemu/300/agent/network-get-interfaces" | grep -o '"result"')
  SSH="closed"
  (echo > /dev/tcp/192.168.1.200/22) >/dev/null 2>&1 && SSH="OPEN"
  echo "[$i] $(echo $S | tr '\n' ' ') | agent_result: ${A:-none} | ssh22: $SSH"
  if [ "$SSH" = "OPEN" ] || [ -n "$A" ]; then
    echo "REACHABLE"
    exit 0
  fi
  sleep 15
done
echo TIMEOUT
exit 1
