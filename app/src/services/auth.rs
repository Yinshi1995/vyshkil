use leptos::prelude::*;

use crate::types::auth::{LoginResponse, UserRoleRow};

#[server(AuthLogin, "/api")]
pub async fn auth_login(login: String, password: String) -> Result<LoginResponse, ServerFnError> {
    use crate::backend::repo;

    let db = expect_context::<sea_orm::DatabaseConnection>();

    let user = repo::auth::find_user_by_login(&db, &login)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let Some(user) = user else {
        return Ok(LoginResponse {
            success: false,
            error: Some("Невірний логін або пароль".to_string()),
            display_name: None,
            roles: vec![],
        });
    };

    if !user.is_active {
        return Ok(LoginResponse {
            success: false,
            error: Some("Акаунт деактивовано".to_string()),
            display_name: None,
            roles: vec![],
        });
    }

    let password_valid = verify_password(&password, &user.password_hash);
    if !password_valid {
        return Ok(LoginResponse {
            success: false,
            error: Some("Невірний логін або пароль".to_string()),
            display_name: None,
            roles: vec![],
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
async fn extract_session_cookie() -> Option<String> {
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
