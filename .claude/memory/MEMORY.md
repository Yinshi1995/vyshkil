---
tags: [status]
date: 2026-09-27
---

# Поточний стан проєкту

## Зроблено
- Workspace: `app`, `frontend`, `server`, `migration` (Cargo workspace, resolver 2).
- Схема БД (5 таблиц): `unit`, `personnel`, `exercise`, `training_session`, `metric_result` — 5 файлів міграцій у `migration/src/`.
- Axum + Leptos SSR wiring: `server/src/main.rs` (AppState, leptos_routes_with_context, fallback file_and_error_handler).
- mimalloc як глобальний аллокатор, ручний tokio-рантайм (`SERVER_WORKER_THREADS` з env).
- sea-orm ConnectOptions з обмеженим пулом (`DATABASE_MAX_CONNECTIONS`, дефолт 5).
- `.env` (локальний, не в git) + `.env.example`; Dockerfile (rust:slim builder → distroless/cc runtime).
- `#[server] health_check` у `app/src/app.rs` — бере DatabaseConnection з контексту.
- **Перевірено наживо і в Docker, і локально на Windows**: SSR-рендер + `health_check` реально
  повертає "з'єднано", усі 5 міграцій застосовуються при старті сервера.
- Виправлено 2 реальні баги: імпорт `AutoReload`/`HydrationScripts` йде з `leptos::prelude::*`,
  не з `leptos_meta`; `axum` потребував фічу `macros` для `#[derive(FromRef)]` на `AppState`.

## Стан ЗАРАЗ (на момент запису)
- **Postgres**: контейнер `taktoblik-db` (звичайний `docker run`, НЕ через docker-compose) — **запущений**.
  Дані: `postgres://taktoblik:taktoblik@localhost:5432/taktoblik`.
- **Сервер**: **НЕ запущений** (`target/debug/server.exe` завершився без помилки — просто зупинили/
  закрився разом із сесією, у логах нема жодного error). Треба перезапустити вручну (див. нижче).
- `docker ps -a` покаже ще й `vyshkil-app-1`/`vyshkil-db-1` (Exited) — це залишки ПОВНОГО стеку
  через `docker compose up` (кореневий `docker-compose.yml`, збирає образ і теж працює, але
  довше — там своя збірка). Це НЕ те саме, що `taktoblik-db`. Не плутати. Обидва підходи робочі,
  зараз використовується легкий (`taktoblik-db` + локальний бінарник), бо швидше для розробки.

## Як підняти сервер (локально, без Docker для самого застосунку)
```powershell
# 1. Docker Desktop має бути запущений (сам рушій, не тільки іконка) — перевір docker.exe version.
& "C:\Users\eremit\AppData\Local\Programs\DockerDesktop\resources\bin\docker.exe" start taktoblik-db
# якщо контейнера нема — створити наново:
# docker run -d --name taktoblik-db -e POSTGRES_USER=taktoblik -e POSTGRES_PASSWORD=taktoblik -e POSTGRES_DB=taktoblik -p 5432:5432 postgres:16-alpine

# 2. Сервер (.env у корені вже налаштований на localhost:5432)
cd C:\Users\eremit\Projects\vyshkil
$env:LEPTOS_SITE_ROOT = "target/site"
.\target\debug\server.exe
# або, якщо код змінювався: cargo leptos build   (потім так само запустити .exe напряму)
```
Відкрити `http://localhost:3000`.

## Важливі граблі цієї сесії (щоб не наступати знову)
- **Windows Smart App Control** блокував виконання свіжоскомпільованих/незнайомих бінарників —
  і компіляцію (build-scripts), і сам запуск `server.exe`. Користувач свідомо вимкнув
  (`HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy!VerifiedAndReputablePolicyState = 0`) і
  **перезавантажив машину** — без ребута компіляція запрацювала, а запуск exe — ні (Code Integrity
  кешується в ядрі при старті ОС). Див. [[smart-app-control-blocks-local-build]]. Якщо після
  чергового перезавантаження Windows блок повернувся (Microsoft теоретично може повернути захист
  автоматично) — це системна, не кодова проблема; фолбек — Docker (`docker build .` завжди працює,
  бо в лінукс-контейнері цієї політики нема).
- **`cargo leptos watch` зависає** на кроці "Creating ignore list from '.gitignore' file"
  (файловий watcher над великим `target/` на Windows). НЕ використовуй watch — роби
  `cargo leptos build` один раз і запускай `target/debug/server.exe` напряму.
- **Git Bash `ps aux` показує НЕ справжні Windows PID** для нативних .exe в деяких випадках —
  якщо треба вбити процес по PID, перевіряй через `Get-CimInstance Win32_Process` (PowerShell),
  інакше можна вбити не той процес і лишити зомбі-процеси працювати у фоні.
- Стороння папка `source_files/` у корені проєкту (Дельта/Зразок, docx/pptx) — не наша, з іншого
  інструменту/skill, що писав у той самий робочий каталог. Не чіпали, не видаляли.

## Далі (коли повернемось)
- `cargo install cargo-modules` (граф коду, [[cargo-modules-over-graphify]]) ще не встановлено.
- Реальних фіч поки нема — тільки health-check сторінка. Наступний крок за смислом проєкту —
  CRUD для `unit`/`personnel`/`exercise`/`training_session`/`metric_result` через Leptos server functions.

Деталі рішень — у [[decisions-index]] ([DECISIONS.md](DECISIONS.md)). Контракти між крейтами — у [INTERFACES.md](INTERFACES.md).
