# app/src/pages/settings — сторінка налаштувань (`/settings`)

- Можна: `services`, `widgets`, `hooks`, `types`, `domain`, `backend` (лише з `server.rs`, під
  `ssr`), `bus` (лише з `server.rs` — WhatsApp/черги потребують NATS). Не можна: інші сторінки.
- Консолідує: admin_whatsapp (09 §4), admin_queues (09 §5), styleguide-теми (08 §Фаза 2),
  learned-синоніми (dictionaries). SSE-ендпоінт WhatsApp (`server::admin_sse`) лишається в
  крейті `server`, не тут — SSE не вписується в Leptos server fn модель.
- Tabs: Вигляд (теми), Безпека (auth, passkeys), WhatsApp (admin), Черги (admin), Синоніми (admin).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `SettingsPage` — tabs + секції (AppearanceSection, SecuritySection, WhatsappSection, QueuesSection, LearnedAliasSection) | `routes.rs` (`/settings`) |
| `server.rs` | server fn: learned aliases, WhatsApp NATS commands, queue status/retry, passkey list/delete (12-auth.md §1.3) | `mod.rs` |
