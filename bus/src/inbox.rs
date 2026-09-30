//! Ідемпотентність споживача (09-messaging.md §3.2): доставка — "щонайменше один раз", кожен
//! конкретний споживач (relay-результати в `app`, `notifier`) веде власну таблицю `inbox` у
//! СВОЇЙ БД — `bus` лише оголошує контракт трейтом, не знає про Postgres.

use async_trait::async_trait;

#[async_trait]
pub trait InboxStore: Send + Sync {
    /// `true`, якщо це повідомлення вже було оброблене (дублікат — пропустити, підтвердити ack).
    async fn already_processed(&self, message_id: &str) -> Result<bool, String>;

    /// Позначити оброблене. Викликач відповідає за виклик у ТІЙ САМІЙ транзакції, що й власне
    /// побічний ефект обробки (§3.2) — цей трейт сам транзакцій не відкриває.
    async fn mark_processed(&self, message_id: &str) -> Result<(), String>;
}
