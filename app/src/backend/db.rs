//! Дістати `DatabaseConnection` з Leptos-контексту; для операцій, що ПИШУТЬ в аудійовані таблиці
//! (`org`, `org_name_history`, `training_site`, `subordination`, `org_status`, `alias`) —
//! транзакція з виставленим `SET LOCAL app.actor`, щоб generic-тригер `audit_log` знав, хто зробив
//! зміну (лише `org_id:role`, без ПІБ — 01 §2). Читання транзакції не потребує: `connection()`
//! досить.

use leptos::prelude::expect_context;
use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr, TransactionTrait};

use super::policy;
use crate::types::actor::Actor;

/// `DatabaseConnection` з контексту поточного Leptos-роуту (провайджений у `server/src/main.rs`,
/// див. `.claude/memory/INTERFACES.md`).
pub fn connection() -> DatabaseConnection {
    expect_context::<DatabaseConnection>()
}

/// Транзакція з `SET LOCAL app.actor` — для будь-якої server fn, що пише в аудійовану таблицю.
pub async fn actor_transaction(actor: Actor) -> Result<DatabaseTransaction, DbErr> {
    let txn = connection().begin().await?;
    policy::set_session_actor(&txn, actor).await?;
    Ok(txn)
}
