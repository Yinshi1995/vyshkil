# app/src/services — `#[server]`-функції, потрібні ≥ 2 місцям

- Можна: `backend`, `types`, `domain`. Не можна: `pages`, `layout`, `widgets` (напрямок
  залежностей — services нижче за pages/widgets, 07 §2.3).
- Функція, яку використовує лише одна сторінка → переносити в `pages/<p>/server.rs` (07 §2.1).
- `submission_grid.rs` — виняток із "лише #[server] тут": плейн async-функції (не `#[server]`
  самі), тому й модуль ganтиться `#[cfg(feature = "ssr")]` у `mod.rs`, а не через тіла функцій
  (як решта файлів тут) — деталі в коментарі над оголошенням модуля.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `auth.rs` | `auth_login`/`auth_logout`/`get_current_user`/`get_user_roles`/`switch_actor` — автентифікація, cookie-сесія (12-auth.md); `require_auth()`/`resolve_actor()` (ssr) — серверне забезпечення прав, `resolve_actor` використовують інші server fn замість довіри клієнтському актору | `pages/login`, `layout`, `services/*`, `pages/*/server.rs` |
| `health.rs` | `health_check` — пінг БД, smoke-test | `pages/home` |
| `orgs.rs` | `list_orgs` (без прав), `search_orgs` (нечіткий пошук, звужений до видимого піддерева) | `layout::ActorSwitcher`, `pages/home`, `widgets/group_grid` |
| `dictionaries.rs` | `get_dictionaries_overview` — усі прості довідники Етапу 2 одним викликом | `widgets/group_grid` |
| `groups.rs` | `search_vos_position_course`, `get_training_sites` (02 §3, §1 колонка 5) | `widgets/group_grid` |
| `submission_grid.rs` | `get_draft_impl`/`save_draft_impl`/`commit_grid_impl` — спільна логіка чернетки/фіксації (form/table) | `pages/training_form/server.rs`, `pages/import/server.rs` |
| `notifications.rs` | `get_notifications`/`mark_read` — серверні функції для сповіщень (Етап 8 зріз 4, 04 §5) | `layout`, `pages/*` |
