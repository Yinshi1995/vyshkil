use crate::types::notification::NotificationRow;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

pub async fn insert(
    db: &impl ConnectionTrait,
    org_id: i32,
    kind: &str,
    title: &str,
    body: Option<&str>,
    link: Option<&str>,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO notification (org_id, kind, title, body, link) VALUES ($1, $2, $3, $4, $5)",
        [org_id.into(), kind.into(), title.into(), body.into(), link.into()],
    ))
    .await?;
    Ok(())
}

pub async fn unread_count(db: &DatabaseConnection, org_id: i32) -> Result<i64, DbErr> {
    #[derive(FromQueryResult)]
    struct Count {
        cnt: i64,
    }
    let row = Count::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT COUNT(*)::bigint AS cnt FROM notification \
         WHERE org_id IN (SELECT $1::int UNION SELECT descendant_id FROM subordination_closure \
         WHERE ancestor_id = $1 AND daterange(valid_from, valid_to, '[)') @> CURRENT_DATE) \
         AND is_read = false",
        [org_id.into()],
    ))
    .one(db)
    .await?;
    Ok(row.map(|r| r.cnt).unwrap_or(0))
}

pub async fn list_for_org(
    db: &DatabaseConnection,
    org_id: i32,
    limit: i64,
) -> Result<Vec<NotificationRow>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        kind: String,
        title: String,
        body: Option<String>,
        link: Option<String>,
        is_read: bool,
        created_at: String,
    }
    let rows = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT n.id, n.kind, n.title, n.body, n.link, n.is_read, \
                to_char(n.created_at, 'YYYY-MM-DD\"T\"HH24:MI:SS') AS created_at \
         FROM notification n \
         WHERE n.org_id IN (SELECT $1::int UNION SELECT descendant_id FROM subordination_closure \
         WHERE ancestor_id = $1 AND daterange(valid_from, valid_to, '[)') @> CURRENT_DATE) \
         ORDER BY n.created_at DESC LIMIT $2",
        [org_id.into(), limit.into()],
    ))
    .all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| NotificationRow {
            id: r.id,
            kind: r.kind,
            title: r.title,
            body: r.body,
            link: r.link,
            is_read: r.is_read,
            created_at: r.created_at,
        })
        .collect())
}

pub async fn mark_read(db: &DatabaseConnection, org_id: i32, notification_id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE notification SET is_read = true WHERE id = $1 \
         AND org_id IN (SELECT $2::int UNION SELECT descendant_id FROM subordination_closure \
         WHERE ancestor_id = $2 AND daterange(valid_from, valid_to, '[)') @> CURRENT_DATE)",
        [notification_id.into(), org_id.into()],
    ))
    .await?;
    Ok(())
}

pub async fn mark_all_read(db: &DatabaseConnection, org_id: i32) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE notification SET is_read = true \
         WHERE org_id IN (SELECT $1::int UNION SELECT descendant_id FROM subordination_closure \
         WHERE ancestor_id = $1 AND daterange(valid_from, valid_to, '[)') @> CURRENT_DATE) \
         AND is_read = false",
        [org_id.into()],
    ))
    .await?;
    Ok(())
}
