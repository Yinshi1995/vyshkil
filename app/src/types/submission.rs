//! DTO для сітки введення (02) і чернеток (`submission`, 01 §5). Сама сітка (стан редагування,
//! undo/redo) — у `pages/training_form` (Leptos-специфічне, тут лише те, що їде клієнт↔сервер).

use serde::{Deserialize, Serialize};

/// Один підрозділ у розкривному підрядку "Розподіл за підрозділами" (02 §1 колонка 8).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CompositionRow {
    pub subunit_org_id: Option<i32>,
    pub subunit_label: String,
    pub count: i64,
}

/// Один рядок сітки = одна група на навчанні (02 §1). Поля вже **резолвлені** автокомплітом
/// (вибір із випадайки записує id + label; вільний текст лишається лише для `equipment_text`
/// і `note`) — сітка не надсилає сирий нерозпізнаний текст на коміт, лише вибране.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GroupFormRow {
    pub sender_org_id: Option<i32>,
    pub sender_org_label: String,

    pub training_kind_id: Option<i32>,
    pub training_kind_label: String,
    pub bzvp_program_id: Option<i32>,

    pub vos_id: Option<i32>,
    pub position_id: Option<i32>,
    pub course_id: Option<i32>,
    pub vos_position_course_label: String,

    pub equipment_text: String,

    pub site_id: Option<i32>,
    pub site_label: String,

    pub planned_start_raw: String,
    pub planned_end_raw: String,

    pub planned_count: i64,
    pub arrived_count: i64,
    pub in_training_count: i64,

    // #[serde(default)]: form-урленкодед серіалізація сервер-функцій (Leptos) не передає ключ
    // для порожнього Vec узагалі -- без default десеріалізація падає "missing field" щоразу,
    // коли підрозділи не заповнені (типовий випадок, 02 §1: "опційно").
    #[serde(default)]
    pub composition: Vec<CompositionRow>,

    pub organizer_org_id: Option<i32>,
    pub organizer_org_label: String,
    pub basis_doc_number: String,
    pub basis_doc_date_raw: String,

    pub note: String,
}

impl GroupFormRow {
}

/// Чернетка сітки: увесь незбережений стан форми (02 §6) — те, що автозберігається кожні
/// кілька секунд у `submission.draft_payload`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DraftPayload {
    pub as_of_date: String,
    #[serde(default)]
    pub rows: Vec<GroupFormRow>,
}

/// Результат спроби зафіксувати сітку (`Ctrl+Enter`, 02 §5): або все збережено, або перша
/// помилкова клітинка (індекс рядка + назва поля + текст) — сітка фокусує саме її.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommitOutcome {
    Committed { group_ids: Vec<i32> },
    ValidationFailed { row_index: usize, field: String, message: String },
}
