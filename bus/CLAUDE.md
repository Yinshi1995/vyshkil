# bus/ — тонка обгортка над `async-nats` (09-messaging.md §4, Фаза 0)

- Не знає про Postgres/`outbox`/`inbox`-таблиці конкретно — той стан лишається в `app`/
  `services/notifier`; `bus` — лише транспорт (publish з дедуплікацією, pull-consumer, контракт
  ідемпотентності трейтом).
- Помилки — один плаский `BusError(String)` (той самий підхід, що `DbErr::Custom(format!(...))`
  у `app::backend::repo` — жодного `thiserror` в цьому воркспейсі більше нема).
- Інтеграційні тести (`tests/pubsub.rs`) — проти РЕАЛЬНОГО `nats-server -js` у Docker, не мока;
  потребують `TEST_NATS_URL` (той самий підхід, що `TEST_DATABASE_URL` в `app/tests/common`) —
  без нього пропущено, не падає.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `publish.rs` | `connect`, `jetstream`, `publish_with_msg_id`/`publish_raw_with_msg_id` (дедуплікація за `Nats-Msg-Id`, §3.1), `request` (core request-reply, admin-команди §4) | `server` (relay), `app::pages::admin_whatsapp::server` |
| `consumer.rs` | `ConsumerSpec`, `ensure_pull_consumer` — durable pull-consumer (`ack_wait`/`max_deliver`) | `services/notifier` (Фаза 2) |
| `stream.rs` | `ensure_stream` — ідемпотентне get-or-create стріму (relay забезпечує `EVENTS` на старті) | `server::relay` |
| `inbox.rs` | `InboxStore` — трейт ідемпотентності споживача, реалізація — власна БД кожного споживача | `server`, `services/notifier` |
| `error.rs` | `BusError`/`BusResult` | усі модулі цього крейту |
| `lib.rs` | `SharedNatsClient` (`Arc<Mutex<Option<async_nats::Client>>>`) — relay єдиний володіє з'єднанням, `/admin/whatsapp` (`app`+`server::admin_sse`) лише читає клон через контекст (09 §4) | `server::relay`, `server::state`, `server::admin_sse`, `app::pages::admin_whatsapp::server` |
