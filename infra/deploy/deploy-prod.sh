#!/usr/bin/env bash
# Автодеплой на САМІЙ прод-VM (docs/spec/11-prod-deploy.md §5.1, .claude/decisions/prod-autodeploy-pull.md).
#
# systemd-таймер (roles/prod_autodeploy) щохвилини: GHCR :main має новий revision →
# бекап БД → pull образів <sha> → :latest → compose up → перевірка → відкат на :previous при збої.
# Образи збирає GitHub Actions (.github/workflows/images.yml); жодна інша машина не потрібна.
#
# Ручний запуск (root): deploy-prod.sh [--force] [<sha>]
set -euo pipefail

REGISTRY="${REGISTRY:-ghcr.io/yinshi1995}"
CHANNEL="${CHANNEL:-main}"
STATE_DIR="${STATE_DIR:-/var/lib/vyshkil-deploy}"
PROD_DIR="${PROD_DIR:-/opt/vyshkil}"
HEALTH_URL="${HEALTH_URL:-http://10.90.0.10:3000/}"
KEEP_BACKUPS=14

FORCE=0
if [[ "${1:-}" == "--force" ]]; then FORCE=1; shift; fi
WANT_SHA="${1:-}"

mkdir -p "$STATE_DIR"
exec 9>"$STATE_DIR/lock"
flock -n 9 || exit 0

log() { echo "[$(date '+%F %T')] $*"; }
compose() { docker compose --project-directory "$PROD_DIR" "$@"; }

# --- 1. Що деплоїти: revision образу :main у GHCR ---
if [[ -n "$WANT_SHA" ]]; then
  SHA="$WANT_SHA"
else
  if ! docker pull -q "$REGISTRY/vyshkil-app:$CHANNEL" >/dev/null 2>"$STATE_DIR/pull.err"; then
    # Мережа/GHCR тимчасово недоступні — тихо чекаємо наступного тіку (в журнал раз на зміну).
    if ! cmp -s "$STATE_DIR/pull.err" "$STATE_DIR/pull.err.last" 2>/dev/null; then
      log "GHCR недоступний: $(head -c 300 "$STATE_DIR/pull.err")"; cp "$STATE_DIR/pull.err" "$STATE_DIR/pull.err.last"
    fi
    exit 0
  fi
  rm -f "$STATE_DIR/pull.err.last"
  SHA="$(docker image inspect -f '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$REGISTRY/vyshkil-app:$CHANNEL")"
fi
[[ "$SHA" =~ ^[0-9a-f]{40}$ ]] || { log "некоректний revision '$SHA'"; exit 1; }
SHORT="${SHA:0:8}"
LAST="$(cat "$STATE_DIR/deployed-sha" 2>/dev/null || true)"
FAILED="$(cat "$STATE_DIR/failed-sha" 2>/dev/null || true)"
if [[ $FORCE -eq 0 ]]; then
  [[ "$SHA" == "$LAST" || "$SHA" == "$FAILED" ]] && exit 0
fi
log "деплой $SHORT"

fail() { log "ПОМИЛКА: $*"; echo "$SHA" > "$STATE_DIR/failed-sha"; exit 1; }

# --- 2. Образи цього коміту ---
for n in vyshkil-app vyshkil-notifier; do
  docker pull -q "$REGISTRY/$n:$SHA" >/dev/null || fail "pull $n:$SHORT"
done

# --- 3. Бекап БД (міграції застосовуються на старті сервера) ---
STAMP="$(date +%Y%m%d-%H%M%S)"
mkdir -p "$PROD_DIR/backups" && chmod 700 "$PROD_DIR/backups"
for db in taktoblik notifier; do
  compose exec -T db pg_dump -U taktoblik -Fc "$db" > "$PROD_DIR/backups/$db-$STAMP-$SHORT.dump" \
    || fail "бекап БД $db"
  ls -1t "$PROD_DIR/backups/$db-"*.dump | tail -n +$((KEEP_BACKUPS + 1)) | xargs -r rm -f
done
log "бекап БД: $PROD_DIR/backups/*-$STAMP-$SHORT.dump"

# --- 4. Перемикання з запасним :previous ---
for n in vyshkil-app vyshkil-notifier; do
  docker tag "$n:latest" "$n:previous" 2>/dev/null || true
  docker tag "$REGISTRY/$n:$SHA" "$n:latest"
done
compose up -d app notifier >/dev/null 2>&1 || true

# --- 5. Перевірка: HTTP 200 і жодної паніки (напр. міграція) ---
ok=0
for _ in $(seq 1 30); do
  sleep 4
  if [[ "$(curl -s -o /dev/null -m 5 -w '%{http_code}' "$HEALTH_URL" || true)" == 200 ]] \
     && ! compose logs --since 3m app 2>/dev/null | grep -q panicked; then ok=1; break; fi
done
if [[ $ok -ne 1 ]]; then
  log "перевірка не пройшла — логи app:"
  compose logs --tail 30 app 2>&1 | sed 's/^/    /' || true
  log "ВІДКАТ на :previous (БД: бекап $STAMP — відновлювати вручну, якщо міграція встигла)"
  for n in vyshkil-app vyshkil-notifier; do docker tag "$n:previous" "$n:latest" || true; done
  compose up -d app notifier >/dev/null 2>&1 || true
  fail "реліз не піднявся, повернуто попередній"
fi

echo "$SHA" > "$STATE_DIR/deployed-sha"
rm -f "$STATE_DIR/failed-sha"
log "готово: $SHORT на проді"

# --- 6. Прибирання: лишаються :latest, :previous і поточний sha ---
docker images --format '{{.Repository}}:{{.Tag}}' \
  | grep -E "^$REGISTRY/vyshkil-(app|notifier):[0-9a-f]{40}$" | grep -v ":$SHA\$" \
  | xargs -r docker rmi >/dev/null 2>&1 || true
docker image prune -f >/dev/null 2>&1 || true
