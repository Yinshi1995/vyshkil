//! DTO для словників Етапу 2 (docs/spec/01-domain-model.md §2). Самі запити — в
//! `backend::repo::dictionaries` (лише сервер).

use serde::{Deserialize, Serialize};

/// Одна підказка "ОВТ/сленг → ВОС" (02 §3: "вамп" → 218, "mavic" → 217 …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentVosHint {
    pub vos_code: String,
    pub vos_title: String,
    pub matched_equipment: String,
    pub matched_raw: String,
    pub is_exact: bool,
}
