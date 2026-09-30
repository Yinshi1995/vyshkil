# migration/

- Перша міграція має вмикати розширення `btree_gist` і `pg_trgm`.
- Enum-подібні значення — довідкові таблиці або `text` + `CHECK`, **не** Postgres `ENUM` (додавання
  значення не повинно вимагати міграції типу).
- Кожна доменна таблиця — під generic тригер `audit_log` (один тригер на всі, не окремий на таблицю).
- Soft delete скрізь, де сутність може "зникнути", але має лишитись у БД — `deleted_at`.
- Періоди дії (`subordination`, `org_name_history` тощо) — `valid_from`/`valid_to` +
  `EXCLUDE USING gist` на діапазон, щоб періоди для одного ключа не перетинались.
- Реєструвати міграції в `Migrator::migrations()` за FK-залежностями (батьківські таблиці перед тими,
  що на них посилаються) — інакше `Migrator::up` впаде на чистій базі.
- Префікс файлу — реальна дата (`m20260927_…`), не вигадана.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `lib.rs` | `Migrator::migrations()` — реєстр усіх міграцій за FK-порядком | `server/src/main.rs`, `app/tests/*.rs` |
| `m…_000001…000010` | схема Етапу 1: `org`, `org_name_history`, `training_site`, `subordination`, `org_status`, `subordination_closure`, `alias`, `audit_log`+тригер, тригер перебудови замикання | — |
| `m…_000011_seed_dev_data` | dev-сід (42 org, 71 alias) зі specи/аналізу джерел, НЕ з `source_files/` | локальна розробка, `app/tests/*.rs` |
| `m…_000012…027` | Етапи 2-3: довідники підготовки, `vos`/`equipment_vos` (79 кодів), `training_group`, `group_event` | — |
| `m…_000028_create_submission_table` | `submission` (мінімум під Етап 4: form/draft_payload, без reported_*/file_id — ті з Етапу 5) | `backend/repo/submissions.rs` |
| `m…_000029_add_submission_id_to_group_event` | `group_event.submission_id` (nullable FK) | — |
| `m…_000030_attach_audit_trigger_submission` | audit-тригер на `submission` | — |
| `m…_000031_extend_training_group_for_stage4` | `training_group` += equipment_text/basis_doc_number/basis_doc_date/inflow_source (відкладено Етапом 3, потрібно сітці) | — |
| `m…_000032_create_group_composition_table` | `group_composition` (розподіл за підрозділами, опційно) | `backend/repo/groups.rs` |
| `m…_000033_attach_audit_trigger_group_composition` | audit-тригер на `group_composition` | — |
| `m…_000034_create_staffing_tables` | `staffing_snapshot`+`staffing_metric` (01 §4, Етап 5 — КВід/ІВС) | `backend/repo/imports_kvid.rs` |
| `m…_000035_attach_audit_triggers_staffing` | audit-тригери на обидві | — |
| `m…_000036_refine_subordination_dates_from_kontrolka` | Етап 6: реальні дати переходу 17 АК→7 КШР з Контролька замість умовної 01.08.2026 у dev-сіді (сам факт переходу не змінюється) | — |
| `m…_000037_create_generated_document_table` | `generated_document` (05 §вступ, мінімум під Етап 7/D1: kind/org_id/as_of_date/file_path) | `backend/repo/documents.rs` |
| `m…_000038_widen_generated_document_kind` | Розширює CHECK на `generated_document.kind`: `'d1'` → `'d1','d2'` (Postgres CHECK — лише DROP+ADD, не ALTER) | — |
