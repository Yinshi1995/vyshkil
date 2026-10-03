//! DTO-и організацій, що їздять клієнт↔сервер (JSON, через `#[server]`-функції). Самі запити,
//! які їх наповнюють, — у `backend::repo::orgs` (лише на сервері); ці типи компілюються завжди
//! (клієнт отримує їх як звичайний Rust-тип завдяки `serde`).

use serde::{Deserialize, Serialize};

/// Один результат нечіткого пошуку організацій: канонічна форма + який саме синонім збігся.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrgSearchResult {
    pub org_id: i32,
    pub label: String,
    pub matched_raw: String,
    pub is_exact: bool,
}

/// Один вузол дерева підпорядкування: пряма (`depth = 1`) ланка з `subordination_closure` на дату.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrgTreeRow {
    pub id: i32,
    pub label: String,
    pub full_name: Option<String>,
    pub parent_id: Option<i32>,
}

/// Картка частини з історією назв і статусів.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrgDetail {
    pub id: i32,
    pub short_name: String,
    pub full_name: Option<String>,
    pub kind: String,
    pub is_active: bool,
    pub name_history: Vec<(String, String, Option<String>)>,
    pub status_history: Vec<(String, String, Option<String>, Option<String>)>,
}

/// Вузол ієрархічного дерева для адмін-сторінки підрозділів.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrgHierarchyNode {
    pub id: i32,
    pub short_name: String,
    pub kind: String,
    pub echelon: Option<String>,
    pub is_active: bool,
    pub parent_id: Option<i32>,
}

/// Номер ВЧ — окремий від назви (ДСК).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrgNumber {
    pub number_kind: Option<String>,
    pub number: Option<String>,
}

/// Зв'язок підпорядкування для деталей org.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubordinationLink {
    pub id: i32,
    pub other_org_id: i32,
    pub other_org_name: String,
    pub axis: String,
    pub valid_from: String,
    pub valid_to: Option<String>,
}
