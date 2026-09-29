# app/src/backend/repo — SQL/SeaORM по агрегатах, без прав

- Можна: `sea-orm`, `types`, `domain`. Не можна: `policy` (репозиторій не перевіряє права —
  це робить виклик у `pages/*/server.rs`/`services/` ПІСЛЯ отримання даних або через
  `policy::visible_org_ids`/`can_view_org` ДО), `leptos`.
- Кожна функція бере `&DatabaseConnection` явним аргументом (без `expect_context`) — тому
  тестована напряму з `app/tests/<агрегат>.rs`, без сервера й без Leptos-контексту.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `orgs.rs` | `list_orgs`, `search_orgs`, `resolve_org` (найкращий кандидат, Етап 5), `subordination_tree`, `org_detail` — SQL по `org`/`alias`/`subordination_closure`/`org_name_history`/`org_status` | `services/orgs.rs`, `pages/home/server.rs`, `pages/org_detail/server.rs`, `repo/imports_fah.rs`, `app/tests/orgs.rs`, `app/tests/policy.rs` |
| `dictionaries.rs` | `equipment_vos_hint`, `dictionaries_overview`, `learned_aliases`, `confirm_learned_alias`, `reject_learned_alias`, `resolve_vos_by_code`/`resolve_position`/`training_kind_id_by_code` (Етап 5) | `pages/vos_lookup/server.rs`, `pages/dictionaries/server.rs`, `repo/imports_fah.rs` |
| `groups.rs` | `group_events` (читання), `search_vos_position_course`/`training_site_options` (02 §3, §1), `validate_row`/`commit_group_rows` (фіксація сітки, 02 §5), `find_or_create_training_site` (Етап 5) | `services/groups.rs`, `repo/imports_fah.rs`, `app/tests/groups.rs` |
| `submissions.rs` | `latest_draft_for_org`/`save_draft`/`mark_committed` — чернетки (`submission`, 02 §6), `source_type` параметром (`'form'`/`'table'`) | `services/submission_grid.rs` |
| `imports_fah.rs` | `resolve_rows` — `RawFahRow` → `GroupFormRow`, резолюція org/vos/посада/місце через довідники (Етап 5) | `pages/import/server.rs` |
