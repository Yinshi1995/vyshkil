# app/src/pages/admin_queues — адмін-екран "Черги" (09-messaging.md §5, Фаза 4)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під
  `ssr`), `bus` (лише з `server.rs`, той самий виняток, що `admin_whatsapp`). Не можна: інші сторінки.
- Звичайний `Resource` + кнопка "Оновити" — не SSE (на відміну від `/admin/whatsapp`): тут немає
  постійно змінюваного стану, ручного оновлення досить.
- DLQ читається НЕ через звичайний consumer (це б спожило повідомлення) — `Stream::
  get_raw_message(seq)` напряму за номером, останні `DLQ_DISPLAY_LIMIT` (20). "Повторити" —
  декодує збережений payload, публікує з НОВИМ `Nats-Msg-Id` (не оригінальним — інакше брокер
  міг би відкинути як дублікат), видаляє запис із DLQ лише після вдалого republish.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `AdminQueuesPage` — outbox/стріми/DLQ-список з кнопками "Оновити"/"Повторити" | `routes.rs` |
| `server.rs` | `get_queue_status`/`retry_dlq_entry` — `repo::outbox::backlog` + NATS JetStream stream/DLQ-читання через `bus::SharedNatsClient` | `mod.rs` |
