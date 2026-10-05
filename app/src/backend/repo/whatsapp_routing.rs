use contracts::{subjects, Envelope, NotifySend, NotifyTemplate};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use uuid::Uuid;

use super::outbox;

#[derive(FromQueryResult, serde::Serialize)]
pub struct DestinationRow {
    pub id: i32,
    pub org_id: i32,
    pub kind: String,
    pub phone_masked: String,
    pub group_id: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

pub async fn list_destinations(db: &DatabaseConnection, org_id: i32) -> Result<Vec<DestinationRow>, DbErr> {
    DestinationRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, org_id, kind, phone_masked, group_id, is_active, \
         to_char(created_at, 'YYYY-MM-DD') AS created_at \
         FROM whatsapp_destination WHERE org_id = $1 ORDER BY created_at DESC",
        [org_id.into()],
    ))
    .all(db)
    .await
}

pub async fn add_destination(
    db: &DatabaseConnection,
    user_id: Option<i32>,
    org_id: i32,
    kind: &str,
    phone_masked: &str,
    group_id: Option<&str>,
) -> Result<i32, DbErr> {
    #[derive(FromQueryResult)]
    struct NewId { id: i32 }
    let row = NewId::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO whatsapp_destination (user_id, org_id, kind, phone_masked, group_id) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
        [user_id.into(), org_id.into(), kind.into(), phone_masked.into(), group_id.into()],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("INSERT whatsapp_destination failed".into()))?;
    Ok(row.id)
}

pub async fn toggle_destination(db: &DatabaseConnection, id: i32, active: bool) -> Result<bool, DbErr> {
    let result = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE whatsapp_destination SET is_active = $1 WHERE id = $2",
        [active.into(), id.into()],
    )).await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_destination(db: &DatabaseConnection, id: i32) -> Result<bool, DbErr> {
    let result = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM whatsapp_destination WHERE id = $1",
        [id.into()],
    )).await?;
    Ok(result.rows_affected() > 0)
}

#[derive(FromQueryResult, serde::Serialize)]
pub struct SubscriptionRow {
    pub id: i32,
    pub destination_id: i32,
    pub notification_type_code: String,
    pub type_name: String,
    pub is_active: bool,
}

pub async fn list_subscriptions(db: &DatabaseConnection, destination_id: i32) -> Result<Vec<SubscriptionRow>, DbErr> {
    SubscriptionRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT ns.id, ns.destination_id, ns.notification_type_code, \
         nt.name AS type_name, ns.is_active \
         FROM notification_subscription ns \
         JOIN notification_type nt ON nt.code = ns.notification_type_code \
         WHERE ns.destination_id = $1 ORDER BY nt.code",
        [destination_id.into()],
    ))
    .all(db)
    .await
}

#[derive(FromQueryResult, serde::Serialize)]
pub struct NotificationTypeRow {
    pub code: String,
    pub name: String,
    pub scope: String,
}

pub async fn list_notification_types(db: &DatabaseConnection) -> Result<Vec<NotificationTypeRow>, DbErr> {
    NotificationTypeRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT code, name, scope FROM notification_type ORDER BY code",
    ))
    .all(db)
    .await
}

pub async fn set_subscription(
    db: &impl ConnectionTrait,
    destination_id: i32,
    type_code: &str,
    active: bool,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO notification_subscription (destination_id, notification_type_code, is_active) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (destination_id, notification_type_code) \
         DO UPDATE SET is_active = $3",
        [destination_id.into(), type_code.into(), active.into()],
    )).await?;
    Ok(())
}

#[derive(FromQueryResult, serde::Serialize)]
pub struct SyncContactRow {
    pub org_id: i32,
    pub phone: String,
    pub kind: String,
    pub is_active: bool,
}

pub async fn all_destinations_for_sync(db: &DatabaseConnection) -> Result<Vec<SyncContactRow>, DbErr> {
    SyncContactRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT org_id, \
                CASE WHEN kind = 'group' THEN COALESCE(NULLIF(group_id, ''), phone_masked) \
                     ELSE phone_masked END AS phone, \
                kind, is_active \
         FROM whatsapp_destination ORDER BY org_id",
    ))
    .all(db)
    .await
}

/// Знаходить усі активні WhatsApp-призначення для `org_id`, підписані на `notification_type_code`,
/// і створює для кожного `vyshkil.notify.send.v1` запис в outbox (relay доставить у NATS NOTIFY_CMD).
pub async fn dispatch_wa_notifications(
    db: &impl ConnectionTrait,
    org_id: i32,
    notification_type_code: &str,
    template: NotifyTemplate,
) -> Result<(), DbErr> {
    #[derive(FromQueryResult)]
    struct MatchedDest {
        dest_org_id: i32,
    }
    let matched = MatchedDest::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT DISTINCT wd.org_id AS dest_org_id \
         FROM whatsapp_destination wd \
         JOIN notification_subscription ns ON ns.destination_id = wd.id \
         WHERE wd.is_active \
           AND ns.notification_type_code = $2 \
           AND ns.is_active \
           AND (wd.org_id = $1 OR wd.org_id IN ( \
               SELECT ancestor_id FROM subordination_closure WHERE descendant_id = $1 \
           ))",
        [org_id.into(), notification_type_code.into()],
    ))
    .all(db)
    .await?;

    for dest in &matched {
        let dedupe_key = format!("org:{org_id}:{notification_type_code}:{}", Uuid::now_v7());
        let payload = NotifySend {
            recipient_org_id: dest.dest_org_id as i64,
            template,
            dedupe_key,
        };
        let envelope = Envelope::new(
            subjects::NOTIFY_SEND_V1,
            1,
            "app::backend::repo::whatsapp_routing",
            Uuid::now_v7(),
            None,
            payload,
        );
        let json = serde_json::to_string(&envelope)
            .map_err(|e| DbErr::Custom(format!("серіалізація NOTIFY_SEND: {e}")))?;
        outbox::insert(db, subjects::NOTIFY_SEND_V1, &json, "{}").await?;
    }
    Ok(())
}
