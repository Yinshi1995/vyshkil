---
tags: [decision, build, frontend, cleanup]
date: 2026-11-06
---

# Видалити Leptos-інтерфейс (fallback), лишити REST + React SPA

**Чому:** React SPA (`web/`) повністю замінила Leptos UI; сервер віддає `web/dist/` і REST (`server/src/api.rs`).
Leptos-код ніхто не використовував, але його компіляція (сотні транзитивних крейтів: `leptos`, `leptos_axum`, `tachys`,
`wasm-bindgen`…) тримала повільну збірку, а ~12 тис. рядків мертвого UI й дубль `#[server]`-функцій ускладнювали
підтримку (права в двох місцях — REST і server fn).

**Що видалено:** `app/src/{pages,components,layout,widgets,hooks,app.rs,routes.rs}`, `#[server]`-функції в
`app/src/services/*` (лишився `auth.rs` — хелпери аварійного парольного входу й ліміт спроб), крейти `style`,
`style_macros`, `frontend`, `app/style`, `app/assets`, залежності `leptos*`/`wasm-bindgen`/`web-sys`/`js-sys`, налаштування
`[workspace.metadata.leptos]`; `main.rs` більше не має fallback-рендера (SSR), `AppState` без `LeptosOptions`.

**Що лишилось:** `app` тепер серверна бібліотека (домен, `backend`, `types`, `services::auth`); `LEPTOS_SITE_ADDR`
лишається ім'ям змінної адреси (не міняємо compose/прод-env). Правило "спільна логіка компілюється й для WASM" знято.

**Відкат:** `git revert` коміту чистки; Leptos-код лишається в історії (тег/гілка перед видаленням: `pre-remove-leptos`).

**Ціна:** немає fallback-UI, якщо `web/dist/` відсутній — сервер віддає лише API й попереджає в логах.
