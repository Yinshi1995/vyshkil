# contracts/ — типи повідомлень брокера (09-messaging.md §3.4, Фаза 0)

- Без `tokio`/`sea-orm`/web-специфічних залежностей — компілюється і нативно, і в wasm32
  (`app` читає ці типи так само на клієнті, як на сервері). Нова залежність тут — спершу
  перевір, чи сама компілюється під wasm32-unknown-unknown.
- Зміна контракту = нова версія типу/subject-а (`v2`), стара лишається, поки є споживачі — не
  редагуємо існуючий тип на місці, якщо це ламає вже опубліковані повідомлення.
- Приватність (09 §3.5) — `notify.*`-типи не мають полів вільного тексту, лише `enum`/ідентифікатори;
  тест у `notify.rs` фіксує ОЧІКУВАНУ форму (allowlist полів), не автоматичний type-check —
  Rust не має рефлексії полів у stable.
- **`schema-gen`-фіча** (optional, вимикається за замовчуванням): `schemars`-деривативи +
  `src/bin/gen_schema.rs` + `contracts/schema/*.json` — джерело TS-типів для `services/notifier`
  (`json-schema-to-typescript`, `.claude/decisions/notifier-ts-whatsapp-web-js.md`). Змінив поле
  в `notify.rs`/`events.rs`/`envelope.rs` → перегенеруй: `cargo run -p contracts --features
  schema-gen --bin gen_schema` (тест `tests/schema.rs` ловить забуте перегенерування, потребує
  тієї самої фічі: `cargo test -p contracts --features schema-gen`).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `envelope.rs` | `Envelope<T>` — спільна обгортка (id/type/version/occurred_at/producer/correlation_id/causation_id/payload) | `bus`, `app::backend::repo::outbox` (Фаза 1), `services/notifier` |
| `subjects.rs` | Константи subject-ів і назв JetStream-стрімів (§3.3) | `bus`, `server` (relay), `services/notifier` |
| `notify.rs` | `NotifySend`/`NotifyResult`/`NotifyTemplate`/`NotifyStatus` — команда доставки й результат, приватність вбудована в тип | `services/notifier`, майбутній продюсер "хто/коли сповіщати" (04 §5, поза цими фазами) |
| `events.rs` | `DiscrepancyOpened`/`.Resolved`/`DiscrepancyMetric` — перший реальний продюсер, Фаза 1 (`repo::reconciliation::refresh_horizontal`) | `app::backend::repo::reconciliation` (Фаза 1) |
| `schema/*.json` | Згенеровані JSON Schema (НЕ редагувати вручну — перезаписуються `gen_schema`) | `services/notifier` (`json2ts`) |
| `src/bin/gen_schema.rs` | Генератор `schema/*.json` з реальних Rust-типів (той самий підхід, що `style/src/bin/gen.rs` для CSS) | ручний запуск при зміні контракту |
