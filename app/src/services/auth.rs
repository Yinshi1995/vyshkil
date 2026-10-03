use leptos::prelude::*;

use crate::types::auth::{LoginResponse, UserRoleRow};

#[server(AuthLogin, "/api")]
pub async fn auth_login(login: String, password: String) -> Result<LoginResponse, ServerFnError> {
    use crate::backend::repo;

    let client_ip = extract_client_ip().await.unwrap_or_default();
    if !check_rate_limit(&client_ip) {
        return Ok(LoginResponse {
            success: false,
            error: Some("Забагато спроб входу. Спробуйте через 15 хвилин.".to_string()),
            display_name: None,
            roles: vec![],
            must_change_password: false,
        });
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();

    let user = repo::auth::find_user_by_login(&db, &login)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let Some(user) = user else {
        record_failed_attempt(&client_ip);
        return Ok(LoginResponse {
            success: false,
            error: Some("Невірний логін або пароль".to_string()),
            display_name: None,
            roles: vec![],
            must_change_password: false,
        });
    };

    if !user.is_active {
        return Ok(LoginResponse {
            success: false,
            error: Some("Акаунт деактивовано".to_string()),
            display_name: None,
            roles: vec![],
            must_change_password: false,
        });
    }

    let password_valid = verify_password(&password, &user.password_hash);
    if !password_valid {
        record_failed_attempt(&client_ip);
        return Ok(LoginResponse {
            success: false,
            error: Some("Невірний логін або пароль".to_string()),
            display_name: None,
            roles: vec![],
            must_change_password: false,
        });
    }

    let roles = repo::auth::user_roles(&db, user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let (first_org, first_role) = roles.first().map(|r| (Some(r.org_id), Some(r.role.as_str()))).unwrap_or((None, None));

    let session_id = repo::auth::create_session(&db, user.id, first_org, first_role)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let response = expect_context::<leptos_axum::ResponseOptions>();
    let cookie = format!(
        "session={session_id}; HttpOnly; SameSite=Strict; Path=/; Max-Age=2592000"
    );
    response.insert_header(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(&cookie).unwrap(),
    );

    Ok(LoginResponse {
        success: true,
        error: None,
        display_name: user.display_name,
        roles,
        must_change_password: user.must_change_password,
    })
}

#[server(AuthLogout, "/api")]
pub async fn auth_logout() -> Result<(), ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();

    if let Some(session_id) = extract_session_cookie().await {
        let _ = repo::auth::delete_session(&db, &session_id).await;
    }

    let response = expect_context::<leptos_axum::ResponseOptions>();
    let cookie = "session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";
    response.insert_header(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(cookie).unwrap(),
    );

    Ok(())
}

#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<crate::types::auth::UserSession>, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();

    let Some(session_id) = extract_session_cookie().await else {
        return Ok(None);
    };

    repo::auth::find_session(&db, &session_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(GetUserRoles, "/api")]
pub async fn get_user_roles() -> Result<Vec<UserRoleRow>, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();

    let Some(session_id) = extract_session_cookie().await else {
        return Err(ServerFnError::new("не автентифіковано"));
    };

    let Some(session) = repo::auth::find_session(&db, &session_id).await.map_err(|e| ServerFnError::new(e.to_string()))? else {
        return Err(ServerFnError::new("сесія прострочена"));
    };

    repo::auth::user_roles(&db, session.user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(SwitchActor, "/api")]
pub async fn switch_actor(org_id: i32, role: String) -> Result<(), ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();

    let Some(session_id) = extract_session_cookie().await else {
        return Err(ServerFnError::new("не автентифіковано"));
    };

    repo::auth::update_session_actor(&db, &session_id, org_id, &role)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[cfg(feature = "ssr")]
fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};
    let Ok(parsed) = PasswordHash::new(hash) else { return false };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

#[cfg(feature = "ssr")]
pub async fn extract_session_cookie() -> Option<String> {
    let req = use_context::<http::request::Parts>()?;
    let cookies = req.headers.get(http::header::COOKIE)?.to_str().ok()?;
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

/// Перевіряє cookie-сесію і повертає автентифікованого користувача (12-auth.md §3).
/// Використовувати у server functions замість довіри клієнтському `actor`.
#[cfg(feature = "ssr")]
pub async fn require_auth() -> Result<crate::types::auth::AuthUser, ServerFnError> {
    use crate::backend::repo;
    use crate::types::actor::{Actor, Role};
    use crate::types::auth::AuthUser;

    let db = expect_context::<sea_orm::DatabaseConnection>();
    let session_id = extract_session_cookie()
        .await
        .ok_or_else(|| ServerFnError::new("не автентифіковано"))?;
    let session = repo::auth::find_session(&db, &session_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("сесія прострочена"))?;

    let actor = match (session.active_org_id, session.active_role.as_deref()) {
        (Some(org_id), Some(role_str)) => {
            Role::parse(role_str).map(|role| Actor { org_id, role })
        }
        _ => None,
    };

    Ok(AuthUser {
        user_id: session.user_id,
        actor,
        display_name: session.display_name,
    })
}

// ---------------------------------------------------------------------------
// Зміна пароля (must_change_password flow)
// ---------------------------------------------------------------------------

#[server(ChangePassword, "/api")]
pub async fn change_password(
    old_password: String,
    new_password: String,
) -> Result<(), ServerFnError> {
    use crate::backend::repo;
    use argon2::{Argon2, PasswordHash, PasswordVerifier, PasswordHasher};
    use argon2::password_hash::{SaltString, rand_core::OsRng};
    use sea_orm::FromQueryResult;

    if new_password.len() < 6 {
        return Err(ServerFnError::new("Новий пароль занадто короткий (мін. 6 символів)"));
    }

    let auth_user = require_auth().await?;
    let db = expect_context::<sea_orm::DatabaseConnection>();

    #[derive(FromQueryResult)]
    struct PwRow { password_hash: String }
    let pw_row = PwRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT password_hash FROM user_account WHERE id = $1",
        [auth_user.user_id.into()],
    ))
    .one(&db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("користувача не знайдено"))?;

    let parsed = PasswordHash::new(&pw_row.password_hash)
        .map_err(|_| ServerFnError::new("пошкоджений хеш"))?;
    if Argon2::default().verify_password(old_password.as_bytes(), &parsed).is_err() {
        return Err(ServerFnError::new("Невірний поточний пароль"));
    }

    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(new_password.as_bytes(), &salt)
        .map_err(|e| ServerFnError::new(format!("помилка хешування: {e}")))?
        .to_string();

    repo::auth::set_password(&db, auth_user.user_id, &new_hash)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

// ---------------------------------------------------------------------------
// Rate limiting (12-auth.md §6: 5 спроб / 15 хв на IP)
// ---------------------------------------------------------------------------

#[cfg(feature = "ssr")]
const MAX_LOGIN_ATTEMPTS: usize = 5;
#[cfg(feature = "ssr")]
const RATE_LIMIT_WINDOW_SECS: u64 = 900; // 15 хвилин

#[cfg(feature = "ssr")]
fn rate_limiter() -> &'static std::sync::Mutex<std::collections::HashMap<String, Vec<std::time::Instant>>> {
    use std::sync::OnceLock;
    static LIMITER: OnceLock<std::sync::Mutex<std::collections::HashMap<String, Vec<std::time::Instant>>>> = OnceLock::new();
    LIMITER.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

#[cfg(feature = "ssr")]
pub fn check_rate_limit(ip: &str) -> bool {
    let mut map = rate_limiter().lock().unwrap_or_else(|e| e.into_inner());
    let now = std::time::Instant::now();
    let window = std::time::Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
    if let Some(attempts) = map.get_mut(ip) {
        attempts.retain(|t| now.duration_since(*t) < window);
        attempts.len() < MAX_LOGIN_ATTEMPTS
    } else {
        true
    }
}

#[cfg(feature = "ssr")]
pub fn record_failed_attempt(ip: &str) {
    let mut map = rate_limiter().lock().unwrap_or_else(|e| e.into_inner());
    let now = std::time::Instant::now();
    let window = std::time::Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
    let attempts = map.entry(ip.to_string()).or_default();
    attempts.retain(|t| now.duration_since(*t) < window);
    attempts.push(now);
}

#[cfg(feature = "ssr")]
async fn extract_client_ip() -> Option<String> {
    let req = use_context::<http::request::Parts>()?;
    if let Some(forwarded) = req.headers.get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            return s.split(',').next().map(|ip| ip.trim().to_string());
        }
    }
    if let Some(real_ip) = req.headers.get("x-real-ip") {
        return real_ip.to_str().ok().map(|s| s.to_string());
    }
    None
}

/// Визначає актора: якщо є валідна auth-сесія — бере з неї (серверне забезпечення),
/// інакше — фолбек на клієнтського актора (dev-режим). Для поступової міграції
/// з клієнтського довірчого контракту на серверний.
#[cfg(feature = "ssr")]
pub async fn resolve_actor(
    client_actor: Option<crate::types::actor::Actor>,
) -> Result<crate::types::actor::Actor, ServerFnError> {
    if let Ok(auth_user) = require_auth().await {
        if let Some(actor) = auth_user.actor {
            return Ok(actor);
        }
    }
    client_actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))
}
