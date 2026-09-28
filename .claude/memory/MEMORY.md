---
tags: [status]
date: 2026-09-27
---

# Поточний стан проєкту

## Етап: 1, у процесі (за `docs/spec/06-roadmap.md`)

Етап 0 виконано повністю. Етап 1 — бекенд + перший шматок UI:

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
- `app::actor::{Actor, Role}` + `server::policy` (реекспортує ті самі типи) — `can_view_org`
  (через `subordination_closure`), `can_edit_org`, `set_session_actor`. **Логіка перевірки прав
  ще не викликається** з жодної server function (нема ще жодної CRUD-операції, яку захищати).
- **UI, перший шматок**: `Header` + `ActorSwitcher` у шапці (`app::app::list_orgs()` читає `org`
  напряму SQL, без entity), `RwSignal<Option<Actor>>` через контекст. Коректно показує порожній
  стан, поки `org` пустий ("немає організацій — сід ще не завантажено").
- **Глобальний стиль — 3-тя і фінальна ітерація**: перша спроба взяла палітру документів
  (05-documents.md), друга — приблизні кольори з пікселів pptx. Третя — **виміряно з живого сайту
  striy.pp.ua** (Playwright `getComputedStyle`, та сама організація УВ(с) "Південь"): фон `#0D0F0A`,
  текст `#E8E4D8`, єдиний акцент-золото `#C9A84C`, шрифти **Oswald** (заголовки/nav, uppercase) +
  **Roboto** (тіло) — self-hosted `.woff2` (`app/assets/fonts/`, `assets-dir` в Cargo.toml,
  без CDN — CLAUDE.md вимагає локальні шрифти). Компоненти: `.icon-box` (квадрат, золота рамка,
  inline SVG), `.card`, `.btn`/`.btn--outline` (скошений край через `clip-path`), `.eyebrow`.
  Деталі й точні виміри — `.claude/decisions/ui-visual-style-source.md`. Перевірено Playwright-
  скріншотом — консоль чиста, шрифти реально завантажуються (`/fonts/*.woff2`, HTTP 200).
- **Побічний фікс**: `file_and_error_handler` у `server/src/main.rs` падав з panic
  ("no DatabaseConnection in context") на БУДЬ-ЯКИЙ незнайдений шлях (typo в асеті, бот-скан) —
  бо фолбек-рендер не провайдив контекст, на відміну від основного роутера. Виправлено через
  `render_app_to_stream_with_context`.
- **Dev-сід (`m20260927_000011_seed_dev_data`)**: 42 org, 71 alias — командний ланцюг
  (УВ(с)→17/20 АК, 30 КМП, 7 КШР, ОТУ "Одеса", ЧБП), 22 частини з реальними номерами
  (`docs/source-analysis.md`), сценарій переходу 6 частин 17 АК→7 КШР (серпень 2026),
  110 омбр подвійно (staff 17 АК + operational 20 АК), 152 нц (А4896, приклад з
  `02-input-forms-ux.md`), alias-варіанти (423 обБпС/опБпС, польша→Республіка Польща тощо).
  **Джерело — вже прочитані фрагменти `docs/`, НЕ сирі файли `source_files/`** (ті — Read-заборонені,
  ДСК). Це НЕ Етап 6 (archive_seed) — повний перенос архіву робиться окремо пізніше.
  Перевірено на реальних (не rollback) даних: 29→23 підлеглих 17 АК на 20.07→20.09.2026, точно
  ті самі 6 частин пішли — критерій готовності Етапу 1 виконано.

**НЕ зроблено (критерії готовності Етапу 1, ще не виконані):**
- Дерево підпорядкування на дату (з перемикачем осі staff/operational), картка частини з історією —
  дані для цього вже є (сід), бракує тільки UI/server function.
- Тест "пошук 152НЦ/а4896/польша знаходить канонічні організації" — alias-рядки для цього вже
  засіяні, бракує server function над `alias`+`pg_trgm` для нечіткого пошуку.
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
- **Міграції**: 11 застосовано (org/…/audit_log/dev-seed, префікс `m20260927_…`), схема+сід Етапу 1 в БД живі.
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
- **`permissions.deny` з `Read(glob)` блокує НЕ тільки інструмент Read** — Bash/PowerShell-виклики
  (`ls`, `Get-ChildItem` тощо), що просто згадують шлях під той glob у команді, теж мовчки
  блокуються. Через це `Read(target/**)` зробив хибний висновок, що копіювання асетів у
  `cargo leptos build` не працює (насправді працювало — `-v` показав успіх). Правило: тримай
  deny-глоби вузькими і перевіряй "порожній результат" свіжим викликом, а не одразу вір йому.
  (Той самий клас проблеми раніше стався з `*.zip`.) Запис у deny-списку вже прибрано.

## RSS сервера (наскрізна вимога — вимірювати після кожного етапу)
Ще не вимірювалось на реальних даних (Етап 1 щойно починається).

Деталі рішень — у `.claude/memory/DECISIONS.md`. Контракти між крейтами — у `.claude/memory/INTERFACES.md`.
Домен — `docs/spec/`. Кінець сесії — `/handoff`. Початок нової — `/next-stage`.
