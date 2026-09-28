# server/

- Тонкий крейт: axum-роутер, конфіг, пул БД, стан. Доменної логіки й прав тут немає — `policy`
  тепер в `app/src/backend/policy.rs` (07-code-structure.md §5), бо `#[server]`-функції живуть
  у крейті `app`, а `server` залежить від `app`, не навпаки.
- Гарячі запити по ієрархії підпорядкування — через `subordination_closure` (матеріалізоване
  замикання), **не** рекурсивні CTE.
- Важкі задачі (імпорт великого файлу, OCR, генерація pptx) — фоновими задачами з прогресом;
  файли обробляти потоково, не завантажувати цілком у пам'ять.
- RSS сервера контролювати: 8 ГБ на все (Postgres + застосунок) — див. вимірювання в MEMORY.md.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `main.rs` | `main()`, tokio-рантайм, axum-роутер, `leptos_routes_with_context`, fallback-хендлер | точка входу бінарника |
| `config.rs` | `Config::from_env()` — адреса, воркер-потоки, `DATABASE_URL` тощо | `main.rs` |
| `db.rs` | `connect()` — `sea_orm::Database` з обмеженим пулом | `main.rs` |
| `state.rs` | `AppState` (`FromRef`) — `LeptosOptions` + `DatabaseConnection` для axum-роутера | `main.rs` |
