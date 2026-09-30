# app/src/pages/admin_whatsapp — прив'язка WhatsApp-сесії (09-messaging.md §4)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під
  `ssr`), `bus` (лише з `server.rs` — request-reply команди до `notifier`, той самий виняток, що
  `sea-orm`/`backend`). Не можна: інші сторінки.
- Статус/QR — **не** через `#[server]`-функцію/`Resource` (ті не стрімлять): клієнтський
  `web_sys::EventSource` у `mod.rs` слухає сирий Axum SSE-ендпоінт `server::admin_sse`
  (`/api/admin/whatsapp/events`), який живе в крейті `server`, не тут — SSE не вписується в
  Leptos server fn модель.
- QR рендериться в SVG на боці `server` (не тут, не client-side JS з CDN) — `mod.rs` лише вставляє
  готовий SVG через `inner_html`.
- Права — подвійно: клієнт ховає UI для не-admin (`Role::Admin` check у `mod.rs`, косметика), але
  РЕАЛЬНА перевірка — і в `server.rs` (`policy::is_admin` у кожній `#[server]`-функції), і в
  `server::admin_sse` (query-параметри actor, 403 якщо не admin) — SSE-ендпоінт поза Leptos'ового
  контексту актора, тому не може покладатись лише на клієнтську приховану кнопку.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `AdminWhatsappPage` — EventSource-підписка, форми "прив'язати за номером"/"відв'язати"/"тестове" | `routes.rs` |
| `server.rs` | `request_pairing_code`/`logout_whatsapp`/`send_test_notification` — NATS request-reply до `notifier` (`bus::request`, `bus::SharedNatsClient` з контексту) | `mod.rs` |
