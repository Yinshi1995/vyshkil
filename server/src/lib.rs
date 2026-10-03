//! Виділено з `main.rs` у бібліотечну ціль лише заради тестованості `relay` (09-messaging.md §6:
//! "інтеграційні з реальним nats-server у Docker") — `server` лишається тонким (07-code-structure
//! §"Інші крейти"), сам бінарник (`main.rs`) — тонка обгортка над цими модулями, як і раніше.

pub mod admin_sse;
pub mod api;
pub mod config;
pub mod db;
pub mod relay;
pub mod spa;
pub mod state;
