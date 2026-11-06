//! Серверна бібліотека Вишкіла: домен, SQL/SeaORM (`backend`), типи DTO і хелпери автентифікації.
//! Leptos-інтерфейс і `#[server]`-функції видалено (рішення `remove-leptos-fallback`); UI — `web/` (React).

pub mod domain;
pub mod services;
pub mod types;

// Лише на сервері: SQL/SeaORM (repo), перевірка прав (policy), документи, імпорт.
#[cfg(feature = "ssr")]
pub mod backend;
