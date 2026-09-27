---
tags: [status]
date: 2026-09-27
---

# Поточний стан проєкту

## Етап: 1, у процесі (за `docs/spec/06-roadmap.md`)

Етап 0 виконано повністю. Етап 1 — зроблено бекенд-частину, **UI ще нема**:

**Зроблено:**
- 10 міграцій (`m20260927_000001…000010`): `org` (+часткові унікальні індекси на номер ВЧ, окремо
  для `number_kind IS NULL`/`NOT NULL`), `org_name_history` (+EXCLUDE), `training_site`,
  `subordination` (+EXCLUDE по (child_org_id, axis, період)), `org_status` (+EXCLUDE по (org_id,
  період)), `subordination_closure`, `alias` (+pg_trgm GIN), `audit_log` (generic тригер на 6
  таблиць: org/org_name_history/training_site/subordination/org_status/alias — не на
  subordination_closure, вона TRUNCATE+INSERT перебудовується), тригер автоперебудови
  `subordination_closure` при будь-якій зміні `subordination`.
- `app::normalize` — нормалізація синонімів, табличні тести на реальних граблях, рішення
  [[normalize-direction-one-way]] (транслітерація лише Latin→Cyrillic, не навпаки).
- `server::policy` — `Actor`/`Role`, `can_view_org` (через `subordination_closure`),
  `can_edit_org`, `set_session_actor`. **Ще не викликається** з жодної server function.
- Перевірено наживо: перехід 6 частин 17 АК → 7 КШР у серпні (різні набори підлеглих на 20.07/20.09);
  110 омбр одночасно в 17 АК (staff) і 20 АК (operational) без конфлікту; усі EXCLUDE/unique
  constraint реально відхиляють дублі/перетини (перевірено `psql` в транзакціях з ROLLBACK).

**НЕ зроблено (критерії готовності Етапу 1, ще не виконані):**
- UI: перемикач актора в шапці, дерево підпорядкування на дату (з перемикачем осі), картка частини.
- Сід реальних даних з `source_files/` (номери частин, підпорядкування, синоніми).
- Тест "пошук 152НЦ/а4896/польша знаходить канонічні організації" — потребує UI/server function
  над `alias`+`pg_trgm`, ще не написано.
- `cargo test` для міграцій/policy як автоматизовані тести (зараз перевірено вручну через psql —
  вартувало б перенести у справжні integration-тести, окрема тестова БД, див. правило #3 в
  "Як вести розробку далі").

## Каркас (не змінюється до кінця проєкту)
- Workspace: `app`, `frontend`, `server`, `migration` (Cargo workspace, resolver 2).
- Axum + Leptos SSR wiring: `server/src/main.rs` (AppState, leptos_routes_with_context, fallback file_and_error_handler).
- mimalloc як глобальний аллокатор, ручний tokio-рантайм (`SERVER_WORKER_THREADS` з env).
- sea-orm ConnectOptions з обмеженим пулом (`DATABASE_MAX_CONNECTIONS`, дефолт 5).
- `.env` (локальний, не в git) + `.env.example`; Dockerfile (rust:slim builder → distroless/cc runtime).
- `#[server] health_check` у `app/src/app.rs` — бере DatabaseConnection з контексту (лишиться як smoke-test).
- Docker-збірка і локальний запуск (Windows) перевірені наживо (SSR + БД + міграції) до переходу на спек.

## Стан ЗАРАЗ
- **Postgres**: контейнер `taktoblik-db` — **перестворений з нуля** (стара схема видалена разом
  із контейнером). Дані: `postgres://taktoblik:taktoblik@localhost:5432/taktoblik`.
- **Міграції**: 10 застосовано (org/…/audit_log, префікс `m20260927_…`), схема Етапу 1 в БД жива.
- **Сервер**: не запущений (треба перезапустити вручну — `cargo leptos build` + `target/debug/server.exe`).
- `source_files/` — **дані замовника** (архів старого обліку + еталонні документи для золотих
  тестів і сідів), не сторонній інструмент. З грифом ДСК: не комітити, не цитувати в логах,
  `Read`-заборонено в `.claude/settings.json`. Деталі й граблі — `docs/source-analysis.md`.
- `docker ps -a` може показати ще й `vyshkil-app-1`/`vyshkil-db-1` (Exited, з `docker compose up` —
  повний стек із власним білдом) — залишки з попередньої сесії, не використовуються зараз.

## Як підняти локально
```powershell
& "C:\Users\eremit\AppData\Local\Programs\DockerDesktop\resources\bin\docker.exe" start taktoblik-db
cd C:\Users\eremit\Projects\vyshkil
cargo leptos build            # НЕ watch — див. граблі нижче
$env:LEPTOS_SITE_ROOT = "target/site"
.\target\debug\server.exe
```

## Граблі (щоб не наступати знову)
- **Windows Smart App Control** блокував компіляцію і запуск свіжих бінарників — вимкнено
  (`VerifiedAndReputablePolicyState = 0`) і машину перезавантажено; тепер працює локально без
  Docker. Якщо колись повернеться — `docs/spec` не постраждає, фолбек: `docker build .`.
- **`cargo leptos watch` зависає** на "Creating ignore list from '.gitignore' file" (великий
  `target/`, повільна FS Windows) — використовуй `cargo leptos build` + прямий запуск `.exe`.
- Git Bash `ps aux` іноді бреше про PID нативних .exe — для kill за PID перевіряй через
  `Get-CimInstance Win32_Process` (PowerShell), інакше лишаються зомбі-процеси.
- `.claudeignore` **не діє** в цій версії Claude Code (перевірено емпірично) — видалено;
  контроль контексту через `permissions.deny` у `.claude/settings.json`.

## RSS сервера (наскрізна вимога — вимірювати після кожного етапу)
Ще не вимірювалось на реальних даних (Етап 1 щойно починається).

Деталі рішень — у `.claude/memory/DECISIONS.md`. Контракти між крейтами — у `.claude/memory/INTERFACES.md`.
Домен — `docs/spec/`. Кінець сесії — `/handoff`. Початок нової — `/next-stage`.
