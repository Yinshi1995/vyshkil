//! Актор = організація + роль (docs/spec/01-domain-model.md §6). Чистий тип без sea-orm/tokio —
//! той самий у WASM (перемикач у шапці) і на сервері (server::policy). Дозволу тут нема,
//! лише представлення "хто зараз обраний" — саму перевірку прав робить server::policy.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Admin,
    OrgEditor,
    Viewer,
}

impl Role {
    pub const ALL: [Role; 3] = [Role::Admin, Role::OrgEditor, Role::Viewer];

    pub fn label(&self) -> &'static str {
        match self {
            Role::Admin => "Адміністратор",
            Role::OrgEditor => "Редактор частини",
            Role::Viewer => "Перегляд",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::OrgEditor => "org_editor",
            Role::Viewer => "viewer",
        }
    }

    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "admin" => Some(Role::Admin),
            "org_editor" => Some(Role::OrgEditor),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
    pub org_id: i32,
    pub role: Role,
}
