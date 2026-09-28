# app/src/domain — чиста логіка без UI і БД, компілюється у WASM

- Можна: нічого доменно-специфічного поза `std`/`serde`. Не можна: `leptos`, `sea_orm`, `tokio`,
  файлова система — усе тут мусить збиратись під `wasm32-unknown-unknown`.
- Юніт-тести — табличні, на реальних граблях із `docs/source-analysis.md`, у тому ж файлі.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `normalize.rs` | `normalize(&str) -> String` — єдина функція нормалізації синонімів (лапки/дужки/дефіс/"в-с", Latin→Cyrillic) | `backend/repo/orgs.rs` (пошук), майбутній `app::validation` |
