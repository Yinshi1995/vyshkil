use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

use crate::types::auth::{UserAccount, UserRoleRow, UserSession};

#[derive(FromQueryResult)]
struct AccountRow {
    id: i32,
    login: String,
    password_hash: String,
    display_name: Option<String>,
    is_active: bool,
    must_change_password: bool,
    can_see_org_names: bool,
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
    callsign: Option<String>,
    avatar_path: Option<String>,
    can_see_org_names: bool,
}

pub async fn find_user_by_login(db: &DatabaseConnection, login: &str) -> Result<Option<UserAccount>, DbErr> {
    let row = AccountRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, login, password_hash, display_name, is_active, must_change_password, can_see_org_names \
         FROM user_account WHERE login = $1",
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
        must_change_password: r.must_change_password,
        can_see_org_names: r.can_see_org_names,
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
         ORDER BY ur.org_id, o.short_name, ur.role",
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
                ua.login, ua.display_name, ua.callsign, ua.avatar_path, \
                ua.can_see_org_names \
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
        callsign: r.callsign,
        avatar_path: r.avatar_path,
        can_see_org_names: r.can_see_org_names,
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

pub async fn list_users(db: &DatabaseConnection) -> Result<Vec<crate::types::auth::AdminUserRow>, DbErr> {
    #[derive(FromQueryResult)]
    struct UserRow {
        id: i32,
        login: String,
        display_name: Option<String>,
        is_active: bool,
        must_change_password: bool,
        can_see_org_names: bool,
        created_at: String,
    }
    let users = UserRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, login, display_name, is_active, must_change_password, can_see_org_names, \
                to_char(created_at, 'DD.MM.YYYY') AS created_at \
         FROM user_account ORDER BY id",
        [],
    ))
    .all(db)
    .await?;

    let mut result = Vec::with_capacity(users.len());
    for u in users {
        let roles = user_roles(db, u.id).await?;
        result.push(crate::types::auth::AdminUserRow {
            id: u.id,
            login: u.login,
            display_name: u.display_name,
            is_active: u.is_active,
            must_change_password: u.must_change_password,
            can_see_org_names: u.can_see_org_names,
            roles,
            created_at: u.created_at,
        });
    }
    Ok(result)
}

pub async fn create_user(
    db: &DatabaseConnection,
    login: &str,
    password_hash: &str,
    display_name: Option<&str>,
) -> Result<i32, DbErr> {
    #[derive(FromQueryResult)]
    struct NewId { id: i32 }
    let row = NewId::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO user_account (login, password_hash, display_name, must_change_password) \
         VALUES ($1, $2, $3, true) RETURNING id",
        [login.into(), password_hash.into(), display_name.into()],
    ))
    .one(db)
    .await?
    .ok_or(DbErr::Custom("INSERT user_account returned nothing".into()))?;
    Ok(row.id)
}

pub async fn add_user_role(
    db: &DatabaseConnection,
    user_id: i32,
    org_id: i32,
    role: &str,
) -> Result<(), DbErr> {
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO user_role (user_id, org_id, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        [user_id.into(), org_id.into(), role.into()],
    ))
    .await?;
    Ok(())
}

pub async fn reset_password(
    db: &DatabaseConnection,
    user_id: i32,
    password_hash: &str,
) -> Result<(), DbErr> {
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE user_account SET password_hash = $1, must_change_password = true, \
         updated_at = now() WHERE id = $2",
        [password_hash.into(), user_id.into()],
    ))
    .await?;
    Ok(())
}

pub async fn set_password(
    db: &DatabaseConnection,
    user_id: i32,
    password_hash: &str,
) -> Result<(), DbErr> {
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE user_account SET password_hash = $1, must_change_password = false, \
         updated_at = now() WHERE id = $2",
        [password_hash.into(), user_id.into()],
    ))
    .await?;
    Ok(())
}

pub async fn list_submissions(
    db: &DatabaseConnection,
    org_ids: Option<&[i32]>,
) -> Result<Vec<crate::types::auth::AdminSubmissionRow>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        org_label: String,
        source_type: String,
        status: String,
        as_of_date: String,
        updated_at: String,
    }

    let (sql, params): (String, Vec<sea_orm::Value>) = match org_ids {
        Some(ids) if !ids.is_empty() => {
            let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect();
            let sql = format!(
                "SELECT s.id, \
                 COALESCE(o.short_name, 'org#' || s.reporting_org_id::text) AS org_label, \
                 s.source_type, s.status, to_char(s.as_of_date, 'DD.MM.YYYY') AS as_of_date, \
                 to_char(s.updated_at, 'DD.MM.YYYY HH24:MI') AS updated_at \
                 FROM submission s LEFT JOIN org o ON o.id = s.reporting_org_id \
                 WHERE s.reporting_org_id IN ({}) AND s.status != 'draft' \
                 ORDER BY s.updated_at DESC LIMIT 200",
                placeholders.join(", ")
            );
            let params: Vec<sea_orm::Value> = ids.iter().map(|&id| id.into()).collect();
            (sql, params)
        }
        _ => {
            let sql = "SELECT s.id, \
                 COALESCE(o.short_name, 'org#' || s.reporting_org_id::text) AS org_label, \
                 s.source_type, s.status, to_char(s.as_of_date, 'DD.MM.YYYY') AS as_of_date, \
                 to_char(s.updated_at, 'DD.MM.YYYY HH24:MI') AS updated_at \
                 FROM submission s LEFT JOIN org o ON o.id = s.reporting_org_id \
                 WHERE s.status != 'draft' \
                 ORDER BY s.updated_at DESC LIMIT 200".to_string();
            (sql, vec![])
        }
    };

    let rows = Row::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres, &sql, params,
    ))
    .all(db)
    .await?;

    Ok(rows.into_iter().map(|r| crate::types::auth::AdminSubmissionRow {
        id: r.id,
        org_label: r.org_label,
        source_type: r.source_type,
        status: r.status,
        as_of_date: r.as_of_date,
        updated_at: r.updated_at,
    }).collect())
}

pub async fn list_training_groups(
    db: &DatabaseConnection,
    org_ids: Option<&[i32]>,
) -> Result<Vec<crate::types::auth::AdminGroupRow>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        org_id: i32,
        org_label: String,
        training_kind: String,
        vos_label: String,
        site_label: String,
        planned_start: String,
        planned_end: String,
        planned_count: i32,
        arrived_count: i32,
        in_training_count: i32,
    }

    let base_select = "\
        SELECT tg.id, \
        tg.sender_org_id AS org_id, \
        COALESCE(o.short_name, 'org#' || tg.sender_org_id::text) AS org_label, \
        COALESCE(tk.name, '') AS training_kind, \
        COALESCE(v.code || ' — ' || v.title, p.name, c.name, '') AS vos_label, \
        COALESCE(ts.locality, '') AS site_label, \
        COALESCE(to_char(tg.planned_start, 'DD.MM.YYYY'), '') AS planned_start, \
        COALESCE(to_char(tg.planned_end, 'DD.MM.YYYY'), '') AS planned_end, \
        COALESCE((SELECT SUM(ge.count) FROM group_event ge WHERE ge.group_id = tg.id AND ge.event_type = 'planned'), 0)::int AS planned_count, \
        COALESCE((SELECT SUM(ge.count) FROM group_event ge WHERE ge.group_id = tg.id AND ge.event_type = 'arrived'), 0)::int AS arrived_count, \
        ( COALESCE((SELECT SUM(ge.count) FROM group_event ge WHERE ge.group_id = tg.id AND ge.event_type IN ('started','added')), 0) \
        - COALESCE((SELECT SUM(ge.count) FROM group_event ge WHERE ge.group_id = tg.id AND ge.event_type IN ('attrition','completed')), 0) \
        )::int AS in_training_count \
        FROM training_group tg \
        LEFT JOIN org o ON o.id = tg.sender_org_id \
        LEFT JOIN training_kind tk ON tk.id = tg.training_kind_id \
        LEFT JOIN vos v ON v.id = tg.vos_id \
        LEFT JOIN \"position\" p ON p.id = tg.position_id \
        LEFT JOIN course c ON c.id = tg.course_id \
        LEFT JOIN training_site ts ON ts.id = tg.site_id";

    let (sql, params): (String, Vec<sea_orm::Value>) = match org_ids {
        Some(ids) if !ids.is_empty() => {
            let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect();
            let sql = format!(
                "{base_select} WHERE tg.sender_org_id IN ({}) \
                 ORDER BY tg.planned_start DESC NULLS LAST, tg.id DESC LIMIT 500",
                placeholders.join(", ")
            );
            let params: Vec<sea_orm::Value> = ids.iter().map(|&id| id.into()).collect();
            (sql, params)
        }
        _ => {
            let sql = format!(
                "{base_select} ORDER BY tg.planned_start DESC NULLS LAST, tg.id DESC LIMIT 500"
            );
            (sql, vec![])
        }
    };

    let rows = Row::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres, &sql, params,
    ))
    .all(db)
    .await?;

    Ok(rows.into_iter().map(|r| crate::types::auth::AdminGroupRow {
        id: r.id,
        org_id: r.org_id,
        org_label: r.org_label,
        training_kind: r.training_kind,
        vos_label: r.vos_label,
        site_label: r.site_label,
        planned_start: r.planned_start,
        planned_end: r.planned_end,
        planned_count: r.planned_count,
        arrived_count: r.arrived_count,
        in_training_count: r.in_training_count,
    }).collect())
}

pub async fn list_users_by_orgs(
    db: &DatabaseConnection,
    org_ids: &[i32],
) -> Result<Vec<crate::types::auth::AdminUserRow>, DbErr> {
    if org_ids.is_empty() {
        return Ok(vec![]);
    }

    #[derive(FromQueryResult)]
    struct UserRow {
        id: i32,
        login: String,
        display_name: Option<String>,
        is_active: bool,
        must_change_password: bool,
        can_see_org_names: bool,
        created_at: String,
    }

    let placeholders: Vec<String> = org_ids.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect();
    let sql = format!(
        "SELECT DISTINCT ua.id, ua.login, ua.display_name, ua.is_active, ua.must_change_password, \
                ua.can_see_org_names, to_char(ua.created_at, 'DD.MM.YYYY') AS created_at \
         FROM user_account ua \
         JOIN user_role ur ON ur.user_id = ua.id \
         WHERE ur.org_id IN ({}) \
         ORDER BY ua.id",
        placeholders.join(", ")
    );
    let params: Vec<sea_orm::Value> = org_ids.iter().map(|&id| id.into()).collect();

    let users = UserRow::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres, &sql, params,
    ))
    .all(db)
    .await?;

    let mut result = Vec::with_capacity(users.len());
    for u in users {
        let roles = user_roles(db, u.id).await?;
        result.push(crate::types::auth::AdminUserRow {
            id: u.id,
            login: u.login,
            display_name: u.display_name,
            is_active: u.is_active,
            must_change_password: u.must_change_password,
            can_see_org_names: u.can_see_org_names,
            roles,
            created_at: u.created_at,
        });
    }
    Ok(result)
}

pub async fn toggle_active(
    db: &DatabaseConnection,
    user_id: i32,
    is_active: bool,
) -> Result<(), DbErr> {
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE user_account SET is_active = $1, updated_at = now() WHERE id = $2",
        [is_active.into(), user_id.into()],
    ))
    .await?;
    Ok(())
}

pub async fn set_can_see_org_names(
    db: &DatabaseConnection,
    user_id: i32,
    value: bool,
) -> Result<(), DbErr> {
    db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE user_account SET can_see_org_names = $1, updated_at = now() WHERE id = $2",
        [value.into(), user_id.into()],
    ))
    .await?;
    Ok(())
}
