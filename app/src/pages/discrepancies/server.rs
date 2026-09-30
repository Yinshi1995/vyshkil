use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::reconciliation::DiscrepancyRow;

/// Розбіжності (04 §4, Етап 8 зріз 1) — звужені до видимого акторові піддерева, той самий
/// принцип, що `pages/home/server.rs::get_subordination_tree` (репозиторій читає все, права
/// фільтрують після — `policy::visible_org_ids` тут не потребує окремого SQL на рядок).
/// `status`: `None`/`""` — усі статуси.
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
