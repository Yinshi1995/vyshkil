# Taktoblik (Vyshkil)

Облік заходів підготовки військових частин УВ(с) "Південь": збір даних (форма/таблиця/звернення) →
нормалізація → звірка між рівнями підпорядкування → виявлення розбіжностей → звітні документи.
Домен — `docs/spec/README.md` (джерело істини; код узгоджується з ним, не навпаки).

Стек: Axum + React 19 (SPA, Vite + TypeScript + shadcn/ui) + SeaORM + PostgreSQL.
Workspace: `app/`, `frontend/`, `server/`, `migration/`, `web/` (React SPA).
Leptos-код (`app/`, `frontend/`) лишається як fallback — сервер автоматично використовує React
SPA, якщо `web/dist/` існує (`server/src/spa.rs`).

## Жорсткі правила

- Людей поіменно не зберігати й не логувати — тільки кількості груп.
- Жодних мережевих викликів з даними назовні (OCR, шрифти, CDN — локально); виняток — знеособлене WhatsApp-повідомлення.
- Нічого не перезаписувати без історії: періоди дії, події, `audit_log`.
- Спершу специфікація, потім код: відступ від `docs/spec` = спершу оновити spec + рішення в `.claude/decisions/`.
- `source_files/` — дані замовника з грифом ДСК. **Читати дозволено** (рішення користувача,
  2026-09-28 — [[source-files-read-access]]) для видобування довідникових/сід-даних, але:
  не комітити самі файли, не цитувати сирий текст дослівно в комітах/логах/відповідях — лише
  агреговані/нормалізовані похідні (як `docs/source-analysis.md`); тести з ними — skip за
  відсутності.
- Права — тільки через модуль `policy`.
- Спільна логіка (нормалізація, парсинг дат, валідація) — в `app`, компілюється і для SSR, і для WASM.

## Куди йти за контекстом

| Питання | Джерело |
|---|---|
| Що зроблено зараз / що в процесі / на якому етапі roadmap | `.claude/memory/MEMORY.md` |
| Чому так, а не інакше (архітектурні рішення) | `.claude/memory/DECISIONS.md` → `.claude/decisions/*.md` |
| Контракт між app/frontend/server | `.claude/memory/INTERFACES.md` |
| Домен і вимоги (сутності, форми, імпорт, звірка, документи, roadmap) | `docs/spec/` — див. таблицю в `docs/spec/README.md` |
| Наступний крок | `docs/spec/06-roadmap.md` (поточний етап — у MEMORY.md) |
| Де лежить код / куди класти новий | `app/src/CLAUDE.md` (дерево рішень), `docs/spec/07-code-structure.md` |
| Дані джерел і граблі імпорту | `docs/source-analysis.md` |
| "хто що викликає" / залежності символів | `cargo modules dependencies` — перед grep по всьому проєкту |
| Конкретна логіка в конкретному файлі | читай файл напряму |
| Стилізація компонента (класи, кольори, варіанти) | skill `styling` (`.claude/skills/styling/SKILL.md`) — не `style/grammar_data.rs` напряму |
| Інфраструктура (dev-VM на Proxmox, майбутній прод) | `infra/README.md`, `docs/spec/10-dev-vm.md`, `docs/spec/11-prod-deploy.md` |

## Автономний режим (tmux Remote Control на dev-VM, нічний раннер)

Якщо ця сесія запущена автономно (без живого користувача за терміналом — tmux "vyshkil-dev"
через Remote Control, або `infra/night-runner/run.sh`): читай `docs/GOALS.md` на старті, бери
ПЕРШУ незакриту ціль. Заблокований на рішенні, яке не твоє (дані/credential/відступ від spec) →
записуй у `docs/QUESTIONS.md` і переходь далі, НЕ зупиняй весь цикл. Коротко оновлюй
`docs/STATUS.md` (поточна ціль, стан) — не лог усіх подій, лише зараз. `[x]` у GOALS.md — лише
після зеленого `cargo test`/e2e, не "на слово". `git push` лишається заблокованим
(`.claude/settings.json`) і в автономному режимі теж — коміти локальні, живий користувач сам
вирішує, коли пушити. Решта правил цього файлу (жорсткі правила вище, спершу-spec) — без винятків.

## Запуск і перевірка (коротко — деталі в MEMORY.md)

```bash
# 1. Postgres
docker start taktoblik-db          # fresh: docker run — див. MEMORY.md

# 2. Frontend (React SPA)
cd web && npm ci && npx vite build  # → web/dist/

# 3. Server
cargo build -p server
LEPTOS_SITE_ROOT="target/site" ./target/debug/server
```
Відкрити `http://localhost:3000`.

### Docker Compose (повний стек)

```bash
docker compose up --build           # db + nats + app + notifier
```
