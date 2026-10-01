# app/src/pages/login — сторінка входу (`/login`)

- Можна: `services/auth`, `types`. Не можна: `backend` (auth logic through services only).
- Мінімальна сторінка: форма логін+пароль → `auth_login` server fn → cookie → redirect.
- Не використовує `PageHeader`/`PageContent` — повноекранний центрований layout.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `mod.rs` | `LoginPage` — форма входу | `routes.rs` (`/login`) |
