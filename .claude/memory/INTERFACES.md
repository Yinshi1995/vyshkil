---
tags: [interfaces, contracts]
date: 2026-09-27
---

# Контракти між крейтами

## `app` → `frontend`
`frontend` залежить від `app` з фічею `hydrate`. Єдина точка входу:
`app::app::App` (компонент). `frontend/src/lib.rs::hydrate()` викликає
`leptos::mount::hydrate_body(App)`. Не додавай у `app` нічого, що тягне
за собою `sea-orm` поза `#[cfg(feature = "ssr")]`-гілкою — інакше зламається wasm-збірка.

## `app` → `server`
`server` залежить від `app` з фічею `ssr`. Використовує:
- `app::app::shell(LeptosOptions) -> impl IntoView` — HTML-обгортка для SSR і для fallback-хендлера помилок.
- `app::app::App` — корінь дерева маршрутів, передається в `generate_route_list` / `leptos_routes_with_context`.

**Контракт по контексту:** будь-яка `#[server]`-функція в `app`, якій потрібен доступ до БД,
викликає `expect_context::<sea_orm::DatabaseConnection>()` (або `backend::db::connection()`,
той самий виклик). Це працює, тому що `server/src/main.rs` реєструє `db.clone()` через
контекст-замикання в `leptos_routes_with_context`. Якщо додаєш новий тип контексту в `app` —
онови це замикання в `server/src/main.rs`, інакше отримаєш паніку `expect_context` у рантаймі.
`server` більше не знає про `policy` — той тепер повністю в `app` (див. нижче), `server` лишається
тонким (`main.rs`, `config.rs`, `db.rs`-пул, `state.rs`), як і вимагає `docs/spec/07-code-structure.md`.

## Права: `app::backend::policy` (не `server`)

`#[server]`-функції живуть у крейті `app` (цього вимагає Leptos, щоб клієнт міг їх викликати),
а `server` залежить від `app`, не навпаки — тому `server::policy` був для них недосяжний, і
жодна server fn права не перевіряла (`docs/spec/07-code-structure.md` §5, виправлено).
`server/src/policy.rs` видалено; логіка — в `app/src/backend/policy.rs` (лише під `feature = "ssr"`).

**Контракт передачі актора.** У Stage 1 немає реальної автентифікації — клієнт тримає
`RwSignal<Option<Actor>>` у контексті (перемикач у шапці, `layout::ActorSwitcher`). Кожна
`#[server]`-функція, що читає доменні дані одного/кількох органів, приймає `actor: Option<Actor>`
**першим параметром** і сама викликає `backend::policy` (жодних перевірок в UI-компонентах):
- `actor: None` → трактується як "нікого не обрано" — функція повертає порожній результат
  (`search_orgs`, `get_subordination_tree`) або помилку `"оберіть актора вгорі"` (`get_org_detail`,
  де порожній результат був би непомітною відмовою).
- Одна ціль (`get_org_detail`) → `policy::can_view_org(db, actor, org_id)`, `false` → помилка.
- Багато рядків (`search_orgs`, `get_subordination_tree`) → `policy::visible_org_ids(db, actor)`
  (`None` = admin, без обмежень; `Some(ids)` — список видимих org_id), потім фільтрація/обрізання
  результату (`Vec::retain`/`policy::restrict_tree` — не SQL, `repo/` про політику нічого не знає).
- Виняток: `services::orgs::list_orgs` **не** приймає актора — він живить сам перемикач (треба
  вміти обрати організацію ДО того, як стаєш актором), повертає лише `id`+`short_name`, без
  чутливих даних.

**`backend::db`** — `connection()` (те саме, що `expect_context::<DatabaseConnection>()`) і
`actor_transaction(actor) -> DatabaseTransaction` (транзакція з виставленим `SET LOCAL app.actor`
через `policy::set_session_actor` — для майбутніх server fn, що ПИШУТЬ в аудійовані таблиці;
жодна поточна server fn ще не пише, тому цей хелпер поки не викликається, але вже є).

## `server` → `migration`
`server` викликає `migration::Migrator::up(&db, None)` при старті (`server/src/main.rs::run()`).
Додавання нової таблиці = новий файл `migration/src/mYYYYMMDD_*.rs` + реєстрація в
`migration::Migrator::migrations()` (`migration/src/lib.rs`) у порядку FK-залежностей
(спочатку батьківські таблиці, потім ті, що на них посилаються).

## `server::state::AppState`
`#[derive(FromRef)]`-структура з полями `leptos_options: LeptosOptions` і `db: DatabaseConnection`.
Слугує єдиним джерелом стану для Axum-роутера — і Leptos SSR, і майбутні звичайні Axum-хендлери
(наприклад REST-ендпоінти поза Leptos) мають діставати БД через `State<AppState>`, а не заводити
власне з'єднання.

## Реалізовані модулі (доповнювати в міру появи коду)

- `app::domain::normalize::normalize(&str) -> String` — єдина функція нормалізації синонімів
  (`docs/spec/01-domain-model.md` §2, рішення [[alias-normalization-in-app]],
  [[normalize-direction-one-way]]): нижній регістр, латинські→кириличні двійники (один напрямок),
  прибрати лапки/дужки, уніфікувати дефіс/тире, "в/с". Без залежностей поза WASM. Табличні тести
  на реальних граблях у самому файлі (`app/src/domain/normalize.rs`).
- `app::types::actor::{Actor, Role}` — чистий тип (org_id + роль), без sea-orm/tokio, з
  `Serialize`/`Deserialize` (їздить як параметр `#[server]`-функцій). Використовується і клієнтом
  (`layout::ActorSwitcher`, `RwSignal<Option<Actor>>` через `provide_context` в `app::app::App`),
  і сервером (`backend::policy` реекспортує ці ж типи — `pub use crate::types::actor::{Actor, Role};`).
  Якщо міняєш поля `Actor`/варіанти `Role` — онови обидва боки одразу.
- `app::services::orgs::list_orgs()` — `#[server]`-функція, читає `org` напряму SQL через
  `backend::repo::orgs::list_orgs`. Повертає `(id, "назва (номер)")` для перемикача актора;
  **не** перевіряє права (живить сам вибір актора, див. розділ вище).
- `app::backend::policy` — `can_view_org`/`visible_org_ids` (через `subordination_closure`),
  `can_edit_org`, `restrict_tree` (обрізає `OrgTreeRow` до видимої множини), `session_tag`/
  `set_session_actor` (для `audit_log.actor`). Викликається з `pages/home/server.rs` (search,
  tree) і `pages/org_detail/server.rs` (детальна картка) — перша реальна перевірка прав у проєкті.

## Плановані модулі (Етап 1+, docs/spec) — доповнювати сюди в міру появи коду

- `app::validation` — правила валідації з `docs/spec/03-import-validation.md` §5, спільні для форми
  введення (`02`) і превʼю імпорту (`03`) — та сама функція викликається і в WASM (реальний час), і
  на сервері (фіксація/імпорт).
- `app::dates` — парсинг дат у довільних форматах (`dd.mm`, діапазони, рік з контексту `as_of_date`) —
  `docs/spec/03-import-validation.md` §4.
- Формат `reported_*` ↔ канон (`training_group` тощо) — зіставлення за ключем "відправник + вид +
  (ВОС|курс|програма) + місце + початок ±3 дні" (`04-reconciliation-notifications.md` §2); правило
  визначення канонічного значення — одна функція, `training_group.canonical_from` фіксує крок.
