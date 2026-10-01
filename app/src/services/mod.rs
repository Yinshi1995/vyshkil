pub mod auth;
pub mod dictionaries;
pub mod groups;
pub mod health;
pub mod notifications;
pub mod orgs;
// Не #[server] сама по собі (плейн async-функції, що ділить логіка форми/імпорту) -- на відміну
// від решти services/*, посилається на `backend` НЕ лише всередині тіл #[server]-функцій (07 §2.3
// про backend-gating стосується самого `backend`, а виклики цього файлу відбуваються з обох
// #[server]-функцій одразу, тож простіше й чесніше ganти сам модуль).
#[cfg(feature = "ssr")]
pub mod submission_grid;
