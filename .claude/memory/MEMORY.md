---
tags: [status]
date: 2026-09-28
---

# Поточний стан проєкту

## Етап: 1, усі явні критерії роадмапу виконані (за `docs/spec/06-roadmap.md`)

Етап 0 виконано повністю. Етап 1 — усі три "Готово, коли" з роадмапу перевірено (тепер і
автотестом, не тільки вручну): перехід 17 АК→7 КШР, правило 110 омбр, пошук 152НЦ/а4896/польша.
Перехід до Етапу 2 — рішення користувача (див. кінець розмови), ще не підтверджено.

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

**Нечіткий пошук організацій (02 §3)** — `#[server] search_orgs` (`app/src/app.rs`) + `<OrgSearch/>`
на головній: `alias.norm` = `normalize(query)` (та сама функція, що й при сіді/введенні) → точний
збіг → потім `pg_trgm` (`%`/`similarity`) з ранжуванням "точний синонім → uses_count → similarity"
(рядок SQL з `ROW_NUMBER() OVER (PARTITION BY org_id ...)`, щоб одна org не дублювалась кількома
своїми alias-рядками). Перевірено live-запитами (не тільки очима): "152НЦ" → 152 нц, "а4896" →
152 нц, "польша" → Республіка Польща, "423 опБпС" → 423 обБпС (exact) + 433/422 як similarity-
фолбек — критерій готовності Етапу 1 з роадмапу виконано.

**Дерево підпорядкування + картка частини** — `get_subordination_tree` (депф=1 з
`subordination_closure`, **не** рекурсивний CTE — server/CLAUDE.md), `<SubordinationTree/>` з
перемикачем осі (штатне/оперативне) і полем дати; `get_org_detail` + `<OrgDetailPage/>` на
`/org/:id` (поточні дані + історія назв/статусів; порожня історія — легітимний стан, поки в
сіді немає жодного перейменування/зміни статусу). Перевірено live-запитами (curl на
хешовані `/api/...` шляхи, витягнуті з `taktoblik.wasm` — див. Граблі): 17 АК 07-20→09-20
29→23 (7 пішли в 7 КШР: 142/154/61/5/92 омбр/ошбр/225 ошп/44 оабр, +1 прийшла: 67 омбр з 20 АК —
чиста різниця 6, як і задокументовано); 110 омбр на operational-осі показує 20 АК (на staff — 17 АК).
Обидва критерії готовності Етапу 1 з роадмапу виконано.

**Інтеграційні тести проти реальної БД** ("Як вести розробку далі" §3) — `app/src/queries.rs`:
запити (`search_orgs`/`subordination_tree`/`org_detail`) винесено з `#[server]`-тіл у `app.rs` в
окремі функції, що беруть `&DatabaseConnection` явним аргументом (без `expect_context`/Leptos-
контексту) — завдяки цьому їх можна тестувати напряму, без сервера й без браузера. Один
`#[tokio::test]` (`queries::ssr::tests::stage1_readiness_scenarios`) перестворює окрему тестову БД
(`DROP`+`CREATE DATABASE`, через maintenance-з'єднання до `postgres`) і ганяє всі міграції
(включно з dev-сідом) з нуля, потім перевіряє всі три Stage-1-критерії одразу: пошук
152НЦ/а4896/польша, 17 АК 29→23 (з точним списком, хто пішов у 7 КШР), 110 омбр — різний
батько на staff/operational. Потребує `TEST_DATABASE_URL`; без нього — skip (той самий підхід,
що й для `source_files/`-тестів). Запуск:
```powershell
$env:TEST_DATABASE_URL = "postgres://taktoblik:taktoblik@localhost:5432/taktoblik_test"
cargo test -p app --features ssr queries
```
`app/Cargo.toml`: sea-orm тепер тягне повний `sqlx-postgres`+`runtime-tokio-rustls` (не лише
`macros`) під `ssr`, щоб `queries`-тести могли самі з'єднуватись з Postgres без сервера.

**НЕ зроблено (залишається на майбутнє, не є критерієм готовності Етапу 1):**
- Аналогічні `cargo test` для `migration`/`policy` (EXCLUDE-констрейнти, closure-тригер,
  `can_view_org`/`can_edit_org`) — поки не потрібні: `policy` ще не викликається жодним CRUD.

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
- **Leptos `#[server(Name, "/api")]` без явного третього аргументу (шляху) генерує URL з дописаним
  compile-time xxhash-суфіксом** (`/api/search_orgs13551936677940061918`, не просто
  `/api/search_orgs`) — тому curl "навмання" на людський шлях завжди дає 404/фолбек-рендер. Щоб
  дізнатись реальний шлях для ручної перевірки: `grep -a -o "/api/[A-Za-z_0-9]*" target/site/pkg/*.wasm`
  (рядок буквально закодований у wasm; `strings` немає в Git Bash на Windows, `grep -a` працює).
- **Git Bash → нативний `curl.exe` на Windows манглить кирилицю в аргументах командного рядка**
  (транскодування через ANSI-кодову сторінку консолі) — `--data-urlencode "query=польша"` іноді
  тихо шле не ті байти (не помилка, просто інший, часто короткий/невірний result). Симптом:
  результат порожній або "invalid utf-8", хоча `echo -n "…" | xxd` показує правильні UTF-8 байти.
  Обхід для ручної перевірки: збирати тіло вручну як percent-encoded ASCII
  (`--data-binary "query=%D0%BF%D0%BE..."`) або писати запит у файл через `printf '\xNN...'` і
  слати `--data-binary @file`.

## RSS сервера (наскрізна вимога — вимірювати після кожного етапу)
Ще не вимірювалось на реальних даних (Етап 1 щойно починається).

Деталі рішень — у `.claude/memory/DECISIONS.md`. Контракти між крейтами — у `.claude/memory/INTERFACES.md`.
Домен — `docs/spec/`. Кінець сесії — `/handoff`. Початок нової — `/next-stage`.
