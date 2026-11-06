# app/src/services

Лишився лише `auth.rs`: хелпери аварійного парольного входу (`PASSWORD_LOGIN`) і ліміт спроб. Усі Leptos `#[server]`-функції
видалено — API живе в `server/src/api.rs` (REST).
