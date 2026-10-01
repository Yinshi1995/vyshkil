#!/usr/bin/env bash
# Нічний раннер (docs/spec/10-dev-vm.md, автономний режим) — запускається systemd-таймером
# 22:00-08:00 (infra/ansible/roles/autonomy). Бере ОДНУ ціль з docs/GOALS.md, працює в ОКРЕМОМУ
# git-worktree (не чіпає робочу копію живої tmux Remote Control сесії), комітить локально
# (НІКОЛИ не пушить — те саме deny-правило .claude/settings.json), пише звіт на ранок.
#
# Свідомо без параметрів — усе через GOALS.md/env нижче, щоб systemd-юніт лишався тривіальним.
set -euo pipefail

REPO="${VYSHKIL_REPO:-$HOME/vyshkil}"
WORKTREE_ROOT="${VYSHKIL_NIGHT_WORKTREE_ROOT:-$HOME/vyshkil-night}"
REPORT_DIR="$REPO/infra/night-runner/reports"
TS="$(date -u +%Y-%m-%dT%H%M%SZ)"
WORKTREE="$WORKTREE_ROOT/$TS"
MAX_RETRIES=2
BACKOFF_SECONDS=30
LOCK="/tmp/vyshkil-night-runner.lock"

mkdir -p "$REPORT_DIR"
REPORT="$REPORT_DIR/$TS.md"

log() { printf '%s %s\n' "$(date -u +%H:%M:%S)" "$1" | tee -a "$REPORT" >&2; }

# Один прогін за раз (systemd timer + можливий ручний запуск одночасно — не хочемо двох
# паралельних worktree/cargo build, що й так тісно на 8 ГБ RAM).
exec 9>"$LOCK"
if ! flock -n 9; then
    echo "Інший прогін нічного раннера вже триває — вихід без помилки." >&2
    exit 0
fi

{
    echo "# Нічний раннер — звіт $TS"
    echo
} > "$REPORT"

# Resource guard: не стартувати, якщо вільної RAM замало (конкурентний build/docker-стек уже
# триває) — краще пропустити ніч, ніж OOM-кілнути щось живе на VM.
AVAILABLE_MB="$(awk '/MemAvailable/ {print int($2/1024)}' /proc/meminfo)"
MIN_AVAILABLE_MB=1500
if [ "$AVAILABLE_MB" -lt "$MIN_AVAILABLE_MB" ]; then
    log "Пропуск: вільно лише ${AVAILABLE_MB}МБ RAM (потрібно ≥${MIN_AVAILABLE_MB}МБ) — щось інше вже вантажить VM."
    exit 0
fi

cleanup() {
    log "Прибирання worktree $WORKTREE"
    git -C "$REPO" worktree remove --force "$WORKTREE" 2>/dev/null || true
}
trap cleanup EXIT

log "Створення ізольованого worktree з гілки main: $WORKTREE"
git -C "$REPO" worktree add -b "night/$TS" "$WORKTREE" main >>"$REPORT" 2>&1

NEXT_GOAL="$(grep -m1 '^- \[ \]' "$WORKTREE/docs/GOALS.md" || true)"
if [ -z "$NEXT_GOAL" ] || printf '%s' "$NEXT_GOAL" | grep -q 'порожньо'; then
    log "docs/GOALS.md не має незакритої конкретної цілі — нічого робити, вихід."
    exit 0
fi
log "Ціль цього прогону: $NEXT_GOAL"

attempt=1
success=0
while [ "$attempt" -le "$((MAX_RETRIES + 1))" ]; do
    log "Спроба $attempt/$((MAX_RETRIES + 1))"
    if (
        cd "$WORKTREE" && \
        claude -p "Прочитай docs/GOALS.md, docs/QUESTIONS.md і CLAUDE.md (розділ \"Автономний режим\"). Візьми першу незакриту ціль і виконай її повністю: код → тести → коміт (БЕЗ push). Якщо натрапиш на рішення поза твоєю компетенцією — запиши в docs/QUESTIONS.md і переходь до наступної цілі замість зупинки. Оновлюй docs/STATUS.md короткими записами по ходу." \
            --permission-mode auto \
            >>"$REPORT" 2>&1
    ); then
        success=1
        break
    fi
    log "Спроба $attempt провалилась, backoff ${BACKOFF_SECONDS}с"
    sleep "$BACKOFF_SECONDS"
    attempt=$((attempt + 1))
    BACKOFF_SECONDS=$((BACKOFF_SECONDS * 2))
done

if [ "$success" -eq 1 ]; then
    log "Успішно. Коміти цього прогону (НЕ запушені):"
    git -C "$WORKTREE" log main.."night/$TS" --oneline >>"$REPORT" 2>&1 || true
    log "Злиття night/$TS у main локально (fast-forward якщо можливо, інакше гілка лишається)."
    if git -C "$REPO" merge --ff-only "night/$TS" 2>>"$REPORT"; then
        log "main оновлено локально до night/$TS."
        git -C "$REPO" branch -d "night/$TS" 2>/dev/null || true
    else
        log "Не fast-forward — лишаю гілку night/$TS НЕзлитою для ручного огляду."
    fi
else
    log "Усі спроби провалились — лишаю гілку night/$TS для ручного огляду, main не чіпаю."
fi

log "Звіт: $REPORT"
