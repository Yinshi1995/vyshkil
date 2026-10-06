#!/usr/bin/env bash
# Автодеплой прод-VM з GitHub (docs/spec/11-prod-deploy.md §5.1, .claude/decisions/prod-autodeploy-pull.md).
#
# Живе на dev-VM, запускається systemd-таймером (infra/ansible/roles/prod_autodeploy) щохвилини:
#   новий коміт у origin/main → збірка образів тут (на проді 2 ГБ RAM — Rust не збирається)
#   → бекап БД проду → docker save | ssh docker load → compose up → перевірка → відкат при збої.
# Pull, не push: GitHub нічого не знає про інфраструктуру, вхідних з'єднань і токенів не треба.
#
# Ручний запуск: deploy-prod.sh [--force] [<sha>]
set -euo pipefail

REPO_URL="${REPO_URL:-https://github.com/Yinshi1995/vyshkil.git}"
BRANCH="${BRANCH:-main}"
STATE_DIR="${STATE_DIR:-$HOME/deploy}"
MIRROR="$STATE_DIR/mirror.git"
BUILD_DIR="$STATE_DIR/build"
PROD_KEY="${PROD_KEY:-$HOME/.ssh/vyshkil-prod}"
PROD_HOST="${PROD_HOST:-deploy@10.90.0.10}"
JUMP_HOST="${JUMP_HOST:-vyos@192.168.1.2}"
PROD_DIR=/opt/vyshkil
HEALTH_URL="http://10.90.0.10:3000/"
KEEP_BACKUPS=14

FORCE=0
if [[ "${1:-}" == "--force" ]]; then FORCE=1; shift; fi
WANT_SHA="${1:-}"

mkdir -p "$STATE_DIR"
exec 9>"$STATE_DIR/lock"
flock -n 9 || { echo "інший деплой ще йде — пропускаю"; exit 0; }

log() { echo "[$(date '+%F %T')] $*"; }
SSH_OPTS=(-o BatchMode=yes -o IdentitiesOnly=yes -i "$PROD_KEY" -o StrictHostKeyChecking=accept-new
  -o "ProxyCommand=ssh -i $PROD_KEY -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=accept-new -W %h:%p $JUMP_HOST")
prod() { ssh "${SSH_OPTS[@]}" "$PROD_HOST" "$@"; }
compose() { prod "sudo docker compose --project-directory $PROD_DIR $*"; }

# --- 1. Що деплоїти ---
[[ -d "$MIRROR" ]] || git clone --quiet --mirror "$REPO_URL" "$MIRROR"
git -C "$MIRROR" fetch --quiet --prune origin
SHA="${WANT_SHA:-$(git -C "$MIRROR" rev-parse "refs/heads/$BRANCH")}"
SHA="$(git -C "$MIRROR" rev-parse "$SHA^{commit}")"
SHORT="${SHA:0:8}"
LAST="$(cat "$STATE_DIR/deployed-sha" 2>/dev/null || true)"
FAILED="$(cat "$STATE_DIR/failed-sha" 2>/dev/null || true)"

if [[ $FORCE -eq 0 ]]; then
  [[ "$SHA" == "$LAST" ]] && exit 0                     # нічого нового — тихо
  if [[ "$SHA" == "$FAILED" ]]; then exit 0; fi          # уже падав — чекаємо наступного коміту
fi
log "деплой $SHORT ($(git -C "$MIRROR" log -1 --format=%s "$SHA"))"

fail() {
  log "ПОМИЛКА: $*"
  echo "$SHA" > "$STATE_DIR/failed-sha"
  exit 1
}

# --- 2. Збірка ---
rm -rf "$BUILD_DIR" && mkdir -p "$BUILD_DIR"
git -C "$MIRROR" archive "$SHA" | tar -x -C "$BUILD_DIR"
log "збірка vyshkil-app"
docker build -q -t "vyshkil-app:$SHORT" "$BUILD_DIR" >/dev/null || fail "збірка app"
log "збірка vyshkil-notifier"
docker build -q -t "vyshkil-notifier:$SHORT" "$BUILD_DIR/services/notifier" >/dev/null || fail "збірка notifier"

# --- 3. Бекап БД проду (міграції застосовуються на старті сервера) ---
STAMP="$(date +%Y%m%d-%H%M%S)"
log "бекап БД → $PROD_DIR/backups/taktoblik-$STAMP-$SHORT.dump"
prod "sudo mkdir -p $PROD_DIR/backups && sudo chmod 700 $PROD_DIR/backups && \
  sudo docker compose --project-directory $PROD_DIR exec -T db pg_dump -U taktoblik -Fc taktoblik \
    | sudo tee $PROD_DIR/backups/taktoblik-$STAMP-$SHORT.dump >/dev/null && \
  sudo docker compose --project-directory $PROD_DIR exec -T db pg_dump -U taktoblik -Fc notifier \
    | sudo tee $PROD_DIR/backups/notifier-$STAMP-$SHORT.dump >/dev/null && \
  sudo sh -c 'cd $PROD_DIR/backups && for p in taktoblik notifier; do \
    ls -1t \$p-*.dump | tail -n +$((KEEP_BACKUPS + 1)) | xargs -r rm -f; done'" || fail "бекап БД"
# backups/ — root 0700: ротацію робить root (sudo sh -c), не cd з-під deploy.

# --- 4. Перенесення образів ---
log "перенесення образів на прод"
docker save "vyshkil-app:$SHORT" "vyshkil-notifier:$SHORT" | gzip -1 | prod "sudo docker load -q" >/dev/null \
  || fail "docker load на проді"

# --- 5. Перемикання з запасним :previous ---
prod "sudo docker tag vyshkil-app:latest vyshkil-app:previous 2>/dev/null; \
      sudo docker tag vyshkil-notifier:latest vyshkil-notifier:previous 2>/dev/null; \
      sudo docker tag vyshkil-app:$SHORT vyshkil-app:latest && \
      sudo docker tag vyshkil-notifier:$SHORT vyshkil-notifier:latest" || fail "тегування"
compose "up -d app notifier" >/dev/null 2>&1 || true

# --- 6. Перевірка: HTTP 200 і жодної паніки (напр. міграція) ---
ok=0
for _ in $(seq 1 30); do
  sleep 4
  if prod "curl -s -o /dev/null -m 5 -w '%{http_code}' $HEALTH_URL" 2>/dev/null | grep -q '^200$' \
     && ! compose "logs --since 3m app" 2>/dev/null | grep -q 'panicked'; then ok=1; break; fi
done

if [[ $ok -ne 1 ]]; then
  log "перевірка не пройшла — логи app:"
  compose "logs --tail 30 app" 2>&1 | sed 's/^/    /' || true
  log "ВІДКАТ на :previous (БД — бекап $STAMP, відновлювати вручну, якщо міграція встигла частково)"
  prod "sudo docker tag vyshkil-app:previous vyshkil-app:latest && \
        sudo docker tag vyshkil-notifier:previous vyshkil-notifier:latest" || true
  compose "up -d app notifier" >/dev/null 2>&1 || true
  fail "новий реліз не піднявся, повернуто попередній"
fi

echo "$SHA" > "$STATE_DIR/deployed-sha"
rm -f "$STATE_DIR/failed-sha"
log "готово: $SHORT на проді"

# --- 7. Прибирання старих образів (лишаються :latest, :previous, поточний sha) ---
prod "sudo docker image prune -f >/dev/null; \
  sudo docker images --format '{{.Repository}}:{{.Tag}}' | grep -E '^vyshkil-(app|notifier):[0-9a-f]{8}$' \
    | grep -v ':$SHORT\$' | sudo xargs -r docker rmi >/dev/null 2>&1" || true
docker images --format '{{.Repository}}:{{.Tag}}' | grep -E '^vyshkil-(app|notifier):[0-9a-f]{8}$' \
  | grep -v ":$SHORT\$" | xargs -r docker rmi >/dev/null 2>&1 || true
docker image prune -f >/dev/null 2>&1 || true
