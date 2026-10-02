use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::notification::NotificationRow;

#[server(GetUnreadCount, "/api")]
pub async fn get_unread_count(actor: Option<Actor>) -> Result<i64, ServerFnError> {
    use crate::backend::repo;
    use crate::services::auth::resolve_actor;

    let Ok(actor) = resolve_actor(actor).await else { return Ok(0) };
    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::notifications::unread_count(&db, actor.org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(GetNotifications, "/api")]
pub async fn get_notifications(actor: Option<Actor>) -> Result<Vec<NotificationRow>, ServerFnError> {
    use crate::backend::repo;
    use crate::services::auth::resolve_actor;

    let Ok(actor) = resolve_actor(actor).await else { return Ok(Vec::new()) };
    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::notifications::list_for_org(&db, actor.org_id, 50)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(MarkNotificationRead, "/api")]
pub async fn mark_notification_read(
    actor: Option<Actor>,
    notification_id: i32,
) -> Result<(), ServerFnError> {
    use crate::backend::repo;
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::notifications::mark_read(&db, actor.org_id, notification_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(MarkAllNotificationsRead, "/api")]
pub async fn mark_all_notifications_read(actor: Option<Actor>) -> Result<(), ServerFnError> {
    use crate::backend::repo;
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::notifications::mark_all_read(&db, actor.org_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
