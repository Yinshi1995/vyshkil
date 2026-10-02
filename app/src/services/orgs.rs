use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::org::OrgSearchResult;

/// Список організацій: (id, "назва (номер)"). Використовують ≥ 2 місця
/// (`layout::ActorSwitcher` і `pages::home`) — тому тут, а не в `pages/<p>/server.rs` (07 §3.5).
#[server(ListOrgs, "/api")]
pub async fn list_orgs() -> Result<Vec<(i32, String)>, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::orgs::list_orgs(&db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Нечіткий пошук організацій (02 §3, критерій готовності Етапу 1: "152НЦ"/"а4896"/"польша"
/// знаходять канонічні організації). Потрібен ≥ 2 сторінкам (`pages::home`, `pages::training_form`)
/// — тому тут, а не в `pages/<p>/server.rs` (07 §3.5). Сама логіка — в `backend::repo::orgs::
/// search_orgs`; результат звужений до видимого акторові піддерева (`backend::policy::
/// visible_org_ids`, 01 §6) — `actor: None` (актор у шапці ще не обраний) означає "нічого не видно".
#[server(SearchOrgs, "/api")]
pub async fn search_orgs(
    actor: Option<Actor>,
    query: String,
) -> Result<Vec<OrgSearchResult>, ServerFnError> {
    use crate::backend::{policy, repo};
    use crate::services::auth::resolve_actor;

    let Ok(actor) = resolve_actor(actor).await else { return Ok(Vec::new()) };
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
