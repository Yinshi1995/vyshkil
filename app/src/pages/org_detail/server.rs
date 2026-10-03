use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::auth::{AdminGroupRow, AdminSubmissionRow, AdminUserRow};
use crate::types::org::OrgDetail;

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

#[server(GetOrgChildren, "/api")]
pub async fn get_org_children(
    actor: Option<Actor>,
    org_id: i32,
) -> Result<Vec<(i32, String)>, ServerFnError> {
    use crate::backend::{policy, repo};
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let allowed = policy::can_view_org(&db, actor, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if !allowed {
        return Err(ServerFnError::new("немає прав"));
    }

    repo::orgs::direct_children(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(GetOrgGroups, "/api")]
pub async fn get_org_groups(
    actor: Option<Actor>,
    org_id: i32,
) -> Result<Vec<AdminGroupRow>, ServerFnError> {
    use crate::backend::{policy, repo};
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let allowed = policy::can_view_org(&db, actor, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if !allowed {
        return Err(ServerFnError::new("немає прав"));
    }

    let subtree = repo::orgs::subtree_org_ids(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    repo::auth::list_training_groups(&db, Some(&subtree))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(GetOrgSubmissions, "/api")]
pub async fn get_org_submissions(
    actor: Option<Actor>,
    org_id: i32,
) -> Result<Vec<AdminSubmissionRow>, ServerFnError> {
    use crate::backend::{policy, repo};
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let allowed = policy::can_view_org(&db, actor, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if !allowed {
        return Err(ServerFnError::new("немає прав"));
    }

    let subtree = repo::orgs::subtree_org_ids(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    repo::auth::list_submissions(&db, Some(&subtree))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(GetOrgUsers, "/api")]
pub async fn get_org_users(
    actor: Option<Actor>,
    org_id: i32,
) -> Result<Vec<AdminUserRow>, ServerFnError> {
    use crate::backend::{policy, repo};
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    if !policy::is_admin(actor) {
        return Err(ServerFnError::new("тільки адміністратор"));
    }

    let subtree = repo::orgs::subtree_org_ids(&db, org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    repo::auth::list_users_by_orgs(&db, &subtree)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
