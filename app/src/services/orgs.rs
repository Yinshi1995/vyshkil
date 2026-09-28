use leptos::prelude::*;

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
