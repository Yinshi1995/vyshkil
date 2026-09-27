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
викликає `expect_context::<sea_orm::DatabaseConnection>()`. Це працює, тому що
`server/src/main.rs` реєструє `db.clone()` через контекст-замикання в
`leptos_routes_with_context`. Якщо додаєш новий тип контексту в `app` — онови це замикання
в `server/src/main.rs`, інакше отримаєш паніку `expect_context` у рантаймі.

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

## Плановані модулі (Етап 1+, docs/spec) — доповнювати сюди в міру появи коду

- `app::normalize` — єдина функція нормалізації синонімів (`docs/spec/01-domain-model.md` §2,
  рішення [[alias-normalization-in-app]]): нижній регістр, латинські↔кириличні двійники, прибрати
  лапки/дужки, уніфікувати дефіс/тире. Без залежностей поза WASM — компілюється і для SSR, і для
  frontend. Порівняння між сирим і канонічним значенням — по результату цієї функції (`alias.norm`).
- `app::validation` — правила валідації з `docs/spec/03-import-validation.md` §5, спільні для форми
  введення (`02`) і превʼю імпорту (`03`) — та сама функція викликається і в WASM (реальний час), і
  на сервері (фіксація/імпорт).
- `app::dates` — парсинг дат у довільних форматах (`dd.mm`, діапазони, рік з контексту `as_of_date`) —
  `docs/spec/03-import-validation.md` §4.
- `server::policy` — єдиний модуль перевірки прав (`admin`/`org_editor`/`viewer`,
  `docs/spec/04-reconciliation-notifications.md` §6). **Кожна** server function викликає його першою
  (`server/CLAUDE.md`) — ніяких перевірок прав в іншому місці.
- Формат `reported_*` ↔ канон (`training_group` тощо) — зіставлення за ключем "відправник + вид +
  (ВОС|курс|програма) + місце + початок ±3 дні" (`04-reconciliation-notifications.md` §2); правило
  визначення канонічного значення — одна функція, `training_group.canonical_from` фіксує крок.
