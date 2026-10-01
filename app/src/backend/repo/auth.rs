use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult};

use crate::types::auth::{UserAccount, UserRoleRow, UserSession};

#[derive(FromQueryResult)]
struct AccountRow {
    id: i32,
    login: String,
    password_hash: String,
    display_name: Option<String>,
    is_active: bool,
}

#[derive(FromQueryResult)]
struct RoleRow {
    org_id: i32,
    role: String,
    org_label: String,
}

#[derive(FromQueryResult)]
struct SessionRow {
    session_id: String,
    user_id: i32,
    active_org_id: Option<i32>,
    active_role: Option<String>,
    login: String,
    display_name: Option<String>,
}

pub async fn find_user_by_login(db: &DatabaseConnection, login: &str) -> Result<Option<UserAccount>, DbErr> {
    let row = AccountRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, login, password_hash, display_name, is_active FROM user_account WHERE login = $1",
        [login.into()],
    ))
    .one(db)
    .await?;

    Ok(row.map(|r| UserAccount {
        id: r.id,
        login: r.login,
        password_hash: r.password_hash,
        display_name: r.display_name,
        is_active: r.is_active,
    }))
}

pub async fn user_roles(db: &DatabaseConnection, user_id: i32) -> Result<Vec<UserRoleRow>, DbErr> {
    let rows = RoleRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT ur.org_id, ur.role, \
                COALESCE(o.short_name, 'org#' || ur.org_id::text) AS org_label \
         FROM user_role ur \
         LEFT JOIN org o ON o.id = ur.org_id \
         WHERE ur.user_id = $1 \
         ORDER BY o.short_name, ur.role",
        [user_id.into()],
    ))
    .all(db)
    .await?;

    Ok(rows.into_iter().map(|r| UserRoleRow {
        org_id: r.org_id,
        role: r.role,
        org_label: r.org_label,
    }).collect())
}

pub async fn create_session(
    db: &DatabaseConnection,
    user_id: i32,
    org_id: Option<i32>,
    role: Option<&str>,
) -> Result<String, DbErr> {
    #[derive(FromQueryResult)]
    struct SessionId {
        id: String,
    }

    let row = SessionId::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO user_session (user_id, active_org_id, active_role) \
         VALUES ($1, $2, $3) \
         RETURNING id::text",
        [
            user_id.into(),
            org_id.map(|v| sea_orm::Value::Int(Some(v))).unwrap_or(sea_orm::Value::Int(None)),
            role.map(|s| sea_orm::Value::String(Some(Box::new(s.to_string())))).unwrap_or(sea_orm::Value::String(None)),
        ],
    ))
    .one(db)
    .await?
    .ok_or(DbErr::Custom("session insert returned nothing".to_string()))?;

    Ok(row.id)
}

pub async fn find_session(db: &DatabaseConnection, session_id: &str) -> Result<Option<UserSession>, DbErr> {
    let row = SessionRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT s.id::text AS session_id, s.user_id, s.active_org_id, s.active_role, \
                ua.login, ua.display_name \
         FROM user_session s \
         JOIN user_account ua ON ua.id = s.user_id \
         WHERE s.id = $1::uuid AND s.expires_at > now() AND ua.is_active = true",
        [session_id.into()],
    ))
    .one(db)
    .await?;

    Ok(row.map(|r| UserSession {
        session_id: r.session_id,
        user_id: r.user_id,
        active_org_id: r.active_org_id,
        active_role: r.active_role,
        login: r.login,
        display_name: r.display_name,
    }))
}

pub async fn delete_session(db: &DatabaseConnection, session_id: &str) -> Result<(), DbErr> {
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM user_session WHERE id = $1::uuid",
        [session_id.into()],
    ))
    .await?;
    Ok(())
}

pub async fn update_session_actor(
    db: &DatabaseConnection,
    session_id: &str,
    org_id: i32,
    role: &str,
) -> Result<(), DbErr> {
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE user_session SET active_org_id = $1, active_role = $2 WHERE id = $3::uuid",
        [org_id.into(), role.into(), session_id.into()],
    ))
    .await?;
    Ok(())
}
