#!/usr/bin/env bash
# Інвентаризація Proxmox-хоста для docs/spec/10-dev-vm.md §1.
# ТІЛЬКИ ЧИТАЄ: нічого не створює, не змінює і не перезапускає.
# Секрети (паролі, ключі шифрування, токени) у вивід не потрапляють — маскуються.
#
# Запуск на PVE-хості від root:
#   bash pve-inventory.sh
# Результат: /root/pve-inventory-<дата>.txt — його вміст передати агенту.

set -u
OUT="/root/pve-inventory-$(date +%Y%m%d-%H%M).txt"
exec > >(tee "$OUT") 2>&1

section() { printf '\n==================== %s ====================\n' "$*"; }
run() { printf '\n$ %s\n' "$*"; eval "$@" 2>&1 || echo "(команда недоступна або завершилась з помилкою)"; }
mask() { sed -E 's/^(\s*(password|encryption-key|keyring|key|token|secret)\s+).*/\1***замасковано***/I'; }

section "0. Хост"
run "hostname -f"
run "date -Is"
run "uptime"

section "1. Версія PVE і кластер"
run "pveversion"
run "pveversion -v | grep -E 'proxmox-ve|pve-manager|pve-kernel|proxmox-kernel|qemu-server|pve-qemu|cloud-init'"
if [ -f /etc/pve/corosync.conf ]; then
  run "pvecm status"
  run "pvecm nodes"
else
  echo "Кластер: НІ (одиночний вузол — /etc/pve/corosync.conf відсутній)"
fi
run "pvesh get /nodes --output-format text"

section "2. Доступ до API (тільки імена, без секретів)"
run "pveum user list --output-format text"
run "pveum user token list root@pam --output-format text"
for u in $(pveum user list --output-format json 2>/dev/null | grep -oE '"userid":"[^"]+"' | cut -d'"' -f4); do
  t=$(pveum user token list "$u" --output-format json 2>/dev/null)
  [ -n "$t" ] && [ "$t" != "[]" ] && echo "Токени є у користувача: $u"
done
run "pveum role list --output-format text | head -40"
run "ss -tlnp | grep -E ':8006|:22 '"

section "3. Storage"
run "pvesm status"
printf '\n$ /etc/pve/storage.cfg (секрети замасковано)\n'
mask < /etc/pve/storage.cfg 2>/dev/null || echo "(нема доступу)"
run "zpool list 2>/dev/null || echo 'ZFS не використовується'"
run "zfs list -o name,used,avail,mountpoint 2>/dev/null | head -30"
run "lvs 2>/dev/null"
run "vgs 2>/dev/null"

section "3. Бекапи"
printf '\n$ /etc/pve/jobs.cfg\n'; cat /etc/pve/jobs.cfg 2>/dev/null || echo "(файла нема — запланованих задач бекапу нема)"
printf '\n$ /etc/pve/vzdump.cron\n'; cat /etc/pve/vzdump.cron 2>/dev/null || echo "(нема)"
if grep -q '^pbs:' /etc/pve/storage.cfg 2>/dev/null; then echo "Proxmox Backup Server: ПІДКЛЮЧЕНО"; else echo "Proxmox Backup Server: НЕ підключено"; fi

section "4. Шаблони й образи"
run "qm list"
echo; echo "--- Шаблони VM (template: 1):"
found=0
for conf in /etc/pve/qemu-server/*.conf /etc/pve/nodes/*/qemu-server/*.conf; do
  [ -f "$conf" ] || continue
  if grep -q '^template: 1' "$conf"; then
    found=1
    id=$(basename "$conf" .conf)
    echo "ID $id: $(grep -E '^(name|ostype|cores|memory|scsi0|virtio0|ide2|ciuser|ipconfig0|net0)' "$conf" | tr '\n' ' ')"
  fi
done
[ $found -eq 0 ] && echo "Шаблонів VM нема"
echo; echo "--- ISO, cloud-образи і шаблони контейнерів у storage:"
for s in $(pvesm status --output-format json 2>/dev/null | grep -oE '"storage":"[^"]+"' | cut -d'"' -f4); do
  l=$(pvesm list "$s" --content iso 2>/dev/null; pvesm list "$s" --content import 2>/dev/null; pvesm list "$s" --content vztmpl 2>/dev/null)
  [ -n "$l" ] && { echo "[$s]"; echo "$l"; }
done
run "ls -la /var/lib/vz/template/iso/ 2>/dev/null"

section "5. Ресурси хоста"
run "lscpu | grep -E 'Model name|^CPU\\(s\\)|Thread|Core|Socket|Virtualization'"
run "free -h"
run "cat /proc/meminfo | grep -E 'MemTotal|MemAvailable|SwapTotal|HugePages_Total'"
run "df -h -x tmpfs -x devtmpfs -x overlay"
run "lsblk -o NAME,SIZE,TYPE,ROTA,MODEL,MOUNTPOINT"
echo; echo "--- Скільки вже виділено запущеним VM/контейнерам:"
run "pvesh get /cluster/resources --type vm --output-format text"

section "6. Мережа"
printf '\n$ /etc/network/interfaces\n'; cat /etc/network/interfaces 2>/dev/null
run "ls /etc/network/interfaces.d/ && cat /etc/network/interfaces.d/* 2>/dev/null"
run "ip -br addr"
run "ip route"
run "bridge vlan show 2>/dev/null | head -30"
printf '\n$ /etc/resolv.conf\n'; cat /etc/resolv.conf
run "cat /etc/pve/sdn/*.cfg 2>/dev/null || echo 'SDN не налаштовано'"

section "6. Файрвол PVE"
run "pve-firewall status"
printf '\n$ /etc/pve/firewall/cluster.fw\n'; cat /etc/pve/firewall/cluster.fw 2>/dev/null || echo "(нема)"

section "8-9. Доступність з хоста (лише перевірка з'єднання, жодних даних не відправляється)"
for url in https://index.crates.io https://static.crates.io https://static.rust-lang.org \
           https://registry.npmjs.org https://github.com https://objects.githubusercontent.com \
           https://registry-1.docker.io https://production.cloudflare.docker.com \
           https://api.anthropic.com https://deb.nodesource.com https://archive.ubuntu.com \
           https://deb.debian.org https://cloud-images.ubuntu.com https://playwright.azureedge.net \
           https://web.whatsapp.com; do
  code=$(curl -s -o /dev/null -m 8 -w '%{http_code}' "$url" 2>/dev/null)
  [ -z "$code" ] || [ "$code" = "000" ] && code="нема з'єднання"
  printf '%-45s %s\n' "$url" "$code"
done
[ -n "${http_proxy:-}${https_proxy:-}" ] && echo "Проксі на хості: http_proxy=${http_proxy:-} https_proxy=${https_proxy:-}"
run "cat /etc/apt/apt.conf.d/*proxy* 2>/dev/null || echo 'apt-проксі не налаштовано'"

section "Готово"
echo "Звіт збережено: $OUT"
echo "Перевір очима, що в ньому нема того, чим не хочеш ділитись, і передай вміст агенту."
