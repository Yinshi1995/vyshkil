//! JSON API для React-фронтенду — прямі Axum-хендлери замість Leptos #[server].
//! Ті самі repo/policy, що й server fn, але без leptos_axum::ResponseOptions / expect_context.

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};
use serde::{Deserialize, Serialize};

use app::backend::{policy, repo};
use app::types::actor::{Actor, Role};
use app::types::auth::{AccountInfo, AuthUser, LoginResponse};

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn err(status: StatusCode, msg: &str) -> impl IntoResponse {
    (status, Json(serde_json::json!({ "error": msg })))
}

fn session_from_cookies(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookies.split(';') {
        let trimmed = part.trim();
        if let Some(val) = trimmed.strip_prefix("session=") {
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

pub(crate) async fn require_auth(db: &DatabaseConnection, headers: &HeaderMap) -> Result<AuthUser, StatusCode> {
    let session_id = session_from_cookies(headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = repo::auth::find_session(db, &session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let actor = match (session.active_org_id, session.active_role.as_deref()) {
        (Some(org_id), Some(role_str)) => Role::parse(role_str).map(|role| Actor { org_id, role }),
        _ => None,
    };

    Ok(AuthUser {
        user_id: session.user_id,
        actor,
        display_name: session.display_name,
    })
}

pub(crate) fn require_admin_actor(user: &AuthUser) -> Result<Actor, StatusCode> {
    let actor = user.actor.ok_or(StatusCode::FORBIDDEN)?;
    if !policy::is_admin(actor) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(actor)
}

fn actor_or_err(user: &AuthUser) -> Result<Actor, StatusCode> {
    user.actor.ok_or(StatusCode::BAD_REQUEST)
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LoginReq {
    login: String,
    password: String,
}

async fn login_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LoginReq>,
) -> impl IntoResponse {
    let db = &state.db;

    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers.get("x-real-ip").and_then(|v| v.to_str().ok()).map(|s| s.to_string())
        })
        .unwrap_or_default();

    if !app::services::auth::check_rate_limit(&client_ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(LoginResponse {
                success: false,
                error: Some("Забагато спроб входу. Спробуйте через 15 хвилин.".into()),
                display_name: None,
                roles: vec![],
                must_change_password: false,
            }),
        )
            .into_response();
    }

    let user = match repo::auth::find_user_by_login(db, &body.login).await {
        Ok(Some(u)) => u,
        _ => {
            app::services::auth::record_failed_attempt(&client_ip);
            return Json(LoginResponse {
                success: false,
                error: Some("Невірний логін або пароль".into()),
                display_name: None,
                roles: vec![],
                must_change_password: false,
            })
            .into_response();
        }
    };

    if !user.is_active {
        return Json(LoginResponse {
            success: false,
            error: Some("Акаунт деактивовано".into()),
            display_name: None,
            roles: vec![],
            must_change_password: false,
        })
        .into_response();
    }

    if !verify_password(&body.password, &user.password_hash) {
        app::services::auth::record_failed_attempt(&client_ip);
        return Json(LoginResponse {
            success: false,
            error: Some("Невірний логін або пароль".into()),
            display_name: None,
            roles: vec![],
            must_change_password: false,
        })
        .into_response();
    }

    let roles = repo::auth::user_roles(db, user.id).await.unwrap_or_default();
    let (first_org, first_role) = roles.first().map(|r| (Some(r.org_id), Some(r.role.as_str()))).unwrap_or((None, None));

    let session_id = match repo::auth::create_session(db, user.id, first_org, first_role).await {
        Ok(id) => id,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    };

    let cookie = format!("session={session_id}; HttpOnly; SameSite=Strict; Path=/; Max-Age=2592000");

    (
        [(header::SET_COOKIE, cookie)],
        Json(LoginResponse {
            success: true,
            error: None,
            display_name: user.display_name,
            roles,
            must_change_password: user.must_change_password,
        }),
    )
        .into_response()
}

async fn logout_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(session_id) = session_from_cookies(&headers) {
        let _ = repo::auth::delete_session(&state.db, &session_id).await;
    }
    let cookie = "session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";
    ([(header::SET_COOKIE, cookie.to_string())], StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct MeResponse {
    user_id: i32,
    actor: Option<ActorDto>,
    display_name: Option<String>,
}

#[derive(Serialize)]
struct ActorDto {
    org_id: i32,
    role: String,
    org_label: String,
}

async fn me_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await.map_err(|s| (s, ""))?;
    let actor_dto = if let Some(actor) = user.actor {
        use sea_orm::{ConnectionTrait, FromQueryResult};
        #[derive(FromQueryResult)]
        struct LabelRow { label: String }
        let label = LabelRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
            state.db.get_database_backend(),
            "SELECT short_name AS label FROM org WHERE id = $1",
            [actor.org_id.into()],
        ))
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .map(|r| r.label)
        .unwrap_or_else(|| format!("Org #{}", actor.org_id));
        Some(ActorDto {
            org_id: actor.org_id,
            role: actor.role.as_str().to_string(),
            org_label: label,
        })
    } else {
        None
    };
    Ok::<_, (StatusCode, &str)>(Json(MeResponse {
        user_id: user.user_id,
        actor: actor_dto,
        display_name: user.display_name,
    }))
}

#[derive(Deserialize)]
struct SwitchActorReq {
    org_id: i32,
    role: String,
}

async fn switch_actor_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SwitchActorReq>,
) -> impl IntoResponse {
    let session_id = session_from_cookies(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    repo::auth::update_session_actor(&state.db, &session_id, body.org_id, &body.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct ChangePasswordReq {
    old_password: String,
    new_password: String,
}

async fn change_password_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ChangePasswordReq>,
) -> impl IntoResponse {
    use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
    use argon2::password_hash::{SaltString, rand_core::OsRng};
    use sea_orm::FromQueryResult;

    let user = require_auth(&state.db, &headers).await?;

    if body.new_password.len() < 6 {
        return Err(StatusCode::BAD_REQUEST);
    }

    #[derive(FromQueryResult)]
    struct PwRow { password_hash: String }
    let pw_row = PwRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT password_hash FROM user_account WHERE id = $1",
        [user.user_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let parsed = PasswordHash::new(&pw_row.password_hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if Argon2::default().verify_password(body.old_password.as_bytes(), &parsed).is_err() {
        return Err(StatusCode::FORBIDDEN);
    }

    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(body.new_password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    repo::auth::set_password(&state.db, user.user_id, &new_hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn account_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    use sea_orm::FromQueryResult;

    let user = require_auth(&state.db, &headers).await?;

    #[derive(FromQueryResult)]
    struct AccountRow {
        login: String,
        display_name: Option<String>,
        created_at: String,
    }
    let row = AccountRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT login, display_name, to_char(created_at, 'YYYY-MM-DD') AS created_at FROM user_account WHERE id = $1",
        [user.user_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let role_rows = repo::auth::user_roles(&state.db, user.user_id)
        .await
        .unwrap_or_default();
    let roles: Vec<String> = role_rows.into_iter().map(|r| format!("{} ({})", r.role, r.org_label)).collect();

    Ok::<_, StatusCode>(Json(AccountInfo {
        login: row.login,
        full_name: row.display_name,
        roles,
        created_at: row.created_at,
    }))
}

// ---------------------------------------------------------------------------
// Request account (public, no auth)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RequestAccountReq {
    contact: String,
    unit: Option<String>,
    message: Option<String>,
}

async fn request_account_handler(
    State(state): State<AppState>,
    Json(body): Json<RequestAccountReq>,
) -> impl IntoResponse {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    if body.contact.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "Вкажіть контактні дані").into_response();
    }

    #[derive(FromQueryResult)]
    struct AdminOrg { org_id: i32 }
    let admin = AdminOrg::find_by_statement(Statement::from_sql_and_values(
        state.db.get_database_backend(),
        "SELECT ur.org_id FROM user_role ur WHERE ur.role = 'admin' LIMIT 1",
        [],
    ))
    .one(&state.db)
    .await
    .ok()
    .flatten();

    let org_id = admin.map(|r| r.org_id).unwrap_or(1);

    let unit_str = body.unit.as_deref().unwrap_or("—");
    let msg_str = body.message.as_deref().unwrap_or("—");
    let notif_body = format!(
        "Контакт: {}\nЧастина: {}\nПовідомлення: {}",
        body.contact.trim(),
        if unit_str.trim().is_empty() { "—" } else { unit_str.trim() },
        if msg_str.trim().is_empty() { "—" } else { msg_str.trim() },
    );

    if repo::notifications::insert(
        &state.db, org_id, "system",
        "Запит на створення облікового запису",
        Some(&notif_body), Some("/settings"),
    )
    .await
    .is_err()
    {
        return err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create notification").into_response();
    }

    StatusCode::NO_CONTENT.into_response()
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct DashboardStatsDto {
    total_orgs: i64,
    total_groups: i64,
    total_submissions: i64,
    total_discrepancies: i64,
    groups_by_kind: Vec<KindCountDto>,
    recent_submissions: Vec<RecentSubDto>,
    recent_discrepancies: Vec<RecentDiscDto>,
}

#[derive(Serialize)]
struct KindCountDto {
    kind: String,
    count: i64,
}

#[derive(Serialize)]
struct RecentSubDto {
    id: i64,
    org_label: String,
    source_type: String,
    updated_at: String,
}

#[derive(Serialize)]
struct RecentDiscDto {
    id: i64,
    org_label: String,
    metric_label: String,
    status: String,
    created_at: String,
}

async fn dashboard_stats_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (org_filter, group_filter, sub_filter, disc_filter) = match &visible {
        None => (String::new(), String::new(), String::new(), String::new()),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            (
                format!(" AND id IN ({list})"),
                format!(" WHERE tg.sender_org_id IN ({list})"),
                format!(" AND s.reporting_org_id IN ({list})"),
                format!(" AND d.org_id IN ({list})"),
            )
        }
    };

    #[derive(FromQueryResult)]
    struct Counts {
        org_count: i64,
        group_count: i64,
        sub_count: i64,
        disc_count: i64,
    }

    let counts_sql = format!(
        "SELECT \
            (SELECT count(*) FROM org WHERE is_active{org_filter}) AS org_count, \
            (SELECT count(*) FROM training_group tg{group_filter}) AS group_count, \
            (SELECT count(*) FROM submission s WHERE true{sub_filter}) AS sub_count, \
            (SELECT count(*) FROM discrepancy d WHERE status = 'open'{disc_filter}) AS disc_count"
    );

    let row = Counts::find_by_statement(Statement::from_string(
        state.db.get_database_backend(),
        counts_sql,
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .unwrap_or(Counts { org_count: 0, group_count: 0, sub_count: 0, disc_count: 0 });

    #[derive(FromQueryResult)]
    struct KindRow {
        kind: String,
        count: i64,
    }
    let kinds_sql = match &visible {
        None => "SELECT tk.code AS kind, count(*) AS count FROM training_group tg \
                 JOIN training_kind tk ON tk.id = tg.training_kind_id \
                 GROUP BY tk.code ORDER BY count DESC".to_string(),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(
                "SELECT tk.code AS kind, count(*) AS count FROM training_group tg \
                 JOIN training_kind tk ON tk.id = tg.training_kind_id \
                 WHERE tg.sender_org_id IN ({list}) \
                 GROUP BY tk.code ORDER BY count DESC"
            )
        }
    };
    let kinds = KindRow::find_by_statement(Statement::from_string(
        state.db.get_database_backend(),
        kinds_sql,
    ))
    .all(&state.db)
    .await
    .unwrap_or_default();

    #[derive(FromQueryResult)]
    struct SubRow {
        id: i64,
        org_label: String,
        source_type: String,
        updated_at: String,
    }
    let subs_sql = match &visible {
        None => "SELECT s.id::bigint AS id, COALESCE(o.short_name, '') AS org_label, \
                 s.source_type, to_char(s.updated_at, 'DD.MM.YYYY HH24:MI') AS updated_at \
                 FROM submission s JOIN org o ON o.id = s.reporting_org_id \
                 ORDER BY s.updated_at DESC LIMIT 5".to_string(),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(
                "SELECT s.id::bigint AS id, COALESCE(o.short_name, '') AS org_label, \
                 s.source_type, to_char(s.updated_at, 'DD.MM.YYYY HH24:MI') AS updated_at \
                 FROM submission s JOIN org o ON o.id = s.reporting_org_id \
                 WHERE s.reporting_org_id IN ({list}) \
                 ORDER BY s.updated_at DESC LIMIT 5"
            )
        }
    };
    let recent_subs = SubRow::find_by_statement(Statement::from_string(
        state.db.get_database_backend(),
        subs_sql,
    ))
    .all(&state.db)
    .await
    .unwrap_or_default();

    #[derive(FromQueryResult)]
    struct DiscRow {
        id: i64,
        org_label: String,
        metric_label: String,
        status: String,
        created_at: String,
    }
    let discs_sql = match &visible {
        None => "SELECT d.id::bigint AS id, COALESCE(o.short_name, '') AS org_label, \
                 d.metric AS metric_label, d.status, to_char(d.created_at, 'DD.MM.YYYY HH24:MI') AS created_at \
                 FROM discrepancy d JOIN org o ON o.id = d.org_id \
                 ORDER BY d.created_at DESC LIMIT 5".to_string(),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(
                "SELECT d.id::bigint AS id, COALESCE(o.short_name, '') AS org_label, \
                 d.metric AS metric_label, d.status, to_char(d.created_at, 'DD.MM.YYYY HH24:MI') AS created_at \
                 FROM discrepancy d JOIN org o ON o.id = d.org_id \
                 WHERE d.org_id IN ({list}) \
                 ORDER BY d.created_at DESC LIMIT 5"
            )
        }
    };
    let recent_discs = DiscRow::find_by_statement(Statement::from_string(
        state.db.get_database_backend(),
        discs_sql,
    ))
    .all(&state.db)
    .await
    .unwrap_or_default();

    Ok::<_, StatusCode>(Json(DashboardStatsDto {
        total_orgs: row.org_count,
        total_groups: row.group_count,
        total_submissions: row.sub_count,
        total_discrepancies: row.disc_count,
        groups_by_kind: kinds.into_iter().map(|k| KindCountDto { kind: k.kind, count: k.count }).collect(),
        recent_submissions: recent_subs.into_iter().map(|s| RecentSubDto {
            id: s.id, org_label: s.org_label, source_type: s.source_type, updated_at: s.updated_at,
        }).collect(),
        recent_discrepancies: recent_discs.into_iter().map(|d| RecentDiscDto {
            id: d.id, org_label: d.org_label, metric_label: d.metric_label, status: d.status, created_at: d.created_at,
        }).collect(),
    }))
}

// ---------------------------------------------------------------------------
// Orgs
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct OrgChildDto {
    id: i32,
    label: String,
}

async fn org_detail_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let allowed = policy::can_view_org(&state.db, actor, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    let detail = repo::orgs::org_detail(&state.db, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(detail))
}

async fn org_children_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let allowed = policy::can_view_org(&state.db, actor, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    let children = repo::orgs::direct_children(&state.db, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let dtos: Vec<OrgChildDto> = children.into_iter().map(|(id, label)| OrgChildDto { id, label }).collect();
    Ok(Json(dtos))
}

async fn org_groups_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let allowed = policy::can_view_org(&state.db, actor, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    let subtree = repo::orgs::subtree_org_ids(&state.db, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let groups = repo::auth::list_training_groups(&state.db, Some(&subtree))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(groups))
}

async fn org_submissions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let allowed = policy::can_view_org(&state.db, actor, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }

    let subtree = repo::orgs::subtree_org_ids(&state.db, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let subs = repo::auth::list_submissions(&state.db, Some(&subtree))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(subs))
}

async fn org_users_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    require_admin_actor(&user)?;

    let subtree = repo::orgs::subtree_org_ids(&state.db, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let users = repo::auth::list_users_by_orgs(&state.db, &subtree)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(users))
}

// ---------------------------------------------------------------------------
// Discrepancies
// ---------------------------------------------------------------------------

async fn discrepancies_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = repo::reconciliation::list_discrepancies(&state.db, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filtered = match visible {
        None => rows,
        Some(ids) => rows.into_iter().filter(|r| ids.contains(&r.org_id)).collect(),
    };

    Ok::<_, StatusCode>(Json(filtered))
}

#[derive(Deserialize)]
struct UpdateDiscrepancyReq {
    status: String,
    resolution_note: Option<String>,
}

async fn update_discrepancy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(disc_id): Path<i32>,
    Json(body): Json<UpdateDiscrepancyReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let allowed = ["resolved", "dismissed", "in_progress", "open"];
    if !allowed.contains(&body.status.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let org_id = repo::reconciliation::update_discrepancy_status(
        &state.db,
        disc_id,
        &body.status,
        body.resolution_note.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(ids) = &visible {
        if !ids.contains(&org_id) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn compare_discrepancy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(disc_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = actor_or_err(&user)?;

    let comparison = repo::reconciliation::compare_discrepancy(&state.db, disc_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok::<_, StatusCode>(Json(comparison))
}

// ---------------------------------------------------------------------------
// Admin: user management
// ---------------------------------------------------------------------------

async fn admin_list_users_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    require_admin_actor(&user)?;

    let users = repo::auth::list_users(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(users))
}

#[derive(Deserialize)]
struct CreateUserReq {
    login: String,
    display_name: Option<String>,
    temp_password: String,
}

async fn admin_create_user_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateUserReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let admin = require_admin_actor(&user)?;

    if body.login.len() < 3 || body.temp_password.len() < 6 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let password_hash = hash_password(&body.temp_password).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user_id = repo::auth::create_user(
        &state.db,
        &body.login,
        &password_hash,
        body.display_name.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    repo::auth::add_user_role(&state.db, user_id, admin.org_id, "viewer")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "id": user_id })))
}

#[derive(Deserialize)]
struct ResetPasswordReq {
    temp_password: String,
}

async fn admin_reset_password_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i32>,
    Json(body): Json<ResetPasswordReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    require_admin_actor(&user)?;

    let password_hash = hash_password(&body.temp_password).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    repo::auth::reset_password(&state.db, user_id, &password_hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_toggle_active_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    require_admin_actor(&user)?;

    use sea_orm::FromQueryResult;
    #[derive(FromQueryResult)]
    struct ActiveRow { is_active: bool }
    let row = ActiveRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT is_active FROM user_account WHERE id = $1",
        [user_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    repo::auth::toggle_active(&state.db, user_id, !row.is_active)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Password hashing
// ---------------------------------------------------------------------------

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let Ok(parsed) = PasswordHash::new(hash) else { return false };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

fn hash_password(password: &str) -> Result<String, ()> {
    use argon2::{Argon2, PasswordHasher};
    use argon2::password_hash::{SaltString, rand_core::OsRng};
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| ())
}

// ---------------------------------------------------------------------------
// Org search (autocomplete)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct OrgSearchQuery {
    q: Option<String>,
    scope: Option<String>,
}

async fn org_search_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<OrgSearchQuery>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let query = params.q.unwrap_or_default();
    let results = repo::orgs::search_orgs(&state.db, &query)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if params.scope.as_deref() == Some("visible") {
        let actor = actor_or_err(&user)?;
        let visible = policy::visible_org_ids(&state.db, actor)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some(ids) = visible {
            let filtered = results
                .into_iter()
                .filter(|r| ids.contains(&r.org_id))
                .collect::<Vec<_>>();
            return Ok::<_, StatusCode>(Json(filtered));
        }
    }

    Ok::<_, StatusCode>(Json(results))
}

// ---------------------------------------------------------------------------
// Documents: generation endpoints (D1–D6)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct DocGenOrgDate {
    org_id: i32,
    date: String,
}

#[derive(Deserialize)]
struct DocGenDate {
    date: String,
}

async fn generate_d1_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenOrgDate>,
) -> impl IntoResponse {
    use app::backend::{documents, repo};
    use chrono::Datelike;

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    if !policy::can_view_org(&state.db, actor, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let any_day = chrono::NaiveDate::parse_from_str(&body.date, "%Y-%m-%d")
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let week_start =
        any_day - chrono::Duration::days(any_day.weekday().num_days_from_monday() as i64);

    let org_label = repo::documents::org_label(&state.db, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut days = Vec::with_capacity(7);
    for offset in 0..7 {
        let date = week_start + chrono::Duration::days(offset);
        let date_str = date.format("%Y-%m-%d").to_string();
        let rollup = repo::documents::daily_training_rollup(&state.db, body.org_id, &date_str)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        days.push((date, rollup));
    }

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d1::build_weekly_file(&mut workbook, &org_label, week_start, &days)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bytes = workbook.save_to_buffer().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let week_str = week_start.format("%Y-%m-%d").to_string();
    let filename = format!("D1_{org_label}_{week_str}.xlsx");
    Ok(xlsx_response(bytes, &filename))
}

async fn generate_d2_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenDate>,
) -> impl IntoResponse {
    use app::backend::{documents, repo};
    use chrono::Datelike;

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let any_day = chrono::NaiveDate::parse_from_str(&body.date, "%Y-%m-%d")
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let week_start =
        any_day - chrono::Duration::days(any_day.weekday().num_days_from_monday() as i64);

    let corps = repo::documents::top_level_orgs(&state.db, &body.date)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for (corps_id, _) in &corps {
        if !policy::can_view_org(&state.db, actor, *corps_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let mut days = Vec::with_capacity(7);
    for offset in 0..7 {
        let date = week_start + chrono::Duration::days(offset);
        let date_str = date.format("%Y-%m-%d").to_string();
        let mut day_corps = Vec::with_capacity(corps.len());
        for (corps_id, corps_label) in &corps {
            let rollup = repo::documents::daily_training_rollup(&state.db, *corps_id, &date_str)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if rollup.main.is_empty() && rollup.out_of_zone.is_empty() {
                continue;
            }
            let mut orgs = rollup.main;
            orgs.extend(rollup.out_of_zone);
            day_corps.push((corps_label.clone(), orgs));
        }
        days.push(documents::d2::DayBlock { date, corps: day_corps });
    }

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d2::build_week_sheet(&mut workbook, week_start, &days)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bytes = workbook.save_to_buffer().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let week_str = week_start.format("%Y-%m-%d").to_string();
    let filename = format!("D2_Контролька_{week_str}.xlsx");
    Ok(xlsx_response(bytes, &filename))
}

async fn generate_d3_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenDate>,
) -> impl IntoResponse {
    use app::backend::{documents, repo};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let today = chrono::NaiveDate::parse_from_str(&body.date, "%Y-%m-%d")
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let yesterday = today - chrono::Duration::days(1);

    let corps = repo::documents::top_level_orgs(&state.db, &body.date)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for (corps_id, _) in &corps {
        if !policy::can_view_org(&state.db, actor, *corps_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let today_str = today.format("%Y-%m-%d").to_string();
    let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
    let mut corps_days = Vec::with_capacity(corps.len());
    for (corps_id, corps_label) in &corps {
        let rollup_today = repo::documents::daily_training_rollup(&state.db, *corps_id, &today_str)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let rollup_yesterday = repo::documents::daily_training_rollup(&state.db, *corps_id, &yesterday_str)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        corps_days.push(documents::d3::CorpsDay {
            label: corps_label.clone(),
            today: rollup_today,
            yesterday: rollup_yesterday,
        });
    }

    let template_path = std::env::var("DOCUMENTS_D3_TEMPLATE").ok();
    let template_text = template_path.as_ref().and_then(|p| std::fs::read_to_string(p).ok());
    let template = template_text.as_deref().unwrap_or(documents::d3::DEFAULT_TEMPLATE);

    let paragraphs = documents::d3::render_paragraphs(template, today, &corps_days);
    let bytes = documents::d3::build_docx(&paragraphs)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("D3_Говорілка_{}.docx", body.date);
    Ok(docx_response(bytes, &filename))
}

async fn generate_d4_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenDate>,
) -> impl IntoResponse {
    use app::backend::{documents, repo};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let today = chrono::NaiveDate::parse_from_str(&body.date, "%Y-%m-%d")
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let yesterday = today - chrono::Duration::days(1);

    let corps = repo::documents::top_level_orgs(&state.db, &body.date)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for (corps_id, _) in &corps {
        if !policy::can_view_org(&state.db, actor, *corps_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let today_str = today.format("%Y-%m-%d").to_string();
    let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
    let mut corps_slides = Vec::with_capacity(corps.len());
    for (corps_id, corps_label) in &corps {
        let rollup_today = repo::documents::daily_training_rollup(&state.db, *corps_id, &today_str)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let rollup_yesterday = repo::documents::daily_training_rollup(&state.db, *corps_id, &yesterday_str)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        corps_slides.push(documents::d4::CorpsSlide {
            label: corps_label.clone(),
            today: rollup_today,
            yesterday: rollup_yesterday,
        });
    }

    let bytes = documents::d4::build_pptx(today, &corps_slides)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("D4_Підготовка_{}.pptx", body.date);
    Ok(pptx_response(bytes, &filename))
}

#[derive(Serialize)]
struct D4KindJson {
    total: i64,
    finishing: i64,
    started: i64,
}

#[derive(Serialize)]
struct D4UnitJson {
    label: String,
    bzvp: D4KindJson,
    special: D4KindJson,
    adaptation: D4KindJson,
}

#[derive(Serialize)]
struct D4CorpsJson {
    label: String,
    units: Vec<D4UnitJson>,
    yesterday_bzvp: i64,
    yesterday_special: i64,
    yesterday_adaptation: i64,
}

#[derive(Serialize)]
struct D4DataJson {
    date: String,
    corps: Vec<D4CorpsJson>,
}

async fn d4_data_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenDate>,
) -> impl IntoResponse {
    use app::backend::repo;

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let today = chrono::NaiveDate::parse_from_str(&body.date, "%Y-%m-%d")
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let yesterday = today - chrono::Duration::days(1);

    let corps = repo::documents::top_level_orgs(&state.db, &body.date)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for (corps_id, _) in &corps {
        if !policy::can_view_org(&state.db, actor, *corps_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let today_str = today.format("%Y-%m-%d").to_string();
    let yesterday_str = yesterday.format("%Y-%m-%d").to_string();

    let mut result_corps = Vec::with_capacity(corps.len());
    for (corps_id, corps_label) in &corps {
        let rollup_today = repo::documents::daily_training_rollup(&state.db, *corps_id, &today_str)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let rollup_yesterday = repo::documents::daily_training_rollup(&state.db, *corps_id, &yesterday_str)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let units: Vec<D4UnitJson> = rollup_today.main.iter().chain(rollup_today.out_of_zone.iter())
            .map(|r| D4UnitJson {
                label: r.org_label.clone(),
                bzvp: D4KindJson { total: r.bzvp.total, finishing: r.bzvp.finishing_today, started: r.bzvp.started_today },
                special: D4KindJson { total: r.special.total, finishing: r.special.finishing_today, started: r.special.started_today },
                adaptation: D4KindJson { total: r.adaptation.total, finishing: r.adaptation.finishing_today, started: r.adaptation.started_today },
            })
            .collect();

        let (yb, ys, ya) = {
            let mut b = 0i64; let mut s = 0i64; let mut a = 0i64;
            for r in rollup_yesterday.main.iter().chain(rollup_yesterday.out_of_zone.iter()) {
                b += r.bzvp.total; s += r.special.total; a += r.adaptation.total;
            }
            (b, s, a)
        };

        result_corps.push(D4CorpsJson {
            label: corps_label.clone(),
            units,
            yesterday_bzvp: yb,
            yesterday_special: ys,
            yesterday_adaptation: ya,
        });
    }

    Ok(Json(D4DataJson { date: body.date, corps: result_corps }))
}

async fn generate_d6_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenDate>,
) -> impl IntoResponse {
    use app::backend::{documents, repo};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let corps = repo::documents::top_level_orgs(&state.db, &body.date)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for (corps_id, _) in &corps {
        if !policy::can_view_org(&state.db, actor, *corps_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let rows = repo::documents::transferred_orgs_report(&state.db, &body.date)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d6::build_transferred_report(&mut workbook, &body.date, &rows)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bytes = workbook.save_to_buffer().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("D6_Передані_{}.xlsx", body.date);
    Ok(xlsx_response(bytes, &filename))
}

async fn generate_d5_fah_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenOrgDate>,
) -> impl IntoResponse {
    use app::backend::{documents, repo};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    if !policy::can_view_org(&state.db, actor, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let org_label = repo::documents::org_label(&state.db, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let groups = repo::documents::group_detail_for_corps(&state.db, body.org_id, &body.date, "special")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d5::build_fah(&mut workbook, &org_label, &body.date, &groups)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bytes = workbook.save_to_buffer().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("D5_Фах_{org_label}_{}.xlsx", body.date);
    Ok(xlsx_response(bytes, &filename))
}

async fn generate_d5_bps_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenOrgDate>,
) -> impl IntoResponse {
    use app::backend::{documents, repo};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    if !policy::can_view_org(&state.db, actor, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let org_label = repo::documents::org_label(&state.db, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let groups = repo::documents::group_detail_bps(&state.db, body.org_id, &body.date)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d5::build_bps(&mut workbook, &org_label, &body.date, &groups)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bytes = workbook.save_to_buffer().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("D5_БпС_{org_label}_{}.xlsx", body.date);
    Ok(xlsx_response(bytes, &filename))
}

async fn generate_d5_kvid_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenOrgDate>,
) -> impl IntoResponse {
    use app::backend::{documents, repo};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    if !policy::can_view_org(&state.db, actor, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let org_label = repo::documents::org_label(&state.db, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let rows = repo::documents::staffing_for_corps(&state.db, body.org_id, &body.date, "squad_leaders")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d5::build_kvid(&mut workbook, &org_label, &body.date, &rows)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bytes = workbook.save_to_buffer().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("D5_КВід_{org_label}_{}.xlsx", body.date);
    Ok(xlsx_response(bytes, &filename))
}

async fn generate_d5_ivs_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenOrgDate>,
) -> impl IntoResponse {
    use app::backend::{documents, repo};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    if !policy::can_view_org(&state.db, actor, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let org_label = repo::documents::org_label(&state.db, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let rows = repo::documents::staffing_for_corps(&state.db, body.org_id, &body.date, "instructors")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d5::build_ivs(&mut workbook, &org_label, &body.date, &rows)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bytes = workbook.save_to_buffer().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("D5_ІВС_{org_label}_{}.xlsx", body.date);
    Ok(xlsx_response(bytes, &filename))
}

async fn generate_d5_terminy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DocGenOrgDate>,
) -> impl IntoResponse {
    use app::backend::{documents, repo};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    if !policy::can_view_org(&state.db, actor, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let org_label = repo::documents::org_label(&state.db, body.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let (bzvp, special, adaptation) =
        repo::documents::terminy_detail(&state.db, body.org_id, &body.date)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    documents::d5::build_terminy(&mut workbook, &org_label, &body.date, &bzvp, &special, &adaptation)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bytes = workbook.save_to_buffer().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("D5_Терміни_{org_label}_{}.xlsx", body.date);
    Ok(xlsx_response(bytes, &filename))
}

// ---------------------------------------------------------------------------
// WhatsApp admin
// ---------------------------------------------------------------------------

const SUBJECT_PAIR_CODE: &str = "vyshkil.notifier.whatsapp.pair.code";
const SUBJECT_LOGOUT: &str = "vyshkil.notifier.whatsapp.logout";
const SUBJECT_TEST: &str = "vyshkil.notifier.whatsapp.test";

fn nats_client(state: &AppState) -> Result<bus::async_nats::Client, StatusCode> {
    state
        .nats
        .lock()
        .expect("shared NATS mutex отруєний")
        .clone()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

#[derive(Deserialize)]
struct PairReq {
    phone: String,
}

#[derive(Deserialize)]
struct PairCodeReply {
    pairing_code: Option<String>,
}

async fn whatsapp_pair_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PairReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    require_admin_actor(&user)?;

    let client = nats_client(&state)?;
    let payload = serde_json::json!({ "phone": body.phone }).to_string();
    let raw = bus::request(&client, SUBJECT_PAIR_CODE, payload.into_bytes())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let raw_str = String::from_utf8(raw).map_err(|_| StatusCode::BAD_GATEWAY)?;

    let reply: PairCodeReply =
        serde_json::from_str(&raw_str).map_err(|_| StatusCode::BAD_GATEWAY)?;
    let code = reply
        .pairing_code
        .ok_or(StatusCode::BAD_GATEWAY)?;

    Ok::<_, StatusCode>(Json(serde_json::json!({ "pairing_code": code })))
}

async fn whatsapp_logout_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    require_admin_actor(&user)?;

    let client = nats_client(&state)?;
    let _raw = bus::request(&client, SUBJECT_LOGOUT, b"{}".to_vec())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct TestNotifReq {
    org_id: i32,
}

async fn whatsapp_test_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TestNotifReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    require_admin_actor(&user)?;

    let client = nats_client(&state)?;
    let payload = serde_json::json!({ "org_id": body.org_id }).to_string();
    let raw = bus::request(&client, SUBJECT_TEST, payload.into_bytes())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let raw_str = String::from_utf8(raw).map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok::<_, StatusCode>(raw_str)
}

// ---------------------------------------------------------------------------
// Binary response helpers
// ---------------------------------------------------------------------------

fn xlsx_response(bytes: Vec<u8>, filename: &str) -> impl IntoResponse {
    binary_response(
        bytes,
        filename,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    )
}

fn docx_response(bytes: Vec<u8>, filename: &str) -> impl IntoResponse {
    binary_response(
        bytes,
        filename,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    )
}

fn pptx_response(bytes: Vec<u8>, filename: &str) -> impl IntoResponse {
    binary_response(
        bytes,
        filename,
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    )
}

fn binary_response(bytes: Vec<u8>, filename: &str, content_type: &str) -> impl IntoResponse {
    let disposition = format!(
        "attachment; filename*=UTF-8''{}",
        urlencoding::encode(filename)
    );
    (
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                disposition,
            ),
        ],
        bytes,
    )
}

// ---------------------------------------------------------------------------
// Training: list all groups visible to actor
// ---------------------------------------------------------------------------

async fn all_groups_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let groups = repo::auth::list_training_groups(&state.db, visible.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(groups))
}

// ---------------------------------------------------------------------------
// Submissions: recent for import page
// ---------------------------------------------------------------------------

async fn recent_submissions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let subs = repo::auth::list_submissions(&state.db, visible.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(subs))
}

// ---------------------------------------------------------------------------
// Data Workspace
// ---------------------------------------------------------------------------

async fn data_groups_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let groups = repo::groups::list_groups_extended(&state.db, visible.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(groups))
}

#[derive(Deserialize)]
struct UpdateGroupField {
    field: String,
    value: String,
}

async fn data_update_group_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<i32>,
    Json(body): Json<UpdateGroupField>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;

    let ok = repo::groups::update_group_field(&state.db, group_id, &body.field, &body.value)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if ok {
        Ok::<_, StatusCode>(Json(serde_json::json!({ "ok": true })))
    } else {
        Ok(Json(serde_json::json!({ "ok": false, "error": "invalid field or not found" })))
    }
}

async fn data_delete_group_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;

    let ok = repo::groups::delete_group(&state.db, group_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(serde_json::json!({ "ok": ok })))
}

async fn data_group_events_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = actor_or_err(&user)?;

    let events = repo::groups::list_group_events(&state.db, group_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(events))
}

#[derive(Deserialize)]
struct AddEventBody {
    event_type: String,
    count: i32,
    occurred_on: String,
    reason_id: Option<i32>,
    note: Option<String>,
}

async fn data_add_event_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(group_id): Path<i32>,
    Json(body): Json<AddEventBody>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;

    if body.count <= 0 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if body.occurred_on.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let txn = state.db.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    policy::set_session_actor(&txn, _actor).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let id = repo::groups::add_group_event(
        &txn,
        group_id,
        &body.event_type,
        body.count,
        &body.occurred_on,
        body.reason_id,
        body.note.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    txn.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let event_label = match body.event_type.as_str() {
        "planned" => "План",
        "arrived" => "Прибуло",
        "started" => "Розпочали",
        "added" => "Додано",
        "attrition" => "Вибуло",
        "completed" => "Завершили",
        "vos_awarded" => "ВОС присвоєно",
        "vos_not_awarded" => "ВОС не присвоєно",
        "correction" => "Корекція",
        _ => &body.event_type,
    };
    if let Ok(Some(org_id)) = repo::groups::group_org_id(&state.db, group_id).await {
        let title = format!("{event_label}: {}", body.count);
        let _ = repo::notifications::insert(
            &state.db, org_id, "group_event", &title,
            Some(&format!("Група #{group_id}, дата {}", body.occurred_on)),
            Some("/data"),
        ).await;
    }

    let _ = repo::reconciliation::refresh_horizontal(&state.db, group_id).await;
    let _ = repo::reconciliation::refresh_temporal(&state.db, group_id).await;

    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

async fn data_delete_event_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((group_id, event_id)): Path<(i32, i32)>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;

    let txn = state.db.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    policy::set_session_actor(&txn, _actor).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let ok = repo::groups::delete_group_event(&txn, event_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    txn.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if ok {
        let _ = repo::reconciliation::refresh_horizontal(&state.db, group_id).await;
        let _ = repo::reconciliation::refresh_temporal(&state.db, group_id).await;
    }

    Ok::<_, StatusCode>(Json(serde_json::json!({ "ok": ok })))
}

#[derive(Deserialize)]
struct CreateGroupBody {
    sender_org_id: i32,
    training_kind_id: i32,
    site_id: i32,
    vos_id: Option<i32>,
    position_id: Option<i32>,
    course_id: Option<i32>,
    bzvp_program_id: Option<i32>,
    organizer_org_id: Option<i32>,
    planned_start: String,
    planned_end: String,
    equipment_text: Option<String>,
    basis_doc_number: Option<String>,
    basis_doc_date: Option<String>,
    note: Option<String>,
    planned_count: i32,
    #[serde(default)]
    arrived_count: i32,
    #[serde(default)]
    in_training_count: i32,
}

async fn data_create_group_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateGroupBody>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;

    if body.planned_count <= 0 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if body.arrived_count < 0 || body.in_training_count < 0 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if body.planned_start.is_empty() || body.planned_end.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = repo::groups::create_group_with_events(
        &state.db,
        body.sender_org_id,
        body.training_kind_id,
        body.site_id,
        body.vos_id,
        body.position_id,
        body.course_id,
        body.bzvp_program_id,
        body.organizer_org_id,
        &body.planned_start,
        &body.planned_end,
        body.equipment_text.as_deref(),
        body.basis_doc_number.as_deref(),
        body.basis_doc_date.as_deref(),
        body.note.as_deref(),
        body.planned_count,
        body.arrived_count,
        body.in_training_count,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

async fn training_kinds_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = actor_or_err(&user)?;

    let kinds = repo::groups::list_training_kinds(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(kinds))
}

async fn attrition_reasons_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = actor_or_err(&user)?;

    let reasons = repo::groups::list_attrition_reasons(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(reasons))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    limit: Option<u32>,
}

async fn site_search_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<SearchQuery>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = actor_or_err(&user)?;

    let q = params.q.unwrap_or_default();
    let limit = params.limit.unwrap_or(10).min(20) as i64;

    let sites = repo::groups::search_training_sites(&state.db, &q, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(sites))
}

// ---------------------------------------------------------------------------
// Notifications
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct NotificationsQuery {
    limit: Option<i64>,
}

async fn notifications_list_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<NotificationsQuery>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;
    let limit = params.limit.unwrap_or(20).min(50);
    let items = repo::notifications::list_for_org(&state.db, actor.org_id, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(items))
}

async fn notification_mark_all_read_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;
    repo::notifications::mark_all_read(&state.db, actor.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "ok": true })))
}

async fn notification_mark_read_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;
    repo::notifications::mark_read(&state.db, actor.org_id, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Import upload
// ---------------------------------------------------------------------------

async fn import_upload_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user = require_auth(&state.db, &headers).await
        .map_err(|s| (s, Json(serde_json::json!({"error": "Unauthorized"}))))?;
    let _actor = require_admin_actor(&user)
        .map_err(|s| (s, Json(serde_json::json!({"error": "Forbidden"}))))?;

    let mut kind: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Bad multipart"}))))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "kind" => {
                kind = Some(field.text().await
                    .map_err(|_| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Bad kind field"}))))?);
            }
            "file" => {
                let bytes = field.bytes().await
                    .map_err(|_| (StatusCode::PAYLOAD_TOO_LARGE, Json(serde_json::json!({"error": "File too large"}))))?;
                if bytes.len() > 20 * 1024 * 1024 {
                    return Err((StatusCode::PAYLOAD_TOO_LARGE, Json(serde_json::json!({"error": "File exceeds 20MB"}))));
                }
                file_bytes = Some(bytes.to_vec());
            }
            _ => {}
        }
    }

    let kind = kind.ok_or_else(|| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Missing kind"}))))?;
    let bytes = file_bytes.ok_or_else(|| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Missing file"}))))?;
    let db = &state.db;
    let ise = |_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal error"})));
    let parse_err = |e: app::backend::import::fah::ParseError| {
        (StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error": e.0})))
    };

    let today = chrono::Utc::now().date_naive();

    let imported: i64 = match kind.as_str() {
        "fah" => {
            let raw = app::backend::import::fah::extract(&bytes).map_err(parse_err)?;
            let rows = repo::imports_fah::resolve_rows(db, raw).await.map_err(ise)?;
            let validated = validate_import_rows(&rows, today)?;
            let txn = db.begin().await.map_err(ise)?;
            policy::set_session_actor(&txn, _actor).await.map_err(ise)?;
            let sub_id = create_import_submission(&txn, _actor.org_id).await.map_err(ise)?;
            let ids = repo::groups::commit_group_rows(&txn, &validated, sub_id).await.map_err(ise)?;
            let count = ids.len() as i64;
            txn.commit().await.map_err(ise)?;
            count
        }
        "bps" => {
            let raw = app::backend::import::bps::extract(&bytes)
                .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error": e.0}))))?;
            let rows = repo::imports_bps::resolve_rows(db, raw).await.map_err(ise)?;
            let validated = validate_import_rows(&rows, today)?;
            let txn = db.begin().await.map_err(ise)?;
            policy::set_session_actor(&txn, _actor).await.map_err(ise)?;
            let sub_id = create_import_submission(&txn, _actor.org_id).await.map_err(ise)?;
            let ids = repo::groups::commit_group_rows(&txn, &validated, sub_id).await.map_err(ise)?;
            let count = ids.len() as i64;
            txn.commit().await.map_err(ise)?;
            count
        }
        "terminy" => {
            let raw = app::backend::import::terminy::extract(&bytes)
                .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error": e.0}))))?;
            let rows = repo::imports_terminy::resolve_rows(db, raw).await.map_err(ise)?;
            let validated = validate_import_rows(&rows, today)?;
            let txn = db.begin().await.map_err(ise)?;
            policy::set_session_actor(&txn, _actor).await.map_err(ise)?;
            let sub_id = create_import_submission(&txn, _actor.org_id).await.map_err(ise)?;
            let ids = repo::groups::commit_group_rows(&txn, &validated, sub_id).await.map_err(ise)?;
            let count = ids.len() as i64;
            txn.commit().await.map_err(ise)?;
            count
        }
        "archive" => {
            let raw = app::backend::import::vch_archive::extract(&bytes)
                .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error": e.0}))))?;
            let rows = repo::imports_vch_archive::resolve_rows(db, raw).await.map_err(ise)?;
            let validated = validate_import_rows(&rows, today)?;
            let txn = db.begin().await.map_err(ise)?;
            policy::set_session_actor(&txn, _actor).await.map_err(ise)?;
            let sub_id = create_import_submission(&txn, _actor.org_id).await.map_err(ise)?;
            let ids = repo::groups::commit_group_rows(&txn, &validated, sub_id).await.map_err(ise)?;
            let count = ids.len() as i64;
            txn.commit().await.map_err(ise)?;
            count
        }
        "kvid" => {
            let raw = app::backend::import::kvid::extract(&bytes)
                .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error": e.0}))))?;
            let rows = repo::imports_kvid::resolve_rows(db, raw).await.map_err(ise)?;
            let txn = db.begin().await.map_err(ise)?;
            policy::set_session_actor(&txn, _actor).await.map_err(ise)?;
            let sub_id = create_import_submission(&txn, _actor.org_id).await.map_err(ise)?;
            let as_of = today.to_string();
            let mut count = 0i64;
            for row in &rows {
                let org_id = row.org_id.unwrap_or(_actor.org_id);
                repo::staffing::insert_snapshot(&txn, org_id, &as_of, "kvid", sub_id, row)
                    .await
                    .map_err(ise)?;
                count += 1;
            }
            txn.commit().await.map_err(ise)?;
            count
        }
        "ivs" => {
            let raw = app::backend::import::ivs::extract(&bytes)
                .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error": e.0}))))?;
            let resolved = repo::imports_ivs::resolve_rows(db, raw).await.map_err(ise)?;
            let txn = db.begin().await.map_err(ise)?;
            policy::set_session_actor(&txn, _actor).await.map_err(ise)?;
            let sub_id = create_import_submission(&txn, _actor.org_id).await.map_err(ise)?;
            let as_of = today.to_string();
            let mut count = 0i64;
            for row in &resolved.staffing {
                let org_id = row.org_id.unwrap_or(_actor.org_id);
                repo::staffing::insert_instructor_snapshot(&txn, org_id, &as_of, sub_id, row)
                    .await
                    .map_err(ise)?;
                count += 1;
            }
            if !resolved.groups.is_empty() {
                let validated = validate_import_rows(&resolved.groups, today)?;
                let ids = repo::groups::commit_group_rows(&txn, &validated, sub_id).await.map_err(ise)?;
                count += ids.len() as i64;
            }
            txn.commit().await.map_err(ise)?;
            count
        }
        _ => return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Unknown kind"})))),
    };

    Ok(Json(serde_json::json!({ "imported": imported })))
}

type ImportValidated = Vec<(app::types::submission::GroupFormRow, repo::groups::ValidatedRow)>;
type ApiError = (StatusCode, Json<serde_json::Value>);

fn validate_import_rows(
    rows: &[app::types::submission::GroupFormRow],
    today: chrono::NaiveDate,
) -> Result<ImportValidated, ApiError> {
    let mut out = Vec::with_capacity(rows.len());
    let mut errors = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        match repo::groups::validate_row(row, today) {
            Ok(v) => out.push((row.clone(), v)),
            Err((field, msg)) => errors.push(format!("Рядок {}: {} — {}", i + 1, field, msg)),
        }
    }
    if !errors.is_empty() {
        let msg = format!("{} помилок валідації:\n{}", errors.len(), errors.into_iter().take(10).collect::<Vec<_>>().join("\n"));
        return Err((StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({"error": msg}))));
    }
    Ok(out)
}

async fn create_import_submission(
    db: &impl ConnectionTrait,
    org_id: i32,
) -> Result<i32, sea_orm::DbErr> {
    use sea_orm::{FromQueryResult, Statement};
    #[derive(FromQueryResult)]
    struct NewId { id: i32 }
    let today = chrono::Utc::now().date_naive().to_string();
    let row = NewId::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO submission (reporting_org_id, source_type, as_of_date, status) \
         VALUES ($1, 'table', $2::date, 'committed') RETURNING id",
        [org_id.into(), today.into()],
    ))
    .one(db)
    .await?
    .ok_or_else(|| sea_orm::DbErr::Custom("INSERT submission не повернув id".into()))?;
    Ok(row.id)
}

// ---------------------------------------------------------------------------
// WhatsApp routing (destinations + subscriptions)
// ---------------------------------------------------------------------------

async fn wa_destinations_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = require_admin_actor(&user)?;
    let rows = repo::whatsapp_routing::list_destinations(&state.db, actor.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(rows))
}

#[derive(Deserialize)]
struct AddDestBody {
    kind: String,
    phone_masked: String,
}

async fn wa_add_destination_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AddDestBody>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let actor = require_admin_actor(&user)?;
    if body.phone_masked.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let id = repo::whatsapp_routing::add_destination(
        &state.db, Some(user.user_id), actor.org_id, &body.kind, body.phone_masked.trim(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

async fn wa_delete_destination_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(dest_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let ok = repo::whatsapp_routing::delete_destination(&state.db, dest_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "ok": ok })))
}

async fn wa_toggle_destination_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(dest_id): Path<i32>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let active = body.get("is_active").and_then(|v| v.as_bool()).unwrap_or(true);
    let ok = repo::whatsapp_routing::toggle_destination(&state.db, dest_id, active)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "ok": ok })))
}

async fn wa_notification_types_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let rows = repo::whatsapp_routing::list_notification_types(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(rows))
}

async fn wa_subscriptions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(dest_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let rows = repo::whatsapp_routing::list_subscriptions(&state.db, dest_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(rows))
}

#[derive(Deserialize)]
struct SetSubBody {
    notification_type_code: String,
    is_active: bool,
}

async fn wa_set_subscription_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(dest_id): Path<i32>,
    Json(body): Json<SetSubBody>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::whatsapp_routing::set_subscription(
        &state.db, dest_id, &body.notification_type_code, body.is_active,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Admin: Dictionary management (VOS & Equipment)
// ---------------------------------------------------------------------------

async fn admin_dictionaries_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let overview = repo::dictionaries::dictionaries_overview(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(overview))
}

#[derive(Deserialize)]
struct CreateVosReq {
    code: String,
    title: String,
}

async fn admin_create_vos_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateVosReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let id = repo::dictionaries::create_vos(&state.db, &body.code, &body.title)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

async fn admin_update_vos_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
    Json(body): Json<CreateVosReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::dictionaries::update_vos(&state.db, id, &body.code, &body.title)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_delete_vos_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::dictionaries::delete_vos(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct CreateEquipmentReq {
    name: String,
    category: Option<String>,
}

async fn admin_create_equipment_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateEquipmentReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let id = repo::dictionaries::create_equipment(&state.db, &body.name, body.category.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

async fn admin_update_equipment_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
    Json(body): Json<CreateEquipmentReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::dictionaries::update_equipment(&state.db, id, &body.name, body.category.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_delete_equipment_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::dictionaries::delete_equipment(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Admin: Generic dictionary CRUD (training_kind, training_direction, etc.)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct GenericDictReq {
    name: String,
    code: Option<String>,
    requires_note: Option<bool>,
}

async fn admin_create_dict_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(dict_type): Path<String>,
    Json(body): Json<GenericDictReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let kind = repo::dictionaries::DictKind::parse(&dict_type)
        .ok_or(StatusCode::NOT_FOUND)?;
    let id = repo::dictionaries::create_dict(
        &state.db,
        kind,
        &body.name,
        body.code.as_deref(),
        body.requires_note,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

async fn admin_update_dict_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((dict_type, id)): Path<(String, i32)>,
    Json(body): Json<GenericDictReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let kind = repo::dictionaries::DictKind::parse(&dict_type)
        .ok_or(StatusCode::NOT_FOUND)?;
    repo::dictionaries::update_dict(
        &state.db,
        kind,
        id,
        &body.name,
        body.code.as_deref(),
        body.requires_note,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_delete_dict_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((dict_type, id)): Path<(String, i32)>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let kind = repo::dictionaries::DictKind::parse(&dict_type)
        .ok_or(StatusCode::NOT_FOUND)?;
    repo::dictionaries::delete_dict(&state.db, kind, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Admin: Training Site CRUD (місця підготовки)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct TrainingSiteReq {
    org_id: i32,
    locality: String,
}

async fn admin_create_training_site_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TrainingSiteReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let id = repo::dictionaries::create_training_site(&state.db, body.org_id, &body.locality)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

async fn admin_update_training_site_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
    Json(body): Json<TrainingSiteReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::dictionaries::update_training_site(&state.db, id, body.org_id, &body.locality)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_delete_training_site_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::dictionaries::delete_training_site(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Admin: Org hierarchy + CRUD
// ---------------------------------------------------------------------------

async fn admin_org_tree_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let tree = repo::orgs::admin_hierarchy(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(tree))
}

#[derive(Deserialize)]
struct CreateOrgReq {
    short_name: String,
    kind: String,
    echelon: Option<String>,
}

async fn admin_create_org_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateOrgReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let id = repo::orgs::create_org(&state.db, &body.short_name, &body.kind, body.echelon.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

async fn admin_update_org_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
    Json(body): Json<CreateOrgReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::orgs::update_org(&state.db, id, &body.short_name, &body.kind, body.echelon.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_delete_org_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::orgs::delete_org(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_org_number_get_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let num = repo::orgs::get_org_number(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(num))
}

#[derive(Deserialize)]
struct UpdateOrgNumberReq {
    number_kind: Option<String>,
    number: Option<String>,
}

async fn admin_org_number_put_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
    Json(body): Json<UpdateOrgNumberReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::orgs::update_org_number(
        &state.db,
        id,
        body.number_kind.as_deref(),
        body.number.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_org_subordination_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let (parents, children) = repo::orgs::org_subordination_links(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "parents": parents, "children": children })))
}

#[derive(Deserialize)]
struct CreateSubordinationReq {
    child_org_id: i32,
    parent_org_id: i32,
    axis: String,
    valid_from: String,
}

async fn admin_create_subordination_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateSubordinationReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let id = repo::orgs::create_subordination(
        &state.db,
        body.child_org_id,
        body.parent_org_id,
        &body.axis,
        &body.valid_from,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

#[derive(Deserialize)]
struct CloseSubordinationReq {
    valid_to: String,
}

async fn admin_close_subordination_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(sub_id): Path<i32>,
    Json(body): Json<CloseSubordinationReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::orgs::close_subordination(&state.db, sub_id, &body.valid_to)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_delete_subordination_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(sub_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::orgs::delete_subordination(&state.db, sub_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Import template (xlsx with enumerations from DB)
// ---------------------------------------------------------------------------

async fn import_template_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _user = require_auth(&state.db, &headers).await?;
    let enums = repo::dictionaries::export_enums_for_template(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let bytes = app::backend::documents::import_template::generate(enums)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>((
        [
            (axum::http::header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
            (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"import-template.xlsx\""),
        ],
        bytes,
    ))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn api_router() -> Router<AppState> {
    Router::new()
        // Auth
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/logout", post(logout_handler))
        .route("/api/auth/me", get(me_handler))
        .route("/api/auth/switch-actor", post(switch_actor_handler))
        .route("/api/auth/change-password", post(change_password_handler))
        .route("/api/auth/account", get(account_handler))
        .route("/api/auth/request-account", post(request_account_handler))
        // Dashboard
        .route("/api/dashboard/stats", get(dashboard_stats_handler))
        // Orgs
        .route("/api/orgs/search", get(org_search_handler))
        .route("/api/orgs/:org_id", get(org_detail_handler))
        .route("/api/orgs/:org_id/children", get(org_children_handler))
        .route("/api/orgs/:org_id/groups", get(org_groups_handler))
        .route("/api/orgs/:org_id/submissions", get(org_submissions_handler))
        .route("/api/orgs/:org_id/users", get(org_users_handler))
        // Discrepancies
        .route("/api/discrepancies", get(discrepancies_handler))
        .route("/api/discrepancies/:disc_id", axum::routing::patch(update_discrepancy_handler))
        .route("/api/discrepancies/:disc_id/compare", get(compare_discrepancy_handler))
        // Documents
        .route("/api/documents/d1", post(generate_d1_handler))
        .route("/api/documents/d2", post(generate_d2_handler))
        .route("/api/documents/d3", post(generate_d3_handler))
        .route("/api/documents/d4", post(generate_d4_handler))
        .route("/api/documents/d4/data", post(d4_data_handler))
        .route("/api/documents/d5/fah", post(generate_d5_fah_handler))
        .route("/api/documents/d5/bps", post(generate_d5_bps_handler))
        .route("/api/documents/d5/kvid", post(generate_d5_kvid_handler))
        .route("/api/documents/d5/ivs", post(generate_d5_ivs_handler))
        .route("/api/documents/d5/terminy", post(generate_d5_terminy_handler))
        .route("/api/documents/d6", post(generate_d6_handler))
        // Training
        .route("/api/training/groups", get(all_groups_handler))
        .route("/api/submissions/recent", get(recent_submissions_handler))
        // Data Workspace
        .route("/api/data/groups", get(data_groups_handler))
        .route("/api/data/groups", post(data_create_group_handler))
        .route("/api/data/groups/:group_id", axum::routing::put(data_update_group_handler))
        .route("/api/data/groups/:group_id", axum::routing::delete(data_delete_group_handler))
        .route("/api/data/groups/:group_id/events", get(data_group_events_handler))
        .route("/api/data/groups/:group_id/events", post(data_add_event_handler))
        .route("/api/data/groups/:group_id/events/:event_id", axum::routing::delete(data_delete_event_handler))
        .route("/api/data/training-kinds", get(training_kinds_handler))
        .route("/api/data/attrition-reasons", get(attrition_reasons_handler))
        .route("/api/data/training-sites", get(site_search_handler))
        // Admin: Org hierarchy
        .route("/api/admin/orgs/tree", get(admin_org_tree_handler))
        .route("/api/admin/orgs", post(admin_create_org_handler))
        .route("/api/admin/orgs/:id", axum::routing::put(admin_update_org_handler))
        .route("/api/admin/orgs/:id", axum::routing::delete(admin_delete_org_handler))
        .route("/api/admin/orgs/:id/number", get(admin_org_number_get_handler))
        .route("/api/admin/orgs/:id/number", axum::routing::put(admin_org_number_put_handler))
        .route("/api/admin/orgs/:id/subordination", get(admin_org_subordination_handler))
        .route("/api/admin/subordination", post(admin_create_subordination_handler))
        .route("/api/admin/subordination/:sub_id/close", axum::routing::put(admin_close_subordination_handler))
        .route("/api/admin/subordination/:sub_id", axum::routing::delete(admin_delete_subordination_handler))
        // Admin: Users
        .route("/api/admin/users", get(admin_list_users_handler))
        .route("/api/admin/users", post(admin_create_user_handler))
        .route("/api/admin/users/:user_id/reset-password", post(admin_reset_password_handler))
        .route("/api/admin/users/:user_id/toggle-active", post(admin_toggle_active_handler))
        // Admin: Dictionaries
        .route("/api/admin/dictionaries", get(admin_dictionaries_handler))
        .route("/api/admin/dictionaries/vos", post(admin_create_vos_handler))
        .route("/api/admin/dictionaries/vos/:id", axum::routing::put(admin_update_vos_handler))
        .route("/api/admin/dictionaries/vos/:id", axum::routing::delete(admin_delete_vos_handler))
        .route("/api/admin/dictionaries/equipment", post(admin_create_equipment_handler))
        .route("/api/admin/dictionaries/equipment/:id", axum::routing::put(admin_update_equipment_handler))
        .route("/api/admin/dictionaries/equipment/:id", axum::routing::delete(admin_delete_equipment_handler))
        // Admin: Training Sites
        .route("/api/admin/dictionaries/training_site", post(admin_create_training_site_handler))
        .route("/api/admin/dictionaries/training_site/:id", axum::routing::put(admin_update_training_site_handler))
        .route("/api/admin/dictionaries/training_site/:id", axum::routing::delete(admin_delete_training_site_handler))
        // Admin: Generic dictionary CRUD
        .route("/api/admin/dictionaries/:dict_type", post(admin_create_dict_handler))
        .route("/api/admin/dictionaries/:dict_type/:id", axum::routing::put(admin_update_dict_handler))
        .route("/api/admin/dictionaries/:dict_type/:id", axum::routing::delete(admin_delete_dict_handler))
        // Import
        .route("/api/import/template", get(import_template_handler))
        .route("/api/import/upload", post(import_upload_handler))
        // Notifications
        .route("/api/notifications", get(notifications_list_handler))
        .route("/api/notifications/read-all", post(notification_mark_all_read_handler))
        .route("/api/notifications/:id/read", post(notification_mark_read_handler))
        // Admin: WhatsApp
        .route("/api/admin/whatsapp/pair", post(whatsapp_pair_handler))
        .route("/api/admin/whatsapp/logout", post(whatsapp_logout_handler))
        .route("/api/admin/whatsapp/test", post(whatsapp_test_handler))
        // WhatsApp routing
        .route("/api/whatsapp/destinations", get(wa_destinations_handler))
        .route("/api/whatsapp/destinations", post(wa_add_destination_handler))
        .route("/api/whatsapp/destinations/:dest_id", axum::routing::delete(wa_delete_destination_handler))
        .route("/api/whatsapp/destinations/:dest_id/toggle", post(wa_toggle_destination_handler))
        .route("/api/whatsapp/destinations/:dest_id/subscriptions", get(wa_subscriptions_handler))
        .route("/api/whatsapp/destinations/:dest_id/subscriptions", post(wa_set_subscription_handler))
        .route("/api/whatsapp/notification-types", get(wa_notification_types_handler))
}
