# app/src/pages/request_account — запит облікового запису

Публічна сторінка (без автентифікації) для запиту створення облікового запису.
Надсилає нотифікацію адміністратору з контактними даними заявника.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `RequestAccountPage` — Leptos-компонент форми запиту | `routes.rs` (`/request-account`) |
| `server.rs` | `request_account` — `#[server]` fn: знаходить admin org_id, створює notification | `mod.rs` |
