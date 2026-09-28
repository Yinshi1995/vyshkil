use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::org::OrgTreeRow;

/// Дерево підпорядкування на дату (06-roadmap.md, Етап 1): перемикач осі штатне/оперативне.
/// Потрібен тільки головній сторінці. Сама логіка — в `backend::repo::orgs::subordination_tree`
/// (гарячий запит через `subordination_closure`, `depth = 1`, **не** рекурсивний CTE —
/// server/CLAUDE.md); результат обрізаний до видимого акторові піддерева
/// (`backend::policy::restrict_tree`, 01 §6) — `actor: None` означає "нічого не видно".
#[server(GetSubordinationTree, "/api")]
pub async fn get_subordination_tree(
    actor: Option<Actor>,
    as_of: String,
    axis: String,
) -> Result<Vec<OrgTreeRow>, ServerFnError> {
    use crate::backend::{policy, repo};

    let Some(actor) = actor else { return Ok(Vec::new()) };
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let visible = policy::visible_org_ids(&db, actor)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let rows = repo::orgs::subordination_tree(&db, &as_of, &axis)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(match visible {
        None => rows,
        Some(ids) => policy::restrict_tree(rows, &ids),
    })
}
