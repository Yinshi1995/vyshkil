# app/src/pages/dictionaries — довідники Етапу 2 + learned-синоніми (`/dictionaries`)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під `ssr`).
  Не можна: інші сторінки.
- `server.rs` оголошений без `pub` (`mod server;`) — приватний для цієї сторінки.
- Перегляд довідників — не org-scoped (як `list_orgs`), без `policy`. Черга learned-синонімів і
  підтвердження/відхилення — лише `admin` (`policy::is_admin`), перевіряється на сервері.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `DictionariesPage` — список довідників + `LearnedAliasQueue` | `pages/CLAUDE.md` → `routes.rs` |
| `server.rs` | `get_learned_aliases`, `confirm_learned_alias`, `reject_learned_alias` (`get_dictionaries_overview` переїхав у `services/dictionaries.rs`, потрібен ще й `training_form`) | `mod.rs` |
