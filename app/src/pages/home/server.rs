use leptos::prelude::*;

use crate::types::org::{OrgSearchResult, OrgTreeRow};

/// Нечіткий пошук організацій (02 §3, критерій готовності Етапу 1: "152НЦ"/"а4896"/"польша"
/// знаходять канонічні організації). Потрібен тільки головній сторінці — тому тут, а не в
/// `services/` (07 §3.5). Сама логіка — в `backend::repo::orgs::search_orgs`.
#[server(SearchOrgs, "/api")]
pub async fn search_orgs(query: String) -> Result<Vec<OrgSearchResult>, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::orgs::search_orgs(&db, &query)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Дерево підпорядкування на дату (06-roadmap.md, Етап 1): перемикач осі штатне/оперативне.
/// Потрібен тільки головній сторінці. Сама логіка — в `backend::repo::orgs::subordination_tree`
/// (гарячий запит через `subordination_closure`, `depth = 1`, **не** рекурсивний CTE —
/// server/CLAUDE.md).
#[server(GetSubordinationTree, "/api")]
pub async fn get_subordination_tree(
    as_of: String,
    axis: String,
) -> Result<Vec<OrgTreeRow>, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::orgs::subordination_tree(&db, &as_of, &axis)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
