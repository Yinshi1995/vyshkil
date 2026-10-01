#!/usr/bin/env bash
set -uo pipefail
for i in $(seq 1 20); do
  (echo > /dev/tcp/192.168.1.200/22) >/dev/null 2>&1 && { echo "OPEN at attempt $i"; exit 0; }
  sleep 3
done
echo TIMEOUT
exit 1
