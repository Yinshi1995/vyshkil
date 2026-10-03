//! DTO для екрана розбіжностей (04 §4, Етап 8 зріз 1).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyValue {
    pub submission_id: i32,
    pub source_label: String,
    pub value: String,
}

/// Знімок одного подання для порівняння в розбіжності — повний рядок `reported_group`
/// з резолвленими лейблами (вид, ВОС, місце тощо).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportedGroupSnapshot {
    pub submission_id: i32,
    pub source_label: String,
    pub training_kind: String,
    pub vos_label: Option<String>,
    pub position_label: Option<String>,
    pub course_label: Option<String>,
    pub site_label: String,
    pub organizer_label: Option<String>,
    pub planned_start: String,
    pub planned_end: String,
    pub equipment_text: Option<String>,
    pub basis_doc_number: Option<String>,
    pub note: Option<String>,
    pub planned_count: i32,
    pub arrived_count: i32,
    pub in_training_count: i32,
}

/// Вміст одного подання — список reported_group рядків з лейблами.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmissionDetail {
    pub submission_id: i32,
    pub source_label: String,
    pub status: String,
    pub as_of_date: String,
    pub rows: Vec<ReportedGroupSnapshot>,
}

/// Відповідь на порівняння подань у розбіжності — конфліктна метрика + знімки обох рядків.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyComparison {
    pub metric: String,
    pub metric_label: String,
    pub rows: Vec<ReportedGroupSnapshot>,
}

/// Одна розбіжність — рядок для `/discrepancies`, уже з контекстом (назва частини/групи), щоб
/// сторінці не довелось окремо резолвити id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyRow {
    pub id: i32,
    pub kind: String,
    pub org_id: i32,
    pub org_label: String,
    pub group_id: Option<i32>,
    /// Короткий підпис канонічної групи ("Фахова · ВОС 218 · з 18.08.2026") — `None`, якщо
    /// `group_id` немає (майбутні kind без прив'язки до однієї групи, 04 §4).
    pub group_label: Option<String>,
    pub as_of: String,
    pub metric: String,
    /// Українська підпис метрика для інтерфейсу ("Прибуло", не "arrived_count") —
    /// `domain::reconciliation::METRICS`-ключ → підпис, мапиться на сервері (`repo`), не тут:
    /// `types/` — лише дані, без логіки підпису.
    pub metric_label: String,
    pub values: Vec<DiscrepancyValue>,
    pub status: String,
    pub created_at: String,
}
