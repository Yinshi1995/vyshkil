use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::reconciliation::DiscrepancyRow;

#[server(GetDiscrepancies, "/api")]
pub async fn get_discrepancies(
    actor: Option<Actor>,
    status: Option<String>,
) -> Result<Vec<DiscrepancyRow>, ServerFnError> {
    use crate::backend::{policy, repo};

    let Some(actor) = actor else { return Ok(Vec::new()) };
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let visible = policy::visible_org_ids(&db, actor)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let status_filter = status.filter(|s| !s.is_empty());
    let rows = repo::reconciliation::list_discrepancies(&db, status_filter.as_deref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(match visible {
        None => rows,
        Some(ids) => rows.into_iter().filter(|r| ids.contains(&r.org_id)).collect(),
    })
}

#[server(UpdateDiscrepancyStatus, "/api")]
pub async fn update_discrepancy_status(
    actor: Option<Actor>,
    discrepancy_id: i32,
    new_status: String,
    resolution_note: Option<String>,
) -> Result<(), ServerFnError> {
    use crate::backend::{db, policy, repo};

    let Some(actor) = actor else {
        return Err(ServerFnError::new("актора не обрано"));
    };

    let valid_statuses = ["in_progress", "resolved", "dismissed"];
    if !valid_statuses.contains(&new_status.as_str()) {
        return Err(ServerFnError::new(format!("невалідний статус: {new_status}")));
    }

    let txn = db::actor_transaction(actor).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    let org_id = repo::reconciliation::update_discrepancy_status(
        &txn,
        discrepancy_id,
        &new_status,
        resolution_note.as_deref(),
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let Some(org_id) = org_id else {
        return Err(ServerFnError::new("розбіжність не знайдена"));
    };

    if !policy::can_edit_org(actor, org_id) {
        txn.rollback().await.map_err(|e| ServerFnError::new(e.to_string()))?;
        return Err(ServerFnError::new("немає прав на цю організацію"));
    }

    txn.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
