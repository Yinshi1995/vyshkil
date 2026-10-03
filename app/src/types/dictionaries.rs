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

/// Один рядок будь-якого простого довідника (training_kind/training_direction/bzvp_program/
/// vos/position/equipment/course/attrition_reason) — узагальнено, щоб не заводити 8 майже
/// однакових DTO. `extra` — другорядна інформація (код/статус/категорія), якщо є.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub id: i32,
    pub label: String,
    pub extra: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_id: Option<i32>,
}

/// Довідники Етапу 2 одним запитом — сторінка адмінки показує всі одразу (01 §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictionariesOverview {
    pub training_kinds: Vec<DictionaryEntry>,
    pub training_directions: Vec<DictionaryEntry>,
    pub bzvp_programs: Vec<DictionaryEntry>,
    pub vos: Vec<DictionaryEntry>,
    pub positions: Vec<DictionaryEntry>,
    pub equipment: Vec<DictionaryEntry>,
    pub courses: Vec<DictionaryEntry>,
    pub attrition_reasons: Vec<DictionaryEntry>,
    pub training_sites: Vec<DictionaryEntry>,
}

/// Один learned-синонім, що чекає підтвердження адміном (01 §"alias", §"Навчання").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearnedAlias {
    pub id: i32,
    pub target_type: String,
    pub raw: String,
    pub norm: String,
    pub uses_count: i32,
    pub created_at: String,
}
