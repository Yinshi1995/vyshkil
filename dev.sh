#!/usr/bin/env bash
# Швидкий цикл розробки (Linux/WSL): сервер перезбирається й перезапускається САМ після правки файлу.
# Після першої збірки (≈3 хв) правка в app/ чи server/ → пересборка ≈ 6 с (інкрементальна dev-збірка + mold).
#
#   ./dev.sh            — сервер на 127.0.0.1:3000 з автовходом admin (ЛИШЕ локальна розробка!)
#   ./dev.sh front      — Vite dev-сервер фронтенду на :4173 з проксі на 127.0.0.1:3000
#   ./dev.sh personnel  — те саме, що без аргументів, але з увімкненим модулем "особи поіменно"
#
# Потрібно: rustup (1.97.1 з rust-toolchain.toml), mold, watchexec, локальний Postgres.
# Перша підготовка БД:  createdb taktoblik_dev  (роль taktoblik/taktoblik, суперкористувач — для pg_trgm).
# Міграції накочуються при старті сервера. Windows напряму не збирається (LNK1120) — використовуйте WSL.
set -euo pipefail
cd "$(dirname "$0")"

case "${1:-server}" in
  front)
    cd web
    [ -d node_modules ] || npm ci
    API_PROXY_TARGET="${API_PROXY_TARGET:-http://127.0.0.1:3000}" exec npx vite --port 4173 --host 127.0.0.1
    ;;
  server|personnel)
    command -v watchexec >/dev/null || { echo "потрібен watchexec (pacman -S watchexec / cargo install watchexec-cli)"; exit 1; }
    export DATABASE_URL="${DATABASE_URL:-postgres://taktoblik:taktoblik@127.0.0.1:5432/taktoblik_dev}"
    export NATS_URL="${NATS_URL:-nats://x:y@127.0.0.1:1}"      # NATS локально не потрібен: relay лише попереджає
    export LEPTOS_SITE_ADDR="${LEPTOS_SITE_ADDR:-127.0.0.1:3000}"
    export PASSWORD_LOGIN=1 DEV_AUTOLOGIN="${DEV_AUTOLOGIN:-admin}"   # без форми входу; на проді з ІКС сервер із цим не стартує
    export RUST_LOG="${RUST_LOG:-info,sqlx=warn}"
    [ "${1:-server}" = "personnel" ] && export PERSONNEL_ENABLED=1
    echo "→ сервер: http://127.0.0.1:3000  (автовхід: $DEV_AUTOLOGIN${PERSONNEL_ENABLED:+, особи поіменно УВІМКНЕНО})"
    exec watchexec --restart --quiet --wrap-process=session \
      --watch app/src --watch server/src --watch migration/src --watch contracts/src --watch bus/src \
      --exts rs,toml -- cargo run -p server
    ;;
  *) echo "використання: ./dev.sh [server|front|personnel]"; exit 2 ;;
esac
