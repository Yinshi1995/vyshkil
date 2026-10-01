use serde::{Deserialize, Serialize};

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
