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
use contracts::NotifyTemplate;

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

    let is_admin = actor.map_or(false, policy::is_admin);
    let can_see_org_names = is_admin || session.can_see_org_names;

    Ok(AuthUser {
        user_id: session.user_id,
        actor,
        display_name: session.display_name,
        callsign: session.callsign,
        avatar_url: session.avatar_path.map(|p| format!("/api/avatars/{}", p)),
        can_see_org_names,
    })
}

async fn org_number_labels(db: &DatabaseConnection, org_ids: &[i32]) -> std::collections::HashMap<i32, String> {
    use sea_orm::FromQueryResult;
    #[derive(FromQueryResult)]
    struct Row { id: i32, label: String }

    if org_ids.is_empty() {
        return Default::default();
    }
    let placeholders: Vec<String> = org_ids.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect();
    let sql = format!(
        "SELECT id, masked_label AS label FROM org WHERE id IN ({})",
        placeholders.join(", ")
    );
    let params: Vec<sea_orm::Value> = org_ids.iter().map(|&id| id.into()).collect();
    Row::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres, &sql, params,
    ))
    .all(db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| (r.id, r.label))
    .collect()
}

async fn org_number_label_one(db: &DatabaseConnection, org_id: i32) -> String {
    let map = org_number_labels(db, &[org_id]).await;
    map.into_values().next().unwrap_or_else(|| format!("#{org_id}"))
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

    let mut roles = repo::auth::user_roles(db, user.id).await.unwrap_or_default();
    let has_admin = roles.iter().any(|r| r.role == "admin");
    let show_names = has_admin || user.can_see_org_names;
    if !show_names {
        let ids: Vec<i32> = roles.iter().map(|r| r.org_id).collect();
        let labels = org_number_labels(db, &ids).await;
        for r in &mut roles {
            if let Some(l) = labels.get(&r.org_id) {
                r.org_label = l.clone();
            }
        }
    }

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
    callsign: Option<String>,
    avatar_url: Option<String>,
    can_see_org_names: bool,
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
        let label = if user.can_see_org_names {
            LabelRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
                state.db.get_database_backend(),
                "SELECT short_name AS label FROM org WHERE id = $1",
                [actor.org_id.into()],
            ))
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .map(|r| r.label)
            .unwrap_or_else(|| format!("Org #{}", actor.org_id))
        } else {
            org_number_label_one(&state.db, actor.org_id).await
        };
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
        display_name: user.display_name.clone(),
        callsign: user.callsign.clone(),
        avatar_url: user.avatar_url.clone(),
        can_see_org_names: user.can_see_org_names,
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
        first_name: Option<String>,
        last_name: Option<String>,
        rank: Option<String>,
        phone: Option<String>,
        callsign: Option<String>,
        delta_nick: Option<String>,
        avatar_path: Option<String>,
        created_at: String,
    }
    let row = AccountRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT login, display_name, first_name, last_name, rank, phone, callsign, \
                delta_nick, avatar_path, to_char(created_at, 'YYYY-MM-DD') AS created_at \
         FROM user_account WHERE id = $1",
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

    let avatar_url = row.avatar_path.as_ref().map(|p| format!("/api/avatars/{}", p));

    Ok::<_, StatusCode>(Json(AccountInfo {
        login: row.login,
        full_name: row.display_name,
        first_name: row.first_name,
        last_name: row.last_name,
        rank: row.rank,
        phone: row.phone,
        callsign: row.callsign,
        delta_nick: row.delta_nick,
        avatar_url,
        roles,
        created_at: row.created_at,
    }))
}

// ---------------------------------------------------------------------------
// Update profile
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct UpdateProfileReq {
    first_name: Option<String>,
    last_name: Option<String>,
    rank: Option<String>,
    phone: Option<String>,
    callsign: Option<String>,
    delta_nick: Option<String>,
}

async fn update_profile_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UpdateProfileReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;

    state.db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE user_account SET first_name = $1, last_name = $2, rank = $3, \
         phone = $4, callsign = $5, delta_nick = $6, updated_at = now() WHERE id = $7",
        [
            body.first_name.into(), body.last_name.into(), body.rank.into(),
            body.phone.into(), body.callsign.into(), body.delta_nick.into(),
            user.user_id.into(),
        ],
    ))
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Directory — list visible users
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct DirectoryQuery {
    include_inactive: Option<bool>,
}

async fn directory_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<DirectoryQuery>,
) -> impl IntoResponse {
    use sea_orm::FromQueryResult;

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let is_admin = policy::is_admin(actor);
    let show_inactive = is_admin && params.include_inactive.unwrap_or(false);

    #[derive(FromQueryResult, Serialize)]
    struct DirectoryUser {
        user_id: i32,
        login: String,
        display_name: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        rank: Option<String>,
        phone: Option<String>,
        callsign: Option<String>,
        delta_nick: Option<String>,
        avatar_url: Option<String>,
        is_active: bool,
        role: String,
        org_label: String,
    }

    #[derive(FromQueryResult)]
    struct RawDirUser {
        user_id: i32,
        login: String,
        display_name: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        rank: Option<String>,
        phone: Option<String>,
        callsign: Option<String>,
        delta_nick: Option<String>,
        avatar_path: Option<String>,
        is_active: bool,
        role: String,
        org_label: String,
    }

    let (visibility_filter, active_filter) = match (&visible, show_inactive) {
        (None, true) => (String::new(), String::new()),
        (None, false) => (String::new(), " AND ua.is_active = true".to_string()),
        (Some(ids), true) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            (format!(" AND EXISTS (SELECT 1 FROM user_role ur2 WHERE ur2.user_id = ua.id AND ur2.org_id IN ({list}))"), String::new())
        }
        (Some(ids), false) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            (format!(" AND EXISTS (SELECT 1 FROM user_role ur2 WHERE ur2.user_id = ua.id AND ur2.org_id IN ({list}))"), " AND ua.is_active = true".to_string())
        }
    };

    let org_label_col = if user.can_see_org_names { "o.short_name" } else { "o.masked_label" };
    let sql = format!(
        "SELECT DISTINCT ON (ua.id) ua.id AS user_id, ua.login, ua.display_name, \
                ua.first_name, ua.last_name, ua.rank, ua.phone, ua.callsign, \
                ua.delta_nick, ua.avatar_path, ua.is_active, \
                ur.role, {org_label_col} AS org_label \
         FROM user_account ua \
         JOIN user_role ur ON ur.user_id = ua.id \
         JOIN org o ON o.id = ur.org_id \
         WHERE true{visibility_filter}{active_filter} \
         ORDER BY ua.id, \
                CASE ur.role WHEN 'admin' THEN 0 WHEN 'org_editor' THEN 1 ELSE 2 END, \
                ur.id"
    );

    let raw_rows = RawDirUser::find_by_statement(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        sql,
    ))
    .all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows: Vec<DirectoryUser> = raw_rows.into_iter().map(|r| {
        DirectoryUser {
            user_id: r.user_id,
            login: r.login,
            display_name: r.display_name,
            first_name: r.first_name,
            last_name: r.last_name,
            rank: r.rank,
            phone: r.phone,
            callsign: r.callsign,
            delta_nick: r.delta_nick,
            avatar_url: r.avatar_path.map(|p| format!("/api/avatars/{}", p)),
            is_active: r.is_active,
            role: r.role,
            org_label: r.org_label,
        }
    }).collect();

    Ok::<_, StatusCode>(Json(rows))
}

async fn can_manage_user(db: &DatabaseConnection, admin_actor: Actor, target_user_id: i32) -> Result<bool, StatusCode> {
    use sea_orm::FromQueryResult;

    let visible = policy::visible_org_ids(db, admin_actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    #[derive(FromQueryResult)]
    struct RoleRow { org_id: i32, role: String }
    let target_roles = RoleRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT org_id, role FROM user_role WHERE user_id = $1",
        [target_user_id.into()],
    ))
    .all(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if target_roles.is_empty() {
        return Ok(false);
    }
    if target_roles.iter().any(|r| r.role == "admin") {
        return Ok(false);
    }
    if let Some(ids) = &visible {
        if !target_roles.iter().all(|r| ids.contains(&r.org_id)) {
            return Ok(false);
        }
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// Avatar upload & serve
// ---------------------------------------------------------------------------

async fn avatar_upload_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = require_auth(&state.db, &headers).await?;

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if field.name() == Some("avatar") {
            content_type = field.content_type().map(|s| s.to_string());
            let bytes = field.bytes().await.map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
            if bytes.len() > 2 * 1024 * 1024 {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            file_bytes = Some(bytes.to_vec());
        }
    }

    let bytes = file_bytes.ok_or(StatusCode::BAD_REQUEST)?;
    let ct = content_type.unwrap_or_default();
    let ext = match ct.as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        _ => "jpg",
    };

    let avatar_dir = std::path::Path::new("data/avatars");
    std::fs::create_dir_all(avatar_dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("user_{}.{}", user.user_id, ext);
    let filepath = avatar_dir.join(&filename);
    std::fs::write(&filepath, &bytes).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE user_account SET avatar_path = $1, updated_at = now() WHERE id = $2",
        [filename.clone().into(), user.user_id.into()],
    ))
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(Json(serde_json::json!({ "avatar_url": format!("/api/avatars/{}?v={}", filename, ts) })))
}

async fn avatar_delete_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;

    use sea_orm::FromQueryResult;
    #[derive(FromQueryResult)]
    struct AvatarRow { avatar_path: Option<String> }

    let old = AvatarRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT avatar_path FROM user_account WHERE id = $1",
        [user.user_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(row) = old {
        if let Some(path) = row.avatar_path {
            let _ = std::fs::remove_file(format!("data/avatars/{}", path));
        }
    }

    state.db.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "UPDATE user_account SET avatar_path = NULL, updated_at = now() WHERE id = $1",
        [user.user_id.into()],
    ))
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn avatar_serve_handler(
    Path(filename): Path<String>,
) -> impl IntoResponse {
    let safe_name = filename.replace(['/', '\\', '.', '.'], "");
    let filepath = format!("data/avatars/{}", filename);
    let path = std::path::Path::new(&filepath);

    if !path.exists() || safe_name.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let bytes = std::fs::read(path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let ct = if filename.ends_with(".png") {
        "image/png"
    } else if filename.ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg"
    };

    Ok((
        [(axum::http::header::CONTENT_TYPE, ct), (axum::http::header::CACHE_CONTROL, "public, max-age=60")],
        bytes,
    ))
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

    let org_label_col = if user.can_see_org_names { "o.short_name" } else { "o.masked_label" };

    #[derive(FromQueryResult)]
    struct SubRow {
        id: i64,
        org_label: String,
        source_type: String,
        updated_at: String,
    }
    let subs_sql = match &visible {
        None => format!(
                "SELECT s.id::bigint AS id, COALESCE({org_label_col}, '') AS org_label, \
                 s.source_type, to_char(s.updated_at, 'DD.MM.YYYY HH24:MI') AS updated_at \
                 FROM submission s JOIN org o ON o.id = s.reporting_org_id \
                 ORDER BY s.updated_at DESC LIMIT 5"),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(
                "SELECT s.id::bigint AS id, COALESCE({org_label_col}, '') AS org_label, \
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
        None => format!(
                "SELECT d.id::bigint AS id, COALESCE({org_label_col}, '') AS org_label, \
                 d.metric AS metric_label, d.status, to_char(d.created_at, 'DD.MM.YYYY HH24:MI') AS created_at \
                 FROM discrepancy d JOIN org o ON o.id = d.org_id \
                 ORDER BY d.created_at DESC LIMIT 5"),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(
                "SELECT d.id::bigint AS id, COALESCE({org_label_col}, '') AS org_label, \
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
        recent_submissions: recent_subs.into_iter().map(|s| {
            RecentSubDto { id: s.id, org_label: s.org_label, source_type: s.source_type, updated_at: s.updated_at }
        }).collect(),
        recent_discrepancies: recent_discs.into_iter().map(|d| {
            RecentDiscDto { id: d.id, org_label: d.org_label, metric_label: d.metric_label, status: d.status, created_at: d.created_at }
        }).collect(),
    }))
}

// ---------------------------------------------------------------------------
// Dashboard: pipeline (training funnel)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct PipelineDto {
    planned: i64,
    arrived: i64,
    in_training: i64,
    completed: i64,
    attrition: i64,
}

async fn dashboard_pipeline_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filter = match &visible {
        None => String::new(),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(" AND tg.sender_org_id IN ({list})")
        }
    };

    #[derive(FromQueryResult)]
    struct Row {
        planned: i64,
        arrived: i64,
        started: i64,
        completed: i64,
        attrition: i64,
    }

    let sql = format!(
        "SELECT \
            COALESCE(SUM(CASE WHEN ge.event_type = 'planned' THEN ge.count END), 0) AS planned, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'arrived' THEN ge.count END), 0) AS arrived, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'started' THEN ge.count END), 0) AS started, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'completed' THEN ge.count END), 0) AS completed, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'attrition' THEN ge.count END), 0) AS attrition \
         FROM group_event ge \
         JOIN training_group tg ON tg.id = ge.group_id \
         WHERE true{filter}"
    );

    let row = Row::find_by_statement(Statement::from_string(
        state.db.get_database_backend(), sql,
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .unwrap_or(Row { planned: 0, arrived: 0, started: 0, completed: 0, attrition: 0 });

    Ok::<_, StatusCode>(Json(PipelineDto {
        planned: row.planned,
        arrived: row.arrived,
        in_training: row.started,
        completed: row.completed,
        attrition: row.attrition,
    }))
}

// ---------------------------------------------------------------------------
// Dashboard: by-org breakdown
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct OrgBreakdownDto {
    org_name: String,
    group_count: i64,
    planned: i64,
    completed: i64,
    attrition: i64,
}

async fn dashboard_by_org_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filter = match &visible {
        None => String::new(),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(" WHERE tg.sender_org_id IN ({list})")
        }
    };

    #[derive(FromQueryResult)]
    struct Row {
        org_name: String,
        group_count: i64,
        planned: i64,
        completed: i64,
        attrition: i64,
    }

    let org_label_col = if user.can_see_org_names { "o.short_name" } else { "o.masked_label" };
    let sql = format!(
        "SELECT \
            COALESCE({org_label_col}, '') AS org_name, \
            COUNT(DISTINCT tg.id) AS group_count, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'planned' THEN ge.count END), 0) AS planned, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'completed' THEN ge.count END), 0) AS completed, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'attrition' THEN ge.count END), 0) AS attrition \
         FROM training_group tg \
         JOIN org o ON o.id = tg.sender_org_id \
         LEFT JOIN group_event ge ON ge.group_id = tg.id \
         {filter} \
         GROUP BY o.id, {org_label_col} \
         ORDER BY planned DESC \
         LIMIT 20"
    );

    let rows = Row::find_by_statement(Statement::from_string(
        state.db.get_database_backend(), sql,
    ))
    .all(&state.db)
    .await
    .unwrap_or_default();

    let dtos: Vec<OrgBreakdownDto> = rows.into_iter().map(|r| {
        OrgBreakdownDto { org_name: r.org_name, group_count: r.group_count, planned: r.planned, completed: r.completed, attrition: r.attrition }
    }).collect();

    Ok::<_, StatusCode>(Json(dtos))
}

// ---------------------------------------------------------------------------
// Dashboard: timeline (events per week)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct TimelineDto {
    week: String,
    planned: i64,
    started: i64,
    completed: i64,
    attrition: i64,
}

async fn dashboard_timeline_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filter = match &visible {
        None => String::new(),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(" AND tg.sender_org_id IN ({list})")
        }
    };

    #[derive(FromQueryResult)]
    struct Row {
        week: String,
        planned: i64,
        started: i64,
        completed: i64,
        attrition: i64,
    }

    let sql = format!(
        "SELECT \
            to_char(date_trunc('week', ge.occurred_on), 'YYYY-MM-DD') AS week, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'planned' THEN ge.count END), 0) AS planned, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'started' THEN ge.count END), 0) AS started, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'completed' THEN ge.count END), 0) AS completed, \
            COALESCE(SUM(CASE WHEN ge.event_type = 'attrition' THEN ge.count END), 0) AS attrition \
         FROM group_event ge \
         JOIN training_group tg ON tg.id = ge.group_id \
         WHERE ge.occurred_on >= CURRENT_DATE - INTERVAL '6 months'{filter} \
         GROUP BY date_trunc('week', ge.occurred_on) \
         ORDER BY week"
    );

    let rows = Row::find_by_statement(Statement::from_string(
        state.db.get_database_backend(), sql,
    ))
    .all(&state.db)
    .await
    .unwrap_or_default();

    let dtos: Vec<TimelineDto> = rows.into_iter().map(|r| TimelineDto {
        week: r.week, planned: r.planned, started: r.started,
        completed: r.completed, attrition: r.attrition,
    }).collect();

    Ok::<_, StatusCode>(Json(dtos))
}

// ---------------------------------------------------------------------------
// Dashboard: discrepancies chart
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct DiscChartDto {
    by_status: Vec<DiscStatusDto>,
    by_org: Vec<DiscOrgDto>,
}

#[derive(Serialize)]
struct DiscStatusDto {
    status: String,
    count: i64,
}

#[derive(Serialize)]
struct DiscOrgDto {
    org_name: String,
    open: i64,
    resolved: i64,
    accepted: i64,
}

async fn dashboard_disc_chart_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filter = match &visible {
        None => String::new(),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(" WHERE d.org_id IN ({list})")
        }
    };

    #[derive(FromQueryResult)]
    struct StatusRow {
        status: String,
        count: i64,
    }
    let status_sql = format!(
        "SELECT d.status, count(*) AS count FROM discrepancy d{filter} \
         GROUP BY d.status ORDER BY count DESC"
    );
    let by_status = StatusRow::find_by_statement(Statement::from_string(
        state.db.get_database_backend(), status_sql,
    ))
    .all(&state.db)
    .await
    .unwrap_or_default();

    let filter2 = match &visible {
        None => String::new(),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(" AND d.org_id IN ({list})")
        }
    };

    let org_label_col = if user.can_see_org_names { "o.short_name" } else { "o.masked_label" };
    #[derive(FromQueryResult)]
    struct OrgRow {
        org_name: String,
        open: i64,
        resolved: i64,
        accepted: i64,
    }
    let org_sql = format!(
        "SELECT COALESCE({org_label_col}, '') AS org_name, \
            COALESCE(SUM(CASE WHEN d.status = 'open' THEN 1 END), 0) AS open, \
            COALESCE(SUM(CASE WHEN d.status = 'resolved' THEN 1 END), 0) AS resolved, \
            COALESCE(SUM(CASE WHEN d.status = 'accepted' THEN 1 END), 0) AS accepted \
         FROM discrepancy d \
         JOIN org o ON o.id = d.org_id \
         WHERE true{filter2} \
         GROUP BY o.id, {org_label_col} \
         ORDER BY open DESC \
         LIMIT 15"
    );
    let by_org = OrgRow::find_by_statement(Statement::from_string(
        state.db.get_database_backend(), org_sql,
    ))
    .all(&state.db)
    .await
    .unwrap_or_default();

    Ok::<_, StatusCode>(Json(DiscChartDto {
        by_status: by_status.into_iter().map(|r| DiscStatusDto { status: r.status, count: r.count }).collect(),
        by_org: by_org.into_iter().map(|r| {
            DiscOrgDto { org_name: r.org_name, open: r.open, resolved: r.resolved, accepted: r.accepted }
        }).collect(),
    }))
}

// ---------------------------------------------------------------------------
// Dashboard: staffing overview
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct StaffingDto {
    org_name: String,
    category: String,
    authorized: i64,
    assigned: i64,
}

async fn dashboard_staffing_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    let user = require_auth(&state.db, &headers).await?;
    let actor = actor_or_err(&user)?;

    let visible = policy::visible_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filter = match &visible {
        None => String::new(),
        Some(ids) => {
            let list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            format!(" AND ss.org_id IN ({list})")
        }
    };

    let org_label_col = if user.can_see_org_names { "o.short_name" } else { "o.masked_label" };

    #[derive(FromQueryResult)]
    struct Row {
        org_name: String,
        category: String,
        authorized: i64,
        assigned: i64,
    }

    let sql = format!(
        "WITH latest AS ( \
            SELECT DISTINCT ON (org_id, category) id, org_id, category \
            FROM staffing_snapshot \
            ORDER BY org_id, category, as_of DESC \
         ) \
         SELECT COALESCE({org_label_col}, '') AS org_name, \
                ss.category, \
                COALESCE((SELECT sm.value::bigint FROM staffing_metric sm WHERE sm.snapshot_id = ss.id AND sm.metric = 'by_tos'), 0) AS authorized, \
                COALESCE((SELECT sm.value::bigint FROM staffing_metric sm WHERE sm.snapshot_id = ss.id AND sm.metric = 'by_list'), 0) AS assigned \
         FROM latest ss \
         JOIN org o ON o.id = ss.org_id \
         WHERE true{filter} \
         ORDER BY org_name, category"
    );

    let rows = Row::find_by_statement(Statement::from_string(
        state.db.get_database_backend(), sql,
    ))
    .all(&state.db)
    .await
    .unwrap_or_default();

    let dtos: Vec<StaffingDto> = rows.into_iter().map(|r| {
        StaffingDto { org_name: r.org_name, category: r.category, authorized: r.authorized, assigned: r.assigned }
    }).collect();

    Ok::<_, StatusCode>(Json(dtos))
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

    let mut groups = repo::auth::list_training_groups(&state.db, Some(&subtree))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !user.can_see_org_names {
        let ids: Vec<i32> = groups.iter().map(|g| g.org_id).collect();
        let labels = org_number_labels(&state.db, &ids).await;
        for g in &mut groups {
            if let Some(lbl) = labels.get(&g.org_id) {
                g.org_label = lbl.clone();
            }
        }
    }

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

    let mut filtered: Vec<_> = match visible {
        None => rows,
        Some(ids) => rows.into_iter().filter(|r| ids.contains(&r.org_id)).collect(),
    };

    if !user.can_see_org_names {
        let ids: Vec<i32> = filtered.iter().map(|r| r.org_id).collect();
        let labels = org_number_labels(&state.db, &ids).await;
        for r in &mut filtered {
            if let Some(lbl) = labels.get(&r.org_id) {
                r.org_label = lbl.clone();
            }
        }
    }

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
    Path(target_id): Path<i32>,
    Json(body): Json<ResetPasswordReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let admin = require_admin_actor(&user)?;

    if !can_manage_user(&state.db, admin, target_id).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let password_hash = hash_password(&body.temp_password).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    repo::auth::reset_password(&state.db, target_id, &password_hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_toggle_active_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let admin = require_admin_actor(&user)?;

    if !can_manage_user(&state.db, admin, target_id).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    use sea_orm::FromQueryResult;
    #[derive(FromQueryResult)]
    struct ActiveRow { is_active: bool }
    let row = ActiveRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT is_active FROM user_account WHERE id = $1",
        [target_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    repo::auth::toggle_active(&state.db, target_id, !row.is_active)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_toggle_org_names_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let admin = require_admin_actor(&user)?;

    if !can_manage_user(&state.db, admin, target_id).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    use sea_orm::FromQueryResult;
    #[derive(FromQueryResult)]
    struct FlagRow { can_see_org_names: bool }
    let row = FlagRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT can_see_org_names FROM user_account WHERE id = $1",
        [target_id.into()],
    ))
    .one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    repo::auth::set_can_see_org_names(&state.db, target_id, !row.can_see_org_names)
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
    let mut results = repo::orgs::search_orgs(&state.db, &query)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !user.can_see_org_names {
        for r in &mut results {
            r.label = std::mem::take(&mut r.masked_label);
        }
    }

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
const SUBJECT_GROUPS_LIST: &str = "vyshkil.notifier.whatsapp.groups.list";
const SUBJECT_CONTACTS_UPSERT: &str = "vyshkil.notifier.whatsapp.contacts.upsert";
const SUBJECT_CONTACTS_REMOVE: &str = "vyshkil.notifier.whatsapp.contacts.remove";
const SUBJECT_CONTACTS_SYNC: &str = "vyshkil.notifier.whatsapp.contacts.sync";

fn nats_client(state: &AppState) -> Result<bus::async_nats::Client, StatusCode> {
    state
        .nats
        .lock()
        .expect("shared NATS mutex отруєний")
        .clone()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

pub async fn sync_wa_contacts(db: &sea_orm::DatabaseConnection, nats: &bus::SharedNatsClient) {
    let client = match nats.lock().expect("shared NATS mutex отруєний").clone() {
        Some(c) => c,
        None => return,
    };
    let rows = match repo::whatsapp_routing::all_destinations_for_sync(db).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("sync_wa_contacts: не вдалось прочитати whatsapp_destination: {e}");
            return;
        }
    };
    let contacts: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "org_id": r.org_id,
                "phone": r.phone,
                "kind": r.kind,
                "active": r.is_active,
            })
        })
        .collect();
    let payload = serde_json::json!({ "contacts": contacts }).to_string();
    match bus::request(&client, SUBJECT_CONTACTS_SYNC, payload.into_bytes()).await {
        Ok(resp) => {
            let body = String::from_utf8_lossy(&resp);
            tracing::info!("sync_wa_contacts: відповідь нотифікатора: {body}");
        }
        Err(e) => {
            tracing::warn!("sync_wa_contacts: NATS request failed: {e}");
        }
    }
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

async fn whatsapp_groups_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    require_admin_actor(&user)?;

    let client = nats_client(&state)?;
    let raw = bus::request(&client, SUBJECT_GROUPS_LIST, b"{}".to_vec())
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

    let mut groups = repo::auth::list_training_groups(&state.db, visible.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !user.can_see_org_names {
        let ids: Vec<i32> = groups.iter().map(|g| g.org_id).collect();
        let labels = org_number_labels(&state.db, &ids).await;
        for g in &mut groups {
            if let Some(lbl) = labels.get(&g.org_id) {
                g.org_label = lbl.clone();
            }
        }
    }

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

    let mut groups = repo::groups::list_groups_extended(&state.db, visible.as_deref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !user.can_see_org_names {
        for g in &mut groups {
            g.org_label = std::mem::take(&mut g.org_masked_label);
            g.organizer_label = std::mem::take(&mut g.organizer_masked_label);
        }
    }

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
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user = require_auth(&state.db, &headers).await
        .map_err(|s| (s, Json(serde_json::json!({ "error": "Не авторизовано" }))))?;
    let _actor = require_admin_actor(&user)
        .map_err(|s| (s, Json(serde_json::json!({ "error": "Недостатньо прав" }))))?;

    if body.count <= 0 {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "К-ть має бути > 0" }))));
    }
    if body.occurred_on.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Дата обов'язкова" }))));
    }

    let txn = state.db.begin().await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Помилка сервера" }))))?;
    policy::set_session_actor(&txn, _actor).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Помилка сервера" }))))?;

    let id = match repo::groups::add_group_event(
        &txn,
        group_id,
        &body.event_type,
        body.count,
        &body.occurred_on,
        body.reason_id,
        body.note.as_deref(),
        Some(user.user_id),
    )
    .await
    {
        Ok(id) => id,
        Err(sea_orm::DbErr::Custom(msg)) => {
            return Err((StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({ "error": msg }))));
        }
        Err(_) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Помилка сервера" }))));
        }
    };

    txn.commit().await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Помилка сервера" }))))?;

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
        let _ = repo::whatsapp_routing::dispatch_wa_notifications(
            &state.db, org_id, "group_event_added", NotifyTemplate::GroupEventAdded,
        ).await;
    }

    let _ = repo::reconciliation::refresh_horizontal(&state.db, group_id).await;
    let _ = repo::reconciliation::refresh_temporal(&state.db, group_id).await;

    Ok(Json(serde_json::json!({ "id": id })))
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
    #[serde(default)]
    site_id: Option<i32>,
    venue_type: Option<String>,
    training_venue_id: Option<i32>,
    city_id: Option<i32>,
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
    #[serde(default)]
    force: bool,
}

async fn data_create_group_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateGroupBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user = require_auth(&state.db, &headers).await
        .map_err(|s| (s, Json(serde_json::json!({ "error": "Не авторизовано" }))))?;
    let _actor = require_admin_actor(&user)
        .map_err(|s| (s, Json(serde_json::json!({ "error": "Недостатньо прав" }))))?;

    if body.planned_count <= 0 {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "К-ть за планом має бути > 0" }))));
    }
    if body.arrived_count < 0 || body.in_training_count < 0 {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "К-ть не може бути від'ємною" }))));
    }
    if body.planned_start.is_empty() || body.planned_end.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Дати обов'язкові" }))));
    }

    use repo::groups::CreateGroupResult;

    let result = repo::groups::create_group_with_events(
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
        body.venue_type.as_deref(),
        body.training_venue_id,
        body.city_id,
        None,
        body.force,
    )
    .await
    .map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Помилка створення групи" })))
    })?;

    match result {
        CreateGroupResult::Duplicates(existing) => {
            Ok(Json(serde_json::json!({
                "warning": "Схожі заходи вже існують",
                "existing": existing
            })))
        }
        CreateGroupResult::Created(id) => {
            let sub_id = create_form_submission(&state.db, body.sender_org_id)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Помилка створення подання" }))))?;
            let _ = state.db.execute(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "UPDATE group_event SET submission_id = $1 WHERE group_id = $2 AND submission_id IS NULL",
                [sub_id.into(), id.into()],
            )).await;

            let _ = repo::notifications::insert(
                &state.db, body.sender_org_id, "group_event", "Створено нову групу підготовки",
                None, Some("/data"),
            ).await;
            let _ = repo::whatsapp_routing::dispatch_wa_notifications(
                &state.db, body.sender_org_id, "group_event_added", NotifyTemplate::GroupEventAdded,
            ).await;

            Ok(Json(serde_json::json!({ "id": id })))
        }
    }
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

    let venues = repo::venues::search_venues(&state.db, &q, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(venues))
}

async fn city_search_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<SearchQuery>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = actor_or_err(&user)?;

    let q = params.q.unwrap_or_default();
    let limit = params.limit.unwrap_or(10).min(20) as i64;

    let cities = repo::venues::search_cities(&state.db, &q, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok::<_, StatusCode>(Json(cities))
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

    let no_data_err = |kind_label: &str| {
        (StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({
            "error": format!("Файл не містить розпізнаних даних типу \"{}\". Перевірте тип файлу або структуру таблиці.", kind_label),
            "issues": []
        })))
    };

    let imported: i64 = match kind.as_str() {
        "fah" => {
            let raw = app::backend::import::fah::extract(&bytes).map_err(parse_err)?;
            if raw.is_empty() { return Err(no_data_err("Фах")); }
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
            if raw.is_empty() { return Err(no_data_err("БпС")); }
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
            if raw.bzvp.is_empty() && raw.special.is_empty() && raw.adapt.is_empty() { return Err(no_data_err("Терміни")); }
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
            if raw.is_empty() { return Err(no_data_err("Архів ВЧ")); }
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
            if raw.is_empty() { return Err(no_data_err("КВід")); }
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
            if raw.staffing.is_empty() && raw.internships.is_empty() && raw.courses.is_empty() { return Err(no_data_err("ІВС")); }
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

    if imported > 0 {
        let _ = repo::notifications::insert(
            db, _actor.org_id, "import", &format!("Імпорт \"{kind}\": {imported} записів"),
            None, Some("/data"),
        ).await;
        let _ = repo::whatsapp_routing::dispatch_wa_notifications(
            db, _actor.org_id, "import_completed", NotifyTemplate::ImportCompleted,
        ).await;
    }

    Ok(Json(serde_json::json!({ "imported": imported })))
}

type ImportValidated = Vec<(app::types::submission::GroupFormRow, repo::groups::ValidatedRow)>;
type ApiError = (StatusCode, Json<serde_json::Value>);

fn parse_import_note(note: &str) -> (String, u32) {
    if let Some(rest) = note.strip_prefix("Імпорт: ") {
        if let Some(pos) = rest.rfind(' ') {
            let sheet = rest[..pos].to_string();
            let row = rest[pos + 1..].parse().unwrap_or(0);
            return (sheet, row);
        }
    }
    (String::new(), 0)
}

fn collect_unresolved_issues(rows: &[app::types::submission::GroupFormRow]) -> Vec<serde_json::Value> {
    let mut issues = Vec::new();
    for row in rows {
        let (sheet, row_num) = parse_import_note(&row.note);
        if row.sender_org_id.is_none() && !row.sender_org_label.is_empty() {
            issues.push(serde_json::json!({
                "row": row_num, "sheet": sheet,
                "field": "Частина",
                "value": row.sender_org_label,
                "message": "не знайдено в довіднику організацій"
            }));
        }
        if row.site_id.is_none() && !row.site_label.is_empty() {
            issues.push(serde_json::json!({
                "row": row_num, "sheet": sheet,
                "field": "Місце",
                "value": row.site_label,
                "message": "не знайдено в довіднику організацій/полігонів"
            }));
        }
        if row.vos_id.is_none() && row.position_id.is_none() && row.course_id.is_none()
            && !row.vos_position_course_label.is_empty()
        {
            issues.push(serde_json::json!({
                "row": row_num, "sheet": sheet,
                "field": "ВОС/Посада",
                "value": row.vos_position_course_label,
                "message": "не знайдено в довідниках ВОС, посад або курсів"
            }));
        }
    }
    issues
}

fn validate_import_rows(
    rows: &[app::types::submission::GroupFormRow],
    today: chrono::NaiveDate,
) -> Result<ImportValidated, ApiError> {
    let mut issues = collect_unresolved_issues(rows);

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let (sheet, row_num) = parse_import_note(&row.note);
        match repo::groups::validate_row(row, today) {
            Ok(v) => out.push((row.clone(), v)),
            Err((field, msg)) => {
                issues.push(serde_json::json!({
                    "row": row_num, "sheet": sheet,
                    "field": field, "value": "",
                    "message": msg
                }));
            }
        }
    }
    if !issues.is_empty() {
        let total = issues.len();
        issues.truncate(50);
        return Err((StatusCode::UNPROCESSABLE_ENTITY, Json(serde_json::json!({
            "error": format!("Знайдено {} помилок у файлі", total),
            "issues": issues
        }))));
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

async fn create_form_submission(
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
         VALUES ($1, 'form', $2::date, 'committed') RETURNING id",
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
    group_id: Option<String>,
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
        body.group_id.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Sync to notifier's org_contact via NATS (best-effort, doesn't block response)
    if let Ok(client) = nats_client(&state) {
        let phone = if body.kind == "group" {
            body.group_id.as_deref()
                .filter(|g| !g.trim().is_empty())
                .unwrap_or(body.phone_masked.trim())
                .to_string()
        } else {
            body.phone_masked.trim().to_string()
        };
        let payload = serde_json::json!({
            "org_id": actor.org_id,
            "phone": phone,
            "kind": body.kind,
            "active": true,
        }).to_string();
        let _ = bus::request(&client, SUBJECT_CONTACTS_UPSERT, payload.into_bytes()).await;
    }

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

    if ok {
        let db = state.db.clone();
        let nats = state.nats.clone();
        tokio::spawn(async move { sync_wa_contacts(&db, &nats).await });
    }

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

    if ok {
        let db = state.db.clone();
        let nats = state.nats.clone();
        tokio::spawn(async move { sync_wa_contacts(&db, &nats).await });
    }

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
// Admin: City CRUD
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CityReq { name: String }

async fn admin_cities_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let rows = repo::venues::list_cities(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(rows))
}

async fn admin_create_city_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CityReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let id = repo::venues::create_city(&state.db, &body.name).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

async fn admin_update_city_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
    Json(body): Json<CityReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::venues::update_city(&state.db, id, &body.name).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_delete_city_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::venues::delete_city(&state.db, id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Admin: Training Venue CRUD (навчальні центри, ВВНЗ)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct VenueReq {
    kind: String,
    name: String,
    short_name: Option<String>,
    military_number: Option<String>,
    city_id: i32,
    org_id: Option<i32>,
}

async fn admin_venues_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let rows = repo::venues::list_venues(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(rows))
}

async fn admin_create_venue_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<VenueReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    let id = repo::venues::create_venue(
        &state.db, &body.kind, &body.name, body.short_name.as_deref(),
        body.military_number.as_deref(), body.city_id, body.org_id,
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(Json(serde_json::json!({ "id": id })))
}

async fn admin_update_venue_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
    Json(body): Json<VenueReq>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::venues::update_venue(
        &state.db, id, &body.kind, &body.name, body.short_name.as_deref(),
        body.military_number.as_deref(), body.city_id, body.org_id,
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

async fn admin_delete_venue_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let user = require_auth(&state.db, &headers).await?;
    let _actor = require_admin_actor(&user)?;
    repo::venues::delete_venue(&state.db, id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok::<_, StatusCode>(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Admin: Training Site CRUD (legacy — deprecated, kept for backward compat)
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
    let actor = require_admin_actor(&user)?;
    let tree = repo::orgs::admin_hierarchy(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let ids = policy::manageable_org_ids(&state.db, actor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let filtered: Vec<_> = tree
        .into_iter()
        .filter(|n| ids.contains(&n.id))
        .map(|mut n| {
            if let Some(pid) = n.parent_id {
                if !ids.contains(&pid) {
                    n.parent_id = None;
                }
            }
            n
        })
        .collect();
    Ok::<_, StatusCode>(Json(filtered))
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
    let actor = require_admin_actor(&user)?;
    if !policy::can_manage_org(&state.db, actor, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }
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
    let actor = require_admin_actor(&user)?;
    if !policy::can_manage_org(&state.db, actor, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }
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
    let actor = require_admin_actor(&user)?;
    if !policy::can_manage_org(&state.db, actor, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }
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
    let actor = require_admin_actor(&user)?;
    if !policy::can_manage_org(&state.db, actor, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }
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
    let actor = require_admin_actor(&user)?;
    if !policy::can_manage_org(&state.db, actor, body.parent_org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::FORBIDDEN);
    }
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
        .route("/api/auth/account", get(account_handler).put(update_profile_handler))
        .route("/api/auth/avatar", post(avatar_upload_handler).delete(avatar_delete_handler))
        .route("/api/avatars/:filename", get(avatar_serve_handler))
        .route("/api/auth/request-account", post(request_account_handler))
        .route("/api/directory", get(directory_handler))
        // Dashboard
        .route("/api/dashboard/stats", get(dashboard_stats_handler))
        .route("/api/dashboard/pipeline", get(dashboard_pipeline_handler))
        .route("/api/dashboard/by-org", get(dashboard_by_org_handler))
        .route("/api/dashboard/timeline", get(dashboard_timeline_handler))
        .route("/api/dashboard/discrepancies-chart", get(dashboard_disc_chart_handler))
        .route("/api/dashboard/staffing", get(dashboard_staffing_handler))
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
        .route("/api/data/cities", get(city_search_handler))
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
        .route("/api/admin/users/:user_id/toggle-org-names", post(admin_toggle_org_names_handler))
        // Admin: Dictionaries
        .route("/api/admin/dictionaries", get(admin_dictionaries_handler))
        .route("/api/admin/dictionaries/vos", post(admin_create_vos_handler))
        .route("/api/admin/dictionaries/vos/:id", axum::routing::put(admin_update_vos_handler))
        .route("/api/admin/dictionaries/vos/:id", axum::routing::delete(admin_delete_vos_handler))
        .route("/api/admin/dictionaries/equipment", post(admin_create_equipment_handler))
        .route("/api/admin/dictionaries/equipment/:id", axum::routing::put(admin_update_equipment_handler))
        .route("/api/admin/dictionaries/equipment/:id", axum::routing::delete(admin_delete_equipment_handler))
        // Admin: Cities
        .route("/api/admin/cities", get(admin_cities_handler))
        .route("/api/admin/cities", post(admin_create_city_handler))
        .route("/api/admin/cities/:id", axum::routing::put(admin_update_city_handler))
        .route("/api/admin/cities/:id", axum::routing::delete(admin_delete_city_handler))
        // Admin: Training Venues
        .route("/api/admin/venues", get(admin_venues_handler))
        .route("/api/admin/venues", post(admin_create_venue_handler))
        .route("/api/admin/venues/:id", axum::routing::put(admin_update_venue_handler))
        .route("/api/admin/venues/:id", axum::routing::delete(admin_delete_venue_handler))
        // Admin: Training Sites (legacy)
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
        .route("/api/admin/whatsapp/groups", get(whatsapp_groups_handler))
        // WhatsApp routing
        .route("/api/whatsapp/destinations", get(wa_destinations_handler))
        .route("/api/whatsapp/destinations", post(wa_add_destination_handler))
        .route("/api/whatsapp/destinations/:dest_id", axum::routing::delete(wa_delete_destination_handler))
        .route("/api/whatsapp/destinations/:dest_id/toggle", post(wa_toggle_destination_handler))
        .route("/api/whatsapp/destinations/:dest_id/subscriptions", get(wa_subscriptions_handler))
        .route("/api/whatsapp/destinations/:dest_id/subscriptions", post(wa_set_subscription_handler))
        .route("/api/whatsapp/notification-types", get(wa_notification_types_handler))
}
