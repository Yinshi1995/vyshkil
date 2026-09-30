# server/

- Тонкий крейт: axum-роутер, конфіг, пул БД, стан. Доменної логіки й прав тут немає — `policy`
  тепер в `app/src/backend/policy.rs` (07-code-structure.md §5), бо `#[server]`-функції живуть
  у крейті `app`, а `server` залежить від `app`, не навпаки.
- Гарячі запити по ієрархії підпорядкування — через `subordination_closure` (матеріалізоване
  замикання), **не** рекурсивні CTE.
- Важкі задачі (імпорт великого файлу, OCR, генерація pptx) — фоновими задачами з прогресом;
  файли обробляти потоково, не завантажувати цілком у пам'ять.
- RSS сервера контролювати: 8 ГБ на все (Postgres + застосунок) — див. вимірювання в MEMORY.md.
- `relay.rs` — перша фонова задача (09-messaging.md §3.1, Фаза 1 брокера,
  `.claude/decisions/broker-nats-jetstream.md`): не використовуй як шаблон "де класти
  доменну логіку" — вона тут не живе, лише читає/оновлює `outbox` і публікує вже готові байти.
- `admin_sse.rs` — сирий Axum SSE-хендлер (09 §4, `/admin/whatsapp`), НЕ Leptos `#[server]` fn
  (ті не стрімлять) — права тут-таки через query-параметри actor (той самий довірчий контракт,
  що всюди на цьому етапі, INTERFACES.md), QR рендериться в SVG тут (не client-side JS з CDN).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `main.rs` | `main()`, tokio-рантайм, axum-роутер, `leptos_routes_with_context`, fallback-хендлер, `tokio::spawn(relay::run(...))` | точка входу бінарника |
| `config.rs` | `Config::from_env()` — адреса, воркер-потоки, `DATABASE_URL`, `NATS_URL` тощо | `main.rs` |
| `db.rs` | `connect()` — `sea_orm::Database` з обмеженим пулом | `main.rs` |
| `state.rs` | `AppState` (`FromRef`) — `LeptosOptions`+`DatabaseConnection`+`bus::SharedNatsClient` для axum-роутера | `main.rs` |
| `relay.rs` | `run()` — фонова задача: непубліковані рядки `outbox` → NATS JetStream (таймер-опитувач, не LISTEN/NOTIFY), забезпечує стрім `EVENTS` | `main.rs` (`tokio::spawn`) |
| `admin_sse.rs` | `whatsapp_status_stream` — SSE з NATS KV `notifier_status`, QR → SVG на сервері | `main.rs` (маршрут `/api/admin/whatsapp/events`) |
