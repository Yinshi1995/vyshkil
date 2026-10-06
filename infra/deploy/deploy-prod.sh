#!/usr/bin/env bash
# Автодеплой на САМІЙ прод-VM (docs/spec/11-prod-deploy.md §5.1, .claude/decisions/prod-autodeploy-pull.md).
#
# systemd-таймер (roles/prod_autodeploy) щохвилини: новий коміт у GitHub main → збірка образів тут
# (обмежений buildx-builder, щоб не задушити робочий стек) → бекап БД → :latest (старі → :previous)
# → compose up → перевірка → відкат при збої. Від GitHub потрібен лише `git fetch` публічного
# репо — ні Actions, ні реєстру, ні машини розробника.
#
# Ручний запуск (root): vyshkil-deploy [--force] [<sha>]
set -euo pipefail

REPO_URL="${REPO_URL:-https://github.com/Yinshi1995/vyshkil.git}"
BRANCH="${BRANCH:-main}"
STATE_DIR="${STATE_DIR:-/var/lib/vyshkil-deploy}"
PROD_DIR="${PROD_DIR:-/opt/vyshkil}"
HEALTH_URL="${HEALTH_URL:-http://10.90.0.10:3000/}"
BUILDER="${BUILDER:-vyshkil-builder}"
KEEP_BACKUPS=14
SRC="$STATE_DIR/src"

FORCE=0
if [[ "${1:-}" == "--force" ]]; then FORCE=1; shift; fi
WANT_SHA="${1:-}"

mkdir -p "$STATE_DIR"
exec 9>"$STATE_DIR/lock"
flock -n 9 || exit 0

log() { echo "[$(date '+%F %T')] $*"; }
compose() { docker compose --project-directory "$PROD_DIR" "$@"; }

# --- 1. Що деплоїти ---
[[ -d "$SRC/.git" ]] || git clone -q "$REPO_URL" "$SRC"
if ! git -C "$SRC" fetch -q origin "$BRANCH" 2>"$STATE_DIR/fetch.err"; then
  # GitHub/мережа тимчасово недоступні — чекаємо наступного тіку, у журнал лише при зміні помилки.
  cmp -s "$STATE_DIR/fetch.err" "$STATE_DIR/fetch.err.last" 2>/dev/null \
    || { log "git fetch: $(head -c 300 "$STATE_DIR/fetch.err")"; cp "$STATE_DIR/fetch.err" "$STATE_DIR/fetch.err.last"; }
  exit 0
fi
rm -f "$STATE_DIR/fetch.err.last"
SHA="$(git -C "$SRC" rev-parse "${WANT_SHA:-origin/$BRANCH}^{commit}")"
SHORT="${SHA:0:8}"
LAST="$(cat "$STATE_DIR/deployed-sha" 2>/dev/null || true)"
FAILED="$(cat "$STATE_DIR/failed-sha" 2>/dev/null || true)"
if [[ $FORCE -eq 0 ]]; then
  [[ "$SHA" == "$LAST" || "$SHA" == "$FAILED" ]] && exit 0
fi
log "деплой $SHORT ($(git -C "$SRC" log -1 --format=%s "$SHA"))"

fail() { log "ПОМИЛКА: $*"; echo "$SHA" > "$STATE_DIR/failed-sha"; exit 1; }

# --- 2. Збірка (один повтор на випадок мережі apt/npm; хвіст логу — у журнал) ---
git -C "$SRC" checkout -q --force --detach "$SHA"
git -C "$SRC" clean -qfdx
build() {
  local tag=$1 ctx=$2 logf="$STATE_DIR/build.log"
  for attempt in 1 2; do
    docker buildx build --builder "$BUILDER" --load -t "$tag" "$ctx" >"$logf" 2>&1 && return 0
    log "збірка $tag: спроба $attempt невдала"
  done
  tail -25 "$logf" | sed 's/^/    /'
  return 1
}
T0=$(date +%s)
build "vyshkil-app:$SHORT" "$SRC" || fail "збірка app"
build "vyshkil-notifier:$SHORT" "$SRC/services/notifier" || fail "збірка notifier"
log "зібрано за $(( ($(date +%s) - T0) / 60 )) хв"

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
  docker tag "$n:$SHORT" "$n:latest"
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

# --- 6. Прибирання: лишаються :latest, :previous і поточний sha; кеш builder-а — до 10 ГБ ---
docker images --format '{{.Repository}}:{{.Tag}}' \
  | grep -E '^vyshkil-(app|notifier):[0-9a-f]{8}$' | grep -v ":$SHORT\$" \
  | xargs -r docker rmi >/dev/null 2>&1 || true
docker image prune -f >/dev/null 2>&1 || true
docker buildx prune --builder "$BUILDER" -f --keep-storage 10gb >/dev/null 2>&1 || true
