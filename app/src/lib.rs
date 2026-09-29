pub mod app;
pub mod components;
pub mod domain;
pub mod hooks;
pub mod layout;
pub mod pages;
pub mod routes;
pub mod services;
pub mod types;
pub mod widgets;

// Лише на сервері: SQL/SeaORM (repo) і — від Кроку 3 — перевірка прав (policy). Клієнт (WASM)
// цю гілку взагалі не бачить (07 §1, §2.3): backend не збирається без sea-orm.
#[cfg(feature = "ssr")]
pub mod backend;
