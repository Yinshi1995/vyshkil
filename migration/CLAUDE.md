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
