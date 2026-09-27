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
