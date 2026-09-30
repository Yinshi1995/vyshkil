# app/src/domain — чиста логіка без UI і БД, компілюється у WASM

- Можна: `chrono` (без `"clock"`/`"wasmbind"`, [[chrono-in-domain]]), `std`, `serde`. Не можна:
  `leptos`, `sea_orm`, `tokio`, файлова система — усе тут мусить збиратись під `wasm32-unknown-unknown`.
- Юніт-тести — табличні, на реальних граблях із `docs/source-analysis.md`, у тому ж файлі.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `normalize.rs` | `normalize(&str) -> String` — єдина функція нормалізації синонімів (лапки/дужки/дефіс/"в-с", Latin→Cyrillic) | `backend/repo/orgs.rs` (пошук) |
| `counting.rs` | Воронка групи з подій: `in_training`/`events_on`/`finishing_on`, бітемпоральний `known_at` | `backend/repo/groups.rs` |
| `dates.rs` | `parse_date`/`parse_maybe_range`/`parse_end_date`/`validate_period` (02 §4, 03 §4) | `pages/training_form` (Етап 4) |
| `validation.rs` | `validate_count`/`validate_funnel_order`/`looks_like_personal_name` (03 §5) | `pages/training_form` (Етап 4) |
| `reconciliation.rs` | `detect_horizontal` — горизонтальна звірка (04 §3, Етап 8 зріз 1): та сама канонічна група, незгодний метрик між поданнями | `backend/repo/reconciliation.rs` |
