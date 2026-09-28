# app/src/backend — лише сервер (весь модуль під `#[cfg(feature = "ssr")]`)

- Можна: `sea-orm`, `types`, `domain`. Не можна: `leptos` view-код (`#[component]`), `pages`,
  `services`, `layout` — ця тека нічого не знає про UI, лише дані й права.
- SQL пишемо тільки в `repo/`; перевірку прав — тільки в `policy.rs` (07 §2.4).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `db.rs` | `connection()` (DatabaseConnection з контексту), `actor_transaction()` (транзакція + `SET LOCAL app.actor`) | server fn, що пишуть в аудійовані таблиці |
| `policy.rs` | `Actor`/`Role` (реекспорт), `can_view_org`/`visible_org_ids`, `can_edit_org`, `restrict_tree`, `set_session_actor` | `pages/*/server.rs`, `services/` |
| `repo/` | SQL/SeaORM по агрегатах (своя карта) | `pages/*/server.rs`, `services/` |
