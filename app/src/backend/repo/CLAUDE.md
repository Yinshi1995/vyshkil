# app/src/backend/repo — SQL/SeaORM по агрегатах, без прав

- Можна: `sea-orm`, `types`, `domain`. Не можна: `policy` (репозиторій не перевіряє права —
  це робить виклик у `pages/*/server.rs`/`services/` ПІСЛЯ отримання даних або через
  `policy::visible_org_ids`/`can_view_org` ДО), `leptos`.
- Кожна функція бере `&DatabaseConnection` явним аргументом (без `expect_context`) — тому
  тестована напряму з `app/tests/<агрегат>.rs`, без сервера й без Leptos-контексту.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `orgs.rs` | `list_orgs`, `search_orgs`, `resolve_org` (Етап 5-6: суворіше за `search_orgs` — ведучий номер частини в запиті МАЄ збігтись у кандидата, інакше `None`, бо pg_trgm-схожість сама не розрізняє "17 овмбр"/"128 овмбр"), `subordination_tree`, `org_detail` — SQL по `org`/`alias`/`subordination_closure`/`org_name_history`/`org_status` | `services/orgs.rs`, `pages/home/server.rs`, `pages/org_detail/server.rs`, `repo/imports_*.rs`, `app/tests/orgs.rs`, `app/tests/policy.rs` |
| `dictionaries.rs` | `equipment_vos_hint`, `dictionaries_overview`, `learned_aliases`, `confirm_learned_alias`, `reject_learned_alias`, `resolve_vos_by_code`/`resolve_position`/`resolve_course`/`training_kind_id_by_code` (Етап 5) | `pages/vos_lookup/server.rs`, `pages/dictionaries/server.rs`, `repo/imports_*.rs` |
| `groups.rs` | `group_events` (читання), `search_vos_position_course`/`training_site_options` (02 §3, §1), `validate_row`/`commit_group_rows` (фіксація сітки, 02 §5), `find_or_create_training_site` (Етап 5) | `services/groups.rs`, `repo/imports_*.rs`, `app/tests/groups.rs` |
| `submissions.rs` | `latest_draft_for_org`/`save_draft`/`mark_committed` — чернетки (`submission`, 02 §6), `source_type` параметром (`'form'`/`'table'`) | `services/submission_grid.rs` |
| `imports_fah.rs` | `resolve_rows` — `RawFahRow` → `GroupFormRow`, резолюція org/vos/посада/місце через довідники (Етап 5) | `pages/import/server.rs` |
| `imports_bps.rs` | `resolve_rows` — `RawBpsRow` → `GroupFormRow`, org/site через `org.number` (точніше за фах-текст) | `pages/import/server.rs` |
| `imports_kvid.rs` | `resolve_rows` — `RawKvidRow` → `StaffingRow`, лише org резолюція | `pages/import/server.rs` |
| `imports_ivs.rs` | `resolve_rows` — `IvsExtract` → `(InstructorStaffingRow, GroupFormRow)` РАЗОМ: стажування+курси йдуть у ту саму сітку, що й Фах/БпС | `pages/import/server.rs` |
| `imports_terminy.rs` | `resolve_rows` — `TerminyExtract` (3 паралельні списки) → `GroupFormRow`, той самий Grid, різні `training_kind` (bzvp/special/adaptation) | `pages/import/server.rs` |
| `imports_vch_archive.rs` | `resolve_rows` — Етап 6, `RawVchArchiveRow` → `GroupFormRow`, `training_kind='special'`, "Місце проведення" — вільний географічний текст (часто нерозпізнане) | `pages/import/server.rs` |
| `staffing.rs` | `insert_snapshot`/`insert_instructor_snapshot` — `staffing_snapshot`+`staffing_metric` (01 §4, різні набори метрик КВід/ІВС) | `pages/import/server.rs` |
