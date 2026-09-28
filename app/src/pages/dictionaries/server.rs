use leptos::prelude::*;

use crate::types::actor::Actor;

/// Learned-синоніми на підтвердження — лише `admin` (01 §"Навчання": "Адміністратор бачить нові
/// learned-синоніми в окремому списку").
#[server(GetLearnedAliases, "/api")]
pub async fn get_learned_aliases(
    actor: Option<Actor>,
) -> Result<Vec<crate::types::dictionaries::LearnedAlias>, ServerFnError> {
    use crate::backend::{policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    if !policy::is_admin(actor) {
        return Err(ServerFnError::new("лише адміністратор бачить чергу learned-синонімів"));
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::dictionaries::learned_aliases(&db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Підтвердити learned-синонім (`source` -> `'manual'`) — лише `admin`. Транзакція з `SET LOCAL
/// app.actor`, щоб audit_log знав, хто підтвердив (07-code-structure.md §5, `backend::db`).
#[server(ConfirmLearnedAlias, "/api")]
pub async fn confirm_learned_alias(actor: Option<Actor>, alias_id: i32) -> Result<(), ServerFnError> {
    use crate::backend::{db, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    if !policy::is_admin(actor) {
        return Err(ServerFnError::new("лише адміністратор підтверджує синоніми"));
    }

    let txn = db::actor_transaction(actor).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    repo::dictionaries::confirm_learned_alias(&txn, alias_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    txn.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Відхилити learned-синонім (видалити з `alias`) — лише `admin`.
#[server(RejectLearnedAlias, "/api")]
pub async fn reject_learned_alias(actor: Option<Actor>, alias_id: i32) -> Result<(), ServerFnError> {
    use crate::backend::{db, policy, repo};

    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    if !policy::is_admin(actor) {
        return Err(ServerFnError::new("лише адміністратор відхиляє синоніми"));
    }

    let txn = db::actor_transaction(actor).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    repo::dictionaries::reject_learned_alias(&txn, alias_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    txn.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
