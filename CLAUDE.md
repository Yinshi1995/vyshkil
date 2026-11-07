# Taktoblik (Vyshkil)

Облік заходів підготовки військових частин УВ(с) "Південь": збір даних (форма/таблиця/звернення) →
нормалізація → звірка між рівнями підпорядкування → виявлення розбіжностей → звітні документи.
Домен — `docs/spec/README.md` (джерело істини; код узгоджується з ним, не навпаки).

Стек: Axum + React 19 (SPA, Vite + TypeScript + shadcn/ui) + SeaORM + PostgreSQL.
Workspace: `app/` (серверна бібліотека: домен, SQL, права), `server/`, `migration/`, `contracts/`, `bus/`, `web/` (React SPA).
Leptos-інтерфейс видалено (`.claude/decisions/remove-leptos-fallback.md`); сервер віддає `web/dist/` (`server/src/spa.rs`) і REST.

## Жорсткі правила

- Людей поіменно не зберігати й не логувати — тільки кількості груп. **Виняток (рішення 2026-11-07):** модуль `personnel` (звання, прізвище, ім'я, дата народження до подій групи) — код є, але **вимкнений** прапорцем `PERSONNEL_ENABLED` до завершення юридичної перевірки; увімкнений чи ні — імена ніколи не в логах, `audit_log`-тексті, NATS/WhatsApp, URL і документах ([[personnel-named-persons]]).
- Жодних мережевих викликів з даними назовні (OCR, шрифти, CDN — локально); виняток — знеособлене WhatsApp-повідомлення.
- Нічого не перезаписувати без історії: періоди дії, події, `audit_log`.
- Спершу специфікація, потім код: відступ від `docs/spec` = спершу оновити spec + рішення в `.claude/decisions/`.
- `source_files/` — дані замовника з грифом ДСК. **Читати дозволено** (рішення користувача,
  2026-09-28 — [[source-files-read-access]]) для видобування довідникових/сід-даних, але:
  не комітити самі файли, не цитувати сирий текст дослівно в комітах/логах/відповідях — лише
  агреговані/нормалізовані похідні (як `docs/source-analysis.md`); тести з ними — skip за
  відсутності.
- Перед `git push` у `main` (автодеплой на прод) — прогін скіла `qa`: `cargo test` + e2e + `dogfood` по зачеплених екранах; червоне або NOT CHECKED = не пушити, а чесно сказати користувачеві. Фронтенд-зміни — ще й обхід сторінок при 1600 і 390 px. Скіли: `qa`, `dogfood` (дослідницьке), `qa-uat` (UAT-звіт).
- Права — тільки через модуль `policy`.
- Спільна логіка (нормалізація, парсинг дат, валідація) — в `app` (`domain/`); клієнтські копії для React — у `web/src/lib`, тримати узгодженими.

## Куди йти за контекстом

| Питання | Джерело |
|---|---|
| Що зроблено зараз / що в процесі / на якому етапі roadmap | `.claude/memory/MEMORY.md` |
| Чому так, а не інакше (архітектурні рішення) | `.claude/memory/DECISIONS.md` → `.claude/decisions/*.md` |
| Контракт між app/frontend/server | `.claude/memory/INTERFACES.md` |
| Домен і вимоги (сутності, форми, імпорт, звірка, документи, roadmap) | `docs/spec/` — див. таблицю в `docs/spec/README.md` |
| Наступний крок | `docs/spec/06-roadmap.md` (поточний етап — у MEMORY.md) |
| Де лежить код / куди класти новий | `app/src/CLAUDE.md`, `docs/spec/07-code-structure.md` (розділи про Leptos застарілі) |
| Дані джерел і граблі імпорту | `docs/source-analysis.md` |
| "хто що викликає" / залежності символів | `cargo modules dependencies` — перед grep по всьому проєкту |
| Конкретна логіка в конкретному файлі | читай файл напряму |
| QA-прогін / ревʼю інтерфейсу / UX перед викаткою | skills `qa`, `dogfood`, `qa-uat` (`.claude/skills/`) |
| Інфраструктура (dev-VM на Proxmox, майбутній прод) | `infra/README.md`, `docs/spec/10-dev-vm.md`, `docs/spec/11-prod-deploy.md` |

## Автономний режим (tmux Remote Control на dev-VM, нічний раннер)

Якщо ця сесія запущена автономно (без живого користувача за терміналом — tmux "vyshkil-dev"
через Remote Control, або `infra/night-runner/run.sh`): читай `docs/GOALS.md` на старті, бери
ПЕРШУ незакриту ціль. Заблокований на рішенні, яке не твоє (дані/credential/відступ від spec) →
записуй у `docs/QUESTIONS.md` і переходь далі, НЕ зупиняй весь цикл. Коротко оновлюй
`docs/STATUS.md` (поточна ціль, стан) — не лог усіх подій, лише зараз. `[x]` у GOALS.md — лише
після зеленого `cargo test`/e2e, не "на слово". `git push` у `origin main` дозволено
й в автономному режимі (рішення користувача 2026-10-06, `.claude/settings.json`) — але пуш у
`main` = автодеплой на прод (`docs/spec/11-prod-deploy.md` §5.1), тож пушити лише після зеленого
`cargo test`/e2e, ніколи `--force`. Решта правил цього файлу (жорсткі правила вище, спершу-spec) — без винятків.

## Швидкий цикл розробки (Linux/WSL, пересборка ≈ 6 с)

```bash
./dev.sh            # сервер: watchexec перезбирає й перезапускає сам, автовхід admin (лише локально)
./dev.sh front      # фронтенд: Vite dev з проксі на сервер
./dev.sh personnel  # те саме, що перший, але з увімкненим модулем "особи поіменно"
```
Холодна збірка один раз ≈ 3 хв, далі правка в `app/` чи `server/` ≈ 6 с. Windows напряму не збирається (`LNK1120`) — працюйте з WSL
(rustup + `mold` + `watchexec` + локальний Postgres; деталі у шапці `dev.sh`). Тести: `cargo test --workspace` з
`TEST_DATABASE_URL=postgres://taktoblik:taktoblik@127.0.0.1:5432/taktoblik_test` (≈ 1 хв); e2e — `e2e/` проти стенда
з `PASSWORD_LOGIN=1 DEV_AUTOLOGIN=admin` (`BASE_URL=... npx playwright test`).

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
