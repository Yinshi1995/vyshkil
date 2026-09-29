# app/src/backend — лише сервер (весь модуль під `#[cfg(feature = "ssr")]`)

- Можна: `sea-orm`, `types`, `domain`. Не можна: `leptos` view-код (`#[component]`), `pages`,
  `services`, `layout` — ця тека нічого не знає про UI, лише дані й права.
- SQL пишемо тільки в `repo/`; перевірку прав — тільки в `policy.rs` (07 §2.4).
- `import/` — виняток із "можна лише sea-orm/types/domain": розбір файлів (Етап 5) не торкається
  БД взагалі (`calamine`, не `sea-orm`) — це навмисно ЧИСТА структурна функція (байти → рядки),
  щоб бути тестованою без БД (`app/tests/import_fah.rs`); резолюція в БД — окремим кроком у `repo/`.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `db.rs` | `connection()` (DatabaseConnection з контексту), `actor_transaction()` (транзакція + `SET LOCAL app.actor`) | server fn, що пишуть в аудійовані таблиці (перший приклад — `pages/dictionaries`) |
| `policy.rs` | `Actor`/`Role` (реекспорт), `can_view_org`/`visible_org_ids`, `can_edit_org`, `is_admin`, `restrict_tree`, `set_session_actor` | `pages/*/server.rs`, `services/` |
| `repo/` | SQL/SeaORM по агрегатах (своя карта) | `pages/*/server.rs`, `services/` |
| `import/` | Структурні екстрактори файлів, без БД (своя карта) | `repo/imports_*.rs` |
| `documents/` | Генератори документів (Етап 7, 05), байти з готових даних, без БД (своя карта) | `pages/documents/server.rs` |
