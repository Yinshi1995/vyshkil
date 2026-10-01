#!/usr/bin/env bash
set -euo pipefail
curl -sk -d "username=root@pam" --data-urlencode "password=${1}" \
  https://192.168.1.106:8006/api2/json/access/ticket > /tmp/ticket.json
cat /tmp/ticket.json
echo
