use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::org::OrgDetail;

/// Картка частини з історією назв і статусів (06-roadmap.md, Етап 1). Потрібен тільки цій
/// сторінці. Сама логіка — в `backend::repo::orgs::org_detail`; доступ перевіряється через
/// `backend::policy::can_view_org` (01 §6) — своя організація або її піддерево, `admin` бачить усе.
#[server(GetOrgDetail, "/api")]
pub async fn get_org_detail(actor: Option<Actor>, org_id: i32) -> Result<OrgDetail, ServerFnError> {
    use crate::backend::{policy, repo};
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let allowed = policy::can_view_org(&db, actor, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if !allowed {
        return Err(ServerFnError::new("немає прав переглядати цю організацію"));
    }

    repo::orgs::org_detail(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("організацію не знайдено"))
}
