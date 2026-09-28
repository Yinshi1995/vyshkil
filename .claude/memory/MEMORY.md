---
tags: [status]
date: 2026-09-28
---

# Поточний стан проєкту

## Етап: 2, у процесі (за `docs/spec/06-roadmap.md`) — Етап 1 повністю закритий

Етап 1 — усі три "Готово, коли" перевірено автотестом: перехід 17 АК→7 КШР, правило 110 омбр,
пошук 152НЦ/а4896/польша. Етап 2 розпочато й **явний критерій готовності вже виконано**:
запит підказки "вамп"→218, "mavic"→217, "fpv"→219, "нрк"→129, "darts"→216 з поясненням "бо …"
(перевірено автотестом `app/tests/dictionaries.rs` і наживо curl).

**НЕ зроблено для повного закриття Етапу 2** (роадмап описує ширше, ніж один критерій):
- Повний словник (91 ВОС, 226 посад, 287 ОВТ) — зараз засіяно тільки 17 перевірених пар з
  `docs/source-analysis.md` §3 (дозволене джерело). Решту роадмап сам каже брати скриптом з
  `A5150 Фах`/`Терміни` в `source_files/` — я їх читати не можу (ДСК); або користувач дає
  скрипт-результат окремо, або чекаємо офіційний перелік від замовника.
- UI: адмінка довідників (CRUD на training_kind/vos/position/equipment/course/…) і черга
  learned-синонімів на підтвердження — з роадмапу, ще не почато.

**Зроблено:**
- 10 міграцій (`m20260927_000001…000010`): `org` (+часткові унікальні індекси на номер ВЧ, окремо
  для `number_kind IS NULL`/`NOT NULL`), `org_name_history` (+EXCLUDE), `training_site`,
  `subordination` (+EXCLUDE по (child_org_id, axis, період)), `org_status` (+EXCLUDE по (org_id,
  період)), `subordination_closure`, `alias` (+pg_trgm GIN), `audit_log` (generic тригер на 6
  таблиць: org/org_name_history/training_site/subordination/org_status/alias — не на
  subordination_closure, вона TRUNCATE+INSERT перебудовується), тригер автоперебудови
  `subordination_closure` при будь-якій зміні `subordination`.
- `app::domain::normalize` — нормалізація синонімів, табличні тести на реальних граблях, рішення
  [[normalize-direction-one-way]] (транслітерація лише Latin→Cyrillic, не навпаки).
- **UI, перший шматок**: `Header` + `ActorSwitcher` у шапці (`app::services::orgs::list_orgs()`
  читає `org` напряму SQL, без entity), `RwSignal<Option<Actor>>` через контекст. Коректно показує
  порожній стан, поки `org` пустий ("немає організацій — сід ще не завантажено").
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

**Нечіткий пошук організацій (02 §3)** — `search_orgs` (`backend::repo::orgs`, викликається з
`pages/home/server.rs`) + `<OrgSearch/>` на головній: `alias.norm` = `normalize(query)` → точний
збіг → потім `pg_trgm` з ранжуванням "точний синонім → uses_count → similarity". Перевірено
live-запитами: "152НЦ" → 152 нц, "а4896" → 152 нц, "польша" → Республіка Польща — критерій
готовності Етапу 1 виконано.

**Дерево підпорядкування + картка частини** — `subordination_tree`/`org_detail`
(`backend::repo::orgs`), `<SubordinationTree/>` (перемикач осі) і `<OrgDetailPage/>` на `/org/:id`.
Перевірено: 17 АК 07-20→09-20 29→23 (7 пішли в 7 КШР, +1 прийшла — 67 омбр з 20 АК, чиста різниця
6); 110 омбр на operational-осі показує 20 АК (на staff — 17 АК). Обидва критерії готовності
Етапу 1 виконано.

**Реорганізація структури коду за `docs/spec/07-code-structure.md`** (новий файл специфікації) —
`app/src` розкладено з трьох плоских файлів (`app.rs`/`queries.rs`/`normalize.rs`) у дерево
`app.rs`+`routes.rs`+`layout/`+`pages/`+`services/`+`types/`+`domain/`+`backend/` (лише `ssr`).
**Заразом виправлено дефект**: `policy` переїхав `server/src/policy.rs` → `app/src/backend/policy.rs`
(був фізично недосяжний для `#[server]`-функцій, бо ті живуть у крейті `app`, а `server` залежить
від `app`, не навпаки) — тепер `search_orgs`/`get_subordination_tree`/`get_org_detail` реально
фільтрують результат через `policy::visible_org_ids`/`can_view_org` за поточним актором (перша
реальна перевірка прав у проєкті; `list_orgs` лишається без перевірки — живить сам вибір актора).
Кожна непорожня тека `app/src/**` має карту `CLAUDE.md` (≤40 рядків); карти не застарівають —
перевіряє `app/tests/architecture.rs` (`cargo test --test architecture`, без БД). CSS-колокацію
з того ж плану **відклали**: cargo-leptos не розгортає `@import` і обробляє стилі до збірки
Rust-крейта — [[css-colocation-deferred]]; `main.css` лишається одним файлом.

**Інтеграційні тести проти реальної БД** ("Як вести розробку далі" §3) — `app/tests/orgs.rs`
(три Stage-1-критерії з таблиці вище) + `app/tests/policy.rs` ("org_editor з 17 АК не бачить
сусіднє 7 КШР і його піддерево"), спільний сетап — `app/tests/common/mod.rs`. Кожен тест
перестворює окрему тестову БД (`DROP`+`CREATE DATABASE`) і ганяє всі міграції (включно з
dev-сідом) з нуля. Потребують `TEST_DATABASE_URL`; без нього — skip. Запуск:
```powershell
$env:TEST_DATABASE_URL = "postgres://taktoblik:taktoblik@localhost:5432/taktoblik_test"
cargo test -p app --features ssr
```
`app/Cargo.toml`: sea-orm тягне повний `sqlx-postgres`+`runtime-tokio-rustls` (не лише `macros`)
під `ssr`, щоб тести самі з'єднувались з Postgres без сервера.

**НЕ зроблено (залишається на майбутнє, не є критерієм готовності Етапу 1):**
- Аналогічні `cargo test` для `migration` (EXCLUDE-констрейнти, closure-тригер) — поки не критично.

**UI-полірування після Кроку 3 (реорганізації)**: без обраного актора пошук/дерево/картка мовчки
показували порожній результат — плутало, виглядало як "нічого не знайдено". Додав `hooks::use_actor`
(централізує `expect_context::<RwSignal<Option<Actor>>>()`, дублювався в 4 місцях) і
`widgets::ActorNotice` ("Оберіть актора вгорі…") — перші реальні файли в цих теках (07-code-
structure.md описував їх наперед, тепер є що покласти).

**Етап 2, частина 1**: схема 10 нових таблиць (`training_kind`, `training_direction`,
`bzvp_program`, `vos`, `position`, `vos_position`, `equipment`, `equipment_vos`, `course`,
`attrition_reason`) + generic audit-тригер на всі. `backend::repo::dictionaries::
equipment_vos_hint` — той самий `alias`+`pg_trgm` патерн, що й Етап 1 `search_orgs`. Нова
сторінка `/vos-lookup` + посилання в шапці. Сід — 17 перевірених пар ОВТ→ВОС з
`docs/source-analysis.md` §3 (НЕ з `source_files/`), назви training_kind/course/attrition_reason/
position — з `01-domain-model.md` §2. `vos.title` відомий лише для 218 (`status='official'`);
решта — чесний плейсхолдер `status='draft'`, не вигадана назва спеціальності.
**Граблі, що варто пам'ятати надалі**: якщо сідуєш `alias` — рахуй `norm` через РЕАЛЬНУ
`app::domain::normalize()` (додав `migration` → `app`-залежність саме для цього), а не SQL
`lower()`. У Етапі 1 це збіглось випадково (org-назви майже всі кириличні, `normalize()` на них —
просто lower()); для Етапу 2 з латиничними ОВТ-термінами ("Mavic", "FPV") розбіжність між
seed-time `lower()` і query-time `normalize()` (яка транслітерує окремі латинські літери в
кириличні — задумано для номерів ВЧ) ламала пошук мовчки. Спіймано тестом, виправлено до коміту.

**НЕ зроблено для Етапу 2** (див. секцію "Етап" вище): повний словник (91 ВОС/226 посад/287 ОВТ,
потребує `source_files/` — ДСК), адмінка довідників, черга learned-синонімів.

`components/`, `state/` з 07-code-structure.md ще не існують (нема чого туди класти).

## Каркас (не змінюється до кінця проєкту)
- Workspace: `app`, `frontend`, `server`, `migration` (Cargo workspace, resolver 2).
- Axum + Leptos SSR wiring: `server/src/main.rs` (AppState, leptos_routes_with_context, fallback file_and_error_handler).
- mimalloc як глобальний аллокатор, ручний tokio-рантайм (`SERVER_WORKER_THREADS` з env).
- sea-orm ConnectOptions з обмеженим пулом (`DATABASE_MAX_CONNECTIONS`, дефолт 5).
- `.env` (локальний, не в git) + `.env.example`; Dockerfile (rust:slim builder → distroless/cc runtime).
- `#[server] health_check` у `app/src/services/health.rs` — бере DatabaseConnection з контексту (лишиться як smoke-test).
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
- **Playwright MCP іноді диз'єднаний на початку сесії** (`CONNECT_TIMEOUT`) — тоді поведінку
  перевіряй curl-запитами на реальні хешовані `/api/...` шляхи (дивись вище) замість браузера;
  цього достатньо для "поведінка не змінилась", але не показує реальний рендер/консоль — якщо
  потрібен саме UI-скріншот, попроси користувача перепідключити MCP.
- **`taktoblik-db` один раз сам зупинився посеред сесії** (`received a fast shutdown request` у
  логах контейнера — НЕ від мене: я не викликав `docker stop`), причина не з'ясована (ймовірно
  Docker Desktop). Симптом: `Connection pool timed out` після 8с на кожному запиті. Якщо таке
  повториться — спершу `docker ps` (не одразу `docker start`, щоб не гадати), тоді `docker start
  taktoblik-db`.

## RSS сервера (наскрізна вимога — вимірювати після кожного етапу)
Ще не вимірювалось на реальних даних (Етап 1 щойно починається).

Деталі рішень — у `.claude/memory/DECISIONS.md`. Контракти між крейтами — у `.claude/memory/INTERFACES.md`.
Домен — `docs/spec/`. Кінець сесії — `/handoff`. Початок нової — `/next-stage`.
