//! Транзакція з виставленим `SET LOCAL app.actor` — для операцій, що ПИШУТЬ в аудійовані таблиці
//! (`org`, `org_name_history`, `training_site`, `subordination`, `org_status`, `alias`), щоб generic-тригер
//! `audit_log` знав, хто зробив зміну (лише `org_id:role`, без ПІБ — 01 §2).

use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr, TransactionTrait};

use super::policy;
use crate::types::actor::Actor;

pub async fn actor_transaction(db: &DatabaseConnection, actor: Actor) -> Result<DatabaseTransaction, DbErr> {
    let txn = db.begin().await?;
    policy::set_session_actor(&txn, actor).await?;
    Ok(txn)
}
