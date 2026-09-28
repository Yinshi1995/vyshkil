use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::org::{OrgSearchResult, OrgTreeRow};

/// Нечіткий пошук організацій (02 §3, критерій готовності Етапу 1: "152НЦ"/"а4896"/"польша"
/// знаходять канонічні організації). Потрібен тільки головній сторінці — тому тут, а не в
/// `services/` (07 §3.5). Сама логіка — в `backend::repo::orgs::search_orgs`; результат
/// звужений до видимого акторові піддерева (`backend::policy::visible_org_ids`, 01 §6) —
/// `actor: None` (актор у шапці ще не обраний) означає "нічого не видно".
#[server(SearchOrgs, "/api")]
pub async fn search_orgs(
    actor: Option<Actor>,
    query: String,
) -> Result<Vec<OrgSearchResult>, ServerFnError> {
    use crate::backend::{policy, repo};

    let Some(actor) = actor else { return Ok(Vec::new()) };
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let visible = policy::visible_org_ids(&db, actor)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let results = repo::orgs::search_orgs(&db, &query)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(match visible {
        None => results,
        Some(ids) => results.into_iter().filter(|r| ids.contains(&r.org_id)).collect(),
    })
}

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
