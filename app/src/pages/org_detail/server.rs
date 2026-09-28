use leptos::prelude::*;

use crate::types::org::OrgDetail;

/// Картка частини з історією назв і статусів (06-roadmap.md, Етап 1). Потрібен тільки цій
/// сторінці. Сама логіка — в `backend::repo::orgs::org_detail`.
#[server(GetOrgDetail, "/api")]
pub async fn get_org_detail(org_id: i32) -> Result<OrgDetail, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::orgs::org_detail(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("організацію не знайдено"))
}
