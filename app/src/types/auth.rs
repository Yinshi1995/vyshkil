use serde::{Deserialize, Serialize};

use super::actor::Actor;

/// Автентифікований користувач — результат перевірки cookie-сесії на сервері (12-auth.md §3).
/// Провайдиться як Leptos-контекст `Option<AuthUser>` — `None` якщо запит без валідної сесії.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user_id: i32,
    pub actor: Option<Actor>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub id: i32,
    pub login: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleRow {
    pub org_id: i32,
    pub role: String,
    pub org_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub session_id: String,
    pub user_id: i32,
    pub active_org_id: Option<i32>,
    pub active_role: Option<String>,
    pub login: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub error: Option<String>,
    pub display_name: Option<String>,
    pub roles: Vec<UserRoleRow>,
}

/// DTO зареєстрованого passkey для UI в /settings (12-auth.md §1.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasskeyInfo {
    pub id: i32,
    pub name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMode {
    Dev,
    Auth,
}
